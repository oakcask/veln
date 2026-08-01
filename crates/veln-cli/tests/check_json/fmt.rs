use super::support::*;

#[test]
fn fmt_formats_supported_golden_and_is_idempotent() {
    let project = TestProject::new("fmt-golden");
    project.write(
        "main.veln",
        concat!(
            "mod app\n",
            "use stdio\n",
            "pub   fn   main ( name : String ) -> Result < () , AppError > effects [ stdio ]\n",
            " require name != \"\"\n",
            " let payload : { message : String, values : Vec<Int> } = { message : name , values : [ 1 , 2 , add ( 3 , 4 ) ] }\n",
            " stdio::println ( payload )\n",
            " _result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "fn helper(value)\n",
            "value\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "mod app\n",
            "use stdio\n",
            "\n",
            "pub fn main(name: String) -> Result<(), AppError> effects [stdio]\n",
            "\trequire name != \"\"\n",
            "\tlet payload: { message : String, values : Vec<Int> } = { message: name, values: [1, 2, add(3, 4)] }\n",
            "\tstdio::println(payload)\n",
            "\t_result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "\n",
            "fn helper(value)\n",
            "\tvalue\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "mod app\n",
            "use stdio\n",
            "\n",
            "pub fn main(name: String) -> Result<(), AppError> effects [stdio]\n",
            "\trequire name != \"\"\n",
            "\tlet payload: { message : String, values : Vec<Int> } = { message: name, values: [1, 2, add(3, 4)] }\n",
            "\tstdio::println(payload)\n",
            "\t_result satisfy candidate => candidate != \"\"\n",
            "end\n",
            "\n",
            "fn helper(value)\n",
            "\tvalue\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_formats_focused_supported_forms_across_multiple_files() {
    let project = TestProject::new("fmt-focused-golden");
    project.write(
        "main.veln",
        concat!(
            "fn parse ( raw : String ) -> Result < Int , AppError >\n",
            " Ok ( 1 )\n",
            "end\n",
            "pub fn main ( raw : String ) -> Result < { value : Int, tags : Vec<String> } , AppError >\n",
            " ensure output.value >= 0 and not ( output.value == - 1 )\n",
            " let parsed : Int = parse ( raw ) ?\n",
            " { value : parsed + 1 * ( 2 + 3 ) , tags : [ choose ( raw , \"fallback\" ) , \"done\" ] }\n",
            "end\n",
        ),
    );
    project.write(
        "helpers.veln",
        concat!(
            "fn choose ( value : String , fallback : String ) -> String\n",
            " if_missing ( { primary : value, nested : { fallback : fallback } } )\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln", "helpers.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(stdout(&output), "");
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "\tOk(1)\n",
            "end\n",
            "\n",
            "pub fn main(raw: String) -> Result<{ value : Int, tags : Vec<String> }, AppError>\n",
            "\tensure output.value >= 0 and not(output.value == - 1)\n",
            "\tlet parsed: Int = parse(raw)?\n",
            "\t{ value: parsed + 1 * (2 + 3), tags: [choose(raw, \"fallback\"), \"done\"] }\n",
            "end\n",
        )
    );
    assert_eq!(
        project.read("helpers.veln"),
        concat!(
            "fn choose(value: String, fallback: String) -> String\n",
            "\tif_missing({ primary: value, nested: { fallback: fallback } })\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln", "helpers.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn parse(raw: String) -> Result<Int, AppError>\n",
            "\tOk(1)\n",
            "end\n",
            "\n",
            "pub fn main(raw: String) -> Result<{ value : Int, tags : Vec<String> }, AppError>\n",
            "\tensure output.value >= 0 and not(output.value == - 1)\n",
            "\tlet parsed: Int = parse(raw)?\n",
            "\t{ value: parsed + 1 * (2 + 3), tags: [choose(raw, \"fallback\"), \"done\"] }\n",
            "end\n",
        )
    );
    assert_eq!(
        project.read("helpers.veln"),
        concat!(
            "fn choose(value: String, fallback: String) -> String\n",
            "\tif_missing({ primary: value, nested: { fallback: fallback } })\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_canonicalizes_binary_schema_compatibility_primitives() {
    let project = TestProject::new("fmt-schema-primitives");
    project.write(
        "main.veln",
        concat!(
            "schema Wire\n",
            "  format binary\n",
            "  count: UInt8\n",
            "  flags: UInt16le\n",
            "  padding: ReservedBits( 16 , 43981 )\n",
            "  values: Repeat( count , UInt24be )\n",
            "  payload: Dispatch( count, 1 => UInt8, 2 => ReservedBits(16, 43981), 3 => UInt8 )\n",
            "  wrapped: List<UInt8>\n",
            "end\n",
            "\n",
            "schema Neutral\n",
            "  value: UInt8\n",
            "end\n",
        ),
    );

    let expected = concat!(
        "schema Wire\n",
        "\tformat binary\n",
        "\n",
        "\tcount: uint8\n",
        "\tflags: uint16le\n",
        "\tpadding: uint16be reserves 43981\n",
        "\tvalues: [uint24be; count]\n",
        "\tpayload: Dispatch(count, 1 => uint8, 2 => uint16be reserves 43981, 3 => uint8)\n",
        "\twrapped: List<UInt8>\n",
        "end\n",
        "\n",
        "schema Neutral\n",
        "\n",
        "\tvalue: UInt8\n",
        "end\n",
    );

    project.assert_fmt_idempotent(&["main.veln"], &[("main.veln", expected)]);
}

#[test]
fn fmt_formats_match_expressions_with_tab_relative_indentation() {
    let project = TestProject::new("fmt-match-indent");
    project.write(
        "main.veln",
        concat!(
            "fn describe ( value : Option<Int> ) -> String\n",
            " match value\n",
            " Some(count) => \"some\"\n",
            " None => \"none\"\n",
            " end\n",
            "end\n",
            "fn nested ( value : Option<Int> ) -> { labels : Vec<String>, primary : String }\n",
            " { labels : [ wrap ( match value\n",
            " Some(count) => \"some\"\n",
            " None => \"none\"\n",
            " end ) ], primary : match value\n",
            " Some(count) => \"some\"\n",
            " None => \"none\"\n",
            " end }\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn describe(value: Option<Int>) -> String\n",
            "\tmatch value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend\n",
            "end\n",
            "\n",
            "fn nested(value: Option<Int>) -> { labels : Vec<String>, primary : String }\n",
            "\t{ labels: [wrap(match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend)], primary: match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend }\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn describe(value: Option<Int>) -> String\n",
            "\tmatch value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend\n",
            "end\n",
            "\n",
            "fn nested(value: Option<Int>) -> { labels : Vec<String>, primary : String }\n",
            "\t{ labels: [wrap(match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend)], primary: match value\n",
            "\t\tSome(count) => \"some\"\n",
            "\t\tNone => \"none\"\n",
            "\tend }\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_rewrites_literal_equality_match_chains() {
    let project = TestProject::new("fmt-literal-match-chain");
    project.write(
        "main.veln",
        concat!(
            "fn display(value: String) -> String\n",
            " match value == \"\\n\"\n",
            " true => \"<lf>\"\n",
            " false => match value == \"hpack-byte-00\"\n",
            " true => \"<nul>\"\n",
            " false => value\n",
            " end\n",
            " end\n",
            "end\n",
            "fn provenance(fact: String) -> String\n",
            " match fact == \"content_length_invalid\" or fact == \"content_length_mismatch\"\n",
            " true => \"rfc9113_content_length\"\n",
            " false => \"rfc9113_request_pseudo_headers\"\n",
            " end\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn display(value: String) -> String\n",
            "\tmatch value\n",
            "\t\t\"\\n\" => \"<lf>\"\n",
            "\t\t\"hpack-byte-00\" => \"<nul>\"\n",
            "\t\t_ => value\n",
            "\tend\n",
            "end\n",
            "\n",
            "fn provenance(fact: String) -> String\n",
            "\tmatch fact\n",
            "\t\t\"content_length_invalid\" => \"rfc9113_content_length\"\n",
            "\t\t\"content_length_mismatch\" => \"rfc9113_content_length\"\n",
            "\t\t_ => \"rfc9113_request_pseudo_headers\"\n",
            "\tend\n",
            "end\n",
        )
    );

    let second_output = project.fmt(&["main.veln"]);

    assert!(second_output.status.success(), "{}", stderr(&second_output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "fn display(value: String) -> String\n",
            "\tmatch value\n",
            "\t\t\"\\n\" => \"<lf>\"\n",
            "\t\t\"hpack-byte-00\" => \"<nul>\"\n",
            "\t\t_ => value\n",
            "\tend\n",
            "end\n",
            "\n",
            "fn provenance(fact: String) -> String\n",
            "\tmatch fact\n",
            "\t\t\"content_length_invalid\" => \"rfc9113_content_length\"\n",
            "\t\t\"content_length_mismatch\" => \"rfc9113_content_length\"\n",
            "\t\t_ => \"rfc9113_request_pseudo_headers\"\n",
            "\tend\n",
            "end\n",
        )
    );
}

#[test]
fn fmt_rejects_unknown_flags_before_writing_files() {
    let project = TestProject::new("fmt-unknown-flag");
    let text = "fn   ok ( ) -> ()\n()\nend\n";
    project.write("main.veln", text);

    let output = project.fmt(&["--json", "main.veln"]);

    assert_eq!(output.status.code(), Some(2));
    assert_eq!(stdout(&output), "");
    assert_eq!(stderr(&output), "veln: unknown fmt flag `--json`\n");
    assert_eq!(project.read("main.veln"), text);
}

#[test]
fn fmt_preserves_files_when_any_input_has_parse_errors() {
    let project = TestProject::new("fmt-parse-error");
    project.write("bad.veln", "fn bad() -> ()\n  @\nend\n");
    project.write("good.veln", "fn   ok ( ) -> ()\n()\nend\n");

    let output = project.fmt(&["bad.veln", "good.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(project.read("bad.veln"), "fn bad() -> ()\n  @\nend\n");
    assert_eq!(project.read("good.veln"), "fn   ok ( ) -> ()\n()\nend\n");
    assert_contains_all(
        stderr(&output),
        &["bad.veln:2:3: error[parse.invalid_token]: invalid token in expression"],
    );
}

#[test]
fn fmt_formats_comment_bearing_files() {
    let project = TestProject::new("fmt-comments");
    let text = concat!(
        "# keep leading comment\n",
        "fn   main ( ) -> ()\n",
        "  () # keep trailing comment\n",
        "# keep closing comment\n",
        "end # keep end comment\n",
    );
    project.write("main.veln", text);

    let output = project.fmt(&["main.veln"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "# keep leading comment\n",
            "fn main() -> ()\n",
            "\t()  # keep trailing comment\n",
            "\t# keep closing comment\n",
            "end  # keep end comment\n",
        )
    );
}

#[test]
fn fmt_rejects_legacy_slash_comment_source() {
    let project = TestProject::new("fmt-slash-comments");
    project.write(
        "main.veln",
        concat!(
            "// keep leading comment\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        ),
    );

    let output = project.fmt(&["main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(
        project.read("main.veln"),
        concat!(
            "// keep leading comment\n",
            "fn main() -> ()\n",
            "  ()\n",
            "end\n",
        )
    );
    assert_contains_all(
        stderr(&output),
        &[
            "main.veln:1:1: error[parse.expected_item]: expected a function, test, type, effect, handler, or schema declaration",
        ],
    );
}

#[test]
fn fmt_formats_files_with_attached_standalone_comments() {
    let project = TestProject::new("fmt-attached-comments");
    project.write(
        "main.veln",
        concat!(
            "# module docs\n",
            "mod   app\n",
            "## public docs\n",
            "pub  fn   main ( value : Unit ) -> Unit effects [stdio]\n",
            "# return docs\n",
            "()\n",
            "end\n",
        ),
    );

    let expected = concat!(
        "# module docs\n",
        "mod app\n",
        "\n",
        "## public docs\n",
        "pub fn main(value: ()) -> () effects [stdio]\n",
        "\t# return docs\n",
        "\t()\n",
        "end\n",
    );
    project.assert_fmt_idempotent(&["main.veln"], &[("main.veln", expected)]);
}

#[test]
fn fmt_attaches_comments_to_imports_contracts_and_end_lines() {
    let project = TestProject::new("fmt-comment-targets");
    project.write(
        "main.veln",
        concat!(
            "mod   app\n",
            "# import docs\n",
            "use   platform.io\n",
            "# function docs\n",
            "fn   main ( ready : Bool ) -> Unit\n",
            "# require docs\n",
            "require ready\n",
            "# body docs\n",
            "()\n",
            "# end docs\n",
            "end\n",
        ),
    );

    let expected = concat!(
        "mod app\n",
        "# import docs\n",
        "use platform.io\n",
        "\n",
        "# function docs\n",
        "fn main(ready: Bool) -> ()\n",
        "\t# require docs\n",
        "\trequire ready\n",
        "\t# body docs\n",
        "\t()\n",
        "\t# end docs\n",
        "end\n",
    );
    project.assert_fmt_idempotent(&["main.veln"], &[("main.veln", expected)]);
}
