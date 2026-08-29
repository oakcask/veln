use super::*;

pub(super) struct SimilarityCandidate {
    pub(super) declaration: SimilarityDeclarationMetric,
    pub(super) tokens: Vec<NormalizedToken>,
    pub(super) fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct NormalizedToken {
    pub(super) kind: TokenKind,
    pub(super) text: String,
}

pub(super) fn similarity_instances(
    project: &Project,
    selected_paths: &BTreeSet<String>,
    min_tokens: usize,
) -> (Vec<SimilarityInstanceMetric>, usize) {
    let candidates = similarity_candidates(project, selected_paths)
        .into_iter()
        .filter(|candidate| candidate.tokens.len() >= min_tokens)
        .collect::<Vec<_>>();
    let fingerprint_count = candidates.len();
    (
        similarity_instances_from_candidates(candidates),
        fingerprint_count,
    )
}

pub(super) fn similarity_instances_from_candidates(
    candidates: Vec<SimilarityCandidate>,
) -> Vec<SimilarityInstanceMetric> {
    let mut by_fingerprint = BTreeMap::<String, Vec<SimilarityCandidate>>::new();
    for candidate in candidates {
        by_fingerprint
            .entry(candidate.fingerprint.clone())
            .or_default()
            .push(candidate);
    }

    let mut instances = Vec::new();
    for candidates in by_fingerprint.into_values() {
        let mut by_tokens = Vec::<Vec<SimilarityCandidate>>::new();
        for candidate in candidates {
            if let Some(group) = by_tokens.iter_mut().find(|group| {
                group
                    .first()
                    .is_some_and(|first| first.tokens == candidate.tokens)
            }) {
                group.push(candidate);
            } else {
                by_tokens.push(vec![candidate]);
            }
        }
        for mut group in by_tokens {
            if group.len() < 2 {
                continue;
            }
            group.sort_by(compare_similarity_candidates);
            let fingerprint = group[0].fingerprint.clone();
            let token_count = group[0].tokens.len();
            let declarations = group
                .into_iter()
                .map(|candidate| candidate.declaration)
                .collect::<Vec<_>>();
            instances.push(SimilarityInstanceMetric {
                identity: format!("similarity:{fingerprint}"),
                fingerprint,
                token_count,
                experimental: true,
                declarations,
            });
        }
    }
    instances.sort_by(compare_similarity_instances);
    instances
}

pub(super) fn similarity_candidates(
    project: &Project,
    selected_paths: &BTreeSet<String>,
) -> Vec<SimilarityCandidate> {
    let mut candidates = Vec::new();
    for source in &project.files {
        let path = source.path().as_str().to_string();
        if !selected_paths.contains(&path) || is_generated_or_doctest_path(&path) {
            continue;
        }
        let parsed = parse(source);
        if !parsed.diagnostics.is_empty() {
            continue;
        }
        let lexed = lex(source);
        for item in parsed.tree.items {
            let SyntaxItem::Function(function) = item else {
                continue;
            };
            let Some(body_range) = function_body_range(&function) else {
                continue;
            };
            let tokens = normalized_body_tokens(&lexed.tokens, body_range);
            if tokens.is_empty() {
                continue;
            }
            let declaration = similarity_declaration(source, &path, &function, body_range);
            let fingerprint = similarity_fingerprint(&tokens);
            candidates.push(SimilarityCandidate {
                declaration,
                tokens,
                fingerprint,
            });
        }
    }
    candidates
}

pub(super) fn function_body_range(function: &FunctionDecl) -> Option<TextRange> {
    function
        .body
        .iter()
        .map(body_line_range)
        .reduce(TextRange::cover)
}

pub(super) fn body_line_range(line: &BodyLine) -> TextRange {
    let span = match line {
        BodyLine::Let { span, .. } | BodyLine::Expr { span, .. } => span,
    };
    TextRange::new(span.start.offset, span.end.offset)
}

pub(super) fn normalized_body_tokens(tokens: &[Token], range: TextRange) -> Vec<NormalizedToken> {
    tokens
        .iter()
        .filter(|token| token.range.start >= range.start && token.range.end <= range.end)
        .filter(|token| {
            !matches!(
                token.kind,
                TokenKind::Whitespace | TokenKind::Comment | TokenKind::Newline | TokenKind::Eof
            )
        })
        .map(|token| NormalizedToken {
            kind: token.kind,
            text: token.text.clone(),
        })
        .collect()
}

pub(super) fn similarity_declaration(
    source: &SourceFile,
    path: &str,
    function: &FunctionDecl,
    body_range: TextRange,
) -> SimilarityDeclarationMetric {
    let kind = match function.kind {
        FunctionKind::Function => AbcSubjectKind::Function,
        FunctionKind::Test => AbcSubjectKind::Test,
    };
    let name = function
        .name
        .clone()
        .unwrap_or_else(|| "<anonymous>".to_string());
    SimilarityDeclarationMetric {
        identity: format!("{path}::{name}"),
        path: path.to_string(),
        name,
        kind,
        generated: false,
        span: source.span(TextRange::new(
            function.span.start.offset,
            function.span.end.offset,
        )),
        body_span: source.span(body_range),
    }
}

pub(super) fn similarity_fingerprint(tokens: &[NormalizedToken]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for token in tokens {
        hash = fnv1a(hash, &[token.kind as u8]);
        hash = fnv1a(hash, &[0]);
        hash = fnv1a(hash, token.text.as_bytes());
        hash = fnv1a(hash, &[0xff]);
    }
    format!("{hash:016x}")
}

pub(super) fn fnv1a(mut hash: u64, bytes: &[u8]) -> u64 {
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub(super) fn compare_similarity_candidates(
    left: &SimilarityCandidate,
    right: &SimilarityCandidate,
) -> std::cmp::Ordering {
    compare_similarity_declarations(&left.declaration, &right.declaration)
}

pub(super) fn compare_similarity_declarations(
    left: &SimilarityDeclarationMetric,
    right: &SimilarityDeclarationMetric,
) -> std::cmp::Ordering {
    left.path
        .cmp(&right.path)
        .then_with(|| left.span.start.offset.cmp(&right.span.start.offset))
        .then_with(|| left.kind.as_str().cmp(right.kind.as_str()))
}

pub(super) fn compare_similarity_instances(
    left: &SimilarityInstanceMetric,
    right: &SimilarityInstanceMetric,
) -> std::cmp::Ordering {
    right
        .token_count
        .cmp(&left.token_count)
        .then_with(|| {
            compare_similarity_declarations(&left.declarations[0], &right.declarations[0])
        })
        .then_with(|| left.fingerprint.cmp(&right.fingerprint))
}
