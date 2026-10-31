use super::*;

fn run_java_runtime_harness(
    name: &str,
    source_name: &str,
    source: &str,
    program: &JvmProgram,
) -> std::process::Output {
    run_java_runtime_harness_with_options(name, source_name, source, program, &[], &[])
}

fn run_java_runtime_harness_with_env(
    name: &str,
    source_name: &str,
    source: &str,
    program: &JvmProgram,
    env: &[(&str, &str)],
) -> std::process::Output {
    run_java_runtime_harness_with_options(name, source_name, source, program, env, &[])
}

fn run_java_runtime_harness_with_options(
    name: &str,
    source_name: &str,
    source: &str,
    program: &JvmProgram,
    env: &[(&str, &str)],
    jvm_args: &[&str],
) -> std::process::Output {
    let root = temp_dir(name);
    write_jvm_program(&root, program);
    fs::write(root.join(source_name), source).expect("Java harness should be written");
    let compiled = Command::new("javac")
        .arg(source_name)
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        compiled.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        compiled.status.code(),
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
    let class_name = source_name
        .strip_suffix(".java")
        .expect("Java harness source should end in .java");
    let mut command = Command::new("java");
    command
        .args(jvm_args)
        .arg("-cp")
        .arg(&root)
        .arg(class_name)
        .current_dir(&root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env {
        command.env(key, value);
    }
    let child = command.spawn().expect("Java harness should run");
    let output = wait_for_test_child(child, name, std::time::Duration::from_secs(30));
    let _ = fs::remove_dir_all(&root);
    output
}

#[test]
fn jvm_runtime_schema_refinement_validation_rejects_negative_offsets() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-refinement-negative-offset",
        "SchemaRefinementNegativeOffsetHarness.java",
        r#"
public final class SchemaRefinementNegativeOffsetHarness {
    public static void main(String[] args) {
        Object value = VelnRuntime.record(
            "state",
            VelnRuntime.adt("State::Connected", new Object[] {})
        );
        Object result = VelnRuntime.validateSchemaRefinements(
            value,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(-1) }),
            "ConnectedPacket",
            VelnRuntime.list("state"),
            VelnRuntime.list(VelnRuntime.list("Connected")),
            VelnRuntime.list(VelnRuntime.list())
        );
        System.out.println(result);
    }
}
"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Err(DecodeErrorWithReason(schema.validation_offset_invalid, ByteOffset(-1), /ConnectedPacket, offset=-1; expected a non-negative ByteOffset))\n"
    );
}

#[test]
fn jvm_runtime_schema_refinement_validation_preserves_record_identity_on_success() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping schema refinement identity verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-refinement-success-identity",
        "SchemaRefinementSuccessIdentityHarness.java",
        r#"
public final class SchemaRefinementSuccessIdentityHarness {
    public static void main(String[] args) {
        Object state = VelnRuntime.adt("State::Connected", new Object[] {});
        Object child = VelnRuntime.record("a/b~c", state);
        Object value = VelnRuntime.record("child", child);
        Object suppliedChild = VelnRuntime.recordField(value, "child");
        Object suppliedState = VelnRuntime.recordField(suppliedChild, "a/b~c");
        Object result = VelnRuntime.validateSchemaRefinements(
            value,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(7) }),
            "Parent/Schema~",
            VelnRuntime.list("child"),
            VelnRuntime.list(VelnRuntime.list()),
            VelnRuntime.list(VelnRuntime.list(
                VelnRuntime.list("a/b~c"),
                VelnRuntime.list(VelnRuntime.list("Connected")),
                VelnRuntime.list(VelnRuntime.list())
            ))
        );
        if (VelnRuntime.unwrapOk(result) != value) {
            throw new AssertionError("validation rebuilt the supplied record");
        }
        if (VelnRuntime.recordField(value, "child") != suppliedChild
                || VelnRuntime.recordField(suppliedChild, "a/b~c") != suppliedState) {
            throw new AssertionError("validation rebuilt the supplied payload");
        }
        Object rejected = VelnRuntime.validateSchemaRefinements(
            VelnRuntime.record(
                "child",
                VelnRuntime.record(
                    "a/b~c",
                    VelnRuntime.adt("State::Closed", new Object[] {})
                ),
                "later",
                VelnRuntime.adt("State::Closed", new Object[] {})
            ),
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(31) }),
            "Parent/Schema~",
            VelnRuntime.list("child", "later"),
            VelnRuntime.list(VelnRuntime.list(), VelnRuntime.list("Connected")),
            VelnRuntime.list(
                VelnRuntime.list(
                    VelnRuntime.list("a/b~c"),
                    VelnRuntime.list(VelnRuntime.list("Connected")),
                    VelnRuntime.list(VelnRuntime.list())
                ),
                VelnRuntime.list()
            )
        );
        System.out.println("validation preserved record identity");
        System.out.println(rejected);
    }
}
"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "validation preserved record identity\n",
            "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, ",
            "ByteOffset(31), /Parent~1Schema~0/child/a~1b~0c, ",
            "actual variant `Closed`; expected one of `Connected`))\n"
        )
    );
}

#[test]
fn jvm_runtime_schema_predicates_follow_source_precedence_and_contain_division_failure() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping schema predicate verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-predicate-precedence",
        "SchemaPredicatePrecedenceHarness.java",
        r#"
public final class SchemaPredicatePrecedenceHarness {
    private static Object validate(long value, String fieldPredicate, String validation) {
        return VelnRuntime.validateSchemaRefinementAndPredicates(
            VelnRuntime.record("value", Long.valueOf(value)),
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(19) }),
            "Packet",
            VelnRuntime.list("record", VelnRuntime.list()),
            java.util.Arrays.asList(
                "Packet",
                VelnRuntime.list("value"),
                VelnRuntime.list(fieldPredicate),
                validation,
                VelnRuntime.list("")
            )
        );
    }

    public static void main(String[] args) {
        System.out.println(validate(
            1,
            "(value & 1) == 1",
            "value << 1 == 2 and 1 < 2 == true"
        ));
        System.out.println(validate(2, "(value & 1) == 1", ""));
        System.out.println(validate(2, "value & 1 == 1", ""));
        System.out.println(validate(0, "10 / value > 0", ""));
    }
}
"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        concat!(
            "Ok({value=1})\n",
            "Err(DecodeErrorWithReason(schema.validation_failed, ByteOffset(19), /Packet/value, ",
            "predicate `(value & 1) == 1` failed; supplied_values=value=2))\n",
            "Err(DecodeErrorWithReason(schema.validation_failed, ByteOffset(19), /Packet/value, ",
            "predicate `value & 1 == 1` evaluation failed: schema predicate operator `&` ",
            "expects Int operands; supplied_values=value=2))\n",
            "Err(DecodeErrorWithReason(schema.validation_failed, ByteOffset(19), /Packet/value, ",
            "predicate `10 / value > 0` evaluation failed: schema predicate division by zero; ",
            "supplied_values=value=0))\n",
        )
    );
    assert!(!stdout.contains("ArithmeticException"));
    assert!(!stdout.contains("/ by zero"));
    assert!(!String::from_utf8_lossy(&output.stderr).contains("java.lang"));
}

#[test]
fn jvm_runtime_schema_refinement_rejection_leaves_input_unchanged() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping schema refinement rejection verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-refinement-rejection-identity",
        "SchemaRefinementRejectionIdentityHarness.java",
        r#"
public final class SchemaRefinementRejectionIdentityHarness {
    public static void main(String[] args) {
        Object statePayload = Long.valueOf(9);
        Object state = VelnRuntime.adt("State::Closed", new Object[] { statePayload });
        Object metadata = VelnRuntime.list("unchanged");
        Object value = VelnRuntime.record("state", state, "metadata", metadata);
        Object suppliedState = VelnRuntime.recordField(value, "state");
        Object suppliedMetadata = VelnRuntime.recordField(value, "metadata");
        Object suppliedStatePayload = VelnRuntime.adtPayload(suppliedState, 0);
        String before = value.toString();
        VelnRuntime.Result result = (VelnRuntime.Result) VelnRuntime.validateSchemaRefinements(
            value,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(7) }),
            "ConnectedPacket",
            VelnRuntime.list("state", "metadata"),
            VelnRuntime.list(VelnRuntime.list("Connected"), VelnRuntime.list()),
            VelnRuntime.list(VelnRuntime.list(), VelnRuntime.list())
        );
        if (result.isOk()
                || !result.toString().contains("schema.variant_refinement_mismatch")) {
            throw new AssertionError("expected validation rejection");
        }
        if (VelnRuntime.recordField(value, "state") != suppliedState
                || VelnRuntime.recordField(value, "metadata") != suppliedMetadata
                || VelnRuntime.adtPayload(suppliedState, 0) != suppliedStatePayload
                || !Long.valueOf(9).equals(suppliedStatePayload)
                || !value.toString().equals(before)) {
            throw new AssertionError("validation changed the rejected input");
        }
        System.out.println("rejection left input unchanged");
    }
}
"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "rejection left input unchanged\n"
    );
}

#[test]
fn jvm_runtime_schema_refinement_plan_traverses_standard_containers_deterministically() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping container refinement verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-refinement-containers",
        "SchemaRefinementContainersHarness.java",
        r#"
public final class SchemaRefinementContainersHarness {
    private static Object variant(String expected) {
        return VelnRuntime.list("variant", VelnRuntime.list(expected));
    }

    private static Object field(String name, Object plan) {
        return VelnRuntime.list(name, plan);
    }

    private static VelnRuntime.Result validate(Object value, Object plan) {
        return (VelnRuntime.Result) VelnRuntime.validateSchemaRefinementPlan(
            value,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(37) }),
            "ContainerPacket",
            plan
        );
    }

    private static void requireFailurePath(Object value, Object plan, String path) {
        VelnRuntime.Result result = validate(value, plan);
        if (result.isOk() || !result.toString().contains(path)) {
            throw new AssertionError("expected failure at " + path + ": " + result);
        }
    }

    public static void main(String[] args) {
        Object connected = VelnRuntime.adt("State::Connected", new Object[] {});
        Object closed = VelnRuntime.adt("State::Closed", new Object[] {});
        Object statePlan = variant("Connected");
        Object rootPlan = VelnRuntime.list(
            "record",
            VelnRuntime.list(
                field("optional", VelnRuntime.list("option", statePlan)),
                field(
                    "outcome",
                    VelnRuntime.list("result", statePlan, variant("Closed"))
                ),
                field("linked", VelnRuntime.list("list", statePlan)),
                field("vector", VelnRuntime.list("vec", statePlan)),
                field("table", VelnRuntime.list("dict", statePlan)),
                field(
                    "mixed",
                    VelnRuntime.list(
                        "option",
                        VelnRuntime.list(
                            "vec",
                            VelnRuntime.list(
                                "dict",
                                VelnRuntime.list(
                                    "result",
                                    statePlan,
                                    variant("Closed")
                                )
                            )
                        )
                    )
                )
            )
        );
        Object value = VelnRuntime.record(
            "optional", VelnRuntime.none(),
            "outcome", VelnRuntime.err(closed),
            "linked", VelnRuntime.listCons(connected, VelnRuntime.listCons(connected, VelnRuntime.listNil())),
            "vector", VelnRuntime.list(connected, connected),
            "table", VelnRuntime.dict("two", connected, "one", connected),
            "mixed", VelnRuntime.some(VelnRuntime.list(VelnRuntime.dict("ok", VelnRuntime.ok(connected))))
        );
        VelnRuntime.Result accepted = validate(value, rootPlan);
        if (!accepted.isOk() || accepted.value() != value) {
            throw new AssertionError("successful validation did not preserve input identity");
        }

        Object linked = VelnRuntime.listCons(
            connected,
            VelnRuntime.listCons(closed, VelnRuntime.listCons(closed, VelnRuntime.listNil()))
        );
        Object rejectedList = VelnRuntime.record(
            "optional", VelnRuntime.none(),
            "outcome", VelnRuntime.err(closed),
            "linked", linked,
            "vector", VelnRuntime.list(),
            "table", VelnRuntime.dict(),
            "mixed", VelnRuntime.none()
        );
        requireFailurePath(rejectedList, rootPlan, "/ContainerPacket/linked/1");

        Object vector = VelnRuntime.list(connected, closed, closed);
        Object rejected = VelnRuntime.record(
            "optional", VelnRuntime.none(),
            "outcome", VelnRuntime.err(closed),
            "linked", VelnRuntime.listNil(),
            "vector", vector,
            "table", VelnRuntime.dict(),
            "mixed", VelnRuntime.none()
        );
        Object suppliedVector = VelnRuntime.recordField(rejected, "vector");
        requireFailurePath(rejected, rootPlan, "/ContainerPacket/vector/1");
        if (VelnRuntime.recordField(rejected, "vector") != suppliedVector) {
            throw new AssertionError("failed validation changed input identity");
        }

        Object dictPlan = VelnRuntime.list(
            "record",
            VelnRuntime.list(field("table", VelnRuntime.list("dict", statePlan)))
        );
        String firstUtf8 = "\uE000";
        String laterUtf8 = "\uD800\uDC00";
        Object insertionOne = VelnRuntime.record(
            "table", VelnRuntime.dict(laterUtf8, closed, firstUtf8, closed)
        );
        Object insertionTwo = VelnRuntime.record(
            "table", VelnRuntime.dict(firstUtf8, closed, laterUtf8, closed)
        );
        requireFailurePath(insertionOne, dictPlan, "/ContainerPacket/table/" + firstUtf8);
        requireFailurePath(insertionTwo, dictPlan, "/ContainerPacket/table/" + firstUtf8);

        Object escaped = VelnRuntime.record("table", VelnRuntime.dict("a/b~c", closed));
        requireFailurePath(escaped, dictPlan, "/ContainerPacket/table/a~1b~0c");

        java.util.Map<Object, Object> manyLongKeys =
            new java.util.LinkedHashMap<Object, Object>();
        StringBuilder prefixBuilder = new StringBuilder();
        for (int index = 0; index < 512; index += 1) prefixBuilder.append('a');
        String prefix = prefixBuilder.toString();
        for (int index = 255; index >= 0; index -= 1) {
            String key = prefix + String.format("-%03d", Integer.valueOf(index));
            manyLongKeys.put(key, index == 0 ? closed : connected);
        }
        Object many = VelnRuntime.record("table", manyLongKeys);
        requireFailurePath(many, dictPlan, "/ContainerPacket/table/" + prefix + "-000");
        System.out.println("container refinement traversal passed");
    }
}

"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "container refinement traversal passed\n"
    );
}

#[test]
fn jvm_runtime_schema_refinement_list_fails_at_zero_with_bounded_heap() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping bounded-list refinement verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness_with_options(
        "runtime-schema-refinement-bounded-list",
        "SchemaRefinementBoundedListHarness.java",
        r#"
public final class SchemaRefinementBoundedListHarness {
    public static void main(String[] args) {
        Object connected = VelnRuntime.adt("State::Connected", new Object[] {});
        Object closed = VelnRuntime.adt("State::Closed", new Object[] {});
        Object items = VelnRuntime.listNil();
        for (int index = 0; index < 1000000; index += 1) {
            items = VelnRuntime.listCons(connected, items);
        }
        items = VelnRuntime.listCons(closed, items);
        Object plan = VelnRuntime.list(
            "list",
            VelnRuntime.list("variant", VelnRuntime.list("Connected"))
        );
        Object result = VelnRuntime.validateSchemaRefinementPlan(
            items,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(79) }),
            "Packet",
            plan
        );
        String rendered = result.toString();
        if (!rendered.contains("schema.variant_refinement_mismatch")
                || !rendered.contains("ByteOffset(79)")
                || !rendered.contains("/Packet/0")) {
            throw new AssertionError("expected immediate list failure at index zero: " + rendered);
        }
        System.out.println("bounded list refinement failed at index zero");
    }
}
"#,
        &program,
        &[],
        &["-Xms16m", "-Xmx64m"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "bounded list refinement failed at index zero\n"
    );
}

#[test]
fn jvm_runtime_schema_refinement_vector_is_incremental_with_bounded_heap() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping bounded-vector refinement verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness_with_options(
        "runtime-schema-refinement-bounded-vector",
        "SchemaRefinementBoundedVectorHarness.java",
        r#"
public final class SchemaRefinementBoundedVectorHarness {
    private static final Object PLAN = VelnRuntime.list(
        "vec",
        VelnRuntime.list("variant", VelnRuntime.list("Connected"))
    );

    private static VelnRuntime.Result validate(Object value) {
        return (VelnRuntime.Result) VelnRuntime.validateSchemaRefinementPlan(
            value,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(79) }),
            "Packet",
            PLAN
        );
    }

    private static void requireSuccess(Object value) {
        VelnRuntime.Result result = validate(value);
        if (!result.isOk() || result.value() != value) {
            throw new AssertionError("vector success did not preserve input identity");
        }
    }

    private static void requireFailure(Object value, String path) {
        String rendered = validate(value).toString();
        String expected = "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, "
            + "ByteOffset(79), " + path + ", actual variant `Closed`; "
            + "expected one of `Connected`))";
        if (!rendered.equals(expected)) {
            throw new AssertionError("unexpected vector failure: " + rendered);
        }
    }

    public static void main(String[] args) {
        Object connected = VelnRuntime.adt("State::Connected", new Object[] {});
        Object closed = VelnRuntime.adt("State::Closed", new Object[] {});
        requireSuccess(VelnRuntime.list());
        requireSuccess(VelnRuntime.list(connected, connected));
        requireFailure(VelnRuntime.list(connected, closed, connected), "/Packet/1");

        java.util.ArrayList<Object> items = new java.util.ArrayList<Object>(1000001);
        items.add(closed);
        for (int index = 0; index < 1000000; index += 1) items.add(connected);
        requireFailure(items, "/Packet/0");
        System.out.println("bounded vector refinement remained incremental");
    }
}
"#,
        &program,
        &[],
        &["-Xms16m", "-Xmx64m"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "bounded vector refinement remained incremental\n"
    );
}

#[test]
fn jvm_runtime_schema_refinement_plan_visits_only_actual_source_adt_payloads() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping source ADT refinement verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-refinement-source-adt",
        "SchemaRefinementSourceAdtHarness.java",
        r#"
public final class SchemaRefinementSourceAdtHarness {
    private static Object variant(String expected) {
        return VelnRuntime.list("variant", VelnRuntime.list(expected));
    }
    private static Object payload(int index, String segment, Object plan) {
        return VelnRuntime.list(Integer.valueOf(index), segment, plan);
    }
    private static Object sourceAdtPlan() {
        return VelnRuntime.list(
            "adt",
            VelnRuntime.list(),
            VelnRuntime.list(
                VelnRuntime.list("Boxed", VelnRuntime.list(payload(0, "0", variant("Connected")))),
                VelnRuntime.list("Named", VelnRuntime.list(payload(0, "a/b~c", variant("Connected")))),
                VelnRuntime.list("Empty", VelnRuntime.list())
            )
        );
    }
    private static Object rootPlan() {
        return VelnRuntime.list(
            "record",
            VelnRuntime.list(VelnRuntime.list("item", sourceAdtPlan()))
        );
    }
    private static Object orderedPayloadPlan() {
        return VelnRuntime.list(
            Integer.valueOf(2),
            VelnRuntime.list(
                VelnRuntime.list("variant", VelnRuntime.list("Connected")),
                VelnRuntime.list(
                    "adt",
                    VelnRuntime.list(),
                    VelnRuntime.list(VelnRuntime.list(
                        "Pair",
                        VelnRuntime.list(
                            VelnRuntime.list(Integer.valueOf(0), "first", Integer.valueOf(0)),
                            VelnRuntime.list(Integer.valueOf(1), "second", Integer.valueOf(0))
                        )
                    ))
                ),
                VelnRuntime.list(
                    "record",
                    VelnRuntime.list(VelnRuntime.list("item", Integer.valueOf(1)))
                )
            )
        );
    }
    private static Object nestedSourceAdtPlan() {
        return VelnRuntime.list(
            "record",
            VelnRuntime.list(VelnRuntime.list(
                "item",
                VelnRuntime.list(
                    "adt",
                    VelnRuntime.list(),
                    VelnRuntime.list(VelnRuntime.list(
                        "Nested",
                        VelnRuntime.list(payload(0, "child", sourceAdtPlan()))
                    ))
                )
            ))
        );
    }
    private static Object validateWithPlan(Object value, Object plan) {
        return VelnRuntime.validateSchemaRefinementPlan(
            VelnRuntime.record("item", value),
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(43) }),
            "Packet",
            plan
        );
    }
    private static Object validate(Object value) {
        return validateWithPlan(value, rootPlan());
    }
    public static void main(String[] args) {
        Object connected = VelnRuntime.adt("State::Connected", new Object[] {});
        Object closed = VelnRuntime.adt("State::Closed", new Object[] {});
        Object accepted = VelnRuntime.record(
            "item",
            VelnRuntime.adt("Box::Boxed", new Object[] { connected })
        );
        Object acceptedResult = VelnRuntime.validateSchemaRefinementPlan(
            accepted,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(43) }),
            "Packet",
            rootPlan()
        );
        if (VelnRuntime.unwrapOk(acceptedResult) != accepted) {
            throw new AssertionError("source ADT validation rebuilt the input");
        }
        validate(VelnRuntime.adt("Box::Empty", new Object[] {}));
        System.out.println(validate(VelnRuntime.adt("Box::Boxed", new Object[] { closed })));
        System.out.println(validate(VelnRuntime.adt("Box::Named", new Object[] { closed })));
        System.out.println(validateWithPlan(
            VelnRuntime.adt("Pair::Pair", new Object[] { connected, closed }),
            orderedPayloadPlan()
        ));
        System.out.println(validateWithPlan(
            VelnRuntime.adt(
                "Outer::Nested",
                new Object[] { VelnRuntime.adt("Box::Boxed", new Object[] { closed }) }
            ),
            nestedSourceAdtPlan()
        ));
        System.out.println("source ADT success preserved input identity");
    }
}
"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        concat!(
            "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, ",
            "ByteOffset(43), /Packet/item/Boxed/0, actual variant `Closed`; ",
            "expected one of `Connected`))\n",
            "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, ",
            "ByteOffset(43), /Packet/item/Named/a~1b~0c, actual variant `Closed`; ",
            "expected one of `Connected`))\n",
            "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, ",
            "ByteOffset(43), /Packet/item/Pair/second, actual variant `Closed`; ",
            "expected one of `Connected`))\n",
            "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, ",
            "ByteOffset(43), /Packet/item/Nested/child/Boxed/0, actual variant `Closed`; ",
            "expected one of `Connected`))\n",
            "source ADT success preserved input identity\n"
        )
    );
}

#[test]
fn jvm_runtime_schema_refinement_plan_traverses_deep_recursive_values_iteratively() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        eprintln!("skipping deep recursive refinement verification because a JDK is unavailable");
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-schema-refinement-deep-recursive-source-adt",
        "SchemaRefinementDeepRecursiveSourceAdtHarness.java",
        r#"
public final class SchemaRefinementDeepRecursiveSourceAdtHarness {
    private static final Object PLAN = plan();

    private static Object plan() {
        Object state = VelnRuntime.list("variant", VelnRuntime.list("Connected"));
        Object optionalRecursive = VelnRuntime.list("option", Integer.valueOf(2));
        Object recursive = VelnRuntime.list(
            "adt",
            VelnRuntime.list("Next"),
            VelnRuntime.list(
                VelnRuntime.list(
                    "Next",
                    VelnRuntime.list(
                        VelnRuntime.list(Integer.valueOf(0), "state/value~", Integer.valueOf(0)),
                        VelnRuntime.list(Integer.valueOf(1), "next/value~", Integer.valueOf(1))
                    )
                )
            )
        );
        Object root = VelnRuntime.list(
            "record",
            VelnRuntime.list(VelnRuntime.list("item", Integer.valueOf(2)))
        );
        return VelnRuntime.list(
            Integer.valueOf(3),
            VelnRuntime.list(
                state,
                optionalRecursive,
                recursive,
                root
            )
        );
    }

    private static Object value(int depth, String state) {
        Object item = VelnRuntime.adt("Node::Next", new Object[] {
            VelnRuntime.adt("State::" + state, new Object[] {}),
            VelnRuntime.none()
        });
        for (int index = 0; index < depth; index += 1) {
            item = VelnRuntime.adt("Node::Next", new Object[] {
                VelnRuntime.adt("State::Connected", new Object[] {}),
                VelnRuntime.some(item)
            });
        }
        return VelnRuntime.record("item", item);
    }

    private static Object constructorFailure(int depth) {
        Object item = VelnRuntime.adt("Node::End", new Object[] {});
        for (int index = 0; index < depth; index += 1) {
            item = VelnRuntime.adt("Node::Next", new Object[] {
                VelnRuntime.adt("State::Connected", new Object[] {}),
                VelnRuntime.some(item)
            });
        }
        return VelnRuntime.record("item", item);
    }

    private static VelnRuntime.Result validate(Object value) {
        return (VelnRuntime.Result) VelnRuntime.validateSchemaRefinementPlan(
            value,
            VelnRuntime.adt("ByteOffset", new Object[] { Long.valueOf(71) }),
            "Packet",
            PLAN
        );
    }

    private static long allocatedByValidation(
        com.sun.management.ThreadMXBean allocations,
        Object value
    ) {
        long thread = Thread.currentThread().getId();
        long before = allocations.getThreadAllocatedBytes(thread);
        VelnRuntime.Result result = validate(value);
        long allocated = allocations.getThreadAllocatedBytes(thread) - before;
        if (!result.isOk() || result.value() != value) {
            throw new AssertionError("deep recursive validation did not preserve input identity");
        }
        return allocated;
    }

    public static void main(String[] args) {
        validate(value(256, "Connected"));
        com.sun.management.ThreadMXBean allocations =
            (com.sun.management.ThreadMXBean) java.lang.management.ManagementFactory
                .getThreadMXBean();
        if (!allocations.isThreadAllocatedMemorySupported()) {
            throw new AssertionError("thread allocation accounting is required");
        }
        allocations.setThreadAllocatedMemoryEnabled(true);
        Object small = value(2000, "Connected");
        Object large = value(4000, "Connected");
        long smallBytes = allocatedByValidation(allocations, small);
        long largeBytes = allocatedByValidation(allocations, large);
        if (largeBytes > smallBytes * 3L) {
            throw new AssertionError(
                "deep validation allocation did not scale linearly: "
                    + smallBytes + " -> " + largeBytes
            );
        }

        int failureDepth = 64;
        StringBuilder path = new StringBuilder("/Packet/item");
        for (int index = 0; index < failureDepth; index += 1) {
            path.append("/Next/next~1value~0");
        }
        path.append("/Next/state~1value~0");
        String expected = "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, "
            + "ByteOffset(71), " + path + ", actual variant `Closed`; "
            + "expected one of `Connected`))";
        String actual = validate(value(failureDepth, "Closed")).toString();
        if (!actual.equals(expected)) {
            throw new AssertionError("unexpected deep failure path: " + actual);
        }
        String expectedConstructorPath = "/Packet/item";
        for (int index = 0; index < failureDepth; index += 1) {
            expectedConstructorPath += "/Next/next~1value~0";
        }
        String expectedConstructorFailure =
            "Err(DecodeErrorWithReason(schema.variant_refinement_mismatch, "
            + "ByteOffset(71), " + expectedConstructorPath + ", actual variant `End`; "
            + "expected one of `Next`))";
        String actualConstructorFailure = validate(constructorFailure(failureDepth)).toString();
        if (!actualConstructorFailure.equals(expectedConstructorFailure)) {
            throw new AssertionError(
                "unexpected deep constructor failure path: " + actualConstructorFailure
            );
        }
        System.out.println("deep recursive refinement traversal passed");
    }
}
"#,
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "deep recursive refinement traversal passed\n"
    );
}

#[test]
fn jvm_runtime_preserves_cancellation_entering_host_connect_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-connect-cancellation",
        "ConnectCancellationHarness.java",
        include_str!("../../test-support/ConnectCancellationHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "connect cancellation remained primary\n"
    );
}

#[test]
fn jvm_runtime_does_not_classify_runtime_failure_text_as_cancellation_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-cancellation-identity",
        "CancellationIdentityHarness.java",
        include_str!("../../test-support/CancellationIdentityHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "runtime failure identity remained primary\n"
    );
}

#[test]
fn jvm_runtime_preserves_network_system_commit_and_ownership_invariants_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_network_facade_entry_to_ir("", "write_all_selected");
    let program = generate_classfiles_with_entry(&ir, "write_all_selected");
    let output = run_java_runtime_harness_with_env(
        "runtime-network-system-lifecycle",
        "NetworkSystemLifecycleHarness.java",
        include_str!("../../test-support/NetworkSystemLifecycleHarness.java"),
        &program,
        &[
            ("VELN_RESULT_ERRORS", "result-errors.tsv"),
            ("VELN_TRANSPORT_ERRORS", "transport-errors.tsv"),
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "network system lifecycle invariants held\n"
    );
}

#[test]
fn jvm_runtime_concurrent_cancellation_preserves_cleanup_before_join_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir(include_str!("../../test-support/join_blocking.veln"));
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-join-cleanup-blocking",
        "JoinBlockingHarness.java",
        include_str!("../../test-support/JoinBlockingHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "concurrent cancellation preserved cleanup completion ordering\n"
    );
}

#[test]
fn jvm_runtime_cleanup_diagnostics_aggregate_nested_failures_linearly_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-cleanup-diagnostic-growth",
        "CleanupDiagnosticGrowthHarness.java",
        include_str!("../../test-support/CleanupDiagnosticGrowthHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "cleanup diagnostics aggregate nested failures linearly\n"
    );
}

#[test]
fn jvm_runtime_bounds_standard_input_read_ahead_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-stdin-backpressure");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeStdinBackpressureHarness.java"),
        RUNTIME_STDIN_BACKPRESSURE_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeStdinBackpressureHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeStdinBackpressureHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let reads = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<usize>()
        .expect("harness should print its input read count");
    assert!(reads <= 3, "stdin pump read ahead {reads} chunks");
}

#[test]
fn jvm_runtime_rechecks_partial_socket_write_deadline_and_cancellation_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-socket-partial-write");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeSocketPartialWriteHarness.java"),
        RUNTIME_SOCKET_PARTIAL_WRITE_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeSocketPartialWriteHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeSocketPartialWriteHarness")
        .current_dir(&root)
        .output()
        .expect("java should run");
    let _ = fs::remove_dir_all(&root);

    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("Operation not permitted") {
        return;
    }
    assert!(output.status.success(), "{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "WriteDeadlineExpired\nWriteCancelled\n"
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
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
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
fn jvm_runtime_reports_receiver_list_timeout_result_outcomes_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-channel-select-many-timeout-result");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeChannelSelectManyTimeoutResultHarness.java"),
        RUNTIME_CHANNEL_SELECT_MANY_TIMEOUT_RESULT_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeChannelSelectManyTimeoutResultHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeChannelSelectManyTimeoutResultHarness")
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
        concat!(
            "Ok(Some({index=1, value=21}))\n",
            "Ok(None)\n",
            "Err(interrupted)\n",
            "Err(cancelled)\n",
            "Err(cancelled)\n",
        )
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
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
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
fn jvm_runtime_reads_and_writes_byte_views_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }

    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let root = temp_dir("runtime-byte-view");
    write_jvm_program(&root, &program);
    fs::write(
        root.join("RuntimeByteViewHarness.java"),
        RUNTIME_BYTE_VIEW_HARNESS,
    )
    .expect("Java harness should be written");

    let javac = Command::new("javac")
        .arg("RuntimeByteViewHarness.java")
        .current_dir(&root)
        .output()
        .expect("javac should run");
    assert!(
        javac.status.success(),
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
        String::from_utf8_lossy(&javac.stderr)
    );

    let output = Command::new("java")
        .arg("-cp")
        .arg(&root)
        .arg("RuntimeByteViewHarness")
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
        concat!(
            "Ok(1)\n",
            "Ok(258)\n",
            "Ok(66051)\n",
            "Ok(513)\n",
            "Ok(197121)\n",
            "ByteCount(3)\n",
            "Ok(258)\n",
            "Ok(3)\n",
            "Ok(515)\n",
            "Cons(ByteChunk([Byte(1), Byte(2)]), Cons(ByteChunk([Byte(3)]), Nil))\n",
            "Err(byte view count exceeds view length)\n",
            "Ok(16909311)\n",
            "Ok(2147483647)\n",
            "Ok(4294967295)\n",
            "Ok(1099511627775)\n",
            "Ok(281474976710655)\n",
            "Ok(2147483647)\n",
            "Ok(4294967295)\n",
            "Ok(1099511627775)\n",
            "Ok(281474976710655)\n",
            "Ok(9223372036854775807)\n",
            "Ok(9223372036854775807)\n",
            "Ok(ByteChunk([Byte(52), Byte(18)]))\n",
            "Ok(ByteChunk([Byte(3), Byte(2), Byte(1)]))\n",
            "Ok(ByteChunk([Byte(4), Byte(3), Byte(2), Byte(1)]))\n",
            "Err(byte read requires 3 bytes but view has 2)\n",
            "Err(byte read requires 3 bytes but view has 2)\n",
            "Err(byte view range exceeds chunk length)\n",
            "Err(byte_write_u8_be value must be between 0 and 255)\n",
            "Err(byte_write_u32_le value must be between 0 and 4294967295)\n",
            "Err(byte_read_u31_be value exceeds maximum 2147483647)\n",
            "Err(byte_read_u31_le value exceeds maximum 2147483647)\n",
            "Err(byte_read_u64_be value exceeds maximum 9223372036854775807)\n",
            "Err(byte_read_u64_le value exceeds maximum 9223372036854775807)\n",
            "Ok({length=5, kind=1, flags=4, stream_id=1})\n",
            "Ok({short_value=4660, wide_value=3735928559})\n",
            "Ok({length=5, padding_length=4})\n",
            "Err(schema validation failed at byte offset 3)\n",
            "Err(reserved bits mismatch at byte offset 5)\n",
        )
    );
}

#[test]
fn jvm_runtime_byte_chunk_slicing_has_linear_work_and_bounded_retention_when_java_is_available() {
    if Command::new("java").arg("-version").output().is_err()
        || Command::new("javac").arg("-version").output().is_err()
    {
        return;
    }
    let ir = lower_to_ir("pub fn main() -> ()\n  ()\nend\n");
    let program = generate_classfiles_with_entry(&ir, "main");
    let output = run_java_runtime_harness(
        "runtime-byte-chunk-slice-work",
        "ByteChunkSliceWorkHarness.java",
        include_str!("../../test-support/ByteChunkSliceWorkHarness.java"),
        &program,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "byte chunk slicing remained linear and retention stayed bounded\n"
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
        "javac failed with status {:?}\nstdout:\n{}\nstderr:\n{}",
        javac.status.code(),
        String::from_utf8_lossy(&javac.stdout),
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
