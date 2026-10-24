pub(super) fn recover_type_case(name: &str) -> Option<String> {
    let (prefix, leaf) = name.rsplit_once("::").unwrap_or(("", name));
    let mut chars = leaf.chars();
    let first = chars.next()?;
    if !first.is_ascii_lowercase() {
        return None;
    }
    let recovered_leaf = format!("{}{}", first.to_ascii_uppercase(), chars.as_str());
    Some(if prefix.is_empty() {
        recovered_leaf
    } else {
        format!("{prefix}::{recovered_leaf}")
    })
}
