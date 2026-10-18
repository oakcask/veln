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
