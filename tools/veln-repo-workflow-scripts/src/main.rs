use std::env;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use veln_repo_workflow_scripts::{
    CommandResult, check_memory_guards, check_memory_guards_with_workers,
    download_prior_nextest_reports, escape_annotation_message, plan_shards,
    read_nextest_elapsed_seconds, render_duplication_summary, render_shard_summary,
    shard_numbers_json,
};

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
    let target_seconds = positive_environment_integer("NEXTEST_TARGET_SECONDS")?;
    let fallback_shards = positive_environment_integer("NEXTEST_FALLBACK_SHARDS")?;
    let max_shards = positive_environment_integer("NEXTEST_MAX_SHARDS")?;
    let runner_temp = required_environment_path(
        "RUNNER_TEMP",
        "Run shard planning on a GitHub Actions runner so prior reports have a workspace.",
    )?;
    let output_path = required_environment_path(
        "GITHUB_OUTPUT",
        "Run shard planning as a GitHub Actions step so its matrix outputs can be published.",
    )?;

    let report_root = download_prior_nextest_reports(
        &runner_temp,
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
    let elapsed = read_nextest_elapsed_seconds(&report_root).map_err(|error| error.to_string())?;
    let plan = plan_shards(&elapsed, target_seconds, fallback_shards, max_shards);

    append_line(&output_path, &format!("shard_count={}", plan.shard_count))?;
    append_line(
        &output_path,
        &format!("shards={}", shard_numbers_json(&plan)),
    )?;
    let summary = render_shard_summary(&plan, target_seconds, max_shards);
    println!("{summary}");
    if let Some(path) = optional_environment_path("GITHUB_STEP_SUMMARY") {
        append_line(&path, &summary)?;
    }
    Ok(ExitCode::SUCCESS)
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
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    writeln!(file, "{line}").map_err(|error| error.to_string())
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
