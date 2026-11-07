pub(super) fn find_unquoted(line: &str, pattern: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    for (index, c) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }
            continue;
        }
        if c == '"' {
            in_string = true;
            continue;
        }
        if line[index..].starts_with(pattern) {
            return Some(index);
        }
    }
    None
}

pub(super) fn strip_comment(line: &str) -> &str {
    let end = ["#", ";", "//"]
        .iter()
        .filter_map(|pattern| find_unquoted(line, pattern))
        .min()
        .unwrap_or(line.len());
    &line[..end]
}

pub(super) fn split_unquoted_commas(args: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = args.trim();
    if rest.is_empty() {
        return values;
    }
    while let Some(comma) = find_unquoted(rest, ",") {
        values.push(rest[..comma].trim().to_string());
        rest = &rest[comma + 1..];
    }
    values.push(rest.trim().to_string());
    values
}

pub(super) fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() || first == '_' || first == '.' => {
            text != "." && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        }
        _ => false,
    }
}

