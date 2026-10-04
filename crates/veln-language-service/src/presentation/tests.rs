use super::*;
use veln_source::SourcePath;

fn snapshot(text: &str) -> EffectiveProjectSnapshot {
    EffectiveProjectSnapshot::new(vec![SourceFile::new("main.veln", text)])
}

#[test]
fn completion_distinguishes_modifier_builtin_and_ordinary_contexts() {
    let snapshot = snapshot(concat!(
        "fn located() -> SourceLocation callsite\n",
        "require callsite.start_line > 0\n",
        "  callsite\n",
        "end\n",
        "fn ordinary() -> Int\n",
        "  1\n",
        "end\n",
    ));
    let modifier = completion_at(
        &snapshot,
        &SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 5,
            column: 21,
        },
    );
    assert_eq!(
        modifier[0].kind,
        CompletionCandidateKind::DeclarationModifier
    );
    let builtin = completion_at(
        &snapshot,
        &SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 3,
            column: 3,
        },
    );
    assert_eq!(builtin[0].kind, CompletionCandidateKind::BuiltinLocal);
    assert!(
        completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 9
            }
        )
        .is_empty()
    );
    assert!(
        completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 6,
                column: 3
            }
        )
        .is_empty()
    );
}

#[test]
fn completion_bounds_unfinished_call_nesting_before_parse() {
    let mut source = String::from("fn located() -> SourceLocation callsite\n  ");
    source.push_str(&"callsite(".repeat(10_000));
    let snapshot = snapshot(&source);

    assert!(
        completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 2,
                column: 3,
            }
        )
        .is_empty()
    );
}

#[test]
fn completion_covers_empty_bodies_and_only_the_terminal_modifier_slot() {
    let snapshot = snapshot(concat!(
        "fn empty() -> SourceLocation callsite\n",
        "\n",
        "end\n",
        "fn higher(callback: fn() -> Int) -> Int\n",
        "  1\n",
        "end\n",
        "test example() -> Int\n",
        "  1\n",
        "end\n",
    ));
    let empty_body = completion_at(
        &snapshot,
        &SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 2,
            column: 1,
        },
    );
    assert_eq!(empty_body[0].kind, CompletionCandidateKind::BuiltinLocal);
    assert!(
        completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 25,
            }
        )
        .is_empty()
    );
    let terminal = completion_at(
        &snapshot,
        &SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 4,
            column: 40,
        },
    );
    assert_eq!(
        terminal[0].kind,
        CompletionCandidateKind::DeclarationModifier
    );
    assert!(
        completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 8,
                column: 3,
            }
        )
        .is_empty()
    );
}

#[test]
fn modifier_completion_stops_at_a_trailing_comment() {
    let snapshot = snapshot(concat!(
        "fn noted() -> Int # keep this note\n",
        "  1\n",
        "end\n",
    ));

    for column in [18, 19] {
        let candidates = completion_at(
            &snapshot,
            &SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column,
            },
        );
        assert_eq!(
            candidates.first().map(|candidate| candidate.kind),
            Some(CompletionCandidateKind::DeclarationModifier),
            "column {column}"
        );
    }
    for column in [20, 35] {
        assert!(
            completion_at(
                &snapshot,
                &SourcePosition {
                    source: SourcePath::new("main.veln"),
                    line: 1,
                    column,
                },
            )
            .is_empty(),
            "column {column}"
        );
    }
}

#[test]
fn signature_help_keeps_callsite_outside_the_parameter_list() {
    let snapshot = snapshot(concat!(
        "fn located(message: String) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  located(\"hello\")\n",
        "end\n",
    ));
    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 5,
            column: 18,
        },
    )
    .expect("signature help");
    assert_eq!(
        help.label,
        "fn located(message: String) -> SourceLocation callsite"
    );
    assert_eq!(help.parameters, ["message: String"]);
    assert_eq!(help.active_parameter, 0);
}

#[test]
fn signature_help_resolves_contextual_function_names_across_sources() {
    let declarations = ["callsite", "handle", "handler", "handles"]
        .into_iter()
        .map(|name| format!("pub fn {name}(value: Int) -> Int callsite\n  value\nend\n"))
        .collect::<String>();
    let caller = concat!(
        "use declarations\n",
        "fn caller() -> Int\n",
        "  declarations::callsite(1)\n",
        "  declarations::handle(1)\n",
        "  declarations::handler(1)\n",
        "  declarations::handles(1)\n",
        "end\n",
    );
    let snapshot = EffectiveProjectSnapshot::new(vec![
        SourceFile::new("declarations.veln", declarations),
        SourceFile::new("main.veln", caller),
    ]);

    for (line, name) in [
        (3, "callsite"),
        (4, "handle"),
        (5, "handler"),
        (6, "handles"),
    ] {
        let source_line = caller.lines().nth(line - 1).expect("call line");
        let column = source_line.find(')').expect("closing parenthesis") + 1;
        let help = signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line,
                column,
            },
        )
        .expect("contextual-name signature help");
        assert_eq!(help.label, format!("fn {name}(value: Int) -> Int callsite"));
    }
}

#[test]
fn signature_help_does_not_treat_bare_handle_as_a_call_candidate() {
    let snapshot = snapshot(concat!(
        "effect Read\n",
        "  read() -> Int\n",
        "end\n",
        "handler answer() handles Read\n",
        "  read() => 1\n",
        "end\n",
        "fn caller() -> Int\n",
        "  handle (1) with answer()\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 8,
                column: 10,
            },
        )
        .is_none()
    );
}

#[test]
fn signature_help_is_absent_in_a_function_declaration_header() {
    let snapshot = snapshot(concat!(
        "fn located(message: String) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column: 20,
            },
        )
        .is_none()
    );
}

#[test]
fn signature_help_is_absent_in_a_test_declaration_header() {
    let snapshot = snapshot(concat!(
        "test example(value: Int) -> Int\n",
        "  value\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 1,
                column: 22,
            },
        )
        .is_none()
    );
}

#[test]
fn signature_help_drops_an_unclosed_call_before_a_later_declaration_header() {
    let snapshot = snapshot(concat!(
        "fn located(message: String) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn unfinished() -> SourceLocation\n",
        "  located(\n",
        "end\n",
        "fn recovered(value: Int) -> Int\n",
        "  value\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 7,
                column: 20,
            },
        )
        .is_none()
    );
}

#[test]
fn signature_help_skips_grouping_inside_a_call_argument() {
    let snapshot = snapshot(concat!(
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  located((1 + 2))\n",
        "end\n",
    ));

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 5,
            column: 15,
        },
    )
    .expect("outer call signature help");
    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
}

#[test]
fn signature_help_prefers_the_innermost_nested_call() {
    let snapshot = snapshot(concat!(
        "fn outer(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn inner(message: String) -> Int callsite\n",
        "  1\n",
        "end\n",
        "fn caller() -> Int\n",
        "  outer(inner(\"hello\"))\n",
        "end\n",
    ));

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 8,
            column: 21,
        },
    )
    .expect("inner call signature help");
    assert_eq!(help.label, "fn inner(message: String) -> Int callsite");
    assert_eq!(help.parameters, ["message: String"]);
}

#[test]
fn signature_help_skips_a_non_function_identifier_before_grouping() {
    let snapshot = snapshot(concat!(
        "fn located(value: Int) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn caller() -> SourceLocation\n",
        "  let value: Int = 1\n",
        "  located(value(1))\n",
        "end\n",
    ));

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 6,
            column: 18,
        },
    )
    .expect("outer call signature help");
    assert_eq!(
        help.label,
        "fn located(value: Int) -> SourceLocation callsite"
    );
}

#[test]
fn signature_help_does_not_bypass_parameter_shadowing() {
    let snapshot = snapshot(concat!(
        "fn located(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn caller(located: fn(Int) -> Int) -> Int\n",
        "  located(1)\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 12,
            },
        )
        .is_none()
    );
}

#[test]
fn signature_help_does_not_bypass_the_callsite_builtin() {
    let snapshot = snapshot(concat!(
        "fn callsite(value: Int) -> Int\n",
        "  value\n",
        "end\n",
        "fn caller() -> Int callsite\n",
        "  callsite(1)\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 5,
                column: 14,
            },
        )
        .is_none()
    );
}

#[test]
fn signature_help_resolves_workspace_function_aliases() {
    let snapshot = snapshot(concat!(
        "pub fn located(message: String) -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "pub fn renamed = located\n",
        "fn caller() -> SourceLocation\n",
        "  renamed(\"hello\")\n",
        "end\n",
    ));
    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 6,
            column: 18,
        },
    )
    .expect("alias signature help");
    assert_eq!(
        help.label,
        "fn located(message: String) -> SourceLocation callsite"
    );
    assert_eq!(help.parameters, ["message: String"]);
}

#[test]
fn signature_help_resolves_alias_chains_beyond_the_former_depth_limit() {
    let mut source = String::from(
        "pub fn located(message: String) -> SourceLocation callsite\n  callsite\nend\n",
    );
    source.push_str("pub fn alias_0 = located\n");
    for index in 1..70 {
        source.push_str(&format!("pub fn alias_{index} = alias_{}\n", index - 1));
    }
    source.push_str("fn caller() -> SourceLocation\n  alias_69(\"hello\")\nend\n");
    let snapshot = snapshot(&source);

    let help = signature_help_at(
        &snapshot,
        SourcePosition {
            source: SourcePath::new("main.veln"),
            line: 75,
            column: 19,
        },
    )
    .expect("deep alias signature help");

    assert_eq!(
        help.label,
        "fn located(message: String) -> SourceLocation callsite"
    );
}

#[test]
fn signature_help_rejects_a_public_alias_cycle() {
    let snapshot = snapshot(concat!(
        "pub fn first = second\n",
        "pub fn second = first\n",
        "fn caller() -> Int\n",
        "  first()\n",
        "end\n",
    ));

    assert!(
        signature_help_at(
            &snapshot,
            SourcePosition {
                source: SourcePath::new("main.veln"),
                line: 4,
                column: 9,
            },
        )
        .is_none()
    );
}

#[path = "tests/work.rs"]
mod work;
