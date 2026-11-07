use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::address::AddressMap;
use super::disassembler::{disassemble_for_cemu, disassemble_with_imm};
use super::gecko::parse_gecko;
use super::isa::branch_target;
use super::options::{PortOptions, Ported};
use super::pairs::{
    build_segments, hi_overlaps, is_lis, lone_low_halves, pair_consumers, translate_lone_high_half, translate_pair,
    Segment,
};

#[derive(Clone, Copy)]
pub(super) enum Half {
    Ha,
    H,
    L,
}

impl Half {
    fn suffix(self) -> &'static str {
        match self {
            Half::Ha => "ha",
            Half::H => "h",
            Half::L => "l",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct PairReloc {
    target: u32,
    half: Half,
}

pub(super) struct BranchRef {
    source: u32,
    target: u32,
    absolute: bool,
}

pub(super) fn scan_lis(
    run: &Segment,
    lis_index: usize,
    opts: &PortOptions,
    relocs: &mut BTreeMap<u32, PairReloc>,
    rewrites: &mut BTreeMap<u32, u32>,
    warnings: &mut Vec<String>,
) {
    let lis_addr = run.addr_of(lis_index);
    let hi = run.words[lis_index] & 0xFFFF;
    let consumers = pair_consumers(run, lis_index);
    let Some(&(first_index, first_raw, first_ori)) = consumers.first() else {
        if let Some(area) = [&opts.window.code, &opts.window.data]
            .into_iter()
            .find(|area| hi_overlaps(hi, &area.console_range()))
        {
            let (chosen, other) = translate_lone_high_half((hi << 16).wrapping_sub(area.console_offset));
            rewrites.insert(lis_addr, (run.words[lis_index] & 0xFFFF_0000) | chosen);
            if let Some(other) = other {
                warnings.push(format!(
                    "lis at 0x{:08X} has its low half in game code; 0x{:04X} was translated to 0x{:04X}, which is right unless that low half crosses a 64 KiB boundary after translation (then use 0x{:04X})",
                    lis_addr, hi, chosen, other
                ));
            }
        }
        return;
    };
    let first_target = opts.window.to_exec(first_raw);
    let relocated = opts.cave.contains(&first_target);
    if !relocated {
        let to_cemu = |target: u32| {
            let exec = opts.window.to_exec(target);
            (exec != target).then_some(exec)
        };
        match translate_pair(run, lis_index, to_cemu) {
            Ok(words) => rewrites.extend(words),
            Err(message) => warnings.push(message),
        }
        for &(index, raw, _) in &consumers[1..] {
            if opts.cave.contains(&opts.window.to_exec(raw)) {
                warnings.push(format!(
                    "0x{:08X} forms 0x{:08X} from the lis at 0x{:08X}, but only part of that sequence points into the code cave; check it by hand",
                    run.addr_of(index), raw, lis_addr
                ));
            }
        }
        return;
    }
    let half = if first_ori { Half::H } else { Half::Ha };
    relocs.insert(lis_addr, PairReloc { target: first_target, half });
    relocs.insert(run.addr_of(first_index), PairReloc { target: first_target, half: Half::L });
    for &(index, raw, _) in &consumers[1..] {
        let addr = run.addr_of(index);
        let target = opts.window.to_exec(raw);
        if opts.cave.contains(&target) {
            relocs.insert(addr, PairReloc { target, half: Half::L });
            warnings.push(format!(
                "0x{:08X} reuses the high half loaded by the lis at 0x{:08X}; the Cemu output assumes both cave labels share it",
                addr, lis_addr
            ));
        } else {
            warnings.push(format!(
                "0x{:08X} forms 0x{:08X} from the lis at 0x{:08X}, but only part of that sequence points into the code cave; check it by hand",
                addr, raw, lis_addr
            ));
        }
    }
}

pub(super) fn require_written(segments: &[Segment], window: &AddressMap, source: u32, target: u32) -> Result<(), String> {
    if segments.iter().any(|segment| segment.contains(target)) {
        return Ok(());
    }
    Err(format!(
        "0x{:08X} (console 0x{:08X}) references 0x{:08X} (console 0x{:08X}), which is inside the code cave but not written by this code",
        source,
        window.to_console(source),
        target,
        window.to_console(target)
    ))
}

pub fn console_to_cemu(gecko: &str, opts: &PortOptions) -> Result<Ported, String> {
    let mut warnings = Vec::new();
    let writes = parse_gecko(gecko, &opts.window, &mut warnings)?;
    if writes.is_empty() {
        return Err("No Gecko writes found".into());
    }

    let mut cave_writes = BTreeMap::new();
    let mut code_patches = BTreeMap::new();
    let mut data_patches = BTreeMap::new();
    for (&addr, &value) in &writes {
        if opts.cave.contains(&addr) {
            cave_writes.insert(addr, value);
        } else if opts.window.is_code(addr) {
            code_patches.insert(addr, value);
        } else {
            data_patches.insert(addr, value);
        }
    }

    let segments = build_segments(&cave_writes, opts.max_gap);
    let patch_runs = build_segments(&code_patches, 0);

    let mut branch_refs = Vec::new();
    let mut pair_relocs = BTreeMap::new();
    let mut word_rewrites = BTreeMap::new();
    for run in patch_runs.iter().chain(segments.iter()) {
        for (index, &word) in run.words.iter().enumerate() {
            let addr = run.addr_of(index);
            if let Some((target, absolute)) = branch_target(word, addr)
                && opts.cave.contains(&target)
            {
                branch_refs.push(BranchRef { source: addr, target, absolute });
            }
            if is_lis(word) {
                scan_lis(run, index, opts, &mut pair_relocs, &mut word_rewrites, &mut warnings);
            }
        }
    }
    let low_shift = opts.window.data.console_offset & 0xFFFF;
    if low_shift != 0 {
        for run in patch_runs.iter().chain(segments.iter()) {
            for (index, lo) in lone_low_halves(run) {
                let addr = run.addr_of(index);
                let translated = lo.wrapping_sub(low_shift) & 0xFFFF;
                word_rewrites.insert(addr, (run.words[index] & 0xFFFF_0000) | translated);
                warnings.push(format!(
                    "0x{:08X}: the displacement 0x{:04X} from r{} was translated to 0x{:04X}, assuming r{} holds the high half of a game data address set up outside this patch; if it is a plain field offset, keep 0x{:04X}",
                    addr,
                    lo,
                    (run.words[index] >> 16) & 0x1F,
                    translated,
                    (run.words[index] >> 16) & 0x1F,
                    lo
                ));
            }
        }
    }
    for (&addr, &word) in &data_patches {
        if opts.window.is_mapped_console(word) {
            warnings.push(format!(
                "the data word 0x{:08X} written to 0x{:08X} looks like a console address; on Cemu it would be 0x{:08X}",
                word,
                addr,
                opts.window.to_exec(word)
            ));
        }
    }

    let mut label_addrs: BTreeSet<u32> = segments.iter().map(|segment| segment.start).collect();
    for reference in &branch_refs {
        require_written(&segments, &opts.window, reference.source, reference.target)?;
        label_addrs.insert(reference.target);
    }
    for (&source, reloc) in &pair_relocs {
        require_written(&segments, &opts.window, source, reloc.target)?;
        label_addrs.insert(reloc.target & !3);
    }
    let labels: HashMap<u32, String> = label_addrs
        .iter()
        .enumerate()
        .map(|(index, &addr)| (addr, format!("cave{}", index + 1)))
        .collect();

    for reference in branch_refs.iter().filter(|reference| reference.absolute) {
        if let Some(label) = labels.get(&reference.target) {
            warnings.push(format!(
                "absolute branch at 0x{:08X} into the code cave was rewritten as a relative branch to {}",
                reference.source, label
            ));
        }
    }

    let render = |addr: u32, word: u32| -> Result<String, String> {
        if let Some(reloc) = pair_relocs.get(&addr) {
            let label_addr = reloc.target & !3;
            let label = labels
                .get(&label_addr)
                .ok_or_else(|| format!("no label for 0x{:08X}", label_addr))?;
            let offset = reloc.target & 3;
            let expr = if offset == 0 {
                format!("{}@{}", label, reloc.half.suffix())
            } else {
                format!("{}+{}@{}", label, offset, reloc.half.suffix())
            };
            return disassemble_with_imm(word, &expr)
                .ok_or_else(|| format!("cannot relocate the instruction at 0x{:08X}", addr));
        }
        let word = word_rewrites.get(&addr).copied().unwrap_or(word);
        let text = disassemble_for_cemu(word, addr, &labels);
        if text.starts_with(".int")
            && branch_target(word, addr).is_some_and(|(target, _)| labels.contains_key(&target))
        {
            return Err(format!(
                "the branch at 0x{:08X} into the code cave has no Cemu mnemonic and cannot be relocated",
                addr
            ));
        }
        Ok(text)
    };

    let mut lines: Vec<String> = Vec::new();
    if let Some(header) = &opts.header {
        lines.push(format!("[{}]", header.name));
        lines.push(format!("moduleMatches = 0x{:08X}", header.module_matches));
        lines.push(String::new());
    }

    let mut patches: Vec<(u32, String)> = Vec::new();
    for (&addr, &word) in &code_patches {
        patches.push((addr, render(addr, word)?));
    }
    for (&addr, &word) in &data_patches {
        patches.push((addr, format!(".int 0x{:08X}", word)));
    }
    patches.sort_by_key(|(addr, _)| *addr);
    for (addr, text) in patches {
        lines.push(format!("0x{:08X} = {}", addr, text));
    }

    if !segments.is_empty() {
        if lines.last().is_some_and(|line| !line.is_empty()) {
            lines.push(String::new());
        }
        lines.push(".origin = codecave".to_string());
        for segment in &segments {
            lines.push(String::new());
            for (index, &word) in segment.words.iter().enumerate() {
                let addr = segment.addr_of(index);
                if let Some(label) = labels.get(&addr) {
                    if index > 0 {
                        lines.push(String::new());
                    }
                    lines.push(format!("{}:", label));
                }
                lines.push(render(addr, word)?);
            }
        }
    }

    Ok(Ported { text: lines.join("\n"), warnings })
}

