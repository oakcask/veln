use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;

use serde_json::Value;

mod duplication;

pub use duplication::{escape_annotation_message, render_duplication_summary};

#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct GuardFailure {
    pub file: PathBuf,
    pub line: usize,
}

pub fn check_memory_guards(targets: &[PathBuf]) -> io::Result<Vec<GuardFailure>> {
    let worker_count = thread::available_parallelism().map_or(1, usize::from);
    check_memory_guards_with_workers(targets, worker_count)
}

pub fn check_memory_guards_with_workers(
    targets: &[PathBuf],
    requested_workers: usize,
) -> io::Result<Vec<GuardFailure>> {
    let mut files = Vec::new();
    for target in targets {
        collect_yaml_files(target, &mut files)?;
    }
    files.sort();
    files.dedup();

    let worker_count = requested_workers.max(1).min(files.len().max(1));
    let failures = Mutex::new(Vec::new());
    let first_error = Mutex::new(None);

    thread::scope(|scope| {
        for worker in 0..worker_count {
            let files = &files;
            let failures = &failures;
            let first_error = &first_error;
            scope.spawn(move || {
                for file in files.iter().skip(worker).step_by(worker_count) {
                    match fs::read_to_string(file) {
                        Ok(source) => {
                            let found =
                                unguarded_rust_test_lines(&source).into_iter().map(|line| {
                                    GuardFailure {
                                        file: file.clone(),
                                        line,
                                    }
                                });
                            failures
                                .lock()
                                .expect("failure lock poisoned")
                                .extend(found);
                        }
                        Err(error) => {
                            let mut slot = first_error.lock().expect("error lock poisoned");
                            if slot.is_none() {
                                *slot = Some(error);
                            }
                            break;
                        }
                    }
                }
            });
        }
    });

    if let Some(error) = first_error.into_inner().expect("error lock poisoned") {
        return Err(error);
    }
    let mut failures = failures.into_inner().expect("failure lock poisoned");
    failures.sort();
    Ok(failures)
}

pub fn unguarded_rust_test_lines(source: &str) -> Vec<usize> {
    workflow_run_commands(source)
        .into_iter()
        .filter_map(|(line, command)| {
            let is_test = contains_words(command, &["cargo", "test"])
                || contains_words(command, &["cargo", "nextest", "run"])
                || (contains_words(command, &["cargo", "llvm-cov"])
                    && command
                        .split_ascii_whitespace()
                        .any(|word| word == "nextest"));
            let guarded = contains_words(command, &["bash", "scripts/ci-run"]);
            (is_test && !guarded).then_some(line)
        })
        .collect()
}

fn workflow_run_commands(source: &str) -> Vec<(usize, &str)> {
    let lines: Vec<_> = source.lines().collect();
    let mut commands = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim_start();
        let candidate = trimmed.strip_prefix("- ").unwrap_or(trimmed);
        let Some(value) = candidate.strip_prefix("run:") else {
            index += 1;
            continue;
        };
        let value = value.trim();
        if !matches!(value, "|" | "|-" | "|+" | ">" | ">-" | ">+") {
            commands.push((index + 1, value));
            index += 1;
            continue;
        }

        let run_indent = line.len() - trimmed.len();
        index += 1;
        while index < lines.len() {
            let command = lines[index];
            let command_indent = command.len() - command.trim_start().len();
            if !command.trim().is_empty() && command_indent <= run_indent {
                break;
            }
            if !command.trim().is_empty() {
                commands.push((index + 1, command.trim()));
            }
            index += 1;
        }
    }
    commands
}

fn contains_words(command: &str, expected: &[&str]) -> bool {
    let words: Vec<_> = command.split_ascii_whitespace().collect();
    words
        .windows(expected.len())
        .any(|window| window == expected)
}

fn collect_yaml_files(path: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if path.is_file() {
        if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yaml" | "yml")
        ) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_yaml_files(&path, files)?;
        } else if matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("yaml" | "yml")
        ) {
            files.push(path);
        }
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub struct ShardPlan {
    pub shard_count: u64,
    pub measured_seconds: Option<f64>,
    pub required_shards: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct CommandResult {
    pub success: bool,
    pub stdout: String,
}

pub fn download_prior_nextest_reports(
    runner_temp: &Path,
    mut run_command: impl FnMut(&[String]) -> io::Result<CommandResult>,
    mut notice: impl FnMut(&str),
) -> io::Result<Option<PathBuf>> {
    let query_args = [
        "run",
        "list",
        "--workflow",
        "test--rust.yaml",
        "--status",
        "success",
        "--limit",
        "1",
        "--json",
        "databaseId",
        "--jq",
        ".[0].databaseId",
    ]
    .map(str::to_owned);
    let query = match run_command(&query_args) {
        Ok(result) if result.success => result,
        _ => {
            notice(
                "::notice::Using fallback Rust test shards because workflow history could not be queried; rerun if shard planning repeatedly cannot access prior reports.",
            );
            return Ok(None);
        }
    };
    let run_id = query.stdout.trim();
    if run_id.is_empty() || run_id == "null" {
        return Ok(None);
    }

    let download_root = create_download_directory(runner_temp)?;
    let report_root = runner_temp.join("nextest-history");
    let download_args = vec![
        "run".to_owned(),
        "download".to_owned(),
        run_id.to_owned(),
        "--pattern".to_owned(),
        "nextest-junit-*".to_owned(),
        "--dir".to_owned(),
        download_root.to_string_lossy().into_owned(),
    ];
    let result = run_command(&download_args);
    let downloaded = matches!(result, Ok(CommandResult { success: true, .. }));
    if downloaded {
        fs::rename(&download_root, &report_root)?;
        return Ok(Some(report_root));
    }

    let _ = fs::remove_dir_all(&download_root);
    notice(
        "::notice::Using fallback Rust test shards because prior timing reports could not be downloaded; the next successful run will provide fresh reports.",
    );
    Ok(None)
}

fn create_download_directory(runner_temp: &Path) -> io::Result<PathBuf> {
    static NEXT_ID: AtomicU64 = AtomicU64::new(0);
    loop {
        let id = NEXT_ID.fetch_add(1, Ordering::Relaxed);
        let path = runner_temp.join(format!(
            "nextest-history-download-{}-{id}",
            std::process::id()
        ));
        match fs::create_dir(&path) {
            Ok(()) => return Ok(path),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
}

pub fn read_nextest_elapsed_seconds(root: &Path) -> io::Result<Vec<f64>> {
    let mut reports = Vec::new();
    collect_named_files(root, "junit.xml", &mut reports)?;
    reports.sort();
    let mut elapsed = Vec::new();
    for report in reports {
        let xml = fs::read_to_string(report)?;
        let Some(tag_start) = xml.match_indices("<testsuites").find_map(|(index, _)| {
            let next = xml[index + "<testsuites".len()..].chars().next();
            next.is_none_or(|character| !character.is_alphanumeric() && character != '_')
                .then_some(index)
        }) else {
            continue;
        };
        let Some(tag_end_offset) = xml[tag_start..].find('>') else {
            continue;
        };
        let tag = &xml[tag_start..tag_start + tag_end_offset];
        let Some(time_value) = tag
            .split_ascii_whitespace()
            .find_map(|attribute| attribute.strip_prefix("time=\""))
        else {
            continue;
        };
        let Some(time_end) = time_value.find('"') else {
            continue;
        };
        if let Ok(value) = time_value[..time_end].parse::<f64>()
            && value.is_finite()
            && value >= 0.0
        {
            elapsed.push(value);
        }
    }
    Ok(elapsed)
}

pub fn plan_shards(
    elapsed_seconds: &[f64],
    target_seconds: u64,
    fallback_shards: u64,
    max_shards: u64,
) -> ShardPlan {
    if elapsed_seconds.is_empty() {
        return ShardPlan {
            shard_count: fallback_shards.min(max_shards),
            measured_seconds: None,
            required_shards: None,
        };
    }
    let measured_seconds = elapsed_seconds.iter().sum::<f64>();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let required_shards = (measured_seconds / target_seconds as f64).ceil().max(1.0) as u64;
    ShardPlan {
        shard_count: required_shards.min(max_shards),
        measured_seconds: Some(measured_seconds),
        required_shards: Some(required_shards),
    }
}

pub fn shard_numbers_json(plan: &ShardPlan) -> Value {
    Value::Array((1..=plan.shard_count).map(Value::from).collect())
}

pub fn render_shard_summary(plan: &ShardPlan, target_seconds: u64, max_shards: u64) -> String {
    let Some(measured_seconds) = plan.measured_seconds else {
        return format!(
            "No prior nextest timing was available, so this run uses {} fallback shards. A successful run will provide timings for the next plan.",
            plan.shard_count
        );
    };
    let mut summary = format!(
        "Planned {} Rust test shards from {measured_seconds:.1} seconds of prior test time with a {target_seconds}-second target.",
        plan.shard_count
    );
    if plan
        .required_shards
        .is_some_and(|required| required > max_shards)
    {
        summary.push_str(&format!(
            " The plan was capped at {max_shards}; reduce test runtime or raise NEXTEST_MAX_SHARDS if shards keep exceeding the target."
        ));
    }
    summary
}

fn collect_named_files(path: &Path, name: &str, files: &mut Vec<PathBuf>) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    if path.is_file() {
        if path.file_name().and_then(|value| value.to_str()) == Some(name) {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_named_files(&path, name, files)?;
        } else if entry.file_name() == name {
            files.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_directory(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "veln-workflow-scripts-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        root
    }

    #[test]
    fn finds_unguarded_commands_at_source_lines() {
        let source = "steps:\n  - run: cargo test --workspace\n  - run: |\n      prepare\n      cargo nextest run --workspace\n  - run: bash scripts/ci-run cargo test --workspace\n";
        assert_eq!(unguarded_rust_test_lines(source), vec![2, 5]);
    }

    #[test]
    fn accepts_guarded_commands_and_ignores_non_test_commands() {
        let source = "steps:\n  - run: bash scripts/ci-run cargo test --workspace\n  - run: bash scripts/ci-run cargo nextest run --workspace\n  - run: bash scripts/ci-run cargo llvm-cov --no-report nextest --workspace\n  - run: cargo run --workspace\n";
        assert!(unguarded_rust_test_lines(source).is_empty());
    }

    #[test]
    fn plans_measured_and_fallback_shards() {
        assert_eq!(
            plan_shards(&[], 90, 4, 16),
            ShardPlan {
                shard_count: 4,
                measured_seconds: None,
                required_shards: None,
            }
        );
        assert_eq!(
            plan_shards(&[100.0, 91.0], 90, 4, 16),
            ShardPlan {
                shard_count: 3,
                measured_seconds: Some(191.0),
                required_shards: Some(3),
            }
        );
    }

    #[test]
    fn caps_shards_without_hiding_the_uncapped_requirement() {
        let plan = plan_shards(&[1_500.0], 90, 4, 16);
        assert_eq!(plan.shard_count, 16);
        assert_eq!(plan.required_shards, Some(17));
        assert_eq!(
            shard_numbers_json(&plan),
            serde_json::json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
        );
        assert!(render_shard_summary(&plan, 90, 16).contains("capped at 16"));
    }

    #[test]
    fn reads_only_valid_top_level_nextest_elapsed_times() {
        let root = temporary_directory("elapsed");
        for name in ["nextest-junit-1", "nextest-junit-2", "other", "lookalike"] {
            fs::create_dir(root.join(name)).unwrap();
        }
        fs::write(
            root.join("nextest-junit-1/junit.xml"),
            r#"<testsuites tests="10" time="12.5"><testsuite time="99"/></testsuites>"#,
        )
        .unwrap();
        fs::write(
            root.join("nextest-junit-2/junit.xml"),
            r#"<testsuites time="7.25" tests="8"></testsuites>"#,
        )
        .unwrap();
        fs::write(
            root.join("other/junit.xml"),
            r#"<testsuites time="invalid"/>"#,
        )
        .unwrap();
        fs::write(
            root.join("lookalike/junit.xml"),
            r#"<testsuitesExtra time="4.0"/><testsuites runtime="5.0"/>"#,
        )
        .unwrap();

        assert_eq!(
            read_nextest_elapsed_seconds(&root).unwrap(),
            vec![12.5, 7.25]
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn downloads_reports_from_latest_successful_run() {
        let root = temporary_directory("download");
        let mut calls = Vec::new();
        let report_root = download_prior_nextest_reports(
            &root,
            |args| {
                calls.push(args.to_vec());
                if args.get(1).map(String::as_str) == Some("list") {
                    return Ok(CommandResult {
                        success: true,
                        stdout: "12345\n".to_owned(),
                    });
                }
                let download_root = PathBuf::from(args.last().unwrap());
                fs::create_dir(download_root.join("nextest-junit-1"))?;
                fs::write(
                    download_root.join("nextest-junit-1/junit.xml"),
                    "<testsuites/>",
                )?;
                Ok(CommandResult {
                    success: true,
                    stdout: String::new(),
                })
            },
            |_| {},
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            calls[0][..4],
            ["run", "list", "--workflow", "test--rust.yaml"]
        );
        assert_eq!(calls[1][..3], ["run", "download", "12345"]);
        assert!(report_root.join("nextest-junit-1/junit.xml").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn falls_back_and_removes_partial_downloads() {
        let root = temporary_directory("fallback");
        let mut notices = Vec::new();
        let report_root = download_prior_nextest_reports(
            &root,
            |args| {
                if args.get(1).map(String::as_str) == Some("list") {
                    return Ok(CommandResult {
                        success: true,
                        stdout: "12345\n".to_owned(),
                    });
                }
                fs::write(
                    PathBuf::from(args.last().unwrap()).join("partial"),
                    "partial",
                )?;
                Ok(CommandResult {
                    success: false,
                    stdout: String::new(),
                })
            },
            |notice| notices.push(notice.to_owned()),
        )
        .unwrap();

        assert_eq!(report_root, None);
        assert_eq!(notices.len(), 1);
        assert!(notices[0].contains("prior timing reports could not be downloaded"));
        let remaining: Vec<_> = fs::read_dir(&root).unwrap().collect();
        assert!(remaining.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn falls_back_when_history_is_unavailable_or_empty() {
        let root = temporary_directory("history-unavailable");
        let mut notices = Vec::new();
        let unavailable = download_prior_nextest_reports(
            &root,
            |_| {
                Ok(CommandResult {
                    success: false,
                    stdout: String::new(),
                })
            },
            |notice| notices.push(notice.to_owned()),
        )
        .unwrap();
        assert_eq!(unavailable, None);
        assert!(notices[0].contains("workflow history could not be queried"));

        let mut call_count = 0;
        let empty = download_prior_nextest_reports(
            &root,
            |_| {
                call_count += 1;
                Ok(CommandResult {
                    success: true,
                    stdout: "null\n".to_owned(),
                })
            },
            |_| {},
        )
        .unwrap();
        assert_eq!(empty, None);
        assert_eq!(call_count, 1);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn renders_largest_duplication_first() {
        let report = serde_json::json!({
            "statistics": { "total": {
                "sources": 2, "lines": 40, "clones": 2, "duplicatedLines": 12, "percentage": 30.0
            }},
            "duplicates": [
                { "lines": 4, "tokens": 20, "firstFile": {"name": "b.rs", "start": 1}, "secondFile": {"name": "c.rs", "start": 2}},
                { "lines": 8, "tokens": 40, "firstFile": {"name": "a.rs", "start": 3}, "secondFile": {"name": "d.rs", "start": 4}}
            ]
        });
        let summary = render_duplication_summary(&report, 10).unwrap();
        assert!(summary.find("a.rs:3").unwrap() < summary.find("b.rs:1").unwrap());
    }

    #[test]
    fn renders_equal_sized_duplications_in_a_stable_order() {
        let report = serde_json::json!({
            "statistics": { "total": {
                "sources": 6, "lines": 60, "clones": 6, "duplicatedLines": 24, "percentage": 40.0
            }},
            "duplicates": [
                { "lines": 4, "tokens": 19, "firstFile": {"name": "a.rs", "start": 1}, "secondFile": {"name": "b.rs", "start": 1}},
                { "lines": 4, "tokens": 20, "firstFile": {"name": "b.rs", "start": 1}, "secondFile": {"name": "a.rs", "start": 1}},
                { "lines": 4, "tokens": 20, "firstFile": {"name": "a.rs", "start": 2}, "secondFile": {"name": "b.rs", "start": 1}},
                { "lines": 4, "tokens": 20, "firstFile": {"name": "a.rs", "start": 1}, "secondFile": {"name": "c.rs", "start": 1}},
                { "lines": 4, "tokens": 20, "firstFile": {"name": "a.rs", "start": 1}, "secondFile": {"name": "b.rs", "start": 2}},
                { "lines": 4, "tokens": 20, "firstFile": {"name": "a.rs", "start": 1}, "secondFile": {"name": "b.rs", "start": 1}}
            ]
        });

        let summary = render_duplication_summary(&report, 10).unwrap();
        let expected_rows = [
            "| 4 | 20 | `a.rs:1` | `b.rs:1` |",
            "| 4 | 20 | `a.rs:1` | `b.rs:2` |",
            "| 4 | 20 | `a.rs:1` | `c.rs:1` |",
            "| 4 | 20 | `a.rs:2` | `b.rs:1` |",
            "| 4 | 20 | `b.rs:1` | `a.rs:1` |",
            "| 4 | 19 | `a.rs:1` | `b.rs:1` |",
        ];
        let positions: Vec<_> = expected_rows
            .iter()
            .map(|row| summary.find(row).unwrap())
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn validates_and_escapes_duplication_reports() {
        let report = serde_json::json!({
            "statistics": { "total": {
                "sources": 1, "lines": 10, "clones": 1, "duplicatedLines": 4, "percentage": 40.0
            }},
            "duplicates": [
                { "lines": 4, "tokens": 20, "firstFile": {"name": "a|b`c.rs", "start": 1}, "secondFile": {"name": "d.rs", "start": 2}}
            ]
        });
        let summary = render_duplication_summary(&report, 10).unwrap();
        assert!(summary.contains("a\\|b\\`c.rs:1"));

        let invalid =
            serde_json::json!({"statistics": {"total": {"sources": 1}}, "duplicates": []});
        assert_eq!(
            render_duplication_summary(&invalid, 10).unwrap_err(),
            "expected statistics.total.lines to be a non-negative integer"
        );
    }

    #[test]
    fn limits_duplication_details_and_renders_empty_reports() {
        let report = serde_json::json!({
            "statistics": { "total": {
                "sources": 2, "lines": 40, "clones": 2, "duplicatedLines": 12, "percentage": 30.0
            }},
            "duplicates": [
                { "lines": 8, "tokens": 40, "firstFile": {"name": "a.rs", "start": 3}, "secondFile": {"name": "d.rs", "start": 4}},
                { "lines": 4, "tokens": 20, "firstFile": {"name": "b.rs", "start": 1}, "secondFile": {"name": "c.rs", "start": 2}}
            ]
        });
        let summary = render_duplication_summary(&report, 1).unwrap();
        assert!(summary.contains("a.rs:3"));
        assert!(!summary.contains("b.rs:1"));
        assert!(summary.contains("1 more clone pair(s) omitted"));

        let empty = serde_json::json!({
            "statistics": { "total": {
                "sources": 2, "lines": 40, "clones": 0, "duplicatedLines": 0, "percentage": 0.0
            }},
            "duplicates": []
        });
        assert!(
            render_duplication_summary(&empty, 10)
                .unwrap()
                .contains("No exact clone pairs were detected")
        );
    }

    #[test]
    fn rejects_incomplete_duplicate_locations() {
        let report = serde_json::json!({
            "statistics": { "total": {
                "sources": 1, "lines": 10, "clones": 1, "duplicatedLines": 4, "percentage": 40.0
            }},
            "duplicates": [
                { "lines": 4, "tokens": 20, "firstFile": {"name": "a.rs"}, "secondFile": {"name": "d.rs", "start": 2}}
            ]
        });
        assert!(
            render_duplication_summary(&report, 10)
                .unwrap_err()
                .contains("file name and start line")
        );
    }

    #[test]
    fn escapes_annotation_control_characters() {
        assert_eq!(
            escape_annotation_message("bad%value\r\nnext"),
            "bad%25value%0D%0Anext"
        );
    }
}
