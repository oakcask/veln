use veln_ast::ContractKind;

use crate::types::{Binding, Type};

pub(crate) enum ContractValidation {
    Valid,
    NonBoolean { actual_type: String },
    UnsupportedConstruct { reason: &'static str },
    UnresolvedName { name: String },
}

pub(crate) fn contract_kind_text(kind: ContractKind) -> &'static str {
    match kind {
        ContractKind::Require => "require",
        ContractKind::Ensure => "ensure",
    }
}

pub(crate) fn contains_call_like_construct(predicate: &str) -> bool {
    let bytes = predicate.as_bytes();
    bytes.windows(1).enumerate().any(|(index, window)| {
        window == b"(" && index > 0 && predicate[..index].trim_end().ends_with_identifier()
    })
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
    bindings.iter().any(|binding| {
        binding.name == trimmed && matches!(binding.ty, Type::Named { ref name, ref args } if name == "Bool" && args.is_empty())
    })
}

pub(crate) fn predicate_rendered_type(predicate: &str, bindings: &[Binding]) -> String {
    let trimmed = predicate.trim();
    if trimmed.starts_with('"') {
        return Type::string().render();
    }
    if trimmed.chars().all(|ch| ch.is_ascii_digit()) {
        return Type::int().render();
    }
    bindings
        .iter()
        .find(|binding| binding.name == trimmed)
        .map_or_else(|| "unknown".to_string(), |binding| binding.ty.render())
}
