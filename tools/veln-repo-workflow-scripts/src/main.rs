use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use veln_repo_workflow_scripts::{
    CommandResult, check_memory_guards, check_memory_guards_with_workers,
    download_prior_nextest_reports, escape_annotation_message, plan_shards,
    read_nextest_elapsed_seconds, render_duplication_summary, render_shard_outputs,
    render_shard_summary,
};

struct NextestShardConfig {
    target_seconds: u64,
    fallback_shards: u64,
    max_shards: u64,
    runner_temp: PathBuf,
    output_path: PathBuf,
    summary_path: Option<PathBuf>,
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(code) => code,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<ExitCode, String> {
    match args.first().map(String::as_str) {
        Some("check-memory-guards") => check_memory_guard_command(&args[1..]),
        Some("plan-nextest-shards") => plan_nextest_shards_command(&args[1..]),
        Some("summarize-duplication") => summarize_duplication_command(&args[1..]),
        _ => Err("usage: veln-repo-workflow-scripts <check-memory-guards|plan-nextest-shards|summarize-duplication> ...".to_owned()),
    }
}

fn check_memory_guard_command(args: &[String]) -> Result<ExitCode, String> {
    if args.is_empty() {
        return Err(
            "usage: veln-repo-workflow-scripts check-memory-guards <workflow-or-action>..."
                .to_owned(),
        );
    }
    let targets: Vec<_> = args.iter().map(PathBuf::from).collect();
    let failures = match env::var("VELN_WORKFLOW_SCRIPT_THREADS") {
        Ok(value) => {
            let workers = value
                .parse::<usize>()
                .ok()
                .filter(|value| *value > 0)
                .ok_or_else(|| {
                    "set VELN_WORKFLOW_SCRIPT_THREADS to a positive integer when overriding scan parallelism".to_owned()
                })?;
            check_memory_guards_with_workers(&targets, workers)
        }
        Err(_) => check_memory_guards(&targets),
    }
    .map_err(|error| error.to_string())?;
    for failure in &failures {
        eprintln!(
            "{}:{}: run this Rust test command through `bash scripts/ci-run`; the CI runner fixes Cargo concurrency and process memory so tests cannot exhaust the host before the job timeout",
            failure.file.display(),
            failure.line,
        );
    }
    Ok(if failures.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn plan_nextest_shards_command(args: &[String]) -> Result<ExitCode, String> {
    if !args.is_empty() {
        return Err("usage: veln-repo-workflow-scripts plan-nextest-shards".to_owned());
    }

    let config = nextest_shard_config()?;
    let report_root = prior_nextest_report_root(&config.runner_temp)?;
    let elapsed = read_nextest_elapsed_seconds(&report_root).map_err(|error| error.to_string())?;
    let plan = plan_shards(
        &elapsed,
        config.target_seconds,
        config.fallback_shards,
        config.max_shards,
    );
    publish_nextest_shard_plan(&config, &plan)?;
    Ok(ExitCode::SUCCESS)
}

fn nextest_shard_config() -> Result<NextestShardConfig, String> {
    Ok(NextestShardConfig {
        target_seconds: positive_environment_integer("NEXTEST_TARGET_SECONDS")?,
        fallback_shards: positive_environment_integer("NEXTEST_FALLBACK_SHARDS")?,
        max_shards: positive_environment_integer("NEXTEST_MAX_SHARDS")?,
        runner_temp: required_environment_path(
            "RUNNER_TEMP",
            "Run shard planning on a GitHub Actions runner so prior reports have a workspace.",
        )?,
        output_path: required_environment_path(
            "GITHUB_OUTPUT",
            "Run shard planning as a GitHub Actions step so its matrix outputs can be published.",
        )?,
        summary_path: optional_environment_path("GITHUB_STEP_SUMMARY"),
    })
}

fn prior_nextest_report_root(runner_temp: &Path) -> Result<PathBuf, String> {
    let report_root = download_prior_nextest_reports(
        runner_temp,
        |args| {
            let output = Command::new("gh")
                .args(args)
                .stdin(Stdio::null())
                .stderr(Stdio::inherit())
                .output()?;
            Ok(CommandResult {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            })
        },
        |notice| println!("{notice}"),
    )
    .map_err(|error| error.to_string())?
    .unwrap_or_else(|| runner_temp.join("nextest-history"));
    Ok(report_root)
}

fn publish_nextest_shard_plan(
    config: &NextestShardConfig,
    plan: &veln_repo_workflow_scripts::ShardPlan,
) -> Result<(), String> {
    append_text(&config.output_path, &render_shard_outputs(plan))?;
    let summary = render_shard_summary(plan, config.target_seconds, config.max_shards);
    println!("{summary}");
    if let Some(path) = &config.summary_path {
        append_line(path, &summary)?;
    }
    Ok(())
}

fn positive_environment_integer(name: &str) -> Result<u64, String> {
    const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
    env::var(name)
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| (1..=MAX_SAFE_INTEGER).contains(value))
        .ok_or_else(|| {
            format!("Set {name} to a positive integer so shard planning has a valid bound.")
        })
}

fn required_environment_path(name: &str, message: &str) -> Result<PathBuf, String> {
    optional_environment_path(name).ok_or_else(|| message.to_owned())
}

fn optional_environment_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn append_line(path: &Path, line: &str) -> Result<(), String> {
    append_text(path, &format!("{line}\n"))
}

fn append_text(path: &Path, text: &str) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    file.write_all(text.as_bytes())
        .map_err(|error| error.to_string())
}

fn summarize_duplication_command(args: &[String]) -> Result<ExitCode, String> {
    let result = summarize_duplication(args);
    match result {
        Ok(()) => Ok(ExitCode::SUCCESS),
        Err(error) => {
            let message = format!(
                "Regenerate the jscpd JSON report before rerunning this check; CI cannot summarize code duplication from an invalid report. {error}"
            );
            if env::var("GITHUB_ACTIONS").as_deref() == Ok("true") {
                Err(format!(
                    "::error title=Invalid jscpd report::{}",
                    escape_annotation_message(&message)
                ))
            } else {
                Err(message)
            }
        }
    }
}

fn summarize_duplication(args: &[String]) -> Result<(), String> {
    if args.len() != 1 {
        return Err("expected the jscpd JSON report path as the first argument".to_owned());
    }
    let input = fs::read_to_string(&args[0]).map_err(|error| error.to_string())?;
    let report = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let summary = render_duplication_summary(&report, 10)?;
    if let Some(path) = optional_environment_path("GITHUB_STEP_SUMMARY") {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.write_all(summary.as_bytes())
            .map_err(|error| error.to_string())?;
    } else {
        println!("{summary}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_github_outputs_for_the_planned_shards() {
        let plan = plan_shards(&[191.0], 90, 4, 16);
        assert_eq!(
            render_shard_outputs(&plan),
            "shard_count=3\nshards=[1,2,3]\n"
        );
    }

    #[test]
    fn publishes_github_outputs_and_step_summary() {
        let root = env::temp_dir().join(format!(
            "veln-workflow-shard-publish-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let config = NextestShardConfig {
            target_seconds: 90,
            fallback_shards: 4,
            max_shards: 16,
            runner_temp: root.clone(),
            output_path: root.join("output"),
            summary_path: Some(root.join("summary")),
        };
        let plan = plan_shards(&[191.0], 90, 4, 16);

        publish_nextest_shard_plan(&config, &plan).unwrap();

        assert_eq!(
            fs::read_to_string(&config.output_path).unwrap(),
            "shard_count=3\nshards=[1,2,3]\n"
        );
        assert_eq!(
            fs::read_to_string(config.summary_path.as_ref().unwrap()).unwrap(),
            format!("{}\n", render_shard_summary(&plan, 90, 16))
        );
        fs::remove_dir_all(root).unwrap();
    }
}
