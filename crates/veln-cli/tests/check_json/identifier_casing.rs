use super::support::*;

#[test]
fn loaded_dependency_invalid_casing_stops_before_jvm_artifact() {
    let project = TestProject::new("identifier-casing-loaded-dependency");
    project.write(
        "main.veln",
        concat!(
            "use foo from \"github.com/oakcask/foo\"\n",
            "\n",
            "pub fn main() -> Int\n",
            "  foo::entry()\n",
            "end\n",
        ),
    );
    project.write(
        "veln.toml",
        concat!(
            "[dependencies.\"github.com/oakcask/foo\"]\n",
            "path = \"vendor/foo\"\n",
        ),
    );
    project.write(
        "vendor/foo/foo.veln",
        concat!(
            "pub fn Bad() -> Int\n",
            "  1\n",
            "end\n",
            "\n",
            "pub fn entry() -> Int\n",
            "  Bad()\n",
            "end\n",
        ),
    );
    project.write(
        "vendor/foo/veln.toml",
        concat!(
            "[package]\n",
            "name = \"github.com/oakcask/foo\"\n",
            "\n",
            "[lib]\n",
            "exports = [\"foo.veln\"]\n",
        ),
    );

    let output = project.run(&["--json", "main", "main.veln"]);

    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    assert_contains_all(
        stdout(&output),
        &[
            "\"status\":\"error\"",
            "\"id\":\"name.invalid_case\"",
            "function name `Bad` must start with an ASCII lowercase letter",
        ],
    );
    assert!(
        !project.root.join(".veln-test-cache/jvm").exists(),
        "source invalid-case static gate must stop before JVM artifact generation"
    );
}
