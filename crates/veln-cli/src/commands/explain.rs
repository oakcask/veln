use std::process::ExitCode;

struct Explanation {
    id: &'static str,
    title: &'static str,
    meaning: &'static str,
    repair: &'static str,
}

const EXPLANATIONS: &[Explanation] = &[
    Explanation {
        id: "hole.unfilled",
        title: "unfilled typed hole",
        meaning: "The checker reached a hole expression and kept the program checkable, but executable lowering is blocked if the hole is reachable from the selected entry.",
        repair: "Fill the hole with an expression assignable to the expected type, or use the ranked candidate query records as unapplied repair hints.",
    },
    Explanation {
        id: "hole.satisfy_type_mismatch",
        title: "non-boolean satisfy predicate",
        meaning: "A hole satisfy suffix parsed, but the predicate does not have Bool type after binding the local candidate to the hole's expected type.",
        repair: "Rewrite the satisfy predicate as a pure boolean condition over the candidate and visible bindings.",
    },
    Explanation {
        id: "hole.satisfy_candidate_shadow",
        title: "shadowed satisfy candidate",
        meaning: "The candidate name in a satisfy suffix conflicts with a visible binding or compiler-known helper name.",
        repair: "Choose a candidate name that is local to the satisfy predicate and does not match an existing visible name.",
    },
    Explanation {
        id: "hole.satisfy_candidate_unused",
        title: "unused satisfy candidate",
        meaning: "A hole satisfy suffix parsed, but the predicate does not mention the local candidate binding.",
        repair: "Reference the candidate in the predicate, or remove the satisfy suffix if no repair constraint is intended.",
    },
    Explanation {
        id: "parse.contract_predicate",
        title: "unsupported contract predicate syntax",
        meaning: "A require, ensure, or invariant clause uses syntax outside the supported pure boolean predicate grammar.",
        repair: "Use names, literals, grouping, field access, pure calls, arithmetic, comparisons, equality, and boolean operators only.",
    },
    Explanation {
        id: "parse.satisfy_candidate",
        title: "missing satisfy candidate",
        meaning: "A hole satisfy suffix starts with satisfy but does not provide the local candidate binding before the predicate arrow.",
        repair: "Write the suffix as satisfy candidate => predicate, choosing a candidate name that can be used inside the predicate.",
    },
    Explanation {
        id: "parse.satisfy_arrow",
        title: "missing satisfy arrow",
        meaning: "A hole satisfy suffix has a candidate binding but does not include the required => arrow before the predicate.",
        repair: "Insert => between the candidate binding and the satisfy predicate.",
    },
    Explanation {
        id: "parse.satisfy_predicate",
        title: "unsupported satisfy predicate syntax",
        meaning: "A hole satisfy suffix uses syntax outside the same pure predicate grammar used by contracts.",
        repair: "Keep the suffix in the form satisfy candidate => predicate, where predicate is a pure boolean expression over the candidate.",
    },
];

pub(crate) fn explain(list: bool, diagnostic_id: Option<String>) -> Result<ExitCode, String> {
    if list {
        for explanation in EXPLANATIONS {
            println!("{} - {}", explanation.id, explanation.title);
        }
        return Ok(ExitCode::SUCCESS);
    }

    let Some(diagnostic_id) = diagnostic_id else {
        return Err("explain requires a diagnostic id or --list".to_string());
    };
    let Some(explanation) = EXPLANATIONS
        .iter()
        .find(|explanation| explanation.id == diagnostic_id)
    else {
        return Err(format!("no explanation for diagnostic `{diagnostic_id}`"));
    };

    println!("{}: {}", explanation.id, explanation.title);
    println!();
    println!("Meaning: {}", explanation.meaning);
    println!("Repair: {}", explanation.repair);
    Ok(ExitCode::SUCCESS)
}
