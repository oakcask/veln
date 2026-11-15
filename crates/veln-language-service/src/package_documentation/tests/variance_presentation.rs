use super::*;

const MANIFEST: &str = concat!(
    "[package]\n",
    "name = \"variance-presentation\"\n",
    "version = \"0.1.0\"\n",
    "\n[lib]\n",
    "exports = [\"main.veln\"]\n",
);

const SOURCE: &str = concat!(
    "pub type Covariant<T>\n",
    "  pub CovariantValue(T)\n",
    "end\n",
    "pub type Contravariant<T>\n",
    "  pub ContravariantValue(fn(T) -> ())\n",
    "end\n",
    "pub type Invariant<T>\n",
    "  pub InvariantValue(fn(T) -> T)\n",
    "end\n",
    "pub type Irrelevant<T>\n",
    "  pub IrrelevantValue\n",
    "end\n",
    "pub fn present(produced: Covariant<String>, consumed: Contravariant<String>, fixed: Invariant<String>, ignored: Irrelevant<String>, transform: fn(Int) -> String, result: fn(String) -> Int) -> fn(String) -> Int\n",
    "  result\n",
    "end\n",
);

#[test]
fn package_documentation_keeps_inferred_variance_out_of_human_and_json_presentation() {
    let result = generate(MANIFEST, &[("main.veln", SOURCE)]);
    let catalog = catalog_or_panic(&result);
    let declarations = &catalog.modules[0].declarations;

    let expected = [
        (
            "Covariant",
            "type Covariant<T>",
            "CovariantValue { value: T }",
        ),
        (
            "Contravariant",
            "type Contravariant<T>",
            "ContravariantValue { value: fn(T) -> () }",
        ),
        (
            "Invariant",
            "type Invariant<T>",
            "InvariantValue { value: fn(T) -> T }",
        ),
        ("Irrelevant", "type Irrelevant<T>", "IrrelevantValue"),
    ];
    for (name, signature, constructor) in expected {
        let declaration = declarations
            .iter()
            .find(|declaration| declaration.name == name)
            .unwrap_or_else(|| panic!("published type {name}"));
        assert_eq!(declaration.signature, signature);
        assert_eq!(declaration.constructors[0].signature, constructor);
    }

    let function = declarations
        .iter()
        .find(|declaration| declaration.name == "present")
        .expect("published presentation function");
    let function_signature = "fn present(produced: Covariant<String>, consumed: Contravariant<String>, fixed: Invariant<String>, ignored: Irrelevant<String>, transform: fn(Int) -> String, result: fn(String) -> Int) -> fn(String) -> Int";
    assert_eq!(function.signature, function_signature);

    let canonical = std::str::from_utf8(result.canonical_bytes()).unwrap();
    for signature in [
        "type Covariant<T>",
        "CovariantValue { value: T }",
        "type Contravariant<T>",
        "ContravariantValue { value: fn(T) -> () }",
        "type Invariant<T>",
        "InvariantValue { value: fn(T) -> T }",
        "type Irrelevant<T>",
        "IrrelevantValue",
    ] {
        assert!(canonical.contains(&format!("\"signature\":\"{signature}\"")));
    }
    assert!(canonical.contains(&format!("\"signature\":\"{function_signature}\"")));
    let markdown = render_package_documentation(&result)
        .into_iter()
        .find(|resource| resource.uri == function.uri)
        .expect("rendered function resource");
    assert!(
        markdown
            .text
            .contains(&format!("- Signature: {function_signature}"))
    );
}
