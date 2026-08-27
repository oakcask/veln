use super::*;
use crate::types::environment::TypeEnvironment;
use std::collections::BTreeSet;
use veln_ast::{UseDecl, lower_surface_ast_with_module_identity};
use veln_source::TextRange;

#[test]
fn recovery_is_not_visible_through_an_import() {
    let module = merged_modules(vec![
        SourceFile::new(
            "main.veln",
            concat!(
                "mod main\n",
                "use helper\n",
                "fn main() -> Int\n",
                "  helper::Bad()\n",
                "end\n",
            ),
        ),
        SourceFile::new("helper.veln", "mod helper\npub fn Bad() -> Int\n  1\nend\n"),
    ]);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `helper::Bad`"
    }));
}

#[test]
fn dependency_function_recovery_does_not_cross_into_consumer_environment() {
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dependency/foo.veln",
            concat!("mod foo\n", "pub fn Bad() -> Int\n", "  1\n", "end\n"),
        ))
        .tree,
    );
    let dependency_environment = TypeEnvironment::from_module(&dependency);
    assert_eq!(
        dependency_environment.local_call_recovery_candidate_count("Bad", Some("foo"), 0),
        1
    );
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "main.veln",
            concat!(
                "mod main\n",
                "use foo\n",
                "fn main() -> Int\n",
                "  Bad()\n",
                "end\n",
            ),
        ))
        .tree,
    );

    let environment =
        TypeEnvironment::from_module_with_base_for_test(&consumer, &dependency_environment);

    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("main"), 0),
        0
    );
    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("foo"), 0),
        0
    );
}

#[test]
fn consumer_function_recovery_does_not_cross_into_dependency_environment() {
    let consumer = lower_surface_ast(
        &parse(&SourceFile::new(
            "main.veln",
            concat!("mod main\n", "pub fn Bad() -> Int\n", "  1\n", "end\n"),
        ))
        .tree,
    );
    let consumer_environment = TypeEnvironment::from_module(&consumer);
    assert_eq!(
        consumer_environment.local_call_recovery_candidate_count("Bad", Some("main"), 0),
        1
    );
    let dependency = lower_surface_ast(
        &parse(&SourceFile::new(
            "dependency/foo.veln",
            concat!("mod foo\n", "fn read() -> Int\n", "  Bad()\n", "end\n",),
        ))
        .tree,
    );

    let environment =
        TypeEnvironment::from_module_with_base_for_test(&dependency, &consumer_environment);

    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("foo"), 0),
        0
    );
    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("main"), 0),
        0
    );
}

#[test]
fn application_recovery_does_not_cross_into_implicit_prelude_environment() {
    let application = module_with_identity(
        "main.veln",
        concat!(
            "mod app\n",
            "pub fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "type bad\n",
            "  Token(Int)\n",
            "end\n",
        ),
        "app",
    );
    let application_environment = TypeEnvironment::from_module(&application);
    assert_eq!(
        application_environment.local_call_recovery_candidate_count("Bad", Some("app"), 0),
        1
    );
    assert_eq!(
        application_environment.local_call_recovery_candidate_count("Token", Some("app"), 1),
        1
    );
    let prelude = module_with_identity(
        "prelude.veln",
        concat!(
            "mod std::prelude\n",
            "pub fn read_missing_function() -> Int\n",
            "  Bad()\n",
            "end\n",
            "pub fn read_missing_constructor() -> Int\n",
            "  Token(1)\n",
            "end\n",
        ),
        "std::prelude",
    );

    let environment =
        TypeEnvironment::from_module_with_base_for_test(&prelude, &application_environment);

    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("std::prelude"), 0),
        0
    );
    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("app"), 0),
        0
    );
    assert_eq!(
        environment.local_call_recovery_candidate_count("Token", Some("std::prelude"), 1),
        0
    );
    let diagnostics = analyze_surface_module_with_base_for_test(&prelude, &application_environment);

    assert_has_diagnostic(
        &diagnostics,
        "name.unresolved",
        "unresolved call_target `Bad`",
    );
    assert_has_diagnostic(
        &diagnostics,
        "name.unresolved",
        "unresolved call_target `Token`",
    );
}

#[test]
fn implicit_prelude_recovery_does_not_cross_into_application_environment() {
    let prelude = module_with_identity(
        "prelude.veln",
        concat!(
            "mod std::prelude\n",
            "pub fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "type bad\n",
            "  Token(Int)\n",
            "end\n",
        ),
        "std::prelude",
    );
    let prelude_environment = TypeEnvironment::from_module(&prelude);
    assert_eq!(
        prelude_environment.local_call_recovery_candidate_count("Bad", Some("std::prelude"), 0),
        1
    );
    assert_eq!(
        prelude_environment.local_call_recovery_candidate_count("Token", Some("std::prelude"), 1),
        1
    );
    let application = application_module_with_implicit_prelude(
        "main.veln",
        concat!(
            "mod app\n",
            "fn missing_function() -> Int\n",
            "  Bad()\n",
            "end\n",
            "fn missing_constructor() -> Int\n",
            "  Token(1)\n",
            "end\n",
        ),
        "app",
    );

    let environment =
        TypeEnvironment::from_module_with_base_for_test(&application, &prelude_environment);

    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("app"), 0),
        0
    );
    assert_eq!(
        environment.local_call_recovery_candidate_count("Bad", Some("std::prelude"), 0),
        0
    );
    assert_eq!(
        environment.local_call_recovery_candidate_count("Token", Some("app"), 1),
        0
    );
    let reusable = prepare_reusable_standard_surface_module_environment(&prelude)
        .with_current_identity_for_test();
    let mut selected_standard_module_names = BTreeSet::new();
    selected_standard_module_names.insert("std::prelude".to_string());

    let (diagnostics, lowered) = check_project_surface_module_with_standard_modules_environment(
        &application,
        &selected_standard_module_names,
        &reusable,
    );

    assert_has_diagnostic(
        &diagnostics,
        "name.unresolved",
        "unresolved call_target `Bad`",
    );
    assert_has_diagnostic(
        &diagnostics,
        "name.unresolved",
        "unresolved call_target `Token`",
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn valid_implicit_prelude_function_precedes_application_recovery_record() {
    let standard = module_with_identity(
        "prelude.veln",
        concat!(
            "mod std::prelude\n",
            "pub fn token(value: Int) -> Int\n",
            "  value\n",
            "end\n",
        ),
        "std::prelude",
    );
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let application = application_module_with_implicit_prelude(
        "main.veln",
        concat!(
            "mod app\n",
            "type bad\n",
            "  token(Int)\n",
            "end\n",
            "fn main() -> Int\n",
            "  token(1)\n",
            "end\n",
        ),
        "app",
    );
    let mut selected_standard_module_names = BTreeSet::new();
    selected_standard_module_names.insert("std::prelude".to_string());

    let (diagnostics, lowered) = check_project_surface_module_with_standard_modules_environment(
        &application,
        &selected_standard_module_names,
        &reusable,
    );

    assert_no_name_resolution_failure(&diagnostics);
    assert_no_name_resolution_failure(&lowered.diagnostics);
    assert_has_diagnostic(
        &diagnostics,
        "name.invalid_case",
        "type name `bad` must start with an ASCII uppercase letter",
    );
    assert_has_diagnostic(
        &diagnostics,
        "name.invalid_case",
        "constructor name `token` must start with an ASCII uppercase letter",
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn valid_implicit_prelude_constructor_precedes_application_recovery_record() {
    let standard = module_with_identity(
        "prelude.veln",
        concat!(
            "mod std::prelude\n",
            "pub type Token\n",
            "  Valid(Int)\n",
            "end\n",
        ),
        "std::prelude",
    );
    let reusable = prepare_reusable_standard_surface_module_environment(&standard)
        .with_current_identity_for_test();
    let application = application_module_with_implicit_prelude(
        "main.veln",
        concat!(
            "mod app\n",
            "type bad\n",
            "  Valid(Int)\n",
            "end\n",
            "fn main() -> prelude::Token\n",
            "  Valid(1)\n",
            "end\n",
        ),
        "app",
    );
    let mut selected_standard_module_names = BTreeSet::new();
    selected_standard_module_names.insert("std::prelude".to_string());

    let (diagnostics, lowered) = check_project_surface_module_with_standard_modules_environment(
        &application,
        &selected_standard_module_names,
        &reusable,
    );

    assert_no_name_resolution_failure(&diagnostics);
    assert_no_name_resolution_failure(&lowered.diagnostics);
    assert_has_diagnostic(
        &diagnostics,
        "name.invalid_case",
        "type name `bad` must start with an ASCII uppercase letter",
    );
    assert!(lowered.core.is_none());
    assert!(lowered.ir.is_none());
}

#[test]
fn recovery_is_not_visible_through_public_alias_targets() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "type item\n",
            "  Value\n",
            "end\n",
            "pub fn exposed = Bad\n",
            "pub type Exposed = item\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "function name `Bad` must start with an ASCII lowercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message == "type name `item` must start with an ASCII uppercase letter"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved function alias target `Bad`"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved type alias target `item`"
    }));

    let environment = TypeEnvironment::from_module(&module);
    assert!(environment.function("exposed").is_none());
}

fn module_with_identity(path: &str, text: &str, module_name: &str) -> SurfaceModule {
    let source = SourceFile::new(path, text);
    lower_surface_ast_with_module_identity(
        &parse(&source).tree,
        module_name.to_string(),
        source.span(TextRange::new(0, 0)),
    )
}

fn application_module_with_implicit_prelude(
    path: &str,
    text: &str,
    module_name: &str,
) -> SurfaceModule {
    let mut module = module_with_identity(path, text, module_name);
    let source = SourceFile::new(path, text);
    module.uses.push(UseDecl::implicit_standard_prelude(
        module_name.to_string(),
        source.span(TextRange::new(0, 0)),
    ));
    module
}

fn assert_has_diagnostic(diagnostics: &[Diagnostic], id: &str, message: &str) {
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.id == id && diagnostic.message == message),
        "missing {id} diagnostic `{message}` in {diagnostics:#?}"
    );
}

fn assert_no_name_resolution_failure(diagnostics: &[Diagnostic]) {
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"
                && diagnostic.id != "name.ambiguous"),
        "{diagnostics:#?}"
    );
}

#[test]
fn invalid_public_function_alias_recovery_suppresses_derivative_unresolved_without_lookup() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn main() -> Int\n",
            "  Exported()\n",
            "end\n",
            "fn good() -> Int\n",
            "  1\n",
            "end\n",
            "pub fn Exported = good\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.invalid_case"
            && diagnostic.message
                == "function name `Exported` must start with an ASCII lowercase letter"
    }));
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
    assert!(
        TypeEnvironment::from_module(&module)
            .function("Exported")
            .is_none()
    );
}

#[test]
fn recovery_facts_cover_functions_aliases_and_constructors_together() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type item\n",
            "  Value(Int)\n",
            "end\n",
            "fn Bad(input: Int) -> Int\n",
            "  input\n",
            "end\n",
            "fn good(input: Int) -> Int\n",
            "  input\n",
            "end\n",
            "pub fn Exported = good\n",
            "fn main() -> Int\n",
            "  let direct = Bad(1)\n",
            "  let aliased = Exported(1)\n",
            "  let constructed = Value(1)\n",
            "  direct\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);
    let diagnostics = analyze_surface_module(&module);

    assert_eq!(
        diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.id == "name.invalid_case")
            .count(),
        3,
        "{diagnostics:#?}"
    );
    assert!(
        diagnostics
            .iter()
            .all(|diagnostic| diagnostic.id != "name.unresolved"),
        "{diagnostics:#?}"
    );
}
