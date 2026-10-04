use super::*;

fn rejected_nested_candidate_reference_collections(depth: usize) -> usize {
    let mut source = String::from(concat!(
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  let value: Int = 1\n",
        "  located("
    ));
    source.push_str(&"value(".repeat(depth));
    source.push('1');
    let column = source.lines().last().expect("call line").chars().count() + 1;
    source.push_str(&")".repeat(depth));
    source.push_str(")\nend\n");
    let snapshot = snapshot(&source);
    crate::navigation::reset_function_scope_collections();

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 6,
            column,
        },
    )
    .expect("outer signature help");
    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
    crate::navigation::function_scope_collections()
}

#[test]
fn rejected_nested_signature_candidates_do_not_collect_references() {
    assert_eq!(rejected_nested_candidate_reference_collections(100), 0);
    assert_eq!(rejected_nested_candidate_reference_collections(200), 0);
}

fn unmatched_parenthesis_work(depth: usize) -> (usize, usize, usize) {
    let mut source = String::from(concat!(
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  let value: Int = 1\n",
        "  located("
    ));
    source.push_str(&"value(".repeat(depth));
    let column = source.lines().last().expect("call line").chars().count() + 1;
    let snapshot = snapshot(&source);
    reset_signature_help_work();

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 6,
            column,
        },
    )
    .expect("outer signature help");

    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
    signature_help_work()
}

#[test]
fn signature_help_handles_deep_unmatched_parentheses_with_linear_work() {
    let smaller = unmatched_parenthesis_work(5_000);
    let larger = unmatched_parenthesis_work(10_000);

    assert!(
        larger.0 <= smaller.0 * 2,
        "signature index token visits grew too quickly: {smaller:?} -> {larger:?}"
    );
    assert_eq!(larger.1, smaller.1 * 2 - 1);
    assert_eq!(smaller.2, 1);
    assert_eq!(larger.2, 1);
}

fn incomplete_same_line_declaration_work(
    declaration_count: usize,
) -> (usize, usize, usize, std::time::Duration) {
    let mut source = String::from(concat!(
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
    ));
    for index in 0..declaration_count {
        source.push_str(&format!("fn incomplete_{index:04} "));
    }
    source.push_str("fn caller() -> SourceLocation located(");
    let column = source.lines().last().expect("call line").chars().count() + 1;
    let snapshot = snapshot(&source);
    reset_signature_help_work();
    let started = std::time::Instant::now();

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 4,
            column,
        },
    )
    .expect("selected signature help");

    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
    let work = signature_help_work();
    (
        work.0,
        signature_index_owned_text_bytes(),
        work.2,
        started.elapsed(),
    )
}

#[test]
fn incomplete_same_line_declaration_index_has_linear_work_and_retention() {
    let smaller = incomplete_same_line_declaration_work(1_000);
    let larger = incomplete_same_line_declaration_work(2_000);
    eprintln!("incomplete same-line signatures: 1000={smaller:?}, 2000={larger:?}");

    assert!(
        larger.0 <= smaller.0 * 2 + 16,
        "signature index work grew too quickly: {smaller:?} -> {larger:?}"
    );
    assert!(
        larger.1 <= smaller.1 * 2 + 128,
        "signature index retention grew too quickly: {smaller:?} -> {larger:?}"
    );
    assert_eq!(smaller.2, 1);
    assert_eq!(larger.2, 1);
}

fn undefined_callee_work(depth: usize) -> ((usize, usize, usize), usize, std::time::Duration) {
    let mut source = String::from(concat!(
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  located("
    ));
    for index in 0..depth {
        source.push_str(&format!("missing_{index}("));
    }
    let column = source.lines().last().expect("call line").chars().count() + 1;
    let snapshot = snapshot(&source);
    reset_signature_help_work();
    let started = std::time::Instant::now();

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 5,
            column,
        },
    )
    .expect("outer local signature help");

    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
    assert!(!snapshot.navigation_index_is_prepared());
    (
        signature_help_work(),
        signature_name_lookups(),
        started.elapsed(),
    )
}

#[test]
fn undefined_callees_use_a_parse_free_linear_name_index() {
    let smaller = undefined_callee_work(5_000);
    let larger = undefined_callee_work(10_000);
    eprintln!(
        "undefined callee signature help: 5000={:?}, 10000={:?}",
        smaller.2, larger.2
    );

    assert!(
        larger.0.0 <= smaller.0.0 * 2,
        "signature index token visits grew too quickly: {smaller:?} -> {larger:?}"
    );
    assert_eq!(larger.0.1, smaller.0.1 * 2 - 1);
    assert_eq!(smaller.0.2, 1);
    assert_eq!(larger.0.2, 1);
    assert_eq!(smaller.1, 5_000);
    assert_eq!(larger.1, 10_000);
}

fn inaccessible_callee_navigation_lookups(depth: usize) -> (usize, usize) {
    let mut hidden = String::from("mod hidden\n");
    for index in 0..depth {
        hidden.push_str(&format!(
            "fn hidden_{index}(value: Int) -> Int\n  value\nend\n"
        ));
    }
    let mut main = String::from(concat!(
        "mod app\n",
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  located("
    ));
    for index in 0..depth {
        main.push_str(&format!("hidden_{index}("));
    }
    let column = main.lines().last().expect("call line").chars().count() + 1;
    let snapshot = EffectiveProjectSnapshot::new(vec![
        SourceFile::new("main.veln", main),
        SourceFile::new("hidden.veln", hidden),
    ]);
    reset_signature_help_work();

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 6,
            column,
        },
    )
    .expect("outer local signature help");

    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
    (signature_name_lookups(), signature_navigation_lookups())
}

#[test]
fn inaccessible_global_signature_candidates_have_bounded_navigation_work() {
    let smaller = inaccessible_callee_navigation_lookups(80);
    let larger = inaccessible_callee_navigation_lookups(160);

    assert_eq!(smaller.0, 80);
    assert_eq!(larger.0, 160);
    assert_eq!(smaller.1, MAX_SIGNATURE_NAVIGATION_LOOKUPS);
    assert_eq!(larger.1, MAX_SIGNATURE_NAVIGATION_LOOKUPS);
}

fn alias_target_lookups(alias_count: usize) -> usize {
    let mut source = String::from(
        "pub fn located(message: String) -> SourceLocation callsite\n  callsite\nend\n",
    );
    source.push_str("pub fn alias_0 = located\n");
    for index in 1..alias_count {
        source.push_str(&format!("pub fn alias_{index} = alias_{}\n", index - 1));
    }
    source.push_str(&format!(
        "fn caller() -> SourceLocation\n  alias_{}(\"hello\")\nend\n",
        alias_count - 1
    ));
    let snapshot = snapshot(&source);
    crate::navigation::reset_function_alias_target_lookups();
    function_signature_definition_at(
        &snapshot,
        &SourcePosition {
            source: SourcePath::new("main.veln"),
            line: alias_count + 5,
            column: 3,
        },
    )
    .expect("resolved alias target");
    crate::navigation::function_alias_target_lookups()
}

#[test]
fn function_alias_resolution_uses_one_index_lookup_per_hop() {
    assert_eq!(alias_target_lookups(128), 128);
    assert_eq!(alias_target_lookups(256), 256);
}

fn qualified_alias_sources(alias_count: usize, filler_count: usize) -> (Vec<SourceFile>, usize) {
    let mut sources = vec![SourceFile::new(
        "target.veln",
        concat!(
            "pub fn located(message: String) -> SourceLocation callsite\n",
            "  callsite\n",
            "end\n",
        ),
    )];
    for index in 0..alias_count {
        let target_module = if index == 0 {
            "target".to_string()
        } else {
            format!("alias_{}", index - 1)
        };
        let target_name = if index == 0 {
            "located".to_string()
        } else {
            format!("alias_{}", index - 1)
        };
        sources.push(SourceFile::new(
            format!("alias_{index}.veln"),
            format!("use {target_module}\npub fn alias_{index} = {target_module}::{target_name}\n"),
        ));
    }
    for index in 0..filler_count {
        sources.push(SourceFile::new(
            format!("filler_{index}.veln"),
            format!("pub fn filler_{index}() -> Int\n  {index}\nend\n"),
        ));
    }
    let final_alias = format!("alias_{}", alias_count - 1);
    let main = format!(
        "use {final_alias}\nfn caller() -> SourceLocation\n  {final_alias}::{final_alias}(\"hello\")\nend\n"
    );
    let column = main.lines().nth(2).expect("call line").chars().count();
    sources.push(SourceFile::new("main.veln", main));
    (sources, column)
}

fn qualified_alias_chain_work(alias_count: usize, filler_count: usize) -> (usize, usize) {
    let (sources, column) = qualified_alias_sources(alias_count, filler_count);
    let snapshot = EffectiveProjectSnapshot::new(sources);
    crate::navigation::reset_function_alias_target_lookups();
    crate::navigation::reset_function_alias_declaring_file_lookups();

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 3,
            column,
        },
    )
    .expect("qualified alias signature help");

    assert_eq!(
        help.label,
        "fn located(message: String) -> SourceLocation callsite"
    );
    (
        crate::navigation::function_alias_target_lookups(),
        crate::navigation::function_alias_declaring_file_lookups(),
    )
}

#[test]
fn qualified_alias_resolution_indexes_each_declaring_file() {
    assert_eq!(qualified_alias_chain_work(64, 64), (64, 64));
    assert_eq!(qualified_alias_chain_work(128, 64), (128, 128));
    assert_eq!(qualified_alias_chain_work(64, 128), (64, 64));
    assert_eq!(qualified_alias_chain_work(128, 128), (128, 128));
}
