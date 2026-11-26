use super::*;
use crate::semantic_model::Type;
use crate::types::TypeEnvironment;

fn analyze(source_text: &str) -> (SurfaceModule, Vec<Diagnostic>) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    (module, diagnostics)
}

fn named_identity(ty: &Type) -> &str {
    let Type::Named { identity, .. } = ty else {
        panic!("expected named type, found {ty:?}");
    };
    identity
}

#[test]
fn function_binders_are_retained_with_declaration_owned_rigid_identities() {
    let (module, diagnostics) = analyze(concat!(
        "fn first<T>(value: T) -> output: T\n",
        "ensure output == value\n",
        "  let local: T = value\n",
        "  begin\n",
        "    let nested: T = local\n",
        "    nested\n",
        "  end\n",
        "end\n",
        "fn second<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
    ));
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let environment = TypeEnvironment::from_module(&module);
    let first = environment.function("first").expect("first signature");
    let second = environment.function("second").expect("second signature");

    assert_eq!(first.type_parameters.len(), 1);
    assert_eq!(first.type_parameters[0].name, "T");
    assert_eq!(first.type_parameters[0].ordinal, 0);
    assert_eq!(
        first.type_parameters[0].identity,
        named_identity(&first.params[0])
    );
    assert_eq!(
        named_identity(&first.params[0]),
        named_identity(&first.return_type)
    );
    assert_eq!(second.type_parameters.len(), 1);
    assert_ne!(
        first.type_parameters[0].identity,
        second.type_parameters[0].identity
    );
}

#[test]
fn binder_scope_ends_at_the_owning_declaration() {
    let (_, diagnostics) = analyze(concat!(
        "fn owner<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "fn outside(value: T) -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "name.unresolved" && diagnostic.message == "unresolved type `T`"
        })
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        unresolved[0].span.as_ref().map(|span| span.start.line),
        Some(4)
    );
}

#[test]
fn rigid_binder_rejects_a_concrete_body_result() {
    let (_, diagnostics) = analyze(concat!("fn invalid<T>(value: T) -> T\n", "  0\n", "end\n",));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `T`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn generic_calls_remain_outside_this_slice() {
    let (_, diagnostics) = analyze(concat!(
        "fn identity<T>(value: T) -> T\n",
        "  value\n",
        "end\n",
        "fn use_identity() -> Int\n",
        "  identity(1)\n",
        "end\n",
    ));

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message.contains("expected `T`")
                && diagnostic.message.contains("found `Int`")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn binders_cannot_shadow_builtin_or_source_types() {
    let (_, diagnostics) = analyze(concat!(
        "type Item\n",
        "  Item\n",
        "end\n",
        "fn builtin<Int>(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn source<Item>(value: Item) -> Item\n",
        "  value\n",
        "end\n",
    ));

    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 2, "{diagnostics:#?}");
    assert!(shadows[0].related.is_empty(), "{:#?}", shadows[0]);
    assert!(shadows[1].related.iter().any(|related| {
        related
            .to_json()
            .contains("\"kind\":\"shadowed_type_declaration\"")
    }));
    assert!(
        shadows
            .iter()
            .all(|diagnostic| { diagnostic.details.to_json().contains("\"ordinal\":0") })
    );
}

#[test]
fn binder_cannot_shadow_an_imported_type_alias() {
    let module = merged_modules_with_identities(vec![
        (
            "api",
            SourceFile::new(
                "api.veln",
                concat!(
                    "pub type State\n",
                    "  pub Ready\n",
                    "end\n",
                    "pub type Item = State\n",
                ),
            ),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use api\n",
                    "fn shadow<Item>(value: Item) -> Item\n",
                    "  value\n",
                    "end\n",
                ),
            ),
        ),
    ]);

    let diagnostics = analyze_surface_module(&module);
    let shadow = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .expect("imported alias shadow diagnostic");
    assert_eq!(
        shadow.span.as_ref().map(|span| span.file.as_str()),
        Some("app.veln")
    );
    assert!(shadow.related.iter().any(|related| {
        let json = related.to_json();
        json.contains("\"kind\":\"shadowed_type_declaration\"")
            && json.contains("\"file\":\"api.veln\"")
            && json.contains("\"line\":4")
    }));
}
