use super::*;

#[test]
fn contract_predicate_literal_bound_implication_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> output: Int effects []\n",
            "require not (value > 10 and value < 20) or value > 5\n",
            "require not (5 <= value and value <= 10) or value <= 10\n",
            "ensure not (output >= 1 + 1 and output < 10) or output >= 2\n",
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
fn contract_predicate_literal_bound_implication_follows_equality_aliases() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, alias: Int, total: Int) -> output: Int effects []\n",
            "require not (value == alias and alias > 10) or value > 5\n",
            "require not (total == alias and alias <= 10) or total < 20\n",
            "ensure not (output == alias and alias >= 2) or output >= 1\n",
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
fn contract_predicate_literal_bound_implication_follows_equality_alias_paths() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, first: Int, second: Int) -> output: Int effects []\n",
            "require not (value == first and first == second and second > 10) or value > 5\n",
            "ensure not (output == first and first == second and second <= 10) or output < 20\n",
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
fn contract_predicate_literal_bound_alias_does_not_change_bound_direction() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, alias: Int) -> output: Int effects []\n",
            "require not (value == alias and alias > 10) or value < 20\n",
            "ensure not (output == alias and alias <= 10) or output >= 1\n",
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
fn contract_predicate_literal_bound_alias_does_not_weaken_strictness() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, alias: Int) -> output: Int effects []\n",
            "require not (value == alias and alias >= 10) or value > 10\n",
            "ensure not (output == alias and alias <= 10) or output < 10\n",
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
fn contract_predicate_literal_bound_implication_uses_alias_in_either_position() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, alias: Int) -> output: Int effects []\n",
            "require not (value == alias and value > 10) or alias >= 10\n",
            "ensure not (alias == output and output < 20) or alias < 25\n",
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
fn contract_predicate_literal_bound_implication_proves_disequality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, alias: Int) -> output: Int effects []\n",
            "require not (value > 10) or value != 10\n",
            "require not (value == alias and alias <= 1 / 2) or value != 0.75\n",
            "ensure not (output == alias and alias < 20) or output != 20\n",
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
fn contract_predicate_literal_bound_implication_keeps_possible_endpoint_disequality_runtime() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int, alias: Int) -> output: Int effects []\n",
            "require not (value >= 10) or value != 10\n",
            "require not (value == alias and alias <= 20) or value != 20\n",
            "ensure not (output == alias and alias > 5) or output != 6\n",
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
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_literal_bound_non_implication_requires_runtime_check() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: Int) -> Int effects []\n",
            "require not (value >= 10 and value < 20) or value > 10\n",
            "require not (value > 10 and value <= 20) or value < 20\n",
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
fn contract_predicate_equality_edges_transitively_imply_order_bounds() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low < mid and mid == high) or low < high\n",
            "require not (low == mid and mid <= high) or low <= high\n",
            "ensure not (output == mid and high >= mid) or output <= high\n",
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
fn contract_predicate_transitive_order_implies_strict_or_equality_disjunction() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low <= mid and mid <= high) or low < high or low == high\n",
            "ensure not (output == mid and mid <= high) or output < high or high == output\n",
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

#[test]
fn contract_predicate_strict_or_equality_disjunction_requires_matching_order_path() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low <= mid and mid <= high) or high < low or low == high\n",
            "ensure not (output == mid and mid <= high) or high < output or output == high\n",
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
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_non_strict_cycles_transitively_imply_equality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low == mid and mid == high) or low == high\n",
            "require not (low <= mid and mid <= low) or low == mid\n",
            "ensure not (output == mid and mid == high) or output == high\n",
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
fn contract_predicate_equality_paths_transitively_imply_disequality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low == mid and mid != high) or low != high\n",
            "require not (high != mid and mid == low) or high != low\n",
            "ensure not (output == mid and mid != high) or output != high\n",
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
fn contract_predicate_multi_hop_equality_paths_imply_disequality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(a: Int, b: Int, c: Int, d: Int, e: Int) -> output: Int effects []\n",
            "require not (a == b and b == c and c != d and d == e) or a != e\n",
            "require not (a == b and c != d and d == e) or e != c\n",
            "ensure not (output == b and b == c and c != d and d == e) or output != e\n",
            "  a\n",
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
fn contract_predicate_equality_paths_do_not_imply_disequality_without_disequality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low == mid and mid == high) or low != high\n",
            "ensure not (output == mid and mid == high) or output != high\n",
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
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_strict_order_paths_transitively_imply_disequality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low < mid and mid <= high) or low != high\n",
            "require not (high >= mid and mid > low) or high != low\n",
            "ensure not (output == mid and mid < high) or output != high\n",
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
fn contract_predicate_strict_paths_do_not_imply_equality() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low < mid and mid <= high) or low == high\n",
            "ensure not (output < mid and mid <= high) or output == high\n",
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
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_order_paths_contradict_equality_relations() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low <= mid and mid <= low and low != mid)\n",
            "require not (low < mid and mid <= high and low == high)\n",
            "ensure not (output <= mid and mid <= output and output != mid)\n",
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
fn contract_predicate_strict_order_cycles_are_statically_proven_false() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low < mid and mid <= high and high <= low)\n",
            "ensure not (output <= mid and mid < high and high <= output)\n",
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

#[test]
fn contract_predicate_order_path_contradiction_requires_matching_path() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low <= mid and mid <= high and low != high)\n",
            "require not (low <= mid and mid <= high and low == high)\n",
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
        contract.obligation_status == ContractObligationStatus::RuntimeRequired
    }));
}

#[test]
fn contract_predicate_non_strict_order_path_does_not_imply_strict_bound() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(low: Int, mid: Int, high: Int) -> output: Int effects []\n",
            "require not (low <= mid and mid <= high) or low < high\n",
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
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}
