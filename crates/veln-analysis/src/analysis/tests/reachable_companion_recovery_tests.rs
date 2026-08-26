use veln_ast::FunctionKind;
use veln_project::Project;
use veln_source::SourceFile;

use crate::surface::{load_surface_module, reachable_entry_module};

#[test]
fn companion_test_does_not_materialize_target_function_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new("math.veln", concat!("fn Bad() -> Int\n", "  1\n", "end\n")),
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test companion_rejects_target_recovery() -> ()\n",
                    "  let observed = math::Bad()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(
        &module,
        "companion_rejects_target_recovery",
        FunctionKind::Test,
    );

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(
        reachable.functions.iter().all(|function| {
            function.module_name.as_deref() != Some("math")
                || function.name.as_deref() != Some("Bad")
        }),
        "{:#?}",
        reachable.functions
    );
}

#[test]
fn companion_test_does_not_materialize_target_binding_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn helper() -> Int\n",
                    "  let Bad: fn() -> Int = helper\n",
                    "  Bad()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "\n",
                    "test companion_rejects_target_binding_recovery() -> ()\n",
                    "  let value = math::helper()\n",
                    "  let observed = math::Bad()\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(
        &module,
        "companion_rejects_target_binding_recovery",
        FunctionKind::Test,
    );

    let invalid_names = reachable
        .invalid_names
        .iter()
        .map(|invalid| invalid.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(invalid_names, vec!["Bad"]);
    assert!(
        reachable.functions.iter().all(|function| {
            function.module_name.as_deref() != Some("math")
                || function.name.as_deref() != Some("Bad")
        }),
        "{:#?}",
        reachable.functions
    );
}

#[test]
fn target_run_does_not_materialize_companion_function_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.veln",
                concat!(
                    "use math__test_companion\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  math__test_companion::Bad()\n",
                    "end\n"
                ),
            ),
            SourceFile::new(
                "math.test.veln",
                concat!("fn Bad() -> Int\n", "  1\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(
        reachable.functions.iter().all(|function| {
            function.module_name.as_deref() != Some("math__test_companion")
                || function.name.as_deref() != Some("Bad")
        }),
        "{:#?}",
        reachable.functions
    );
}

#[test]
fn target_run_does_not_materialize_companion_binding_recovery() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.veln",
                concat!(
                    "use math__test_companion\n",
                    "\n",
                    "fn main() -> Int\n",
                    "  math__test_companion::Bad()\n",
                    "end\n"
                ),
            ),
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "fn helper() -> Int\n",
                    "  let Bad: fn() -> Int = helper\n",
                    "  Bad()\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);

    assert!(
        reachable.invalid_names.is_empty(),
        "{:#?}",
        reachable.invalid_names
    );
    assert!(
        reachable.functions.iter().all(|function| {
            function.module_name.as_deref() != Some("math__test_companion")
                || function.name.as_deref() != Some("Bad")
        }),
        "{:#?}",
        reachable.functions
    );
}
