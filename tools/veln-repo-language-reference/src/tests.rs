use super::*;
use std::path::PathBuf;

struct StaticGrammar(&'static str);

impl GrammarSource for StaticGrammar {
    fn complete_grammar(&self, _repo_root: &Path) -> Result<String, String> {
        Ok(self.0.to_string())
    }
}

struct StaticExamples(BTreeMap<String, BTreeMap<String, String>>);

impl ExampleSource for StaticExamples {
    fn selected_examples(
        &self,
        _repo_root: &Path,
        _descriptors: &[Descriptor],
    ) -> Result<BTreeMap<String, BTreeMap<String, String>>, String> {
        Ok(self.0.clone())
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

type DescriptorMutation = fn(&mut Vec<Descriptor>);

#[test]
fn checked_artifact_digest_matches_checked_digest() {
    verify_checked_digest().unwrap();
}

#[test]
fn markdown_renderer_produces_deterministic_index_and_topic() {
    let rendered = render_language_reference(&mini_catalog("value"), "abc123").unwrap();
    assert_eq!(
        rendered
            .resources
            .iter()
            .map(|resource| resource.uri.as_str())
            .collect::<Vec<_>>(),
        [
            "veln-doc:///language/snapshot/abc123/index",
            "veln-doc:///language/snapshot/abc123/topic/alpha-topic"
        ]
    );
    assert_eq!(
        rendered.resources[0].text,
        "# Veln Language Reference\n\n- [Alpha Topic](veln-doc:///language/snapshot/abc123/topic/alpha-topic) - Alpha summary.\n"
    );
    assert_eq!(
        rendered.resources[1].text,
        concat!(
            "# Alpha Topic\n\n",
            "Alpha summary.\n\n",
            "First paragraph.\n\n",
            "## Grammar\n\n",
            "### Expr\n\n",
            "```ebnf\n",
            "Expr ::= Name\n",
            "```\n\n",
            "## Examples\n\n",
            "### Alpha example\n\n",
            "#### main.veln\n\n",
            "```veln\n",
            "value\n",
            "```\n\n",
            "## Keywords\n\n",
            "- alpha\n\n",
            "## Related Topics\n\n",
        )
    );
}

#[test]
fn markdown_renderer_enforces_resource_byte_limit() {
    let empty = render_language_reference(&mini_catalog(""), "abc123").unwrap();
    let fixed_topic_len = empty.resources[1].text.len();
    let at_limit_source = "a".repeat(LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT - fixed_topic_len);
    let at_limit = render_language_reference(&mini_catalog(&at_limit_source), "abc123").unwrap();
    assert_eq!(
        at_limit.resources[1].text.len(),
        LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT
    );

    let too_large_source = "a".repeat(LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT - fixed_topic_len + 1);
    assert!(render_language_reference(&mini_catalog(&too_large_source), "abc123").is_err());
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
    let cases: &[(&str, DescriptorMutation)] = &[
        ("invalid topic identifier", |descriptors| {
            descriptors[0].id = "Bad"
        }),
        ("duplicate topic identifier", |descriptors| {
            descriptors[1].id = "lexical-structure"
        }),
        ("empty display name", |descriptors| {
            descriptors[0].title = " \t"
        }),
        ("empty summary", |descriptors| descriptors[0].summary = ""),
        ("empty keywords", |descriptors| {
            descriptors[0].keywords = &[]
        }),
        ("duplicate normalized keywords", |descriptors| {
            descriptors[0].keywords = &["same", "same"]
        }),
        ("missing relation", |descriptors| {
            descriptors[0].related = &["missing"]
        }),
        ("self relation", |descriptors| {
            descriptors[0].related = &["lexical-structure"]
        }),
        ("duplicate normalized relation", |descriptors| {
            descriptors[0].related = &["contracts", "contracts"]
        }),
        ("unknown grammar", |descriptors| {
            descriptors[0].grammar = &["Missing"]
        }),
        ("duplicate normalized grammar", |descriptors| {
            descriptors[0].grammar = &["Module", "Module"]
        }),
        ("empty example display name", |descriptors| {
            descriptors[0].examples = &[ExampleSelection {
                case: "check/source-surface",
                display_name: "",
                files: &["main.veln"],
            }]
        }),
        ("empty example case", |descriptors| {
            descriptors[0].examples = &[ExampleSelection {
                case: "",
                display_name: "case",
                files: &["main.veln"],
            }]
        }),
        ("empty example files", |descriptors| {
            descriptors[0].examples = &[ExampleSelection {
                case: "check/source-surface",
                display_name: "case",
                files: &[],
            }]
        }),
        ("external example case", |descriptors| {
            descriptors[0].examples = &[ExampleSelection {
                case: "../check/source-surface",
                display_name: "case",
                files: &["main.veln"],
            }]
        }),
        ("external example file", |descriptors| {
            descriptors[0].examples = &[ExampleSelection {
                case: "check/source-surface",
                display_name: "case",
                files: &["../main.veln"],
            }]
        }),
    ];
    for (name, mutate) in cases {
        let mut descriptors = topic_descriptors();
        mutate(&mut descriptors);
        assert!(
            validate_descriptors(&descriptors, &grammar).is_err(),
            "{name} should be rejected"
        );
    }

    assert!(parse_grammar("Module ::= Item\nModule ::= Other\n").is_err());
}

#[test]
fn descriptor_rejections_keep_specific_error_messages() {
    let grammar = parse_grammar(MINI_GRAMMAR).unwrap();
    let cases: &[(&str, DescriptorMutation)] = &[
        (
            "topic identifiers must be lowercase ASCII identifiers: Bad",
            |descriptors| descriptors[0].id = "Bad",
        ),
        (
            "topic `lexical-structure` selects unknown grammar production `Missing`",
            |descriptors| descriptors[0].grammar = &["Missing"],
        ),
        (
            "topic `lexical-structure` has non-repository-relative `example file` value `../main.veln`",
            |descriptors| {
                descriptors[0].examples = &[ExampleSelection {
                    case: "check/source-surface",
                    display_name: "case",
                    files: &["../main.veln"],
                }]
            },
        ),
        (
            "topic `lexical-structure` relates to unknown topic `missing`",
            |descriptors| descriptors[0].related = &["missing"],
        ),
    ];

    for (expected, mutate) in cases {
        let mut descriptors = topic_descriptors();
        mutate(&mut descriptors);
        assert_eq!(
            validate_descriptors(&descriptors, &grammar),
            Err((*expected).to_string())
        );
    }
}

#[test]
fn example_rejections_cover_manifest_selection_boundary() {
    let repo = repo_root();
    let mut descriptors = topic_descriptors();
    descriptors[0].examples = &[ExampleSelection {
        case: "missing/case",
        display_name: "missing case",
        files: &["main.veln"],
    }];
    assert!(validate_examples(&repo, &descriptors).is_err());

    let mut descriptors = topic_descriptors();
    descriptors[0].examples = &[ExampleSelection {
        case: "check/source-surface",
        display_name: "not selected",
        files: &["missing.veln"],
    }];
    assert!(validate_examples(&repo, &descriptors).is_err());
}

#[test]
fn public_token_projection_matches_lexer_recognition_in_both_directions() {
    validate_token_projection(PUBLIC_KEYWORDS, PUBLIC_PUNCTUATION).unwrap();
    assert!(
        validate_token_projection(
            &[PUBLIC_KEYWORDS[0], PUBLIC_KEYWORDS[0]],
            PUBLIC_PUNCTUATION
        )
        .is_err()
    );
    assert!(
        validate_token_projection(
            &[
                PUBLIC_KEYWORDS[0],
                veln_syntax::PublicToken {
                    kind: PUBLIC_KEYWORDS[0].kind,
                    spelling: "fresh",
                }
            ],
            PUBLIC_PUNCTUATION,
        )
        .is_err()
    );
    assert!(validate_token_projection(&PUBLIC_KEYWORDS[1..], PUBLIC_PUNCTUATION).is_err());
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
    let rendered = render_checked_language_reference().unwrap();
    let contract: Value = serde_json::from_str(SPEC_CONTRACT).unwrap();
    for forbidden in contract["forbidden_fragments"].as_array().unwrap() {
        let forbidden = forbidden.as_str().unwrap();
        assert_no_forbidden_fragment("checked catalog", bytes, forbidden);
        for resource in &rendered.resources {
            assert_no_forbidden_fragment("resource uri", &resource.uri, forbidden);
            assert_no_forbidden_fragment("resource name", &resource.name, forbidden);
            assert_no_forbidden_fragment("resource title", &resource.title, forbidden);
            if let Some(description) = &resource.description {
                assert_no_forbidden_fragment("resource description", description, forbidden);
            }
            assert_no_forbidden_fragment("resource MIME type", resource.mime_type, forbidden);
            assert_no_forbidden_fragment("resource text", &resource.text, forbidden);
        }
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
    assert_catalog_contract_shape(&value, &contract);
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
fn freshness_detects_artifact_and_digest_mismatches() {
    let repo = repo_root();
    let grammar = StaticGrammar(MINI_GRAMMAR);
    let descriptors = topic_descriptors();
    let generated = generate_catalog_with_inputs(
        &repo,
        &grammar,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
    )
    .unwrap();
    let artifact_mismatch = verify_freshness_against_sources(
        &repo,
        &grammar,
        &RepositoryExampleSource,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        FreshnessBaseline {
            artifact: "{}\n",
            digest: &generated.digest,
            rendered_digest: &generated.rendered_digest,
        },
    )
    .unwrap_err();
    assert!(!artifact_mismatch.artifact_matches);
    assert!(artifact_mismatch.digest_matches);

    let digest_mismatch = verify_freshness_against_sources(
        &repo,
        &grammar,
        &RepositoryExampleSource,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        FreshnessBaseline {
            artifact: &generated.bytes,
            digest: checked_catalog_digest(),
            rendered_digest: &generated.rendered_digest,
        },
    )
    .unwrap_err();
    assert!(digest_mismatch.artifact_matches);
    assert!(!digest_mismatch.digest_matches);
}

#[test]
fn freshness_rejects_renderer_digest_drift_even_when_catalog_digest_matches() {
    let repo = repo_root();
    let grammar = StaticGrammar(MINI_GRAMMAR);
    let descriptors = topic_descriptors();
    let examples = StaticExamples(selected_examples(&descriptors, "x\n"));
    let generated = generate_catalog_with_sources(
        &repo,
        &grammar,
        &examples,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
    )
    .unwrap();
    let mismatch = verify_freshness_against_sources(
        &repo,
        &grammar,
        &examples,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        FreshnessBaseline {
            artifact: &generated.bytes,
            digest: &generated.digest,
            rendered_digest: "wrong",
        },
    )
    .unwrap_err();
    assert!(mismatch.artifact_matches);
    assert!(!mismatch.digest_matches);
    assert!(
        mismatch
            .generated_digest
            .contains(&generated.rendered_digest)
    );
}

#[test]
fn generation_rejects_renderer_size_drift_before_output_replacement() {
    let repo = repo_root();
    let descriptors = topic_descriptors();
    let source = "a".repeat(LANGUAGE_REFERENCE_RESOURCE_BYTE_LIMIT);
    let result = generate_catalog_with_sources(
        &repo,
        &StaticGrammar(MINI_GRAMMAR),
        &StaticExamples(selected_examples(&descriptors, &source)),
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
    );
    assert!(result.unwrap_err().contains("above the 262144 byte limit"));
}

#[test]
fn input_mutations_change_fresh_generation_and_fail_freshness() {
    let repo = repo_root();
    let grammar = StaticGrammar(MINI_GRAMMAR);
    let descriptors = topic_descriptors();
    let baseline = generate_catalog_with_inputs(
        &repo,
        &grammar,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
    )
    .unwrap();

    let changed_grammar = StaticGrammar(concat!(
        "Module        ::= Item* | UseDecl*\n",
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
    ));
    assert_changed_and_stale(
        &repo,
        &changed_grammar,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        &baseline,
    );

    let mut descriptor_change = descriptors.clone();
    descriptor_change[0].summary = "Changed catalog-owned summary.";
    assert_changed_and_stale(
        &repo,
        &grammar,
        &descriptor_change,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        &baseline,
    );

    let mut example_sources = validate_examples(&repo, &descriptors).unwrap();
    let source = example_sources
        .get_mut("check/source-surface")
        .unwrap()
        .get_mut("main.veln")
        .unwrap();
    source.push_str("\nfn changed_example_source() -> Int\n\t1\nend\n");
    let changed_examples = StaticExamples(example_sources);
    assert_changed_and_stale_with_examples(
        &repo,
        &grammar,
        &changed_examples,
        &descriptors,
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
        &baseline,
    );

    let mut moved_keywords = PUBLIC_KEYWORDS[1..].to_vec();
    let mut moved_punctuation = PUBLIC_PUNCTUATION.to_vec();
    moved_punctuation.push(PUBLIC_KEYWORDS[0]);
    moved_keywords.sort_by_key(|token| token.spelling);
    assert_changed_and_stale(
        &repo,
        &grammar,
        &descriptors,
        &moved_keywords,
        &moved_punctuation,
        &baseline,
    );
}

#[test]
fn non_authoritative_and_equivalent_input_changes_do_not_change_generation() {
    let repo = repo_root();
    let grammar = StaticGrammar(MINI_GRAMMAR);
    let baseline = generate_catalog_with_inputs(
        &repo,
        &grammar,
        &topic_descriptors(),
        PUBLIC_KEYWORDS,
        PUBLIC_PUNCTUATION,
    )
    .unwrap();

    let mut reordered = topic_descriptors();
    reordered[0].keywords = &["literals", "tokens", "comments", "grammar", "lexing"];
    assert_eq!(
        generate_catalog_with_inputs(
            &repo,
            &grammar,
            &reordered,
            PUBLIC_KEYWORDS,
            PUBLIC_PUNCTUATION
        )
        .unwrap()
        .bytes,
        baseline.bytes
    );

    let development_doc_change_is_not_an_input = "docs/reference/implemented-proposals/ignored.md";
    assert!(
        !baseline
            .bytes
            .contains(development_doc_change_is_not_an_input)
    );
    assert_eq!(catalog_digest(baseline.bytes.as_bytes()), baseline.digest);
}

#[test]
fn selected_source_only_receives_newline_normalization() {
    assert_eq!(normalize_source_text("e\u{301}\r\n"), "e\u{301}\n");
    assert_ne!(
        normalize_source_text("e\u{301}\r\n"),
        normalize_catalog_text("e\u{301}\r\n")
    );
}

#[test]
fn rejected_generation_does_not_replace_checked_outputs() {
    let repo = repo_root();
    let artifact_path = repo
        .join("tools/veln-repo-language-reference/generated/language-reference-catalog-v1.json");
    let digest_path = repo
        .join("tools/veln-repo-language-reference/generated/language-reference-catalog-v1.sha256");
    let artifact_before = fs::read_to_string(&artifact_path).unwrap();
    let digest_before = fs::read_to_string(&digest_path).unwrap();
    let mut descriptors = topic_descriptors();
    descriptors[0].id = "Bad";
    assert!(
        generate_catalog_with_inputs(
            &repo,
            &StaticGrammar(MINI_GRAMMAR),
            &descriptors,
            PUBLIC_KEYWORDS,
            PUBLIC_PUNCTUATION,
        )
        .is_err()
    );
    assert_eq!(fs::read_to_string(&artifact_path).unwrap(), artifact_before);
    assert_eq!(fs::read_to_string(&digest_path).unwrap(), digest_before);
}

fn assert_changed_and_stale(
    repo: &Path,
    grammar: &impl GrammarSource,
    descriptors: &[Descriptor],
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
    baseline: &GeneratedCatalog,
) {
    let generated =
        generate_catalog_with_inputs(repo, grammar, descriptors, keywords, punctuation).unwrap();
    assert_ne!(generated.bytes, baseline.bytes);
    assert_ne!(generated.digest, baseline.digest);
    let mismatch = verify_freshness_against(
        repo,
        grammar,
        descriptors,
        keywords,
        punctuation,
        &baseline.bytes,
        &baseline.digest,
    )
    .unwrap_err();
    assert!(!mismatch.artifact_matches);
    assert!(!mismatch.digest_matches);
}

fn assert_changed_and_stale_with_examples(
    repo: &Path,
    grammar: &impl GrammarSource,
    examples: &impl ExampleSource,
    descriptors: &[Descriptor],
    keywords: &[veln_syntax::PublicToken],
    punctuation: &[veln_syntax::PublicToken],
    baseline: &GeneratedCatalog,
) {
    let generated =
        generate_catalog_with_sources(repo, grammar, examples, descriptors, keywords, punctuation)
            .unwrap();
    assert_ne!(generated.bytes, baseline.bytes);
    assert_ne!(generated.digest, baseline.digest);
    let mismatch = verify_freshness_against_sources(
        repo,
        grammar,
        examples,
        descriptors,
        keywords,
        punctuation,
        FreshnessBaseline {
            artifact: &baseline.bytes,
            digest: &baseline.digest,
            rendered_digest: &baseline.rendered_digest,
        },
    )
    .unwrap_err();
    assert!(!mismatch.artifact_matches);
    assert!(!mismatch.digest_matches);
}

fn assert_catalog_contract_shape(value: &Value, contract: &Value) {
    for key in contract["required_top_level_keys"].as_array().unwrap() {
        assert!(value.get(key.as_str().unwrap()).is_some());
    }
    for topic in value["topics"].as_array().unwrap() {
        assert_topic_contract_shape(topic, contract);
    }
    assert_public_token_contract_shape(value);
    let expected = catalog_digest(CHECKED_ARTIFACT.as_bytes());
    assert_eq!(expected, checked_catalog_digest());
}

fn assert_topic_contract_shape(topic: &Value, contract: &Value) {
    for key in contract["required_topic_keys"].as_array().unwrap() {
        assert!(topic.get(key.as_str().unwrap()).is_some());
    }
    for related in topic["related"].as_array().unwrap() {
        let related = related.as_str().unwrap();
        assert!(
            contract["topic_ids"]
                .as_array()
                .unwrap()
                .iter()
                .any(|id| id.as_str() == Some(related))
        );
    }
    for grammar in topic["grammar"].as_array().unwrap() {
        assert!(
            grammar["name"]
                .as_str()
                .is_some_and(|name| !name.is_empty())
        );
        assert!(
            grammar["text"]
                .as_str()
                .is_some_and(|text| !text.is_empty())
        );
    }
    for example in topic["examples"].as_array().unwrap() {
        assert!(
            example["case"]
                .as_str()
                .is_some_and(|case| !case.is_empty())
        );
        assert!(
            example["display_name"]
                .as_str()
                .is_some_and(|name| !name.is_empty())
        );
        assert!(
            example["files"]
                .as_array()
                .is_some_and(|files| !files.is_empty())
        );
    }
}

fn assert_public_token_contract_shape(value: &Value) {
    for section in ["keywords", "punctuation"] {
        for token in value["public_tokens"][section].as_array().unwrap() {
            assert!(token["kind"].as_str().is_some_and(|kind| !kind.is_empty()));
            assert!(
                token["spelling"]
                    .as_str()
                    .is_some_and(|spelling| !spelling.is_empty())
            );
        }
    }
}

fn assert_no_forbidden_fragment(label: &str, text: &str, forbidden: &str) {
    assert!(
        !text.contains(forbidden),
        "{label} leaked forbidden text {forbidden}"
    );
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("language-reference package must live under tools")
        .to_path_buf()
}

fn mini_catalog(source: &str) -> String {
    json!({
        "generator_contract_version": 1,
        "grammar": {"complete": "Expr ::= Name\n"},
        "public_tokens": {"keywords": [], "punctuation": []},
        "schema_version": 1,
        "topics": [{
            "body": ["First paragraph."],
            "examples": [{
                "case": "check/alpha",
                "display_name": "Alpha example",
                "files": [{"path": "main.veln", "source": source}]
            }],
            "grammar": [{"name": "Expr", "text": "Expr ::= Name"}],
            "id": "alpha-topic",
            "keywords": ["alpha"],
            "related": [],
            "summary": "Alpha summary.",
            "title": "Alpha Topic"
        }]
    })
    .to_string()
}

fn selected_examples(
    descriptors: &[Descriptor],
    source: &str,
) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut examples = BTreeMap::new();
    for descriptor in descriptors {
        for selection in descriptor.examples {
            let files = examples
                .entry(selection.case.to_string())
                .or_insert_with(BTreeMap::new);
            for file in selection.files {
                files
                    .entry((*file).to_string())
                    .or_insert_with(|| source.to_string());
            }
        }
    }
    examples
}
