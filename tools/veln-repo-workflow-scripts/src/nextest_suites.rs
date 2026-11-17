use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;

use serde_json::json;

use crate::{ShardPlan, plan_shards, read_nextest_elapsed_seconds, render_shard_summary};

const TOOLCHAIN_FILTER: &str = "binary(=toolchain_harness) & test(generated_toolchain_cases::)";

#[derive(Debug, PartialEq)]
pub struct SuiteShardPlan {
    pub suite: &'static str,
    pub filterset: String,
    pub plan: ShardPlan,
}

pub fn plan_nextest_suites(
    root: &Path,
    target_seconds: u64,
    fallback_shards: u64,
    max_shards: u64,
) -> io::Result<Vec<SuiteShardPlan>> {
    if max_shards < 2 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Set NEXTEST_MAX_SHARDS to at least 2 so both Rust test suites can run.",
        ));
    }
    let mut plans = Vec::new();
    for (suite, filterset) in [
        ("workspace", format!("not ({TOOLCHAIN_FILTER})")),
        ("toolchain", TOOLCHAIN_FILTER.to_owned()),
    ] {
        let elapsed = read_suite_elapsed_seconds(root, suite)?;
        plans.push(SuiteShardPlan {
            suite,
            filterset,
            plan: plan_shards(&elapsed, target_seconds, fallback_shards, max_shards),
        });
    }
    cap_total_shards(&mut plans, max_shards);
    Ok(plans)
}

fn read_suite_elapsed_seconds(root: &Path, suite: &str) -> io::Result<Vec<f64>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let prefix = format!("nextest-junit-{suite}-");
    let mut times = BTreeMap::new();
    let mut expected_count = None;
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(suffix) = name.to_str().and_then(|name| name.strip_prefix(&prefix)) else {
            continue;
        };
        let Some((index, count)) = suffix.split_once("-of-") else {
            return Ok(Vec::new());
        };
        let (Ok(index), Ok(count)) = (index.parse::<u64>(), count.parse::<u64>()) else {
            return Ok(Vec::new());
        };
        if index == 0 || index > count || expected_count.is_some_and(|prior| prior != count) {
            return Ok(Vec::new());
        }
        expected_count = Some(count);
        let elapsed = read_nextest_elapsed_seconds(&entry.path())?;
        if elapsed.len() != 1 || times.insert(index, elapsed[0]).is_some() {
            return Ok(Vec::new());
        }
    }
    if expected_count != Some(times.len() as u64) {
        return Ok(Vec::new());
    }
    Ok(times.into_values().collect())
}

fn cap_total_shards(plans: &mut [SuiteShardPlan], max_shards: u64) {
    let desired: Vec<_> = plans
        .iter()
        .map(|suite| suite.plan.required_shards.unwrap_or(suite.plan.shard_count))
        .collect();
    let total: u128 = desired.iter().map(|count| u128::from(*count)).sum();
    if total <= u128::from(max_shards) {
        return;
    }
    // Reserve one shard per suite, then apportion the remaining budget by demand.
    let remaining = max_shards - plans.len() as u64;
    let demand = total - plans.len() as u128;
    let mut remainders = Vec::new();
    for (index, suite) in plans.iter_mut().enumerate() {
        let weighted = u128::from(remaining) * u128::from(desired[index] - 1);
        suite.plan.shard_count = 1 + u64::try_from(weighted / demand).unwrap();
        remainders.push((index, weighted % demand));
    }
    remainders.sort_by_key(|(index, remainder)| (std::cmp::Reverse(*remainder), *index));
    let allocated: u64 = plans.iter().map(|suite| suite.plan.shard_count).sum();
    for (index, _) in remainders
        .into_iter()
        .take((max_shards - allocated) as usize)
    {
        plans[index].plan.shard_count += 1;
    }
}

pub fn render_suite_shard_outputs(plans: &[SuiteShardPlan]) -> String {
    let mut include = Vec::new();
    for suite in plans {
        for shard in 1..=suite.plan.shard_count {
            include.push(json!({
                "suite": suite.suite,
                "shard": shard,
                "shard_count": suite.plan.shard_count,
                "filterset": suite.filterset,
            }));
        }
    }
    format!("matrix={}\n", json!({"include": include}))
}

pub fn render_suite_shard_summary(
    plans: &[SuiteShardPlan],
    target_seconds: u64,
    max_shards: u64,
) -> String {
    plans
        .iter()
        .map(|suite| {
            format!(
                "{}: {}",
                suite.suite,
                render_shard_summary(&suite.plan, target_seconds, max_shards)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_report(root: &Path, name: &str, seconds: &str) {
        let directory = root.join(name);
        fs::create_dir_all(&directory).unwrap();
        fs::write(
            directory.join("junit.xml"),
            format!(r#"<testsuites time="{seconds}"/>"#),
        )
        .unwrap();
    }

    #[test]
    fn plans_suites_independently_and_requires_complete_history() {
        let root = crate::tests::temporary_directory("suite-history");
        write_report(&root, "nextest-junit-workspace-1-of-2", "10");
        write_report(&root, "nextest-junit-workspace-2-of-2", "100");
        write_report(&root, "nextest-junit-toolchain-1-of-2", "60");
        write_report(&root, "nextest-junit-1", "999");
        let plans = plan_nextest_suites(&root, 90, 4, 16).unwrap();
        assert_eq!(plans[0].plan.shard_count, 3);
        assert_eq!(plans[0].plan.measured_seconds, Some(110.0));
        assert_eq!(plans[1].plan.shard_count, 4);
        assert_eq!(plans[1].plan.measured_seconds, None);

        write_report(&root, "nextest-junit-toolchain-2-of-2", "60");
        let plans = plan_nextest_suites(&root, 90, 4, 16).unwrap();
        assert_eq!(plans[1].plan.shard_count, 2);
        assert_eq!(plans[1].plan.measured_seconds, Some(120.0));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_and_legacy_history_use_per_suite_fallbacks_with_one_shared_cap() {
        let root = crate::tests::temporary_directory("suite-fallback");
        write_report(&root, "nextest-junit-1", "1500");
        let plans = plan_nextest_suites(&root, 90, 4, 5).unwrap();
        assert_eq!(plans[0].plan.shard_count, 3);
        assert_eq!(plans[1].plan.shard_count, 2);
        assert!(
            plans
                .iter()
                .all(|suite| suite.plan.measured_seconds.is_none())
        );
        assert!(plan_nextest_suites(&root, 90, 4, 1).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn apportions_shared_cap_without_starving_a_suite() {
        let root = crate::tests::temporary_directory("suite-cap");
        write_report(&root, "nextest-junit-workspace-1-of-1", "1800");
        write_report(&root, "nextest-junit-toolchain-1-of-1", "360");
        let plans = plan_nextest_suites(&root, 90, 4, 16).unwrap();
        assert_eq!(plans[0].plan.required_shards, Some(20));
        assert_eq!(plans[1].plan.required_shards, Some(4));
        assert_eq!(plans[0].plan.shard_count, 13);
        assert_eq!(plans[1].plan.shard_count, 3);
        let plans = plan_nextest_suites(&root, 90, 4, 2).unwrap();
        assert!(plans.iter().all(|suite| suite.plan.shard_count == 1));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn rejects_inconsistent_or_invalid_suite_reports_without_using_partial_timings() {
        let root = crate::tests::temporary_directory("suite-invalid");
        write_report(&root, "nextest-junit-workspace-1-of-2", "10");
        write_report(&root, "nextest-junit-workspace-2-of-3", "10");
        write_report(&root, "nextest-junit-toolchain-1-of-1", "NaN");
        let plans = plan_nextest_suites(&root, 90, 4, 16).unwrap();
        assert!(
            plans
                .iter()
                .all(|suite| suite.plan.measured_seconds.is_none())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn matrix_has_unique_artifact_identities_and_complementary_filters() {
        let root = crate::tests::temporary_directory("suite-matrix");
        let plans = plan_nextest_suites(&root, 90, 2, 16).unwrap();
        let output = render_suite_shard_outputs(&plans);
        let matrix: serde_json::Value =
            serde_json::from_str(output.strip_prefix("matrix=").unwrap().trim()).unwrap();
        let include = matrix["include"].as_array().unwrap();
        assert_eq!(include.len(), 4);
        assert_eq!(include[0]["suite"], "workspace");
        assert_eq!(include[2]["suite"], "toolchain");
        assert_eq!(include[0]["filterset"], format!("not ({TOOLCHAIN_FILTER})"));
        assert_eq!(include[2]["filterset"], TOOLCHAIN_FILTER);
        assert_eq!(include[3]["shard_count"], 2);
        assert_eq!(include[3]["shard"], 2);
        assert!(!output.contains("\nshards="));
        assert!(!output.contains("\nshard_count="));
        fs::remove_dir_all(root).unwrap();
    }
}
