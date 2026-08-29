use super::*;

#[test]
fn lowers_qualified_prelude_builtin_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  prelude_builtin::vec_len(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
}

#[test]
fn http2_private_intrinsics_are_not_bare_prelude_helpers() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(view: ByteView) -> Result<{ length : Int, kind : Int, flags : Int, stream_id : Int, payload : ByteView }, String>\n",
            "  byte_decode_http2_frame(view)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "name.unresolved"
            && diagnostic.message == "unresolved call_target `byte_decode_http2_frame`"
    }));
}

#[test]
fn lowers_qualified_standard_prelude_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  prelude::vec_len(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Call {
            target: CoreCallTarget::PreludeBuiltin(name),
            ..
        } if name == "vec_len"
    ));
}

#[test]
fn stream_input_constructors_resolve_through_standard_prelude_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn bare(chunk: ByteChunk) -> StreamInput\n",
            "  Chunk(chunk)\n",
            "end\n",
            "fn type_qualified(chunk: ByteChunk) -> StreamInput\n",
            "  StreamInput::Chunk(chunk)\n",
            "end\n",
            "fn prelude_qualified(chunk: ByteChunk) -> StreamInput\n",
            "  prelude::Chunk(chunk)\n",
            "end\n",
            "fn prelude_type_qualified(chunk: ByteChunk) -> StreamInput\n",
            "  prelude::StreamInput::Chunk(chunk)\n",
            "end\n",
            "fn done() -> StreamInput\n",
            "  prelude::End\n",
            "end\n",
            "fn decoded(count: ByteCount) -> DecodeStep<Int>\n",
            "  Decoded(7, count)\n",
            "end\n",
            "fn waiting(count: ByteCount) -> DecodeStep<Int>\n",
            "  prelude::DecodeStep::NeedMore(prelude::DecodeReadiness::NeedBytes(count))\n",
            "end\n",
            "fn waiting_for_end() -> DecodeStep<Int>\n",
            "  DecodeStep::NeedMore(NeedEnd)\n",
            "end\n",
            "fn invalid(offset: ByteOffset) -> DecodeStep<Int>\n",
            "  prelude::Invalid(DecodeError(\"codec.invalid\", offset, \"demo.field\"))\n",
            "end\n",
            "fn encoded(chunks: List<ByteChunk>) -> EncodeStep<String>\n",
            "  Encoded(chunks)\n",
            "end\n",
            "fn partial(chunks: List<ByteChunk>, count: ByteCount) -> EncodeStep<String>\n",
            "  prelude::EncodeStep::Partial(chunks, count, \"waiting\")\n",
            "end\n",
            "fn invalid_encode() -> EncodeStep<String>\n",
            "  EncodeStep::Invalid(EncodeError(\"codec.out_of_range\", \"demo.length\", \"too large\"))\n",
            "end\n",
            "fn label(input: StreamInput) -> String\n",
            "  match input\n",
            "    prelude::StreamInput::Chunk(bytes) => int_to_string(byte_count_to_int(byte_chunk_count(bytes)))\n",
            "    prelude::End => \"end\"\n",
            "  end\n",
            "end\n",
            "fn decode_label(step: DecodeStep<Int>) -> String\n",
            "  match step\n",
            "    prelude::DecodeStep::Decoded(value, consumed) => int_to_string(value + byte_count_to_int(consumed))\n",
            "    NeedMore(prelude::DecodeReadiness::NeedBytes(count)) => int_to_string(byte_count_to_int(count))\n",
            "    NeedMore(prelude::NeedEnd) => \"end\"\n",
            "    prelude::DecodeStep::Invalid(DecodeError(id, _, _)) => id\n",
            "    prelude::DecodeStep::Invalid(DecodeErrorWithReason(id, _, _, _)) => id\n",
            "  end\n",
            "end\n",
            "fn encode_label(step: EncodeStep<String>) -> String\n",
            "  match step\n",
            "    prelude::EncodeStep::Encoded(chunks) => int_to_string(list_fold(chunks, 0, count_chunk))\n",
            "    Partial(_, _, state) => state\n",
            "    prelude::EncodeStep::Invalid(EncodeError(id, _, _)) => id\n",
            "  end\n",
            "end\n",
            "fn count_chunk(total: Int, chunk: ByteChunk) -> Int\n",
            "  total + byte_count_to_int(byte_chunk_count(chunk))\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for function_name in [
        "bare",
        "type_qualified",
        "prelude_qualified",
        "prelude_type_qualified",
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert_eq!(expr.ty, CoreType::named("StreamInput", Vec::new()));
        assert!(
            matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
                if name == &vec!["StreamInput".to_string(), "Chunk".to_string()]
                    && payloads.len() == 1),
            "{function_name} should lower to StreamInput::Chunk"
        );
    }
    let done = core
        .functions
        .iter()
        .find(|function| function.name == "done")
        .expect("done should be lowered");
    let CoreStmtKind::Return { expr } = &done.body[0].kind else {
        panic!("done should return a constructor");
    };
    assert_eq!(expr.ty, CoreType::named("StreamInput", Vec::new()));
    assert!(
        matches!(&expr.kind, CoreExprKind::AdtVariant { name, payloads }
            if name == &vec!["StreamInput".to_string(), "End".to_string()]
                && payloads.is_empty())
    );
    for function_name in ["decoded", "waiting", "waiting_for_end", "invalid"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert_eq!(
            expr.ty,
            CoreType::named("DecodeStep", vec![CoreType::int()])
        );
    }
    for function_name in ["encoded", "partial", "invalid_encode"] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .unwrap_or_else(|| panic!("{function_name} should be lowered"));
        let CoreStmtKind::Return { expr } = &function.body[0].kind else {
            panic!("{function_name} should return a constructor");
        };
        assert_eq!(
            expr.ty,
            CoreType::named("EncodeStep", vec![CoreType::string()])
        );
    }
    let label = core
        .functions
        .iter()
        .find(|function| function.name == "label")
        .expect("label should be lowered");
    let CoreStmtKind::Return { expr } = &label.body[0].kind else {
        panic!("label should return a match");
    };
    let CoreExprKind::Match { arms, .. } = &expr.kind else {
        panic!("label should lower to a match");
    };
    assert!(
        matches!(&arms[0].pattern.kind, CorePatternKind::Constructor { name, args }
            if name == &vec!["StreamInput".to_string(), "Chunk".to_string()]
                && args.len() == 1)
    );
    assert!(
        matches!(&arms[1].pattern.kind, CorePatternKind::Constructor { name, args }
            if name == &vec!["StreamInput".to_string(), "End".to_string()]
                && args.is_empty())
    );
}

#[test]
fn standard_package_sources_are_embedded_and_checkable() {
    for source in veln_stdlib::package_bundle().files {
        let module_name = source
            .path
            .strip_suffix(".veln")
            .expect("standard source extension")
            .replace('/', "::");
        let text = format!("mod std::{module_name}\n{}", source.text);
        let file = SourceFile::new(source.path, text);
        let parsed = parse(&file);
        assert!(
            parsed.diagnostics.is_empty(),
            "unexpected parse diagnostics for {}: {:#?}",
            source.path,
            parsed.diagnostics
        );

        let module = lower_surface_ast(&parsed.tree);
        if module.uses.is_empty() {
            let diagnostics = analyze_surface_module(&module);
            assert!(
                diagnostics.is_empty(),
                "unexpected source helper diagnostics for {}: {diagnostics:#?}",
                source.path
            );
        }
        assert!(
            !module.types.is_empty()
                || !module.effects.is_empty()
                || !module.schemas.is_empty()
                || !module.codecs.is_empty()
                || !module.handlers.is_empty()
                || !module.functions.is_empty(),
            "embedded source should define a checkable declaration"
        );
    }
}

#[test]
fn imported_public_function_conflicts_with_implicit_prelude_bare_call() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.measure\n",
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  vec_len(items)\n",
            "end\n",
        ),
    );
    let measure_source = SourceFile::new(
        "measure.veln",
        concat!(
            "mod app.measure\n",
            "pub fn vec_len(items: Vec<Int>) -> Int\n",
            "  0\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let measure = lower_surface_ast(&parse(&measure_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        codecs: Vec::new(),
        types: Vec::new(),
        functions: main
            .functions
            .into_iter()
            .chain(measure.functions)
            .collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic.id == "name.ambiguous"
                && diagnostic.message == "ambiguous call_target `vec_len`"
        })
        .expect("prelude conflict should be ambiguous");
    let related = diagnostic
        .related
        .iter()
        .map(|note| note.to_json())
        .collect::<Vec<_>>();
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `measure::vec_len` to select it"))
    );
    assert!(
        related
            .iter()
            .any(|note| note.contains("use `prelude::vec_len` to select it"))
    );
}

#[test]
fn local_declaration_shadows_implicit_prelude_import() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn vec_len(items: String) -> Int\n",
            "  7\n",
            "end\n",
            "pub fn main() -> Int\n",
            "  vec_len(\"local\")\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    let main = core
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be lowered");
    let CoreStmtKind::Return { expr } = &main.body[0].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, .. } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(target, &CoreCallTarget::Function("vec_len".to_string()));
}

#[test]
fn non_callable_local_shadow_blocks_implicit_prelude_call_resolution() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn main(items: Vec<Int>) -> Int\n",
            "  let vec_len: Int = 1\n",
            "  vec_len(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.unresolved"
                && diagnostic.message == "unresolved call_target `vec_len`"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn user_source_cannot_claim_prelude_module_alias() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.prelude\n",
            "pub fn main() -> Int\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.reserved"
                && diagnostic.message
                    == "import alias `prelude` conflicts with the standard prelude"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn user_source_cannot_claim_prelude_module_identity() {
    let source = SourceFile::new(
        "main.veln",
        concat!("mod prelude\n", "pub fn main() -> Int\n", "  1\n", "end\n",),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(
        diagnostics.iter().any(|diagnostic| {
            diagnostic.id == "name.reserved"
                && diagnostic.message
                    == "module identity `prelude` conflicts with the standard prelude"
        }),
        "{diagnostics:#?}"
    );
}

#[test]
fn compiler_support_source_loads_text_through_standard_fs_subset() {
    let source = veln_stdlib::package_bundle()
        .files
        .iter()
        .find(|source| source.path == "compiler_support.veln")
        .expect("compiler support source should be embedded");
    let file = SourceFile::new(source.path, source.text);
    let parsed = parse(&file);
    assert!(
        parsed.diagnostics.is_empty(),
        "unexpected parse diagnostics for {}: {:#?}",
        source.path,
        parsed.diagnostics
    );

    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);

    assert!(
        lowered.diagnostics.is_empty(),
        "unexpected compiler support diagnostics for {}: {:#?}",
        source.path,
        lowered.diagnostics
    );
    let core = lowered.core.expect("compiler support should lower to core");
    let function = core
        .functions
        .iter()
        .find(|function| function.name == "load_source_text")
        .expect("compiler support entry should lower");
    let CoreStmtKind::Let { expr, .. } = &function.body[0].kind else {
        panic!("first statement should call fs before wrapping the result");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::Try(value) if matches!(
            &value.kind,
            CoreExprKind::Call {
                target: CoreCallTarget::StandardLibraryBuiltin(name),
                ..
            } if name == "fs::read_to_string"
        )
    ));
}

#[test]
fn suggests_vec_try_map_for_result_returning_map_callback() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn parse(value: Int) -> Result<String, AppError>\n",
            "  Ok(\"ok\")\n",
            "end\n",
            "pub fn main(items: Vec<Int>) -> Vec<String>\n",
            "  vec_map(items, parse)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    let diagnostic = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == "type.mismatch")
        .expect("callback type mismatch should be reported");
    assert_eq!(
        diagnostic.message,
        "expected `fn(Int) -> String`, but found `fn(Int) -> Result<String, AppError>`"
    );
    assert!(
        diagnostic
            .related
            .iter()
            .any(|related| { related.to_json().contains("Use `vec_try_map`") })
    );
}
