use std::collections::BTreeMap;

use super::*;

pub(super) fn collect_adr_lite_records(
    source: &SourceFile,
    tokens: &[Token],
    module: Option<&ModuleDecl>,
    items: &[SyntaxItem],
) -> Vec<AdrLiteRecord> {
    let anchors = adr_lite_anchors(module, items);
    let mut records = Vec::new();
    let mut cursor = 0;

    while cursor < tokens.len() {
        let token = &tokens[cursor];
        if token.kind != TokenKind::Comment || !is_adr_lite_marker(&doc_comment_text(token)) {
            cursor += 1;
            continue;
        }
        if let Some(record) = parse_adr_lite_record(source, tokens, &mut cursor, &anchors) {
            records.push(record);
        }
    }

    records
}

fn parse_adr_lite_record(
    source: &SourceFile,
    tokens: &[Token],
    cursor: &mut usize,
    anchors: &[(usize, AdrLiteAnchor)],
) -> Option<AdrLiteRecord> {
    let start = tokens[*cursor].range;
    *cursor += 1;
    let (mut fields, end) = collect_adr_lite_fields(tokens, cursor, start);
    let id = fields.remove("id")?;
    let status = fields.remove("status")?;
    let scope = fields.remove("scope")?;
    let context = fields.remove("context")?;
    let decision = fields.remove("decision")?;
    let consequences = fields.remove("consequences")?;
    let span = source.span(start.cover(end));
    let anchor = anchors
        .iter()
        .find_map(|(offset, anchor)| (*offset >= span.end.offset).then(|| anchor.clone()));
    Some(AdrLiteRecord {
        id,
        status,
        scope,
        context,
        decision,
        consequences,
        anchor,
        span,
    })
}

fn collect_adr_lite_fields(
    tokens: &[Token],
    cursor: &mut usize,
    start: TextRange,
) -> (BTreeMap<String, String>, TextRange) {
    let mut fields = BTreeMap::new();
    let mut end = start;
    while *cursor < tokens.len() {
        match tokens[*cursor].kind {
            TokenKind::Whitespace | TokenKind::Newline => *cursor += 1,
            TokenKind::Comment => {
                let content = doc_comment_text(&tokens[*cursor]);
                if content.starts_with('@') {
                    break;
                }
                if let Some((key, value)) = content.split_once(':') {
                    fields.insert(key.trim().to_string(), value.trim().to_string());
                }
                end = end.cover(tokens[*cursor].range);
                *cursor += 1;
            }
            _ => break,
        }
    }
    (fields, end)
}

fn adr_lite_anchors(
    module: Option<&ModuleDecl>,
    items: &[SyntaxItem],
) -> Vec<(usize, AdrLiteAnchor)> {
    let mut anchors = Vec::new();
    if let Some(module) = module {
        anchors.push((
            module.span.start.offset,
            AdrLiteAnchor::Module {
                name: module.name.clone(),
            },
        ));
    }
    for item in items {
        let SyntaxItem::Function(function) = item else {
            continue;
        };
        if function.visibility == Visibility::Public
            && let Some(name) = &function.name
        {
            anchors.push((
                function.span.start.offset,
                AdrLiteAnchor::Function { name: name.clone() },
            ));
        }
    }
    anchors.sort_by_key(|(offset, _)| *offset);
    anchors
}

fn doc_comment_text(token: &Token) -> String {
    token
        .text
        .strip_prefix("##")
        .map(str::trim)
        .unwrap_or_default()
        .to_string()
}

fn is_adr_lite_marker(content: &str) -> bool {
    matches!(content, "@adr" | "@adr-lite")
}
