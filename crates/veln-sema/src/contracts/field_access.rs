use super::*;

pub(super) fn strip_balanced_outer_parens(text: &str) -> &str {
    let mut trimmed = text.trim();
    loop {
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let mut depth = 0usize;
        let mut balanced_outer = true;
        for (index, ch) in trimmed.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 && index != trimmed.len() - 1 {
                        balanced_outer = false;
                        break;
                    }
                }
                _ => {}
            }
        }
        if !balanced_outer || depth != 0 {
            return trimmed;
        }
        trimmed = trimmed[1..trimmed.len() - 1].trim();
    }
}

pub(super) struct FieldAccess {
    pub(super) base: String,
    pub(super) fields: Vec<String>,
}

pub(super) struct FieldAccessRef<'a> {
    pub(super) base: &'a str,
    pub(super) fields: Vec<&'a str>,
}

pub(super) fn split_field_access(predicate: &str) -> Option<FieldAccessRef<'_>> {
    let mut scanner = FieldAccessScanner::default();
    let mut first_dot = None;
    let mut fields = Vec::new();
    let mut index = 0usize;
    while index < predicate.len() {
        let ch = predicate[index..].chars().next()?;
        if scanner.consume_quoted(ch) {
            index += ch.len_utf8();
            continue;
        }
        match ch {
            '"' => {
                scanner.start_string();
                index += ch.len_utf8();
            }
            '(' => {
                scanner.open_group();
                index += ch.len_utf8();
            }
            ')' => {
                scanner.close_group();
                index += ch.len_utf8();
            }
            '.' if scanner.at_top_level() => {
                let (field, field_end) = parse_field_access_segment(predicate, index)?;
                first_dot.get_or_insert(index);
                fields.push(field);
                index = field_end;
                let rest = predicate[index..].trim_start();
                if rest.is_empty() {
                    break;
                }
                if !rest.starts_with('.') {
                    return None;
                }
            }
            _ => index += ch.len_utf8(),
        }
    }
    let dot = first_dot?;
    let base = predicate[..dot].trim();
    (!base.is_empty() && !fields.is_empty()).then_some(FieldAccessRef { base, fields })
}

#[derive(Default)]
pub(super) struct FieldAccessScanner {
    depth: usize,
    in_string: bool,
    escaped: bool,
}

impl FieldAccessScanner {
    fn consume_quoted(&mut self, ch: char) -> bool {
        if !self.in_string {
            return false;
        }
        if self.escaped {
            self.escaped = false;
        } else if ch == '\\' {
            self.escaped = true;
        } else if ch == '"' {
            self.in_string = false;
        }
        true
    }

    fn start_string(&mut self) {
        self.in_string = true;
    }

    fn open_group(&mut self) {
        self.depth += 1;
    }

    fn close_group(&mut self) {
        self.depth = self.depth.saturating_sub(1);
    }

    fn at_top_level(&self) -> bool {
        self.depth == 0
    }
}

pub(super) fn parse_field_access_segment(
    predicate: &str,
    dot_index: usize,
) -> Option<(&str, usize)> {
    let field_start = dot_index + '.'.len_utf8();
    let field_first = predicate[field_start..].chars().next()?;
    if !(field_first.is_ascii_alphabetic() || field_first == '_') {
        return None;
    }
    let mut field_end = field_start + field_first.len_utf8();
    while field_end < predicate.len() {
        let next = predicate[field_end..].chars().next()?;
        if next.is_ascii_alphanumeric() || next == '_' {
            field_end += next.len_utf8();
        } else {
            break;
        }
    }
    Some((&predicate[field_start..field_end], field_end))
}

pub(super) fn field_accesses(predicate: &str) -> Vec<FieldAccess> {
    let bytes = predicate.as_bytes();
    let mut accesses = Vec::new();
    for call in contract_calls(predicate) {
        if let Some(fields) = field_suffix(&predicate[call.end..]) {
            accesses.push(FieldAccess {
                base: predicate[call.start..call.end].to_string(),
                fields,
            });
        }
    }
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        let ch = bytes[index] as char;
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < bytes.len() {
            let ch = bytes[index] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                index += 1;
            } else {
                break;
            }
        }
        if start >= 1 && &predicate[start - 1..start] == "." {
            continue;
        }
        if start >= 2 && &predicate[start - 2..start] == "::" {
            continue;
        }
        if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
            continue;
        }
        let base = predicate[start..index].to_string();
        let mut fields = Vec::new();
        while index < bytes.len() && &predicate[index..index + 1] == "." {
            let field_start = index + 1;
            if field_start >= bytes.len() {
                break;
            }
            let first = bytes[field_start] as char;
            if !(first.is_ascii_alphabetic() || first == '_') {
                break;
            }
            index = field_start + 1;
            while index < bytes.len() {
                let ch = bytes[index] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' {
                    index += 1;
                } else {
                    break;
                }
            }
            fields.push(predicate[field_start..index].to_string());
        }
        if !fields.is_empty() {
            accesses.push(FieldAccess { base, fields });
        }
    }
    accesses
}

pub(super) fn field_suffix(text: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut rest = text.trim_start();
    while let Some(after_dot) = rest.strip_prefix('.') {
        let mut chars = after_dot.char_indices();
        let (_, first) = chars.next()?;
        if !(first.is_ascii_alphabetic() || first == '_') {
            return None;
        }
        let mut end = first.len_utf8();
        for (index, ch) in chars {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                end = index + ch.len_utf8();
            } else {
                break;
            }
        }
        fields.push(after_dot[..end].to_string());
        rest = after_dot[end..].trim_start();
    }
    (!fields.is_empty()).then_some(fields)
}

pub(super) fn is_complete_string_literal(text: &str) -> bool {
    if !text.starts_with('"') {
        return false;
    }
    string_literal_end(text, 0).is_some_and(|end| end == text.len())
}

pub(super) fn string_literal_end(text: &str, start: usize) -> Option<usize> {
    let mut escaped = false;
    let mut cursor = start + 1;
    while cursor < text.len() {
        let ch = text[cursor..].chars().next()?;
        cursor += ch.len_utf8();
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some(cursor);
        }
    }
    None
}
