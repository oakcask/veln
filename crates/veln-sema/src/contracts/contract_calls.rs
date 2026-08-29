use super::*;

pub(crate) fn contract_calls(predicate: &str) -> Vec<ContractCall> {
    let bytes = predicate.as_bytes();
    let mut calls = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        if bytes[index] != b'('
            || index == 0
            || !predicate[..index].trim_end().ends_with_identifier()
        {
            index += 1;
            continue;
        }
        let Some(callee_start) = callee_start(predicate, index) else {
            index += 1;
            continue;
        };
        let Some(close) = matching_close(predicate, index) else {
            index += 1;
            continue;
        };
        calls.push(ContractCall {
            callee: predicate[callee_start..index].trim().to_string(),
            args: split_call_args(&predicate[index + 1..close]),
            start: callee_start,
            end: close + 1,
        });
        index += 1;
    }
    calls
}

pub(super) trait EndsWithIdentifier {
    fn ends_with_identifier(&self) -> bool;
}

impl EndsWithIdentifier for str {
    fn ends_with_identifier(&self) -> bool {
        self.chars()
            .next_back()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }
}

pub(super) fn callee_start(predicate: &str, open: usize) -> Option<usize> {
    let bytes = predicate.as_bytes();
    let mut index = open;
    while index > 0 {
        let ch = bytes[index - 1] as char;
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' {
            index -= 1;
        } else {
            break;
        }
    }
    (index < open).then_some(index)
}

pub(super) fn matching_close(predicate: &str, open: usize) -> Option<usize> {
    let bytes = predicate.as_bytes();
    let mut depth = 0usize;
    for (index, byte) in bytes.iter().enumerate().skip(open) {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

pub(super) fn split_call_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0usize;
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                let arg = text[start..index].trim();
                if !arg.is_empty() {
                    args.push(arg.to_string());
                }
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    let arg = text[start..].trim();
    if !arg.is_empty() {
        args.push(arg.to_string());
    }
    args
}

pub(crate) fn referenced_names(predicate: &str) -> Vec<String> {
    let mut names = Vec::new();
    let bytes = predicate.as_bytes();
    let mut index = 0usize;
    while index < bytes.len() {
        if bytes[index] == b'"' {
            index = string_literal_end(predicate, index).unwrap_or(predicate.len());
            continue;
        }
        if bytes[index] == b'0'
            && bytes
                .get(index + 1)
                .is_some_and(|byte| matches!(*byte, b'b' | b'B' | b'x' | b'X'))
        {
            index += 2;
            while index < bytes.len()
                && ((bytes[index] as char).is_ascii_alphanumeric() || bytes[index] == b'_')
            {
                index += 1;
            }
            continue;
        }
        let ch = bytes[index] as char;
        if ch.is_ascii_alphabetic() || ch == '_' {
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
            if start >= 2 && &predicate[start - 2..start] == "::" {
                continue;
            }
            if index + 2 <= bytes.len() && &predicate[index..index + 2] == "::" {
                continue;
            }
            if start >= 1 && &predicate[start - 1..start] == "." {
                continue;
            }
            let name = predicate[start..index].to_string();
            if !names.contains(&name) {
                names.push(name);
            }
        } else {
            index += 1;
        }
    }
    names
}

pub(crate) fn is_contract_keyword(name: &str) -> bool {
    matches!(name, "and" | "or" | "not")
}
