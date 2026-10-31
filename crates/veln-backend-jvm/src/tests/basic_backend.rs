use super::*;

#[test]
#[should_panic(expected = "module values must be rejected before JVM backend lowering")]
fn backend_rejects_module_values_before_classfile_generation() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "let answer: Int = 42\n",
            "pub fn main() -> Int\n",
            "  answer\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast_with_module_identity(
        &parsed.tree,
        "main".to_string(),
        source.span(TextRange::at(0)),
    );
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.id == "module.value_not_executable" })
    );
    let ir = lowered.ir.expect("execution gate should retain typed IR");

    generate_classfiles_with_entry(&ir, "main");
}

#[test]
fn explicit_aliases_execute_only_the_selected_package_function_and_handler() {
    // Exercise package selection and JVM linkage for same-spelled declarations.
    let root = temp_dir("explicit-alias-package-identity");
    let analysis = explicit_alias_package_analysis(&root);

    for case in [
        AliasEntryCase::new("local_call", "endpoint", None, "1\n"),
        AliasEntryCase::new("dependency_call", "endpoint", Some("dependency"), "2\n"),
        AliasEntryCase::new("local_handler", "private_value", None, "1\n"),
        AliasEntryCase::new(
            "dependency_handler",
            "private_value",
            Some("dependency"),
            "2\n",
        ),
    ] {
        assert_explicit_alias_entry(&analysis, case);
    }
    let _ = fs::remove_dir_all(root);
}

fn explicit_alias_package_analysis(root: &std::path::Path) -> veln_analysis::ProjectAnalysis {
    fs::create_dir_all(root.join("vendor/dependency")).expect("dependency source directory");
    fs::write(
        root.join("veln.toml"),
        "[dependencies.\"dependency\"]\npath = \"vendor/dependency\"\n",
    )
    .expect("project manifest");
    fs::write(
        root.join("vendor/dependency/veln.toml"),
        "[package]\nname = \"dependency\"\n[lib]\nexports = [\"shared.veln\"]\n",
    )
    .expect("dependency manifest");
    fs::write(root.join("main.veln"), concat!(
        "use shared as local_wire\n",
        "use shared as dependency_wire from \"dependency\"\n",
        "pub fn local_call() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(local_wire::endpoint()))\nend\n",
        "pub fn dependency_call() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(dependency_wire::endpoint()))\nend\n",
        "pub fn local_handler() -> () effects [stdio]\n",
        "  let result = handle perform local_wire::Ask::value() with local_wire::answer()\n",
        "  stdio::println(int_to_string(result))\nend\n",
        "pub fn dependency_handler() -> () effects [stdio]\n",
        "  let result = handle perform dependency_wire::Ask::value() with dependency_wire::answer()\n",
        "  stdio::println(int_to_string(result))\nend\n",
    )).expect("entry sources");
    for (path, value) in [("shared.veln", 1), ("vendor/dependency/shared.veln", 2)] {
        fs::write(
            root.join(path),
            format!(
                concat!(
                    "pub fn endpoint() -> Int\n  {}\nend\n",
                    "pub effect Ask\n  value() -> Int\nend\n",
                    "pub handler answer() for Ask\n  value() => private_value()\nend\n",
                    "fn private_value() -> Int\n  {}\nend\n",
                ),
                value, value
            ),
        )
        .expect("same-spelled target source");
    }
    let project = Project::discover(
        root,
        &[
            std::path::PathBuf::from("main.veln"),
            std::path::PathBuf::from("shared.veln"),
        ],
    )
    .expect("package fixture project");
    let analysis = analyze_project(project, DoctestMode::Exclude);
    let diagnostics = analysis.checked_diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    analysis
}

struct AliasEntryCase<'a> {
    entry: &'a str,
    target: &'a str,
    package: Option<&'a str>,
    output_text: &'a str,
}

impl<'a> AliasEntryCase<'a> {
    const fn new(
        entry: &'a str,
        target: &'a str,
        package: Option<&'a str>,
        output_text: &'a str,
    ) -> Self {
        Self {
            entry,
            target,
            package,
            output_text,
        }
    }
}

fn assert_explicit_alias_entry(
    analysis: &veln_analysis::ProjectAnalysis,
    case: AliasEntryCase<'_>,
) {
    let reachable = analysis.lower_reachable_entry(case.entry, FunctionKind::Function);
    let packages = reachable
        .module
        .functions
        .iter()
        .filter(|function| function.name.as_deref() == Some(case.target))
        .map(|function| function.package_name.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(packages, [case.package], "entry={}", case.entry);
    if case.target == "private_value" {
        assert_selected_handler_package(&reachable.module.handlers, &case);
    }
    assert!(
        reachable.lowered.diagnostics.is_empty(),
        "entry={}: {:#?}",
        case.entry,
        reachable.lowered.diagnostics
    );
    let ir = reachable
        .lowered
        .ir
        .expect("selected package entry should lower");
    let program = generate_classfiles_with_entry(&ir, case.entry);
    if let Some(output) = run_jvm_program_when_java_is_available(case.entry, &program, &[]) {
        assert!(
            output.status.success(),
            "entry={}: {}",
            case.entry,
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            case.output_text,
            "entry={}",
            case.entry
        );
    }
}

fn assert_selected_handler_package(handlers: &[veln_ast::HandlerDecl], case: &AliasEntryCase<'_>) {
    let packages = handlers
        .iter()
        .filter(|handler| handler.name.as_deref() == Some("answer"))
        .map(|handler| handler.package_name.as_deref())
        .collect::<Vec<_>>();
    assert_eq!(packages, [case.package], "entry={}", case.entry);
}

#[test]
fn bytecode_backend_emits_classfiles_without_java_sources() {
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");

    let program = generate_classfiles_with_entry(&ir, "main");

    assert!(program.class("VelnEntry.class").is_some());
    assert!(program.class("VelnProgram.class").is_some());
    assert!(program.class("VelnRuntime.class").is_some());
    assert!(
        program
            .classes
            .iter()
            .all(|class| class.path.ends_with(".class"))
    );
}

#[test]
fn bytecode_backend_sanitizes_custom_program_class_name() {
    let ir = lower_to_ir("pub fn main() -> String\n  \"ok\"\nend\n");
    let program = generate_classfiles_with_entry_arg_types_options(
        &ir,
        "main",
        &[],
        &JvmBackendOptions {
            program_class: "9 bad-name".to_string(),
        },
    );

    assert!(program.class("_9_bad_name.class").is_some());
    assert!(program.class("_9_bad_name$fn_main.class").is_some());
    assert!(program.class("VelnEntry.class").is_some());
}

#[test]
fn bytecode_backend_classfiles_run_when_java_is_available() {
    let ir = lower_to_ir("pub fn main() -> () effects [stdio]\n  stdio::println(\"ok\")\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-run", &program, &[]) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn schema_refinement_validation_preserves_values_and_reports_first_mismatch() {
    let ir = lower_to_ir(concat!(
        "type State\n",
        "  Connected(value: Int)\n",
        "  Closing(value: Int)\n",
        "  Closed(value: Int)\n",
        "end\n",
        "schema Child\n",
        "  first: State::Connected | State::Closing\n",
        "end\n",
        "schema ConnectedPacket\n",
        "  child: Child\n",
        "  nested: {detail: {z_first: State::Connected, a_second: State::Connected}}\n",
        "  second: State::Connected\n",
        "end\n",
        "fn check(packet: {child: {first: State}, nested: {detail: {z_first: State, a_second: State}}, second: State}, base: ByteOffset) -> Result<{child: {first: State::Connected | State::Closing}, nested: {detail: {z_first: State::Connected, a_second: State::Connected}}, second: State::Connected}, DecodeError>\n",
        "  validate ConnectedPacket from packet at base\n",
        "end\n",
        "fn observe_value(packet: {child: {first: State}, nested: {detail: {z_first: State, a_second: State}}, second: State}) -> {child: {first: State}, nested: {detail: {z_first: State, a_second: State}}, second: State} effects [stdio]\n",
        "  stdio::println(\"value\")\n",
        "  packet\n",
        "end\n",
        "fn observe_offset(base: ByteOffset) -> ByteOffset effects [stdio]\n",
        "  stdio::println(\"offset\")\n",
        "  base\n",
        "end\n",
        "fn print_result(result: Result<{child: {first: State::Connected | State::Closing}, nested: {detail: {z_first: State::Connected, a_second: State::Connected}}, second: State::Connected}, DecodeError>) -> () effects [stdio]\n",
        "  match result\n",
        "    Ok(packet) => match packet.child.first\n",
        "      Connected(value) => stdio::println(string::concat(\"ok:\", int_to_string(value)))\n",
        "      Closing(value) => stdio::println(string::concat(\"ok:\", int_to_string(value)))\n",
        "    end\n",
        "    Err(DecodeErrorWithReason(id, carried, path, reason)) => begin\n",
        "      stdio::println(id)\n",
        "      stdio::println(int_to_string(byte_offset_to_int(carried)))\n",
        "      stdio::println(path)\n",
        "      stdio::println(reason)\n",
        "    end\n",
        "    Err(DecodeError(id, carried, path)) => begin\n",
        "      stdio::println(id)\n",
        "      stdio::println(int_to_string(byte_offset_to_int(carried)))\n",
        "      stdio::println(path)\n",
        "    end\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let first_offset = byte_offset(7)?\n",
        "  let second_offset = byte_offset(19)?\n",
        "  let third_offset = byte_offset(23)?\n",
        "  let fourth_offset = byte_offset(29)?\n",
        "  let ordered = validate ConnectedPacket from observe_value({child: {first: Connected(3)}, nested: {detail: {z_first: Connected(4), a_second: Connected(5)}}, second: Connected(6)}) at observe_offset(first_offset)\n",
        "  print_result(ordered)\n",
        "  print_result(check({child: {first: Closed(11)}, nested: {detail: {z_first: Closed(12), a_second: Closed(13)}}, second: Closed(14)}, second_offset))\n",
        "  print_result(check({child: {first: Connected(17)}, nested: {detail: {z_first: Closed(19), a_second: Closed(21)}}, second: Closed(23)}, third_offset))\n",
        "  print_result(check({child: {first: Connected(29)}, nested: {detail: {z_first: Connected(31), a_second: Closed(37)}}, second: Closed(41)}, fourth_offset))\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("schema-refinement-validation", &program, &[])
    else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "value\noffset\nok:3\n",
            "schema.variant_refinement_mismatch\n",
            "19\n",
            "/ConnectedPacket/child/first\n",
            "actual variant `Closed`; expected one of `Connected, Closing`\n",
            "schema.variant_refinement_mismatch\n",
            "23\n",
            "/ConnectedPacket/nested/detail/z_first\n",
            "actual variant `Closed`; expected one of `Connected`\n",
            "schema.variant_refinement_mismatch\n",
            "29\n",
            "/ConnectedPacket/nested/detail/a_second\n",
            "actual variant `Closed`; expected one of `Connected`\n",
        )
    );
}

#[test]
fn schema_refinement_validation_executes_source_adt_payload_plan() {
    let ir = lower_to_ir(concat!(
        "type State\n  Connected\n  Closed\nend\n",
        "type Box<A>\n  Boxed(A)\n  Empty\nend\n",
        "schema Packet\n  item: Box<State::Connected>\nend\n",
        "fn print_result(result: Result<{item: Box<State::Connected>}, DecodeError>) -> () effects [stdio]\n",
        "  match result\n",
        "    Ok(_) => stdio::println(\"ok\")\n",
        "    Err(DecodeErrorWithReason(id, carried, path, _)) => begin\n",
        "      stdio::println(id)\n",
        "      stdio::println(int_to_string(byte_offset_to_int(carried)))\n",
        "      stdio::println(path)\n",
        "    end\n",
        "    Err(_) => stdio::println(\"unexpected\")\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let offset = byte_offset(47)?\n",
        "  let accepted: {item: Box<State>} = {item: Boxed(Connected)}\n",
        "  let rejected: {item: Box<State>} = {item: Boxed(Closed)}\n",
        "  let absent: {item: Box<State>} = {item: Empty}\n",
        "  print_result(validate Packet from accepted at offset)\n",
        "  print_result(validate Packet from rejected at offset)\n",
        "  print_result(validate Packet from absent at offset)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("schema-refinement-source-adt", &program, &[])
    else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "ok\n",
            "schema.variant_refinement_mismatch\n",
            "47\n",
            "/Packet/item/Boxed/0\n",
            "ok\n",
        )
    );
}

#[test]
fn schema_refinement_validation_executes_regular_recursive_source_adt_plans() {
    let ir = lower_to_ir(concat!(
        "type State\n  Connected\n  Closed\nend\n",
        "type Direct<A>\n  DNext(value: Direct<A>)\n  DValue(value: A)\nend\n",
        "type Left<A>\n  LNext(value: Right<A>)\n  LValue(value: A)\nend\n",
        "type Right<A>\n  RBack(value: Left<A>)\nend\n",
        "schema Packet\n",
        "  direct: Direct<State::Connected>::DNext\n",
        "  mutual: Left<State::Connected>\n",
        "end\n",
        "fn print_result(result: Result<{direct: Direct<State::Connected>::DNext, mutual: Left<State::Connected>}, DecodeError>) -> () effects [stdio]\n",
        "  match result\n",
        "    Ok(_) => stdio::println(\"ok\")\n",
        "    Err(DecodeErrorWithReason(id, carried, path, _)) => begin\n",
        "      stdio::println(id)\n",
        "      stdio::println(int_to_string(byte_offset_to_int(carried)))\n",
        "      stdio::println(path)\n",
        "    end\n",
        "    Err(_) => stdio::println(\"unexpected\")\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let offset = byte_offset(53)?\n",
        "  let accepted: {direct: Direct<State>, mutual: Left<State>} = {direct: DNext(DValue(Connected)), mutual: LNext(RBack(LValue(Connected)))}\n",
        "  let root_failure: {direct: Direct<State>, mutual: Left<State>} = {direct: DValue(Connected), mutual: LValue(Connected)}\n",
        "  let direct_failure: {direct: Direct<State>, mutual: Left<State>} = {direct: DNext(DNext(DValue(Closed))), mutual: LValue(Connected)}\n",
        "  let mutual_failure: {direct: Direct<State>, mutual: Left<State>} = {direct: DNext(DValue(Connected)), mutual: LNext(RBack(LNext(RBack(LValue(Closed)))))}\n",
        "  print_result(validate Packet from accepted at offset)\n",
        "  print_result(validate Packet from root_failure at offset)\n",
        "  print_result(validate Packet from direct_failure at offset)\n",
        "  print_result(validate Packet from mutual_failure at offset)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "schema-refinement-recursive-source-adt",
        &program,
        &[],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "ok\n",
            "schema.variant_refinement_mismatch\n",
            "53\n",
            "/Packet/direct\n",
            "schema.variant_refinement_mismatch\n",
            "53\n",
            "/Packet/direct/DNext/value/DNext/value/DValue/value\n",
            "schema.variant_refinement_mismatch\n",
            "53\n",
            "/Packet/mutual/LNext/value/RBack/value/LNext/value/RBack/value/LValue/value\n",
        )
    );
}

#[test]
fn schema_refinement_validation_executes_explicitly_refined_recursive_edges() {
    let ir = lower_to_ir(concat!(
        "type State\n  Connected\n  Closed\nend\n",
        "type Node<A>\n  Next(state: A, next: Option<Node<A>::Next>)\n  End\nend\n",
        "schema Packet\n  item: Node<State::Connected>::Next\nend\n",
        "fn print_result(result: Result<{item: Node<State::Connected>::Next}, DecodeError>) -> () effects [stdio]\n",
        "  match result\n",
        "    Ok(_) => stdio::println(\"ok\")\n",
        "    Err(DecodeErrorWithReason(id, carried, path, _)) => begin\n",
        "      stdio::println(id)\n",
        "      stdio::println(int_to_string(byte_offset_to_int(carried)))\n",
        "      stdio::println(path)\n",
        "    end\n",
        "    Err(_) => stdio::println(\"unexpected\")\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let offset = byte_offset(61)?\n",
        "  let accepted: {item: Node<State>} = {item: Next(Connected, Some(Next(Connected, None)))}\n",
        "  let rejected: {item: Node<State>} = {item: Next(Connected, Some(Next(Closed, None)))}\n",
        "  print_result(validate Packet from accepted at offset)\n",
        "  print_result(validate Packet from rejected at offset)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "schema-refinement-explicit-recursive-edge",
        &program,
        &[],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "ok\n",
            "schema.variant_refinement_mismatch\n",
            "61\n",
            "/Packet/item/Next/next/Next/state\n",
        )
    );
}

#[test]
fn schema_refinement_validation_executes_finite_nested_generic_instantiations() {
    let ir = lower_to_ir(concat!(
        "type State\n  Connected\n  Closed\nend\n",
        "type Box<A>\n  Boxed(value: A)\nend\n",
        "schema Packet\n  item: Box<Box<State::Connected>>\nend\n",
        "fn print_result(result: Result<{item: Box<Box<State::Connected>>}, DecodeError>) -> () effects [stdio]\n",
        "  match result\n",
        "    Ok(_) => stdio::println(\"ok\")\n",
        "    Err(DecodeErrorWithReason(id, _, path, _)) => begin\n",
        "      stdio::println(id)\n",
        "      stdio::println(path)\n",
        "    end\n",
        "    Err(_) => stdio::println(\"unexpected\")\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Result<(), String> effects [stdio]\n",
        "  let accepted: {item: Box<Box<State>>} = {item: Boxed(Boxed(Connected))}\n",
        "  let rejected: {item: Box<Box<State>>} = {item: Boxed(Boxed(Closed))}\n",
        "  print_result(validate Packet from accepted at byte_offset(0)?)\n",
        "  print_result(validate Packet from rejected at byte_offset(0)?)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "schema-refinement-finite-nested-generic-instantiations",
        &program,
        &[],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "ok\n",
            "schema.variant_refinement_mismatch\n",
            "/Packet/item/Boxed/value/Boxed/value\n",
        )
    );
}

#[test]
fn schema_refinement_validation_executes_imported_recursive_source_adt_plan() {
    let app = SourceFile::new(
        "app.veln",
        concat!(
            "use api\n",
            "schema Packet\n  item: api::Node<api::State::Connected>\nend\n",
            "fn print_result(result: Result<{item: api::Node<api::State::Connected>}, DecodeError>) -> () effects [stdio]\n",
            "  match result\n",
            "    Ok(_) => stdio::println(\"ok\")\n",
            "    Err(DecodeErrorWithReason(id, _, path, _)) => begin\n",
            "      stdio::println(id)\n",
            "      stdio::println(path)\n",
            "    end\n",
            "    Err(_) => stdio::println(\"unexpected\")\n",
            "  end\n",
            "end\n",
            "pub fn main() -> Result<(), String> effects [stdio]\n",
            "  let offset = byte_offset(67)?\n",
            "  let accepted: {item: api::Node<api::State>} = {item: api::Next(api::Value(api::Connected))}\n",
            "  let rejected: {item: api::Node<api::State>} = {item: api::Next(api::Next(api::Value(api::Closed)))}\n",
            "  print_result(validate Packet from accepted at offset)\n",
            "  print_result(validate Packet from rejected at offset)\n",
            "  Ok(())\n",
            "end\n",
        ),
    );
    let api = SourceFile::new(
        "api.veln",
        concat!(
            "pub type State\n  pub Connected\n  pub Closed\nend\n",
            "pub type Node<A>\n  pub Next(value: Node<A>)\n  pub Value(value: A)\nend\n",
        ),
    );
    let analysis = analyze_project(
        Project {
            root: std::path::PathBuf::new(),
            files: vec![app, api],
            manifest: None,
        },
        DoctestMode::Exclude,
    );
    let diagnostics = analysis.checked_diagnostics();
    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
    let lowered = analysis
        .lower_reachable_entry("main", FunctionKind::Function)
        .lowered;
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let ir = lowered
        .ir
        .expect("imported recursive source ADT should lower");
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "schema-refinement-imported-recursive-source-adt",
        &program,
        &[],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "ok\n",
            "schema.variant_refinement_mismatch\n",
            "/Packet/item/Next/value/Next/value/Value/value\n",
        )
    );
}

#[test]
fn schema_refinement_diamond_metadata_growth_is_bounded_by_shared_plan_nodes() {
    let n10 = schema_refinement_diamond_ir(10);
    let n14 = schema_refinement_diamond_ir(14);
    assert_eq!(n10.schema_refinement_validators[0].plan.nodes.len(), 13);
    assert_eq!(n14.schema_refinement_validators[0].plan.nodes.len(), 17);

    let n10_footprint = crate::classfile::function_code_footprint(&n10, "check");
    let n14_footprint = crate::classfile::function_code_footprint(&n14, "check");
    assert!(
        n14_footprint < n10_footprint * 2,
        "shared graph metadata should grow linearly: N10={n10_footprint}, N14={n14_footprint}"
    );
    generate_classfiles_with_entry(&n14, "main");
}

fn schema_refinement_diamond_ir(depth: usize) -> TypedProgram {
    let mut source =
        String::from("type State\n  Connected\n  Closed\nend\ntype N0<A>\n  Leaf(value: A)\nend\n");
    for level in 1..=depth {
        source.push_str(&format!(
            "type N{level}<A>\n  Left(value: N{}<A>)\n  Right(value: N{}<A>)\nend\n",
            level - 1,
            level - 1,
        ));
    }
    source.push_str(&format!(
        "schema Packet\n  item: N{depth}<State::Connected>\nend\nfn check(value: {{item: N{depth}<State>}}, base: ByteOffset) -> Result<{{item: N{depth}<State::Connected>}}, DecodeError>\n  validate Packet from value at base\nend\npub fn main() -> ()\n  ()\nend\n"
    ));
    lower_to_ir(&source)
}

#[test]
fn bytecode_backend_executes_signed_integer_remainder_endpoints() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(7 % 3))\n",
        "  stdio::println(int_to_string(-7 % 3))\n",
        "  stdio::println(int_to_string(7 % -3))\n",
        "  stdio::println(int_to_string(-7 % -3))\n",
        "  stdio::println(int_to_string((-9223372036854775807 - 1) % -1))\n",
        "  stdio::println(int_to_string((-9223372036854775807 - 1) % 3))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-remainder", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "1\n-1\n1\n-1\n0\n-2\n"
    );
}

#[test]
fn bytecode_backend_reports_dynamic_zero_remainder_without_host_exception() {
    let ir = lower_to_ir(concat!(
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(\"before\")\n",
        "  let divisor = 1 - 1\n",
        "  stdio::println(int_to_string(7 % divisor))\n",
        "  stdio::println(\"after\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-remainder-zero", &program, &[])
    else {
        return;
    };

    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "before\n");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error[runtime.remainder_by_zero]: remainder divisor is zero"),
        "{stderr}"
    );
    assert!(stderr.contains("note: main.veln"), "{stderr}");
    assert!(!stderr.contains("ArithmeticException"), "{stderr}");
}

#[test]
fn bytecode_backend_remainder_matches_independent_i128_model() {
    let pairs = [
        (i64::MIN, -1),
        (i64::MIN, 3),
        (i64::MIN, i64::MAX),
        (i64::MIN, i64::MIN),
        (i64::MAX, 3),
        (1, i64::MIN),
        (-91, 10),
        (91, -10),
        (-91, -10),
        (0, 17),
    ];
    let mut source = "pub fn main() -> () effects [stdio]\n".to_string();
    let mut expected = String::new();
    let source_int = |value: i64| {
        if value == i64::MIN {
            "(-9223372036854775807 - 1)".to_string()
        } else {
            value.to_string()
        }
    };
    for (left, right) in pairs {
        let left_source = source_int(left);
        let right_source = source_int(right);
        source.push_str(&format!(
            "  stdio::println(int_to_string({left_source} % {right_source}))\n"
        ));
        let quotient = (left as i128) / (right as i128);
        let remainder = (left as i128) - quotient * (right as i128);
        expected.push_str(&format!("{remainder}\n"));
    }
    source.push_str("end\n");
    let ir = lower_to_ir(&source);
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-remainder-i128-model", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
}

#[test]
fn bytecode_backend_evaluates_remainder_operands_once_left_to_right() {
    let ir = lower_to_ir(concat!(
        "fn left() -> Int effects [stdio]\n",
        "  stdio::println(\"left\")\n",
        "  7\n",
        "end\n",
        "fn right() -> Int effects [stdio]\n",
        "  stdio::println(\"right\")\n",
        "  3\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(left() % right()))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-remainder-evaluation-order",
        &program,
        &[],
    ) else {
        return;
    };
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "left\nright\n1\n");
}

#[test]
fn bytecode_backend_remainder_failure_stops_later_evaluation_and_runs_cleanup() {
    let ir = lower_to_ir(concat!(
        "fn zero() -> Int effects [stdio]\n",
        "  stdio::println(\"right\")\n",
        "  let value = 0\n",
        "  value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  defer\n",
        "    stdio::println(\"cleanup\")\n",
        "  end\n",
        "  stdio::println(\"left\")\n",
        "  let ignored = 7 % zero()\n",
        "  stdio::println(\"after\")\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-remainder-cleanup", &program, &[])
    else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "left\nright\ncleanup\n"
    );
}

#[test]
fn bytecode_backend_remainder_left_failure_prevents_right_operand() {
    let ir = lower_to_ir(concat!(
        "fn fail_left() -> Int\n",
        "  let zero = 0\n",
        "  1 % zero\n",
        "end\n",
        "fn right() -> Int effects [stdio]\n",
        "  stdio::println(\"unexpected right\")\n",
        "  2\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let ignored = fail_left() % right()\n",
        "  ()\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-remainder-left-failure", &program, &[])
    else {
        return;
    };
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
}

#[test]
fn bytecode_backend_result_propagation_clears_partial_expression_operands() {
    let ir = lower_to_ir(concat!(
        "fn succeed(value: Int) -> Result<Int, String>\n",
        "  Ok(value)\n",
        "end\n",
        "fn fail() -> Result<Int, String>\n",
        "  Err(\"expected failure\")\n",
        "end\n",
        "fn worker() -> Result<(), String>\n",
        "  let first = succeed(succeed(1)? + 2)\n",
        "  let second = succeed(3)? + fail()?\n",
        "  Ok(())\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  match worker()\n",
        "    Ok(_) => stdio::println(\"unexpected success\")\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-result-expression-operands",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "expected failure\n"
    );
}

#[test]
fn bytecode_backend_preserves_distinct_calling_conventions() {
    let ir = lower_to_ir(concat!(
        "fn increment(value: Int) -> Int\n",
        "  value + 1\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let direct = increment(1)\n",
        "  let callable: fn(Int) -> Int = increment\n",
        "  let indirect = callable(direct)\n",
        "  stdio::println(int_to_string(indirect))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-calling-conventions", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\n");
}

#[test]
fn bytecode_backend_passes_indirect_call_location_to_callsite_function_values() {
    let ir = lower_to_ir(concat!(
        "fn located(value: Int) -> {value: Int, location: SourceLocation} callsite\n",
        "  {value: value, location: callsite}\n",
        "end\n",
        "fn forward(value: Int) -> {value: Int, location: SourceLocation} callsite\n",
        "  let stored: fn(Int) -> {value: Int, location: SourceLocation} = located\n",
        "  stored(value)\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let observed = forward(37)\n",
        "  stdio::println(int_to_string(observed.value))\n",
        "  stdio::println(int_to_string(observed.location.start_line))\n",
        "  stdio::println(int_to_string(observed.location.start_column))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-callsite-function-value", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "37\n9\n18\n");
}

#[test]
fn bytecode_backend_prefers_indirect_call_context_over_bound_callback_context() {
    let ir = lower_to_ir(concat!(
        "fn located() -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn invoke(callback: fn() -> SourceLocation) -> SourceLocation\n",
        "  callback()\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let stored: fn() -> SourceLocation = located\n",
        "  let observed = invoke(stored)\n",
        "  stdio::println(int_to_string(observed.start_line))\n",
        "  stdio::println(int_to_string(observed.start_column))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-bound-callsite-function-value",
        &program,
        &[],
    ) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "5\n3\n");
}

#[test]
fn bytecode_backend_resolves_generated_callsite_origins_all_or_nothing() {
    let text = concat!(
        "fn capture() -> SourceLocation callsite\n",
        "  callsite\n",
        "end\n",
        "fn print_location(location: SourceLocation) -> () effects [stdio]\n",
        "  stdio::println(location.file)\n",
        "  stdio::println(int_to_string(location.start_line))\n",
        "  stdio::println(int_to_string(location.start_column))\n",
        "  stdio::println(int_to_string(location.start_offset))\n",
        "  stdio::println(int_to_string(location.end_line))\n",
        "  stdio::println(int_to_string(location.end_column))\n",
        "  stdio::println(int_to_string(location.end_offset))\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let direct = capture()\n",
        "  let stored: fn() -> SourceLocation = capture\n",
        "  let indirect = stored()\n",
        "  print_location(direct)\n",
        "  print_location(indirect)\n",
        "end\n",
    );
    let direct_start = text
        .find("capture()\n  let stored")
        .expect("direct call start");
    let direct_end = direct_start + "capture()".len();
    let indirect_start = text
        .find("stored()\n  print_location")
        .expect("indirect call start");
    let indirect_end = indirect_start + "stored()".len();
    let original_direct = veln_source::SourceSpan {
        file: "templates/original.veln".into(),
        start: veln_source::LineCol {
            line: 41,
            column: 7,
            offset: 1_001,
        },
        end: veln_source::LineCol {
            line: 43,
            column: 19,
            offset: 1_099,
        },
        generated_origin: None,
    };
    let original_indirect = veln_source::SourceSpan {
        file: "templates/original.veln".into(),
        start: veln_source::LineCol {
            line: 47,
            column: 11,
            offset: 1_201,
        },
        end: veln_source::LineCol {
            line: 49,
            column: 23,
            offset: 1_299,
        },
        generated_origin: None,
    };
    let mapped_boundaries = [
        (direct_start, original_direct.start),
        (direct_end, original_direct.end),
        (indirect_start, original_indirect.start),
        (indirect_end, original_indirect.end),
    ];
    let cases = [
        (
            "mapped",
            SourceFile::generated_with_mappings(
                "virtual/mapped.veln",
                text,
                "templates/original.veln".into(),
                mapped_boundaries,
            ),
            [original_direct.clone(), original_indirect.clone()],
        ),
        (
            "path-only",
            SourceFile::generated(
                "virtual/path-only.veln",
                text,
                Some("templates/original.veln".into()),
            ),
            generated_call_spans(
                "virtual/path-only.veln",
                text,
                [(direct_start, direct_end), (indirect_start, indirect_end)],
            ),
        ),
        (
            "partial",
            SourceFile::generated_with_mappings(
                "virtual/partial.veln",
                text,
                "templates/original.veln".into(),
                [
                    (direct_start, original_direct.start),
                    (indirect_end, original_indirect.end),
                ],
            ),
            generated_call_spans(
                "virtual/partial.veln",
                text,
                [(direct_start, direct_end), (indirect_start, indirect_end)],
            ),
        ),
        (
            "unmapped",
            SourceFile::generated(
                "virtual/unmapped.veln",
                text,
                None::<veln_source::SourcePath>,
            ),
            generated_call_spans(
                "virtual/unmapped.veln",
                text,
                [(direct_start, direct_end), (indirect_start, indirect_end)],
            ),
        ),
    ];

    for (name, source, expected_spans) in cases {
        let ir = lower_source_to_ir(&source);
        let program = generate_classfiles_with_entry(&ir, "main");
        let Some(output) = run_jvm_program_when_java_is_available(
            &format!("callsite-origin-{name}"),
            &program,
            &[],
        ) else {
            return;
        };
        assert!(
            output.status.success(),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout),
            expected_location_output(&expected_spans),
            "{name}"
        );
    }
}

fn generated_call_spans(
    path: &str,
    text: &str,
    ranges: [(usize, usize); 2],
) -> [veln_source::SourceSpan; 2] {
    let source = SourceFile::new(path, text);
    ranges.map(|(start, end)| source.span(TextRange::new(start, end)))
}

fn expected_location_output(spans: &[veln_source::SourceSpan; 2]) -> String {
    let mut output = String::new();
    for span in spans {
        for value in [
            span.file.as_str().to_string(),
            span.start.line.to_string(),
            span.start.column.to_string(),
            span.start.offset.to_string(),
            span.end.line.to_string(),
            span.end.column.to_string(),
            span.end.offset.to_string(),
        ] {
            output.push_str(&value);
            output.push('\n');
        }
    }
    output
}

#[test]
fn bytecode_backend_schema_encode_step_calls_match_runtime_metadata_contract() {
    for budgeted in [false, true] {
        let mut ir = lower_to_ir(concat!(
            "schema PacketWire\n",
            "  format binary\n",
            "\n",
            "  value: UInt8\n",
            "end\n",
            "\n",
            "fn encode_budget() -> ByteCount\n",
            "  match byte_count(1)\n",
            "    Ok(value) => value\n",
            "    Err(_) => encode_budget()\n",
            "  end\n",
            "end\n",
            "\n",
            "pub fn main() -> Result<ByteChunk, EncodeError>\n",
            "  encode PacketWire from {value: 7}\n",
            "end\n",
        ));
        let budget_type = ir
            .functions
            .iter()
            .find(|function| function.name == "encode_budget")
            .map(|function| function.return_type.clone())
            .expect("budget helper should be lowered");
        let function = ir
            .functions
            .iter_mut()
            .find(|function| function.name == "main")
            .expect("main should be lowered");
        let value = function
            .body
            .iter_mut()
            .find_map(|statement| match &mut statement.kind {
                IrStmtKind::Return { value } => Some(value),
                _ => None,
            })
            .expect("main should return the schema encode call");
        let IrExprKind::Call { target, args } = &mut value.kind else {
            panic!("main should return a call");
        };
        assert!(matches!(target, IrCallTarget::SchemaEncode(name) if name == "PacketWire"));
        *target = IrCallTarget::SchemaEncodeStep("PacketWire".to_string());
        if budgeted {
            let node_id = value.node_id;
            let span = value.span.clone();
            args.push(IrExpr {
                node_id,
                ty: budget_type,
                kind: IrExprKind::Call {
                    target: IrCallTarget::Function("encode_budget".to_string()),
                    args: Vec::new(),
                },
                span,
            });
        }

        let program = generate_classfiles_with_entry(&ir, "main");
        let case = if budgeted {
            "schema-encode-step-budgeted"
        } else {
            "schema-encode-step"
        };
        let Some(output) = run_jvm_program_when_java_is_available(case, &program, &[]) else {
            return;
        };

        assert!(
            output.status.success(),
            "{case}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(String::from_utf8_lossy(&output.stdout), "", "{case}");
    }
}

#[test]
fn bytecode_backend_reports_entry_result_failures_when_java_is_available() {
    let cases = [
        (
            "result",
            "pub fn main() -> Result<(), String>\n  Err(\"entry failed\")\nend\n",
            "Err(entry failed)",
        ),
        (
            "encode-invalid",
            concat!(
                "pub fn main() -> EncodeStep<String>\n",
                "  let error: EncodeError = EncodeError(\"codec.out_of_range\", \"entry.value\", \"too large\")\n",
                "  EncodeStep::Invalid(error)\n",
                "end\n",
            ),
            "codec.out_of_range",
        ),
        (
            "decode-invalid",
            concat!(
                "pub fn main() -> DecodeStep<Int>\n",
                "  match byte_offset(9)\n",
                "    Ok(offset) => invalid_decode(offset)\n",
                "    Err(_) => incomplete_decode()\n",
                "  end\n",
                "end\n",
                "fn invalid_decode(offset: ByteOffset) -> DecodeStep<Int>\n",
                "  let error: DecodeError = DecodeErrorWithReason(\"codec.length_mismatch\", offset, \"entry.value\", \"wrong length\")\n",
                "  DecodeStep::Invalid(error)\n",
                "end\n",
                "fn incomplete_decode() -> DecodeStep<Int>\n",
                "  let readiness: DecodeReadiness = NeedEnd\n",
                "  DecodeStep::NeedMore(readiness)\n",
                "end\n",
            ),
            "codec.length_mismatch",
        ),
        (
            "decode-need-more",
            concat!(
                "pub fn main() -> DecodeStep<Int>\n",
                "  let readiness: DecodeReadiness = NeedEnd\n",
                "  DecodeStep::NeedMore(readiness)\n",
                "end\n",
            ),
            "NeedMore(NeedEnd)",
        ),
    ];

    for (name, source, expected_error) in cases {
        let ir = lower_to_ir(source);
        let program = generate_classfiles_with_entry(&ir, "main");
        let Some(output) = run_jvm_program_when_java_is_available(name, &program, &[]) else {
            return;
        };

        assert_eq!(output.status.code(), Some(1), "{name}");
        assert_eq!(String::from_utf8_lossy(&output.stdout), "", "{name}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(expected_error),
            "{name}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn bytecode_backend_runs_lexical_handlers_when_java_is_available() {
    let ir = lower_to_ir(
        "effect Pick\n\
           next(value: Int) -> Int\n\
         end\n\
         fn provide(base: Int, value: Int) -> Int\n\
           base + value\n\
         end\n\
         handler picker(base: Int) for Pick\n\
           next(step) => provide(base, step)\n\
         end\n\
         pub fn main() -> () effects [stdio]\n\
           let total = handle perform Pick::next(1) + perform Pick::next(2) with picker(40)\n\
           stdio::println(int_to_string(total))\n\
         end\n",
    );
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-lexical-handler", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "83\n");
}

#[test]
fn bytecode_backend_dispatches_reusable_test_entries_when_java_is_available() {
    let ir = lower_to_ir(
        "test alpha() -> Result<(), String> effects [stdio]\n\
           stdio::println(\"alpha\")\n\
           Ok(())\n\
         end\n\
         test beta() -> Result<(), String> effects [stdio]\n\
           stdio::println(\"beta\")\n\
           Ok(())\n\
         end\n",
    );
    let program =
        generate_classfiles_with_test_entries(&ir, &["alpha".to_string(), "beta".to_string()]);

    let Some(alpha) =
        run_jvm_program_when_java_is_available("bytecode-test-alpha", &program, &["alpha"])
    else {
        return;
    };
    let beta = run_jvm_program_when_java_is_available("bytecode-test-beta", &program, &["beta"])
        .expect("the same Java runtime should remain available");
    let all =
        run_jvm_program_when_java_is_available("bytecode-test-all", &program, &["alpha", "beta"])
            .expect("the same Java runtime should remain available");

    assert!(
        alpha.status.success(),
        "alpha stdout={} stderr={}",
        String::from_utf8_lossy(&alpha.stdout),
        String::from_utf8_lossy(&alpha.stderr)
    );
    assert!(
        beta.status.success(),
        "beta stdout={} stderr={}",
        String::from_utf8_lossy(&beta.stdout),
        String::from_utf8_lossy(&beta.stderr)
    );
    assert!(
        all.status.success(),
        "all stdout={} stderr={}",
        String::from_utf8_lossy(&all.stdout),
        String::from_utf8_lossy(&all.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&alpha.stdout), "alpha\n");
    assert_eq!(String::from_utf8_lossy(&beta.stdout), "beta\n");
    assert_eq!(String::from_utf8_lossy(&all.stdout), "alpha\nbeta\n");
}

#[test]
fn external_socket_client_uses_an_unregistered_host_listener_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err() {
        return;
    }
    let ir = lower_with_standard_modules_to_ir(concat!(
        "use net from \"std\"\n",
        "use stdio\n",
        "fn end_text(outcome: net::ReadOutcome) -> String\n",
        "  match outcome\n",
        "    net::ReadChunk(_) => \"chunk\"\n",
        "    net::ReadEnd => \"end\"\n",
        "  end\n",
        "end\n",
        "fn fixture_bytes(hex: String) -> ByteChunk\n",
        "  match byte_chunk_from_hex(hex)\n",
        "    Ok(bytes) => bytes\n",
        "    Err(_) => byte_chunk([])\n",
        "  end\n",
        "end\n",
        "fn run_client(address: String) -> Result<(), net::NetError> effects [net::IO, stdio]\n",
        "  let parts = net::split_host_port(address)?\n",
        "  let stream = net::connect(net::Address(net::Tcp4, parts.host, parts.port))?\n",
        "  let _ = net::write_all(stream, fixture_bytes(\"010203\"), None, None)?\n",
        "  let _ = net::shutdown_write(stream)?\n",
        "  let response = net::read(stream)?\n",
        "  let count = match response\n",
        "    net::ReadChunk(bytes) => byte_count_to_int(byte_chunk_count(bytes))\n",
        "    net::ReadEnd => 0\n",
        "  end\n",
        "  let ended = end_text(net::read(stream)?)\n",
        "  let _ = net::close_stream(stream)?\n",
        "  stdio::print(int_to_string(count))\n",
        "  stdio::print(\" \" )\n",
        "  stdio::println(ended)\n",
        "  Ok(())\n",
        "end\n",
        "pub fn main(address: String) -> Result<(), net::NetError> effects [net, time, stdio]\n",
        "  handle run_client(address) with net::system()\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let Some(host_listener) = bind_loopback_listener_when_available() else {
        return;
    };
    let address = host_listener
        .local_addr()
        .expect("host listener should have an address")
        .to_string();
    let host = thread::spawn(move || {
        let (mut stream, _) = host_listener.accept().expect("host listener should accept");
        let mut received = Vec::new();
        stream
            .read_to_end(&mut received)
            .expect("host listener should observe the client half-close");
        assert_eq!(received, [0x01, 0x02, 0x03]);
        stream
            .write_all(&[0x0a, 0x0b])
            .expect("host listener should write its response");
        stream
            .shutdown(Shutdown::Write)
            .expect("host listener should half-close its response");
    });

    let output =
        run_jvm_program_when_java_is_available("external-socket-client", &program, &[&address])
            .expect("java availability was checked");

    host.join().expect("host listener should finish");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "2 end\n");
}

#[test]
fn external_socket_listener_accepts_an_unsynthesized_host_client_when_java_is_available() {
    use std::io::{BufRead, BufReader};

    if Command::new("java").arg("-version").output().is_err() {
        return;
    }
    let Some(reservation) = bind_loopback_listener_when_available() else {
        return;
    };
    drop(reservation);

    // Exercise real host acceptance, byte transfer, and half-close through the JVM.
    // Publish the OS-assigned endpoint only after listen succeeds, so host startup
    // does not depend on a retry window or reuse a released port reservation.
    let ir = lower_with_standard_modules_to_ir(concat!(
        "use net from \"std\"\n",
        "use stdio\n",
        "fn fixture_bytes(hex: String) -> ByteChunk\n",
        "  match byte_chunk_from_hex(hex)\n",
        "    Ok(bytes) => bytes\n",
        "    Err(_) => byte_chunk([])\n",
        "  end\n",
        "end\n",
        "fn read_count(stream: net::Stream) -> Result<Int, net::NetError> effects [net::IO]\n",
        "  match net::read(stream)?\n",
        "    net::ReadChunk(chunk) => Ok(byte_count_to_int(byte_chunk_count(chunk)) + read_count(stream)?)\n",
        "    net::ReadEnd => Ok(0)\n",
        "  end\n",
        "end\n",
        "fn run_listener() -> Result<(), net::NetError> effects [net::IO, time, stdio]\n",
        "  let listener = net::listen(net::Address(net::Tcp4, \"127.0.0.1\", 0))?\n",
        "  let endpoint = net::listener_address(listener)?\n",
        "  let address = match endpoint\n",
        "    net::Endpoint(_, address) => address\n",
        "  end\n",
        "  stdio::println(address)\n",
        "  let stream = net::accept(listener)?\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::cancel(token)\n",
        "  let deadline: Deadline = time::deadline_after_ms(100)\n",
        "  let cancelled = match net::read_with(stream, Some(deadline), Some(token))\n",
        "    Err(net::NetError(_, _, _, net::Cancelled, _)) => \"cancelled\"\n",
        "    _ => \"unexpected\"\n",
        "  end\n",
        "  let request_count = read_count(stream)?\n",
        "  let _ = net::write_all(stream, fixture_bytes(\"0a0b\"), None, None)?\n",
        "  let _ = net::shutdown_write(stream)?\n",
        "  let _ = net::close_stream(stream)?\n",
        "  let _ = net::close_listener(listener)?\n",
        "  stdio::print(cancelled)\n",
        "  stdio::print(\" \" )\n",
        "  stdio::println(int_to_string(request_count))\n",
        "  Ok(())\n",
        "end\n",
        "pub fn main() -> Result<(), net::NetError> effects [net, time, stdio]\n",
        "  handle run_listener() with net::system()\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("external-socket-listener");
    write_jvm_program(&root, &program);
    let child = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("java should run");
    let output = wait_for_test_child_with_stdout(
        child,
        "external-socket-listener",
        Duration::from_secs(30),
        |stdout| {
            let mut stdout = BufReader::new(stdout);
            let mut address = String::new();
            stdout
                .read_line(&mut address)
                .expect("listener should publish its endpoint");
            let address = address
                .trim()
                .parse::<std::net::SocketAddr>()
                .expect("listener should publish a socket address");
            let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5))
                .expect("host client should connect to the ready external listener");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("host client reads should be bounded");
            stream
                .set_write_timeout(Some(Duration::from_secs(5)))
                .expect("host client writes should be bounded");
            stream
                .write_all(&[0x01, 0x02, 0x03])
                .expect("host client should write in order");
            stream
                .shutdown(Shutdown::Write)
                .expect("host client should half-close its request");
            let mut response = Vec::new();
            stream
                .read_to_end(&mut response)
                .expect("host client should observe reclaimed stream output");
            assert_eq!(response, [0x0a, 0x0b]);
            let mut bytes = Vec::new();
            stdout
                .read_to_end(&mut bytes)
                .expect("listener output should be readable");
            bytes
        },
    );
    let _ = fs::remove_dir_all(root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cancelled 3\n");
}
