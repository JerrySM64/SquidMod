use std::collections::HashMap;

use super::address::AddressMap;
use super::assembler::assemble_with_notes;
use super::expr::evaluate;
use super::lexer::split_unquoted_commas;

#[derive(Clone, Copy)]
pub(super) enum DataKind {
    U8,
    U16,
    U32,
    F32,
    F64,
}

impl DataKind {
    pub(super) fn size(self) -> u32 {
        match self {
            DataKind::U8 => 1,
            DataKind::U16 => 2,
            DataKind::U32 | DataKind::F32 => 4,
            DataKind::F64 => 8,
        }
    }
}

pub(super) enum CaveEntry {
    Label(String),
    Instruction(String),
    Data(DataKind, Vec<String>),
    Align(u32),
}

pub(super) fn parse_cave_item(text: &str) -> Result<CaveEntry, String> {
    if !text.starts_with('.') {
        return Ok(CaveEntry::Instruction(text.to_string()));
    }
    let (name, args) = match text.split_once(char::is_whitespace) {
        Some((name, args)) => (name, args.trim()),
        None => (text, ""),
    };
    let kind = match name.to_ascii_lowercase().as_str() {
        ".byte" | ".string" | ".u8" | ".char" => DataKind::U8,
        ".short" | ".word" | ".u16" => DataKind::U16,
        ".int" | ".uint" | ".long" | ".ptr" | ".u32" => DataKind::U32,
        ".float" => DataKind::F32,
        ".double" => DataKind::F64,
        ".align" => {
            let alignment = evaluate(args, 0, &HashMap::new())?;
            if !(1..=255).contains(&alignment) {
                return Err(format!(".align expects a value from 1 to 255, got `{}`", args));
            }
            return Ok(CaveEntry::Align(alignment as u32));
        }
        other => return Err(format!("unsupported directive `{}`", other)),
    };
    let values = split_unquoted_commas(args);
    if values.is_empty() {
        return Err(format!("{} needs at least one value", name));
    }
    Ok(CaveEntry::Data(kind, values))
}

pub(super) fn unescape_string(literal: &str) -> Result<Vec<u8>, String> {
    let inner = literal
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .ok_or_else(|| format!("malformed string literal {}", literal))?;
    let mut bytes = Vec::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            let mut buffer = [0u8; 4];
            bytes.extend_from_slice(c.encode_utf8(&mut buffer).as_bytes());
            continue;
        }
        let escaped = match chars.next() {
            Some('n') => b'\n',
            Some('t') => b'\t',
            Some('r') => b'\r',
            Some('0') => 0,
            Some('\\') => b'\\',
            Some('"') => b'"',
            Some(other) => return Err(format!("unsupported escape \\{} in {}", other, literal)),
            None => return Err(format!("malformed string literal {}", literal)),
        };
        bytes.push(escaped);
    }
    Ok(bytes)
}

pub(super) fn data_size(kind: DataKind, values: &[String]) -> Result<u32, String> {
    let mut total: u32 = 0;
    for value in values {
        let size = if value.starts_with('"') {
            if !matches!(kind, DataKind::U8) {
                return Err(format!("string literals are only allowed in .byte or .string, got `{}`", value));
            }
            unescape_string(value)?.len() as u32 + 1
        } else {
            kind.size()
        };
        total = total.checked_add(size).ok_or("data directive is too large")?;
    }
    Ok(total)
}

pub(super) fn int_in_range(text: &str, addr: u32, symbols: &HashMap<String, u32>, bits: u32) -> Result<i64, String> {
    let value = evaluate(text, addr, symbols)?;
    let min = -(1i64 << (bits - 1));
    let max = (1i64 << bits) - 1;
    if (min..=max).contains(&value) {
        Ok(value)
    } else {
        Err(format!("`{}` does not fit in {} bits", text, bits))
    }
}

pub(super) fn parse_float<T: std::str::FromStr>(text: &str) -> Result<T, String> {
    text.trim().parse::<T>().map_err(|_| format!("invalid floating point value `{}`", text))
}

pub(super) fn encode_value(
    kind: DataKind,
    text: &str,
    addr: u32,
    symbols: &HashMap<String, u32>,
    out: &mut Vec<u8>,
) -> Result<(), String> {
    if text.starts_with('"') {
        out.extend(unescape_string(text)?);
        out.push(0);
        return Ok(());
    }
    match kind {
        DataKind::U8 => out.push(int_in_range(text, addr, symbols, 8)? as u8),
        DataKind::U16 => out.extend((int_in_range(text, addr, symbols, 16)? as u16).to_be_bytes()),
        DataKind::U32 => out.extend((int_in_range(text, addr, symbols, 32)? as u32).to_be_bytes()),
        DataKind::F32 => out.extend(parse_float::<f32>(text)?.to_bits().to_be_bytes()),
        DataKind::F64 => out.extend(parse_float::<f64>(text)?.to_bits().to_be_bytes()),
    }
    Ok(())
}

pub(super) fn encode_data(kind: DataKind, values: &[String], addr: u32, symbols: &HashMap<String, u32>) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for value in values {
        let value_addr = addr.wrapping_add(bytes.len() as u32);
        encode_value(kind, value, value_addr, symbols, &mut bytes)?;
    }
    Ok(bytes)
}

pub(super) fn encode_entry(
    entry: &CaveEntry,
    addr: u32,
    symbols: &HashMap<String, u32>,
    window: &AddressMap,
    notes: &mut Vec<String>,
) -> Result<Vec<u8>, String> {
    match entry {
        CaveEntry::Instruction(text) => {
            let (word, mut instruction_notes) = assemble_with_notes(text, addr, symbols, Some(window))?;
            notes.append(&mut instruction_notes);
            Ok(word.to_be_bytes().to_vec())
        }
        CaveEntry::Data(kind, values) => encode_data(*kind, values, addr, symbols),
        CaveEntry::Label(_) | CaveEntry::Align(_) => Ok(Vec::new()),
    }
}

pub(super) enum PatchBody {
    Instruction(String),
    Data(DataKind, Vec<String>),
}

pub(super) fn align_up(value: u32, alignment: u32) -> Option<u32> {
    let remainder = value % alignment;
    if remainder == 0 { Some(value) } else { value.checked_add(alignment - remainder) }
}

