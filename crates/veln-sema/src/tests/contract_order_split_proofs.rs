use super::*;

#[test]
fn contract_predicate_twelve_atom_boolean_formula_is_statically_proven() {
    let fields = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l"];
    let record_type = bool_record_type(&fields);
    let conjunction = fields
        .iter()
        .map(|field| format!("value.{field}"))
        .collect::<Vec<_>>()
        .join(" and ");
    let output_conjunction = fields
        .iter()
        .map(|field| format!("output.{field}"))
        .collect::<Vec<_>>()
        .join(" and ");
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{{record_type}}}) -> output: {{{record_type}}}\n\
             require not ({conjunction}) or ({conjunction})\n\
             ensure not ({output_conjunction}) or ({output_conjunction})\n\
               value\n\
             end\n"
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
fn contract_predicate_thirteen_atom_boolean_formula_is_statically_proven() {
    let fields = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m",
    ];
    let record_type = bool_record_type(&fields);
    let conjunction = fields
        .iter()
        .map(|field| format!("value.{field}"))
        .collect::<Vec<_>>()
        .join(" and ");
    let output_conjunction = fields
        .iter()
        .map(|field| format!("output.{field}"))
        .collect::<Vec<_>>()
        .join(" and ");
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{{record_type}}}) -> output: {{{record_type}}}\n\
             require not ({conjunction}) or ({conjunction})\n\
             ensure not ({output_conjunction}) or ({output_conjunction})\n\
               value\n\
             end\n"
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
fn contract_predicate_fourteen_atom_boolean_formula_requires_runtime_check() {
    let fields = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
    ];
    let record_type = bool_record_type(&fields);
    let conjunction = fields
        .iter()
        .map(|field| format!("value.{field}"))
        .collect::<Vec<_>>()
        .join(" and ");
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{{record_type}}}) -> output: {{{record_type}}}\n\
             require not ({conjunction}) or ({conjunction})\n\
               value\n\
             end\n"
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contracts = &core.functions[0].contracts;
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn contract_predicate_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int) -> output: {ready: Bool}\n",
            "require value.ready or (not value.ready and true)\n",
            "require value.ready or (1 == 1 and not value.ready)\n",
            "ensure output.ready or (not output.ready and 1 < 2)\n",
            "ensure output.ready or (not output.ready and (1 + 1 == 2))\n",
            "require limit < 10 or (limit >= 10 and true)\n",
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
    assert_eq!(contracts.len(), 5);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_conjoined_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int) -> output: {ready: Bool}\n",
            "require (value.ready and true) or (not value.ready and 1 == 1)\n",
            "require (limit < 10 and true) or (limit >= 10 and 1 + 1 == 2)\n",
            "ensure (output.ready and 2 > 1) or (not output.ready and true)\n",
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
fn contract_predicate_complementary_comparison_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require value == limit or value != limit\n",
            "require value < limit or limit <= value\n",
            "ensure output <= limit or output > limit\n",
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
fn contract_predicate_numeric_literal_alias_comparison_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Float) -> output: Float\n",
            "require value == 1 or value != 1.0\n",
            "ensure output == 2.00 or 2 != output\n",
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
fn contract_predicate_order_trichotomy_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require value < limit or value == limit or value > limit\n",
            "ensure output > limit or limit == output or output < limit\n",
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
fn contract_predicate_inclusive_total_order_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require value <= limit or limit <= value\n",
            "require value >= limit or limit >= value\n",
            "ensure output <= limit or output >= limit\n",
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
fn contract_predicate_disequality_strict_order_split_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require not (value != limit) or value < limit or value > limit\n",
            "ensure not (limit != output) or output < limit or limit < output\n",
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
fn contract_predicate_disequality_inclusive_order_split_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require not (value != limit) or value <= limit\n",
            "require not (value != limit) or limit <= value\n",
            "ensure not (output != limit) or output >= limit\n",
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
fn contract_predicate_disequality_split_requires_both_strict_orders() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require not (value != limit) or value < limit\n",
            "ensure not (output != limit) or output < limit\n",
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
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_negated_exclusive_order_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require not (value < limit and value == limit)\n",
            "require not (value < limit and (value > limit))\n",
            "require not (limit > value and value == limit)\n",
            "ensure not((output == limit) and output > limit)\n",
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
    assert_eq!(contracts.len(), 4);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_inclusive_strict_order_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, limit: Int) -> output: Int\n",
            "require not (value <= limit and limit < value)\n",
            "require not (limit >= value and value > limit)\n",
            "ensure not(output <= limit and limit < output)\n",
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
fn contract_predicate_negated_exclusive_numeric_literal_bounds_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn clamp(value: Int) -> output: Int\n",
            "require not (value > 10 and value < 5)\n",
            "require not (value >= 10 and value <= 9)\n",
            "require not (1 + 1 <= value and value < 2)\n",
            "ensure not (output >= 10 and 10 > output)\n",
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
    assert_eq!(contracts.len(), 4);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_covering_numeric_literal_bounds_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn clamp(value: Int) -> output: Int\n",
            "require value <= 10 or value >= 5\n",
            "require value > 2 or value <= 2\n",
            "require 1 + 1 >= value or value >= 2\n",
            "ensure output < 10 or 5 <= output\n",
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
    assert_eq!(contracts.len(), 4);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_exclusive_literal_equalities_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(name: String, flag: Bool) -> output: String\n",
            "require not (name == \"Ada\" and name == \"Grace\")\n",
            "require not (true == flag and flag == false)\n",
            "ensure not (output == \"ok\" and \"err\" == output)\n",
            "  name\n",
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
fn contract_predicate_matching_literal_equalities_require_runtime_check() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(name: String) -> String\n",
            "require not (name == \"Ada\" and name == \"Ada\")\n",
            "  name\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    assert!(parsed.diagnostics.is_empty(), "{:?}", parsed.diagnostics);
    let module = lower_surface_ast(&parsed.tree);

    let lowered = lower_checked_surface_module(&module);

    assert!(lowered.diagnostics.is_empty(), "{:#?}", lowered.diagnostics);
    let core = lowered.core.expect("valid module should lower to core");
    let contract = &core.functions[0].contracts[0];
    assert_eq!(
        contract.obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn contract_predicate_overlapping_numeric_literal_bounds_require_runtime_check() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int\n",
            "require not (value > 5 and value < 10)\n",
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
    let contract = &core.functions[0].contracts[0];
    assert!(contract.obligation_status == ContractObligationStatus::RuntimeRequired);
}

#[test]
fn contract_predicate_transitive_order_implication_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int\n",
            "require not (low <= mid and mid < high) or low < high\n",
            "require not (high >= mid and mid >= low) or low <= high\n",
            "ensure not (output <= mid and mid < high) or output < high\n",
            "  low\n",
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
fn contract_predicate_disjunctive_transitive_order_implication_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, left: Int, right: Int, high: Int) -> Int\n",
            "require not ((low <= left and left < high) or (low < right and right <= high)) or low < high\n",
            "require not ((low == left and left <= high) or (low <= right and right == high)) or low <= high\n",
            "  low\n",
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
