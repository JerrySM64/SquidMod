use std::collections::BTreeMap;
use std::ops::Range;

pub(super) const NOP: u32 = 0x6000_0000;

pub(super) struct Segment {
    pub(super) start: u32,
    pub(super) words: Vec<u32>,
}

impl Segment {
    pub(super) fn addr_of(&self, index: usize) -> u32 {
        self.start.wrapping_add(4 * index as u32)
    }

    pub(super) fn end(&self) -> u32 {
        self.addr_of(self.words.len())
    }

    pub(super) fn contains(&self, addr: u32) -> bool {
        (self.start..self.end()).contains(&addr)
    }
}

pub(super) fn build_segments(writes: &BTreeMap<u32, u32>, max_gap: u32) -> Vec<Segment> {
    let mut segments: Vec<Segment> = Vec::new();
    for (&addr, &value) in writes {
        if let Some(last) = segments.last_mut()
            && addr.wrapping_sub(last.end()) <= max_gap
        {
            while last.end() < addr {
                last.words.push(NOP);
            }
            last.words.push(value);
            continue;
        }
        segments.push(Segment { start: addr, words: vec![value] });
    }
    segments
}

pub(super) fn sign_extend16(value: u32) -> u32 {
    value as u16 as i16 as i32 as u32
}

pub(super) fn ends_pair_scan(word: u32) -> bool {
    match word >> 26 {
        16..=18 => true,
        19 => matches!((word >> 1) & 0x3FF, 16 | 50 | 528),
        _ => false,
    }
}

pub(super) fn may_write_gpr(word: u32, reg: u32) -> bool {
    let opcode = word >> 26;
    let rt = (word >> 21) & 0x1F;
    let ra = (word >> 16) & 0x1F;
    let update = opcode & 1 == 1;
    match opcode {
        7 | 8 | 12 | 13 | 14 | 15 => rt == reg,
        20 | 21 | 23 | 24..=29 => ra == reg,
        32..=35 | 40..=43 => rt == reg || (update && ra == reg),
        36..=39 | 44 | 45 | 48..=55 => update && ra == reg,
        10 | 11 | 59 | 63 => false,
        31 => may_write_gpr_indexed(word, reg),
        _ => true,
    }
}

pub(super) fn may_write_gpr_indexed(word: u32, reg: u32) -> bool {
    let rt = (word >> 21) & 0x1F;
    let ra = (word >> 16) & 0x1F;
    match (word >> 1) & 0x3FF {
        0 | 32 | 86 | 144 | 146 | 151 | 215 | 246 | 278 | 407 | 467 | 598 | 854 | 535 | 599 | 663 | 727 => false,
        24 | 26 | 28 | 60 | 124 | 284 | 316 | 412 | 444 | 476 | 536 | 792 | 824 | 922 | 954 => ra == reg,
        _ => rt == reg || ra == reg,
    }
}

pub(super) fn pair_consumer(word: u32, reg: u32, hi: u32) -> Option<(u32, bool)> {
    let opcode = word >> 26;
    let rs = (word >> 21) & 0x1F;
    let ra = (word >> 16) & 0x1F;
    let lo = word & 0xFFFF;
    let base = hi << 16;
    match opcode {
        14 | 32..=55 if ra == reg => Some((base.wrapping_add(sign_extend16(lo)), false)),
        24 if rs == reg => Some((base | lo, true)),
        _ => None,
    }
}

pub(super) fn hi_overlaps(hi: u32, range: &Range<u32>) -> bool {
    let candidate = (hi as u64) << 16;
    let low = candidate.saturating_sub(0x8000);
    let high = candidate + 0x1_0000;
    (range.start as u64) < high && low < range.end as u64
}

pub(super) fn is_lis(word: u32) -> bool {
    word >> 26 == 15 && (word >> 16) & 0x1F == 0
}

pub(super) fn high_half(target: u32, is_ori: bool) -> u32 {
    if is_ori { target >> 16 } else { target.wrapping_add(0x8000) >> 16 }
}

pub(super) fn lone_low_halves(run: &Segment) -> Vec<(usize, u32)> {
    let mut found = Vec::new();
    for (index, &word) in run.words.iter().enumerate() {
        let opcode = word >> 26;
        let ra = (word >> 16) & 0x1F;
        if !(opcode == 14 || (32..=55).contains(&opcode)) || matches!(ra, 0 | 1 | 2 | 13) {
            continue;
        }
        let lo = word & 0xFFFF;
        if (sign_extend16(lo) as i32).unsigned_abs() < 0x1000 {
            continue;
        }
        if run.words[..index].iter().any(|&earlier| may_write_gpr(earlier, ra)) {
            continue;
        }
        found.push((index, lo));
    }
    found
}

pub(super) fn translate_lone_high_half(base: u32) -> (u32, Option<u32>) {
    let chosen = (base.wrapping_add(0x8000) >> 16) & 0xFFFF;
    if base & 0xFFFF == 0 {
        return (chosen, None);
    }
    let floor = (base >> 16) & 0xFFFF;
    let other = if chosen == floor { (floor + 1) & 0xFFFF } else { floor };
    (chosen, Some(other))
}

pub(super) fn pair_consumers(run: &Segment, lis_index: usize) -> Vec<(usize, u32, bool)> {
    let lis = run.words[lis_index];
    let reg = (lis >> 21) & 0x1F;
    let hi = lis & 0xFFFF;
    let mut consumers = Vec::new();
    if reg == 0 {
        return consumers;
    }
    for (index, &word) in run.words.iter().enumerate().skip(lis_index + 1) {
        if ends_pair_scan(word) {
            break;
        }
        if let Some((target, is_ori)) = pair_consumer(word, reg, hi) {
            consumers.push((index, target, is_ori));
        }
        if may_write_gpr(word, reg) {
            break;
        }
    }
    consumers
}

pub(super) fn translate_pair(run: &Segment, lis_index: usize, map: impl Fn(u32) -> Option<u32>) -> Result<Vec<(u32, u32)>, String> {
    let consumers = pair_consumers(run, lis_index);
    let Some(&(_, first_target, first_ori)) = consumers.first() else {
        return Ok(Vec::new());
    };
    let Some(first_translated) = map(first_target) else {
        return Ok(Vec::new());
    };
    let lis_addr = run.addr_of(lis_index);
    let hi = high_half(first_translated, first_ori);
    let base = hi << 16;
    let mut words = vec![(lis_addr, (run.words[lis_index] & 0xFFFF_0000) | hi)];
    for &(index, target, is_ori) in &consumers {
        let addr = run.addr_of(index);
        let Some(translated) = map(target) else {
            return Err(format!(
                "0x{:08X} and 0x{:08X} share the lis at 0x{:08X}, but only the first is a relocated address; it was not translated",
                first_target, target, lis_addr
            ));
        };
        let lo = if is_ori {
            if translated & 0xFFFF_0000 != base {
                return Err(format!(
                    "the pointers built from the lis at 0x{:08X} no longer share a high half after translation; it was not translated",
                    lis_addr
                ));
            }
            translated & 0xFFFF
        } else {
            let delta = translated.wrapping_sub(base) as i32;
            if !(-0x8000..0x8000).contains(&delta) {
                return Err(format!(
                    "the pointers built from the lis at 0x{:08X} no longer share a high half after translation; it was not translated",
                    lis_addr
                ));
            }
            delta as u32 & 0xFFFF
        };
        words.push((addr, (run.words[index] & 0xFFFF_0000) | lo));
    }
    Ok(words)
}

