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

pub(crate) fn topic_descriptors() -> Vec<Descriptor> {
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
