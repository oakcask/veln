import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const schemaUrl = new URL(
  "../crates/veln-stdlib/schemas/observability/v1/json-lines-record.schema.json",
  import.meta.url,
);
const schema = JSON.parse(await readFile(schemaUrl, "utf8"));
const hostBoundaryUrl = new URL(
  "../examples/specification/run/observability-json-lines-encoder/case-text/records.jsonl",
  import.meta.url,
);

function resolve(reference) {
  assert.match(reference, /^#\/\$defs\/[a-z_]+$/u);
  return schema.$defs[reference.slice("#/$defs/".length)];
}

function validate(value, rule, path = "record") {
  if (rule.$ref) return validate(value, resolve(rule.$ref), path);
  for (const constraint of rule.allOf ?? []) validate(value, constraint, path);
  if (rule.anyOf) {
    const failures = [];
    for (const choice of rule.anyOf) {
      try {
        validate(value, choice, path);
        return;
      } catch (error) {
        failures.push(error.message);
      }
    }
    assert.fail(`${path} matched no anyOf branch: ${failures.join("; ")}`);
  }
  if (rule.if) {
    let matches = true;
    try {
      validate(value, rule.if, path);
    } catch {
      matches = false;
    }
    if (matches && rule.then) validate(value, rule.then, path);
    if (!matches && rule.else) validate(value, rule.else, path);
  }
  if (rule.not) {
    let matches = true;
    try {
      validate(value, rule.not, path);
    } catch {
      matches = false;
    }
    assert.equal(matches, false, `${path} matched a forbidden schema`);
  }
  if (Object.hasOwn(rule, "const")) assert.deepEqual(value, rule.const, path);
  if (rule.enum) assert.ok(rule.enum.includes(value), `${path} is outside its enum`);
  if (rule.type === "null") assert.equal(value, null, path);
  if (rule.type === "string") {
    assert.equal(typeof value, "string", path);
    if (rule.minLength !== undefined) assert.ok(value.length >= rule.minLength, path);
  }
  if (rule.type === "integer") {
    assert.ok(Number.isSafeInteger(value), path);
    if (rule.minimum !== undefined) assert.ok(value >= rule.minimum, path);
    if (rule.maximum !== undefined) assert.ok(value <= rule.maximum, path);
  }
  if (rule.type === "boolean") assert.equal(typeof value, "boolean", path);
  if (rule.type === "array") {
    assert.ok(Array.isArray(value), path);
    value.forEach((item, index) => validate(item, rule.items, `${path}[${index}]`));
  }
  const objectValue = value !== null && typeof value === "object" && !Array.isArray(value);
  if (rule.type === "object") assert.ok(objectValue, path);
  if (objectValue) {
    for (const name of rule.required ?? []) {
      assert.ok(Object.hasOwn(value, name), `${path}.${name} is required`);
    }
    if (rule.additionalProperties === false) {
      for (const name of Object.keys(value)) {
        assert.ok(Object.hasOwn(rule.properties, name), `${path}.${name} is not allowed`);
      }
    }
    for (const [name, child] of Object.entries(rule.properties ?? {})) {
      if (Object.hasOwn(value, name)) validate(value[name], child, `${path}.${name}`);
    }
  }
}

function representative(kind) {
  return {
    schema_version: 1,
    kind,
    name: kind === "log" ? "log" : "request",
    wall_time: { unix_seconds: 0, nanosecond: 7 },
    sequence: 1,
    severity: kind === "log" ? "info" : null,
    trace: kind === "log" ? { trace_id: "trace-1", span_id: "span-1", sampled: true } : null,
    span: kind.startsWith("span_")
      ? { trace_id: "trace-1", span_id: "span-2", parent_span_id: null }
      : null,
    links: [],
    source: {
      package: "app",
      module: "main",
      file: "src/main.veln",
      start_line: 4,
      start_column: 2,
      end_line: 4,
      end_column: 18,
    },
    task_id: null,
    attributes: [{ name: "attempt", type: "int", value: 1 }],
    resource: { name: "checkout", attributes: [] },
    instrumentation_scope: { name: "checkout.web", version: "1" },
    cardinality: { admitted_metric_observations: 0, dropped_metric_observations: 0 },
    exporter_health: { failed_exports: 0 },
    attribute_facts: { dropped: 0, truncated: 0, redacted: 0 },
    ...(kind === "log" ? { message: "ready" } : {}),
    ...(kind === "counter" || kind === "histogram" ? { value: 1 } : {}),
  };
}

test("the checked schema rejects incompatible versions and unrecognized fields", () => {
  assert.throws(() => validate({ ...representative("event"), schema_version: 2 }, schema));
  assert.throws(() => validate({ ...representative("event"), unexpected: true }, schema));
  assert.throws(() => validate({
    ...representative("event"),
    attributes: [{ name: "attempt", type: "int", value: "1" }],
  }, schema));
  const logWithoutMessage = representative("log");
  delete logWithoutMessage.message;
  assert.throws(() => validate(logWithoutMessage, schema));
  assert.throws(() => validate({ ...representative("event"), message: "unexpected" }, schema));
  const counterWithoutValue = representative("counter");
  delete counterWithoutValue.value;
  assert.throws(() => validate(counterWithoutValue, schema));
  assert.throws(() => validate({ ...representative("span_start"), trace: { trace_id: "trace-1", span_id: "span-1", sampled: true } }, schema));
  assert.throws(() => validate({
    ...representative("log"),
    trace: { trace_id: "", span_id: "span-1", sampled: true },
  }, schema));
  assert.throws(() => validate({
    ...representative("log"),
    trace: { trace_id: "trace-1", span_id: "", sampled: true },
  }, schema));
  assert.throws(() => validate({
    ...representative("span_start"),
    span: { trace_id: "trace-1", span_id: "span-2", parent_span_id: "" },
  }, schema));
  assert.throws(() => validate({
    ...representative("span_start"),
    links: [{ trace_id: "", span_id: "span-3", sampled: false }],
  }, schema));
  assert.throws(() => validate({
    ...representative("event"),
    source: { ...representative("event").source, start_line: -1 },
  }, schema));
  assert.throws(() => validate({
    ...representative("event"),
    source: { ...representative("event").source, end_column: -1 },
  }, schema));
  assert.throws(() => validate({
    ...representative("event"),
    resource: { name: "", attributes: [] },
  }, schema));
  assert.throws(() => validate({
    ...representative("event"),
    instrumentation_scope: { name: "", version: "1" },
  }, schema));
});

test("six captured JVM encoder records satisfy schema, branch, framing, and path contracts", async () => {
  const raw = await readFile(hostBoundaryUrl, "utf8");
  assert.equal(raw.includes("\r"), false);
  assert.equal(raw.endsWith("\n"), true);
  const parts = raw.split("\n");
  assert.equal(parts.length, 7);
  assert.equal(parts[6], "");
  const records = parts.slice(0, 6).map((line) => {
    assert.notEqual(line, "");
    assert.equal(line.includes("\n"), false);
    const record = JSON.parse(line);
    validate(record, schema);
    return record;
  });

  assert.deepEqual(records.map(({ kind }) => kind), [
    "log", "event", "counter", "histogram", "span_start", "span_finish",
  ]);
  assert.deepEqual(records.map(({ sequence }) => sequence), [1, 2, 3, 4, 5, 6]);
  assert.deepEqual(records.map(({ name }) => name), [
    "log", "checkout.completed", "request.count", "request.duration", "request", "request",
  ]);
  assert.equal(records[0].message, "ready");
  assert.deepEqual(records[0].attributes, [
    { name: "secr", type: "string", value: "[REDACTED]" },
    { name: "both", type: "string", value: "abcd" },
  ]);
  assert.deepEqual(records[0].attribute_facts, { dropped: 1, truncated: 5, redacted: 1 });
  assert.deepEqual(records[0].source, {
    package: "",
    module: "a::b",
    file: "a/b.veln",
    start_line: 7,
    start_column: 2,
    end_line: 7,
    end_column: 256,
  });
  assert.deepEqual(records[1].source, {
    package: "<invalid-source>",
    module: "<invalid-source>",
    file: "<invalid-source>",
    start_line: 1,
    start_column: 2,
    end_line: 1,
    end_column: 3,
  });
  assert.equal(records[2].value, 3);
  assert.equal(records[3].value, 7);
  assert.deepEqual(records[4].span, {
    trace_id: "trace-parent",
    span_id: "captured-span-1",
    parent_span_id: "span-parent",
  });
  assert.deepEqual(records[4].links, [
    { trace_id: "trace-link", span_id: "span-link", sampled: false },
  ]);
  assert.deepEqual(records[5].span, records[4].span);
  assert.deepEqual(records[5].links, []);

  for (const { source } of records) {
    for (const component of [source.package, source.module, source.file]) {
      assert.equal(component.startsWith("/"), false);
      assert.equal(component.startsWith("\\\\"), false);
      assert.equal(/^[A-Za-z]:[\\/]/u.test(component), false);
      assert.equal(component.split(/[\\/]|::/u).some((segment) => segment === "." || segment === ".."), false);
    }
  }
  assert.equal(raw.includes("/machine/"), false);
  assert.equal(raw.includes("../private"), false);
  assert.equal(raw.includes("C:\\machine"), false);
});
