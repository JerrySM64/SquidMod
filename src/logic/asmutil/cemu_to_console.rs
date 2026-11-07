use std::collections::{BTreeMap, HashMap};

use super::address::AddressMap;
use super::assembler::assemble_with_notes;
use super::data::{align_up, data_size, encode_data, encode_entry, parse_cave_item, CaveEntry, PatchBody};
use super::expr::evaluate;
use super::lexer::{find_unquoted, is_identifier, strip_comment};
use super::options::{PortOptions, Ported};
use super::pairs::{
    build_segments, hi_overlaps, is_lis, lone_low_halves, pair_consumers, translate_lone_high_half, translate_pair,
};

pub(super) fn translate_literal_pointers(
    code: &BTreeMap<u32, u32>,
    code_lines: &HashMap<u32, usize>,
    window: &AddressMap,
    warnings: &mut Vec<String>,
) -> BTreeMap<u32, u32> {
    let mut fixes = BTreeMap::new();
    let to_console = |target: u32| window.is_mapped_exec(target).then(|| window.to_console(target));
    let low_shift = window.data.console_offset & 0xFFFF;
    for run in build_segments(code, 0) {
        if low_shift != 0 {
            for (index, lo) in lone_low_halves(&run) {
                let addr = run.addr_of(index);
                let line = code_lines.get(&addr).copied().unwrap_or(0);
                let translated = lo.wrapping_add(low_shift) & 0xFFFF;
                fixes.insert(addr, (run.words[index] & 0xFFFF_0000) | translated);
                warnings.push(format!(
                    "line {}: the displacement 0x{:04X} from r{} was translated to 0x{:04X}, assuming r{} holds the high half of a game data address set up outside this patch; if it is a plain field offset, keep 0x{:04X}",
                    line,
                    lo,
                    (run.words[index] >> 16) & 0x1F,
                    translated,
                    (run.words[index] >> 16) & 0x1F,
                    lo
                ));
            }
        }
        for (index, &word) in run.words.iter().enumerate() {
            if !is_lis(word) {
                continue;
            }
            let addr = run.addr_of(index);
            let line = code_lines.get(&addr).copied().unwrap_or(0);
            let hi = word & 0xFFFF;
            if pair_consumers(&run, index).is_empty() {
                if let Some(area) = [&window.code, &window.data].into_iter().find(|area| hi_overlaps(hi, &area.exec)) {
                    let (chosen, other) = translate_lone_high_half((hi << 16).wrapping_add(area.console_offset));
                    fixes.insert(addr, (word & 0xFFFF_0000) | chosen);
                    if let Some(other) = other {
                        warnings.push(format!(
                            "line {}: this lis has its low half in game code; 0x{:04X} was translated to 0x{:04X}, which is right unless that low half crosses a 64 KiB boundary after translation (then use 0x{:04X})",
                            line, hi, chosen, other
                        ));
                    }
                }
                continue;
            }
            match translate_pair(&run, index, to_console) {
                Ok(words) => fixes.extend(words),
                Err(message) => warnings.push(format!("line {}: {}", line, message)),
            }
        }
    }
    fixes
}

pub(super) fn bind_labels(
    pending: &mut Vec<(usize, String)>,
    addr: u32,
    symbols: &mut HashMap<String, u32>,
) -> Result<(), String> {
    for (number, name) in pending.drain(..) {
        if symbols.insert(name.clone(), addr).is_some() {
            return Err(format!("line {}: label `{}` is defined twice", number, name));
        }
    }
    Ok(())
}

pub(super) fn resolve_definitions(
    definitions: Vec<(usize, String, String)>,
    symbols: &mut HashMap<String, u32>,
) -> Result<(), String> {
    let mut unresolved = definitions;
    while !unresolved.is_empty() {
        let before = unresolved.len();
        let mut remaining = Vec::new();
        let mut first_error: Option<String> = None;
        for (number, name, expr) in unresolved {
            match evaluate(&expr, 0, symbols) {
                Ok(value) => {
                    if symbols.insert(name.clone(), value as u32).is_some() {
                        return Err(format!("line {}: `{}` is defined twice", number, name));
                    }
                }
                Err(error) => {
                    if first_error.is_none() {
                        first_error = Some(format!("line {}: {}", number, error));
                    }
                    remaining.push((number, name, expr));
                }
            }
        }
        if remaining.len() == before {
            return Err(first_error.unwrap_or_else(|| "unresolved symbol definitions".to_string()));
        }
        unresolved = remaining;
    }
    Ok(())
}

pub fn cemu_to_console(asm: &str, cave_base: u32, opts: &PortOptions) -> Result<Ported, String> {
    if cave_base % 4 != 0 {
        return Err(format!("cave base 0x{:08X} is not 4-byte aligned", cave_base));
    }
    let mut warnings = Vec::new();
    let mut group: Option<String> = None;
    let mut in_cave = false;
    let mut cave: Vec<(usize, CaveEntry)> = Vec::new();
    let mut patches: Vec<(usize, u32, PatchBody)> = Vec::new();
    let mut definitions: Vec<(usize, String, String)> = Vec::new();

    for (index, raw) in asm.lines().enumerate() {
        let number = index + 1;
        let fail = |message: String| format!("line {}: {}", number, message);
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            continue;
        }
        if find_unquoted(line, "import.").is_some() {
            return Err(fail("`import.` references are not supported".to_string()));
        }
        if find_unquoted(line, "$").is_some() {
            return Err(fail("preset variables ($name) are not supported".to_string()));
        }

        if line.starts_with('[') {
            let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) else {
                return Err(fail(format!("malformed group header `{}`", line)));
            };
            if group.is_some() {
                return Err(fail("a second [Group] was found; multi-group files aren't supported yet".to_string()));
            }
            group = Some(name.trim().to_string());
            continue;
        }

        if let Some(eq) = find_unquoted(line, "=") {
            let key = line[..eq].trim();
            let value = line[eq + 1..].trim();
            if key.eq_ignore_ascii_case(".origin") {
                if value.eq_ignore_ascii_case("codecave") {
                    in_cave = true;
                    continue;
                }
                return Err(fail(format!("only `.origin = codecave` is supported, got `.origin = {}`", value)));
            }
            if key.eq_ignore_ascii_case("moduleMatches") || key.eq_ignore_ascii_case("codeCaveSize") {
                continue;
            }
            if let Some(hex) = key.strip_prefix("0x").or_else(|| key.strip_prefix("0X")) {
                let addr = u32::from_str_radix(hex, 16).map_err(|_| fail(format!("invalid patch address `{}`", key)))?;
                if value.is_empty() {
                    return Err(fail(format!("patch at {} has no instruction", key)));
                }
                let body = if value.starts_with('.') {
                    match parse_cave_item(value).map_err(fail)? {
                        CaveEntry::Data(kind, values) => PatchBody::Data(kind, values),
                        _ => return Err(fail(format!("`{}` cannot be used in a patch write", value))),
                    }
                } else if addr % 4 != 0 {
                    return Err(fail(format!("patch address {} is not 4-byte aligned", key)));
                } else {
                    PatchBody::Instruction(value.to_string())
                };
                patches.push((number, addr, body));
                continue;
            }
            if is_identifier(key) {
                definitions.push((number, key.to_string(), value.to_string()));
                continue;
            }
            return Err(fail(format!("cannot parse `{}`", line)));
        }

        if !in_cave {
            in_cave = true;
            warnings.push(format!(
                "line {}: no `.origin = codecave` before this line; placing it in the code cave",
                number
            ));
        }

        let mut rest = line;
        if let Some(colon) = find_unquoted(line, ":")
            && is_identifier(line[..colon].trim())
        {
            cave.push((number, CaveEntry::Label(line[..colon].trim().to_string())));
            rest = line[colon + 1..].trim();
            if rest.is_empty() {
                continue;
            }
        }
        cave.push((number, parse_cave_item(rest).map_err(fail)?));
    }

    let mut symbols: HashMap<String, u32> = HashMap::new();
    let mut pending: Vec<(usize, String)> = Vec::new();
    let mut placed: Vec<(usize, u32, &CaveEntry)> = Vec::new();
    let mut cursor = cave_base;
    for (number, entry) in &cave {
        let number = *number;
        let overflow = || format!("line {}: the code cave runs past the end of the address space", number);
        match entry {
            CaveEntry::Label(name) => pending.push((number, name.clone())),
            CaveEntry::Align(alignment) => cursor = align_up(cursor, *alignment).ok_or_else(overflow)?,
            CaveEntry::Instruction(_) => {
                if cursor % 4 != 0 {
                    return Err(format!(
                        "line {}: instruction at 0x{:08X} is not 4-byte aligned; add `.align 4` before it",
                        number, cursor
                    ));
                }
                bind_labels(&mut pending, cursor, &mut symbols)?;
                placed.push((number, cursor, entry));
                cursor = cursor.checked_add(4).ok_or_else(overflow)?;
            }
            CaveEntry::Data(kind, values) => {
                cursor = align_up(cursor, kind.size()).ok_or_else(overflow)?;
                bind_labels(&mut pending, cursor, &mut symbols)?;
                placed.push((number, cursor, entry));
                let size = data_size(*kind, values).map_err(|error| format!("line {}: {}", number, error))?;
                cursor = cursor.checked_add(size).ok_or_else(overflow)?;
            }
        }
    }
    bind_labels(&mut pending, cursor, &mut symbols)?;
    for value in symbols.values_mut() {
        *value = opts.window.to_console(*value);
    }
    resolve_definitions(definitions, &mut symbols)?;

    let window = &opts.window;
    let mut code: BTreeMap<u32, u32> = BTreeMap::new();
    let mut code_lines: HashMap<u32, usize> = HashMap::new();

    let mut image = vec![0u8; (cursor - cave_base) as usize];
    for &(number, addr, entry) in &placed {
        let mut notes = Vec::new();
        let bytes = encode_entry(entry, window.to_console(addr), &symbols, window, &mut notes)
            .map_err(|error| format!("line {}: {}", number, error))?;
        warnings.extend(notes.into_iter().map(|note| format!("line {}: {}", number, note)));
        if let CaveEntry::Instruction(_) = entry
            && let [b0, b1, b2, b3] = bytes.as_slice()
        {
            code.insert(addr, u32::from_be_bytes([*b0, *b1, *b2, *b3]));
            code_lines.insert(addr, number);
        }
        let offset = (addr - cave_base) as usize;
        image[offset..offset + bytes.len()].copy_from_slice(&bytes);
    }
    while image.len() % 4 != 0 {
        image.push(0);
    }

    let cave_end = cave_base.wrapping_add(image.len() as u32);
    let mut chunks: Vec<(u32, Vec<u8>, bool)> = Vec::new();
    for (number, addr, body) in &patches {
        let (number, addr) = (*number, *addr);
        let at_line = |message: String| format!("line {}: {}", number, message);
        let (start, bytes, is_instruction) = match body {
            PatchBody::Instruction(text) => {
                let (word, notes) = assemble_with_notes(text, window.to_console(addr), &symbols, Some(window))
                    .map_err(at_line)?;
                warnings.extend(notes.into_iter().map(at_line));
                code.insert(addr, word);
                code_lines.insert(addr, number);
                (addr, word.to_be_bytes().to_vec(), true)
            }
            PatchBody::Data(kind, values) => {
                let data = encode_data(*kind, values, window.to_console(addr), &symbols).map_err(at_line)?;
                let start = addr & !3;
                let mut bytes = vec![0u8; (addr - start) as usize];
                bytes.extend_from_slice(&data);
                while bytes.len() % 4 != 0 {
                    bytes.push(0);
                }
                let padding = bytes.len() - data.len();
                if padding > 0 {
                    warnings.push(at_line(format!(
                        "the {} bytes at 0x{:08X} don't fill whole 32-bit words, so the console writes also zero {} byte(s) next to them",
                        data.len(),
                        addr,
                        padding
                    )));
                }
                (start, bytes, false)
            }
        };
        let end = start.wrapping_add(bytes.len() as u32);
        if start < cave_end && cave_base < end {
            warnings.push(at_line(format!("the patch at 0x{:08X} overwrites part of the code cave", addr)));
        }
        chunks.push((start, bytes, is_instruction));
    }

    let fixes = translate_literal_pointers(&code, &code_lines, window, &mut warnings);
    for (&addr, &word) in &fixes {
        if (cave_base..cave_end).contains(&addr) {
            let offset = (addr - cave_base) as usize;
            image[offset..offset + 4].copy_from_slice(&word.to_be_bytes());
        }
    }
    for (start, bytes, is_instruction) in &mut chunks {
        if *is_instruction && let Some(word) = fixes.get(&*start) {
            bytes.copy_from_slice(&word.to_be_bytes());
        }
    }

    let mut lines = Vec::new();
    for (index, chunk) in image.chunks_exact(4).enumerate() {
        let addr = cave_base.wrapping_add(4 * index as u32);
        let word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        lines.push(format!("{:08X} {:08X}", window.to_console(addr), word));
    }
    for (start, bytes, _) in &chunks {
        for (index, chunk) in bytes.chunks_exact(4).enumerate() {
            let word_addr = start.wrapping_add(4 * index as u32);
            let word = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            lines.push(format!("{:08X} {:08X}", window.to_console(word_addr), word));
        }
    }

    if lines.is_empty() {
        return Err("No instructions to assemble".into());
    }
    Ok(Ported { text: lines.join("\n"), warnings })
}

