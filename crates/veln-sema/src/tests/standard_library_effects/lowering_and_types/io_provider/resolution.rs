use super::*;

#[test]
fn qualified_io_provider_state_calls_share_analysis_and_lowering_types() {
    let provider_source = SourceFile::new(
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
        ),
    );
    let app_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app\n",
            "use io_provider as standard from \"std\"\n",
            "fn step(value: {name: String}, context: ()) -> {state: {name: String}, result: Int}\n",
            "  {state: value, result: 1}\n",
            "end\n",
            "pub fn evidence() -> () effects [concurrency]\n",
            "  let cell = standard::state({name: \"value\"})\n",
            "  let result = standard::update(cell, (), step)\n",
            "  ()\n",
            "end\n",
        ),
    );
    let provider = standard_io_provider_module(&provider_source);
    let mut module = lower_surface_ast(&parse(&app_source).tree);
    module.types.extend(provider.types);
    module.functions.extend(provider.functions);
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
        .expect("qualified state calls should build checked core");
    let evidence = evidence_core(&core);
    let CoreStmtKind::Let { expr: state, .. } = &evidence.body[0].kind else {
        panic!("expected qualified state allocation");
    };
    assert_eq!(state.ty, state_type);
    let CoreStmtKind::Let { expr: update, .. } = &evidence.body[1].kind else {
        panic!("expected qualified state update");
    };
    assert_eq!(update.ty, CoreType::int());

    let ir = lowered
        .ir
        .expect("qualified state calls should lower to typed IR");
    let evidence = evidence_ir(&ir);
    let IrStmtKind::Let { value: state, .. } = &evidence.body[0].kind else {
        panic!("expected qualified IR state allocation");
    };
    assert_eq!(state.ty, state_type);
    let IrStmtKind::Let { value: update, .. } = &evidence.body[1].kind else {
        panic!("expected qualified IR state update");
    };
    assert_eq!(update.ty, CoreType::int());
}
