use super::*;

#[test]
fn call_resolution_prefers_local_callable_over_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Token\n",
            "  pack(Int)\n",
            "end\n",
            "fn pack_value(value: Int) -> Int\n",
            "  value + 10\n",
            "end\n",
            "pub fn parameter_shadow(pack: fn(Int) -> Int) -> Int\n",
            "  pack(1)\n",
            "end\n",
            "pub fn local_shadow() -> Int\n",
            "  let pack = pack_value\n",
            "  pack(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for (function_name, expected_arg) in [("parameter_shadow", "1"), ("local_shadow", "2")] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("shadow function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[function.body.len() - 1].kind else {
            panic!("tail expression should lower as return");
        };
        let CoreExprKind::Call { target, args } = &expr.kind else {
            panic!("tail expression should lower as call");
        };
        assert_eq!(target, &CoreCallTarget::Value("pack".to_string()));
        assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == expected_arg));
    }

    let ir = lowered.ir.expect("complete core should lower to IR");
    for function_name in ["parameter_shadow", "local_shadow"] {
        let function = ir
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("shadow function should be in IR");
        let IrStmtKind::Return { value } = &function.body[function.body.len() - 1].kind else {
            panic!("tail expression should lower as IR return");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::Call {
                target: IrCallTarget::Value(name),
                ..
            } if name == "pack"
        ));
    }
}

#[test]
fn call_resolution_prefers_if_inferred_callable_over_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Token\n",
            "  pack(Int)\n",
            "end\n",
            "fn direct(value: Int) -> Int\n",
            "  value + 10\n",
            "end\n",
            "fn backup(value: Int) -> Int\n",
            "  value + 20\n",
            "end\n",
            "pub fn main(flag: Bool, other: Bool) -> Int\n",
            "  let pack = if flag\n",
            "    direct\n",
            "  else if other\n",
            "    backup\n",
            "  else\n",
            "    direct\n",
            "  end\n",
            "  pack(1)\n",
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
    let CoreStmtKind::Return { expr } = &main.body[main.body.len() - 1].kind else {
        panic!("tail expression should lower as return");
    };
    let CoreExprKind::Call { target, args } = &expr.kind else {
        panic!("tail expression should lower as call");
    };
    assert_eq!(target, &CoreCallTarget::Value("pack".to_string()));
    assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == "1"));

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[main.body.len() - 1].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::Call {
            target: IrCallTarget::Value(name),
            ..
        } if name == "pack"
    ));
}

#[test]
fn call_resolution_preserves_constructor_when_local_binding_is_not_callable() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Token\n",
            "  pack(Int)\n",
            "end\n",
            "pub fn main(pack: Int) -> Token\n",
            "  pack(1)\n",
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
        CoreExprKind::AdtVariant { name, .. } if name == &vec![
            "Token".to_string(),
            "pack".to_string()
        ]
    ));

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Return { value } = &main.body[0].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::AdtVariant { name, .. } if name == &vec![
            "Token".to_string(),
            "pack".to_string()
        ]
    ));
}

#[test]
fn call_resolution_preserves_constructor_when_binding_record_contains_callable_field() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Token\n",
            "  pack(Int)\n",
            "end\n",
            "fn stringify(value: Int) -> String\n",
            "  \"ok\"\n",
            "end\n",
            "pub fn parameter_record(pack: {callback: fn(Int) -> String}) -> Token\n",
            "  pack(1)\n",
            "end\n",
            "pub fn local_record() -> Token\n",
            "  let pack: {callback: fn(Int) -> String} = {callback: stringify}\n",
            "  pack(2)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for (function_name, expected_arg) in [("parameter_record", "1"), ("local_record", "2")] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("record function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[function.body.len() - 1].kind else {
            panic!("tail expression should lower as return");
        };
        let CoreExprKind::AdtVariant { name, payloads } = &expr.kind else {
            panic!("tail expression should lower as constructor");
        };
        assert_eq!(name, &vec!["Token".to_string(), "pack".to_string()]);
        assert!(
            matches!(&payloads[0].kind, CoreExprKind::IntLiteral(value) if value == expected_arg)
        );
    }

    let ir = lowered.ir.expect("complete core should lower to IR");
    for function_name in ["parameter_record", "local_record"] {
        let function = ir
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("record function should be in IR");
        let IrStmtKind::Return { value } = &function.body[function.body.len() - 1].kind else {
            panic!("tail expression should lower as IR return");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::AdtVariant { name, .. } if name == &vec![
                "Token".to_string(),
                "pack".to_string()
            ]
        ));
    }
}

#[test]
fn call_resolution_prefers_callable_record_field_binding_over_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Token\n",
            "  pack(Int)\n",
            "end\n",
            "fn direct(value: Int) -> Int\n",
            "  value + 10\n",
            "end\n",
            "pub fn parameter_field(record: {pack: fn(Int) -> Int}) -> Int\n",
            "  let pack = record.pack\n",
            "  pack(1)\n",
            "end\n",
            "pub fn local_field() -> Int\n",
            "  let record: {pack: fn(Int) -> Int} = {pack: direct}\n",
            "  let alias = record\n",
            "  let pack = alias.pack\n",
            "  pack(2)\n",
            "end\n",
            "fn make_record() -> {pack: fn(Int) -> Int}\n",
            "  {pack: direct}\n",
            "end\n",
            "pub fn returned_field() -> Int\n",
            "  let record = make_record()\n",
            "  let pack = record.pack\n",
            "  pack(3)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    for (function_name, expected_arg) in [
        ("parameter_field", "1"),
        ("local_field", "2"),
        ("returned_field", "3"),
    ] {
        let function = core
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("field shadow function should be lowered");
        let CoreStmtKind::Return { expr } = &function.body[function.body.len() - 1].kind else {
            panic!("tail expression should lower as return");
        };
        let CoreExprKind::Call { target, args } = &expr.kind else {
            panic!("tail expression should lower as call");
        };
        assert_eq!(target, &CoreCallTarget::Value("pack".to_string()));
        assert!(matches!(&args[0].kind, CoreExprKind::IntLiteral(value) if value == expected_arg));
    }

    let ir = lowered.ir.expect("complete core should lower to IR");
    for function_name in ["parameter_field", "local_field", "returned_field"] {
        let function = ir
            .functions
            .iter()
            .find(|function| function.name == function_name)
            .expect("field shadow function should be in IR");
        let IrStmtKind::Return { value } = &function.body[function.body.len() - 1].kind else {
            panic!("tail expression should lower as IR return");
        };
        assert!(matches!(
            &value.kind,
            IrExprKind::Call {
                target: IrCallTarget::Value(name),
                ..
            } if name == "pack"
        ));
    }
}

#[test]
fn call_resolution_preserves_constructor_when_initializer_call_uses_constructor() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "type Maker\n",
            "  make(Int)\n",
            "end\n",
            "type Token\n",
            "  pack(Int)\n",
            "end\n",
            "fn direct(value: Int) -> Int\n",
            "  value\n",
            "end\n",
            "fn make(value: Int) -> fn(Int) -> Int\n",
            "  direct\n",
            "end\n",
            "pub fn main() -> Token\n",
            "  let pack = make(0)\n",
            "  pack(1)\n",
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
    let CoreStmtKind::Let { expr, .. } = &main.body[0].kind else {
        panic!("initializer should lower as let");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::AdtVariant { name, .. } if name.as_slice() == ["Maker", "make"]
    ));
    let CoreStmtKind::Return { expr } = &main.body[1].kind else {
        panic!("tail expression should lower as return");
    };
    assert!(matches!(
        &expr.kind,
        CoreExprKind::AdtVariant { name, .. } if name.as_slice() == ["Token", "pack"]
    ));

    let ir = lowered.ir.expect("complete core should lower to IR");
    let main = ir
        .functions
        .iter()
        .find(|function| function.name == "main")
        .expect("main should be in IR");
    let IrStmtKind::Let { value, .. } = &main.body[0].kind else {
        panic!("initializer should lower as IR let");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::AdtVariant { name, .. } if name.as_slice() == ["Maker", "make"]
    ));
    let IrStmtKind::Return { value } = &main.body[1].kind else {
        panic!("tail expression should lower as IR return");
    };
    assert!(matches!(
        &value.kind,
        IrExprKind::AdtVariant { name, .. } if name.as_slice() == ["Token", "pack"]
    ));
}
