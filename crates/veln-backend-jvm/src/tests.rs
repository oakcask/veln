use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::java::{
    concurrency_method, java_type_identifier, prelude_method, sanitize_identifier_text,
    standard_library_method, stdio_method, unique_java_identifier, veln_string_literal_value,
};
use crate::*;
use veln_ast::lower_surface_ast;
use veln_ir::TypedProgram;
use veln_sema::lower_checked_surface_module;
use veln_source::SourceFile;
use veln_syntax::parse;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn bytecode_backend_emits_classfiles_without_java_sources() {
    let ir = lower_to_ir("pub fn main() -> () effects []\n  ()\nend\n");

    let program = generate_classfiles_with_entry(&ir, "main");

    assert!(program.class("VelnEntry.class").is_some());
    assert!(program.class("VelnProgram.class").is_some());
    assert!(program.class("VelnRuntime.class").is_some());
    assert!(
        program
            .classes
            .iter()
            .all(|class| class.path.ends_with(".class"))
    );
}

#[test]
fn bytecode_backend_sanitizes_custom_program_class_name() {
    let ir = lower_to_ir("pub fn main() -> String effects []\n  \"ok\"\nend\n");
    let program = generate_classfiles_with_entry_arg_types_options(
        &ir,
        "main",
        &[],
        &JvmBackendOptions {
            program_class: "9 bad-name".to_string(),
        },
    );

    assert!(program.class("_9_bad_name.class").is_some());
    assert!(program.class("_9_bad_name$fn_main.class").is_some());
    assert!(program.class("VelnEntry.class").is_some());
}

#[test]
fn bytecode_backend_classfiles_run_when_java_is_available() {
    let ir = lower_to_ir("pub fn main() -> () effects [stdio]\n  stdio::println(\"ok\")\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-run", &program, &[]) else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn bytecode_backend_runs_result_try_collections_and_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn parse(raw: String) -> Result(Int, {message: String}) effects []\n",
        "  Ok(1)\n",
        "end\n",
        "fn stringify(value: Int) -> String effects []\n",
        "  \"ok\"\n",
        "end\n",
        "pub fn main(raw: String) -> Result((), {message: String}) effects [stdio]\n",
        "  let value: Int = parse(raw)?\n",
        "  let mapped: Vec(String) = vec_map([value], stringify)\n",
        "  let message: String = match dict_get({\"first\": \"bad\", \"second\": \"ok\"}, \"second\")\n",
        "    Some(found) => found\n",
        "    None => \"missing\"\n",
        "  end\n",
        "  stdio::println(message)\n",
        "  match vec_len(mapped) == 1\n",
        "    true => Ok(())\n",
        "    false => Err({message: \"bad\"})\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-result-collections", &program, &["raw"])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "ok\n");
}

#[test]
fn bytecode_backend_runs_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn produce() -> String effects []\n",
        "  \"hello\"\n",
        "end\n",
        "pub fn main() -> Result((), JoinError) effects [stdio, concurrency]\n",
        "  let task = task::spawn(produce)\n",
        "  let value: String = task::join(task)?\n",
        "  stdio::println(value)\n",
        "  Ok(())\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-task", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "hello\n");
}

#[test]
fn bytecode_backend_entry_reports_contract_failures_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main(value: Int) -> output: Int effects []\n",
        "  ensure output > 0\n",
        "  value\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Int]);

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-contract-failure", &program, &["0"])
    else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: ensure `output > 0`"));
    assert!(stderr.contains("blame implementation"));
}

#[test]
fn bytecode_backend_javap_reports_target_version_and_entry_descriptor_when_available() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir("pub fn main(value: String) -> String effects []\n  value\nend\n");
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::String]);
    let root = temp_dir("bytecode-javap");
    write_jvm_program(&root, &program);

    let output = Command::new("javap")
        .arg("-verbose")
        .arg("-classpath")
        .arg(&root)
        .arg("VelnEntry")
        .output()
        .expect("javap should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("major version: 49"));
    assert!(stdout.contains("public static void main(java.lang.String[]);"));
    assert!(stdout.contains("descriptor: ([Ljava/lang/String;)V"));
}

#[test]
fn java_identifier_helpers_sanitize_keywords_and_collisions() {
    let mut used_names = std::collections::BTreeSet::new();

    assert_eq!(sanitize_identifier_text("1-value!"), "_1_value_");
    assert_eq!(java_type_identifier("class"), "VelnGenerated");
    assert_eq!(java_type_identifier("app.Main"), "app_Main");
    assert_eq!(unique_java_identifier("", &mut used_names), "_value");
    assert_eq!(unique_java_identifier("return", &mut used_names), "_return");
    assert_eq!(unique_java_identifier("value", &mut used_names), "value");
    assert_eq!(unique_java_identifier("value", &mut used_names), "value_1");
}

#[test]
fn java_method_name_helpers_map_builtin_surface_names() {
    for (surface, method) in [
        ("stdio::print", "stdioPrint"),
        ("stdio::println", "stdioPrintln"),
        ("stdio::eprint", "stdioEprint"),
        ("stdio::eprintln", "stdioEprintln"),
    ] {
        assert_eq!(stdio_method(surface), method);
    }

    for (surface, method) in [
        ("float_negate", "floatNegate"),
        ("float_add", "floatAdd"),
        ("float_subtract", "floatSubtract"),
        ("float_multiply", "floatMultiply"),
        ("float_divide", "floatDivide"),
        ("float_less", "floatLess"),
        ("float_less_equal", "floatLessEqual"),
        ("float_greater", "floatGreater"),
        ("float_greater_equal", "floatGreaterEqual"),
        ("string_split_once", "stringSplitOnce"),
        ("string_parse_int", "stringParseInt"),
        ("int_to_string", "intToString"),
        ("vec_len", "vecLen"),
        ("vec_is_empty", "vecIsEmpty"),
        ("vec_push", "vecPush"),
        ("vec_concat", "vecConcat"),
        ("vec_map", "vecMap"),
        ("vec_filter", "vecFilter"),
        ("vec_fold", "vecFold"),
        ("vec_try_map", "vecTryMap"),
        ("vec_try_map_with", "vecTryMapWith"),
        ("dict_get", "dictGet"),
        ("dict_contains", "dictContains"),
        ("dict_insert", "dictInsert"),
        ("dict_remove", "dictRemove"),
        ("option_map", "optionMap"),
        ("option_and_then", "optionAndThen"),
        ("option_unwrap_or", "optionUnwrapOr"),
        ("result_map", "resultMap"),
        ("result_map_err", "resultMapErr"),
        ("result_and_then", "resultAndThen"),
    ] {
        assert_eq!(prelude_method(surface), method);
    }

    for (surface, method) in [
        ("channel::bounded", "channelBounded"),
        ("channel::clone", "channelClone"),
        ("channel::send", "channelSend"),
        ("channel::recv", "channelRecv"),
        ("channel::select", "channelSelect"),
        ("channel::select_priority", "channelSelectPriority"),
        ("channel::select_timeout", "channelSelectTimeout"),
        ("channel::select_result", "channelSelectResult"),
        (
            "channel::select_priority_result",
            "channelSelectPriorityResult",
        ),
        (
            "channel::select_timeout_result",
            "channelSelectTimeoutResult",
        ),
        ("channel::close", "channelClose"),
        ("task::spawn", "taskSpawn"),
        ("task::join", "taskJoin"),
        ("task::cancel", "taskCancel"),
    ] {
        assert_eq!(concurrency_method(surface), method);
    }

    for (surface, method) in [
        ("fs::read_to_string", "fsReadToString"),
        ("fs::write_string", "fsWriteString"),
        ("fs::exists", "fsExists"),
        ("fs::read_dir", "fsReadDir"),
        ("process::args", "processArgs"),
        ("process::env", "processEnv"),
        ("process::cwd", "processCwd"),
        ("process::exit", "processExit"),
    ] {
        assert_eq!(standard_library_method(surface), method);
    }

    let panic = std::panic::catch_unwind(|| standard_library_method("fs::unknown"));
    assert!(panic.is_err());
}

#[test]
fn veln_string_literal_value_decodes_known_escapes_and_preserves_unknown_ones() {
    assert_eq!(
        veln_string_literal_value("\"line\\nquote\\\"slash\\\\tab\\t\""),
        "line\nquote\"slash\\tab\t"
    );
    assert_eq!(veln_string_literal_value("\"unknown\\q\""), "unknown\\q");
    assert_eq!(veln_string_literal_value("\"trailing\\\""), "trailing\\");
    assert_eq!(veln_string_literal_value("raw"), "raw");
}

fn lower_to_ir(text: &str) -> TypedProgram {
    let source = SourceFile::new("main.veln", text);
    let parsed = parse(&source);
    assert!(
        parsed.diagnostics.is_empty(),
        "parse diagnostics: {:#?}",
        parsed.diagnostics
    );
    let module = lower_surface_ast(&parsed.tree);
    let lowered = lower_checked_surface_module(&module);
    assert!(
        lowered.diagnostics.is_empty(),
        "semantic diagnostics: {:#?}",
        lowered.diagnostics
    );
    lowered.ir.expect("source should lower to typed IR")
}

fn temp_dir(name: &str) -> std::path::PathBuf {
    let id = NEXT_TEST_DIR.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "veln-backend-jvm-{name}-{}-{nanos}-{id}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("test directory should be created");
    root
}

fn run_jvm_program_when_java_is_available(
    name: &str,
    program: &JvmProgram,
    args: &[&str],
) -> Option<std::process::Output> {
    if Command::new("java").arg("-version").output().is_err() {
        return None;
    }

    let root = temp_dir(name);
    write_jvm_program(&root, program);

    let mut command = Command::new("java");
    command
        .arg("-cp")
        .arg(&root)
        .arg("VelnEntry")
        .current_dir(&root);
    for arg in args {
        command.arg(arg);
    }
    let output = command.output().expect("java should run");
    let _ = fs::remove_dir_all(&root);
    Some(output)
}

fn write_jvm_program(root: &std::path::Path, program: &JvmProgram) {
    for class in &program.classes {
        fs::write(root.join(&class.path), &class.contents).expect("classfile should be written");
    }
}
