use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};
use std::process::Command;

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use veln_syntax::TokenKind;

const DIGEST_DOMAIN: &[u8] = b"veln-language-reference/v1\0";
const ARTIFACT_PATH: &str =
    "tools/veln-repo-language-reference/artifacts/language-reference-v1.json";
const DIGEST_PATH: &str =
    "tools/veln-repo-language-reference/artifacts/language-reference-v1.sha256";

pub const CHECKED_ARTIFACT: &[u8] = include_bytes!("../artifacts/language-reference-v1.json");
pub const CHECKED_DIGEST: &str = include_str!("../artifacts/language-reference-v1.sha256");

#[derive(Clone, Copy, Debug)]
struct ExampleDescriptor {
    case: &'static str,
    file: &'static str,
    name: &'static str,
}

#[derive(Clone, Copy, Debug)]
struct TopicDescriptor {
    id: &'static str,
    title: &'static str,
    summary: &'static str,
    keywords: &'static [&'static str],
    body: &'static str,
    related: &'static [&'static str],
    grammar_rules: &'static [&'static str],
    complete_grammar: bool,
    examples: &'static [ExampleDescriptor],
}

const TOPICS: &[TopicDescriptor] = &[
    TopicDescriptor {
        id: "lexical-structure",
        title: "Lexical structure and grammar",
        summary: "Reserved source spellings and the complete executable grammar.",
        keywords: &["grammar", "keywords", "operators", "syntax"],
        body: "The executable grammar and compiler-owned token tables define the public source spellings.",
        related: &[
            "expressions-operators-patterns",
            "tests-doc-comments-doctests",
        ],
        grammar_rules: &[],
        complete_grammar: true,
        examples: &[ExampleDescriptor {
            case: "check/source-surface",
            file: "main.veln",
            name: "source surface",
        }],
    },
    TopicDescriptor {
        id: "modules-packages-visibility",
        title: "Modules, packages, and visibility",
        summary: "Module paths, imports, packages, exports, and visibility boundaries.",
        keywords: &["exports", "imports", "modules", "packages", "visibility"],
        body: "Imports name module paths and public declarations define the visible package surface.",
        related: &["declarations-aliases", "types-inference-constructors"],
        grammar_rules: &["Module", "UseDecl", "ImportSource", "ModulePath"],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/module-imports",
            file: "app.veln",
            name: "module imports",
        }],
    },
    TopicDescriptor {
        id: "declarations-aliases",
        title: "Declarations and aliases",
        summary: "Functions, public declarations, and member aliases.",
        keywords: &["aliases", "declarations", "functions", "public"],
        body: "Declarations introduce named program items and public aliases expose selected members.",
        related: &[
            "modules-packages-visibility",
            "types-inference-constructors",
        ],
        grammar_rules: &["Item", "Function", "PublicAlias"],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/source-surface",
            file: "main.veln",
            name: "declaration surface",
        }],
    },
    TopicDescriptor {
        id: "expressions-operators-patterns",
        title: "Expressions, operators, and patterns",
        summary: "Expression composition, operator forms, conditional branches, and patterns.",
        keywords: &["expressions", "if", "operators", "patterns"],
        body: "Expressions compose values. Patterns bind or select values at supported pattern positions.",
        related: &["lexical-structure", "types-inference-constructors"],
        grammar_rules: &[
            "Expr",
            "BinaryOp",
            "PrefixExpr",
            "PrimaryExpr",
            "If",
            "Pattern",
        ],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/if-expression-syntax",
            file: "main.veln",
            name: "if expressions",
        }],
    },
    TopicDescriptor {
        id: "types-inference-constructors",
        title: "Types, inference, and constructors",
        summary: "Type declarations, inference boundaries, variants, and constructors.",
        keywords: &["constructors", "inference", "types", "variants"],
        body: "Type declarations define variants and constructors. Type checking applies the documented inference boundaries.",
        related: &["declarations-aliases", "expressions-operators-patterns"],
        grammar_rules: &[
            "TypeDecl",
            "TypeParamList",
            "TypeVariant",
            "TypeVariantFields",
        ],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/source-adt-boundaries",
            file: "library.veln",
            name: "source type boundaries",
        }],
    },
    TopicDescriptor {
        id: "effects-handlers",
        title: "Effects and handlers",
        summary: "Effect declarations, effect rows, operations, and lexical handlers.",
        keywords: &["effects", "handlers", "operations"],
        body: "Effect rows describe required effects. Handlers provide explicit clauses for handled operations.",
        related: &["declarations-aliases", "contracts"],
        grammar_rules: &[
            "EffectDecl",
            "EffectOperation",
            "HandlerDecl",
            "HandlerOperationClause",
            "Effects",
        ],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/handler-operation-signatures",
            file: "main.veln",
            name: "handler operation signatures",
        }],
    },
    TopicDescriptor {
        id: "contracts",
        title: "Contracts",
        summary: "Preconditions, postconditions, invariants, and result bindings.",
        keywords: &["contracts", "ensure", "invariant", "require"],
        body: "Contracts state checked predicates at declaration boundaries.",
        related: &["effects-handlers", "holes"],
        grammar_rules: &["Contract", "Return", "ResultBinding"],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/contracts-result-binding",
            file: "main.veln",
            name: "contract result binding",
        }],
    },
    TopicDescriptor {
        id: "schemas",
        title: "Schemas",
        summary: "Schema declarations, binary fields, validation, encoding, and decoding.",
        keywords: &["binary", "decode", "encode", "schemas"],
        body: "Schemas describe structured values and, when formatted as binary, their checked byte layout.",
        related: &["types-inference-constructors", "contracts"],
        grammar_rules: &[
            "SchemaDecl",
            "SchemaFormat",
            "SchemaField",
            "SchemaFieldType",
            "SchemaDecode",
            "SchemaEncode",
        ],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/schema-composition-grammar-precedence",
            file: "binary.veln",
            name: "binary schema composition",
        }],
    },
    TopicDescriptor {
        id: "holes",
        title: "Holes",
        summary: "Typed holes, labels, constraints, and diagnostics.",
        keywords: &["constraints", "diagnostics", "holes", "satisfy"],
        body: "A hole marks an incomplete expression and preserves its optional label in diagnostics.",
        related: &["contracts", "types-inference-constructors"],
        grammar_rules: &["PrimaryExpr"],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/named-hole-labels",
            file: "main.veln",
            name: "named holes",
        }],
    },
    TopicDescriptor {
        id: "tests-doc-comments-doctests",
        title: "Tests, documentation comments, and doctests",
        summary: "Test declarations and checked documentation examples.",
        keywords: &["documentation", "doctests", "tests"],
        body: "Test declarations and visible documentation fences are selected by the shared toolchain harness.",
        related: &["lexical-structure", "declarations-aliases"],
        grammar_rules: &["TestDecl"],
        complete_grammar: false,
        examples: &[ExampleDescriptor {
            case: "check/doctest-static-examples",
            file: "main.veln",
            name: "static doctest examples",
        }],
    },
];

pub struct GeneratedArtifacts {
    pub artifact: Vec<u8>,
    pub digest: String,
}

#[derive(Clone)]
struct Grammar {
    complete: String,
    rules: BTreeMap<String, Vec<String>>,
}

pub fn generate_from_workspace(workspace: &Path) -> Result<GeneratedArtifacts, String> {
    let grammar = executable_grammar(workspace)?;
    generate(workspace, TOPICS, &grammar)
}

pub fn verify_checked_artifacts(
    workspace: &Path,
    generated: &GeneratedArtifacts,
) -> Result<(), String> {
    let artifact = fs::read(workspace.join(ARTIFACT_PATH))
        .map_err(|error| format!("could not read {ARTIFACT_PATH}: {error}"))?;
    if artifact != generated.artifact {
        return Err("artifact input changed; run the generate route".to_string());
    }
    let digest = fs::read_to_string(workspace.join(DIGEST_PATH))
        .map_err(|error| format!("could not read {DIGEST_PATH}: {error}"))?;
    if digest != format!("{}\n", generated.digest) {
        return Err("digest does not match the canonical artifact".to_string());
    }
    Ok(())
}

pub fn write_checked_artifacts(
    workspace: &Path,
    generated: &GeneratedArtifacts,
) -> Result<(), String> {
    fs::write(workspace.join(ARTIFACT_PATH), &generated.artifact)
        .map_err(|error| format!("could not write {ARTIFACT_PATH}: {error}"))?;
    fs::write(
        workspace.join(DIGEST_PATH),
        format!("{}\n", generated.digest),
    )
    .map_err(|error| format!("could not write {DIGEST_PATH}: {error}"))
}

fn executable_grammar(workspace: &Path) -> Result<Grammar, String> {
    let spec = "docs/specification/source-surface-executable.pl";
    let output = Command::new("swipl")
        .args(["-q", "-s", spec, "--", "--grammar"])
        .current_dir(workspace)
        .output()
        .map_err(|error| format!("could not execute SWI-Prolog for {spec}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "SWI-Prolog grammar generation failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let complete = String::from_utf8(output.stdout)
        .map_err(|error| format!("executable grammar was not UTF-8: {error}"))?;
    Grammar::parse(&complete)
}

impl Grammar {
    fn parse(output: &str) -> Result<Self, String> {
        let complete = normalize_text(output);
        if complete.is_empty() || !complete.ends_with('\n') {
            return Err("executable grammar output must be nonempty and LF-terminated".to_string());
        }
        let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut current = None;
        for line in complete.lines() {
            if let Some((name, _)) = line.split_once("::=") {
                let name = name.trim();
                if name.is_empty() || name.contains(char::is_whitespace) {
                    return Err(format!("invalid executable grammar rule name `{name}`"));
                }
                current = Some(name.to_string());
                rules
                    .entry(name.to_string())
                    .or_default()
                    .push(line.to_string());
            } else if line.starts_with(' ') {
                let Some(name) = current.as_ref() else {
                    return Err("grammar continuation appeared before a named rule".to_string());
                };
                rules
                    .get_mut(name)
                    .expect("current rule should exist")
                    .push(line.to_string());
            } else {
                return Err(format!("grammar line has no production name: {line}"));
            }
        }
        Ok(Self { complete, rules })
    }
}

fn generate(
    workspace: &Path,
    topics: &[TopicDescriptor],
    grammar: &Grammar,
) -> Result<GeneratedArtifacts, String> {
    validate_descriptors(workspace, topics, grammar)?;
    let artifact = serialize_catalog(workspace, topics, grammar)?;
    let mut transcript = Vec::with_capacity(DIGEST_DOMAIN.len() + 8 + artifact.len());
    transcript.extend_from_slice(DIGEST_DOMAIN);
    transcript.extend_from_slice(&(artifact.len() as u64).to_be_bytes());
    transcript.extend_from_slice(&artifact);
    let digest = Sha256::digest(&transcript);
    let digest = hex_digest(&digest);
    Ok(GeneratedArtifacts { artifact, digest })
}

fn validate_descriptors(
    workspace: &Path,
    topics: &[TopicDescriptor],
    grammar: &Grammar,
) -> Result<(), String> {
    let mut ids = BTreeSet::new();
    for topic in topics {
        if !valid_topic_id(topic.id) {
            return Err(format!("invalid topic id `{}`", topic.id));
        }
        if !ids.insert(topic.id) {
            return Err(format!("duplicate topic id `{}`", topic.id));
        }
        if [topic.title, topic.summary, topic.body]
            .iter()
            .any(|value| value.trim().is_empty())
            || topic.keywords.is_empty()
        {
            return Err(format!("topic `{}` is missing required metadata", topic.id));
        }
    }
    for topic in topics {
        let mut relations = BTreeSet::new();
        for relation in topic.related {
            if *relation == topic.id || !ids.contains(relation) || !relations.insert(*relation) {
                return Err(format!(
                    "topic `{}` has invalid relation `{relation}`",
                    topic.id
                ));
            }
        }
        let mut selected_rules = BTreeSet::new();
        for rule in topic.grammar_rules {
            if !grammar.rules.contains_key(*rule) || !selected_rules.insert(*rule) {
                return Err(format!(
                    "topic `{}` selects invalid grammar rule `{rule}`",
                    topic.id
                ));
            }
        }
        for example in topic.examples {
            validate_example(workspace, topic.id, example)?;
        }
    }
    Ok(())
}

fn valid_topic_id(id: &str) -> bool {
    !id.is_empty()
        && !id.starts_with('-')
        && !id.ends_with('-')
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn validate_example(
    workspace: &Path,
    topic_id: &str,
    example: &ExampleDescriptor,
) -> Result<(), String> {
    for value in [example.case, example.file] {
        if Path::new(value).is_absolute()
            || Path::new(value)
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "topic `{topic_id}` selects invalid example path `{value}`"
            ));
        }
    }
    let case_root = workspace.join("examples/specification").join(example.case);
    let manifest_path = case_root.join("case.toml");
    let manifest = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "topic `{topic_id}` selects missing case `{}`: {error}",
            example.case
        )
    })?;
    let source_path = case_root.join(example.file);
    if !source_path.is_file() {
        return Err(format!(
            "topic `{topic_id}` selects missing case file `{}/{}`",
            example.case, example.file
        ));
    }
    if source_path.extension().and_then(|value| value.to_str()) != Some("veln") {
        return Err(format!(
            "topic `{topic_id}` selects a non-Veln display file"
        ));
    }
    if !manifest.contains(&format!("\"{}\"", example.file)) {
        return Err(format!(
            "topic `{topic_id}` selects `{}` outside the case command target",
            example.file
        ));
    }
    Ok(())
}

fn serialize_catalog(
    workspace: &Path,
    topics: &[TopicDescriptor],
    grammar: &Grammar,
) -> Result<Vec<u8>, String> {
    let mut out = String::new();
    out.push_str(
        "{\"generator_contract_version\":1,\"schema_version\":1,\"tables\":{\"keywords\":[",
    );
    write_token_table(&mut out, TokenKind::KEYWORDS);
    out.push_str("],\"punctuation\":[");
    write_token_table(&mut out, TokenKind::PUNCTUATION);
    out.push_str("]},\"topics\":[");
    for (topic_index, topic) in topics.iter().enumerate() {
        comma(&mut out, topic_index);
        out.push_str("{\"body\":");
        write_json_string(&mut out, &normalize_text(topic.body));
        out.push_str(",\"examples\":[");
        for (example_index, example) in topic.examples.iter().enumerate() {
            comma(&mut out, example_index);
            let source = fs::read_to_string(
                workspace
                    .join("examples/specification")
                    .join(example.case)
                    .join(example.file),
            )
            .map_err(|error| format!("could not read selected example: {error}"))?;
            out.push_str("{\"name\":");
            write_json_string(&mut out, &normalize_text(example.name));
            out.push_str(",\"source\":");
            write_json_string(&mut out, &normalize_source(&source));
            out.push('}');
        }
        out.push_str("],\"grammar\":[");
        let blocks = grammar_blocks(topic, grammar);
        for (index, block) in blocks.iter().enumerate() {
            comma(&mut out, index);
            write_json_string(&mut out, block);
        }
        out.push_str("],\"id\":");
        write_json_string(&mut out, topic.id);
        out.push_str(",\"keywords\":[");
        write_strings(&mut out, topic.keywords);
        out.push_str("],\"related\":[");
        write_strings(&mut out, topic.related);
        out.push_str("],\"summary\":");
        write_json_string(&mut out, &normalize_text(topic.summary));
        out.push_str(",\"title\":");
        write_json_string(&mut out, &normalize_text(topic.title));
        out.push('}');
    }
    out.push_str("]}\n");
    Ok(out.into_bytes())
}

fn grammar_blocks(topic: &TopicDescriptor, grammar: &Grammar) -> Vec<String> {
    if topic.complete_grammar {
        return vec![grammar.complete.clone()];
    }
    topic
        .grammar_rules
        .iter()
        .map(|rule| {
            let mut block = grammar.rules[*rule].join("\n");
            block.push('\n');
            block
        })
        .collect()
}

fn write_token_table(out: &mut String, kinds: &[TokenKind]) {
    for (index, kind) in kinds.iter().enumerate() {
        comma(out, index);
        write_json_string(out, kind.label());
    }
}

fn write_strings(out: &mut String, values: &[&str]) {
    for (index, value) in values.iter().enumerate() {
        comma(out, index);
        write_json_string(out, &normalize_text(value));
    }
}

fn comma(out: &mut String, index: usize) {
    if index != 0 {
        out.push(',');
    }
}

fn normalize_source(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

fn normalize_text(value: &str) -> String {
    normalize_source(value).nfc().collect()
}

fn write_json_string(out: &mut String, value: &str) {
    out.push('"');
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            character if character <= '\u{1f}' => {
                out.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => out.push(character),
        }
    }
    out.push('"');
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn workspace() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|path| path.parent())
            .unwrap()
            .to_path_buf()
    }

    fn grammar() -> Grammar {
        let document =
            fs::read_to_string(workspace().join("docs/specification/source-surface-full.md"))
                .unwrap();
        let start = document
            .split_once("<!-- source-surface-grammar:start -->\n```text\n")
            .unwrap()
            .1;
        let grammar = start
            .split_once("\n```\n<!-- source-surface-grammar:end -->")
            .unwrap()
            .0;
        Grammar::parse(&format!("{grammar}\n")).unwrap()
    }

    #[test]
    fn checked_artifacts_are_fresh_and_deterministic() {
        let grammar = grammar();
        let first = generate(&workspace(), TOPICS, &grammar).unwrap();
        let second = generate(&workspace(), TOPICS, &grammar).unwrap();
        assert_eq!(first.artifact, second.artifact);
        assert_eq!(first.digest, second.digest);
        verify_checked_artifacts(&workspace(), &first).unwrap();
    }

    #[test]
    fn descriptor_catalog_and_selected_harness_files_are_valid() {
        validate_descriptors(&workspace(), TOPICS, &grammar()).unwrap();
        assert_eq!(TOPICS.len(), 10);
    }

    #[test]
    fn invalid_duplicate_and_broken_topic_links_are_rejected() {
        let mut invalid = TOPICS.to_vec();
        invalid[0].id = "Bad_ID";
        assert!(validate_descriptors(&workspace(), &invalid, &grammar()).is_err());

        let mut duplicate = TOPICS.to_vec();
        duplicate[1].id = duplicate[0].id;
        assert!(validate_descriptors(&workspace(), &duplicate, &grammar()).is_err());

        let mut broken = TOPICS.to_vec();
        broken[0].related = &["missing-topic"];
        assert!(validate_descriptors(&workspace(), &broken, &grammar()).is_err());
    }

    #[test]
    fn invalid_grammar_and_case_selections_are_rejected() {
        let mut bad_rule = TOPICS.to_vec();
        bad_rule[1].grammar_rules = &["MissingRule"];
        assert!(validate_descriptors(&workspace(), &bad_rule, &grammar()).is_err());

        let mut bad_file = TOPICS.to_vec();
        bad_file[1].examples = &[ExampleDescriptor {
            case: "check/module-imports",
            file: "missing.veln",
            name: "missing",
        }];
        assert!(validate_descriptors(&workspace(), &bad_file, &grammar()).is_err());
    }

    #[test]
    fn input_change_causes_freshness_failure() {
        let mut changed = generate(&workspace(), TOPICS, &grammar()).unwrap();
        changed.artifact.push(b' ');
        assert!(verify_checked_artifacts(&workspace(), &changed).is_err());
    }

    #[test]
    fn digest_uses_domain_and_canonical_length() {
        let generated = generate(&workspace(), TOPICS, &grammar()).unwrap();
        let mut transcript = DIGEST_DOMAIN.to_vec();
        transcript.extend_from_slice(&(generated.artifact.len() as u64).to_be_bytes());
        transcript.extend_from_slice(&generated.artifact);
        assert_eq!(generated.digest, hex_digest(&Sha256::digest(transcript)));
    }

    #[test]
    fn public_bundle_excludes_maintenance_and_repository_provenance() {
        let generated = generate(&workspace(), TOPICS, &grammar()).unwrap();
        let text = String::from_utf8(generated.artifact).unwrap();
        for excluded in [
            "docs/proposals",
            "examples/specification",
            "source-surface-executable.pl",
            "veln-repo-language-reference",
            "generate|verify",
        ] {
            assert!(!text.contains(excluded), "bundle contains `{excluded}`");
        }
    }

    #[test]
    fn grammar_parser_preserves_executable_output_bytes() {
        let grammar = grammar();
        let lexical = grammar_blocks(&TOPICS[0], &grammar);
        assert_eq!(lexical, vec![grammar.complete]);
    }
}
