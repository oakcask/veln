use super::*;

#[test]
fn public_function_requires_explicit_type_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main(value)\n  value\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public parameter `value` has no type annotation"
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "type.public_signature_missing"
            && diagnostic.message == "public function has no return type annotation"
    }));
}

#[test]
fn public_function_accepts_omitted_empty_effect_boundary() {
    let source = SourceFile::new("main.veln", "pub fn main() -> Int\n  1\nend\n");
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn nominal_effect_perform_checks_and_lowers_operation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String, count: Int) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "  perform Audit::record(\"user\", 1)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be present");
    assert_eq!(core.effects.len(), 1);
    assert_eq!(core.functions[0].effects, ["Audit"]);
    let veln_core::CoreStmtKind::Return { expr } = &core.functions[0].body[0].kind else {
        panic!("expected return statement");
    };
    assert!(matches!(
        &expr.kind,
        veln_core::CoreExprKind::Perform { effect, operation, args }
            if effect == "Audit" && operation == "record" && args.len() == 2
    ));
}

#[test]
fn nominal_effect_unknown_operation_reports_operation_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String effects [Audit]\n",
            "  perform Audit::missing(\"user\")\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "effect.unknown_operation");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.line, 6);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 18);
}

#[test]
fn nominal_effect_unknown_perform_reports_effect_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main() -> String effects [MissingAudit]\n",
            "  perform MissingAudit::record(\"user\")\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "effect.unknown"
                && diagnostic.message == "performed effect `MissingAudit` is not known"
        })
        .unwrap_or_else(|| panic!("expected performed unknown effect: {diagnostics:#?}"));
    assert_eq!(diagnostic.span.as_ref().unwrap().start.line, 2);
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 11);
    assert_eq!(diagnostic.span.as_ref().unwrap().end.column, 23);
}

#[test]
fn nominal_effect_missing_public_reports_perform_provenance() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> String\n",
            "end\n",
            "\n",
            "pub fn main() -> String\n",
            "  perform Audit::record(\"user\")\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.missing_public")
        .unwrap_or_else(|| panic!("expected missing public effect: {diagnostics:#?}"));
    assert_eq!(
        diagnostic.message,
        "public function uses undeclared effect `Audit`"
    );
    assert_eq!(diagnostic.related.len(), 1);
    let related = diagnostic.related[0].to_json();
    assert!(related.contains("\"kind\":\"effect_provenance\""));
    assert!(related.contains("Call to `Audit::record` requires this effect."));
    assert!(related.contains("\"start\":{\"line\":6,\"column\":3,"));
}

#[test]
fn public_handler_requires_and_canonicalizes_declared_clause_effects() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn traced(offset: Int) -> Int effects [stdio]\n",
            "  stdio::println(\"provider\")\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "pub handler missing(offset: Int) handles Ask\n",
            "  value() => traced(offset)\n",
            "end\n",
            "\n",
            "pub handler declared(offset: Int) handles Ask effects [stdio, stdio]\n",
            "  value() => traced(offset)\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let environment = TypeEnvironment::from_module(&module);
    let declared = match environment.handler_path(&["declared".to_string()], None) {
        HandlerPathResolution::Found(handler) => handler,
        HandlerPathResolution::PrivateCompanionTargetMismatch { .. }
        | HandlerPathResolution::QuarantinedImportTarget
        | HandlerPathResolution::Missing => panic!("declared handler should be present"),
    };
    assert_eq!(declared.effects, ["stdio"]);

    let diagnostics = analyze_surface_module(&module);

    let missing = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "handler.missing_public_effect")
        .unwrap_or_else(|| panic!("expected missing handler effect: {diagnostics:#?}"));
    assert_eq!(
        missing.message,
        "public handler `missing` uses undeclared effect `stdio`"
    );
    assert_eq!(missing.span.as_ref().unwrap().start.line, 11);
}

#[test]
fn handler_clause_parameters_shadow_context_only_inside_clause_body() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Pick\n",
            "  choose(ctx: Int) -> Int\n",
            "  current() -> Int\n",
            "end\n",
            "\n",
            "handler pick(ctx: Int) handles Pick\n",
            "  choose(ctx) => ctx\n",
            "  current() => ctx\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn handler_clause_arity_diagnostic_reports_parameter_boundary() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Pick\n",
            "  next(left: Int, right: Int) -> Int\n",
            "end\n",
            "\n",
            "handler pick() handles Pick\n",
            "  next(left) => left\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "handler.operation_clause_arity")
        .unwrap_or_else(|| panic!("expected operation clause arity diagnostic: {diagnostics:#?}"));
    let span = diagnostic.span.as_ref().expect("arity diagnostic span");
    assert_eq!(span.start.line, 6);
    assert_eq!(span.start.column, 8);
    assert_eq!(span.end.line, 6);
    assert_eq!(span.end.column, 12);
}

#[test]
fn unknown_declared_effect_reports_effect_label_span() {
    let source = SourceFile::new(
        "main.veln",
        "pub fn main() -> () effects [stdio, telepathy]\n  ()\nend\n",
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "effect.unknown");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.line, 1);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 37);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().end.column, 46);
}

#[test]
fn unknown_function_type_effect_reports_effect_label_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(callback: fn() -> () effects [MissingAudit]) -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "effect.unknown"
                && diagnostic.message == "function type effect `MissingAudit` is not known"
        })
        .unwrap_or_else(|| panic!("expected function type unknown effect: {diagnostics:#?}"));
    assert_eq!(diagnostic.span.as_ref().unwrap().start.line, 1);
    assert_eq!(diagnostic.span.as_ref().unwrap().start.column, 43);
    assert_eq!(diagnostic.span.as_ref().unwrap().end.column, 55);
}

#[test]
fn imported_qualified_effect_is_known_in_function_type() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod main\n",
            "use logging\n",
            "\n",
            "pub fn main(callback: fn() -> () effects [logging::Audit]) -> ()\n",
            "  ()\n",
            "end\n",
            "\n",
            "pub effect Audit\n",
            "  record() -> ()\n",
            "end\n",
        ),
    );
    let mut module = lower_surface_ast(&parse(&source).tree);
    module.effects[0].module_name = Some("logging".to_string());

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn matching_companion_resolves_qualified_private_target_effects() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "\n",
            "fn provide(offset: Int) -> Int\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "handler ask(offset: Int) handles math::Ask\n",
            "  value() => provide()\n",
            "end\n",
            "\n",
            "handler traced() handles math::Trace effects [math::Ask]\n",
            "  ping() => trace()\n",
            "end\n",
            "\n",
            "fn trace() -> ()\n",
            "  ()\n",
            "end\n",
            "\n",
            "fn accepts_private_effect_callback(callback: fn() -> () effects [math::Ask]) -> () effects [math::Ask]\n",
            "  perform math::Ask::value()\n",
            "  ()\n",
            "end\n",
            "\n",
            "test companion_uses_private_target_effect() -> () effects [math::Ask]\n",
            "  perform math::Ask::value()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "effect Trace\n",
            "  ping() -> ()\n",
            "end\n",
        ),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: target.effects,
        handlers: companion.handlers,
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn wrong_companion_handler_effect_reports_target_mismatch() {
    let companion_source = SourceFile::new(
        "other.test.veln",
        concat!(
            "mod other__test_companion\n",
            "use other\n",
            "use math\n",
            "\n",
            "handler local() handles other::Local effects [math::Ask]\n",
            "  value() => provide(offset)\n",
            "end\n",
            "\n",
            "fn provide() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "effect Ask\n", "  value() -> Int\n", "end\n",),
    );
    let other_source = SourceFile::new(
        "other.veln",
        concat!(
            "mod other\n",
            "effect Local\n",
            "  value() -> Int\n",
            "end\n",
        ),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let other = lower_surface_ast(&parse(&other_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: target.effects.into_iter().chain(other.effects).collect(),
        handlers: companion.handlers,
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: companion.functions,
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.private_companion_target")
        .unwrap_or_else(|| panic!("expected companion target diagnostic: {diagnostics:#?}"));
    assert_eq!(
        diagnostic.message,
        "private effect `math::Ask` belongs to `math` instead of companion target `other`"
    );
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"boundary\":\"handler_declaration_effects\""));
    assert!(details.contains("\"reason\":\"companion_target_mismatch\""));
    assert!(details.contains("\"companion_path\":\"other.test.veln\""));
    assert!(details.contains("\"companion_target_module\":\"other\""));
    assert!(details.contains("\"effect_module\":\"math\""));
}

#[test]
fn wrong_companion_private_target_effect_reports_target_mismatch() {
    let companion_source = SourceFile::new(
        "other.test.veln",
        concat!(
            "mod other__test_companion\n",
            "use math\n",
            "\n",
            "test wrong_companion_uses_private_target_effect() -> () effects [math::Ask]\n",
            "  perform math::Ask::value()\n",
            "  ()\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!("mod math\n", "effect Ask\n", "  value() -> Int\n", "end\n",),
    );
    let other_source = SourceFile::new(
        "other.veln",
        concat!("mod other\n", "pub fn present() -> ()\n", "  ()\n", "end\n",),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let other = lower_surface_ast(&parse(&other_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: target.effects,
        handlers: Vec::new(),
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(other.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "effect.private_companion_target")
        .unwrap_or_else(|| panic!("expected companion target diagnostic: {diagnostics:#?}"));
    assert_eq!(
        diagnostic.message,
        "private effect `math::Ask` belongs to `math` instead of companion target `other`"
    );
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"reason\":\"companion_target_mismatch\""));
    assert!(details.contains("\"companion_path\":\"other.test.veln\""));
    assert!(details.contains("\"companion_target_module\":\"other\""));
    assert!(details.contains("\"effect_module\":\"math\""));
}

#[test]
fn matching_companion_handles_with_private_target_handler() {
    let companion_source = SourceFile::new(
        "math.test.veln",
        concat!(
            "mod math__test_companion\n",
            "use math\n",
            "\n",
            "test companion_uses_private_target_handler() -> ()\n",
            "  let observed = handle math::compute() with math::ask(41)\n",
            "  if observed == 42\n",
            "    ()\n",
            "  else\n",
            "    ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide(offset: Int) -> Int\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "handler ask(offset: Int) handles Ask\n",
            "  value() => provide(offset)\n",
            "end\n",
            "\n",
            "pub fn compute() -> Int effects [Ask]\n",
            "  perform Ask::value()\n",
            "end\n",
        ),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: target.effects,
        handlers: target.handlers,
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(target.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn wrong_companion_private_target_handler_reports_target_mismatch() {
    let companion_source = SourceFile::new(
        "other.test.veln",
        concat!(
            "mod other__test_companion\n",
            "use other\n",
            "use math\n",
            "\n",
            "test wrong_companion_uses_private_target_handler() -> ()\n",
            "  let observed = handle other::compute() with math::ask(41)\n",
            "  if observed == 1\n",
            "    ()\n",
            "  else\n",
            "    ()\n",
            "  end\n",
            "end\n",
        ),
    );
    let target_source = SourceFile::new(
        "math.veln",
        concat!(
            "mod math\n",
            "effect Ask\n",
            "  value() -> Int\n",
            "end\n",
            "\n",
            "fn provide(offset: Int) -> Int\n",
            "  offset + 1\n",
            "end\n",
            "\n",
            "handler ask(offset: Int) handles Ask\n",
            "  value() => provide(ctx)\n",
            "end\n",
        ),
    );
    let other_source = SourceFile::new(
        "other.veln",
        concat!("mod other\n", "pub fn compute() -> Int\n", "  1\n", "end\n",),
    );
    let companion = lower_surface_ast(&parse(&companion_source).tree);
    let target = lower_surface_ast(&parse(&target_source).tree);
    let other = lower_surface_ast(&parse(&other_source).tree);
    let module = SurfaceModule {
        module: companion.module,
        uses: companion.uses,
        aliases: Vec::new(),
        effects: target.effects,
        handlers: target.handlers,
        types: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        functions: companion
            .functions
            .into_iter()
            .chain(other.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "handler.private_companion_target")
        .unwrap_or_else(|| panic!("expected companion target diagnostic: {diagnostics:#?}"));
    assert_eq!(
        diagnostic.message,
        "private handler `math::ask` belongs to `math` instead of companion target `other`"
    );
    let details = diagnostic.details.to_json();
    assert!(details.contains("\"boundary\":\"handle_expression\""));
    assert!(details.contains("\"reason\":\"companion_target_mismatch\""));
    assert!(details.contains("\"companion_path\":\"other.test.veln\""));
    assert!(details.contains("\"companion_target_module\":\"other\""));
    assert!(details.contains("\"handler_module\":\"math\""));
}

#[test]
fn duplicate_effect_operation_reports_operation_name_span() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "effect Audit\n",
            "  record(user: String) -> String\n",
            "  record(user: String) -> String\n",
            "end\n",
        ),
    );
    let module = lower_surface_ast(&parse(&source).tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
    assert_eq!(diagnostics[0].id, "name.duplicate");
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.line, 3);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().start.column, 3);
    assert_eq!(diagnostics[0].span.as_ref().unwrap().end.column, 9);
    assert_eq!(diagnostics[0].related.len(), 1);
    assert!(
        diagnostics[0].related[0]
            .to_json()
            .contains("\"kind\":\"duplicate_origin\"")
    );
}
