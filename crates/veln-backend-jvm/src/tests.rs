use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::classfile::{TailRecursionEligibility, classify_tail_recursion};
use crate::java::{
    java_type_identifier, sanitize_identifier_text, unique_java_identifier,
    veln_string_literal_value,
};
use crate::runtime::{concurrency_method, prelude_method, standard_library_method, stdio_method};
use crate::*;
use veln_ast::lower_surface_ast;
use veln_ir::TypedProgram;
use veln_sema::lower_checked_surface_module;
use veln_source::SourceFile;
use veln_syntax::parse;

static NEXT_TEST_DIR: AtomicUsize = AtomicUsize::new(0);

const RUNTIME_LIST_HARNESS: &str = r#"
public final class RuntimeListHarness {
    private static int foldCalls = 0;
    private static int tryCalls = 0;

    public static void main(String[] args) {
        Object values = VelnRuntime.listNil();
        for (int index = 0; index < 20000; index += 1) {
            values = VelnRuntime.listCons(Long.valueOf(1), values);
        }
        Object reversed = VelnRuntime.listReverse(values);
        Object total = VelnRuntime.listFold(reversed, Long.valueOf(0), new VelnRuntime.Fn() {
            public Object call(Object... args) {
                foldCalls += 1;
                return Long.valueOf(((Long) args[0]).longValue() + ((Long) args[1]).longValue());
            }
        });
        Object kept = VelnRuntime.listFilter(reversed, new VelnRuntime.Fn() {
            public Object call(Object... args) {
                return Boolean.TRUE;
            }
        });
        Object tried = VelnRuntime.listTryMap(
            VelnRuntime.listCons(
                Long.valueOf(1),
                VelnRuntime.listCons(Long.valueOf(2), VelnRuntime.listCons(Long.valueOf(3), VelnRuntime.listNil()))
            ),
            new VelnRuntime.Fn() {
                public Object call(Object... args) {
                    tryCalls += 1;
                    if (((Long) args[0]).longValue() == 2L) {
                        return VelnRuntime.err("stop");
                    }
                    return VelnRuntime.ok(args[0]);
                }
            }
        );
        System.out.println(
            total
                + ":"
                + foldCalls
                + ":"
                + VelnRuntime.listIsEmpty(VelnRuntime.listNil())
                + ":"
                + VelnRuntime.listIsEmpty(kept)
                + ":"
                + tryCalls
                + ":"
                + tried
        );
    }
}
"#;

const PUBLIC_LIST_HELPER_HARNESS: &str = r#"
public final class PublicListHelperHarness {
    public static void main(String[] args) {
        Object values = VelnRuntime.listNil();
        for (int index = 0; index < 20000; index += 1) {
            values = VelnRuntime.listCons(Long.valueOf(1), values);
        }
        VelnProgram.fn_consume(values);
    }
}
"#;

const RUNTIME_PATH_HARNESS: &str = r#"
public final class RuntimePathHarness {
    public static void main(String[] args) {
        Object cwd = ((VelnRuntime.Result) VelnRuntime.processCwd()).value();
        System.out.println(VelnRuntime.fsExists(cwd));

        Object entries = ((VelnRuntime.Result) VelnRuntime.fsReadDir(cwd)).value();
        Object first = ((java.util.List<?>) entries).get(0);
        System.out.println(VelnRuntime.fsExists(first));
    }
}
"#;

#[test]
fn bytecode_backend_emits_classfiles_without_java_sources() {
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");

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
    let ir = lower_to_ir("pub fn main() -> String\n  \"ok\"\nend\n");
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
        "fn parse(raw: String) -> Result<Int, {message: String}>\n",
        "  Ok(1)\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  \"ok\"\n",
        "end\n",
        "pub fn main(raw: String) -> Result<(), {message: String}> effects [stdio]\n",
        "  let value: Int = parse(raw)?\n",
        "  let mapped: Vec<String> = vec_map([value], stringify)\n",
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
fn bytecode_backend_runs_minimal_list_adt_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn sum(values: List<Int>) -> Int\n",
        "  match values\n",
        "    Nil => 0\n",
        "    Cons(head, tail) => head + sum(tail)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(sum(Cons(1, Cons(2, Nil)))))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) = run_jvm_program_when_java_is_available("bytecode-list-adt", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\n");
}

#[test]
fn bytecode_backend_runs_vec_try_map_with_context_and_error_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn attach(context: String, value: Int) -> Result<{prefix: String, value: Int}, String>\n",
        "  Ok({prefix: context, value: value})\n",
        "end\n",
        "fn stop_at_two(context: String, value: Int) -> Result<{prefix: String, value: Int}, String>\n",
        "  match value == 2\n",
        "    true => Err(context)\n",
        "    false => match value == 3\n",
        "      true => Err(\"later\")\n",
        "      false => Ok({prefix: context, value: value})\n",
        "    end\n",
        "  end\n",
        "end\n",
        "fn add_value(total: Int, item: {prefix: String, value: Int}) -> Int\n",
        "  total + item.value\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let mapped: Result<Vec<{prefix: String, value: Int}>, String> = vec_try_map_with(\"ctx\", [1, 2], attach)\n",
        "  let stopped: Result<Vec<{prefix: String, value: Int}>, String> = vec_try_map_with(\"ctx\", [1, 2, 3], stop_at_two)\n",
        "  match mapped\n",
        "    Ok(items) => stdio::println(int_to_string(vec_fold(items, 0, add_value)))\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "  match stopped\n",
        "    Ok(_) => stdio::println(\"unexpected\")\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-vec-try-map-with", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "3\nctx\n");
}

#[test]
fn bytecode_backend_runs_list_helpers_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn add(total: Int, value: Int) -> Int\n",
        "  total + value\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  int_to_string(value)\n",
        "end\n",
        "fn keep_large(value: Int) -> Bool\n",
        "  value > 1\n",
        "end\n",
        "fn stop_at_two(value: Int) -> Result<String, String>\n",
        "  match value == 2\n",
        "    true => Err(\"stop\")\n",
        "    false => match value == 3\n",
        "      true => Err(\"later\")\n",
        "      false => Ok(int_to_string(value))\n",
        "    end\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let values: List<Int> = list_cons(1, list_cons(2, list_cons(3, list_nil())))\n",
        "  stdio::println(int_to_string(list_fold(values, 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(list_reverse(values), 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(list_filter(values, keep_large), 0, add)))\n",
        "  stdio::println(match list_try_map(values, stop_at_two)\n",
        "    Ok(_) => \"unexpected\"\n",
        "    Err(error) => error\n",
        "  end)\n",
        "  stdio::println(match list_map(values, stringify)\n",
        "    Nil => \"empty\"\n",
        "    Cons(head, _) => head\n",
        "  end)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-list-helpers", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "6\n6\n5\nstop\n1\n"
    );
}

#[test]
fn bytecode_backend_runs_deep_tail_recursion_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn countdown(value: Int) -> Int\n",
        "require value >= 0\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => countdown(value - 1)\n",
        "  end\n",
        "end\n",
        "fn nested_countdown(value: Int, active: Bool) -> Int\n",
        "  match active\n",
        "    true => match value\n",
        "      0 => 0\n",
        "      _ => nested_countdown(value - 1, true)\n",
        "    end\n",
        "    false => 0\n",
        "  end\n",
        "end\n",
        "fn pair_step(first: Int, second: Int, steps: Int) -> Int\n",
        "  match steps\n",
        "    0 => first\n",
        "    _ => pair_step(second, first + second, steps - 1)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  stdio::println(int_to_string(countdown(30000)))\n",
        "  stdio::println(int_to_string(nested_countdown(30000, true)))\n",
        "  stdio::println(int_to_string(pair_step(0, 1, 10)))\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-tail-recursion", &program, &[])
    else {
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "0\n0\n55\n");
}

#[test]
fn bytecode_backend_rechecks_require_contracts_inside_tail_recursion_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn countdown(value: Int) -> Int\n",
        "require value != 2\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => countdown(value - 1)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Int\n",
        "  countdown(4)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-tail-recursion-require", &program, &[])
    else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: require `value != 2`"));
    assert!(stderr.contains("blame caller"));
}

#[test]
fn bytecode_backend_verifies_all_tail_match_recursion_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn reject(value: Int) -> Int\n",
        "require false\n",
        "  match value\n",
        "    _ => reject(value)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> Int\n",
        "  reject(0)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-tail-recursion-all-tail", &program, &[])
    else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: require `false`"));
}

#[test]
fn bytecode_backend_classifies_tail_recursion_conservatively() {
    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn countdown(value: Int) -> Int\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => countdown(value - 1)\n",
        "  end\n",
        "end\n",
        "fn length(items: List<Int>) -> Int\n",
        "  match items\n",
        "    Nil => 0\n",
        "    Cons(_, tail) => 1 + length(tail)\n",
        "  end\n",
        "end\n",
        "fn checked(value: Int) -> result: Int\n",
        "ensure result >= 0\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => checked(value - 1)\n",
        "  end\n",
        "end\n",
        "fn through_value(callback: fn(Int) -> Int, value: Int) -> Int\n",
        "  match value\n",
        "    0 => 0\n",
        "    _ => through_value(callback, callback(value - 1))\n",
        "  end\n",
        "end\n",
    ));

    let function = |name: &str| {
        ir.functions
            .iter()
            .find(|function| function.name == name)
            .expect("function should exist")
    };

    assert_eq!(
        classify_tail_recursion(function("countdown")),
        TailRecursionEligibility::Eligible
    );
    assert_eq!(
        classify_tail_recursion(function("length")),
        TailRecursionEligibility::NonTailSelfCall
    );
    assert_eq!(
        classify_tail_recursion(function("checked")),
        TailRecursionEligibility::RuntimeReturnContract
    );
    assert_eq!(
        classify_tail_recursion(function("through_value")),
        TailRecursionEligibility::IndirectValueCall
    );
}

#[test]
fn jvm_runtime_preserves_path_values_across_standard_calls_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-path-values");
    write_jvm_program(&root, &program);
    fs::write(root.join("RuntimePathHarness.java"), RUNTIME_PATH_HARNESS)
        .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimePathHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "{}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimePathHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Ok(true)\nOk(true)\n"
    );
}

#[test]
fn jvm_runtime_list_helpers_traverse_large_lists_iteratively_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-list-helpers");
    write_jvm_program(&root, &program);
    fs::write(root.join("RuntimeListHarness.java"), RUNTIME_LIST_HARNESS)
        .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeListHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "{}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeListHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "20000:20000:true:false:2:Err(stop)\n"
    );
}

#[test]
fn bytecode_backend_public_list_helpers_traverse_large_lists_iteratively_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir(concat!(
        "type List<A>\n",
        "  Nil\n",
        "  Cons(head: A, tail: List<A>)\n",
        "end\n",
        "fn add(total: Int, value: Int) -> Int\n",
        "  total + value\n",
        "end\n",
        "fn double(value: Int) -> Int\n",
        "  value * 2\n",
        "end\n",
        "fn keep_one(value: Int) -> Bool\n",
        "  value == 1\n",
        "end\n",
        "fn ok_next(value: Int) -> Result<Int, String>\n",
        "  Ok(value + 1)\n",
        "end\n",
        "fn stop_at_two(value: Int) -> Result<Int, String>\n",
        "  match value == 2\n",
        "    true => Err(\"stop\")\n",
        "    false => Ok(value)\n",
        "  end\n",
        "end\n",
        "pub fn consume(values: List<Int>) -> () effects [stdio]\n",
        "  let mapped: List<Int> = list_map(values, double)\n",
        "  let tried: Result<List<Int>, String> = list_try_map(values, ok_next)\n",
        "  stdio::println(int_to_string(list_fold(values, 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(mapped, 0, add)))\n",
        "  stdio::println(int_to_string(list_fold(list_filter(values, keep_one), 0, add)))\n",
        "  match tried\n",
        "    Ok(items) => stdio::println(int_to_string(list_fold(items, 0, add)))\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "  stdio::println(match list_try_map(list_cons(1, list_cons(2, values)), stop_at_two)\n",
        "    Ok(_) => \"unexpected\"\n",
        "    Err(error) => error\n",
        "  end)\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "consume");
    let root = temp_dir("public-list-helpers");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("PublicListHelperHarness.java"),
        PUBLIC_LIST_HELPER_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("PublicListHelperHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "{}",
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("PublicListHelperHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "20000\n40000\n20000\n40000\nstop\n"
    );
}

#[test]
fn bytecode_backend_runs_task_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn produce() -> String\n",
        "  \"hello\"\n",
        "end\n",
        "pub fn main() -> Result<(), JoinError> effects [stdio, concurrency]\n",
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
        "pub fn main(value: Int) -> output: Int\n",
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
fn bytecode_backend_entry_invariant_failure_blames_caller_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "pub fn main(value: Bool) -> Bool\n",
        "invariant value\n",
        "  value\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Bool]);

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-entry-invariant-failure",
        &program,
        &["false"],
    ) else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: invariant `value`"));
    assert!(stderr.contains("blame caller"));
}

#[test]
fn bytecode_backend_return_invariant_failure_blames_implementation_when_java_is_available() {
    let mut ir = lower_to_ir(concat!(
        "pub fn main(value: Bool) -> Bool\n",
        "invariant value\n",
        "  false\n",
        "end\n",
    ));
    // Exercise the return-position bytecode path directly; surface analysis
    // rejects result bindings that duplicate parameter names.
    ir.functions[0].return_binding = Some("value".to_string());
    let program = generate_classfiles_with_entry_arg_types(&ir, "main", &[EntryArgType::Bool]);

    let Some(output) = run_jvm_program_when_java_is_available(
        "bytecode-return-invariant-failure",
        &program,
        &["true"],
    ) else {
        return;
    };

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("contract failure: invariant `value`"));
    assert!(stderr.contains("blame implementation"));
}

#[test]
fn bytecode_backend_javap_reports_target_version_and_entry_descriptor_when_available() {
    if Command::new("javap").arg("-version").output().is_err() {
        return;
    }

    let ir = lower_to_ir("pub fn main(value: String) -> String\n  value\nend\n");
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
        ("list_nil", "listNil"),
        ("list_cons", "listCons"),
        ("list_is_empty", "listIsEmpty"),
        ("list_fold", "listFold"),
        ("list_reverse", "listReverse"),
        ("list_map", "listMap"),
        ("list_filter", "listFilter"),
        ("list_try_map", "listTryMap"),
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
