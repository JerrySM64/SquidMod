use std::collections::HashMap;

pub(super) fn parse_number(s: &str) -> Option<i64> {
    let value = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()?
    } else {
        s.parse::<u64>().ok()?
    };
    if value <= u32::MAX as u64 { Some(value as i64) } else { None }
}

pub(super) fn is_symbol_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '.'
}

pub(super) fn eval_term(term: &str, addr: u32, symbols: &HashMap<String, u32>) -> Result<i64, String> {
    if term == "." {
        return Ok(addr as i64);
    }
    if term.starts_with(|c: char| c.is_ascii_digit()) {
        return parse_number(term).ok_or_else(|| format!("invalid number `{}`", term));
    }
    if !term.chars().all(is_symbol_char) {
        return Err(format!("invalid expression term `{}`", term));
    }
    symbols.get(term).map(|&v| v as i64).ok_or_else(|| format!("unknown symbol `{}`", term))
}

pub(super) fn eval_sum(body: &str, addr: u32, symbols: &HashMap<String, u32>) -> Result<i64, String> {
    let mut total: i64 = 0;
    let mut sign: i64 = 1;
    let mut term = String::new();
    for c in body.chars() {
        if c != '+' && c != '-' {
            term.push(c);
            continue;
        }
        if term.trim().is_empty() {
            if c == '-' {
                sign = -sign;
            }
            continue;
        }
        total += sign * eval_term(term.trim(), addr, symbols)?;
        term.clear();
        sign = if c == '-' { -1 } else { 1 };
    }
    if term.trim().is_empty() {
        return Err(format!("incomplete expression `{}`", body));
    }
    Ok(total + sign * eval_term(term.trim(), addr, symbols)?)
}

pub fn evaluate(expr: &str, addr: u32, symbols: &HashMap<String, u32>) -> Result<i64, String> {
    let expr = expr.trim();
    let (body, modifier) = match expr.rfind('@') {
        Some(pos) => (expr[..pos].trim(), Some(expr[pos + 1..].trim().to_ascii_lowercase())),
        None => (expr, None),
    };
    if body.is_empty() {
        return Err(format!("empty expression `{}`", expr));
    }
    let sum = eval_sum(body, addr, symbols)?;
    let value = sum as u32;
    match modifier.as_deref() {
        None => Ok(sum),
        Some("ha") => Ok((value.wrapping_add(0x8000) >> 16) as i64),
        Some("h") | Some("hi") => Ok((value >> 16) as i64),
        Some("l") | Some("lo") => Ok((value & 0xFFFF) as i64),
        Some(other) => Err(format!("unknown relocation modifier `@{}` in `{}`", other, expr)),
    }
}

