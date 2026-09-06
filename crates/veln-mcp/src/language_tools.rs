use serde_json::{Value, json};
use veln_project::portable_normalized_case_fold;

use crate::language_resources::{LanguageResources, LanguageTopic, PackageSearchCandidate};
use crate::outcome::ToolOutcome;
use crate::schema;

const DEFAULT_LIMIT: usize = 10;
const EXCERPT_LIMIT: usize = 160;

pub(crate) fn search_docs(resources: &LanguageResources, arguments: &Value) -> ToolOutcome {
    let query = arguments["query"]
        .as_str()
        .expect("search_docs schema requires a string query");
    let scope = arguments
        .get("scope")
        .and_then(Value::as_str)
        .unwrap_or("language");
    let limit = arguments
        .get("limit")
        .and_then(schema::json_integer_usize)
        .unwrap_or(DEFAULT_LIMIT);

    let normalized_query = normalize_search_text(query);
    let tokens = normalized_query
        .split_whitespace()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let mut results = Vec::new();
    if matches!(scope, "language" | "all") {
        results.extend(
            resources
                .topics()
                .iter()
                .filter_map(|topic| ranked_language_match(topic, &normalized_query, &tokens)),
        );
    }
    if matches!(scope, "stdlib" | "package" | "all") {
        results.extend(
            resources
                .package_search_candidates()
                .iter()
                .filter(|candidate| match scope {
                    "stdlib" => {
                        candidate.scope
                            == crate::language_resources::PackageSearchScope::StandardLibrary
                    }
                    "package" => {
                        candidate.scope == crate::language_resources::PackageSearchScope::Package
                    }
                    "all" => true,
                    _ => false,
                })
                .filter_map(|candidate| {
                    ranked_package_match(candidate, &normalized_query, &tokens)
                }),
        );
    }
    results.sort_by(|left, right| {
        left.rank
            .cmp(&right.rank)
            .then_with(|| left.uri.as_bytes().cmp(right.uri.as_bytes()))
    });
    ToolOutcome::Success(json!({
        "scope": scope,
        "results": results.into_iter().take(limit).map(|result| {
            json!({
                "uri": result.uri,
                "title": result.title,
                "summary": result.summary,
                "excerpt": result.excerpt.text,
                "prefix_truncated": result.excerpt.prefix_truncated,
                "suffix_truncated": result.excerpt.suffix_truncated,
            })
        }).collect::<Vec<_>>()
    }))
}

pub(crate) fn read_doc(resources: &LanguageResources, arguments: &Value) -> ToolOutcome {
    let uri = arguments["uri"]
        .as_str()
        .expect("read_doc schema requires a string URI");
    match resources.read_doc_result(uri) {
        Some(result) => ToolOutcome::Success(result),
        None => ToolOutcome::DomainFailure {
            code: "resource_not_found",
            message: "language reference resource not found",
            details: json!({"uri": uri}),
        },
    }
}

fn ranked_language_match(
    topic: &LanguageTopic,
    query: &str,
    tokens: &[String],
) -> Option<SearchResult> {
    for rank in 1..=5 {
        let fields = language_tier_fields(topic, rank);
        if tier_matches(rank, query, tokens, &fields) {
            let excerpt = first_excerpt(&fields, tokens);
            return Some(SearchResult {
                rank,
                uri: topic.uri.clone(),
                title: topic.title.clone(),
                summary: topic.summary.clone(),
                excerpt,
            });
        }
    }
    None
}

fn ranked_package_match(
    candidate: &PackageSearchCandidate,
    query: &str,
    tokens: &[String],
) -> Option<SearchResult> {
    for rank in 1..=5 {
        let fields = package_tier_fields(candidate, rank);
        if tier_matches(rank, query, tokens, &fields) {
            let excerpt = first_excerpt(&fields, tokens);
            return Some(SearchResult {
                rank,
                uri: candidate.uri.clone(),
                title: candidate.title.clone(),
                summary: candidate.summary.clone(),
                excerpt,
            });
        }
    }
    None
}

fn tier_matches(rank: u8, query: &str, tokens: &[String], fields: &[&str]) -> bool {
    match rank {
        1 => fields
            .iter()
            .any(|field| normalize_search_text(field) == query),
        2 => fields
            .iter()
            .any(|field| normalize_search_text(field).starts_with(query)),
        _ => tokens.iter().all(|token| {
            fields
                .iter()
                .any(|field| normalize_search_text(field).contains(token))
        }),
    }
}

fn language_tier_fields(topic: &LanguageTopic, rank: u8) -> Vec<&str> {
    match rank {
        1 | 2 => vec![topic.id.as_str(), topic.title.as_str()],
        3 => std::iter::once(topic.title.as_str())
            .chain(topic.keywords.iter().map(String::as_str))
            .collect(),
        4 => vec![topic.summary.as_str()],
        5 => vec![topic.body.as_str()],
        _ => unreachable!("search tier is bounded"),
    }
}

fn package_tier_fields(candidate: &PackageSearchCandidate, rank: u8) -> Vec<&str> {
    match rank {
        1 | 2 => vec![candidate.identifier.as_str(), candidate.title.as_str()],
        3 => std::iter::once(candidate.title.as_str())
            .chain(std::iter::once(candidate.name.as_str()))
            .chain(candidate.keywords.iter().map(String::as_str))
            .collect(),
        4 => std::iter::once(candidate.summary.as_str())
            .chain(candidate.signature.iter().map(String::as_str))
            .collect(),
        5 => candidate.documentation.iter().map(String::as_str).collect(),
        _ => unreachable!("search tier is bounded"),
    }
}

fn first_excerpt(fields: &[&str], tokens: &[String]) -> Excerpt {
    for field in fields {
        if let Some(span) = first_token_span(field, tokens) {
            return excerpt(field, span);
        }
    }
    excerpt(fields[0], 0..0)
}

fn first_token_span(field: &str, tokens: &[String]) -> Option<std::ops::Range<usize>> {
    let folded = folded_scalar_spans(field);
    tokens
        .iter()
        .filter_map(|token| {
            let start = folded.text.find(token)?;
            let end = start + token.len();
            let first = folded.spans.iter().position(|span| span.end > start)?;
            let last = folded.spans.iter().rposition(|span| span.start < end)?;
            Some(first..last + 1)
        })
        .min_by_key(|span| span.start)
}

fn folded_scalar_spans(field: &str) -> FoldedScalarSpans {
    let mut text = String::new();
    let mut spans = Vec::new();
    for character in field.chars() {
        let start = text.len();
        text.push_str(&normalize_search_text(&character.to_string()));
        spans.push(start..text.len());
    }
    FoldedScalarSpans { text, spans }
}

fn excerpt(field: &str, matched_scalars: std::ops::Range<usize>) -> Excerpt {
    let scalars = field.chars().collect::<Vec<_>>();
    if scalars.len() <= EXCERPT_LIMIT {
        return Excerpt {
            text: field.to_string(),
            prefix_truncated: false,
            suffix_truncated: false,
        };
    }
    let start = if matched_scalars.len() <= EXCERPT_LIMIT {
        matched_scalars
            .start
            .min(scalars.len().saturating_sub(EXCERPT_LIMIT))
    } else {
        matched_scalars.start
    };
    let end = (start + EXCERPT_LIMIT).min(scalars.len());
    Excerpt {
        text: scalars[start..end].iter().collect(),
        prefix_truncated: start > 0,
        suffix_truncated: end < scalars.len(),
    }
}

fn normalize_search_text(text: &str) -> String {
    portable_normalized_case_fold(text).trim().to_string()
}

struct SearchResult {
    rank: u8,
    uri: String,
    title: String,
    summary: String,
    excerpt: Excerpt,
}

struct Excerpt {
    text: String,
    prefix_truncated: bool,
    suffix_truncated: bool,
}

struct FoldedScalarSpans {
    text: String,
    spans: Vec<std::ops::Range<usize>>,
}
