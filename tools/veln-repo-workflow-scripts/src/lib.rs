use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::thread;

use serde_json::Value;

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

pub fn read_nextest_elapsed_seconds(root: &Path) -> io::Result<Vec<f64>> {
    let mut reports = Vec::new();
    collect_named_files(root, "junit.xml", &mut reports)?;
    reports.sort();
    let mut elapsed = Vec::new();
    for report in reports {
        let xml = fs::read_to_string(report)?;
        let Some(tag_start) = xml.find("<testsuites") else {
            continue;
        };
        let Some(tag_end_offset) = xml[tag_start..].find('>') else {
            continue;
        };
        let tag = &xml[tag_start..tag_start + tag_end_offset];
        let Some(time_start) = tag.find("time=\"").map(|index| index + 6) else {
            continue;
        };
        let Some(time_end) = tag[time_start..].find('"') else {
            continue;
        };
        if let Ok(value) = tag[time_start..time_start + time_end].parse::<f64>()
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

pub fn render_duplication_summary(
    report: &Value,
    duplicate_limit: usize,
) -> Result<String, String> {
    let duplicates = report
        .get("duplicates")
        .and_then(Value::as_array)
        .ok_or_else(|| "expected a duplicates array".to_owned())?;
    let total = report.pointer("/statistics/total").ok_or_else(|| {
        "expected statistics.total.sources to be a non-negative integer".to_owned()
    })?;
    let sources = non_negative_integer(total, "sources")?;
    let lines = non_negative_integer(total, "lines")?;
    let clones = non_negative_integer(total, "clones")?;
    let duplicated_lines = non_negative_integer(total, "duplicatedLines")?;
    let percentage = total
        .get("percentage")
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite() && *value >= 0.0)
        .ok_or_else(|| {
            "expected statistics.total.percentage to be a non-negative finite number".to_owned()
        })?;

    let mut rows = Vec::with_capacity(duplicates.len());
    for duplicate in duplicates {
        rows.push(DuplicateRow::parse(duplicate)?);
    }
    rows.sort_by(|left, right| {
        right
            .lines
            .cmp(&left.lines)
            .then_with(|| right.tokens.cmp(&left.tokens))
            .then_with(|| left.first_name.cmp(&right.first_name))
            .then_with(|| left.first_start.cmp(&right.first_start))
            .then_with(|| left.second_name.cmp(&right.second_name))
            .then_with(|| left.second_start.cmp(&right.second_start))
    });

    let mut output = format!(
        "## Code Duplication Refactor Signals\n\nInspect the largest exact clone pairs when changing either occurrence; consolidating shared behavior can prevent fixes from diverging. This report is advisory because some repetition is intentional.\n\n- Rust files analyzed: {sources}\n- Rust lines analyzed: {lines}\n- Exact clone pairs: {clones}\n- Duplicated lines: {duplicated_lines} ({percentage:.2}%)\n\n### Largest exact clone pairs\n\n"
    );
    if rows.is_empty() {
        output.push_str("No exact clone pairs were detected.\n\n");
        return Ok(output);
    }
    output.push_str(
        "| Lines | Tokens | First occurrence | Second occurrence |\n| ---: | ---: | --- | --- |\n",
    );
    for row in rows.iter().take(duplicate_limit) {
        output.push_str(&format!(
            "| {} | {} | `{}:{}` | `{}:{}` |\n",
            row.lines,
            row.tokens,
            escape_markdown(&row.first_name),
            row.first_start,
            escape_markdown(&row.second_name),
            row.second_start,
        ));
    }
    if rows.len() > duplicate_limit {
        output.push_str(&format!(
            "\n{} more clone pair(s) omitted from this summary.\n",
            rows.len() - duplicate_limit
        ));
    }
    output.push('\n');
    Ok(output)
}

#[derive(Debug)]
struct DuplicateRow {
    lines: u64,
    tokens: u64,
    first_name: String,
    first_start: u64,
    second_name: String,
    second_start: u64,
}

impl DuplicateRow {
    fn parse(value: &Value) -> Result<Self, String> {
        let lines = value.get("lines").and_then(Value::as_u64).ok_or_else(|| {
            "expected each duplicate to have non-negative integer lines and tokens".to_owned()
        })?;
        let tokens = value.get("tokens").and_then(Value::as_u64).ok_or_else(|| {
            "expected each duplicate to have non-negative integer lines and tokens".to_owned()
        })?;
        let (first_name, first_start) = file_location(value.get("firstFile"))?;
        let (second_name, second_start) = file_location(value.get("secondFile"))?;
        Ok(Self {
            lines,
            tokens,
            first_name,
            first_start,
            second_name,
            second_start,
        })
    }
}

fn file_location(value: Option<&Value>) -> Result<(String, u64), String> {
    let value = value.ok_or_else(|| {
        "expected each duplicate occurrence to have a file name and start line".to_owned()
    })?;
    let name = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| {
            "expected each duplicate occurrence to have a file name and start line".to_owned()
        })?;
    let start = value.get("start").and_then(Value::as_u64).ok_or_else(|| {
        "expected each duplicate occurrence to have a file name and start line".to_owned()
    })?;
    Ok((name.to_owned(), start))
}

fn non_negative_integer(value: &Value, field: &str) -> Result<u64, String> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("expected statistics.total.{field} to be a non-negative integer"))
}

fn escape_markdown(value: &str) -> String {
    value.replace('|', "\\|").replace('`', "\\`")
}

pub fn shard_plan_json(plan: &ShardPlan) -> Value {
    let mut result = BTreeMap::new();
    result.insert("shardCount", Value::from(plan.shard_count));
    result.insert(
        "measuredSeconds",
        plan.measured_seconds.map_or(Value::Null, Value::from),
    );
    result.insert(
        "requiredShards",
        plan.required_shards.map_or(Value::Null, Value::from),
    );
    serde_json::to_value(result).expect("serializing a map of JSON values cannot fail")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_unguarded_commands_at_source_lines() {
        let source = "steps:\n  - run: cargo test --workspace\n  - run: |\n      prepare\n      cargo nextest run --workspace\n  - run: bash scripts/ci-run cargo test --workspace\n";
        assert_eq!(unguarded_rust_test_lines(source), vec![2, 5]);
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
}
