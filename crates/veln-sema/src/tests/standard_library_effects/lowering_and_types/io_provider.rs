use super::*;
use veln_core::{CheckedProgram, CoreFunction};
use veln_ir::{IrFunction, TypedProgram};

fn standard_io_provider_module(source: &SourceFile) -> SurfaceModule {
    let mut module = lower_standard_package_callables(source);
    for function in &mut module.functions {
        function.module_name = Some("std::io_provider".to_string());
    }
    for declaration in &mut module.types {
        declaration.module_name = Some("std::io_provider".to_string());
        declaration.package_name = Some(veln_stdlib::PACKAGE_NAME.to_string());
    }
    module
}

fn evidence_core(program: &CheckedProgram) -> &CoreFunction {
    program
        .functions
        .iter()
        .find(|function| function.name.ends_with("evidence"))
        .expect("evidence function should lower")
}

fn evidence_ir(program: &TypedProgram) -> &IrFunction {
    program
        .functions
        .iter()
        .find(|function| function.name.ends_with("evidence"))
        .expect("evidence function should lower to IR")
}

fn assert_concrete_core_state_types(evidence: &CoreFunction, expected: &[CoreType]) {
    for (statement, expected) in evidence.body.iter().take(expected.len()).zip(expected) {
        let CoreStmtKind::Let { expr, .. } = &statement.kind else {
            panic!("expected state allocation binding");
        };
        assert_eq!(&expr.ty, expected);
    }
    for statement in evidence.body.iter().skip(expected.len()).take(4) {
        let expr = match &statement.kind {
            CoreStmtKind::Let { expr, .. } | CoreStmtKind::Expr { expr } => expr,
            _ => panic!("expected state update statement: {statement:#?}"),
        };
        assert_eq!(expr.ty, CoreType::int());
        assert!(matches!(
            expr.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::CallbackBoundary { .. },
                ..
            }
        ));
    }
}

fn assert_concrete_ir_state_types(evidence: &IrFunction, expected: &[CoreType]) {
    for (statement, expected) in evidence.body.iter().take(expected.len()).zip(expected) {
        let IrStmtKind::Let { value, .. } = &statement.kind else {
            panic!("expected IR state allocation binding");
        };
        assert_eq!(&value.ty, expected);
    }
}

#[test]
fn exact_io_provider_companion_cannot_match_constructor_free_state() {
    let target_source = SourceFile::new(
        "io_provider.veln",
        concat!("mod io_provider\n", "pub type State<T>\n", "end\n",),
    );
    let companion_source = SourceFile::new(
        "io_provider.test.veln",
        concat!(
            "mod io_provider__test_companion\n",
            "use io_provider\n",
            "fn inspect(cell: io_provider::State<Int>) -> ()\n",
            "  match cell\n",
            "    io_provider::State::State(_) => ()\n",
            "  end\n",
            "end\n",
            "test cannot_match_state() -> ()\n",
            "  let callback: fn(io_provider::State<Int>) -> () = inspect\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_parsed = parse(&target_source);
    let companion_parsed = parse(&companion_source);
    assert!(
        target_parsed.diagnostics.is_empty(),
        "{:#?}",
        target_parsed.diagnostics
    );
    assert!(
        companion_parsed.diagnostics.is_empty(),
        "{:#?}",
        companion_parsed.diagnostics
    );
    let mut target = lower_surface_ast(&target_parsed.tree);
    let mut companion = lower_surface_ast(&companion_parsed.tree);
    for use_decl in &mut companion.uses {
        use_decl.importing_package = Some(veln_stdlib::PACKAGE_NAME.to_string());
        use_decl.package = Some(veln_stdlib::PACKAGE_NAME.to_string());
    }
    for function in &mut companion.functions {
        function.package_name = Some(veln_stdlib::PACKAGE_NAME.to_string());
    }
    target.module = companion.module;
    target.uses = companion.uses;
    target.functions.extend(companion.functions);

    let diagnostics = analyze_surface_module(&target);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message.contains("io_provider::State::State")
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn io_provider_atomic_state_keeps_concrete_types_through_core_and_ir() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\n",
            "end\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\n",
            "end\n",
            "pub fn update(cell: State<T>, context: C, action: fn(T, C) -> {state: T, result: R}) -> R effects [concurrency]\n",
            "  prelude_builtin::io_provider_state_update(cell, context, action)\n",
            "end\n",
            "type Sample\n",
            "  Sample(value: Int)\n",
            "end\n",
            "fn string_step(value: String, context: ()) -> {state: String, result: Int}\n",
            "  {state: value, result: 1}\n",
            "end\n",
            "fn record_step(value: {name: String}, context: ()) -> {state: {name: String}, result: Int}\n",
            "  {state: value, result: 2}\n",
            "end\n",
            "fn list_step(value: List<Int>, context: ()) -> {state: List<Int>, result: Int}\n",
            "  {state: value, result: 3}\n",
            "end\n",
            "fn sample_step(value: Sample, context: ()) -> {state: Sample, result: Int}\n",
            "  {state: value, result: 4}\n",
            "end\n",
            "pub fn evidence() -> () effects [concurrency]\n",
            "  let strings = state(\"value\")\n",
            "  let records = state({name: \"value\"})\n",
            "  let lists = state(list_cons(1, list_nil()))\n",
            "  let samples = state(Sample(1))\n",
            "  let options: State<Option<Int>> = state(None)\n",
            "  let string_result = update(strings, (), string_step)\n",
            "  let record_result = update(records, (), record_step)\n",
            "  let list_result = update(lists, (), list_step)\n",
            "  let sample_result = update(samples, (), sample_step)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = standard_io_provider_module(&source);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);

    let expected = [
        CoreType::string(),
        CoreType::Record(
            [("name".to_string(), CoreType::string())]
                .into_iter()
                .collect(),
        ),
        CoreType::named("List", vec![CoreType::int()]),
        CoreType::named("Sample", Vec::new()),
        CoreType::named("Option", vec![CoreType::int()]),
    ]
    .map(|item| CoreType::named("State", vec![item]));
    let core = lowered
        .core
        .expect("atomic state should build checked core");
    assert_concrete_core_state_types(evidence_core(&core), &expected);
    let ir = lowered.ir.expect("atomic state should lower to typed IR");
    assert_concrete_ir_state_types(evidence_ir(&ir), &expected);
}

fn assert_foreign_signatures(environment: &TypeEnvironment) {
    let allocation = environment
        .function("state")
        .expect("foreign state signature");
    assert_eq!(allocation.package_name.as_deref(), Some("foreign-provider"));
    assert_eq!(allocation.params.as_slice(), &[Type::int()]);
    assert!(matches!(
        &allocation.return_type,
        Type::Named {
            name,
            identity,
            args,
        } if name == "ForeignState"
            && identity == "foreign-provider::std::io_provider::ForeignState"
            && args.is_empty()
    ));
    let update = environment
        .function("update")
        .expect("foreign update signature");
    assert_eq!(update.package_name.as_deref(), Some("foreign-provider"));
    assert_eq!(update.params.len(), 2);
    assert!(matches!(
        &update.params[0],
        Type::Named {
            name,
            identity,
            args,
        } if name == "ForeignState"
            && identity == "foreign-provider::std::io_provider::ForeignState"
            && args.is_empty()
    ));
    assert_eq!(update.params[1], Type::string());
    assert_eq!(update.return_type, Type::string());
}

fn assert_foreign_core(evidence: &CoreFunction, foreign_state: &CoreType) {
    let CoreStmtKind::Let {
        expr: allocation, ..
    } = &evidence.body[0].kind
    else {
        panic!("expected foreign state allocation binding");
    };
    assert_eq!(&allocation.ty, foreign_state);
    assert!(matches!(
        &allocation.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::Function(name),
            ..
        } if name == "foreign-provider::std::io_provider::state"
    ));
    let CoreStmtKind::Return { expr: update } = &evidence.body[1].kind else {
        panic!("expected foreign state update expression");
    };
    assert_eq!(update.ty, CoreType::string());
    assert!(matches!(
        &update.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::Function(name),
            args,
        } if name == "foreign-provider::std::io_provider::update"
            && args.len() == 2
            && &args[0].ty == foreign_state
            && args[1].ty == CoreType::string()
    ));
}

fn assert_foreign_ir(evidence: &IrFunction, foreign_state: &CoreType) {
    let IrStmtKind::Let {
        value: allocation, ..
    } = &evidence.body[0].kind
    else {
        panic!("expected foreign IR state allocation binding");
    };
    assert_eq!(&allocation.ty, foreign_state);
    let IrStmtKind::Return { value: update } = &evidence.body[1].kind else {
        panic!("expected foreign IR state update expression");
    };
    assert_eq!(update.ty, CoreType::string());
    assert!(matches!(
        &update.kind,
        IrExprKind::Call {
            target: IrCallTarget::Function(name),
            args,
        } if name == "foreign-provider::std::io_provider::update"
            && args.len() == 2
            && &args[0].ty == foreign_state
            && args[1].ty == CoreType::string()
    ));
}

#[test]
fn foreign_io_provider_namesakes_keep_declared_types_through_core_and_ir() {
    let source = SourceFile::new(
        "foreign_io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type ForeignState\n",
            "  ForeignState(value: String)\n",
            "end\n",
            "pub fn state(initial: Int) -> ForeignState\n",
            "  ForeignState(\"foreign\")\n",
            "end\n",
            "pub fn update(cell: ForeignState, replacement: String) -> String\n",
            "  replacement\n",
            "end\n",
            "pub fn evidence() -> String\n",
            "  let cell = state(1)\n",
            "  update(cell, \"replacement\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let mut module = lower_surface_ast(&parsed.tree);
    for function in &mut module.functions {
        function.module_name = Some("std::io_provider".to_string());
        function.package_name = Some("foreign-provider".to_string());
    }
    for declaration in &mut module.types {
        declaration.module_name = Some("std::io_provider".to_string());
        declaration.package_name = Some("foreign-provider".to_string());
    }

    assert_foreign_signatures(&TypeEnvironment::from_module(&module));
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let foreign_state = CoreType::named("ForeignState", Vec::new());
    let core = lowered
        .core
        .expect("foreign io-provider namesakes should build checked core");
    assert_foreign_core(evidence_core(&core), &foreign_state);
    let ir = lowered
        .ir
        .expect("foreign io-provider namesakes should lower to typed IR");
    assert_foreign_ir(evidence_ir(&ir), &foreign_state);
}

fn assert_core_allocation_and_update_types(evidence: &CoreFunction, state_type: &CoreType) {
    for statement in evidence.body.iter().take(2) {
        let CoreStmtKind::Let { expr, .. } = &statement.kind else {
            panic!("expected state allocation binding");
        };
        assert_eq!(&expr.ty, state_type);
    }
    for statement in evidence.body.iter().skip(2).take(2) {
        let CoreStmtKind::Let { expr, .. } = &statement.kind else {
            panic!("expected state update binding");
        };
        assert_eq!(expr.ty, CoreType::int());
    }
}

fn assert_ir_allocation_and_update_types(evidence: &IrFunction, state_type: &CoreType) {
    for statement in evidence.body.iter().take(2) {
        let IrStmtKind::Let { value, .. } = &statement.kind else {
            panic!("expected IR state allocation binding");
        };
        assert_eq!(&value.ty, state_type);
    }
    for statement in evidence.body.iter().skip(2).take(2) {
        let IrStmtKind::Let { value, .. } = &statement.kind else {
            panic!("expected IR state update binding");
        };
        assert_eq!(value.ty, CoreType::int());
    }
}

#[test]
fn io_provider_atomic_state_uses_lowered_field_and_control_flow_types() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\n",
            "end\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\n",
            "end\n",
            "pub fn update(cell: State<T>, context: C, action: fn(T, C) -> {state: T, result: R}) -> R effects [concurrency]\n",
            "  prelude_builtin::io_provider_state_update(cell, context, action)\n",
            "end\n",
            "fn step(value: {name: String}, context: ()) -> {state: {name: String}, result: Int}\n",
            "  {state: value, result: 7}\n",
            "end\n",
            "pub fn evidence(input: {name: String}, holder: {action: fn({name: String}, ()) -> {state: {name: String}, result: Int}}, choose: Bool) -> () effects [concurrency]\n",
            "  let field_cell = state({name: input.name})\n",
            "  let selected_cell = state(if choose\n",
            "    {name: input.name}\n",
            "  else\n",
            "    input\n",
            "  end)\n",
            "  let field_result = update(field_cell, (), holder.action)\n",
            "  let selected_result = update(selected_cell, (), if choose\n",
            "    holder.action\n",
            "  else\n",
            "    step\n",
            "  end)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = standard_io_provider_module(&source);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let state_type = CoreType::named(
        "State",
        vec![CoreType::Record(vec![(
            "name".to_string(),
            CoreType::string(),
        )])],
    );
    let core = lowered
        .core
        .expect("field-derived state calls should build checked core");
    assert_core_allocation_and_update_types(evidence_core(&core), &state_type);
    let ir = lowered
        .ir
        .expect("field-derived state calls should lower to typed IR");
    assert_ir_allocation_and_update_types(evidence_ir(&ir), &state_type);
}

#[test]
fn io_provider_atomic_state_preserves_joined_initializer_types_through_core_and_ir() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\n",
            "end\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\n",
            "end\n",
            "pub fn evidence(choose: Bool, first: {label: String, detail: {state: Option<Int>::Some}}, second: {detail: {state: Option<Int>::None}, label: String}) -> () effects [concurrency]\n",
            "  let list_forward = state([first, second])\n",
            "  let list_reverse = state([second, first])\n",
            "  let if_forward = state(if choose\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end)\n",
            "  let if_reverse = state(if choose\n",
            "    second\n",
            "  else\n",
            "    first\n",
            "  end)\n",
            "  let match_forward = state(match choose\n",
            "    true => first\n",
            "    false => second\n",
            "  end)\n",
            "  let match_reverse = state(match choose\n",
            "    true => second\n",
            "    false => first\n",
            "  end)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = standard_io_provider_module(&source);
    let semantic_diagnostics = analyze_surface_module(&module);
    assert!(semantic_diagnostics.is_empty(), "{semantic_diagnostics:#?}");

    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let record_type = CoreType::Record(vec![
        (
            "detail".to_string(),
            CoreType::Record(vec![(
                "state".to_string(),
                CoreType::option(CoreType::int()),
            )]),
        ),
        ("label".to_string(), CoreType::string()),
    ]);
    let cases = [
        ("list_forward", CoreType::vec(record_type.clone())),
        ("list_reverse", CoreType::vec(record_type.clone())),
        ("if_forward", record_type.clone()),
        ("if_reverse", record_type.clone()),
        ("match_forward", record_type.clone()),
        ("match_reverse", record_type),
    ];
    let core = lowered
        .core
        .expect("joined state initializers should build checked core");
    let core_evidence = evidence_core(&core);
    for (index, (name, item_type)) in cases.iter().enumerate() {
        let CoreStmtKind::Let { expr, .. } = &core_evidence.body[index].kind else {
            panic!("expected state allocation binding for {name}");
        };
        let state_type = CoreType::named("State", vec![item_type.clone()]);
        assert_eq!(expr.ty, state_type, "checked Core result for {name}");
        let CoreExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected checked Core state call for {name}");
        };
        assert_eq!(
            args[0].ty, *item_type,
            "checked Core initializer for {name}"
        );
        match &args[0].kind {
            CoreExprKind::List(items) => assert!(
                items
                    .iter()
                    .all(|item| item.ty == record_type_for_case(item_type)),
                "checked Core list contributions for {name}: {items:#?}",
            ),
            CoreExprKind::Match { arms, .. } => assert!(
                arms.iter().all(|arm| arm.expr.ty == *item_type),
                "checked Core control-flow contributions for {name}: {arms:#?}",
            ),
            kind => panic!("unexpected checked Core initializer for {name}: {kind:#?}"),
        }
    }

    let ir = lowered
        .ir
        .expect("joined state initializers should lower to typed IR");
    let ir_evidence = evidence_ir(&ir);
    for (index, (name, item_type)) in cases.iter().enumerate() {
        let IrStmtKind::Let { value, .. } = &ir_evidence.body[index].kind else {
            panic!("expected IR state allocation binding for {name}");
        };
        let state_type = CoreType::named("State", vec![item_type.clone()]);
        assert_eq!(value.ty, state_type, "typed IR result for {name}");
        let IrExprKind::Call { args, .. } = &value.kind else {
            panic!("expected typed IR state call for {name}");
        };
        assert_eq!(args[0].ty, *item_type, "typed IR initializer for {name}");
        match &args[0].kind {
            IrExprKind::List(items) => assert!(
                items
                    .iter()
                    .all(|item| item.ty == record_type_for_case(item_type)),
                "typed IR list contributions for {name}: {items:#?}",
            ),
            IrExprKind::Match { arms, .. } => assert!(
                arms.iter().all(|arm| arm.value.ty == *item_type),
                "typed IR control-flow contributions for {name}: {arms:#?}",
            ),
            kind => panic!("unexpected typed IR initializer for {name}: {kind:#?}"),
        }
    }
}

#[test]
fn io_provider_atomic_state_preserves_width_joined_initializer_types_in_both_orders() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\n",
            "end\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\n",
            "end\n",
            "pub fn evidence(choose: Bool, wider: {label: String, extra: Int}, narrower: {label: String}, holder: {wider: {label: String, extra: Int}, narrower: {label: String}}, wide_option: Option<{label: String, extra: Int}>, narrow_option: Option<{label: String}>) -> () effects [concurrency]\n",
            "  let list_forward = state([wider, narrower])\n",
            "  let list_reverse = state([narrower, wider])\n",
            "  let if_forward = state(if choose wider else narrower end)\n",
            "  let if_reverse = state(if choose narrower else wider end)\n",
            "  let match_forward = state(match choose\n",
            "    true => wider\n",
            "    false => narrower\n",
            "  end)\n",
            "  let match_reverse = state(match choose\n",
            "    true => narrower\n",
            "    false => wider\n",
            "  end)\n",
            "  let field_if_forward = state(if choose holder.wider else holder.narrower end)\n",
            "  let field_if_reverse = state(if choose holder.narrower else holder.wider end)\n",
            "  let pattern_match_forward = state(match wide_option\n",
            "    Some(value) => value\n",
            "    None => holder.narrower\n",
            "  end)\n",
            "  let pattern_match_reverse = state(match narrow_option\n",
            "    Some(value) => value\n",
            "    None => holder.wider\n",
            "  end)\n",
            "  let list_forward_semantic: State<Vec<{label: String}>> = list_forward\n",
            "  let list_reverse_semantic: State<Vec<{label: String}>> = list_reverse\n",
            "  let if_forward_semantic: State<{label: String}> = if_forward\n",
            "  let if_reverse_semantic: State<{label: String}> = if_reverse\n",
            "  let match_forward_semantic: State<{label: String}> = match_forward\n",
            "  let match_reverse_semantic: State<{label: String}> = match_reverse\n",
            "  let field_if_forward_semantic: State<{label: String}> = field_if_forward\n",
            "  let field_if_reverse_semantic: State<{label: String}> = field_if_reverse\n",
            "  let pattern_match_forward_semantic: State<{label: String}> = pattern_match_forward\n",
            "  let pattern_match_reverse_semantic: State<{label: String}> = pattern_match_reverse\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = standard_io_provider_module(&source);
    let semantic_diagnostics = analyze_surface_module(&module);
    assert!(semantic_diagnostics.is_empty(), "{semantic_diagnostics:#?}");

    let record_type = CoreType::Record(vec![("label".to_string(), CoreType::string())]);
    let cases = [
        ("list_forward", CoreType::vec(record_type.clone())),
        ("list_reverse", CoreType::vec(record_type.clone())),
        ("if_forward", record_type.clone()),
        ("if_reverse", record_type.clone()),
        ("match_forward", record_type.clone()),
        ("match_reverse", record_type.clone()),
        ("field_if_forward", record_type.clone()),
        ("field_if_reverse", record_type.clone()),
        ("pattern_match_forward", record_type.clone()),
        ("pattern_match_reverse", record_type),
    ];
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered
        .core
        .expect("width-joined state initializers should build checked core");
    assert_joined_core_state_initializers(evidence_core(&core), &cases);
    let ir = lowered
        .ir
        .expect("width-joined state initializers should lower to typed IR");
    assert_joined_ir_state_initializers(evidence_ir(&ir), &cases);
}

fn assert_joined_core_state_initializers(evidence: &CoreFunction, cases: &[(&str, CoreType)]) {
    for (index, (name, item_type)) in cases.iter().enumerate() {
        let CoreStmtKind::Let { expr, .. } = &evidence.body[index].kind else {
            panic!("expected state allocation binding for {name}");
        };
        assert_eq!(
            expr.ty,
            CoreType::named("State", vec![item_type.clone()]),
            "checked Core result for {name}",
        );
        let CoreExprKind::Call { args, .. } = &expr.kind else {
            panic!("expected checked Core state call for {name}");
        };
        assert_eq!(
            args[0].ty, *item_type,
            "checked Core initializer for {name}"
        );
        match &args[0].kind {
            CoreExprKind::List(items) => assert!(
                items
                    .iter()
                    .all(|item| item.ty == record_type_for_case(item_type)),
                "checked Core list contributions for {name}: {items:#?}",
            ),
            CoreExprKind::Match { arms, .. } => assert!(
                arms.iter().all(|arm| arm.expr.ty == *item_type),
                "checked Core control-flow contributions for {name}: {arms:#?}",
            ),
            kind => panic!("unexpected checked Core initializer for {name}: {kind:#?}"),
        }
    }
}

fn assert_joined_ir_state_initializers(evidence: &IrFunction, cases: &[(&str, CoreType)]) {
    for (index, (name, item_type)) in cases.iter().enumerate() {
        let IrStmtKind::Let { value, .. } = &evidence.body[index].kind else {
            panic!("expected IR state allocation binding for {name}");
        };
        assert_eq!(
            value.ty,
            CoreType::named("State", vec![item_type.clone()]),
            "typed IR result for {name}",
        );
        let IrExprKind::Call { args, .. } = &value.kind else {
            panic!("expected typed IR state call for {name}");
        };
        assert_eq!(args[0].ty, *item_type, "typed IR initializer for {name}");
        match &args[0].kind {
            IrExprKind::List(items) => assert!(
                items
                    .iter()
                    .all(|item| item.ty == record_type_for_case(item_type)),
                "typed IR list contributions for {name}: {items:#?}",
            ),
            IrExprKind::Match { arms, .. } => assert!(
                arms.iter().all(|arm| arm.value.ty == *item_type),
                "typed IR control-flow contributions for {name}: {arms:#?}",
            ),
            kind => panic!("unexpected typed IR initializer for {name}: {kind:#?}"),
        }
    }
}

fn record_type_for_case(item_type: &CoreType) -> CoreType {
    item_type.vec_part().unwrap_or(item_type).clone()
}
