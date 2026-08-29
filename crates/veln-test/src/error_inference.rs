use super::*;

pub(super) fn result_error_signatures(sources: &[SourceFile]) -> BTreeMap<String, Option<String>> {
    let mut signatures = BTreeMap::<String, Option<String>>::new();
    for source in sources {
        for line in source.text().lines() {
            let line = line.trim_start();
            let Some(name) = function_name(line) else {
                continue;
            };
            let Some(error_type) = function_result_error_type(line) else {
                continue;
            };
            signatures
                .entry(name.to_string())
                .and_modify(|existing| {
                    if existing.as_deref() != Some(error_type) {
                        *existing = None;
                    }
                })
                .or_insert_with(|| Some(error_type.to_string()));
        }
    }
    signatures
}

pub(super) fn function_name(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("pub fn ")
        .or_else(|| line.strip_prefix("fn "))?;
    let open = rest.find('(')?;
    let name = rest[..open].trim();
    (!name.is_empty()).then_some(name)
}

pub(super) fn function_result_error_type(line: &str) -> Option<&str> {
    let return_text = line.split_once("->")?.1;
    let return_text = return_text
        .split_once(" effects ")
        .map_or(return_text, |(return_text, _)| return_text)
        .trim();
    let return_text = strip_result_binding(return_text);
    result_error_type(return_text)
}

pub(super) fn inferred_doctest_error_type(
    code: &[String],
    signatures: &BTreeMap<String, Option<String>>,
) -> Option<String> {
    let mut inferred = None::<String>;
    let mut found_try = false;
    for line in code {
        for callee in propagated_call_names(line) {
            found_try = true;
            let error_type = signatures.get(callee).and_then(|value| value.as_deref())?;
            if inferred
                .as_deref()
                .is_some_and(|existing| existing != error_type)
            {
                return None;
            }
            inferred.get_or_insert_with(|| error_type.to_string());
        }
    }
    found_try.then_some(inferred).flatten()
}

pub(super) fn propagated_call_names(line: &str) -> Vec<&str> {
    let mut names = Vec::new();
    for (index, ch) in line.char_indices() {
        if ch != '?' {
            continue;
        }
        let Some(name) = propagated_call_name(&line[..index]) else {
            names.push("");
            continue;
        };
        names.push(name);
    }
    names
}

pub(super) fn propagated_call_name(text: &str) -> Option<&str> {
    let text = text.trim_end();
    if !text.ends_with(')') {
        return None;
    }
    let open = matching_open_paren(text)?;
    let before_open = text[..open].trim_end();
    let start = before_open
        .rfind(|ch: char| !(ch == '_' || ch == ':' || ch.is_ascii_alphanumeric()))
        .map_or(0, |index| index + 1);
    let name = &before_open[start..];
    (!name.is_empty()).then_some(name)
}

pub(super) fn matching_open_paren(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
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

pub(super) fn documented_result_error_type(line: &str) -> Option<String> {
    let line = line.trim_start();
    if !line.starts_with("pub fn ") {
        return None;
    }
    function_result_error_type(line).map(ToString::to_string)
}

pub(super) fn strip_result_binding(return_text: &str) -> &str {
    let Some((binding, ty)) = return_text.split_once(':') else {
        return return_text;
    };
    if binding
        .trim()
        .chars()
        .all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
    {
        ty.trim_start()
    } else {
        return_text
    }
}

pub(super) fn result_error_type(ty: &str) -> Option<&str> {
    let ty = ty.trim();
    let args = ty
        .strip_prefix("Result<")
        .and_then(|ty| ty.strip_suffix('>'))
        .or_else(|| {
            ty.strip_prefix("Result(")
                .and_then(|ty| ty.strip_suffix(')'))
        })?;
    let comma = top_level_comma(args)?;
    let error = args[comma + 1..].trim();
    (!error.is_empty()).then_some(error)
}

pub(super) fn top_level_comma(text: &str) -> Option<usize> {
    let mut depth = 0usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return Some(index),
            _ => {}
        }
    }
    None
}
