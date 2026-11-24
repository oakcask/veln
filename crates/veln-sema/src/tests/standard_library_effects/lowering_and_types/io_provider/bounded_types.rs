use super::*;
use crate::analysis::{reset_constructor_pattern_scan_work, take_constructor_pattern_scan_work};
use crate::lowering::{
    StateInitializerLoweringWork, reset_state_initializer_lowering_work,
    take_state_initializer_lowering_work,
};

#[test]
fn io_provider_atomic_state_preserves_contravariant_source_adt_join_in_core_and_ir() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\nend\n",
            "type Consumer<T>\n  ConsumerValue(fn(T) -> ())\nend\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\nend\n",
            "pub fn evidence(id_only: Consumer<{id: Int}>, label_only: Consumer<{label: String}>, candidate: Consumer<{id: Int, label: String}>) -> () effects [concurrency]\n",
            "  let cell = state([id_only, label_only, candidate])\n",
            "  ()\nend\n",
        ),
    );
    let module = standard_io_provider_module(&source);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let item = CoreType::named(
        "Consumer",
        vec![CoreType::Record(vec![
            ("id".to_string(), CoreType::int()),
            ("label".to_string(), CoreType::string()),
        ])],
    );
    let expected = CoreType::named("State", vec![CoreType::vec(item)]);
    let core = lowered.core.expect("contravariant join should build Core");
    let CoreStmtKind::Let { expr, .. } = &evidence_core(&core).body[0].kind else {
        panic!("expected state binding");
    };
    assert_eq!(expr.ty, expected);
    let ir = lowered
        .ir
        .expect("contravariant join should build typed IR");
    let IrStmtKind::Let { value, .. } = &evidence_ir(&ir).body[0].kind else {
        panic!("expected typed IR state binding");
    };
    assert_eq!(value.ty, expected);
}

#[test]
fn io_provider_atomic_state_rejects_invariant_source_adt_join_before_core() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\nend\n",
            "type Wrapper<T>\n  Output(T)\n  Input(fn(T) -> ())\nend\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\nend\n",
            "pub fn evidence(first: Wrapper<{id: Int}>, second: Wrapper<{label: String}>) -> () effects [concurrency]\n",
            "  let cell = state([first, second])\n",
            "  ()\nend\n",
        ),
    );
    let module = standard_io_provider_module(&source);
    let lowered = lower_checked_surface_module(&module);
    assert_eq!(
        lowered
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "type.mismatch")
            .count(),
        1,
        "{:#?}",
        lowered.diagnostics
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn io_provider_atomic_state_handoff_covers_synthetic_handler_clauses() {
    let source = SourceFile::new(
        "io_provider.veln",
        concat!(
            "mod std.io_provider\n",
            "pub type State<T>\nend\n",
            "type Consumer<T>\n  ConsumerValue(fn(T) -> ())\nend\n",
            "effect Allocate\n  value() -> State<Vec<Consumer<{id: Int, label: String}>>>\nend\n",
            "pub fn state(initial: T) -> State<T> effects [concurrency]\n",
            "  prelude_builtin::io_provider_state(initial)\nend\n",
            "handler allocating(id_only: Consumer<{id: Int}>, label_only: Consumer<{label: String}>, candidate: Consumer<{id: Int, label: String}>) for Allocate effects [concurrency]\n",
            "  value() => state([id_only, label_only, candidate])\n",
            "end\n",
        ),
    );
    let module = standard_io_provider_module(&source);
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let expected = CoreType::named(
        "State",
        vec![CoreType::vec(CoreType::named(
            "Consumer",
            vec![CoreType::Record(vec![
                ("id".to_string(), CoreType::int()),
                ("label".to_string(), CoreType::string()),
            ])],
        ))],
    );
    let core = lowered.core.expect("handler clause should build Core");
    let clause = core
        .functions
        .iter()
        .find(|function| function.name.contains("allocating") && function.return_type == expected)
        .expect("synthetic handler clause");
    let CoreStmtKind::Return { expr } = &clause.body[0].kind else {
        panic!("expected handler clause return");
    };
    assert_eq!(expr.ty, expected);
    assert!(lowered.ir.is_some());
}

fn state_initializer_work(contributions: usize, width: usize) -> StateInitializerLoweringWork {
    let records = (0..contributions)
        .map(|index| {
            let field_count = if index % 2 == 0 { width } else { width * 2 };
            let fields = (0..field_count)
                .map(|field| format!("field_{field}: {field}"))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{{fields}}}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let source = SourceFile::new(
        "io_provider.veln",
        format!(
            "mod std.io_provider\npub type State<T>\nend\npub fn state(initial: T) -> State<T> effects [concurrency]\n  prelude_builtin::io_provider_state(initial)\nend\npub fn evidence() -> () effects [concurrency]\n  let cell = state([{records}])\n  ()\nend\n"
        ),
    );
    let module = standard_io_provider_module(&source);
    reset_state_initializer_lowering_work();
    let lowered = lower_checked_surface_module(&module);
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    assert!(lowered.ir.is_some());
    take_state_initializer_lowering_work()
}

fn total_record_work(work: StateInitializerLoweringWork) -> usize {
    work.expected_record_index_entries
        + work.expected_record_lookups
        + work.retag_record_index_entries
        + work.retag_record_lookups
}

#[test]
fn io_provider_atomic_state_join_handoff_has_bounded_adjacent_size_work() {
    let smaller = state_initializer_work(16, 16);
    let larger = state_initializer_work(32, 32);
    eprintln!("state initializer lowering work: smaller={smaller:?}, larger={larger:?}");
    assert_eq!(smaller.fact_lookups, 1);
    assert_eq!(larger.fact_lookups, 1);
    assert!(larger.materialized_type_nodes <= smaller.materialized_type_nodes * 3);
    assert!(
        total_record_work(larger) <= total_record_work(smaller) * 5,
        "record work grew faster than bounded contribution/width growth: smaller={smaller:?}, larger={larger:?}"
    );
}

fn nested_state_pattern(depth: usize) -> String {
    (0..depth).fold("_".to_string(), |pattern, _| {
        format!("io_provider::State::State({pattern})")
    })
}

fn nested_pattern_module(depth: usize) -> SurfaceModule {
    let target_source = SourceFile::new(
        "io_provider.veln",
        "mod io_provider\npub type State<T>\nend\n",
    );
    let companion_source = SourceFile::new(
        "io_provider.test.veln",
        format!(
            "mod io_provider__test_companion\nuse io_provider\nfn evidence(cell: io_provider::State<Int>) -> ()\n  match cell\n    {} => ()\n    _ => ()\n  end\nend\n",
            nested_state_pattern(depth)
        ),
    );
    let target_parsed = parse(&target_source);
    let companion_parsed = parse(&companion_source);
    assert!(target_parsed.diagnostics.is_empty());
    assert!(companion_parsed.diagnostics.is_empty());
    let mut module = lower_surface_ast(&target_parsed.tree);
    let mut companion = lower_surface_ast(&companion_parsed.tree);
    for use_decl in &mut companion.uses {
        use_decl.importing_package = Some(veln_stdlib::PACKAGE_NAME.to_string());
        use_decl.package = Some(veln_stdlib::PACKAGE_NAME.to_string());
    }
    for function in &mut companion.functions {
        function.package_name = Some(veln_stdlib::PACKAGE_NAME.to_string());
    }
    module.module = companion.module;
    module.uses = companion.uses;
    module.functions.extend(companion.functions);
    module
}

fn constructor_pattern_scan_work(depth: usize) -> crate::analysis::ConstructorPatternScanWork {
    let module = nested_pattern_module(depth);
    reset_constructor_pattern_scan_work();
    let diagnostics = analyze_surface_module(&module);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == "name.unresolved"),
        "{diagnostics:#?}"
    );
    take_constructor_pattern_scan_work()
}

#[test]
fn unavailable_constructor_pattern_scan_retains_each_nested_node_once() {
    let smaller = constructor_pattern_scan_work(64);
    let larger = constructor_pattern_scan_work(128);
    eprintln!("constructor pattern scan work: smaller={smaller:?}, larger={larger:?}");
    assert_eq!(smaller.traversed, 66);
    assert_eq!(smaller.retained, 64);
    assert_eq!(larger.traversed, 130);
    assert_eq!(larger.retained, 128);
}
