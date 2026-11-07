pub(super) fn parse_reg(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(n) = s.strip_prefix('r') {
        n.parse::<u32>().map_err(|_| format!("Invalid register: {}", s))
            .and_then(|v| if v < 32 { Ok(v) } else { Err(format!("Register out of range: {}", s)) })
    } else {
        Err(format!("Expected register, got: {}", s))
    }
}

pub(super) fn parse_freg(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(n) = s.strip_prefix('f') {
        n.parse::<u32>().map_err(|_| format!("Invalid FP register: {}", s))
            .and_then(|v| if v < 32 { Ok(v) } else { Err(format!("FP register out of range: {}", s)) })
    } else {
        Err(format!("Expected FP register, got: {}", s))
    }
}

pub(super) fn parse_creg(s: &str) -> Result<u32, String> {
    let s = s.trim();
    if let Some(n) = s.strip_prefix("cr") {
        n.parse::<u32>().map_err(|_| format!("Invalid CR field: {}", s))
            .and_then(|v| if v < 8 { Ok(v) } else { Err(format!("CR field out of range: {}", s)) })
    } else {
        Err(format!("Expected CR field, got: {}", s))
    }
}

pub(super) fn split_operands(ops_str: &str) -> Vec<&str> {
    if ops_str.is_empty() {
        return Vec::new();
    }
    let mut result = Vec::new();
    let mut depth = 0i32;
    let mut start = 0;
    for (i, c) in ops_str.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                result.push(ops_str[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    result.push(ops_str[start..].trim());
    result
}

