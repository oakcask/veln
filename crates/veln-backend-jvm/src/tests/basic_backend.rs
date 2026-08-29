use super::*;

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
                "  EncodeStep::Invalid(EncodeError(\"codec.out_of_range\", \"entry.value\", \"too large\"))\n",
                "end\n",
            ),
            "codec.out_of_range",
        ),
        (
            "decode-invalid",
            concat!(
                "pub fn main() -> DecodeStep<Int>\n",
                "  match byte_offset(9)\n",
                "    Ok(offset) => DecodeStep::Invalid(DecodeErrorWithReason(\"codec.length_mismatch\", offset, \"entry.value\", \"wrong length\"))\n",
                "    Err(_) => DecodeStep::NeedMore(NeedEnd)\n",
                "  end\n",
                "end\n",
            ),
            "codec.length_mismatch",
        ),
        (
            "decode-need-more",
            "pub fn main() -> DecodeStep<Int>\n  DecodeStep::NeedMore(NeedEnd)\nend\n",
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
         handler picker(base: Int) handles Pick\n\
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
    let ir = lower_to_ir(concat!(
        "use stdio\n",
        "fn end_text(chunk: Option<ByteChunk>) -> String\n",
        "  match chunk\n",
        "    Some(_) => \"chunk\"\n",
        "    None => \"end\"\n",
        "  end\n",
        "end\n",
        "pub fn main(address: String) -> Result<(), String> effects [net, stdio]\n",
        "  let stream: NetStream = net::connect(address)\n",
        "  net::write_chunk(stream, byte_chunk_from_hex(\"01\")?)\n",
        "  net::write_chunk(stream, byte_chunk_from_hex(\"0203\")?)\n",
        "  net::shutdown_write(stream)\n",
        "  let response: ByteChunk = net::read_chunk(stream)\n",
        "  let ended: String = end_text(net::read_chunk_or_end(stream))\n",
        "  net::close_stream(stream)\n",
        "  stdio::print(int_to_string(byte_count_to_int(byte_chunk_count(response))))\n",
        "  stdio::print(\" \" )\n",
        "  stdio::println(ended)\n",
        "  Ok(())\n",
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

    let output = run_jvm_program_with_env_when_java_is_available(
        "external-socket-client",
        &program,
        &[("VELN_NET_RUNTIME", "external")],
        &[&address],
    )
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
    if Command::new("java").arg("-version").output().is_err() {
        return;
    }
    let ir = lower_to_ir(concat!(
        "use stdio\n",
        "fn outcome_text(outcome: StreamReadOutcome) -> String\n",
        "  match outcome\n",
        "    ReadChunk(_) => \"chunk\"\n",
        "    ReadEnd => \"end\"\n",
        "    ReadDeadlineExpired => \"deadline\"\n",
        "    ReadCancelled => \"cancelled\"\n",
        "  end\n",
        "end\n",
        "fn read_count(stream: NetStream) -> Int effects [net]\n",
        "  match net::read_chunk_or_end(stream)\n",
        "    Some(chunk) => byte_count_to_int(byte_chunk_count(chunk)) + read_count(stream)\n",
        "    None => 0\n",
        "  end\n",
        "end\n",
        "pub fn main(address: String) -> Result<(), String> effects [net, time, stdio]\n",
        "  let listener: NetListener = net::listen(address)\n",
        "  let stream: NetStream = net::accept(listener)\n",
        "  let token: CancelToken = time::cancel_token()\n",
        "  time::cancel(token)\n",
        "  let deadline: Deadline = time::deadline_after_ms(100)\n",
        "  let cancelled: String = outcome_text(net::read_chunk_until_cancellable(stream, deadline, token))\n",
        "  let request_count: Int = read_count(stream)\n",
        "  net::write_chunk(stream, byte_chunk_from_hex(\"0a\")?)\n",
        "  net::write_chunk(stream, byte_chunk_from_hex(\"0b\")?)\n",
        "  net::shutdown_write(stream)\n",
        "  net::close_stream(stream)\n",
        "  net::close_listener(listener)\n",
        "  stdio::print(cancelled)\n",
        "  stdio::print(\" \" )\n",
        "  stdio::println(int_to_string(request_count))\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let Some(reservation) = bind_loopback_listener_when_available() else {
        return;
    };
    let address = reservation
        .local_addr()
        .expect("port reservation should have an address")
        .to_string();
    drop(reservation);
    let client_address = address.clone();
    let host = thread::spawn(move || {
        let mut stream = (0..100)
            .find_map(|_| match TcpStream::connect(&client_address) {
                Ok(stream) => Some(stream),
                Err(_) => {
                    thread::sleep(Duration::from_millis(10));
                    None
                }
            })
            .expect("host client should connect to the external listener");
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
    });

    let output = run_jvm_program_with_env_when_java_is_available(
        "external-socket-listener",
        &program,
        &[("VELN_NET_RUNTIME", "external")],
        &[&address],
    )
    .expect("java availability was checked");

    host.join().expect("host client should finish");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "cancelled 3\n");
}
