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

fn annotation_path_classification_source(count: usize, annotation: &str) -> String {
    let mut source = String::from("fn classify(value: Option<Int>) -> ()\n");
    for index in 0..count {
        source.push_str(&format!("  let value{index}: {annotation} = value\n"));
    }
    source.push_str("  ()\nend\n");
    source
}

fn package_module(path: &str, source_text: &str, package: &str, module: &str) -> SurfaceModule {
    let source = SourceFile::new(path, source_text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut lowered = veln_ast::lower_surface_ast_with_package_module_identity(
        &parsed.tree,
        package.to_string(),
        module.to_string(),
        source.span(veln_source::TextRange::new(0, 0)),
    );
    for use_decl in &mut lowered.uses {
        use_decl.module_name = Some(module.to_string());
    }
    for alias in &mut lowered.aliases {
        alias.module_name = Some(module.to_string());
    }
    for effect in &mut lowered.effects {
        effect.module_name = Some(module.to_string());
    }
    for handler in &mut lowered.handlers {
        handler.module_name = Some(module.to_string());
    }
    for schema in &mut lowered.schemas {
        schema.module_name = Some(module.to_string());
    }
    for type_decl in &mut lowered.types {
        type_decl.module_name = Some(module.to_string());
    }
    for value in &mut lowered.module_values {
        value.module_name = Some(module.to_string());
    }
    for function in &mut lowered.functions {
        function.module_name = Some(module.to_string());
    }
    lowered
}

fn extend_surface_module(module: &mut SurfaceModule, other: SurfaceModule) {
    module.uses.extend(other.uses);
    module.aliases.extend(other.aliases);
    module.effects.extend(other.effects);
    module.handlers.extend(other.handlers);
    module.schemas.extend(other.schemas);
    module.types.extend(other.types);
    module.module_values.extend(other.module_values);
    module.functions.extend(other.functions);
    module.invalid_names.extend(other.invalid_names);
}

#[test]
fn annotation_path_classification_work_has_a_linear_bound() {
    fn work(count: usize, annotation: &str) -> ((usize, usize), Vec<Diagnostic>) {
        let source = SourceFile::new(
            "main.veln",
            annotation_path_classification_source(count, annotation),
        );
        let parsed = parse(&source);
        assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
        let module = lower_surface_ast(&parsed.tree);
        crate::pipeline::reset_annotation_path_classification_work();
        let diagnostics = analyze_surface_module(&module);
        (
            crate::pipeline::take_annotation_path_classification_work(),
            diagnostics,
        )
    }

    const SMALL: usize = 32;
    const LARGE: usize = 64;
    for (label, annotation) in [
        ("valid", "Option<Int>"),
        ("unresolved", "Missing"),
        ("refinement", "Option<Int>::Some"),
        ("refinement argument", "Option<Missing>::Some"),
    ] {
        let (small, small_diagnostics) = work(SMALL, annotation);
        let (large, large_diagnostics) = work(LARGE, annotation);
        let small_total = small.0 + small.1;
        let large_total = large.0 + large.1;
        eprintln!(
            "annotation path classification: workload={label} count={SMALL}/{LARGE} work={small_total}/{large_total} paths={}/{}, refinements={}/{}",
            small.0, large.0, small.1, large.1
        );
        assert!(large_total > small_total, "{label}: {small:?} {large:?}");
        assert!(
            large_total <= 2 * small_total + 8,
            "doubling {label} annotations must add only linear classification work: {small:?} -> {large:?}"
        );

        let small_unresolved = small_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count();
        let large_unresolved = large_diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .count();
        if matches!(label, "unresolved" | "refinement argument") {
            assert_eq!((small_unresolved, large_unresolved), (SMALL, LARGE));
        } else {
            assert_eq!((small_unresolved, large_unresolved), (0, 0));
        }
    }
}

#[test]
fn annotation_consistent_imported_alias_selection_preserves_behavior() {
    let (work, module, diagnostics) = consistent_imported_alias_measurement(16);
    assert!(work > 0, "index construction and lookup were not measured");
    assert_consistent_imported_alias_resolution(&module, &diagnostics);
}

#[test]
fn imported_descriptor_index_construction_and_retention_are_linear_across_both_axes() {
    fn measurement(size: usize) -> crate::adt::ImportedDescriptorIndexWork {
        let mut provider = String::new();
        for index in 0..size {
            provider.push_str(&format!(
                "pub type Type{index:03}\n  pub Value{index:03}\nend\n"
            ));
        }
        let mut module = package_module("provider.veln", &provider, "workspace", "provider");
        for index in 0..size {
            extend_surface_module(
                &mut module,
                package_module(
                    &format!("owner{index:03}.veln"),
                    "use provider\n",
                    "workspace",
                    &format!("owner{index:03}"),
                ),
            );
        }

        crate::adt::reset_imported_descriptor_index_work();
        let registry = crate::adt::registry::AdtRegistry::from_module(&module);
        let work = crate::adt::imported_descriptor_index_work();

        let uses = registry.import_scope();
        for owner in [0, size - 1] {
            let module = format!("owner{owner:03}");
            for descriptor in [0, size - 1] {
                let name = format!("Type{descriptor:03}");
                assert!(
                    registry
                        .unique_descriptor_for_type_path(&name, 0, Some(&module), &uses)
                        .is_some(),
                    "owner {module} must resolve {name} uniquely"
                );
            }
        }
        work
    }

    let measured = [16, 32, 64].map(measurement);
    assert!(
        measured
            .iter()
            .all(|work| { work.construction_work > 0 && work.retained_entries > 0 })
    );
    for adjacent in measured.windows(2) {
        assert!(
            adjacent[1].construction_work <= adjacent[0].construction_work * 2 + 16,
            "doubling provider descriptors and importing owners must add only linear construction work: {measured:?}"
        );
        assert!(
            adjacent[1].retained_entries <= adjacent[0].retained_entries * 2 + 16,
            "doubling provider descriptors and importing owners must retain only linear index entries: {measured:?}"
        );
    }
    eprintln!("two-axis imported descriptor index work for sizes 16/32/64: {measured:?}");
}

#[test]
fn unique_imported_descriptor_lookup_work_is_linear_across_providers_and_annotations() {
    fn measurement(size: usize) -> usize {
        let mut module = package_module(
            "provider000.veln",
            "pub type Shared\n  pub Value\nend\n",
            "workspace",
            "provider000",
        );
        for index in 1..size {
            let provider = format!("provider{index:03}");
            extend_surface_module(
                &mut module,
                package_module(
                    &format!("{provider}.veln"),
                    "pub type Shared\n  pub Value\nend\n",
                    "workspace",
                    &provider,
                ),
            );
        }
        extend_surface_module(
            &mut module,
            package_module("app.veln", "use provider000\n", "workspace", "app"),
        );
        let registry = crate::adt::registry::AdtRegistry::from_module(&module);
        let uses = registry.import_scope();
        crate::adt::reset_import_path_comparisons();
        for _ in 0..size {
            let descriptor = registry
                .unique_descriptor_for_type_path("Shared", 0, Some("app"), &uses)
                .expect("the owner-visible provider must resolve uniquely");
            assert_eq!(descriptor.module_name.as_deref(), Some("provider000"));
        }
        crate::adt::import_path_comparisons()
    }

    let measured = [16, 32, 64].map(measurement);
    for adjacent in measured.windows(2) {
        assert!(
            adjacent[1] <= adjacent[0] * 2 + 8,
            "unique lookup work must remain adjacent-linear: {measured:?}"
        );
    }
    eprintln!("unique imported descriptor lookup work for sizes 16/32/64: {measured:?}");
}

#[test]
fn consistent_imported_descriptor_lookup_work_is_linear_across_providers_and_annotations() {
    fn measurement(size: usize) -> usize {
        let mut module = package_module(
            "base.veln",
            "pub type Root\n  pub Value\nend\n",
            "workspace",
            "base",
        );
        let mut app = String::new();
        for index in 0..size {
            let provider = format!("provider{index:03}");
            extend_surface_module(
                &mut module,
                package_module(
                    &format!("{provider}.veln"),
                    "use base\npub type Shared = base::Root\n",
                    "workspace",
                    &provider,
                ),
            );
            app.push_str(&format!("use {provider}\n"));
        }
        extend_surface_module(
            &mut module,
            package_module("app.veln", &app, "workspace", "app"),
        );
        let registry = crate::adt::registry::AdtRegistry::from_module(&module);
        let uses = registry.import_scope();
        crate::adt::reset_import_path_comparisons();
        for _ in 0..size {
            let descriptor = registry
                .consistent_imported_type_path_descriptor_any_arity(
                    "Shared",
                    Some("app"),
                    Some("workspace"),
                    &uses,
                )
                .expect("same-terminal aliases must converge");
            assert_eq!(descriptor.module_name.as_deref(), Some("provider000"));
        }
        crate::adt::import_path_comparisons()
    }

    let measured = [16, 32, 64].map(measurement);
    for adjacent in measured.windows(2) {
        assert!(
            adjacent[1] <= adjacent[0] * 2 + 8,
            "same-terminal lookup work must remain adjacent-linear: {measured:?}"
        );
    }
    eprintln!("consistent imported descriptor lookup work for sizes 16/32/64: {measured:?}");
}

fn consistent_imported_alias_measurement(size: usize) -> (usize, SurfaceModule, Vec<Diagnostic>) {
    let mut module = package_module(
        "base.veln",
        "pub type Root\n  pub Value\nend\n",
        "workspace",
        "base",
    );
    for index in 0..size {
        let provider = format!("provider{index:03}");
        let path = format!("{provider}.veln");
        extend_surface_module(
            &mut module,
            package_module(
                &path,
                "use base\npub type Shared = base::Root\n",
                "workspace",
                &provider,
            ),
        );
    }

    let mut app = String::new();
    for index in 0..size {
        app.push_str(&format!("use provider{index:03}\n"));
    }
    app.push_str("fn accepts(value: Shared) -> Shared\n");
    for index in 0..size {
        app.push_str(&format!("  let alias{index}: Shared = value\n"));
    }
    app.push_str("  value\nend\n");
    app.push_str("fn shadow<Shared>(value: Shared) -> Shared\n  value\nend\n");
    extend_surface_module(
        &mut module,
        package_module("app.veln", &app, "workspace", "app"),
    );

    crate::adt::reset_import_path_comparisons();
    let diagnostics = analyze_surface_module(&module);
    let work = crate::adt::import_path_comparisons();
    (work, module, diagnostics)
}

fn assert_consistent_imported_alias_resolution(module: &SurfaceModule, diagnostics: &[Diagnostic]) {
    assert!(
        diagnostics.iter().all(|diagnostic| {
            diagnostic.id != "name.unresolved"
                || diagnostic
                    .span
                    .as_ref()
                    .is_none_or(|span| span.file.as_str() != "app.veln")
        }),
        "consistent imports must resolve every annotation: {diagnostics:#?}"
    );
    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 1, "{diagnostics:#?}");
    assert!(shadows[0].related.iter().any(|related| {
        let json = related.to_json();
        json.contains("\"file\":\"provider000.veln\"") && json.contains("\"line\":2")
    }));

    let environment = TypeEnvironment::from_module(module);
    let accepts = environment.function("accepts").expect("accepts signature");
    let Type::Named { name, identity, .. } = &accepts.params[0] else {
        panic!(
            "expected resolved imported type, found {:?}",
            accepts.params[0]
        );
    };
    let Type::Named {
        identity: return_identity,
        ..
    } = &accepts.return_type
    else {
        panic!(
            "expected resolved return type, found {:?}",
            accepts.return_type
        );
    };
    assert_eq!(name, "Shared");
    assert_eq!(identity, return_identity);
}

#[test]
fn unimported_binder_spelling_does_not_change_unbound_annotation_diagnostic() {
    fn diagnostics(include_unimported_owner: bool) -> Vec<Diagnostic> {
        let mut sources = vec![(
            "app",
            SourceFile::new("app.veln", "fn outside(value: T) -> ()\n  ()\nend\n"),
        )];
        if include_unimported_owner {
            sources.push((
                "owner",
                SourceFile::new("owner.veln", "fn owner<T>(value: T) -> T\n  value\nend\n"),
            ));
        }
        analyze_surface_module(&merged_modules_with_identities(sources))
            .into_iter()
            .filter(|diagnostic| diagnostic.id == "name.unresolved")
            .collect()
    }

    let isolated = diagnostics(false);
    let with_unimported_owner = diagnostics(true);
    assert_eq!(isolated.len(), 1, "{isolated:#?}");
    assert_eq!(
        veln_diagnostics::diagnostic_to_json(&isolated[0]).to_json(),
        veln_diagnostics::diagnostic_to_json(&with_unimported_owner[0]).to_json(),
        "an inaccessible declaration must not change another module's diagnostic"
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
fn rigid_body_and_contract_operations_are_not_specialized_by_concrete_callers() {
    let (_, diagnostics) = analyze(concat!(
        "fn invalid_body<T>(value: T) -> T\n",
        "  value + 1\n",
        "end\n",
        "fn invalid_contract<T>(value: T) -> result: T\n",
        "ensure result % 2 == 0\n",
        "  value\n",
        "end\n",
        "fn concrete_caller() -> Int\n",
        "  invalid_body(1)\n",
        "end\n",
    ));

    let rigid_operand_mismatches = diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.message == "expected `Int`, but found `T`"
                && diagnostic
                    .details
                    .to_json()
                    .contains("\"constraint\":\"operator_operand\"")
        })
        .collect::<Vec<_>>();
    assert_eq!(rigid_operand_mismatches.len(), 2, "{diagnostics:#?}");
    assert_eq!(
        rigid_operand_mismatches
            .iter()
            .map(|diagnostic| diagnostic.span.as_ref().map(|span| span.start.line))
            .collect::<Vec<_>>(),
        [Some(2), Some(5)]
    );
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "type.mismatch"
                && diagnostic.span.as_ref().map(|span| span.start.line) == Some(9)
                && diagnostic.message == "expected `T`, but found `Int`"
        }),
        "the still-unsupported generic call must not specialize the definition: {diagnostics:#?}"
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
fn compiler_known_binder_shadow_keeps_visible_source_declaration_context() {
    let (_, diagnostics) = analyze(concat!(
        "type WallTime\n",
        "  WallTime\n",
        "end\n",
        "fn shadow<WallTime>(value: WallTime) -> WallTime\n",
        "  value\n",
        "end\n",
    ));

    let shadows = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .collect::<Vec<_>>();
    assert_eq!(shadows.len(), 1, "{diagnostics:#?}");
    let shadow = shadows[0];
    assert_eq!(
        shadow
            .span
            .as_ref()
            .map(|span| (span.start.line, span.start.column)),
        Some((4, 11))
    );
    assert!(
        shadow
            .details
            .to_json()
            .contains("\"shadowed_kind\":\"builtin_type\""),
        "{shadow:#?}"
    );
    assert!(
        shadow.related.iter().any(|related| {
            let json = related.to_json();
            json.contains("\"kind\":\"shadowed_type_declaration\"") && json.contains("\"line\":1")
        }),
        "{shadow:#?}"
    );
}

#[test]
fn opaque_compiler_owned_binder_shadow_keeps_builtin_classification_and_source_context() {
    let (_, diagnostics) = analyze(concat!(
        "type NetListener\n",
        "  Listener\n",
        "end\n",
        "fn shadow<NetListener>(value: NetListener) -> NetListener\n",
        "  value\n",
        "end\n",
    ));

    let shadow = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
        .expect("opaque compiler-owned shadow diagnostic");
    assert!(
        shadow
            .details
            .to_json()
            .contains("\"shadowed_kind\":\"builtin_type\""),
        "{shadow:#?}"
    );
    assert!(shadow.related.iter().any(|related| {
        let json = related.to_json();
        json.contains("\"kind\":\"shadowed_type_declaration\"") && json.contains("\"line\":1")
    }));
}

#[test]
fn descriptor_registered_binder_shadow_keeps_builtin_classification_and_source_context() {
    for (source, related_line) in [
        (
            concat!(
                "fn shadow<StreamAdapterAction>(value: StreamAdapterAction) -> StreamAdapterAction\n",
                "  value\n",
                "end\n",
            ),
            None,
        ),
        (
            concat!(
                "type StreamAdapterAction\n",
                "  LocalAction\n",
                "end\n",
                "fn shadow<StreamAdapterAction>(value: StreamAdapterAction) -> StreamAdapterAction\n",
                "  value\n",
                "end\n",
            ),
            Some(1),
        ),
    ] {
        let (_, diagnostics) = analyze(source);
        let shadow = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.id == "name.type_parameter_shadow")
            .expect("descriptor-registered compiler-owned shadow diagnostic");
        assert_eq!(
            shadow
                .span
                .as_ref()
                .map(|span| (span.start.line, span.start.column)),
            Some((related_line.map_or(1, |_| 4), 11)),
            "{shadow:#?}"
        );
        assert!(
            shadow
                .details
                .to_json()
                .contains("\"shadowed_kind\":\"builtin_type\""),
            "{shadow:#?}"
        );
        match related_line {
            Some(line) => assert!(shadow.related.iter().any(|related| {
                let json = related.to_json();
                json.contains("\"kind\":\"shadowed_type_declaration\"")
                    && json.contains(&format!("\"line\":{line}"))
            })),
            None => assert!(shadow.related.is_empty(), "{shadow:#?}"),
        }
    }
}
