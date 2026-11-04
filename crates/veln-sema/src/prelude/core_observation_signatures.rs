use veln_core::CoreType;

pub(super) fn core_observation_export_signature(
    name: &str,
    expected: &CoreType,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "observation_json_line" => Some((vec![CoreType::Unknown; 9], expected.clone())),
        "observation_export_write" => Some((
            vec![CoreType::Unknown, CoreType::string(), CoreType::bool()],
            CoreType::bool(),
        )),
        "observation_export_permit_create" => Some((vec![], CoreType::int())),
        "observation_export_permit_revoke" => Some((vec![CoreType::int()], CoreType::int())),
        "observation_export_permit_discard" => Some((vec![CoreType::int()], CoreType::unit())),
        "observation_export_permit_count" => Some((vec![], CoreType::int())),
        "observation_export_write_coordinated" => Some((
            vec![
                CoreType::Unknown,
                CoreType::string(),
                CoreType::bool(),
                CoreType::int(),
                CoreType::Unknown,
            ],
            CoreType::int(),
        )),
        "observation_export_identity" => Some((
            vec![CoreType::string(), CoreType::string(), CoreType::int()],
            CoreType::string(),
        )),
        _ => None,
    }
}

pub(super) fn core_production_metric_index_signature(
    name: &str,
) -> Option<(Vec<CoreType>, CoreType)> {
    match name {
        "production_metric_index_new" => Some((vec![CoreType::int()], CoreType::int())),
        "production_metric_index_contains" => {
            Some((vec![CoreType::int(), CoreType::Unknown], CoreType::bool()))
        }
        "production_metric_index_insert" => {
            Some((vec![CoreType::int(), CoreType::Unknown], CoreType::unit()))
        }
        "production_metric_index_work" => Some((vec![CoreType::int()], CoreType::int())),
        "production_metric_index_reset_work" => Some((vec![CoreType::int()], CoreType::unit())),
        _ => None,
    }
}
