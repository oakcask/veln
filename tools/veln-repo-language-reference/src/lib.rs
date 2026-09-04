use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use veln_source::SourceFile;
use veln_syntax::{PUBLIC_FIXED_SPELLING_TOKENS, PUBLIC_KEYWORDS, PUBLIC_PUNCTUATION, lex};

mod descriptors;
mod examples;
mod grammar;
mod markdown;

pub use descriptors::{Descriptor, ExampleSelection};
pub use examples::{ExampleSource, RepositoryExampleSource};
pub use grammar::{GrammarSource, SwiplGrammarSource};
pub use markdown::{
    LANGUAGE_REFERENCE_MARKDOWN_MEDIA_TYPE, LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT,
    RenderedLanguageReference, RenderedResource, render_checked_language_reference,
    render_language_reference, rendered_language_reference_digest,
};

use descriptors::topic_descriptors;
#[cfg(test)]
use examples::validate_examples;
use grammar::{GrammarProduction, parse_grammar};

pub const SCHEMA_VERSION: u64 = 1;
pub const GENERATOR_CONTRACT_VERSION: u64 = 1;
pub const DIGEST_DOMAIN: &[u8] = b"veln-language-reference/v1\0";
pub const RENDERED_DIGEST_DOMAIN: &[u8] = b"veln-language-reference-markdown/v1\0";
pub const CHECKED_ARTIFACT: &str = include_str!("../generated/language-reference-catalog-v1.json");
pub const CHECKED_DIGEST: &str = include_str!("../generated/language-reference-catalog-v1.sha256");
pub const CHECKED_RENDERED_DIGEST: &str =
    include_str!("../generated/language-reference-markdown-v1.sha256");
#[cfg(test)]
const SPEC_CONTRACT: &str =
    include_str!("../../../examples/specification/language-reference/catalog-contract-v1.json");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedCatalog {
    pub bytes: String,
    pub digest: String,
    pub rendered_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshnessMismatch {
    pub artifact_matches: bool,
    pub digest_matches: bool,
    pub generated_digest: String,
    pub checked_digest: String,
}

pub struct FreshnessBaseline<'a> {
    pub artifact: &'a str,
    pub digest: &'a str,
    pub rendered_digest: &'a str,
}

pub fn generate_checked_catalog(repo_root: &Path) -> Result<GeneratedCatalog, String> {
    generate_catalog(repo_root, &SwiplGrammarSource)
}

pub fn generate_catalog(
    repo_root: &Path,
    grammar_source: &impl GrammarSource,
) -> Result<GeneratedCatalog, String> {
    generate_catalog_with_inputs(
        repo_root,
        grammar_source,
        &topic_descriptors(),
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
    )
}

pub fn generate_catalog_with_inputs(
    repo_root: &Path,
    grammar_source: &impl GrammarSource,
    descriptors: &[Descriptor],
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
) -> Result<GeneratedCatalog, String> {
    generate_catalog_with_sources(
        repo_root,
        grammar_source,
        &RepositoryExampleSource,
        descriptors,
        keywords,
        punctuation,
    )
}

pub fn generate_catalog_with_sources(
    repo_root: &Path,
    grammar_source: &impl GrammarSource,
    example_source: &impl ExampleSource,
    descriptors: &[Descriptor],
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
) -> Result<GeneratedCatalog, String> {
    let repo_root = repo_root.canonicalize().map_err(|error| {
        format!("select a readable repository root before generating the language-reference catalog: {error}")
    })?;
    let grammar_text = grammar_source.complete_grammar(&repo_root)?;
    let grammar = parse_grammar(&grammar_text)?;
    validate_descriptors(descriptors, &grammar)?;
    let examples = example_source.selected_examples(&repo_root, descriptors)?;
    validate_token_projection(keywords, punctuation)?;

    let catalog = catalog_value(
        descriptors,
        &grammar_text,
        &grammar,
        &examples,
        keywords,
        punctuation,
    )?;
    let bytes = canonical_json(&catalog)?;
    let digest = catalog_digest(bytes.as_bytes());
    let rendered = render_language_reference(&bytes, &digest)?;
    let rendered_digest = rendered_language_reference_digest(&rendered);
    Ok(GeneratedCatalog {
        bytes,
        digest,
        rendered_digest,
    })
}

pub fn checked_catalog_bytes() -> &'static str {
    CHECKED_ARTIFACT
}

pub fn checked_catalog_digest() -> &'static str {
    CHECKED_DIGEST.trim()
}

pub fn checked_rendered_digest() -> &'static str {
    CHECKED_RENDERED_DIGEST.trim()
}

pub fn verify_checked_digest() -> Result<(), String> {
    let expected = catalog_digest(CHECKED_ARTIFACT.as_bytes());
    if expected != checked_catalog_digest() {
        return Err(format!(
            "regenerate the language-reference catalog digest; checked digest is {}, generated digest is {}",
            checked_catalog_digest(),
            expected
        ));
    }
    let rendered = render_language_reference(CHECKED_ARTIFACT, checked_catalog_digest())?;
    let expected_rendered = rendered_language_reference_digest(&rendered);
    if expected_rendered != checked_rendered_digest() {
        return Err(format!(
            "regenerate the language-reference Markdown digest; checked digest is {}, generated digest is {}",
            checked_rendered_digest(),
            expected_rendered
        ));
    }
    Ok(())
}

pub fn verify_freshness(repo_root: &Path) -> Result<(), FreshnessMismatch> {
    verify_freshness_against(
        repo_root,
        &SwiplGrammarSource,
        &topic_descriptors(),
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        CHECKED_ARTIFACT,
        checked_catalog_digest(),
    )
}

pub fn verify_freshness_against(
    repo_root: &Path,
    grammar_source: &impl GrammarSource,
    descriptors: &[Descriptor],
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
    checked_artifact: &str,
    checked_digest: &str,
) -> Result<(), FreshnessMismatch> {
    verify_freshness_against_sources(
        repo_root,
        grammar_source,
        &RepositoryExampleSource,
        descriptors,
        keywords,
        punctuation,
        FreshnessBaseline {
            artifact: checked_artifact,
            digest: checked_digest,
            rendered_digest: checked_rendered_digest(),
        },
    )
}

pub fn verify_freshness_against_sources(
    repo_root: &Path,
    grammar_source: &impl GrammarSource,
    example_source: &impl ExampleSource,
    descriptors: &[Descriptor],
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
    baseline: FreshnessBaseline<'_>,
) -> Result<(), FreshnessMismatch> {
    let generated = generate_catalog_with_sources(
        repo_root,
        grammar_source,
        example_source,
        descriptors,
        keywords,
        punctuation,
    )
    .map_err(|message| FreshnessMismatch {
        artifact_matches: false,
        digest_matches: false,
        generated_digest: message,
        checked_digest: baseline.digest.to_string(),
    })?;
    let artifact_matches = generated.bytes == baseline.artifact;
    let digest_matches = generated.digest == baseline.digest;
    if let Err(message) = render_language_reference(&generated.bytes, &generated.digest) {
        return Err(FreshnessMismatch {
            artifact_matches,
            digest_matches: false,
            generated_digest: message,
            checked_digest: baseline.digest.to_string(),
        });
    }
    let rendered_matches = generated.rendered_digest == baseline.rendered_digest;
    if artifact_matches && digest_matches && rendered_matches {
        Ok(())
    } else {
        Err(FreshnessMismatch {
            artifact_matches,
            digest_matches: digest_matches && rendered_matches,
            generated_digest: format!("{}/{}", generated.digest, generated.rendered_digest),
            checked_digest: format!("{}/{}", baseline.digest, baseline.rendered_digest),
        })
    }
}

pub fn write_checked_outputs(repo_root: &Path, generated: &GeneratedCatalog) -> Result<(), String> {
    let output_dir = repo_root.join("tools/veln-repo-language-reference/generated");
    fs::create_dir_all(&output_dir).map_err(|error| {
        format!(
            "create the language-reference generated output directory before writing checked artifacts: {error}"
        )
    })?;
    fs::write(
        output_dir.join("language-reference-catalog-v1.json"),
        &generated.bytes,
    )
    .map_err(|error| format!("write the checked language-reference catalog: {error}"))?;
    fs::write(
        output_dir.join("language-reference-catalog-v1.sha256"),
        format!("{}\n", generated.digest),
    )
    .map_err(|error| format!("write the checked language-reference digest: {error}"))?;
    fs::write(
        output_dir.join("language-reference-markdown-v1.sha256"),
        format!("{}\n", generated.rendered_digest),
    )
    .map_err(|error| format!("write the checked language-reference Markdown digest: {error}"))?;
    Ok(())
}

fn catalog_value(
    descriptors: &[Descriptor],
    complete_grammar: &str,
    grammar: &[GrammarProduction],
    examples: &BTreeMap<String, BTreeMap<String, String>>,
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
) -> Result<Value, String> {
    let grammar_index = grammar
        .iter()
        .map(|production| (production.name.as_str(), production))
        .collect::<BTreeMap<_, _>>();
    let topics = descriptors
        .iter()
        .map(|descriptor| {
            let selected_grammar = descriptor
                .grammar
                .iter()
                .map(|name| {
                    let production = grammar_index
                        .get(name)
                        .expect("descriptor grammar was validated");
                    json!({
                        "name": production.name,
                        "text": production.lines.join("\n"),
                    })
                })
                .collect::<Vec<_>>();
            let selected_examples = descriptor
                .examples
                .iter()
                .map(|example| {
                    let files = examples
                        .get(example.case)
                        .expect("example case was validated");
                    let displayed_files = example
                        .files
                        .iter()
                        .map(|file| {
                            json!({
                                "path": normalize_catalog_text(file),
                                "source": files.get(*file).expect("example file was validated"),
                            })
                        })
                        .collect::<Vec<_>>();
                    json!({
                        "case": normalize_catalog_text(example.case),
                        "display_name": normalize_catalog_text(example.display_name),
                        "files": displayed_files,
                    })
                })
                .collect::<Vec<_>>();
            json!({
                "body": normalize_lines(descriptor.body),
                "examples": selected_examples,
                "grammar": selected_grammar,
                "id": normalize_catalog_text(descriptor.id),
                "keywords": normalized_set(descriptor.keywords),
                "related": normalized_set(descriptor.related),
                "summary": normalize_catalog_text(descriptor.summary),
                "title": normalize_catalog_text(descriptor.title),
            })
        })
        .collect::<Vec<_>>();

    Ok(json!({
        "generator_contract_version": GENERATOR_CONTRACT_VERSION,
        "grammar": {
            "complete": normalize_source_text(complete_grammar),
        },
        "public_tokens": {
            "keywords": token_values(keywords),
            "punctuation": token_values(punctuation),
        },
        "schema_version": SCHEMA_VERSION,
        "topics": topics,
    }))
}

fn token_values(tokens: &[veln_syntax::PublicToken]) -> Vec<Value> {
    let mut values = tokens
        .iter()
        .map(|token| {
            json!({
                "kind": format!("{:?}", token.kind),
                "spelling": token.spelling,
            })
        })
        .collect::<Vec<_>>();
    values.sort_by(|left, right| {
        left["spelling"]
            .as_str()
            .unwrap()
            .cmp(right["spelling"].as_str().unwrap())
    });
    values
}

fn validate_descriptors(
    descriptors: &[Descriptor],
    grammar: &[GrammarProduction],
) -> Result<(), String> {
    let grammar_names = grammar
        .iter()
        .map(|production| production.name.as_str())
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    for descriptor in descriptors {
        validate_descriptor_identity(descriptor, &mut ids)?;
        validate_descriptor_metadata(descriptor)?;
        validate_descriptor_grammar(descriptor, &grammar_names)?;
        validate_descriptor_examples(descriptor)?;
    }
    validate_descriptor_relations(descriptors, &ids)
}

fn validate_descriptor_identity<'a>(
    descriptor: &'a Descriptor,
    ids: &mut BTreeSet<&'a str>,
) -> Result<(), String> {
    if !valid_identifier(descriptor.id) {
        return Err(format!(
            "topic identifiers must be lowercase ASCII identifiers: {}",
            descriptor.id
        ));
    }
    if !ids.insert(descriptor.id) {
        return Err(format!("duplicate topic identifier `{}`", descriptor.id));
    }
    Ok(())
}

fn validate_descriptor_metadata(descriptor: &Descriptor) -> Result<(), String> {
    reject_empty(descriptor.id, "title", descriptor.title)?;
    reject_empty(descriptor.id, "summary", descriptor.summary)?;
    reject_empty_set(descriptor.id, "keywords", descriptor.keywords)?;
    reject_empty_set(descriptor.id, "body", descriptor.body)?;
    reject_empty_set(descriptor.id, "grammar", descriptor.grammar)?;
    reject_duplicate_normalized_set(descriptor.id, "keywords", descriptor.keywords)?;
    reject_duplicate_normalized_set(descriptor.id, "related", descriptor.related)?;
    reject_duplicate_normalized_set(descriptor.id, "grammar", descriptor.grammar)
}

fn validate_descriptor_grammar(
    descriptor: &Descriptor,
    grammar_names: &BTreeSet<&str>,
) -> Result<(), String> {
    for name in descriptor.grammar {
        if !grammar_names.contains(name) {
            return Err(format!(
                "topic `{}` selects unknown grammar production `{name}`",
                descriptor.id
            ));
        }
    }
    Ok(())
}

fn validate_descriptor_examples(descriptor: &Descriptor) -> Result<(), String> {
    for example in descriptor.examples {
        reject_empty(descriptor.id, "example display_name", example.display_name)?;
        reject_empty(descriptor.id, "example case", example.case)?;
        reject_repository_relative(descriptor.id, "example case", example.case)?;
        reject_empty_set(descriptor.id, "example files", example.files)?;
        reject_duplicate_normalized_set(descriptor.id, "example files", example.files)?;
        for file in example.files {
            reject_repository_relative(descriptor.id, "example file", file)?;
        }
    }
    Ok(())
}

fn validate_descriptor_relations(
    descriptors: &[Descriptor],
    ids: &BTreeSet<&str>,
) -> Result<(), String> {
    for descriptor in descriptors {
        for related in descriptor.related {
            if *related == descriptor.id {
                return Err(format!(
                    "topic `{}` must not relate to itself",
                    descriptor.id
                ));
            }
            if !ids.contains(related) {
                return Err(format!(
                    "topic `{}` relates to unknown topic `{related}`",
                    descriptor.id
                ));
            }
        }
    }
    Ok(())
}

fn validate_token_projection(
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
) -> Result<(), String> {
    reject_duplicate_tokens("public keyword", keywords)?;
    reject_duplicate_tokens("public punctuation", punctuation)?;
    let records = keywords
        .iter()
        .chain(punctuation.iter())
        .map(|token| ((format!("{:?}", token.kind), token.spelling), token))
        .collect::<BTreeMap<_, _>>();
    for token in PUBLIC_FIXED_SPELLING_TOKENS {
        if !records.contains_key(&(format!("{:?}", token.kind), token.spelling)) {
            return Err(format!(
                "public fixed-spelling token `{}` ({:?}) is missing from the catalog projection",
                token.spelling, token.kind
            ));
        }
    }
    for token in keywords {
        let source = SourceFile::new("tokens.veln", token.spelling.to_string());
        let first = lex(&source).tokens[0].kind;
        if first != token.kind {
            return Err(format!(
                "public keyword `{}` projects {:?} but the lexer recognized {:?}",
                token.spelling, token.kind, first
            ));
        }
    }
    for token in punctuation {
        let source = SourceFile::new("tokens.veln", token.spelling.to_string());
        let first = lex(&source).tokens[0].kind;
        if first != token.kind {
            return Err(format!(
                "public punctuation `{}` projects {:?} but the lexer recognized {:?}",
                token.spelling, token.kind, first
            ));
        }
    }
    Ok(())
}

fn catalog_digest(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(DIGEST_DOMAIN);
    digest.update((bytes.len() as u64).to_be_bytes());
    digest.update(bytes);
    hex_lower(&digest.finalize())
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn canonical_json(value: &Value) -> Result<String, String> {
    let mut out = serde_json::to_string(value)
        .map_err(|error| format!("serialize canonical schema-v1 catalog JSON: {error}"))?;
    out.push('\n');
    Ok(out)
}

fn normalized_set(values: &[&str]) -> Vec<String> {
    let mut values = values
        .iter()
        .map(|value| normalize_catalog_text(value))
        .collect::<Vec<_>>();
    values.sort();
    values
}

fn normalize_lines(values: &[&str]) -> Vec<String> {
    values
        .iter()
        .map(|value| normalize_catalog_text(value))
        .collect()
}

fn normalize_catalog_text(text: &str) -> String {
    normalize_source_text(text).nfc().collect()
}

fn normalize_source_text(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    if text.ends_with('\n') {
        text
    } else {
        format!("{text}\n").trim_end_matches('\n').to_string()
    }
}

fn reject_empty(topic: &str, field: &str, value: &str) -> Result<(), String> {
    if normalize_catalog_text(value).trim().is_empty() {
        Err(format!(
            "topic `{topic}` has empty required field `{field}`"
        ))
    } else {
        Ok(())
    }
}

fn reject_empty_set(topic: &str, field: &str, values: &[&str]) -> Result<(), String> {
    if values.is_empty() {
        Err(format!("topic `{topic}` has empty required set `{field}`"))
    } else {
        for value in values {
            reject_empty(topic, field, value)?;
        }
        Ok(())
    }
}

fn reject_duplicate_normalized_set(
    topic: &str,
    field: &str,
    values: &[&str],
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        let normalized = normalize_catalog_text(value);
        if !seen.insert(normalized.clone()) {
            return Err(format!(
                "topic `{topic}` repeats normalized `{field}` value `{normalized}`"
            ));
        }
    }
    Ok(())
}

fn reject_repository_relative(topic: &str, field: &str, value: &str) -> Result<(), String> {
    let path = Path::new(value);
    if repository_relative(path) {
        Ok(())
    } else {
        Err(format!(
            "topic `{topic}` has non-repository-relative `{field}` value `{value}`"
        ))
    }
}

fn reject_duplicate_tokens(label: &str, tokens: &[veln_syntax::PublicToken]) -> Result<(), String> {
    let mut spellings = BTreeSet::new();
    let mut kinds = BTreeSet::new();
    for token in tokens {
        if !spellings.insert(token.spelling) {
            return Err(format!(
                "{label} records repeat spelling `{}`",
                token.spelling
            ));
        }
        if !kinds.insert(format!("{:?}", token.kind)) {
            return Err(format!("{label} records repeat kind `{:?}`", token.kind));
        }
    }
    Ok(())
}

fn valid_identifier(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        && id.chars().next().is_some_and(|ch| ch.is_ascii_lowercase())
        && id.chars().last().is_some_and(|ch| ch.is_ascii_lowercase())
}

fn slash_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn repository_relative(path: &Path) -> bool {
    !path.is_absolute()
        && path.components().all(|component| {
            !matches!(
                component,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
}

#[cfg(test)]
mod tests;
