use super::*;
use crate::types::TypeEnvironment;

#[test]
fn checked_covariance_joins_across_inference_consumers_and_order() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "type Positive<T>\n",
            "  Present(T)\n",
            "end\n",
            "type Nested<T>\n",
            "  NestedValue(Positive<T>)\n",
            "end\n",
            "type Pair<T>\n",
            "  Paired(T, T)\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "fn aggregate(first: Positive<State::Ready>, second: Positive<State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn aggregate_reverse(first: Positive<State::Ready>, second: Positive<State::Closed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn nested(first: Nested<State::Ready>, second: Nested<State::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn recursive(first: Positive<Positive<State::Ready>>, second: Positive<Positive<State::Closed>>)\n",
            "  [first, second]\n",
            "end\n",
            "fn aliases(first: Positive<First::Ready>, second: Positive<Second::Closed>)\n",
            "  [first, second]\n",
            "end\n",
            "fn aliases_reverse(first: Positive<First::Ready>, second: Positive<Second::Closed>)\n",
            "  [second, first]\n",
            "end\n",
            "fn selected(flag: Bool, first: Positive<State::Ready>, second: Positive<State::Closed>)\n",
            "  if flag\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end\n",
            "end\n",
            "fn payload(first: Positive<State::Ready>, second: Positive<State::Closed>)\n",
            "  Paired(first, second)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    for (function, expected) in [
        ("aggregate", "Vec<Positive<State::Ready | State::Closed>>"),
        (
            "aggregate_reverse",
            "Vec<Positive<State::Ready | State::Closed>>",
        ),
        ("nested", "Vec<Nested<State::Ready | State::Closed>>"),
        (
            "recursive",
            "Vec<Positive<Positive<State::Ready | State::Closed>>>",
        ),
        ("aliases", "Vec<Positive<State::Ready | State::Closed>>"),
        (
            "aliases_reverse",
            "Vec<Positive<State::Ready | State::Closed>>",
        ),
        ("selected", "Positive<State::Ready | State::Closed>"),
        (
            "payload",
            "Pair<Positive<State::Ready | State::Closed>>::Paired",
        ),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("missing function {function}"))
                .return_type
                .render(),
            expected,
            "unexpected inferred result for {function}",
        );
    }
}

#[test]
fn aggregate_join_uses_checked_variance_in_parameter_order() {
    let source_text = concat!(
        "type Joined\n",
        "  First\n",
        "  Second\n",
        "end\n",
        "type Exact\n",
        "  First\n",
        "  Second\n",
        "end\n",
        "type Mixed<Join, ExactArgument>\n",
        "  Value(Join)\n",
        "end\n",
        "fn joined(first: Mixed<Joined::First, Exact::First>, second: Mixed<Joined::Second, Exact::First>)\n",
        "  [first, second]\n",
        "end\n",
        "fn rejected(first: Mixed<Joined::First, Exact::First>, second: Mixed<Joined::First, Exact::Second>) -> ()\n",
        "  let values = [first, second]\n",
        "  ()\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(&source_text[span.start.offset..span.end.offset], "second");

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("joined")
            .expect("joined function")
            .return_type
            .render(),
        "Vec<Mixed<Joined, Exact::First>>",
    );
}

#[test]
fn aggregate_join_keys_variance_by_resolved_package_declaration_identity() {
    let (module, app_text) = package_identity_join_fixture();
    assert_package_identity_join_diagnostics(&module, app_text);
    assert_local_package_identity_join(&module);
}

fn package_identity_join_fixture() -> (SurfaceModule, &'static str) {
    const APP_TEXT: &str = concat!(
        "use shared as dep from \"dependency/pkg\"\n",
        "type Wrapper<T>\n",
        "  Value(T)\n",
        "end\n",
        "fn local_join(first: Wrapper<Option<Int>::Some>, second: Wrapper<Option<Int>::None>)\n",
        "  [first, second]\n",
        "end\n",
        "fn imported_rejected(first: dep::Wrapper<dep::ImportedState::First>, second: dep::Wrapper<dep::ImportedState::Second>) -> ()\n",
        "  let values = [first, second]\n",
        "  ()\n",
        "end\n",
        "fn distinct_owners(local: Wrapper<Int>, imported: dep::Wrapper<Int>) -> ()\n",
        "  let values = [local, imported]\n",
        "  ()\n",
        "end\n",
    );
    const DEPENDENCY_TEXT: &str = concat!(
        "pub type ImportedState\n",
        "  pub First\n",
        "  pub Second\n",
        "end\n",
        "pub type Wrapper<T>\n",
        "  pub Empty\n",
        "end\n",
    );

    let mut module = package_module("app.veln", APP_TEXT, "application/pkg", "app");
    module.uses.first_mut().expect("dependency import").name = "shared".to_string();
    let mut dependency = package_module(
        "dependency/shared.veln",
        DEPENDENCY_TEXT,
        "dependency/pkg",
        "shared",
    );
    for declaration in &mut dependency.types {
        declaration.module_name = Some("shared".to_string());
    }
    module.uses.extend(dependency.uses);
    module.aliases.extend(dependency.aliases);
    module.effects.extend(dependency.effects);
    module.handlers.extend(dependency.handlers);
    module.schemas.extend(dependency.schemas);
    module.types.extend(dependency.types);
    module.module_values.extend(dependency.module_values);
    module.functions.extend(dependency.functions);
    module.invalid_names.extend(dependency.invalid_names);
    (module, APP_TEXT)
}

fn package_module(path: &str, text: &str, package: &str, module: &str) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    veln_ast::lower_surface_ast_with_package_module_identity(
        &parsed.tree,
        package.to_string(),
        module.to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    )
}

fn assert_package_identity_join_diagnostics(module: &SurfaceModule, app_text: &str) {
    let diagnostics = analyze_surface_module(module);
    assert_eq!(diagnostics.len(), 2, "{diagnostics:#?}");
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id == "type.mismatch"),
        "{diagnostics:#?}",
    );
    let failed_values = diagnostics
        .iter()
        .map(|diagnostic| {
            let span = diagnostic.span.as_ref().expect("mismatch span");
            &app_text[span.start.offset..span.end.offset]
        })
        .collect::<Vec<_>>();
    assert_eq!(failed_values, ["second", "imported"]);
}

fn assert_local_package_identity_join(module: &SurfaceModule) {
    let environment = TypeEnvironment::from_module(module);
    assert_eq!(
        environment
            .function("local_join")
            .expect("local join function")
            .return_type
            .render(),
        "Vec<Wrapper<Option<Int>>>",
    );
}

#[test]
fn failed_nested_covariant_join_rolls_back_arguments_refinements_and_presentation() {
    let source_text = concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "end\n",
        "pub type First = State\n",
        "pub type Second = State\n",
        "type Product<A, B>\n",
        "  ProductValue(A, B)\n",
        "end\n",
        "fn retained(first: Product<First::Ready, First::Ready>, rejected: Product<Second::Failed, String>, later: Product<First::Closed, First::Ready>)\n",
        "  [first, rejected, later]\n",
        "end\n",
    );
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
    let span = diagnostics[0].span.as_ref().expect("mismatch span");
    assert_eq!(&source_text[span.start.offset..span.end.offset], "rejected",);

    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("retained")
            .expect("retained function")
            .return_type
            .render(),
        "Vec<Product<First::Ready | First::Closed, First::Ready>>",
    );
}

#[test]
fn non_covariant_source_slots_remain_exact_during_inference() {
    for (name, declaration) in [
        (
            "contravariant",
            "type Wrapper<T>\n  Value(fn(T) -> ())\nend\n",
        ),
        (
            "invariant",
            "type Wrapper<T>\n  Output(T)\n  Input(fn(T) -> ())\nend\n",
        ),
        ("irrelevant", "type Wrapper<T>\n  Empty\nend\n"),
    ] {
        let source_text = format!(
            "type State\n  Ready\n  Closed\nend\n{declaration}\
             fn first(value: Wrapper<State::Ready>) -> Wrapper<State::Ready>\n\
               value\n\
             end\n\
             fn second(value: Wrapper<State::Closed>) -> Wrapper<State::Closed>\n\
               value\n\
             end\n\
             fn rejected(first_value: Wrapper<State::Ready>, second_value: Wrapper<State::Closed>) -> ()\n\
               let values = [first(first_value), second(second_value)]\n\
               ()\n\
             end\n"
        );
        let source = SourceFile::new("main.veln", &source_text);
        let parsed = parse(&source);
        assert!(
            parsed.diagnostics.is_empty(),
            "{name}: {:#?}",
            parsed.diagnostics
        );
        let diagnostics = analyze_surface_module(&lower_surface_ast(&parsed.tree));
        assert_eq!(diagnostics.len(), 1, "{name}: {diagnostics:#?}");
        assert_eq!(
            diagnostics[0].id, "type.mismatch",
            "{name}: {diagnostics:#?}"
        );
    }
}

#[test]
fn unavailable_variance_metadata_falls_back_to_exact_inference() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "type Positive<T>\n",
            "  Present(T)\n",
            "end\n",
            "fn first(value: Positive<State::Ready>) -> Positive<State::Ready>\n",
            "  value\n",
            "end\n",
            "fn second(value: Positive<State::Closed>) -> Positive<State::Closed>\n",
            "  value\n",
            "end\n",
            "fn rejected(first_value: Positive<State::Ready>, second_value: Positive<State::Closed>) -> ()\n",
            "  let values = [first(first_value), second(second_value)]\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let mut environment = TypeEnvironment::from_module(&module);
    let identity = environment
        .adts
        .descriptors()
        .iter()
        .find(|descriptor| descriptor.type_name == "Positive")
        .expect("positive descriptor")
        .identity();
    environment
        .adts
        .clear_parameter_variances_for_identity(&identity);

    let diagnostics =
        crate::pipeline::analyze_surface_module_with_environment_for_test(&module, &environment);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{diagnostics:#?}");
}
