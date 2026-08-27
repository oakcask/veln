use super::*;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

mod byte;
mod protocol;
mod routing;
mod value_diagnostics;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn run_generation_excludes_companion_sources() {
    let root = temp_dir("run-generation-excludes-companion-sources");
    fs::write(root.join("main.veln"), "pub fn main() -> Int\n\t1\nend\n")
        .expect("production source should be written");
    fs::write(
        root.join("main.test.veln"),
        "pub fn companion_marker() -> Int\n\t2\nend\n",
    )
    .expect("companion source should be written");

    let analysis_inputs =
        production_analysis_inputs(&root, &[]).expect("production inputs should resolve");
    assert_eq!(analysis_inputs.len(), 1);
    assert!(analysis_inputs[0].ends_with("main.veln"));
    let project = Project::discover(root.clone(), &analysis_inputs)
        .expect("production project should discover");
    let analysis = veln_analysis::analyze_project(project, DoctestMode::Exclude);
    assert!(
        analysis.checked_diagnostics().is_empty(),
        "production analysis should exclude companion diagnostics: {:#?}",
        analysis.checked_diagnostics()
    );
    let ir = lower_run_entry(false, &analysis, "main", None)
        .expect("entry should lower")
        .expect("entry should produce IR");

    let jvm = generate_classfiles_with_entry_arg_types(&ir, "main", &[]);

    assert!(
        jvm.classes
            .iter()
            .any(|class| class.path == "VelnProgram$fn_main.class")
    );
    assert!(
        jvm.classes
            .iter()
            .all(|class| !class.path.contains("companion_marker")),
        "companion function should not be emitted in run classfiles: {:?}",
        jvm.classes
            .iter()
            .map(|class| class.path.as_str())
            .collect::<Vec<_>>()
    );

    fs::remove_dir_all(root).expect("test project should be removed");
}

#[test]
fn run_analysis_timings_write_deterministic_json_lines() {
    let root = temp_dir("run-analysis-timings-json-lines");
    let timing_file = root.join("timings.jsonl");
    let mut timings = RunAnalysisTimings {
        file: timing_file.clone(),
        workload: "http2_core".to_string(),
        run: "new-1".to_string(),
        records: Vec::new(),
    };

    timings.push("source_loading", Duration::from_nanos(250_000_000));
    timings.write().expect("timing records should be written");

    assert_eq!(
        fs::read_to_string(&timing_file).expect("timing file should be readable"),
        "{\"workload\":\"http2_core\",\"run\":\"new-1\",\"stage\":\"source_loading\",\"boundary\":\"source_loading\",\"duration_seconds\":0.250000000}\n"
    );

    fs::remove_dir_all(root).expect("test project should be removed");
}

fn byte_preview(data: &str) -> JsonValue {
    byte_preview_with_counts(data, (data.len() / 2) as i64, false)
}

fn temp_dir(name: &str) -> PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = env::temp_dir().join(format!("veln-cli-{name}-{nanos}-{id}"));
    fs::create_dir_all(&root).expect("test directory should be created");
    root
}

fn byte_preview_with_counts(data: &str, total_byte_count: i64, truncated: bool) -> JsonValue {
    let preview_byte_count = (data.len() / 2) as i64;
    JsonValue::object([
        ("encoding", JsonValue::string("hex")),
        ("data", JsonValue::string(data)),
        ("preview_byte_count", JsonValue::Number(preview_byte_count)),
        ("total_byte_count", JsonValue::Number(total_byte_count)),
        ("truncated", JsonValue::Bool(truncated)),
    ])
}
