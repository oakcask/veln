use super::*;
use crate::semantic_model::Type;
use crate::types::TypeEnvironment;

#[test]
fn qualified_imported_types_and_aliases_that_match_prelude_names_remain_visible() {
    let module = merged_modules_with_identities(vec![
        (
            "main",
            SourceFile::new(
                "main.veln",
                concat!(
                    "use source_vec\n",
                    "fn accept(values: source_vec::Vec<Int>, table: source_vec::Table<String, Int>) -> source_vec::Dict<String, Int>\n",
                    "  table\n",
                    "end\n",
                ),
            ),
        ),
        (
            "source_vec",
            SourceFile::new(
                "source_vec.veln",
                concat!(
                    "pub type Vec<A>\n",
                    "  pub Wrapped(A)\n",
                    "end\n",
                    "pub type Dict<K, V>\n",
                    "  pub Entry(K, V)\n",
                    "end\n",
                    "pub type Table = Dict\n",
                ),
            ),
        ),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}

#[test]
fn qualified_annotation_selection_checks_the_imported_leaf() {
    let module = merged_modules_with_identities(vec![
        (
            "api",
            SourceFile::new(
                "api.veln",
                concat!(
                    "pub type Present<A>\n",
                    "  pub Present(A)\n",
                    "end\n",
                    "pub type PresentAlias = Present\n",
                    "type Private\n",
                    "  Private\n",
                    "end\n",
                ),
            ),
        ),
        (
            "app",
            SourceFile::new(
                "app.veln",
                concat!(
                    "use api\n",
                    "fn accepts(value: api::Present<Int>, alias: api::PresentAlias<Int>) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn missing(value: api::Missing) -> ()\n",
                    "  ()\n",
                    "end\n",
                    "fn private(value: api::Private) -> ()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ),
    ]);
    let environment = TypeEnvironment::from_module(&module);
    let accepts = environment.function("accepts").expect("accepts signature");
    let Type::Named {
        identity: direct_identity,
        args: direct_args,
        ..
    } = &accepts.params[0]
    else {
        panic!(
            "expected imported generic type, found {:?}",
            accepts.params[0]
        );
    };
    let Type::Named {
        identity: alias_identity,
        args: alias_args,
        ..
    } = &accepts.params[1]
    else {
        panic!(
            "expected imported generic alias, found {:?}",
            accepts.params[1]
        );
    };
    assert_eq!(alias_identity, direct_identity);
    assert_eq!(direct_args, &[Type::int()]);
    assert_eq!(alias_args, &[Type::int()]);

    let diagnostics = analyze_surface_module(&module);
    let unresolved = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .map(|diagnostic| {
            (
                diagnostic.message.as_str(),
                diagnostic
                    .span
                    .as_ref()
                    .map(|span| (span.start.line, span.start.column)),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        unresolved,
        [
            ("unresolved type `api::Missing`", Some((5, 24))),
            ("unresolved type `api::Private`", Some((8, 24)))
        ],
        "{diagnostics:#?}"
    );
}

fn standard_annotation_diagnostics(include_unrelated_module: bool) -> Vec<Diagnostic> {
    let source = SourceFile::new(
        "standard.veln",
        concat!(
            "fn known(value: Int, optional: Option<Int>, bytes: ByteChunk, step: DecodeStep<Int>) -> Int\n",
            "  value\n",
            "end\n",
            "fn unknown(value: T) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let mut module = lower_standard_package_callables(&source);
    set_standard_module_name(&mut module);
    if include_unrelated_module {
        let unrelated_source = SourceFile::new(
            "unrelated.veln",
            "fn owner<T>(value: T) -> T\n  value\nend\n",
        );
        let mut unrelated = lower_standard_package_callables(&unrelated_source);
        for function in &mut unrelated.functions {
            function.module_name = Some("std::unrelated".to_string());
        }
        module.functions.extend(unrelated.functions);
    }
    analyze_surface_module(&module)
}

fn set_standard_module_name(module: &mut SurfaceModule) {
    for function in &mut module.functions {
        function.module_name = Some("std::prelude".to_string());
    }
}

fn standard_callable_diagnostics(path: &str, source_text: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new(path, source_text);
    let mut module = lower_standard_package_callables(&source);
    set_standard_module_name(&mut module);
    analyze_surface_module(&module)
}

fn unresolved_messages(diagnostics: &[Diagnostic]) -> Vec<&str> {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .map(|diagnostic| diagnostic.message.as_str())
        .collect()
}

#[test]
fn standard_package_annotations_use_the_same_resolution_facts() {
    let baseline_diagnostics = standard_annotation_diagnostics(false);
    let unresolved = baseline_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.unresolved")
        .collect::<Vec<_>>();
    assert_eq!(unresolved.len(), 1, "{baseline_diagnostics:#?}");
    assert_eq!(unresolved[0].message, "unresolved type `T`");
    assert_eq!(
        unresolved[0].span.as_ref().map(|span| span.file.as_str()),
        Some("standard.veln")
    );
    let with_unrelated = standard_annotation_diagnostics(true)
        .into_iter()
        .find(|diagnostic| diagnostic.id == "name.unresolved")
        .expect("unresolved standard-package annotation");
    assert_eq!(
        veln_diagnostics::diagnostic_to_json(unresolved[0]).to_json(),
        veln_diagnostics::diagnostic_to_json(&with_unrelated).to_json(),
        "an unrelated standard-package module must not change the diagnostic"
    );
}

#[test]
fn known_standard_signature_compatibility_does_not_apply_to_unrelated_functions() {
    let diagnostics = standard_callable_diagnostics(
        "standard.veln",
        concat!(
            "fn vec_len(value: A) -> A\n",
            "  value\n",
            "end\n",
            "fn unrelated(value: A) -> A\n",
            "  value\n",
            "end\n",
        ),
    );
    assert_eq!(
        unresolved_messages(&diagnostics),
        ["unresolved type `A`", "unresolved type `A`"],
        "{diagnostics:#?}"
    );
}

#[test]
fn known_standard_signature_compatibility_does_not_replace_declared_binders() {
    let retained_binder_diagnostics = standard_callable_diagnostics(
        "retained_binder.veln",
        "fn vec_len<T>(value: A) -> A\n  value\nend\n",
    );
    assert_eq!(
        unresolved_messages(&retained_binder_diagnostics),
        ["unresolved type `A`", "unresolved type `A`"],
        "{retained_binder_diagnostics:#?}"
    );

    let declared_binder_diagnostics = standard_callable_diagnostics(
        "declared_binder.veln",
        "fn vec_len<A>(value: A) -> A\n  value\nend\n",
    );
    assert!(
        declared_binder_diagnostics.is_empty(),
        "{declared_binder_diagnostics:#?}"
    );
}
