use super::*;

#[test]
fn bytecode_backend_runs_result_try_collections_and_function_values_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn parse(raw: String) -> Result<Int, {message: String}>\n",
        "  Ok(1)\n",
        "end\n",
        "fn stringify(value: Int) -> String\n",
        "  \"ok\"\n",
        "end\n",
        "fn entry_name(key: String, value: Int) -> String\n",
        "  key\n",
        "end\n",
        "fn keep_second(key: String, value: Int) -> Bool\n",
        "  key == \"second\"\n",
        "end\n",
        "fn latest_key(acc: String, key: String, value: Int) -> String\n",
        "  key\n",
        "end\n",
        "fn try_entry_name(key: String, value: Int) -> Result<String, {message: String}>\n",
        "  Ok(key)\n",
        "end\n",
        "pub fn main(raw: String) -> Result<(), {message: String}> effects [stdio]\n",
        "  let value: Int = parse(raw)?\n",
        "  let mapped: Vec<String> = vec_map([value], stringify)\n",
        "  let table: Dict<String, Int> = {\"first\": 1, \"second\": 2}\n",
        "  let dict_mapped: Dict<String, String> = dict_map(table, entry_name)\n",
        "  let dict_filtered: Dict<String, Int> = dict_filter(table, keep_second)\n",
        "  let folded: String = dict_fold(table, \"\", latest_key)\n",
        "  let tried: Result<Dict<String, String>, {message: String}> = dict_try_map(table, try_entry_name)\n",
        "  let message: String = match dict_get(dict_mapped, \"second\")\n",
        "    Some(found) => found\n",
        "    None => \"missing\"\n",
        "  end\n",
        "  let filtered_message: String = match dict_contains(dict_filtered, \"second\")\n",
        "    true => \"kept\"\n",
        "    false => \"missing\"\n",
        "  end\n",
        "  let tried_message: String = match tried\n",
        "    Ok(values) => match dict_get(values, \"first\")\n",
        "      Some(found) => found\n",
        "      None => \"missing\"\n",
        "    end\n",
        "    Err(error) => error.message\n",
        "  end\n",
        "  stdio::println(message)\n",
        "  stdio::println(folded)\n",
        "  stdio::println(filtered_message)\n",
        "  stdio::println(tried_message)\n",
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
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "second\nsecond\nkept\nfirst\n"
    );
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
fn bytecode_backend_runs_dict_callback_aliases_when_java_is_available() {
    let ir = lower_to_ir(concat!(
        "fn label(context: String, key: String, value: Int) -> String\n",
        "  context\n",
        "end\n",
        "fn keep(context: Int, key: String, value: Int) -> Bool\n",
        "  value == context\n",
        "end\n",
        "fn fold_label(context: String, acc: String, key: String, value: Int) -> String\n",
        "  context\n",
        "end\n",
        "fn try_label(context: String, key: String, value: Int) -> Result<String, String>\n",
        "  match key == \"second\"\n",
        "    true => Err(context)\n",
        "    false => Ok(context)\n",
        "  end\n",
        "end\n",
        "pub fn main() -> () effects [stdio]\n",
        "  let table: Dict<String, Int> = {\"first\": 1, \"second\": 2}\n",
        "  let mapped: Dict<String, String> = dict_map_with(\"mapped\", table, label)\n",
        "  let filtered: Dict<String, Int> = dict_filter_with(2, table, keep)\n",
        "  let folded: String = dict_fold_with(\"folded\", table, \"\", fold_label)\n",
        "  let tried: Result<Dict<String, String>, String> = dict_try_map_with(\"err\", table, try_label)\n",
        "  match dict_get(mapped, \"second\")\n",
        "    Some(found) => stdio::println(found)\n",
        "    None => stdio::println(\"missing\")\n",
        "  end\n",
        "  match dict_contains(filtered, \"first\")\n",
        "    true => stdio::println(\"first\")\n",
        "    false => stdio::println(\"no-first\")\n",
        "  end\n",
        "  stdio::println(folded)\n",
        "  match tried\n",
        "    Ok(_) => stdio::println(\"ok\")\n",
        "    Err(error) => stdio::println(error)\n",
        "  end\n",
        "end\n",
    ));
    let program = generate_classfiles_with_entry(&ir, "main");

    let Some(output) =
        run_jvm_program_when_java_is_available("bytecode-dict-callback-aliases", &program, &[])
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
        "mapped\nno-first\nfolded\nerr\n"
    );
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
