use super::*;

fn lower_recovery_source(path: &str, text: &str) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    lower_surface_ast(&parsed.tree)
}

fn assign_recovery_package(module: &mut SurfaceModule, package: &str) {
    for function in &mut module.functions {
        function.package_name = Some(package.to_string());
    }
    for alias in &mut module.aliases {
        alias.package_name = Some(package.to_string());
    }
    for type_decl in &mut module.types {
        type_decl.package_name = Some(package.to_string());
    }
}

fn extend_recovery_declarations(module: &mut SurfaceModule, other: SurfaceModule) {
    module.functions.extend(other.functions);
    module.aliases.extend(other.aliases);
    module.types.extend(other.types);
    module.invalid_names.extend(other.invalid_names);
}

#[test]
fn reachable_invalid_name_recovery_keeps_selected_package_identity() {
    let cases = [
        (
            "function",
            "fn main() -> Int\n  Bad()\nend\n",
            "pub fn Bad() -> Int\n  1\nend\n",
            "Bad",
        ),
        (
            "function alias",
            "fn main() -> Int\n  BadAlias()\nend\n",
            "fn good() -> Int\n  1\nend\npub fn BadAlias = good\n",
            "BadAlias",
        ),
        (
            "type",
            "fn main(value: item) -> item\n  value\nend\n",
            "pub type item\n  Value\nend\n",
            "item",
        ),
        (
            "constructor",
            "fn main() -> Item\n  value\nend\n",
            "pub type Item\n  value\nend\n",
            "value",
        ),
    ];

    for (carrier, main_body, declaration, invalid_name) in cases {
        for reverse in [false, true] {
            let mut module = lower_recovery_source("app.veln", &format!("mod shared\n{main_body}"));
            let selected = lower_recovery_source(
                "selected/shared.veln",
                &format!("mod shared\n{declaration}"),
            );
            let mut namesake = lower_recovery_source(
                "namesake/shared.veln",
                &format!("mod shared\n{declaration}"),
            );
            assign_recovery_package(&mut namesake, "namesake");
            let declarations = if reverse {
                [namesake, selected]
            } else {
                [selected, namesake]
            };
            for declarations in declarations {
                extend_recovery_declarations(&mut module, declarations);
            }

            let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
            let selected_invalid = reachable
                .invalid_names
                .iter()
                .filter(|invalid| invalid.name == invalid_name)
                .collect::<Vec<_>>();
            assert_eq!(
                selected_invalid.len(),
                1,
                "carrier={carrier}, reverse={reverse}: {:#?}",
                reachable.invalid_names,
            );
            assert_eq!(
                selected_invalid[0].span.file.as_str(),
                "selected/shared.veln",
                "carrier={carrier}, reverse={reverse}",
            );
        }
    }
}

#[test]
fn run_entry_filters_unreachable_invalid_non_function_names() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  1\n",
        "end\n",
        "fn Bad() -> Int\n",
        "  2\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "pub fn Exported = Bad\n",
        "pub type exported = item\n",
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "handler ask(Context: Int) for Ask\n",
        "  value() => Context\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(reachable.aliases.is_empty(), "{:#?}", reachable.aliases);
    assert!(reachable.handlers.is_empty(), "{:#?}", reachable.handlers);
}

#[test]
fn run_entry_keeps_invalid_type_names_referenced_by_reachable_signature() {
    let module = lower(concat!(
        "type item\n",
        "  value\n",
        "end\n",
        "fn main() -> item\n",
        "  1\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_does_not_reach_invalid_type_from_local_value_spelling() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  let item = 1\n",
        "  item\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_match_binding_shadows_invalid_constructor_recovery() {
    let module = lower(concat!(
        "fn main(input: Int) -> Int\n",
        "  match input\n",
        "    item => item\n",
        "  end\n",
        "end\n",
        "type Item\n",
        "  item\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_reach_invalid_type_from_record_field_spelling() {
    let module = lower(concat!(
        "fn main() -> {item: Int}\n",
        "  {item: 1}\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_reach_invalid_alias_from_return_type_spelling() {
    let module = lower(concat!(
        "type Item\n",
        "  Value\n",
        "end\n",
        "fn main() -> Item\n",
        "  Value\n",
        "end\n",
        "fn good() -> Item\n",
        "  Value\n",
        "end\n",
        "pub fn Item = good\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(reachable.aliases.is_empty(), "{:#?}", reachable.aliases);
}

#[test]
fn run_entry_keeps_reachable_invalid_function_alias_name() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Exported()\n",
        "end\n",
        "fn good() -> Int\n",
        "  1\n",
        "end\n",
        "pub fn Exported = good\n",
        "pub fn Unreachable = good\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["Exported"]);
    assert!(
        reachable
            .aliases
            .iter()
            .any(|alias| alias.name.as_deref() == Some("Exported"))
    );
    assert!(
        reachable
            .aliases
            .iter()
            .all(|alias| alias.name.as_deref() != Some("Unreachable")),
        "unreachable invalid aliases must not materialize: {:#?}",
        reachable.aliases
    );
}

#[test]
fn run_entry_keeps_invalid_constructor_referenced_by_reachable_expression_path() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  value\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_keeps_unique_invalid_constructor_call_by_arity() {
    let module = lower(concat!(
        "fn main() -> item\n",
        "  value(1)\n",
        "end\n",
        "type item\n",
        "  value(Int)\n",
        "end\n",
        "type other\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item", "value"]);
}

#[test]
fn run_entry_keeps_only_selected_invalid_constructor_in_valid_type() {
    let module = lower(concat!(
        "fn main() -> Item\n",
        "  value(1)\n",
        "end\n",
        "type Item\n",
        "  value(Int)\n",
        "  other(Int)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["value"]);
}

#[test]
fn run_entry_keeps_invalid_type_for_reachable_valid_nullary_constructor() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Value\n",
        "end\n",
        "type item\n",
        "  Value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item"]);
}

#[test]
fn run_entry_keeps_invalid_type_for_reachable_valid_payload_constructor() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Payload(1)\n",
        "end\n",
        "type item\n",
        "  Payload(Int)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["item"]);
}

#[test]
fn run_entry_does_not_choose_ambiguous_owned_constructor_recovery_with_same_owner_span() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Value\n",
        "end\n",
        "type item\n",
        "  Value\n",
        "  Value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_reach_unreachable_invalid_type_with_valid_constructor() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  1\n",
        "end\n",
        "type item\n",
        "  Value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_choose_ambiguous_constructor_recovery() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  value\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "type other\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_does_not_choose_cross_class_recovery_ambiguity() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Bad(1)\n",
        "end\n",
        "type item\n",
        "  Bad(Int)\n",
        "end\n",
        "fn Bad(value: Int) -> Int\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert_eq!(
        reachable
            .functions
            .iter()
            .filter(|function| {
                function.kind == FunctionKind::Function && function.name.as_deref() == Some("Bad")
            })
            .count(),
        0
    );
}

#[test]
fn run_entry_filters_same_name_recovery_peers_by_call_arity() {
    let module = lower(concat!(
        "fn main() -> Int\n",
        "  Bad(1)\n",
        "end\n",
        "fn Bad(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn Bad(left: Int, right: Int) -> Int\n",
        "  left + right\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["Bad"]);
    assert_eq!(
        reachable
            .functions
            .iter()
            .filter(|function| {
                function.kind == FunctionKind::Function && function.name.as_deref() == Some("Bad")
            })
            .count(),
        1
    );
}

#[test]
fn run_entry_uses_valid_constructor_before_same_spelled_function_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  Bad\n",
        "end\n",
        "fn main() -> Item\n",
        "  Bad\n",
        "end\n",
        "fn Bad() -> Item\n",
        "  Bad\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_uses_valid_function_value_before_constructor_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  bad\n",
        "end\n",
        "fn bad() -> Int\n",
        "  1\n",
        "end\n",
        "fn main() -> Int\n",
        "  let callable: fn() -> Int = bad\n",
        "  callable()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_uses_valid_function_arity_error_before_constructor_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  good(Int)\n",
        "end\n",
        "fn good() -> Int\n",
        "  7\n",
        "end\n",
        "fn main() -> Int\n",
        "  good(1)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_uses_valid_constructor_arity_error_before_function_recovery() {
    let module = lower(concat!(
        "type Item\n",
        "  Bad(Int)\n",
        "end\n",
        "fn Bad() -> Item\n",
        "  Bad(1)\n",
        "end\n",
        "fn main() -> Item\n",
        "  Bad()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}
