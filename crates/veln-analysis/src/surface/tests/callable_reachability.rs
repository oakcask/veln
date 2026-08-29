use super::*;

#[test]
fn test_entry_can_reach_function_callee() {
    let module = lower(concat!(
        "test foo() -> ()\n",
        "  helper()\n",
        "end\n",
        "fn helper() -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("helper")),
        ]
    );
}

#[test]
fn test_entry_can_reach_function_value_reference() {
    let module = lower(concat!(
        "test foo() -> ()\n",
        "  vec_map([1], stringify)\n",
        "  ()\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  \"ok\"\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("stringify")),
        ]
    );
}

#[test]
fn test_entry_reaches_same_shape_variadic_function_value_targets() {
    let module = lower(concat!(
        "test foo(callback: fn(String, ...String) -> String) -> ()\n",
        "  callback(\"prefix\", \"a\", \"b\")\n",
        "  ()\n",
        "end\n",
        "fn join(prefix: String, values: ...String) -> String\n",
        "  prefix\n",
        "end\n",
        "fn fixed(prefix: String, value: String) -> String\n",
        "  prefix\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("join")),
        ]
    );
}

#[test]
fn test_entry_does_not_reach_variadic_function_value_targets_for_too_few_args() {
    let module = lower(concat!(
        "test foo(callback: fn(String, ...String) -> String) -> ()\n",
        "  callback()\n",
        "  ()\n",
        "end\n",
        "fn join(prefix: String, values: ...String) -> String\n",
        "  prefix\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Test, Some("foo"))]);
}

#[test]
fn test_entry_conservatively_reaches_opaque_function_value_call_targets() {
    let module = lower(concat!(
        "test foo() -> Bool\n",
        "  invoke(ready)\n",
        "end\n",
        "fn invoke(job: fn() -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("invoke")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("risky")),
        ]
    );
}

#[test]
fn test_entry_reaches_opaque_function_value_call_targets_with_spaced_type() {
    let module = lower(concat!(
        "test foo() -> Bool\n",
        "  invoke(ready)\n",
        "end\n",
        "fn invoke(job: fn () -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("invoke")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("risky")),
        ]
    );
}

#[test]
fn test_entry_conservatively_reaches_opaque_local_function_value_call_targets() {
    let module = lower(concat!(
        "test foo() -> Bool\n",
        "  invoke()\n",
        "end\n",
        "fn invoke() -> Bool\n",
        "  let job: fn() -> Bool = ready\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Test, Some("foo")),
            (FunctionKind::Function, Some("invoke")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("risky")),
        ]
    );
}

#[test]
fn run_entry_conservatively_reaches_opaque_function_value_call_targets() {
    let standard = lower("mod std::prelude\n");
    let application = lower(concat!(
        "mod app\n",
        "fn invoke(job: fn() -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "fn risky() -> Bool\n",
        "  _\n",
        "end\n",
        "pub fn main() -> Bool\n",
        "  invoke(ready)\n",
        "end\n",
    ));

    let reachable = reachable_entry_module_with_standard_cache(
        &standard,
        &application,
        "main",
        FunctionKind::Function,
        &ReachabilityCache::default(),
    );
    let functions = reachable_function_names(&reachable);

    assert_eq!(
        functions,
        vec![
            ("app", "invoke"),
            ("app", "main"),
            ("app", "ready"),
            ("app", "risky"),
        ]
    );
}

#[test]
fn test_entry_can_reach_qualified_function_value_reference() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main_test.veln",
                concat!(
                    "use app::text\n",
                    "test foo() -> ()\n",
                    "  vec_map([1], app::text::stringify)\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/text.veln",
                concat!(
                    "pub fn stringify(value: Int) -> String\n",
                    "  \"ok\"\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (Some("app::main_test"), FunctionKind::Test, Some("foo")),
            (Some("app::text"), FunctionKind::Function, Some("stringify")),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_push")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_concat")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_append")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_map")
            ),
            (
                Some("std::prelude"),
                FunctionKind::Function,
                Some("vec_map_step")
            ),
        ]
    );
}

#[test]
fn companion_test_entry_reaches_qualified_private_target_function() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test increment_test() -> Int\n",
                    "  math::increment(1)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn increment(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "increment_test", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        functions.contains(&(
            Some("math__test_companion"),
            FunctionKind::Test,
            Some("increment_test")
        )),
        "{functions:#?}"
    );
    assert!(
        functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_test_entry_keeps_qualified_private_target_handler() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test handler_test() -> Int\n",
                    "  handle math::compute() with math::ask(41)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "effect Ask\n",
                    "  value() -> Int\n",
                    "end\n",
                    "fn provide(offset: Int) -> Int\n",
                    "  offset + 1\n",
                    "end\n",
                    "handler ask(offset: Int) handles Ask\n",
                    "  value() => provide(offset)\n",
                    "end\n",
                    "pub fn compute() -> Int effects [Ask]\n",
                    "  perform Ask::value()\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "handler_test", FunctionKind::Test);
    let handlers = reachable
        .handlers
        .iter()
        .map(|handler| (handler.module_name.as_deref(), handler.name.as_deref()))
        .collect::<Vec<_>>();
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        handlers.contains(&(Some("math"), Some("ask"))),
        "{handlers:#?}"
    );
    assert!(
        functions.contains(&(Some("math"), FunctionKind::Function, Some("provide"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_public_alias_cannot_reexport_private_target_function() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "pub fn expose = math::increment\n",
                    "test expose_test() -> Int\n",
                    "  expose(1)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn increment(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "module.companion_public_declaration")
        .unwrap_or_else(|| {
            panic!("expected companion public declaration diagnostic for alias: {diagnostics:#?}")
        });
    assert_eq!(
        detail_string(diagnostic, "reason"),
        Some("public_function_alias")
    );

    let reachable = reachable_entry_module(&module, "expose_test", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        functions.contains(&(
            Some("math__test_companion"),
            FunctionKind::Test,
            Some("expose_test")
        )),
        "{functions:#?}"
    );
    assert!(
        !functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_test_entry_does_not_reach_private_target_function_value() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test increment_value_test() -> Int\n",
                    "  let mapper: fn(Int) -> Int = math::increment\n",
                    "  mapper(1)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "math.veln",
                concat!(
                    "fn increment(value: Int) -> Int\n",
                    "  value + 1\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "increment_value_test", FunctionKind::Test);
    let functions = reachable
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(
        functions.contains(&(
            Some("math__test_companion"),
            FunctionKind::Test,
            Some("increment_value_test")
        )),
        "{functions:#?}"
    );
    assert!(
        !functions.contains(&(Some("math"), FunctionKind::Function, Some("increment"))),
        "{functions:#?}"
    );
}

#[test]
fn companion_call_does_not_change_production_private_inference_reachability() {
    let target = SourceFile::new(
        "math.veln",
        concat!(
            "fn identity(value)\n",
            "  value\n",
            "end\n",
            "pub fn production() -> Int\n",
            "  identity(_)\n",
            "end\n",
        ),
    );
    let project_without_companion = Project {
        root: ".".into(),
        files: vec![target.clone()],
        manifest: None,
    };
    let project_with_companion = Project {
        root: ".".into(),
        files: vec![
            target,
            SourceFile::new(
                "math.test.veln",
                concat!(
                    "use math\n",
                    "test identity_test() -> Int\n",
                    "  math::identity(1)\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };

    let (without_companion, without_diagnostics) = load_surface_module(&project_without_companion);
    let (with_companion, with_diagnostics) = load_surface_module(&project_with_companion);
    assert!(without_diagnostics.is_empty(), "{without_diagnostics:#?}");
    assert!(with_diagnostics.is_empty(), "{with_diagnostics:#?}");

    let production_without =
        reachable_entry_module(&without_companion, "production", FunctionKind::Function);
    let production_with =
        reachable_entry_module(&with_companion, "production", FunctionKind::Function);
    let without_functions = production_without
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
                function
                    .params
                    .iter()
                    .map(|param| param.ty.as_deref())
                    .collect::<Vec<_>>(),
                function.return_type.as_deref(),
            )
        })
        .collect::<Vec<_>>();
    let with_functions = production_with
        .functions
        .iter()
        .map(|function| {
            (
                function.module_name.as_deref(),
                function.kind,
                function.name.as_deref(),
                function
                    .params
                    .iter()
                    .map(|param| param.ty.as_deref())
                    .collect::<Vec<_>>(),
                function.return_type.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(with_functions, without_functions);
}
