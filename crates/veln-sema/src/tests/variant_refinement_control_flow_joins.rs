use super::*;
use crate::types::TypeEnvironment;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn private_control_flow_results_keep_only_a_common_singleton() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "end\n",
            "fn same(value: Bool)\n",
            "  if value\n",
            "    Ready\n",
            "  else\n",
            "    Ready\n",
            "  end\n",
            "end\n",
            "fn mixed_if(value: Bool)\n",
            "  if value\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn mixed_match(value: Bool)\n",
            "  match value\n",
            "    true => Ready\n",
            "    false => Closed\n",
            "  end\n",
            "end\n",
            "fn accept(value: State) -> ()\n",
            "  ()\n",
            "end\n",
            "fn main() -> ()\n",
            "  accept(mixed_if(true))\n",
            "  accept(mixed_match(true))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    assert_eq!(
        environment.function("same").unwrap().return_type.render(),
        "State::Ready"
    );
    assert_eq!(
        environment
            .function("mixed_if")
            .unwrap()
            .return_type
            .render(),
        "State"
    );
    assert_eq!(
        environment
            .function("mixed_match")
            .unwrap()
            .return_type
            .render(),
        "State"
    );
}

#[test]
fn private_control_flow_result_joins_are_symmetric_and_canonical() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "  Pending\n",
            "end\n",
            "type Box<A>\n",
            "  Empty\n",
            "  Filled(A)\n",
            "  Deferred(A)\n",
            "end\n",
            "fn same_if(flag: Bool)\n",
            "  if flag\n",
            "    Ready\n",
            "  else\n",
            "    Ready\n",
            "  end\n",
            "end\n",
            "fn same_match(flag: Bool)\n",
            "  match flag\n",
            "    true => Ready\n",
            "    false => Ready\n",
            "  end\n",
            "end\n",
            "fn if_forward(flag: Bool)\n",
            "  if flag\n",
            "    Closed\n",
            "  else\n",
            "    Ready\n",
            "  end\n",
            "end\n",
            "fn if_reverse(flag: Bool)\n",
            "  if flag\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn match_forward(flag: Bool)\n",
            "  match flag\n",
            "    true => Closed\n",
            "    false => Ready\n",
            "  end\n",
            "end\n",
            "fn match_reverse(flag: Bool)\n",
            "  match flag\n",
            "    true => Ready\n",
            "    false => Closed\n",
            "  end\n",
            "end\n",
            "fn generic_if_forward(flag: Bool, first: Box<Int>::Filled, second: Box<Int>::Deferred)\n",
            "  if flag\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end\n",
            "end\n",
            "fn generic_if_reverse(flag: Bool, first: Box<Int>::Filled, second: Box<Int>::Deferred)\n",
            "  if flag\n",
            "    second\n",
            "  else\n",
            "    first\n",
            "  end\n",
            "end\n",
            "fn generic_match_forward(flag: Bool, first: Box<Int>::Filled, second: Box<Int>::Deferred)\n",
            "  match flag\n",
            "    true => first\n",
            "    false => second\n",
            "  end\n",
            "end\n",
            "fn generic_match_reverse(flag: Bool, first: Box<Int>::Filled, second: Box<Int>::Deferred)\n",
            "  match flag\n",
            "    true => second\n",
            "    false => first\n",
            "  end\n",
            "end\n",
            "fn if_finite_forward(flag: Bool, first: State::Failed | State::Ready, second: State::Closed)\n",
            "  if flag\n",
            "    first\n",
            "  else\n",
            "    second\n",
            "  end\n",
            "end\n",
            "fn if_finite_reverse(flag: Bool, first: State::Failed | State::Ready, second: State::Closed)\n",
            "  if flag\n",
            "    second\n",
            "  else\n",
            "    first\n",
            "  end\n",
            "end\n",
            "fn match_finite_forward(flag: Bool, first: State::Failed | State::Ready, second: State::Closed)\n",
            "  match flag\n",
            "    true => first\n",
            "    false => second\n",
            "  end\n",
            "end\n",
            "fn match_finite_reverse(flag: Bool, first: State::Failed | State::Ready, second: State::Closed)\n",
            "  match flag\n",
            "    true => second\n",
            "    false => first\n",
            "  end\n",
            "end\n",
            "fn if_base(flag: Bool, refined: State::Ready, base: State)\n",
            "  if flag\n",
            "    refined\n",
            "  else\n",
            "    base\n",
            "  end\n",
            "end\n",
            "fn if_base_reverse(flag: Bool, refined: State::Ready, base: State)\n",
            "  if flag\n",
            "    base\n",
            "  else\n",
            "    refined\n",
            "  end\n",
            "end\n",
            "fn match_base(flag: Bool, refined: State::Ready, base: State)\n",
            "  match flag\n",
            "    true => base\n",
            "    false => refined\n",
            "  end\n",
            "end\n",
            "fn match_base_reverse(flag: Bool, refined: State::Ready, base: State)\n",
            "  match flag\n",
            "    true => refined\n",
            "    false => base\n",
            "  end\n",
            "end\n",
            "fn if_full(first: Bool, second: Bool, third: Bool)\n",
            "  if first\n",
            "    Failed\n",
            "  else if second\n",
            "    Ready\n",
            "  else if third\n",
            "    Pending\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn if_full_reverse(first: Bool, second: Bool, third: Bool)\n",
            "  if first\n",
            "    Closed\n",
            "  else if second\n",
            "    Pending\n",
            "  else if third\n",
            "    Ready\n",
            "  else\n",
            "    Failed\n",
            "  end\n",
            "end\n",
            "fn match_full(state: State)\n",
            "  match state\n",
            "    Ready => Pending\n",
            "    Closed => Ready\n",
            "    Failed => Closed\n",
            "    Pending => Failed\n",
            "  end\n",
            "end\n",
            "fn match_full_reverse(state: State)\n",
            "  match state\n",
            "    Pending => Closed\n",
            "    Failed => Pending\n",
            "    Closed => Failed\n",
            "    Ready => Ready\n",
            "  end\n",
            "end\n",
            "fn nested_if_forward(first: Bool, second: Bool)\n",
            "  if first\n",
            "    if second\n",
            "      Closed\n",
            "    else\n",
            "      Ready\n",
            "    end\n",
            "  else\n",
            "    Failed\n",
            "  end\n",
            "end\n",
            "fn nested_if_reverse(first: Bool, second: Bool)\n",
            "  if first\n",
            "    Failed\n",
            "  else\n",
            "    if second\n",
            "      Ready\n",
            "    else\n",
            "      Closed\n",
            "    end\n",
            "  end\n",
            "end\n",
            "fn nested_match_forward(first: Bool, second: Bool)\n",
            "  match first\n",
            "    true => match second\n",
            "      true => Closed\n",
            "      false => Ready\n",
            "    end\n",
            "    false => Failed\n",
            "  end\n",
            "end\n",
            "fn nested_match_reverse(first: Bool, second: Bool)\n",
            "  match first\n",
            "    true => Failed\n",
            "    false => match second\n",
            "      true => Ready\n",
            "      false => Closed\n",
            "    end\n",
            "  end\n",
            "end\n",
            "fn body_joins(flag: Bool) -> ()\n",
            "  let joined_if = if flag\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "  let joined_match = match flag\n",
            "    true => Closed\n",
            "    false => Ready\n",
            "  end\n",
            "  let exact_if: State::Ready | State::Closed = joined_if\n",
            "  let exact_match: State::Ready | State::Closed = joined_match\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    for (function, expected) in [
        ("same_if", "State::Ready"),
        ("same_match", "State::Ready"),
        ("if_forward", "State::Ready | State::Closed"),
        ("if_reverse", "State::Ready | State::Closed"),
        ("match_forward", "State::Ready | State::Closed"),
        ("match_reverse", "State::Ready | State::Closed"),
        (
            "generic_if_forward",
            "Box<Int>::Filled | Box<Int>::Deferred",
        ),
        (
            "generic_if_reverse",
            "Box<Int>::Filled | Box<Int>::Deferred",
        ),
        (
            "generic_match_forward",
            "Box<Int>::Filled | Box<Int>::Deferred",
        ),
        (
            "generic_match_reverse",
            "Box<Int>::Filled | Box<Int>::Deferred",
        ),
        (
            "if_finite_forward",
            "State::Ready | State::Closed | State::Failed",
        ),
        (
            "if_finite_reverse",
            "State::Ready | State::Closed | State::Failed",
        ),
        (
            "match_finite_forward",
            "State::Ready | State::Closed | State::Failed",
        ),
        (
            "match_finite_reverse",
            "State::Ready | State::Closed | State::Failed",
        ),
        ("if_base", "State"),
        ("if_base_reverse", "State"),
        ("match_base", "State"),
        ("match_base_reverse", "State"),
        ("if_full", "State"),
        ("if_full_reverse", "State"),
        ("match_full", "State"),
        ("match_full_reverse", "State"),
        (
            "nested_if_forward",
            "State::Ready | State::Closed | State::Failed",
        ),
        (
            "nested_if_reverse",
            "State::Ready | State::Closed | State::Failed",
        ),
        (
            "nested_match_forward",
            "State::Ready | State::Closed | State::Failed",
        ),
        (
            "nested_match_reverse",
            "State::Ready | State::Closed | State::Failed",
        ),
    ] {
        assert_eq!(
            environment
                .function(function)
                .unwrap_or_else(|| panic!("{function} should be present"))
                .return_type
                .render(),
            expected,
            "unexpected inferred result for {function}",
        );
    }
}

#[test]
fn private_control_flow_result_joins_preserve_incompatible_mismatches() {
    for (name, expression, expected, actual) in [
        (
            "if refinement first",
            "if flag\n    ready\n  else\n    text\n  end",
            "State",
            "String",
        ),
        (
            "if refinement second",
            "if flag\n    text\n  else\n    ready\n  end",
            "String",
            "State::Ready",
        ),
        (
            "match refinement first",
            "match flag\n    true => ready\n    false => text\n  end",
            "State",
            "String",
        ),
        (
            "match refinement second",
            "match flag\n    true => text\n    false => ready\n  end",
            "String",
            "State::Ready",
        ),
        (
            "if different generic arguments",
            "if flag\n    int_box\n  else\n    string_box\n  end",
            "Box<Int>",
            "Box<String>::Boxed",
        ),
        (
            "match different generic arguments",
            "match flag\n    true => int_box\n    false => string_box\n  end",
            "Box<Int>",
            "Box<String>::Boxed",
        ),
    ] {
        let diagnostics = diagnostics_for(&format!(
            "type State\n  Ready\n  Closed\nend\n\
             type Box<A>\n  Boxed(A)\n  Empty\nend\n\
             fn selected(flag: Bool, ready: State::Ready, text: String, int_box: Box<Int>::Boxed, string_box: Box<String>::Boxed) -> ()\n  let value = {expression}\nend\n"
        ));

        assert_eq!(diagnostics.len(), 1, "{name}: {diagnostics:#?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.id, "type.mismatch", "{name}: {diagnostics:#?}");
        assert_eq!(
            diagnostic.message,
            format!("expected `{expected}`, but found `{actual}`"),
            "{name}: {diagnostics:#?}",
        );
    }
}
