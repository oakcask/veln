use super::*;

#[test]
fn contract_predicate_negated_disjunction_covered_by_complement_conjuncts_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int, extra: Bool) -> output: {ready: Bool}\n",
            "require not ((value.ready or extra) and not value.ready and not extra)\n",
            "require not ((limit < 10 or value.ready or false) and limit >= 10 and not value.ready)\n",
            "ensure not ((output.ready or extra) and not output.ready and not extra)\n",
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
fn contract_predicate_negated_disjunction_with_repeated_branch_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not (value.ready and not (value.ready or extra))\n",
            "ensure not (output.ready and extra and not (output.ready or value.ready))\n",
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
fn contract_predicate_resolved_complementary_disjunctions_are_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not (value.ready and (not value.ready or extra) and (not value.ready or not extra))\n",
            "ensure not (not output.ready and (output.ready or extra) and (output.ready or not extra))\n",
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
fn contract_predicate_partial_negated_disjunction_requires_runtime_check() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not ((value.ready or extra) and not value.ready)\n",
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
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn contract_predicate_factored_case_split_covered_by_complements_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int, extra: Bool) -> output: {ready: Bool}\n",
            "require (value.ready and extra) or (not value.ready and extra) or not extra\n",
            "require (limit < 10 and value.ready) or (limit >= 10 and value.ready) or not value.ready\n",
            "ensure (output.ready and extra and true) or (not output.ready and extra) or not extra\n",
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
fn contract_predicate_partial_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int, extra: Bool) -> output: {ready: Bool}\n",
            "require value.ready or (not value.ready and extra) or (not value.ready and not extra)\n",
            "require limit < 10 or (limit >= 10 and value.ready) or (limit >= 10 and not value.ready)\n",
            "ensure output.ready or (not output.ready and extra) or (not output.ready and not extra)\n",
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
fn contract_predicate_wide_partial_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {a: Bool, b: Bool, c: Bool, d: Bool}) -> output: {a: Bool, b: Bool, c: Bool, d: Bool}\n",
            "require value.a or ",
            "(not value.a and value.b) or ",
            "(not value.a and not value.b and value.c) or ",
            "(not value.a and not value.b and not value.c and value.d) or ",
            "(not value.a and not value.b and not value.c and not value.d)\n",
            "ensure output.a or ",
            "(not output.a and output.b) or ",
            "(not output.a and not output.b and output.c) or ",
            "(not output.a and not output.b and not output.c and output.d) or ",
            "(not output.a and not output.b and not output.c and not output.d)\n",
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
fn contract_predicate_max_width_partial_case_split_or_is_statically_proven() {
    let fields = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"];
    let record_type = bool_record_type(&fields);
    let predicate = partial_case_split_chain_predicate("value", &fields);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{{record_type}}}) -> output: {{{record_type}}}\nrequire {predicate}\n  value\nend\n"
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
        ContractObligationStatus::StaticallyProven
    );
}

#[test]
fn contract_predicate_too_wide_partial_case_split_or_requires_runtime_check() {
    let fields = [
        "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n",
    ];
    let record_type = bool_record_type(&fields);
    let predicate = partial_case_split_chain_predicate("value", &fields);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{{record_type}}}) -> output: {{{record_type}}}\nrequire {predicate}\n  value\nend\n"
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
fn contract_predicate_negated_partial_case_split_and_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not ((value.ready or extra) and (value.ready or not extra) and (not value.ready or extra) and (not value.ready or not extra))\n",
            "require not ((value.ready or extra) and (value.ready or not extra) and not value.ready)\n",
            "ensure not ((output.ready or extra) and (output.ready or not extra) and (not output.ready or extra) and (not output.ready or not extra))\n",
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
fn contract_predicate_partial_case_split_and_without_full_rejection_requires_runtime_check() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not ((value.ready or extra) and (value.ready or not extra) and (not value.ready or extra))\n",
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
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn contract_predicate_exhaustive_pair_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, limit: Int, extra: Bool) -> output: {ready: Bool}\n",
            "require (value.ready and extra) or (value.ready and not extra) or (not value.ready and extra) or (not value.ready and not extra)\n",
            "require (limit < 10 and value.ready) or (limit < 10 and not value.ready) or (limit >= 10 and value.ready) or (limit >= 10 and not value.ready)\n",
            "ensure (output.ready and extra) or (not extra and output.ready) or (not output.ready and extra) or (not output.ready and not extra)\n",
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
fn contract_predicate_exhaustive_triple_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool, paid: Bool, shipped: Bool}) -> output: {ready: Bool, paid: Bool, shipped: Bool}\n",
            "require (value.ready and value.paid and value.shipped) or ",
            "(value.ready and value.paid and not value.shipped) or ",
            "(value.ready and not value.paid and value.shipped) or ",
            "(value.ready and not value.paid and not value.shipped) or ",
            "(not value.ready and value.paid and value.shipped) or ",
            "(not value.ready and value.paid and not value.shipped) or ",
            "(not value.ready and not value.paid and value.shipped) or ",
            "(not value.ready and not value.paid and not value.shipped)\n",
            "ensure (output.ready and output.paid and output.shipped) or ",
            "(output.ready and output.paid and not output.shipped) or ",
            "(output.ready and not output.paid and output.shipped) or ",
            "(output.ready and not output.paid and not output.shipped) or ",
            "(not output.ready and output.paid and output.shipped) or ",
            "(not output.ready and output.paid and not output.shipped) or ",
            "(not output.ready and not output.paid and output.shipped) or ",
            "(not output.ready and not output.paid and not output.shipped)\n",
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
fn contract_predicate_exhaustive_quad_case_split_or_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {a: Bool, b: Bool, c: Bool, d: Bool}) -> output: {a: Bool, b: Bool, c: Bool, d: Bool}\n",
            "require (value.a and value.b and value.c and value.d) or ",
            "(value.a and value.b and value.c and not value.d) or ",
            "(value.a and value.b and not value.c and value.d) or ",
            "(value.a and value.b and not value.c and not value.d) or ",
            "(value.a and not value.b and value.c and value.d) or ",
            "(value.a and not value.b and value.c and not value.d) or ",
            "(value.a and not value.b and not value.c and value.d) or ",
            "(value.a and not value.b and not value.c and not value.d) or ",
            "(not value.a and value.b and value.c and value.d) or ",
            "(not value.a and value.b and value.c and not value.d) or ",
            "(not value.a and value.b and not value.c and value.d) or ",
            "(not value.a and value.b and not value.c and not value.d) or ",
            "(not value.a and not value.b and value.c and value.d) or ",
            "(not value.a and not value.b and value.c and not value.d) or ",
            "(not value.a and not value.b and not value.c and value.d) or ",
            "(not value.a and not value.b and not value.c and not value.d)\n",
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
    assert_eq!(contracts.len(), 1);
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_exhaustive_quint_case_split_or_is_statically_proven() {
    let predicate = exhaustive_case_split_predicate("value", &["a", "b", "c", "d", "e"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}}) -> output: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool}}\nrequire {predicate}\n  value\nend\n"
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
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_exhaustive_sext_case_split_or_is_statically_proven() {
    let predicate = exhaustive_case_split_predicate("value", &["a", "b", "c", "d", "e", "f"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}}) -> output: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool}}\nrequire {predicate}\n  value\nend\n"
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
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_exhaustive_sept_case_split_or_is_statically_proven() {
    let predicate = exhaustive_case_split_predicate("value", &["a", "b", "c", "d", "e", "f", "g"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}) -> output: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool}}\nrequire {predicate}\n  value\nend\n"
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
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_exhaustive_oct_case_split_or_is_statically_proven() {
    let predicate =
        exhaustive_case_split_predicate("value", &["a", "b", "c", "d", "e", "f", "g", "h"]);
    let source = SourceFile::new(
        "main.veln",
        format!(
            "pub fn identity(value: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}) -> output: {{a: Bool, b: Bool, c: Bool, d: Bool, e: Bool, f: Bool, g: Bool, h: Bool}}\nrequire {predicate}\n  value\nend\n"
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
    assert!(contracts.iter().all(|contract| {
        contract.obligation_status == ContractObligationStatus::StaticallyProven
    }));
}

#[test]
fn contract_predicate_negated_conjunction_prefix_requires_runtime_check() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool, paid: Bool}) -> output: {ready: Bool, paid: Bool}\n",
            "require (not value.ready and value.paid) or (value.ready and value.paid)\n",
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
    assert_eq!(contracts.len(), 1);
    assert_eq!(
        contracts[0].obligation_status,
        ContractObligationStatus::RuntimeRequired
    );
}

#[test]
fn contract_predicate_small_boolean_formula_is_statically_proven() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "pub fn identity(value: {ready: Bool}, extra: Bool) -> output: {ready: Bool}\n",
            "require not (value.ready and not extra) or not (not value.ready and not extra)\n",
            "ensure not (output.ready and not extra) or not (not output.ready and not extra)\n",
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
    assert!(
        contracts.iter().all(|contract| {
            contract.obligation_status == ContractObligationStatus::StaticallyProven
        }),
        "{contracts:#?}"
    );
}

#[test]
fn contract_predicate_ten_atom_boolean_formula_is_statically_proven() {
    let fields = ["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"];
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
