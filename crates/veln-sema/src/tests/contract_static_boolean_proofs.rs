use super::*;

#[test]
fn contract_static_analysis_compares_integer_radices_by_value() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int\n",
            "require 0b1010 == 10 and 0x0A == 10\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(
        core.functions[0].contracts[0].obligation_status,
        ContractObligationStatus::StaticallyProven
    );
}

#[test]
fn float_dependent_contract_proofs_use_scalar_signed_zero_equality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Float) -> Float\n",
            "require 9007199254740993.0 + 0.0 > 9007199254740992.0\n",
            "require not (value >= 9007199254740993.0 and value <= 9007199254740992.0)\n",
            "require 9007199254740993 == 9007199254740992.0\n",
            "require -0.0 == 0.0\n",
            "require -0.0 != 0.0\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert_eq!(
        core.functions[0]
            .contracts
            .iter()
            .map(|contract| contract.obligation_status)
            .collect::<Vec<_>>(),
        vec![
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::StaticallyProven,
            ContractObligationStatus::RuntimeRequired,
        ]
    );
}

#[test]
fn symbol_dependent_float_comparisons_remain_runtime_required() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn forward(value: Float) -> Float\n",
            "  value\n",
            "end\n",
            "fn checked(value: Float, other: Float, record: {inner: Float}) -> output: Float\n",
            "require value == value\n",
            "require value <= value\n",
            "require value >= value\n",
            "require not (value != value)\n",
            "require value < other or value >= other\n",
            "require record.inner == record.inner\n",
            "require forward(value) == forward(value)\n",
            "require not (value < value)\n",
            "require not (value > value)\n",
            "ensure output == output\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");

    assert_eq!(
        core.functions[1]
            .contracts
            .iter()
            .map(|contract| contract.obligation_status)
            .collect::<Vec<_>>(),
        vec![
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::StaticallyProven,
            ContractObligationStatus::StaticallyProven,
            ContractObligationStatus::RuntimeRequired,
        ]
    );
}

#[test]
fn float_comparison_guard_detects_each_operand_source_independently() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn boxed() -> {inner: Float}\n",
            "  {inner: 1.0}\n",
            "end\n",
            "fn scalar() -> Float\n",
            "  1.0\n",
            "end\n",
            "fn nested_call_checked() -> ()\n",
            "require boxed().inner == boxed().inner\n",
            "  ()\n",
            "end\n",
            "fn scalar_call_checked() -> ()\n",
            "require scalar() == scalar()\n",
            "  ()\n",
            "end\n",
            "fn binding_checked(record: {inner: Float}) -> ()\n",
            "require record.inner == record.inner\n",
            "  ()\n",
            "end\n",
            "fn strict_call_checked() -> ()\n",
            "require not (boxed().inner < boxed().inner)\n",
            "require not (boxed().inner > boxed().inner)\n",
            "  ()\n",
            "end\n",
            "fn integer_checked() -> ()\n",
            "require 1 == 1\n",
            "  ()\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");

    let statuses = |function_index: usize| {
        core.functions[function_index]
            .contracts
            .iter()
            .map(|contract| contract.obligation_status)
            .collect::<Vec<_>>()
    };
    assert_eq!(statuses(2), vec![ContractObligationStatus::RuntimeRequired]);
    assert_eq!(statuses(3), vec![ContractObligationStatus::RuntimeRequired]);
    assert_eq!(statuses(4), vec![ContractObligationStatus::RuntimeRequired]);
    assert_eq!(
        statuses(5),
        vec![
            ContractObligationStatus::StaticallyProven,
            ContractObligationStatus::StaticallyProven,
        ]
    );
    assert_eq!(
        statuses(6),
        vec![ContractObligationStatus::StaticallyProven]
    );
}

#[test]
fn dotted_and_plain_static_string_contracts_are_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int\n",
            "require \"a.b\" == \"a.b\"\n",
            "require \"ab\" == \"ab\"\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("checked core should be built");
    assert!(core.functions[0].contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_accepts_qualified_function_value_arguments() {
    let main_source = SourceFile::new(
        "main.veln",
        concat!(
            "mod app.main\n",
            "use app.rules\n",
            "fn accepts(job: fn() -> Bool) -> Bool\n",
            "  job()\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require accepts(rules::ready)\n",
            "  value\n",
            "end\n",
        ),
    );
    let rules_source = SourceFile::new(
        "rules.veln",
        concat!(
            "mod app.rules\n",
            "fn ready() -> Bool\n",
            "  true\n",
            "end\n",
        ),
    );
    let main = lower_surface_ast(&parse(&main_source).tree);
    let rules = lower_surface_ast(&parse(&rules_source).tree);
    let module = SurfaceModule {
        module: main.module,
        uses: main.uses,
        aliases: Vec::new(),
        effects: Vec::new(),
        handlers: Vec::new(),
        schemas: Vec::new(),
        types: main.types.into_iter().chain(rules.types).collect(),
        module_values: Vec::new(),
        functions: main.functions.into_iter().chain(rules.functions).collect(),
        invalid_names: Vec::new(),
    };

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_arithmetic_function_call_arguments() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn positive(value: Int) -> Bool\n",
            "  value > 0\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require positive(value + 1)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_arithmetic_function_call_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn next(value: Int) -> Int\n",
            "  value + 1\n",
            "end\n",
            "pub fn identity(value: Int) -> Int\n",
            "require next(value) + 1 > 0\n",
            "require (next(value) * 2) > 0\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_arithmetic_comparisons() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, ratio: Float) -> Int\n",
            "require value + 1 > 0\n",
            "require ratio + 1.5 > 0.0\n",
            "require not (value * 2 < 0)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_accepts_prelude_helper_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn first(items: Vec<Int>) -> Int\n",
            "require vec_len(items) > 0\n",
            "require not vec_is_empty(items)\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn satisfy_predicate_accepts_prelude_helper_calls() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn fallback(items: Vec<Int>) -> Vec<Int>\n",
            "  _value satisfy candidate => vec_len(candidate) >= vec_len(items)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.unfilled" && diagnostic.kind == DiagnosticKind::Hole
    }));
    assert!(!diagnostics.iter().any(|diagnostic| {
        diagnostic.id == "hole.satisfy_unsupported_construct"
            || diagnostic.id == "hole.satisfy_unresolved_name"
            || diagnostic.id == "hole.satisfy_type_mismatch"
    }));
}

#[test]
fn contract_predicate_accepts_left_string_literal_comparison() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: String) -> String\n",
            "require \"ready(helper)\" == value\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert!(diagnostics.is_empty(), "{diagnostics:#?}");
}

#[test]
fn contract_predicate_boolean_identity_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> output: Int\n",
            "require true or value > 0\n",
            "ensure (output >= value or true) and not false\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_literal_comparisons_use_rounded_float_values() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn constant() -> output: Int\n",
            "require 1 < 2 and \"ready\" != \"pending\"\n",
            "ensure true == true and 0.5 <= 1.50\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert_eq!(
        contracts
            .iter()
            .map(|contract| contract.obligation_status)
            .collect::<Vec<_>>(),
        vec![ContractObligationStatus::StaticallyProven; 2]
    );
}

#[test]
fn contract_predicate_float_literals_are_classified_after_rounding() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn constant() -> output: Int\n",
            "require 9007199254740993.0 == 9007199254740992.0\n",
            "require 9007199254740993.0 != 9007199254740992.0\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert_eq!(
        contracts[0].obligation_status,
        ContractObligationStatus::StaticallyProven
    );
    assert_eq!(
        contracts[1].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn unrelated_float_bindings_do_not_downgrade_exact_contract_proofs() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn float_control(unused: Float) -> result: Int\n",
            "require 1 / 2 > 0\n",
            "ensure true\n",
            "  1\n",
            "end\n",
            "pub fn int_control(unused: Int) -> result: Int\n",
            "require 1 / 2 > 0\n",
            "ensure true\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    assert!(core.functions.iter().all(|function| {
        function.contracts.iter().all(|contract| {
            contract.obligation_status == ContractObligationStatus::StaticallyProven
        })
    }));
}

#[test]
fn long_zero_float_contract_uses_validated_bits_without_runtime_emission() {
    let zero = format!("0.{}", "0".repeat(65_540));
    let source = SourceFile::new(
        "main.veln",
        format!("pub fn constant() -> Int\n  require {zero} == 0.0\n  1\nend\n"),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let lowered = lower_checked_surface_module(&lower_surface_ast(&parsed.tree));
    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    assert_eq!(
        core.functions[0].contracts[0].obligation_status,
        ContractObligationStatus::StaticallyProven
    );
}

#[test]
fn contract_predicate_grouped_string_literals_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: String) -> output: String\n",
            "require (\"ready\") == \"ready\"\n",
            "ensure (\"done\") != (\"pending\")\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_static_boolean_comparisons_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn constant() -> output: Int\n",
            "require (1 < 2) == true\n",
            "require (not false) == true and (1 > 2) == false\n",
            "ensure (output == output) != false\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_boolean_formula_comparisons_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool, paid: Bool}) -> output: {ready: Bool, paid: Bool}\n",
            "require (value.ready and value.paid) == (value.paid and value.ready)\n",
            "require (value.ready or value.paid) == (value.paid or value.ready)\n",
            "ensure (output.ready and not output.ready) != (output.paid or not output.paid)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(
        contracts.iter().all(|contract| {
            contract.obligation_status == ContractObligationStatus::StaticallyProven
        }),
        "{contracts:#?}"
    );
}

#[test]
fn contract_predicate_complementary_boolean_comparisons_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int) -> output: {ready: Bool}\n",
            "require value.ready != not value.ready\n",
            "require (limit == 0) != (limit != 0)\n",
            "ensure (output.ready) != not(output.ready)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_literal_arithmetic_keeps_float_clauses_at_runtime() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn constant() -> output: Int\n",
            "require 1 + 1 == 2 and 3 * 4 >= 12\n",
            "require 0.5 + 2.0 == 2.5\n",
            "require 10 - 4 == 6 and 8 / 4 == 2\n",
            "require 1 / 2 == 0.5\n",
            "require 1 / 3 < 0.34 and 2 / 3 > 0.66\n",
            "ensure 2 * 3 == 6\n",
            "  1\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 6);
    assert_eq!(
        contracts
            .iter()
            .map(|contract| contract.obligation_status)
            .collect::<Vec<_>>(),
        vec![
            ContractObligationStatus::StaticallyProven,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::StaticallyProven,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::RuntimeRequired,
            ContractObligationStatus::StaticallyProven,
        ]
    );
}

#[test]
fn contract_predicate_same_shape_comparisons_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, label: String) -> output: Int\n",
            "require value + 1 == value + 1\n",
            "require not(value < value)\n",
            "ensure label == label and output >= output\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_complementary_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool}\n",
            "require value.ready or not value.ready\n",
            "ensure (output.ready) or not(output.ready)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_boolean_literal_alias_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}) -> output: {ready: Bool}\n",
            "require value.ready == true or not value.ready\n",
            "require false == value.ready or value.ready\n",
            "ensure output.ready != false or output.ready == false\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_multi_branch_complementary_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require value.ready or extra or not value.ready\n",
            "ensure output.ready or extra or not(output.ready)\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_nested_complementary_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require value.ready or (extra or not value.ready)\n",
            "ensure output.ready or (extra or not(output.ready))\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 2);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_disjunction_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool, paid: Bool}, limit: Int) -> output: {ready: Bool, paid: Bool}\n",
            "require not (value.ready or value.paid) or value.ready or value.paid\n",
            "require (not (limit < 10 or value.ready)) or limit < 10 or value.ready\n",
            "ensure (not (output.ready or output.paid)) or output.ready or output.paid\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_conjunction_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int) -> output: {ready: Bool}\n",
            "require value.ready or not (value.ready and limit > 0)\n",
            "require (not (limit < 10 and value.ready)) or limit < 10\n",
            "ensure (not (output.ready and limit >= 10)) or output.ready\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_conjunction_covered_by_complement_disjuncts_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int, extra: Bool) -> output: {ready: Bool}\n",
            "require (value.ready and extra) or not value.ready or not extra\n",
            "require (limit < 10 and value.ready and true) or limit >= 10 or not value.ready\n",
            "ensure (output.ready and extra) or not output.ready or not extra\n",
            "  value\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 3);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}
