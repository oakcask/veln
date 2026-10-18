use super::*;

const MANIFEST: &str = concat!(
    "[package]\n",
    "name = \"demo\"\n",
    "version = \"0.1.0\"\n",
    "\n[lib]\n",
    "exports = [\"main.veln\"]\n",
);

#[test]
fn package_catalog_preserves_valid_structural_refinement_signatures() {
    let result = generate(
        MANIFEST,
        &[(
            "main.veln",
            concat!(
                "pub type State\n",
                "  pub Ready\n",
                "  pub Closed\n",
                "end\n",
                "pub type Box<A>\n",
                "  pub Boxed(value: A)\n",
                "end\n",
                "pub type Envelope\n",
                "  pub Wrapped(value: Box<State::Ready | State::Closed>)\n",
                "end\n",
                "pub fn expose(value: Box<State::Ready | State::Closed>) -> fn(State::Ready) -> State::Closed\n",
                "  ()\n",
                "end\n",
            ),
        )],
    );

    let catalog = catalog_or_panic(&result);
    let declaration = catalog.modules[0]
        .declarations
        .iter()
        .find(|declaration| declaration.name == "expose")
        .expect("published function declaration");
    assert_eq!(
        declaration.signature,
        "fn expose(value: Box<State::Ready | State::Closed>) -> fn(State::Ready) -> State::Closed"
    );
    let envelope = catalog.modules[0]
        .declarations
        .iter()
        .find(|declaration| declaration.name == "Envelope")
        .expect("published ADT declaration");
    assert_eq!(
        envelope.constructors[0].signature,
        "Wrapped { value: Box<State::Ready | State::Closed> }"
    );
}

#[test]
fn package_catalog_ignores_private_refinement_exposure_in_non_exported_modules() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!("pub fn answer() -> Int\n", "  42\n", "end\n",),
            ),
            (
                "internal.veln",
                concat!(
                    "pub type State\n",
                    "  Hidden\n",
                    "end\n",
                    "pub type Box<A>\n",
                    "  pub Boxed(value: A)\n",
                    "end\n",
                    "pub fn expose(value: Box<State::Hidden>) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ],
    );

    let catalog = catalog_or_panic(&result);
    assert_eq!(catalog.modules.len(), 1);
    assert_eq!(catalog.modules[0].name, "main");
    assert!(
        catalog.modules[0]
            .declarations
            .iter()
            .any(|declaration| declaration.name == "answer")
    );
    assert!(
        result
            .declaration_uri_for("internal", "function", "expose")
            .is_none()
    );
}

const INTERNAL_BROKEN_ADT: &str = concat!(
    "pub type State\n",
    "  Hidden\n",
    "end\n",
    "pub type Box<A>\n",
    "  pub Boxed(value: A)\n",
    "end\n",
    "pub type Broken\n",
    "  pub Wrapped(value: Box<State::Hidden>)\n",
    "end\n",
);

#[test]
fn package_catalog_rejects_function_with_ordinary_quarantined_adt_dependency() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub fn expose(value: internal::Broken) -> internal::Broken\n",
                    "  value\n",
                    "end\n",
                ),
            ),
            ("internal.veln", INTERNAL_BROKEN_ADT),
        ],
    );

    assert!(result.catalog().is_none());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>(),
        [("semantic_visibility", "type.variant_refinement_private")]
    );
    assert!(
        result
            .declaration_uri_for("main", "function", "expose")
            .is_none()
    );
}

#[test]
fn package_catalog_rejects_generic_quarantined_adt_alias() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!("use internal\n", "pub type Facade = internal::Broken\n",),
            ),
            (
                "internal.veln",
                concat!(
                    "pub type State\n",
                    "  Hidden\n",
                    "end\n",
                    "pub type Broken<A>\n",
                    "  pub Wrapped(value: A, hidden: State::Hidden)\n",
                    "end\n",
                ),
            ),
        ],
    );

    assert!(result.catalog().is_none());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>(),
        [("semantic_visibility", "type.variant_refinement_private")]
    );
    let span = result.status().diagnostics[0]
        .span
        .as_ref()
        .expect("causal visibility diagnostic span");
    assert_eq!(
        span.source_uri,
        source_uri("demo", result.snapshot_digest(), "internal.veln")
    );
    assert!(
        result
            .declaration_uri_for("main", "alias", "Facade")
            .is_none()
    );
}

#[test]
fn package_catalog_reports_only_causal_non_exported_privacy_diagnostics() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use causal\n",
                    "pub fn expose(value: causal::Broken) -> causal::Broken\n",
                    "  value\n",
                    "end\n",
                ),
            ),
            ("causal.veln", INTERNAL_BROKEN_ADT),
            ("unrelated.veln", INTERNAL_BROKEN_ADT),
        ],
    );

    assert!(result.catalog().is_none());
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.gate, "semantic_visibility");
    assert_eq!(diagnostic.code, "type.variant_refinement_private");
    let span = diagnostic
        .span
        .as_ref()
        .expect("causal visibility diagnostic span");
    assert_eq!(
        span.source_uri,
        source_uri("demo", result.snapshot_digest(), "causal.veln")
    );
    assert_eq!((span.line, span.column), (8, 22));
    assert!(
        result
            .declaration_uri_for("main", "function", "expose")
            .is_none()
    );
}

#[test]
fn package_catalog_rejects_adt_payload_with_ordinary_quarantined_dependency() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub type Envelope\n",
                    "  pub Wrapped(value: internal::Broken)\n",
                    "end\n",
                ),
            ),
            ("internal.veln", INTERNAL_BROKEN_ADT),
        ],
    );

    assert!(result.catalog().is_none());
    assert_eq!(result.status().diagnostics.len(), 1);
    assert!(
        result
            .declaration_uri_for("main", "type", "Envelope")
            .is_none()
    );
}

#[test]
fn unrelated_non_exported_quarantined_adt_does_not_block_package_catalog() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!("pub fn answer() -> Int\n", "  42\n", "end\n",),
            ),
            ("internal.veln", INTERNAL_BROKEN_ADT),
        ],
    );

    let catalog = catalog_or_panic(&result);
    assert!(
        catalog.modules[0]
            .declarations
            .iter()
            .any(|declaration| declaration.name == "answer")
    );
    assert!(result.status().diagnostics.is_empty());
}

#[test]
fn package_catalog_rejects_direct_private_refinement_from_non_exported_module() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub fn expose(value: internal::State::Hidden) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            (
                "internal.veln",
                concat!("pub type State\n", "  pub Ready\n", "  Hidden\n", "end\n",),
            ),
        ],
    );

    assert_exported_consumer_rejected(&result, 2, 39, "function", "expose");
}

#[test]
fn package_catalog_rejects_nested_private_refinement_from_non_exported_module() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub fn expose(value: internal::Box<internal::State::Hidden>) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            (
                "internal.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready\n",
                    "  Hidden\n",
                    "end\n",
                    "pub type Box<A>\n",
                    "  pub Boxed(value: A)\n",
                    "end\n",
                ),
            ),
        ],
    );

    assert_exported_consumer_rejected(&result, 2, 22, "function", "expose");
}

#[test]
fn package_catalog_rejects_private_refinement_through_non_exported_alias() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub fn expose(value: internal::Alias::Hidden) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            (
                "internal.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready\n",
                    "  Hidden\n",
                    "end\n",
                    "pub type Alias = State\n",
                ),
            ),
        ],
    );

    assert_exported_consumer_rejected(&result, 2, 32, "function", "expose");
}

#[test]
fn package_catalog_rejects_public_adt_payload_from_non_exported_module() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub type Envelope\n",
                    "  pub Wrapped(value: internal::State::Hidden)\n",
                    "end\n",
                ),
            ),
            (
                "internal.veln",
                concat!("pub type State\n", "  pub Ready\n", "  Hidden\n", "end\n",),
            ),
        ],
    );

    assert_exported_consumer_rejected(&result, 3, 39, "type", "Envelope");
}

#[test]
fn package_catalog_preserves_public_refinement_from_non_exported_module() {
    let result = generate(
        MANIFEST,
        &[
            (
                "main.veln",
                concat!(
                    "use internal\n",
                    "pub fn expose(value: internal::State::Ready) -> internal::State::Ready\n",
                    "  value\n",
                    "end\n",
                ),
            ),
            (
                "internal.veln",
                concat!("pub type State\n", "  pub Ready\n", "end\n",),
            ),
        ],
    );

    let catalog = catalog_or_panic(&result);
    let expose = catalog.modules[0]
        .declarations
        .iter()
        .find(|declaration| declaration.name == "expose")
        .expect("published function declaration");
    assert_eq!(
        expose.signature,
        "fn expose(value: internal::State::Ready) -> internal::State::Ready"
    );
    assert!(
        result
            .declaration_uri_for("internal", "type", "State")
            .is_none()
    );
}

fn assert_exported_consumer_rejected(
    result: &PackageDocResult,
    line: usize,
    column: usize,
    declaration_kind: &str,
    declaration_name: &str,
) {
    assert!(result.catalog().is_none());
    let diagnostics = &result.status().diagnostics;
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    let diagnostic = &diagnostics[0];
    assert_eq!(diagnostic.gate, "semantic_visibility");
    assert_eq!(diagnostic.code, "type.variant_refinement_private");
    let span = diagnostic
        .span
        .as_ref()
        .expect("visibility diagnostic span");
    assert_eq!(
        span.source_uri,
        source_uri("demo", result.snapshot_digest(), "main.veln")
    );
    assert_eq!((span.line, span.column), (line, column));
    assert!(
        result
            .declaration_uri_for("main", declaration_kind, declaration_name)
            .is_none()
    );
}

#[test]
fn package_catalog_rejects_alias_function_and_recursive_exposure_paths() {
    let result = generate(
        MANIFEST,
        &[(
            "main.veln",
            concat!(
                "pub type State\n",
                "  pub Ready\n",
                "  Hidden\n",
                "end\n",
                "pub type Alias = State\n",
                "pub type Box<A>\n",
                "  pub Boxed(value: A)\n",
                "end\n",
                "pub type Recursive\n",
                "  pub Next(value: Recursive)\n",
                "  pub HiddenValue(value: Box<Alias::Hidden>)\n",
                "end\n",
                "pub fn expose(value: fn(Alias::Hidden) -> Alias::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| {
                let span = diagnostic
                    .span
                    .as_ref()
                    .expect("visibility diagnostic span");
                (
                    diagnostic.gate.as_str(),
                    diagnostic.code.as_str(),
                    span.line,
                    span.column,
                )
            })
            .collect::<Vec<_>>(),
        [
            (
                "semantic_visibility",
                "type.variant_refinement_private",
                11,
                26,
            ),
            (
                "semantic_visibility",
                "type.variant_refinement_private",
                13,
                22,
            ),
            (
                "semantic_visibility",
                "type.variant_refinement_private",
                13,
                22,
            ),
        ]
    );
    assert!(
        result
            .declaration_uri_for("main", "type", "Recursive")
            .is_none()
    );
    assert!(
        result
            .declaration_uri_for("main", "function", "expose")
            .is_none()
    );
}

#[test]
fn package_catalog_rejects_private_structural_refinement_exposure_atomically() {
    let result = generate(
        MANIFEST,
        &[(
            "main.veln",
            concat!(
                "pub type State\n",
                "  pub Ready\n",
                "  Hidden\n",
                "end\n",
                "pub type Box<A>\n",
                "  pub Boxed(value: A)\n",
                "end\n",
                "pub type Envelope\n",
                "  pub Wrapped(value: Box<State::Hidden>)\n",
                "end\n",
                "pub type RootEnvelope\n",
                "  pub Wrapped(value: Option< State::Hidden >::Some)\n",
                "end\n",
                "pub fn expose(value: Box<State::Hidden>) -> ()\n",
                "  ()\n",
                "end\n",
                "pub fn expose_root(value: Option<State::Hidden>::Some) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>(),
        [
            ("semantic_visibility", "type.variant_refinement_private"),
            ("semantic_visibility", "type.variant_refinement_private"),
            ("semantic_visibility", "type.variant_refinement_private"),
            ("semantic_visibility", "type.variant_refinement_private"),
        ]
    );
    assert!(
        result
            .declaration_uri_for("main", "type", "Envelope")
            .is_none()
    );
    assert!(
        result
            .declaration_uri_for("main", "function", "expose")
            .is_none()
    );
    assert!(
        result
            .declaration_uri_for("main", "type", "RootEnvelope")
            .is_none()
    );
    assert!(
        result
            .declaration_uri_for("main", "function", "expose_root")
            .is_none()
    );
}

#[test]
fn package_catalog_rejects_invalid_arity_siblings_with_private_refinements_atomically() {
    let result = generate(
        MANIFEST,
        &[(
            "main.veln",
            concat!(
                "pub type State\n",
                "  Hidden\n",
                "end\n",
                "pub type Pair<A, B>\n",
                "  pub Both(first: A, second: B)\n",
                "end\n",
                "pub fn accepted(value: Int) -> Int\n",
                "  value\n",
                "end\n",
                "pub fn function_overlap(value: fn(Option<Int, Int>) -> State::Hidden) -> ()\n",
                "  ()\n",
                "end\n",
                "pub fn wrapper_overlap(value: Pair<State::Hidden, Option<Int, Int>>) -> ()\n",
                "  ()\n",
                "end\n",
            ),
        )],
    );

    assert!(result.catalog().is_none());
    assert_eq!(
        result
            .status()
            .diagnostics
            .iter()
            .map(|diagnostic| (diagnostic.gate.as_str(), diagnostic.code.as_str()))
            .collect::<Vec<_>>(),
        [
            ("semantic_visibility", "type.variant_refinement_private"),
            ("semantic_visibility", "type.variant_refinement_private"),
        ]
    );
    for (kind, name) in [
        ("type", "State"),
        ("type", "Pair"),
        ("function", "accepted"),
        ("function", "function_overlap"),
        ("function", "wrapper_overlap"),
    ] {
        assert!(
            result.declaration_uri_for("main", kind, name).is_none(),
            "atomic failure must hide {name}"
        );
    }
}
