use super::*;

#[test]
fn run_entry_keeps_schema_decode_expression_in_entry_function() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "schema PacketWire\n",
                "  format binary\n",
                "  length: UInt8\n",
                "end\n",
                "\n",
                "pub fn main(view: ByteView, base: ByteOffset) -> DecodeStep<{length: Int}>\n",
                "  decode PacketWire from view at base\n",
                "end\n",
            ),
        )],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
        vec![(Some("main"), FunctionKind::Function, Some("main"))]
    );
}

#[test]
fn run_entry_keeps_schema_encode_expression_in_entry_function() {
    let project = Project {
        root: ".".into(),
        files: vec![SourceFile::new(
            "main.veln",
            concat!(
                "schema PacketWire\n",
                "  format binary\n",
                "  length: UInt8\n",
                "end\n",
                "\n",
                "pub fn main(packet: {length: Int}) -> Result<ByteChunk, EncodeError>\n",
                "  encode PacketWire from packet\n",
                "end\n",
            ),
        )],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
        vec![(Some("main"), FunctionKind::Function, Some("main"))]
    );
}

#[test]
fn run_entry_can_reach_contract_helper() {
    let module = lower(concat!(
        "fn positive(value: Int) -> Bool\n",
        "  value > 0\n",
        "end\n",
        "pub fn main(value: Int) -> output: Int\n",
        "  ensure positive(output)\n",
        "  value\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Function, Some("positive")),
            (FunctionKind::Function, Some("main")),
        ]
    );
}

#[test]
fn run_entry_can_reach_contract_function_value() {
    let module = lower(concat!(
        "fn accepts(job: fn() -> Bool) -> Bool\n",
        "  job()\n",
        "end\n",
        "fn ready() -> Bool\n",
        "  true\n",
        "end\n",
        "pub fn main() -> ()\n",
        "  require accepts(ready)\n",
        "  ()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Function, Some("accepts")),
            (FunctionKind::Function, Some("ready")),
            (FunctionKind::Function, Some("main")),
        ]
    );
}

#[test]
fn run_entry_can_reach_qualified_contract_helper() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::rules\n",
                    "pub fn main(value: Int) -> output: Int\n",
                    "  ensure app::rules::positive(output)\n",
                    "  value\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/rules.veln",
                concat!(
                    "pub fn positive(value: Int) -> Bool\n",
                    "  value > 0\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::rules"), FunctionKind::Function, Some("positive")),
        ]
    );
}

#[test]
fn run_entry_can_reach_imported_qualified_call() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::util\n",
                    "pub fn main() -> Int\n",
                    "  app::util::value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/util.veln",
                concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::util"), FunctionKind::Function, Some("value")),
        ]
    );
}

#[test]
fn run_entry_can_reach_imported_alias_target() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::api\n",
                    "pub fn main() -> Int\n",
                    "  app::api::twice(21)\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/api.veln",
                concat!("use app::impl\n", "pub fn twice = app::impl::double\n",),
            ),
            SourceFile::new(
                "app/impl.veln",
                concat!(
                    "fn double(value: Int) -> Int\n",
                    "  value + value\n",
                    "end\n",
                ),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::impl"), FunctionKind::Function, Some("double")),
        ]
    );
}

#[test]
fn run_entry_can_reach_qualified_contract_function_value() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::rules\n",
                    "fn accepts(job: fn() -> Bool) -> Bool\n",
                    "  job()\n",
                    "end\n",
                    "pub fn main() -> ()\n",
                    "  require accepts(app::rules::ready)\n",
                    "  ()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/rules.veln",
                concat!("pub fn ready() -> Bool\n", "  true\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
            (Some("app::main"), FunctionKind::Function, Some("accepts")),
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::rules"), FunctionKind::Function, Some("ready")),
        ]
    );
}

#[test]
fn imported_reachability_keeps_module_specific_function_names() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "use app::util\n",
                    "fn value() -> Int\n",
                    "  _\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  app::util::value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/util.veln",
                concat!("pub fn value() -> Int\n", "  1\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
            (Some("app::main"), FunctionKind::Function, Some("main")),
            (Some("app::util"), FunctionKind::Function, Some("value")),
        ]
    );
}

#[test]
fn bare_reachability_keeps_current_module_function_names() {
    let project = Project {
        root: ".".into(),
        files: vec![
            SourceFile::new(
                "app/main.veln",
                concat!(
                    "fn value() -> Int\n",
                    "  1\n",
                    "end\n",
                    "pub fn main() -> Int\n",
                    "  value()\n",
                    "end\n",
                ),
            ),
            SourceFile::new(
                "app/other.veln",
                concat!("fn value() -> Int\n", "  _\n", "end\n",),
            ),
        ],
        manifest: None,
    };
    let (module, diagnostics) = load_surface_module(&project);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
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
            (Some("app::main"), FunctionKind::Function, Some("value")),
            (Some("app::main"), FunctionKind::Function, Some("main")),
        ]
    );
}

#[test]
fn local_binding_shadowing_function_name_does_not_reach_function() {
    let module = lower(concat!(
        "fn helper() -> Int\n",
        "  _\n",
        "end\n",
        "pub fn main() -> Int\n",
        "  let helper = 1\n",
        "  helper\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn match_binding_shadowing_function_name_does_not_reach_function() {
    let module = lower(concat!(
        "fn helper() -> Int\n",
        "  _\n",
        "end\n",
        "pub fn main(value: Option<Int>) -> Int\n",
        "  match value\n",
        "    Some(helper) => helper\n",
        "    None => 0\n",
        "  end\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn run_entry_does_not_reach_qualified_call_without_import_alias() {
    let module = lower(concat!(
        "pub fn main() -> Int\n",
        "  util::value()\n",
        "end\n",
        "fn value() -> Int\n",
        "  _\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn contract_reachability_ignores_function_names_inside_strings() {
    let module = lower(concat!(
        "fn positive(value: Int) -> Bool\n",
        "  value > 0\n",
        "end\n",
        "pub fn main() -> output: String\n",
        "  ensure \"positive(\" == output\n",
        "  \"positive(\"\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("main"))]);
}

#[test]
fn run_entry_does_not_include_tests() {
    let module = lower(concat!(
        "test helper() -> ()\n",
        "  ()\n",
        "end\n",
        "fn foo() -> ()\n",
        "  ()\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "foo", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(functions, vec![(FunctionKind::Function, Some("foo"))]);
}

#[test]
fn run_entry_reaches_spawn_with_context_function_value() {
    let module = lower(concat!(
        "fn combine(context: {payload: String, suffix: String}) -> String effects [concurrency]\n",
        "  suffix(context.suffix)\n",
        "end\n",
        "fn suffix(value: String) -> String\n",
        "  value\n",
        "end\n",
        "pub fn main() -> Task<String> effects [concurrency]\n",
        "  task::spawn_with(combine, {payload: \"body\", suffix: \"tail\"})\n",
        "end\n",
    ));

    let reachable = reachable_entry_module(&module, "main", FunctionKind::Function);
    let functions = reachable
        .functions
        .iter()
        .map(|function| (function.kind, function.name.as_deref()))
        .collect::<Vec<_>>();

    assert_eq!(
        functions,
        vec![
            (FunctionKind::Function, Some("combine")),
            (FunctionKind::Function, Some("suffix")),
            (FunctionKind::Function, Some("main")),
        ]
    );
}
