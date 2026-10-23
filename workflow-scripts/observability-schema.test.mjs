import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const schemaUrl = new URL(
  "../crates/veln-stdlib/schemas/observability/v1/json-lines-record.schema.json",
  import.meta.url,
);
const schema = JSON.parse(await readFile(schemaUrl, "utf8"));
const hostBoundaryUrl = new URL(
  "../examples/specification/run/observability-json-lines-host-boundary/case-text/stderr.txt",
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

test("representative JSON Lines observation records satisfy the checked schema", () => {
  for (const kind of ["log", "event", "counter", "histogram", "span_start", "span_finish"]) {
    validate(representative(kind), schema);
  }
});

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
});

test("the checked JVM host-boundary record satisfies the schema and path hygiene", async () => {
  const lines = (await readFile(hostBoundaryUrl, "utf8")).trimEnd().split("\n");
  assert.ok(lines.length > 0);
  for (const line of lines) {
    const record = JSON.parse(line);
    validate(record, schema);
    assert.equal(record.source.file.startsWith("/"), false);
    assert.equal(/^[A-Za-z]:[\\/]/u.test(record.source.file), false);
  }
});

test("representative source names are repository-relative or virtual", () => {
  for (const name of ["src/main.veln", "dependency/pkg.veln", "<observe>"]) {
    assert.equal(name.startsWith("/"), false);
    assert.equal(/^[A-Za-z]:[\\/]/u.test(name), false);
  }
});
