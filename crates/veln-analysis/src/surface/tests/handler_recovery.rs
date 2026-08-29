use super::*;

#[test]
fn run_entry_keeps_invalid_bindings_in_reachable_handler() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask(Context: Int) handles Ask\n",
        "  value(Result) => Context + Result\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask(1)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();

    assert_eq!(invalid_names, vec!["Context", "Result"]);
    assert_eq!(reachable.handlers.len(), 1, "{:#?}", reachable.handlers);
}

#[test]
fn run_entry_ignores_invalid_bindings_in_unreachable_handler() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "handler ask(Context: Int) handles Ask\n",
        "  value(Result) => Context + Result\n",
        "end\n",
        "fn main() -> Int\n",
        "  1\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(reachable.handlers.is_empty(), "{:#?}", reachable.handlers);
}

#[test]
fn run_entry_keeps_invalid_type_from_reachable_handler_parameter_annotation() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask(seed: item) handles Ask\n",
        "  value() => 1\n",
        "end\n",
        "handler unreachable(seed: other) handles Ask\n",
        "  value() => 2\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask(value)\n",
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
fn run_entry_keeps_invalid_constructor_from_reachable_handler_clause_expression() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask() handles Ask\n",
        "  value() => value\n",
        "end\n",
        "handler unreachable() handles Ask\n",
        "  value() => other_value\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask()\n",
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
fn run_entry_keeps_invalid_constructor_from_reachable_handler_match_scrutinee() {
    let module = lower(concat!(
        "effect Ask\n",
        "  value() -> Int\n",
        "end\n",
        "type item\n",
        "  value\n",
        "end\n",
        "fn body() -> Int effects [Ask]\n",
        "  perform Ask::value()\n",
        "end\n",
        "handler ask() handles Ask\n",
        "  value() => match value\n",
        "    value => 1\n",
        "  end\n",
        "end\n",
        "handler unreachable() handles Ask\n",
        "  value() => other_value\n",
        "end\n",
        "type other\n",
        "  other_value\n",
        "end\n",
        "fn main() -> Int\n",
        "  handle body() with ask()\n",
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
fn run_entry_does_not_select_imported_function_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use helper\n",
                    "fn main() -> Int\n",
                    "  helper::Bad()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "helper.veln",
                concat!("mod helper\n", "pub fn Bad() -> Int\n", "  1\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}

#[test]
fn run_entry_preserves_qualified_type_references_for_recovery_selection() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app.veln",
                concat!(
                    "mod app\n",
                    "use helper\n",
                    "fn main(input: helper::item) -> Int\n",
                    "  1\n",
                    "end\n",
                    "type item\n",
                    "  Value\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "helper.veln",
                concat!("mod helper\n", "pub type item\n", "  Value\n", "end\n"),
            ),
        ],
        manifest: None,
    };
    let (module, _) = load_surface_module(&project);

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
}
