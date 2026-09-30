use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use veln_repo_workflow_scripts::{
    check_memory_guards, check_memory_guards_with_workers, plan_shards,
    read_nextest_elapsed_seconds, render_duplication_summary, shard_plan_json,
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
        Some("plan-shards") => plan_shards_command(&args[1..]),
        Some("summarize-duplication") => summarize_duplication_command(&args[1..]),
        _ => Err("usage: veln-repo-workflow-scripts <check-memory-guards|plan-shards|summarize-duplication> ...".to_owned()),
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
            let workers = value.parse::<usize>().ok().filter(|value| *value > 0).ok_or_else(|| {
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

fn plan_shards_command(args: &[String]) -> Result<ExitCode, String> {
    if args.len() != 4 {
        return Err("usage: veln-repo-workflow-scripts plan-shards <report-root> <target-seconds> <fallback-shards> <max-shards>".to_owned());
    }
    let parse = |index: usize, name: &str| {
        args[index]
            .parse::<u64>()
            .ok()
            .filter(|value| *value > 0)
            .ok_or_else(|| {
                format!("set {name} to a positive integer so shard planning has a valid bound")
            })
    };
    let elapsed = read_nextest_elapsed_seconds(PathBuf::from(&args[0]).as_path())
        .map_err(|error| error.to_string())?;
    let plan = plan_shards(
        &elapsed,
        parse(1, "target-seconds")?,
        parse(2, "fallback-shards")?,
        parse(3, "max-shards")?,
    );
    println!("{}", shard_plan_json(&plan));
    Ok(ExitCode::SUCCESS)
}

fn summarize_duplication_command(args: &[String]) -> Result<ExitCode, String> {
    if args.len() != 1 {
        return Err(
            "usage: veln-repo-workflow-scripts summarize-duplication <jscpd-report.json>"
                .to_owned(),
        );
    }
    let input = fs::read_to_string(&args[0]).map_err(|error| error.to_string())?;
    let report = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    let summary = render_duplication_summary(&report, 10)?;
    if let Some(path) = env::var_os("GITHUB_STEP_SUMMARY").filter(|path| !path.is_empty()) {
        use std::io::Write;
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(summary.as_bytes()))
            .map_err(|error| error.to_string())?;
    } else {
        println!("{summary}");
    }
    Ok(ExitCode::SUCCESS)
}
