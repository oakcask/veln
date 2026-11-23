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
    for (statement, expected) in evidence.body.iter().take(4).zip(expected) {
        let CoreStmtKind::Let { expr, .. } = &statement.kind else {
            panic!("expected state allocation binding");
        };
        assert_eq!(&expr.ty, expected);
    }
    for statement in evidence.body.iter().skip(4).take(4) {
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
    for (statement, expected) in evidence.body.iter().take(4).zip(expected) {
        let IrStmtKind::Let { value, .. } = &statement.kind else {
            panic!("expected IR state allocation binding");
        };
        assert_eq!(&value.ty, expected);
    }
}

#[test]
fn io_provider_atomic_state_keeps_concrete_types_through_core_and_ir() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\n",
            "  State(transition: fn(T) -> T)\n",
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
            "  State(transition: fn(T) -> T)\n",
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
