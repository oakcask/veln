use super::*;

fn analyze(sources: &[(&str, &str)]) -> Report {
    let mut index = Index::default();
    for (file, source) in sources {
        index.add(Path::new(file), source).unwrap();
    }
    index.finish(sources.len())
}

fn names(report: &Report) -> Vec<&str> {
    report
        .findings
        .iter()
        .map(|item| item.name.as_str())
        .collect()
}

#[test]
fn reports_public_items_at_definition_lines_without_counting_declarations_as_uses() {
    let report = analyze(&[(
        "src/lib.rs",
        "\n\
        pub struct Unused;\n\
        pub enum Choice { A }\n\
        pub trait Trait {}\n\
        pub type Alias = ();\n\
        pub const VALUE: () = ();\n\
        pub static STATIC: () = ();\n\
        pub fn unused() {}\n\
        pub(crate) fn internal() {}\n\
        fn private() {}",
    )]);
    assert_eq!(
        names(&report),
        [
            "Unused", "Choice", "Trait", "Alias", "VALUE", "STATIC", "unused"
        ]
    );
    assert_eq!(report.definitions, 7);
    assert_eq!(report.findings[0].line, 2);
}

#[test]
fn cross_file_paths_types_and_method_calls_supply_reference_evidence() {
    let report = analyze(&[
        (
            "a/lib.rs",
            "pub struct Value; impl Value { pub fn work(&self) {} } pub fn make() {}",
        ),
        (
            "b/main.rs",
            "fn main() { let x: a::Value = a::Value; x.work(); a::make(); }",
        ),
    ]);
    assert!(report.findings.is_empty());
}

#[test]
fn recursion_and_impl_headers_do_not_prove_a_consumer_exists() {
    let report = analyze(&[(
        "src/lib.rs",
        "pub struct Empty; impl Empty { pub fn recur() { Self::recur(); } } pub fn recursive() { recursive(); }",
    )]);
    assert_eq!(names(&report), ["Empty", "recur", "recursive"]);
}

#[test]
fn distinguishes_test_references_and_excludes_test_definitions() {
    let report = analyze(&[
        (
            "src/lib.rs",
            "pub fn checked() {} pub fn orphan() {} #[cfg(test)] mod tests { pub fn helper() { super::checked(); } }",
        ),
        (
            "tests/integration.rs",
            "pub fn test_helper() { crate::checked(); }",
        ),
        ("src/other_tests.rs", "pub fn other_helper() {}"),
    ]);
    assert_eq!(names(&report), ["checked", "orphan"]);
    assert_eq!(report.findings[0].evidence, "test-only-references");
    assert_eq!(report.findings[1].evidence, "no-references");
}

#[test]
fn test_scope_is_restored_and_file_inner_cfg_is_respected() {
    let report = analyze(&[
        (
            "src/lib.rs",
            "pub fn used() {} #[cfg(test)] fn test() { used(); } fn production() { used(); }",
        ),
        ("src/support.rs", "#![cfg(test)] pub fn helper() {}"),
    ]);
    assert_eq!(report.definitions, 1);
    assert!(report.findings.is_empty());
}

#[test]
fn standalone_test_attributes_mark_references_as_test_only() {
    let report = analyze(&[(
        "src/lib.rs",
        "pub fn helper() {} #[test] fn check() { helper(); }",
    )]);
    assert_eq!(names(&report), ["helper"]);
    assert_eq!(report.findings[0].evidence, "test-only-references");
}

#[test]
fn out_of_line_module_cfg_is_not_propagated_without_module_resolution() {
    let report = analyze(&[
        ("src/lib.rs", "pub fn helper() {} #[cfg(test)] mod support;"),
        ("src/support.rs", "fn check() { crate::helper(); }"),
    ]);
    assert!(report.findings.is_empty());
}

#[test]
fn macro_tokens_and_derives_count_as_conservative_evidence() {
    let report = analyze(&[(
        "src/lib.rs",
        "pub struct MacroArg; pub trait CustomDerive {} invoke!([MacroArg]); #[derive(CustomDerive)] struct Used;",
    )]);
    assert!(report.findings.is_empty());
}

#[test]
fn comments_docs_and_strings_do_not_count_as_code_references() {
    let report = analyze(&[(
        "src/lib.rs",
        "/// Mentioned\npub struct Mentioned; fn f() { let text = \"Mentioned\"; /* Mentioned */ }",
    )]);
    assert_eq!(names(&report), ["Mentioned"]);
}

#[test]
fn imports_and_aliases_hide_unused_exports_by_design() {
    let report = analyze(&[(
        "src/lib.rs",
        "pub struct Exported; pub fn renamed() {} pub use Exported as Alias; use renamed as alias; // no consumers",
    )]);
    assert!(report.findings.is_empty());
}

#[test]
fn same_name_collisions_are_a_documented_false_negative() {
    let report = analyze(&[(
        "src/lib.rs",
        "mod a { pub fn shared() {} } mod b { pub fn shared() {} fn consumer() { shared(); } }",
    )]);
    assert_eq!(report.definitions, 2);
    assert!(report.findings.is_empty());
}

#[test]
fn glob_imports_do_not_resolve_members_and_may_produce_false_positives() {
    let report = analyze(&[(
        "src/lib.rs",
        "mod a { pub struct HiddenByGlob; } pub use a::*;",
    )]);
    assert_eq!(names(&report), ["HiddenByGlob"]);
}

#[test]
fn trait_implementation_methods_are_not_public_candidates() {
    let report = analyze(&[(
        "src/lib.rs",
        "struct S; impl Trait for S { fn callback(&self) {} } impl S { pub fn candidate(&self) {} }",
    )]);
    assert_eq!(names(&report), ["candidate"]);
}

#[test]
fn malformed_sources_fail_instead_of_silently_omitting_a_file() {
    let error = Index::default()
        .add(Path::new("src/broken.rs"), "pub fn {")
        .unwrap_err();
    assert!(error.contains("src/broken.rs"));
}

#[test]
fn generated_collections_preserve_all_distinct_candidates() {
    for count in [1000, 2000] {
        let source: String = (0..count)
            .map(|i| format!("pub fn candidate_{i}() {{}}\n"))
            .collect();
        let report = analyze(&[("src/lib.rs", &source)]);
        assert_eq!(report.definitions, count);
        assert_eq!(report.findings.len(), count);
    }
}
