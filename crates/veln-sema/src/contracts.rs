use veln_ast::ContractKind;

use crate::types::{Binding, Type};

pub(crate) enum ContractValidation {
    Valid,
    NonBoolean { actual_type: String },
    UnsupportedConstruct { reason: &'static str },
    UnresolvedName { name: String },
    MissingField { base_type: String, field: String },
}

pub(crate) struct ContractCall {
    pub(crate) callee: String,
    pub(crate) args: Vec<String>,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

pub(crate) fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
    }
}

pub(crate) fn contract_calls(predicate: &str) -> Vec<ContractCall> {
    let bytes = predicate.as_bytes();
    let mut calls = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
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

trait EndsWithIdentifier {
    fn ends_with_identifier(&self) -> bool;
}

impl EndsWithIdentifier for str {
    fn ends_with_identifier(&self) -> bool {
        self.chars()
            .rev()
            .next()
            .is_some_and(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    }
}

fn callee_start(predicate: &str, open: usize) -> Option<usize> {
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

fn matching_close(predicate: &str, open: usize) -> Option<usize> {
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

fn split_call_args(text: &str) -> Vec<String> {
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

pub(crate) fn missing_contract_field(
    predicate: &str,
    bindings: &[Binding],
) -> Option<(String, String)> {
    for access in field_accesses(predicate) {
        let Some(binding) = bindings.iter().find(|binding| binding.name == access.base) else {
            continue;
        };
        let mut current = &binding.ty;
        for field in access.fields {
            let Some(next) = current.record_field(&field) else {
                return Some((current.render(), field));
            };
            current = next;
        }
    }
    None
}

pub(crate) fn predicate_is_boolean(predicate: &str, bindings: &[Binding]) -> bool {
    let trimmed = predicate.trim();
    if matches!(trimmed, "true" | "false") {
        return true;
    }
    if trimmed.contains(" and ")
        || trimmed.contains(" or ")
        || trimmed.starts_with("not ")
        || ["==", "!=", "<=", ">=", "<", ">"]
            .iter()
            .any(|operator| trimmed.contains(operator))
    {
        return true;
    }
    predicate_type(trimmed, bindings).is_some_and(
        |ty| matches!(ty, Type::Named { name, args } if name == "Bool" && args.is_empty()),
    )
}

pub(crate) fn predicate_rendered_type(predicate: &str, bindings: &[Binding]) -> String {
    let trimmed = predicate.trim();
    if trimmed.starts_with('"') {
        return Type::string().render();
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return Type::int().render();
    }
    predicate_type(trimmed, bindings).map_or_else(|| "unknown".to_string(), |ty| ty.render())
}

fn predicate_type(predicate: &str, bindings: &[Binding]) -> Option<Type> {
    if let Some(binding) = bindings.iter().find(|binding| binding.name == predicate) {
        return Some(binding.ty.clone());
    }
    let mut parts = predicate.split('.');
    let base = parts.next()?;
    let binding = bindings.iter().find(|binding| binding.name == base)?;
    let mut current = binding.ty.clone();
    for field in parts {
        current = current.record_field(field)?.clone();
    }
    Some(current)
}

struct FieldAccess {
    base: String,
    fields: Vec<String>,
}

fn field_accesses(predicate: &str) -> Vec<FieldAccess> {
    let bytes = predicate.as_bytes();
    let mut accesses = Vec::new();
    let mut index = 0usize;
    while index < bytes.len() {
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
