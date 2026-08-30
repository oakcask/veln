pub(crate) fn split_top_level_keyword_raw<'a>(predicate: &'a str, keyword: &str) -> Vec<&'a str> {
    let mut clauses = Vec::new();
    let mut start = 0;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    let mut cursor = 0;

    while cursor < predicate.len() {
        let character = predicate[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        let end = cursor + character.len_utf8();
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            cursor = end;
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ if depth == 0 && predicate[cursor..].starts_with(keyword) => {
                let keyword_end = cursor + keyword.len();
                if is_identifier_word_boundary(predicate, cursor, keyword_end) {
                    clauses.push(&predicate[start..cursor]);
                    start = keyword_end;
                    cursor = keyword_end;
                    continue;
                }
            }
            _ => {}
        }
        cursor = end;
    }

    clauses.push(&predicate[start..]);
    clauses
}

pub(crate) fn split_top_level_operator_where<'a>(
    predicate: &'a str,
    operator: &str,
    operator_at: impl Fn(&str, usize, &str) -> bool,
    operands_are_valid: impl Fn(&str, &str) -> bool,
) -> Option<(&'a str, &'a str)> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in predicate.char_indices().rev() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            ')' => depth += 1,
            '(' => depth = depth.saturating_sub(1),
            _ if depth == 0 && operator_at(predicate, index, operator) => {
                let left = predicate[..index].trim();
                let right = predicate[index + operator.len()..].trim();
                if !left.is_empty() && !right.is_empty() && operands_are_valid(left, right) {
                    return Some((left, right));
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn is_identifier_word_boundary(text: &str, start: usize, end: usize) -> bool {
    let before = text[..start].chars().next_back();
    let after = text[end..].chars().next();
    before.is_none_or(|character| !is_ident_continue(character))
        && after.is_none_or(|character| !is_ident_continue(character))
}

pub(crate) fn is_ident_continue(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

pub(crate) fn compact_predicate_text(predicate: &str) -> String {
    let mut output = String::with_capacity(predicate.len());
    let mut characters = predicate.chars();
    while let Some(character) = characters.next() {
        if character == '"' {
            output.push(character);
            let mut escaped = false;
            for string_character in characters.by_ref() {
                output.push(string_character);
                if escaped {
                    escaped = false;
                } else if string_character == '\\' {
                    escaped = true;
                } else if string_character == '"' {
                    break;
                }
            }
        } else if !character.is_whitespace() {
            output.push(character);
        }
    }
    output
}

pub(crate) fn rewrite_identifiers(
    expression: &str,
    preserve_whitespace: bool,
    mut write_identifier: impl FnMut(&str, bool, &mut String),
) -> String {
    let mut output = String::with_capacity(expression.len());
    let mut characters = expression.char_indices().peekable();
    while let Some((start, character)) = characters.next() {
        if character == '"' {
            output.push(character);
            let mut escaped = false;
            for (_, string_character) in characters.by_ref() {
                output.push(string_character);
                if escaped {
                    escaped = false;
                } else if string_character == '\\' {
                    escaped = true;
                } else if string_character == '"' {
                    break;
                }
            }
        } else if is_ident_start(character) {
            let mut end = start + character.len_utf8();
            while let Some((next, next_character)) = characters.peek().copied() {
                if !is_ident_continue(next_character) {
                    break;
                }
                characters.next();
                end = next + next_character.len_utf8();
            }
            write_identifier(
                &expression[start..end],
                is_value_identifier_position(expression, start, end),
                &mut output,
            );
        } else if preserve_whitespace || !character.is_whitespace() {
            output.push(character);
        }
    }
    output
}

pub(crate) fn is_value_identifier_position(expression: &str, start: usize, end: usize) -> bool {
    !expression[..start].ends_with('.')
        && !expression[..start].ends_with("::")
        && !expression[end..].starts_with("::")
}

pub(crate) fn is_ident_start(character: char) -> bool {
    character.is_ascii_alphabetic() || character == '_'
}

pub(crate) fn visit_unquoted_characters(
    text: &str,
    mut visit: impl FnMut(usize, char, usize) -> bool,
) -> usize {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        if visit(index, character, depth) {
            break;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    depth
}
