use super::*;

pub fn canonical_type_text(text: &str) -> String {
    canonicalize_commas(&canonicalize_type_segment(text))
}

fn canonicalize_commas(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        out.push(ch);
        if ch == ',' {
            while chars.peek().is_some_and(|next| next.is_whitespace()) {
                chars.next();
            }
            if chars
                .peek()
                .is_some_and(|next| !matches!(next, ')' | ']' | '}' | '>'))
            {
                out.push(' ');
            }
        }
    }
    out
}

pub(super) fn canonical_schema_field_type_text(text: &str, binary_schema: bool) -> String {
    let text = canonical_predicate_text(text);
    if binary_schema {
        canonical_binary_schema_field_type_text(&text)
    } else {
        text
    }
}

pub(super) fn canonical_predicate_text(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(" :: ", "::")
        .replace(" (", "(")
        .replace("( ", "(")
        .replace(" )", ")")
        .replace(" . ", ".")
        .replace("[ ", "[")
        .replace(" ]", "]")
        .replace(" ,", ",")
        .replace(" ; ", "; ")
        .replace(" ;", ";")
        .replace(";  ", "; ")
}

fn canonicalize_type_segment(text: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = cursor;
            cursor = consume_type_path(text, cursor);
            let path = &text[start..cursor];
            if path == "Unit" {
                out.push_str("()");
            } else {
                out.push_str(path);
            }

            if path != "fn" && text[cursor..].starts_with('(') {
                if let Some(close) = matching_delimiter(text, cursor, '(', ')') {
                    out.push('<');
                    out.push_str(&canonicalize_type_segment(&text[cursor + 1..close]));
                    out.push('>');
                    cursor = close + 1;
                }
            } else if text[cursor..].starts_with('<')
                && let Some(close) = matching_delimiter(text, cursor, '<', '>')
            {
                out.push('<');
                out.push_str(&canonicalize_type_segment(&text[cursor + 1..close]));
                out.push('>');
                cursor = close + 1;
            }
        } else {
            out.push(ch);
            cursor += ch.len_utf8();
        }
    }
    out
}

fn canonical_binary_schema_field_type_text(text: &str) -> String {
    if let Some(primitive) = canonical_compatible_schema_primitive(text) {
        return primitive;
    }
    if let Some(reserved) = canonical_reserved_bits_schema_field_type_call(text) {
        return reserved;
    }
    if let Some(repeated) = canonical_repeat_schema_field_type_call(text) {
        return repeated;
    }
    if let Some(repeated) = canonical_repeat_schema_field_type_brackets(text) {
        return repeated;
    }
    if let Some(dispatch) = canonical_dispatch_schema_field_type_call(text, "Dispatch") {
        return dispatch;
    }
    if let Some(dispatch) = canonical_dispatch_schema_field_type_call(text, "ExtensionDispatch") {
        return dispatch;
    }
    text.to_string()
}

fn canonical_binary_schema_payload_type_text(text: &str) -> String {
    if let Some(primitive) = canonical_compatible_schema_primitive(text) {
        return primitive;
    }
    if let Some(reserved) = canonical_reserved_bits_schema_field_type_call(text) {
        return reserved;
    }
    text.to_string()
}

fn canonical_repeat_schema_payload_type_text(text: &str) -> String {
    if let Some(repeated) = canonical_repeat_schema_field_type_call(text) {
        return repeated;
    }
    if let Some(repeated) = canonical_repeat_schema_field_type_brackets(text) {
        return repeated;
    }
    canonical_binary_schema_payload_type_text(text)
}

fn canonical_repeat_schema_field_type_call(text: &str) -> Option<String> {
    let inner = exact_call_inner(text, "Repeat")?;
    let args = split_top_level_args(inner);
    let [count, payload] = args.as_slice() else {
        return None;
    };
    let count = canonical_predicate_text(count);
    let payload = canonical_repeat_schema_payload_type_text(&canonical_predicate_text(payload));
    Some(format!("[{payload}; {count}]"))
}

fn canonical_repeat_schema_field_type_brackets(text: &str) -> Option<String> {
    let inner = exact_bracket_inner(text)?;
    let (payload, count) = split_top_level_once(inner, ';')?;
    let payload = canonical_repeat_schema_payload_type_text(&canonical_predicate_text(payload));
    let count = canonical_predicate_text(count);
    Some(format!("[{payload}; {count}]"))
}

fn canonical_reserved_bits_schema_field_type_call(text: &str) -> Option<String> {
    let inner = exact_call_inner(text, "ReservedBits")?;
    let args = split_top_level_args(inner);
    let [width, value] = args.as_slice() else {
        return None;
    };
    let width = width.trim();
    let value = value.trim();
    let Ok(width_literal) = parse_integer_literal(width) else {
        return None;
    };
    if parse_integer_literal(value).is_err() {
        return None;
    }
    if width != width_literal.value.to_string() {
        return Some(format!("ReservedBits({width}, {value})"));
    }
    let Ok(width) = u16::try_from(width_literal.value) else {
        return None;
    };
    let endian = match width {
        1..=8 => "",
        16 | 24 | 31 | 32 | 40 | 48 | 56 | 64 => "be",
        _ => return None,
    };
    Some(format!("uint{width}{endian} reserves {value}"))
}

fn canonical_dispatch_schema_field_type_call(text: &str, name: &str) -> Option<String> {
    let inner = exact_call_inner(text, name)?;
    let args = split_top_level_args(inner)
        .into_iter()
        .map(|arg| {
            if let Some((tag, payload)) = arg.split_once("=>") {
                let tag = canonical_predicate_text(tag.trim());
                let payload = canonical_binary_schema_payload_type_text(&canonical_predicate_text(
                    payload.trim(),
                ));
                format!("{tag} => {payload}")
            } else {
                canonical_predicate_text(&arg)
            }
        })
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("{name}({args})"))
}

fn canonical_compatible_schema_primitive(text: &str) -> Option<String> {
    let rest = text.strip_prefix("UInt")?;
    let width_len = rest
        .char_indices()
        .take_while(|(_, ch)| ch.is_ascii_digit())
        .map(|(index, ch)| index + ch.len_utf8())
        .last()?;
    let width = &rest[..width_len];
    let suffix = &rest[width_len..];
    let width_bits = width.parse::<u16>().ok()?;
    let supported = matches!(
        (width_bits, suffix),
        (1..=8, "") | (16 | 24 | 31 | 32 | 40 | 48 | 56 | 64, "be" | "le")
    );
    supported.then(|| format!("uint{width}{suffix}"))
}

fn exact_call_inner<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let rest = text.strip_prefix(name)?;
    if !rest.starts_with('(') {
        return None;
    }
    let open = name.len();
    let close = matching_delimiter(text, open, '(', ')')?;
    (close + 1 == text.len()).then_some(&text[open + 1..close])
}

fn exact_bracket_inner(text: &str) -> Option<&str> {
    if !text.starts_with('[') {
        return None;
    }
    let close = matching_delimiter(text, 0, '[', ']')?;
    (close + 1 == text.len()).then_some(&text[1..close])
}

fn split_top_level_once(text: &str, delimiter: char) -> Option<(&str, &str)> {
    let cursor = top_level_delimiter_indices(text, delimiter).next()?;
    Some((
        text[..cursor].trim(),
        text[cursor + delimiter.len_utf8()..].trim(),
    ))
}

fn top_level_delimiter_indices(text: &str, delimiter: char) -> impl Iterator<Item = usize> + '_ {
    let mut depth = 0usize;
    text.char_indices().filter_map(move |(cursor, ch)| {
        match ch {
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth = depth.saturating_sub(1),
            _ if ch == delimiter && depth == 0 => return Some(cursor),
            _ => return None,
        }
        None
    })
}

fn split_top_level_args(text: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut start = 0usize;
    for cursor in top_level_delimiter_indices(text, ',') {
        args.push(text[start..cursor].trim().to_string());
        start = cursor + ','.len_utf8();
    }
    args.push(text[start..].trim().to_string());
    args
}

fn consume_type_path(text: &str, mut cursor: usize) -> usize {
    cursor = consume_ident(text, cursor);
    while text[cursor..].starts_with("::") {
        let segment_start = cursor + 2;
        let segment_end = consume_ident(text, segment_start);
        if segment_end == segment_start {
            break;
        }
        cursor = segment_end;
    }
    cursor
}

fn consume_ident(text: &str, mut cursor: usize) -> usize {
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch.is_ascii_alphanumeric() || ch == '_' {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }
    cursor
}

fn matching_delimiter(text: &str, open: usize, open_ch: char, close_ch: char) -> Option<usize> {
    let mut cursor = open;
    let mut depth = 0usize;
    while cursor < text.len() {
        let ch = text[cursor..]
            .chars()
            .next()
            .expect("cursor should stay on a char boundary");
        if ch == open_ch {
            depth += 1;
        } else if ch == close_ch {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(cursor);
            }
        }
        cursor += ch.len_utf8();
    }
    None
}
