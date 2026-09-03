use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;
use veln_project::Project;
use veln_source::SourceFile;
use veln_syntax::{PUBLIC_KEYWORDS, PUBLIC_PUNCTUATION, lex};

#[allow(dead_code)]
#[path = "../../../crates/veln-cli/tests/toolchain_harness/manifest_syntax.rs"]
mod manifest_syntax;

use manifest_syntax::Statement as ManifestStatement;

pub const SCHEMA_VERSION: u64 = 1;
pub const GENERATOR_CONTRACT_VERSION: u64 = 1;
pub const DIGEST_DOMAIN: &[u8] = b"veln-language-reference/v1\0";
pub const CHECKED_ARTIFACT: &str = include_str!("../generated/language-reference-catalog-v1.json");
pub const CHECKED_DIGEST: &str = include_str!("../generated/language-reference-catalog-v1.sha256");
#[cfg(test)]
const SPEC_CONTRACT: &str =
    include_str!("../../../examples/specification/language-reference/catalog-contract-v1.json");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedCatalog {
    pub bytes: String,
    pub digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshnessMismatch {
    pub artifact_matches: bool,
    pub digest_matches: bool,
    pub generated_digest: String,
    pub checked_digest: String,
}

pub trait GrammarSource {
    fn complete_grammar(&self, repo_root: &Path) -> Result<String, String>;
}

pub struct SwiplGrammarSource;

impl GrammarSource for SwiplGrammarSource {
    fn complete_grammar(&self, repo_root: &Path) -> Result<String, String> {
        let spec = repo_root.join("docs/specification/source-surface-executable.pl");
        let output = Command::new("swipl")
            .current_dir(repo_root)
            .args(["-q", "-s"])
            .arg(spec)
            .args(["--", "--grammar"])
            .output()
            .map_err(|error| {
                format!(
                    "install SWI-Prolog before regenerating the language-reference catalog: {error}"
                )
            })?;
        if !output.status.success() {
            return Err(format!(
                "update the executable grammar before regenerating the language-reference catalog:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        String::from_utf8(output.stdout)
            .map_err(|error| format!("the executable grammar printed non-UTF-8 output: {error}"))
    }
}

#[derive(Clone, Debug)]
pub struct Descriptor {
    pub id: &'static str,
    pub title: &'static str,
    pub summary: &'static str,
    pub keywords: &'static [&'static str],
    pub body: &'static [&'static str],
    pub related: &'static [&'static str],
    pub grammar: &'static [&'static str],
    pub examples: &'static [ExampleSelection],
}

#[derive(Clone, Copy, Debug)]
pub struct ExampleSelection {
    pub case: &'static str,
    pub display_name: &'static str,
    pub files: &'static [&'static str],
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GrammarProduction {
    name: String,
    lines: Vec<String>,
}

pub fn generate_checked_catalog(repo_root: &Path) -> Result<GeneratedCatalog, String> {
    generate_catalog(repo_root, &SwiplGrammarSource)
}

pub fn generate_catalog(
    repo_root: &Path,
    grammar_source: &impl GrammarSource,
) -> Result<GeneratedCatalog, String> {
    let repo_root = repo_root.canonicalize().map_err(|error| {
        format!("select a readable repository root before generating the language-reference catalog: {error}")
    })?;
    let grammar_text = grammar_source.complete_grammar(&repo_root)?;
    let grammar = parse_grammar(&grammar_text)?;
    let descriptors = topic_descriptors();
    validate_descriptors(&descriptors, &grammar)?;
    let examples = validate_examples(&repo_root, &descriptors)?;
    validate_token_projection();

    let catalog = catalog_value(&descriptors, &grammar_text, &grammar, &examples)?;
    let bytes = canonical_json(&catalog)?;
    let digest = catalog_digest(bytes.as_bytes());
    Ok(GeneratedCatalog { bytes, digest })
}

pub fn checked_catalog_bytes() -> &'static str {
    CHECKED_ARTIFACT
}

pub fn checked_catalog_digest() -> &'static str {
    CHECKED_DIGEST.trim()
}

pub fn verify_checked_digest() -> Result<(), String> {
    let expected = catalog_digest(CHECKED_ARTIFACT.as_bytes());
    if expected == checked_catalog_digest() {
        Ok(())
    } else {
        Err(format!(
            "regenerate the language-reference catalog digest; checked digest is {}, generated digest is {}",
            checked_catalog_digest(),
            expected
        ))
    }
}

pub fn verify_freshness(repo_root: &Path) -> Result<(), FreshnessMismatch> {
    let generated = generate_checked_catalog(repo_root).map_err(|message| FreshnessMismatch {
        artifact_matches: false,
        digest_matches: false,
        generated_digest: message,
        checked_digest: checked_catalog_digest().to_string(),
    })?;
    let artifact_matches = generated.bytes == CHECKED_ARTIFACT;
    let digest_matches = generated.digest == checked_catalog_digest();
    if artifact_matches && digest_matches {
        Ok(())
    } else {
        Err(FreshnessMismatch {
            artifact_matches,
            digest_matches,
            generated_digest: generated.digest,
            checked_digest: checked_catalog_digest().to_string(),
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
    Ok(())
}

fn catalog_value(
    descriptors: &[Descriptor],
    complete_grammar: &str,
    grammar: &[GrammarProduction],
    examples: &BTreeMap<String, BTreeMap<String, String>>,
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
            "keywords": token_values(PUBLIC_KEYWORDS),
            "punctuation": token_values(PUBLIC_PUNCTUATION),
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

fn parse_grammar(text: &str) -> Result<Vec<GrammarProduction>, String> {
    let mut productions = Vec::new();
    for line in normalize_source_text(text).lines() {
        if line.trim().is_empty() {
            continue;
        }
        if line.starts_with(' ') {
            let Some(last): Option<&mut GrammarProduction> = productions.last_mut() else {
                return Err("the executable grammar starts with a continuation line".to_string());
            };
            last.lines.push(line.to_string());
            continue;
        }
        let Some((name, _)) = line.split_once("::=") else {
            return Err(format!(
                "the executable grammar line does not contain a production separator: {line}"
            ));
        };
        productions.push(GrammarProduction {
            name: name.trim().to_string(),
            lines: vec![line.to_string()],
        });
    }
    let mut names = BTreeSet::new();
    for production in &productions {
        if !names.insert(production.name.clone()) {
            return Err(format!(
                "the executable grammar repeats production `{}`",
                production.name
            ));
        }
    }
    Ok(productions)
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
        if !valid_identifier(descriptor.id) {
            return Err(format!(
                "topic identifiers must be lowercase ASCII identifiers: {}",
                descriptor.id
            ));
        }
        if !ids.insert(descriptor.id) {
            return Err(format!("duplicate topic identifier `{}`", descriptor.id));
        }
        reject_empty(descriptor.id, "title", descriptor.title)?;
        reject_empty(descriptor.id, "summary", descriptor.summary)?;
        reject_empty_set(descriptor.id, "keywords", descriptor.keywords)?;
        reject_empty_set(descriptor.id, "body", descriptor.body)?;
        reject_empty_set(descriptor.id, "grammar", descriptor.grammar)?;
        reject_duplicate_normalized_set(descriptor.id, "keywords", descriptor.keywords)?;
        reject_duplicate_normalized_set(descriptor.id, "related", descriptor.related)?;
        reject_duplicate_normalized_set(descriptor.id, "grammar", descriptor.grammar)?;
        for name in descriptor.grammar {
            if !grammar_names.contains(name) {
                return Err(format!(
                    "topic `{}` selects unknown grammar production `{name}`",
                    descriptor.id
                ));
            }
        }
        for example in descriptor.examples {
            reject_empty(descriptor.id, "example display_name", example.display_name)?;
            reject_duplicate_normalized_set(descriptor.id, "example files", example.files)?;
        }
    }
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

fn validate_examples(
    repo_root: &Path,
    descriptors: &[Descriptor],
) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
    let mut cache = BTreeMap::new();
    for descriptor in descriptors {
        for example in descriptor.examples {
            let case_dir = repo_root.join("examples/specification").join(example.case);
            let manifest = SourceCaseManifest::read(&case_dir.join("case.toml"))?;
            let selected = manifest.selected_sources(&case_dir)?;
            let mut selected_relative = BTreeMap::new();
            for source in selected {
                let relative = source.strip_prefix(&case_dir).map_err(|_| {
                    format!(
                        "{}: selected example source is outside its specification case",
                        example.case
                    )
                })?;
                let relative = slash_path(relative);
                let source_text = fs::read_to_string(&source).map_err(|error| {
                    format!(
                        "{}: read selected example source: {error}",
                        source.display()
                    )
                })?;
                selected_relative.insert(relative, normalize_source_text(&source_text));
            }
            for file in example.files {
                if !selected_relative.contains_key(*file) {
                    return Err(format!(
                        "topic `{}` selects example file `{file}` that is not a source input of `{}`",
                        descriptor.id, example.case
                    ));
                }
            }
            cache.insert(example.case.to_string(), selected_relative);
        }
    }
    Ok(cache)
}

fn validate_token_projection() {
    for token in PUBLIC_KEYWORDS {
        let source = SourceFile::new("tokens.veln", token.spelling.to_string());
        let first = lex(&source).tokens[0].kind;
        assert_eq!(first, token.kind);
    }
    for token in PUBLIC_PUNCTUATION {
        let source = SourceFile::new("tokens.veln", token.spelling.to_string());
        let first = lex(&source).tokens[0].kind;
        assert_eq!(first, token.kind);
    }
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

#[derive(Debug)]
struct SourceCaseManifest {
    command: Vec<String>,
    cwd: Option<PathBuf>,
}

impl SourceCaseManifest {
    fn read(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|error| {
            format!(
                "{}: read the toolchain case manifest before selecting example source: {error}",
                path.display()
            )
        })?;
        let mut section = String::new();
        let mut manifest = Self {
            command: Vec::new(),
            cwd: None,
        };
        for statement in manifest_syntax::parse_document(path, &text) {
            match statement {
                ManifestStatement::Section { name, .. } => section = name,
                ManifestStatement::Assignment { key, value, .. } => match (section.as_str(), key) {
                    ("", "command") => manifest.command = value.parse_string_array(path),
                    ("", "cwd") => manifest.cwd = Some(PathBuf::from(value.parse_string(path))),
                    _ => {}
                },
            }
        }
        if manifest.command.is_empty() {
            return Err(format!(
                "{}: selected language-reference example case has no command",
                path.display()
            ));
        }
        Ok(manifest)
    }

    fn selected_sources(&self, case_dir: &Path) -> Result<Vec<PathBuf>, String> {
        let command_root = self
            .cwd
            .as_deref()
            .map_or_else(|| case_dir.to_path_buf(), |cwd| case_dir.join(cwd));
        let package_root = veln_project::select_package_root(&command_root).map_err(|error| {
            format!(
                "{}: select package root before checking language-reference example source: {error}",
                case_dir.display()
            )
        })?;
        let inputs = command_source_inputs(&self.command)
            .into_iter()
            .map(|input| {
                if input.is_absolute() || command_root == package_root {
                    input
                } else {
                    command_root.join(input)
                }
            })
            .filter(|input| {
                input.is_dir()
                    || input
                        .extension()
                        .is_some_and(|extension| extension == "veln")
            })
            .collect::<Vec<_>>();
        let project = Project::discover(package_root, &inputs).map_err(|error| {
            format!(
                "{}: discover selected specification source inputs before publishing examples: {error}",
                case_dir.display()
            )
        })?;
        Ok(project
            .files
            .into_iter()
            .map(|source| project.root.join(source.path().as_str()))
            .collect())
    }
}

fn command_source_inputs(command: &[String]) -> Vec<PathBuf> {
    match command.first().map(String::as_str) {
        Some("run") => command[1..]
            .iter()
            .take_while(|argument| argument.as_str() != "--")
            .filter(|argument| argument.as_str() != "--json")
            .skip(1)
            .map(PathBuf::from)
            .collect(),
        Some("check" | "doc" | "fmt" | "metrics" | "test") => {
            source_inputs_after_flags(&command[1..])
        }
        _ => Vec::new(),
    }
}

fn source_inputs_after_flags(arguments: &[String]) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let mut arguments = arguments.iter();
    while let Some(argument) = arguments.next() {
        if argument == "--" {
            break;
        }
        if argument == "--json" {
            continue;
        }
        if matches!(
            argument.as_str(),
            "--baseline" | "--write-baseline" | "--jobs" | "-j"
        ) {
            let _ = arguments.next();
            continue;
        }
        if argument.starts_with("--baseline=")
            || argument.starts_with("--write-baseline=")
            || argument.starts_with("--jobs=")
        {
            continue;
        }
        inputs.push(PathBuf::from(argument));
    }
    inputs
}

fn manifest_error(path: &Path, line_number: usize, message: impl std::fmt::Display) -> ! {
    if line_number == 0 {
        panic!("{}: {message}", path.display());
    }
    panic!("{}:{line_number}: {message}", path.display());
}

fn topic_descriptors() -> Vec<Descriptor> {
    vec![
        Descriptor {
            id: "lexical-structure",
            title: "Lexical Structure And Grammar",
            summary: "Source files use ASCII keyword and punctuation tokens, hash comments, identifiers, holes, literals, and the complete executable source grammar.",
            keywords: &["grammar", "lexing", "tokens", "comments", "literals"],
            body: &[
                "The lexical topic publishes the complete executable grammar and the public token projection from compiler-owned records.",
                "The selected example exercises accepted source syntax through the check command.",
            ],
            related: &[
                "declarations-aliases",
                "expressions-patterns",
                "tests-docs-doctests",
            ],
            grammar: &["Module", "Item", "IntLiteral"],
            examples: &[ExampleSelection {
                case: "check/source-surface",
                display_name: "Accepted source-surface case",
                files: &["main.veln"],
            }],
        },
        Descriptor {
            id: "modules-imports-packages",
            title: "Modules, Imports, Packages, Exports, And Visibility",
            summary: "Modules declare package-local paths, import modules or packages, and publish selected declarations through explicit public forms.",
            keywords: &["modules", "imports", "packages", "exports", "visibility"],
            body: &[
                "The topic selects grammar for module headers, imports, package strings, member paths, and public aliases.",
                "The selected example comes from a successful module-import specification case.",
            ],
            related: &["declarations-aliases", "tests-docs-doctests"],
            grammar: &[
                "ModuleHeader",
                "UseDecl",
                "ImportSource",
                "ModulePath",
                "PublicAlias",
            ],
            examples: &[ExampleSelection {
                case: "check/module-imports",
                display_name: "Module import check",
                files: &["app.veln", "math.veln"],
            }],
        },
        Descriptor {
            id: "declarations-aliases",
            title: "Declarations And Aliases",
            summary: "Functions, tests, effects, handlers, type declarations, schemas, and public aliases are source-level items.",
            keywords: &["functions", "types", "aliases", "effects", "handlers"],
            body: &[
                "The descriptor selects the executable item productions for source declarations.",
                "The selected example exercises public member alias re-exports in a checked case.",
            ],
            related: &[
                "modules-imports-packages",
                "types-inference-constructors",
                "effects-handlers",
            ],
            grammar: &[
                "Function",
                "TestDecl",
                "TypeDecl",
                "EffectDecl",
                "HandlerDecl",
                "PublicAlias",
            ],
            examples: &[ExampleSelection {
                case: "check/public-member-alias-reexports",
                display_name: "Public alias re-export check",
                files: &["app.veln", "api.veln", "impl.veln"],
            }],
        },
        Descriptor {
            id: "expressions-patterns",
            title: "Expressions, Operators, And Patterns",
            summary: "Expressions include calls, operators, aggregates, control flow, schema operations, effects, handlers, field access, and patterns.",
            keywords: &["expressions", "operators", "patterns", "match", "if"],
            body: &[
                "The expression grammar selection is production-based and does not duplicate a hand-maintained grammar.",
                "The selected example covers typed operators through the check command.",
            ],
            related: &["lexical-structure", "types-inference-constructors", "holes"],
            grammar: &["Expr", "BinaryOp", "PrefixExpr", "PrimaryExpr", "Pattern"],
            examples: &[ExampleSelection {
                case: "check/types-operators",
                display_name: "Operator type check",
                files: &["main.veln"],
            }],
        },
        Descriptor {
            id: "types-inference-constructors",
            title: "Types, Inference, And Constructors",
            summary: "Type text, parameters, return annotations, result bindings, constructor payloads, and inference-sensitive contexts define typed source behavior.",
            keywords: &[
                "types",
                "inference",
                "constructors",
                "annotations",
                "returns",
            ],
            body: &[
                "The topic selects grammar for type parameters, return annotations, result bindings, and constructor patterns.",
                "The selected example verifies constructor payload inference in a successful check case.",
            ],
            related: &["declarations-aliases", "expressions-patterns", "contracts"],
            grammar: &[
                "TypeParamList",
                "Return",
                "ResultBinding",
                "TypeVariant",
                "ConstructorPattern",
            ],
            examples: &[ExampleSelection {
                case: "check/constructor-payload-callback-inference",
                display_name: "Constructor payload inference check",
                files: &["main.veln"],
            }],
        },
        Descriptor {
            id: "effects-handlers",
            title: "Effects And Handlers",
            summary: "Effect declarations, effect rows, perform expressions, handler declarations, and handler operation clauses describe effectful source behavior.",
            keywords: &[
                "effects",
                "handlers",
                "perform",
                "effect rows",
                "operations",
            ],
            body: &[
                "The topic selects executable grammar for effect operations, effect rows, perform expressions, and handler clauses.",
                "The selected example covers lexical handler behavior in a successful run specification case.",
            ],
            related: &[
                "declarations-aliases",
                "expressions-patterns",
                "tests-docs-doctests",
            ],
            grammar: &[
                "EffectDecl",
                "EffectOperation",
                "Effects",
                "Perform",
                "HandlerDecl",
                "HandlerOperationClause",
            ],
            examples: &[ExampleSelection {
                case: "run/lexical-handler-nesting",
                display_name: "Lexical handler nesting run",
                files: &["main.veln"],
            }],
        },
        Descriptor {
            id: "contracts",
            title: "Contracts",
            summary: "Require, ensure, and invariant clauses attach checked contract predicates to functions, tests, schemas, and runtime obligations.",
            keywords: &["contracts", "require", "ensure", "invariant", "predicates"],
            body: &[
                "The topic selects contract grammar and a checked predicate-call example.",
                "Contract details remain specified by the current contract specification page and checked examples.",
            ],
            related: &["types-inference-constructors", "holes", "schemas"],
            grammar: &["Contract", "SchemaValidation", "SchemaFieldWhere"],
            examples: &[ExampleSelection {
                case: "check/contract-predicate-calls",
                display_name: "Contract predicate calls",
                files: &["main.veln", "predicates.veln"],
            }],
        },
        Descriptor {
            id: "schemas",
            title: "Schemas",
            summary: "Schemas describe format-neutral and binary fields, primitives, repeat forms, codec operations, and validation predicates.",
            keywords: &["schemas", "binary", "codec", "decode", "encode"],
            body: &[
                "The schema topic selects schema declaration, field, primitive, repeat, encode, and decode grammar.",
                "The selected example covers schema composition precedence through a successful check case.",
            ],
            related: &[
                "contracts",
                "types-inference-constructors",
                "expressions-patterns",
            ],
            grammar: &[
                "SchemaDecl",
                "SchemaField",
                "SchemaFieldType",
                "SchemaDecode",
                "SchemaEncode",
            ],
            examples: &[ExampleSelection {
                case: "check/schema-composition-grammar-precedence",
                display_name: "Schema composition grammar precedence",
                files: &["neutral.veln", "binary.veln"],
            }],
        },
        Descriptor {
            id: "holes",
            title: "Holes",
            summary: "Named holes and underscore patterns carry source placeholders, type context, satisfy constraints, diagnostics, and repair candidates.",
            keywords: &["holes", "underscore", "satisfy", "repairs", "diagnostics"],
            body: &[
                "The hole topic selects hole-name and pattern grammar.",
                "The selected example covers successful named-hole checking.",
            ],
            related: &[
                "contracts",
                "types-inference-constructors",
                "expressions-patterns",
            ],
            grammar: &["HoleName", "LetPattern", "Pattern"],
            examples: &[ExampleSelection {
                case: "check/named-hole-labels",
                display_name: "Named hole labels",
                files: &["main.veln"],
            }],
        },
        Descriptor {
            id: "tests-docs-doctests",
            title: "Tests, Documentation Comments, And Doctests",
            summary: "Test declarations, documentation comments, doctest fences, package documentation, and command examples define user-facing specification evidence.",
            keywords: &["tests", "doctests", "documentation", "examples", "comments"],
            body: &[
                "The topic selects test declaration grammar and a documentation command case.",
                "Rendered documentation, search, pagination, and plugin packaging are outside this catalog foundation.",
            ],
            related: &[
                "lexical-structure",
                "declarations-aliases",
                "modules-imports-packages",
            ],
            grammar: &["TestDecl", "Function", "Contract"],
            examples: &[ExampleSelection {
                case: "doc/generated-markdown",
                display_name: "Generated documentation command",
                files: &["main.veln"],
            }],
        },
    ]
}

#[cfg(test)]
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
mod tests {
    use super::*;

    struct StaticGrammar(&'static str);

    impl GrammarSource for StaticGrammar {
        fn complete_grammar(&self, _repo_root: &Path) -> Result<String, String> {
            Ok(self.0.to_string())
        }
    }

    const MINI_GRAMMAR: &str = concat!(
        "Module        ::= Item*\n",
        "Item          ::= Function | TestDecl | TypeDecl | EffectDecl | HandlerDecl | SchemaDecl | PublicAlias\n",
        "IntLiteral    ::= DecimalLiteral\n",
        "ModuleHeader  ::= \"mod\" ModuleHeaderPath NL\n",
        "UseDecl       ::= \"use\" ModulePath ImportSource? NL\n",
        "ImportSource  ::= \"from\" PackageString\n",
        "ModulePath    ::= Name (\"::\" Name)*\n",
        "PublicAlias   ::= \"pub\" (\"fn\" | \"type\" | \"schema\") Name \"=\" MemberPath NL\n",
        "Function      ::= \"pub\"? \"fn\" Name\n",
        "TestDecl      ::= \"test\" Name\n",
        "TypeDecl      ::= \"pub\"? \"type\" Name\n",
        "EffectDecl    ::= \"pub\"? \"effect\" Name\n",
        "HandlerDecl   ::= \"pub\"? \"handler\" Name\n",
        "EffectOperation ::= Name \"(\" \")\"\n",
        "Effects       ::= \"effects\" \"[\" \"]\"\n",
        "Perform       ::= \"perform\" MemberPath\n",
        "HandlerOperationClause ::= Name \"(\" \")\"\n",
        "Expr          ::= PrimaryExpr\n",
        "BinaryOp      ::= \"+\"\n",
        "PrefixExpr    ::= PrimaryExpr\n",
        "PrimaryExpr   ::= Name\n",
        "Pattern       ::= \"_\"\n",
        "TypeParamList ::= \"<\" Name \">\"\n",
        "Return        ::= \"->\" TypeText\n",
        "ResultBinding ::= Name \":\"\n",
        "TypeVariant   ::= UpperName\n",
        "ConstructorPattern ::= ConstructorName\n",
        "Contract      ::= \"require\" ContractPredicate NL\n",
        "SchemaValidation ::= \"validate\" ContractPredicate NL\n",
        "SchemaFieldWhere ::= \"where\" ContractPredicate\n",
        "SchemaDecl    ::= \"schema\" Name\n",
        "SchemaField   ::= Name \":\" TypeText\n",
        "SchemaFieldType ::= TypeText\n",
        "SchemaDecode  ::= \"decode\" MemberPath \"from\" Expr \"at\" Expr\n",
        "SchemaEncode  ::= \"encode\" MemberPath \"from\" Expr\n",
        "HoleName      ::= \"_\" identifier-continue+\n",
        "LetPattern    ::= \"_\"\n",
        "PackageString ::= String\n",
    );

    #[test]
    fn checked_artifact_digest_matches_checked_digest() {
        verify_checked_digest().unwrap();
    }

    #[test]
    fn grammar_parser_groups_continuation_lines() {
        let grammar = parse_grammar("Expr ::= A\n      | B\nName ::= Ident\n").unwrap();
        assert_eq!(grammar[0].name, "Expr");
        assert_eq!(grammar[0].lines, ["Expr ::= A", "      | B"]);
    }

    #[test]
    fn descriptor_rejections_cover_invalid_metadata_and_relations() {
        let grammar = parse_grammar(MINI_GRAMMAR).unwrap();
        let mut descriptors = topic_descriptors();
        descriptors[0].id = "Bad";
        assert!(validate_descriptors(&descriptors, &grammar).is_err());

        let mut descriptors = topic_descriptors();
        descriptors[0].related = &["missing"];
        assert!(validate_descriptors(&descriptors, &grammar).is_err());

        let mut descriptors = topic_descriptors();
        descriptors[0].related = &["lexical-structure"];
        assert!(validate_descriptors(&descriptors, &grammar).is_err());

        let mut descriptors = topic_descriptors();
        descriptors[0].grammar = &["Missing"];
        assert!(validate_descriptors(&descriptors, &grammar).is_err());

        let mut descriptors = topic_descriptors();
        descriptors[0].keywords = &["same", "same"];
        assert!(validate_descriptors(&descriptors, &grammar).is_err());
    }

    #[test]
    fn public_token_projection_matches_lexer_recognition() {
        validate_token_projection();
    }

    #[test]
    fn digest_uses_domain_and_length_transcript() {
        assert_eq!(
            catalog_digest(b"abc"),
            "a3cdd70a4b7a26454e811a4fcc85d17b0dc413e30dc511bbd3d9475aaf7a7921"
        );
        assert_ne!(catalog_digest(b"abc"), catalog_digest(b"abcd"));
    }

    #[test]
    fn normalization_makes_set_order_and_unicode_equivalent() {
        let mut left = json!({"values": normalized_set(&["beta", "e\u{301}clair"])});
        let right = json!({"values": normalized_set(&["éclair", "beta"])});
        assert_eq!(
            canonical_json(&left).unwrap(),
            canonical_json(&right).unwrap()
        );
        left["values"] = json!(normalized_set(&["a\r\nb", "c\rd"]));
        assert_eq!(left["values"], json!(["a\nb", "c\nd"]));
    }

    #[test]
    fn generated_bundle_excludes_development_provenance() {
        let bytes = checked_catalog_bytes();
        let contract: Value = serde_json::from_str(SPEC_CONTRACT).unwrap();
        for forbidden in contract["forbidden_fragments"].as_array().unwrap() {
            let forbidden = forbidden.as_str().unwrap();
            assert!(
                !bytes.contains(forbidden),
                "bundle leaked forbidden text {forbidden}"
            );
        }
    }

    #[test]
    fn checked_artifact_has_closed_schema_v1_topics() {
        let value: Value = serde_json::from_str(CHECKED_ARTIFACT).unwrap();
        let contract: Value = serde_json::from_str(SPEC_CONTRACT).unwrap();
        assert_eq!(value["schema_version"], contract["schema_version"]);
        assert_eq!(
            value["generator_contract_version"],
            contract["generator_contract_version"]
        );
        let ids = value["topics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|topic| topic["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        let expected = contract["topic_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|topic| topic.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(ids, expected);
    }

    #[test]
    fn repository_relative_rejects_external_paths() {
        assert!(repository_relative(Path::new(
            "examples/specification/check/source-surface"
        )));
        assert!(!repository_relative(Path::new("../outside")));
        assert!(!repository_relative(Path::new("/outside")));
    }

    #[test]
    fn static_generation_mutation_changes_digest() {
        let repo = Path::new(".");
        let grammar = StaticGrammar(MINI_GRAMMAR);
        let generated = generate_catalog(repo, &grammar);
        assert!(generated.is_err());
        assert_ne!(
            catalog_digest(br#"{"schema_version":1}"#),
            checked_catalog_digest()
        );
    }
}
