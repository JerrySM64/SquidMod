use std::collections::BTreeMap;

use super::address::AddressMap;

pub(super) fn parse_hex_word(token: &str) -> Option<u32> {
    if token.len() == 8 && token.chars().all(|c| c.is_ascii_hexdigit()) {
        u32::from_str_radix(token, 16).ok()
    } else {
        None
    }
}

pub(super) fn parse_gecko(gecko: &str, window: &AddressMap, warnings: &mut Vec<String>) -> Result<BTreeMap<u32, u32>, String> {
    let mut writes = BTreeMap::new();
    for (index, raw) in gecko.lines().enumerate() {
        let number = index + 1;
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.contains('=') || line.ends_with(':') || line.starts_with('.') || line.starts_with('[') {
            return Err(format!(
                "line {}: `{}` is Cemu assembly, not a Gecko code; switch the mode to Cemu → Console",
                number, line
            ));
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let (addr_text, value_text) = match tokens.as_slice() {
            [addr, value] => (*addr, *value),
            _ => return Err(format!("line {}: expected `AAAAAAAA VVVVVVVV`, got `{}`", number, line)),
        };
        let (Some(addr), Some(value)) = (parse_hex_word(addr_text), parse_hex_word(value_text)) else {
            return Err(format!("line {}: expected two 8-digit hex words, got `{}`", number, line));
        };
        if addr == 0 {
            continue;
        }
        if addr % 4 != 0 {
            return Err(format!("line {}: address {} is not 4-byte aligned", number, addr_text));
        }
        if let Some(previous) = writes.insert(window.to_exec(addr), value) {
            warnings.push(format!(
                "line {}: {} is written more than once (0x{:08X}, then 0x{:08X}); keeping the last value",
                number, addr_text, previous, value
            ));
        }
    }
    Ok(writes)
}

pub(super) fn is_hex_word(token: &str) -> bool {
    token.len() == 8 && token.chars().all(|c| c.is_ascii_hexdigit())
}

pub(super) fn convert_addresses(gecko_text: &str, to_cemu: bool) -> String {
    let window = AddressMap::default();
    let mut output = Vec::new();
    let mut lines = gecko_text.lines();
    while let Some(line) = lines.next() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let is_pair = tokens.len() == 2 && tokens.iter().all(|t| is_hex_word(t));
        if !is_pair {
            output.push(line.to_string());
            continue;
        }
        if tokens[0][..2].eq_ignore_ascii_case("C2") {
            output.push(line.to_string());
            let body_lines = u32::from_str_radix(tokens[1], 16).unwrap_or(0);
            for _ in 0..body_lines {
                match lines.next() {
                    Some(body) => output.push(body.to_string()),
                    None => break,
                }
            }
            continue;
        }
        match u32::from_str_radix(tokens[0], 16) {
            Ok(addr) => {
                let converted = if to_cemu { window.to_exec(addr) } else { window.to_console(addr) };
                output.push(format!("{:08X} {}", converted, tokens[1]));
            }
            Err(_) => output.push(line.to_string()),
        }
    }
    output.join("\n")
}
