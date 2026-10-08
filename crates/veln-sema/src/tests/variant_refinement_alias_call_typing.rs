use super::*;
use crate::types::TypeEnvironment;

#[test]
fn alias_qualified_refinements_share_target_identity_and_preserve_annotations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = First\n",
            "type Box<A>\n",
            "  Empty\n",
            "  Boxed(A)\n",
            "end\n",
            "pub type StateBox = Box\n",
            "pub type OuterStateBox = StateBox\n",
            "fn direct(value: State::Ready) -> First::Ready\n",
            "  value\n",
            "end\n",
            "fn alias_singleton(value: First::Ready) -> State::Ready\n",
            "  value\n",
            "end\n",
            "fn alias_union(value: Second::Closed | Second::Ready | Second::Closed) -> First::Ready | State::Closed\n",
            "  value\n",
            "end\n",
            "fn generic(value: OuterStateBox<Int>::Boxed) -> Box<Int>::Boxed\n",
            "  value\n",
            "end\n",
            "fn alias_constructor()\n",
            "  First::Ready\n",
            "end\n",
            "fn alias_and_target(value: First::Ready | State::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn target_and_alias(value: State::Ready | First::Closed) -> ()\n",
            "  ()\n",
            "end\n",
            "fn alias_transition(value: First::Ready) -> First::Ready\n",
            "  value\n",
            "end\n",
            "fn function_value_identity() -> ()\n",
            "  let transition: fn(State::Ready) -> State::Ready = alias_transition\n",
            "  transition(Ready)\n",
            "  ()\n",
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
        environment.function("direct").unwrap().return_type.render(),
        "First::Ready"
    );
    let alias_singleton = &environment.function("alias_singleton").unwrap().params[0];
    let target_singleton = &environment.function("direct").unwrap().params[0];
    assert!(crate::type_relations::is_assignable(
        alias_singleton,
        target_singleton
    ));
    assert!(crate::type_relations::is_assignable(
        target_singleton,
        alias_singleton
    ));
    assert_eq!(
        environment.function("alias_union").unwrap().params[0].render(),
        "Second::Ready | Second::Closed"
    );
    assert_eq!(
        environment
            .function("alias_union")
            .unwrap()
            .return_type
            .render(),
        "First::Ready | First::Closed"
    );
    assert_eq!(
        environment.function("generic").unwrap().params[0].render(),
        "OuterStateBox<Int>::Boxed"
    );
    assert_eq!(
        environment
            .function("alias_constructor")
            .unwrap()
            .return_type
            .render(),
        "State::Ready"
    );
    assert_eq!(
        environment.function("alias_and_target").unwrap().params[0].render(),
        "First::Ready | First::Closed"
    );
    assert_eq!(
        environment.function("target_and_alias").unwrap().params[0].render(),
        "First::Ready | First::Closed"
    );
}

#[test]
fn alias_refinement_joins_and_mismatches_keep_independent_presentation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type State\n",
            "  Ready\n",
            "  Closed\n",
            "  Failed\n",
            "end\n",
            "pub type First = State\n",
            "pub type Second = State\n",
            "type Envelope<A>\n",
            "  Left(A)\n",
            "  Right(A)\n",
            "end\n",
            "fn same_alias(flag: Bool)\n",
            "  let ready: First::Ready = Ready\n",
            "  let closed: First::Closed = Closed\n",
            "  if flag\n",
            "    ready\n",
            "  else\n",
            "    closed\n",
            "  end\n",
            "end\n",
            "fn conflicting_aliases(flag: Bool)\n",
            "  let ready: First::Ready = Ready\n",
            "  let closed: Second::Closed = Closed\n",
            "  if flag\n",
            "    ready\n",
            "  else\n",
            "    closed\n",
            "  end\n",
            "end\n",
            "fn inferred(flag: Bool)\n",
            "  if flag\n",
            "    Ready\n",
            "  else\n",
            "    Closed\n",
            "  end\n",
            "end\n",
            "fn generic_argument_union(value: Envelope<First::Ready>::Left | Envelope<Second::Ready>::Right) -> ()\n",
            "  ()\n",
            "end\n",
            "fn nested_conflicting_aliases(flag: Bool)\n",
            "  let left: Envelope<First::Ready>::Left = Left(Ready)\n",
            "  let other: Envelope<Second::Ready>::Left = Left(Ready)\n",
            "  if flag\n",
            "    left\n",
            "  else\n",
            "    other\n",
            "  end\n",
            "end\n",
            "fn needs_first(value: First::Ready) -> ()\n",
            "  ()\n",
            "end\n",
            "fn mismatch() -> ()\n",
            "  let actual: Second::Closed = Closed\n",
            "  needs_first(actual)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);
    let diagnostics = analyze_surface_module(&module);
    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(
        diagnostics[0].message,
        "value of type `Second::Closed` is not assignable to variant type `First::Ready`"
    );
    let environment = TypeEnvironment::from_module(&module);
    assert_eq!(
        environment
            .function("same_alias")
            .unwrap()
            .return_type
            .render(),
        "First::Ready | First::Closed"
    );
    assert_eq!(
        environment
            .function("conflicting_aliases")
            .unwrap()
            .return_type
            .render(),
        "State::Ready | State::Closed"
    );
    assert_eq!(
        environment
            .function("inferred")
            .unwrap()
            .return_type
            .render(),
        "State::Ready | State::Closed"
    );
    assert_eq!(
        environment
            .function("generic_argument_union")
            .unwrap()
            .params[0]
            .render(),
        "Envelope<State::Ready>"
    );
    assert_eq!(
        environment
            .function("nested_conflicting_aliases")
            .unwrap()
            .return_type
            .render(),
        "Envelope<State::Ready>::Left"
    );
}
