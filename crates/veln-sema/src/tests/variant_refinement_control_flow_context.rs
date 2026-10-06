use super::*;
use crate::types::TypeEnvironment;

fn diagnostics_for(source: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", source);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

#[test]
fn control_flow_join_does_not_supply_sibling_constructor_context() {
    let diagnostics = diagnostics_for(concat!(
        "type Left\n",
        "  First\n",
        "  Same\n",
        "end\n",
        "type Right\n",
        "  Same\n",
        "end\n",
        "type Maybe<A>\n",
        "  Just(A)\n",
        "  Missing\n",
        "end\n",
        "fn main(flag: Bool) -> ()\n",
        "  let ambiguous_if = if flag\n",
        "    Left::First\n",
        "  else\n",
        "    Same\n",
        "  end\n",
        "  let ambiguous_match = match flag\n",
        "    true => Left::First\n",
        "    false => Same\n",
        "  end\n",
        "  let generic_if = if flag\n",
        "    Maybe::Just(1)\n",
        "  else\n",
        "    Missing\n",
        "  end\n",
        "  let generic_match = match flag\n",
        "    true => Maybe::Just(1)\n",
        "    false => Missing\n",
        "  end\n",
        "  let contextual_if: Left = if flag\n",
        "    First\n",
        "  else\n",
        "    Same\n",
        "  end\n",
        "  let contextual_match: Maybe<Int> = match flag\n",
        "    true => Just(1)\n",
        "    false => Missing\n",
        "  end\n",
        "end\n",
    ));

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.ambiguous")
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "type.inference_ambiguous"
                    && diagnostic.message == "constructor `Missing` needs type context"
            })
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
}

#[test]
fn private_control_flow_join_does_not_infer_sibling_constructor_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Left\n",
            "  First\n",
            "  Same\n",
            "end\n",
            "type Right\n",
            "  Same\n",
            "end\n",
            "type Maybe<A>\n",
            "  Just(A)\n",
            "  Missing\n",
            "  Deferred(A)\n",
            "end\n",
            "fn ambiguous_if(flag: Bool)\n",
            "  if flag\n",
            "    Left::First\n",
            "  else\n",
            "    Same\n",
            "  end\n",
            "end\n",
            "fn ambiguous_match(flag: Bool)\n",
            "  match flag\n",
            "    true => Left::First\n",
            "    false => Same\n",
            "  end\n",
            "end\n",
            "fn generic_if(flag: Bool)\n",
            "  if flag\n",
            "    Maybe::Just(1)\n",
            "  else\n",
            "    Missing\n",
            "  end\n",
            "end\n",
            "fn generic_match(flag: Bool)\n",
            "  match flag\n",
            "    true => Maybe::Just(1)\n",
            "    false => Missing\n",
            "  end\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let environment = TypeEnvironment::from_module(&module);

    for function in ["ambiguous_if", "ambiguous_match"] {
        assert_eq!(
            environment
                .function(function)
                .expect("private function")
                .return_type
                .render(),
            "Left::First",
            "{function}"
        );
    }
    for function in ["generic_if", "generic_match"] {
        assert_eq!(
            environment
                .function(function)
                .expect("private function")
                .return_type
                .render(),
            "Maybe<Int>",
            "{function}"
        );
    }
}

#[test]
fn base_first_control_flow_results_retain_generic_constructor_context() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn private_if(flag: Bool, base: Result<(), String>)\n",
            "  if flag\n",
            "    base\n",
            "  else\n",
            "    Ok(())\n",
            "  end\n",
            "end\n",
            "fn private_match(flag: Bool, base: Result<(), String>)\n",
            "  match flag\n",
            "    true => base\n",
            "    false => Ok(())\n",
            "  end\n",
            "end\n",
            "fn ordinary(flag: Bool, base: Result<(), String>) -> ()\n",
            "  let from_if = if flag\n",
            "    base\n",
            "  else\n",
            "    Ok(())\n",
            "  end\n",
            "  let from_match = match flag\n",
            "    true => base\n",
            "    false => Ok(())\n",
            "  end\n",
            "  let exact_if: Result<(), String> = from_if\n",
            "  let exact_match: Result<(), String> = from_match\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let environment = TypeEnvironment::from_module(&module);

    for function in ["private_if", "private_match"] {
        assert_eq!(
            environment
                .function(function)
                .expect("private function")
                .return_type
                .render(),
            "Result<(), String>",
            "{function}"
        );
    }
}

#[test]
fn ordinary_nested_control_flow_joins_are_symmetric_and_exact() {
    let diagnostics = diagnostics_for(concat!(
        "type State\n",
        "  Ready\n",
        "  Closed\n",
        "  Failed\n",
        "  Pending\n",
        "end\n",
        "fn main(first: Bool, second: Bool) -> ()\n",
        "  let if_forward = if first\n",
        "    if second\n",
        "      Closed\n",
        "    else\n",
        "      Ready\n",
        "    end\n",
        "  else\n",
        "    Failed\n",
        "  end\n",
        "  let if_reverse = if first\n",
        "    Failed\n",
        "  else\n",
        "    if second\n",
        "      Ready\n",
        "    else\n",
        "      Closed\n",
        "    end\n",
        "  end\n",
        "  let match_forward = match first\n",
        "    true => match second\n",
        "      true => Closed\n",
        "      false => Ready\n",
        "    end\n",
        "    false => Failed\n",
        "  end\n",
        "  let match_reverse = match first\n",
        "    true => Failed\n",
        "    false => match second\n",
        "      true => Ready\n",
        "      false => Closed\n",
        "    end\n",
        "  end\n",
        "  let narrow_if_forward: State::Ready = if_forward\n",
        "  let narrow_if_reverse: State::Ready = if_reverse\n",
        "  let narrow_match_forward: State::Ready = match_forward\n",
        "  let narrow_match_reverse: State::Ready = match_reverse\n",
        "end\n",
    ));

    assert_eq!(diagnostics.len(), 4, "{diagnostics:#?}");
    assert!(diagnostics.iter().all(|diagnostic| {
        diagnostic.id == "type.variant_mismatch"
            && diagnostic
                .details
                .to_json()
                .contains("\"actual_type\":\"State::Ready | State::Closed | State::Failed\"")
    }));
}

#[test]
fn failed_control_flow_joins_recover_to_the_base_and_do_not_resume() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "fn private_if(first: Bool, second: Bool, text: String)\n",
            "  if first\n",
            "    Ready\n",
            "  else if second\n",
            "    text\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn private_match(selector: Int, text: String)\n",
            "  match selector\n",
            "    0 => Ready\n",
            "    1 => text\n",
            "    _ => Closed\n",
            "  end\n",
            "end\n",
            "fn ordinary(first: Bool, second: Bool, text: String) -> ()\n",
            "  let failed_if = if first\n",
            "    Ready\n",
            "  else if second\n",
            "    text\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "  let failed_match = match 0\n",
            "    0 => Ready\n",
            "    1 => text\n",
            "    _ => Closed\n",
            "  end\n",
            "  let narrow_if: State::Ready | State::Closed = failed_if\n",
            "  let narrow_match: State::Ready | State::Closed = failed_match\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    let environment = TypeEnvironment::from_module(&module);

    for function in ["private_if", "private_match"] {
        assert_eq!(
            environment
                .function(function)
                .expect("private function")
                .return_type
                .render(),
            "State",
            "{function}"
        );
    }
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "type.mismatch"
                    && diagnostic.message == "expected `State`, but found `String`"
            })
            .count(),
        4,
        "{diagnostics:#?}"
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| {
                diagnostic.id == "type.variant_mismatch"
                    && diagnostic.message.contains("value of type `State`")
            })
            .count(),
        2,
        "{diagnostics:#?}"
    );
    assert_eq!(diagnostics.len(), 6, "{diagnostics:#?}");
}
