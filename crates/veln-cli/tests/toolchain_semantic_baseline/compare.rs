use super::*;

pub(super) fn compare(expected: &Inventory, actual: &Inventory) -> Result<(), String> {
    compare_metadata(expected, actual)?;
    let expected_ids = compare_case_ids(expected, actual)?;
    let differences = collect_field_differences(expected, actual, expected_ids);
    if differences.is_empty() {
        Ok(())
    } else {
        Err(differences.join("\n"))
    }
}

fn compare_metadata(expected: &Inventory, actual: &Inventory) -> Result<(), String> {
    if expected.schema != actual.schema {
        return Err(format!(
            "schema changed: expected `{}`, got `{}`",
            expected.schema, actual.schema
        ));
    }
    if expected.roots != actual.roots {
        return Err(format!(
            "authoritative roots changed: expected {:?}, got {:?}",
            expected.roots, actual.roots
        ));
    }
    Ok(())
}

fn compare_case_ids(expected: &Inventory, actual: &Inventory) -> Result<BTreeSet<String>, String> {
    let expected_ids = expected.cases.keys().cloned().collect::<BTreeSet<_>>();
    let actual_ids = actual.cases.keys().cloned().collect::<BTreeSet<_>>();
    if expected_ids == actual_ids {
        Ok(expected_ids)
    } else {
        Err(case_set_difference_message(&expected_ids, &actual_ids))
    }
}

fn case_set_difference_message(
    expected_ids: &BTreeSet<String>,
    actual_ids: &BTreeSet<String>,
) -> String {
    let removed = expected_ids
        .difference(actual_ids)
        .cloned()
        .collect::<Vec<_>>();
    let added = actual_ids
        .difference(expected_ids)
        .cloned()
        .collect::<Vec<_>>();
    format!("case set changed; removed={removed:?}; added={added:?}")
}

fn collect_field_differences(
    expected: &Inventory,
    actual: &Inventory,
    expected_ids: BTreeSet<String>,
) -> Vec<String> {
    let mut differences = Vec::new();
    for id in expected_ids {
        append_case_field_differences(expected, actual, &id, &mut differences);
        if differences.len() > 20 {
            break;
        }
    }
    differences
}

fn append_case_field_differences(
    expected: &Inventory,
    actual: &Inventory,
    id: &str,
    differences: &mut Vec<String>,
) {
    let expected_fields = &expected.cases[id];
    let actual_fields = &actual.cases[id];
    let paths = expected_fields
        .keys()
        .chain(actual_fields.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for path in paths {
        if let Some(difference) = field_difference(id, &path, expected_fields, actual_fields) {
            differences.push(difference);
        }
        if differences.len() == 20 {
            differences.push("additional differences omitted".to_string());
            break;
        }
    }
}

fn field_difference(
    id: &str,
    path: &str,
    expected_fields: &BTreeMap<String, String>,
    actual_fields: &BTreeMap<String, String>,
) -> Option<String> {
    match (expected_fields.get(path), actual_fields.get(path)) {
        (Some(expected), Some(actual)) if expected != actual => Some(format!(
            "{id} field `{path}` changed: expected {expected}, got {actual}"
        )),
        (Some(expected), None) => Some(format!(
            "{id} field `{path}` was removed (expected {expected})"
        )),
        (None, Some(actual)) => Some(format!("{id} field `{path}` was added ({actual})")),
        _ => None,
    }
}
