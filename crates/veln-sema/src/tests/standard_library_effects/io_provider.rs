use super::*;

fn diagnostics(text: &str) -> Vec<Diagnostic> {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    analyze_surface_module(&lower_surface_ast(&parsed.tree))
}

const PREFIX: &str = "fn add(value: Int, amount: Int) -> {state: Int, result: Bool}\n  {state: value + amount, result: true}\nend\n";

#[test]
fn io_provider_update_links_state_context_callback_and_result() {
    let text = format!(
        "{PREFIX}pub fn main() -> Bool effects [concurrency]\n  let state = io_provider::state(0)\n  io_provider::update(state, 2, add)\nend\n"
    );
    assert!(diagnostics(&text).is_empty(), "{:?}", diagnostics(&text));
}

#[test]
fn io_provider_update_rejects_wrong_context_and_callback_state() {
    for call in [
        "io_provider::update(state, false, add)",
        "io_provider::update(io_provider::state(false), 2, add)",
    ] {
        let text = format!(
            "{PREFIX}pub fn main() -> Bool effects [concurrency]\n  let state = io_provider::state(0)\n  {call}\nend\n"
        );
        let observed = diagnostics(&text);
        assert!(
            observed
                .iter()
                .any(|diagnostic| diagnostic.id == "type.mismatch"),
            "{call}: {observed:?}"
        );
    }
}

#[test]
fn io_provider_update_rejects_effectful_transition() {
    let text = "fn action(value: Int, amount: Int) -> {state: Int, result: Bool} effects [stdio]\n  stdio::println(\"effectful\")\n  {state: value + amount, result: true}\nend\npub fn main() -> Bool effects [concurrency]\n  let state = io_provider::state(0)\n  io_provider::update(state, 2, action)\nend\n";
    let observed = diagnostics(text);
    assert!(
        observed
            .iter()
            .any(|diagnostic| diagnostic.id == "type.mismatch"),
        "{observed:?}"
    );
}

#[test]
fn io_provider_state_requires_only_concurrency() {
    let observed =
        diagnostics("pub fn main() -> ()\n  let state = io_provider::state(0)\n  ()\nend\n");
    assert_eq!(observed.len(), 1, "{observed:?}");
    assert_eq!(observed[0].id, "effect.missing_public");
    assert!(observed[0].message.contains("concurrency"));
}
