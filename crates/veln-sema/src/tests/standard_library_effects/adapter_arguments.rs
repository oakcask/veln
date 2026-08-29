use super::*;

#[test]
fn compiler_adapter_helpers_report_direct_argument_diagnostics() {
    for (helper, source_text, expected_message) in [
        (
            "vec_is_empty",
            concat!(
                "pub fn main(value: Int) -> Bool\n",
                "  vec_is_empty(value)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "vec_push",
            concat!(
                "pub fn main(value: Int) -> Vec<Int>\n",
                "  vec_push(value, 1)\n",
                "end\n",
            ),
            "expected `Vec<Int>`, but found `Int`",
        ),
        (
            "vec_concat",
            concat!(
                "pub fn main(value: Int, other: Vec<Int>) -> Vec<Int>\n",
                "  vec_concat(value, other)\n",
                "end\n",
            ),
            "expected `Vec<Int>`, but found `Int`",
        ),
        (
            "vec_map",
            concat!(
                "fn stringify(value: Int) -> String\n",
                "  \"ok\"\n",
                "end\n",
                "pub fn main(value: Int) -> Vec<String>\n",
                "  vec_map(value, stringify)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "vec_try_map",
            concat!(
                "fn stringify(value: Int) -> Result<String, String>\n",
                "  Ok(\"ok\")\n",
                "end\n",
                "pub fn main(value: Int) -> Result<Vec<String>, String>\n",
                "  vec_try_map(value, stringify)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "vec_try_map_with",
            concat!(
                "fn stringify(context: String, value: Int) -> Result<String, String>\n",
                "  Ok(context)\n",
                "end\n",
                "pub fn main(value: Int) -> Result<Vec<String>, String>\n",
                "  vec_try_map_with(\"prefix\", value, stringify)\n",
                "end\n",
            ),
            "expected `Vec<unknown>`, but found `Int`",
        ),
        (
            "list_is_empty",
            concat!(
                "type List<A>\n",
                "  Nil\n",
                "  Cons(head: A, tail: List<A>)\n",
                "end\n",
                "pub fn main(value: Int) -> Bool\n",
                "  list_is_empty(value)\n",
                "end\n",
            ),
            "expected `List<unknown>`, but found `Int`",
        ),
        (
            "list_map",
            concat!(
                "type List<A>\n",
                "  Nil\n",
                "  Cons(head: A, tail: List<A>)\n",
                "end\n",
                "fn stringify(value: Int) -> String\n",
                "  \"ok\"\n",
                "end\n",
                "pub fn main(value: Int) -> List<String>\n",
                "  list_map(value, stringify)\n",
                "end\n",
            ),
            "expected `List<unknown>`, but found `Int`",
        ),
        (
            "list_try_map",
            concat!(
                "type List<A>\n",
                "  Nil\n",
                "  Cons(head: A, tail: List<A>)\n",
                "end\n",
                "fn stringify(value: Int) -> Result<String, String>\n",
                "  Ok(\"ok\")\n",
                "end\n",
                "pub fn main(value: Int) -> Result<List<String>, String>\n",
                "  list_try_map(value, stringify)\n",
                "end\n",
            ),
            "expected `List<unknown>`, but found `Int`",
        ),
        (
            "dict_get",
            concat!(
                "pub fn main(value: Int) -> Option<String>\n",
                "  dict_get(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict<unknown, String>`, but found `Int`",
        ),
        (
            "dict_contains",
            concat!(
                "pub fn main(value: Int) -> Bool\n",
                "  dict_contains(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict<unknown, unknown>`, but found `Int`",
        ),
        (
            "dict_insert",
            concat!(
                "pub fn main(value: Int) -> Dict<String, Int>\n",
                "  dict_insert(value, \"key\", 1)\n",
                "end\n",
            ),
            "expected `Dict<String, Int>`, but found `Int`",
        ),
        (
            "dict_remove",
            concat!(
                "pub fn main(value: Int) -> Dict<String, Int>\n",
                "  dict_remove(value, \"key\")\n",
                "end\n",
            ),
            "expected `Dict<String, Int>`, but found `Int`",
        ),
        (
            "int_to_string",
            concat!(
                "pub fn main(value: String) -> String\n",
                "  int_to_string(value)\n",
                "end\n",
            ),
            "expected `Int`, but found `String`",
        ),
        (
            "string_parse_int",
            concat!(
                "pub fn main(value: Int) -> Result<Int, String>\n",
                "  string_parse_int(value)\n",
                "end\n",
            ),
            "expected `String`, but found `Int`",
        ),
        (
            "string_split_once",
            concat!(
                "pub fn main(value: Int) -> Option<{left: String, right: String}>\n",
                "  string_split_once(value, \",\")\n",
                "end\n",
            ),
            "expected `String`, but found `Int`",
        ),
    ] {
        assert_helper_user_call_site_type_mismatch(helper, source_text, expected_message);
    }
}

fn assert_helper_user_call_site_type_mismatch(
    helper: &str,
    source_text: &'static str,
    expected_message: &'static str,
) {
    let source = SourceFile::new("main.veln", source_text);
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1, "{helper}");
    assert_eq!(diagnostics[0].id, "type.mismatch", "{helper}");
    assert_eq!(diagnostics[0].message, expected_message, "{helper}");
    let span = diagnostics[0]
        .span
        .as_ref()
        .expect("diagnostic should point at user source");
    assert_eq!(span.file.as_str(), "main.veln");
}

#[test]
fn flows_call_argument_expected_type_into_holes() {
    let source = SourceFile::new(
        "main.veln",
        concat!(
            "fn consume(value: Float) -> ()\n",
            "  ()\n",
            "end\n",
            "pub fn main() -> ()\n",
            "  consume(_)\n",
            "end\n",
        ),
    );
    let parsed = parse(&source);
    let module = lower_surface_ast(&parsed.tree);

    let diagnostics = analyze_surface_module(&module);

    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].id, "hole.unfilled");
    assert!(
        diagnostics[0]
            .details
            .to_json()
            .contains("\"expected_type\":\"Float\"")
    );
    assert_eq!(diagnostics[0].related.len(), 1);
}
