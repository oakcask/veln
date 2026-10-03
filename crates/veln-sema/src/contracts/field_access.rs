use super::*;

pub(super) fn strip_balanced_outer_parens(text: &str) -> &str {
    let mut trimmed = text.trim();
    loop {
        if !(trimmed.starts_with('(') && trimmed.ends_with(')')) {
            return trimmed;
        }
        let mut depth = 0usize;
        let mut balanced_outer = true;
        let mut string_scanner = StringLiteralScanner::default();
        for (index, ch) in trimmed.char_indices() {
            if string_scanner.consume(ch) {
                continue;
            }
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

#[derive(Default)]
pub(super) struct StringLiteralScanner {
    in_string: bool,
    escaped: bool,
}

impl StringLiteralScanner {
    pub(super) fn consume(&mut self, ch: char) -> bool {
        if !self.in_string {
            if ch == '"' {
                self.in_string = true;
                return true;
            }
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
        if scanner.consume_non_field(ch) {
            index += ch.len_utf8();
            continue;
        }
        match ch {
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
    fn consume_non_field(&mut self, ch: char) -> bool {
        if self.consume_quoted(ch) {
            return true;
        }
        match ch {
            '"' => self.start_string(),
            '(' => self.open_group(),
            ')' => self.close_group(),
            _ => return false,
        }
        true
    }

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
    let mut accesses = Vec::new();
    for call in contract_calls(predicate) {
        if let Some(fields) = field_suffix(&predicate[call.end..]) {
            accesses.push(FieldAccess {
                base: predicate[call.start..call.end].to_string(),
                fields,
            });
        }
    }
    collect_binding_field_accesses(predicate, &mut accesses);
    accesses
}

fn collect_binding_field_accesses(predicate: &str, accesses: &mut Vec<FieldAccess>) {
    let bytes = predicate.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        let start = index;
        let Some(end) = field_identifier_end(bytes, start) else {
            index += 1;
            continue;
        };
        index = end;
        if field_access_base_is_qualified(predicate, start, index) {
            continue;
        }
        let base = predicate[start..index].to_string();
        let fields = binding_field_segments(predicate, &mut index);
        if !fields.is_empty() {
            accesses.push(FieldAccess { base, fields });
        }
    }
}

fn field_identifier_end(bytes: &[u8], start: usize) -> Option<usize> {
    let first = *bytes.get(start)? as char;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    let mut end = start + 1;
    while end < bytes.len() {
        let ch = bytes[end] as char;
        if !(ch.is_ascii_alphanumeric() || ch == '_') {
            break;
        }
        end += 1;
    }
    Some(end)
}

fn field_access_base_is_qualified(predicate: &str, start: usize, end: usize) -> bool {
    (start >= 1 && &predicate[start - 1..start] == ".")
        || (start >= 2 && &predicate[start - 2..start] == "::")
        || (end + 2 <= predicate.len() && &predicate[end..end + 2] == "::")
}

fn binding_field_segments(predicate: &str, index: &mut usize) -> Vec<String> {
    let bytes = predicate.as_bytes();
    let mut fields = Vec::new();
    while *index < bytes.len() && &predicate[*index..*index + 1] == "." {
        let field_start = *index + 1;
        let Some(field_end) = field_identifier_end(bytes, field_start) else {
            break;
        };
        *index = field_end;
        fields.push(predicate[field_start..field_end].to_string());
    }
    fields
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
