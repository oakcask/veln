import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const recordSchemaUrl = new URL(
  "../crates/veln-stdlib/schemas/observability/v1/json-lines-record.schema.json",
  import.meta.url,
);
const healthSchemaUrl = new URL(
  "../crates/veln-stdlib/schemas/observability/v1/json-lines-exporter-health.schema.json",
  import.meta.url,
);
const recordSchema = JSON.parse(await readFile(recordSchemaUrl, "utf8"));
const healthSchema = parseLosslessJson(await readFile(healthSchemaUrl, "utf8"));
const encoderRecordsUrl = new URL(
  "../examples/specification/run/observability-json-lines-encoder/case-text/records.jsonl",
  import.meta.url,
);
const hostHealthUrl = new URL(
  "../examples/specification/run/observability-json-lines-host-boundary/case-text/health.jsonl",
  import.meta.url,
);

function parseLosslessJson(text) {
  let transformed = "";
  let inString = false;
  let escaped = false;
  for (let index = 0; index < text.length;) {
    const character = text[index];
    if (inString) {
      transformed += character;
      index += 1;
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === "\"") inString = false;
      continue;
    }
    if (character === "\"") {
      inString = true;
      transformed += character;
      index += 1;
      continue;
    }
    const number = text.slice(index).match(/^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/u)?.[0];
    if (number !== undefined) {
      transformed += number.includes(".") || /[eE]/u.test(number)
        ? number
        : `"__veln_bigint__${number}"`;
      index += number.length;
      continue;
    }
    transformed += character;
    index += 1;
  }
  return JSON.parse(transformed, (_name, value) => (
    typeof value === "string" && value.startsWith("__veln_bigint__")
      ? BigInt(value.slice("__veln_bigint__".length))
      : value
  ));
}

function resolve(reference, rootSchema) {
  assert.match(reference, /^#\/\$defs\/[a-z_]+$/u);
  return rootSchema.$defs[reference.slice("#/$defs/".length)];
}

function validate(value, rule, path = "record", rootSchema = rule) {
  if (rule.$ref) return validate(value, resolve(rule.$ref, rootSchema), path, rootSchema);
  for (const constraint of rule.allOf ?? []) validate(value, constraint, path, rootSchema);
  if (rule.anyOf) {
    const failures = [];
    for (const choice of rule.anyOf) {
      try {
        validate(value, choice, path, rootSchema);
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
      validate(value, rule.if, path, rootSchema);
    } catch {
      matches = false;
    }
    if (matches && rule.then) validate(value, rule.then, path, rootSchema);
    if (!matches && rule.else) validate(value, rule.else, path, rootSchema);
  }
  if (rule.not) {
    let matches = true;
    try {
      validate(value, rule.not, path, rootSchema);
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
    assert.ok(typeof value === "bigint" || Number.isSafeInteger(value), path);
    const integer = typeof value === "bigint" ? value : BigInt(value);
    if (rule.minimum !== undefined) {
      assert.ok(integer >= (typeof rule.minimum === "bigint" ? rule.minimum : BigInt(rule.minimum)), path);
    }
    if (rule.maximum !== undefined) {
      assert.ok(integer <= (typeof rule.maximum === "bigint" ? rule.maximum : BigInt(rule.maximum)), path);
    }
  }
  if (rule.type === "boolean") assert.equal(typeof value, "boolean", path);
  if (rule.type === "array") {
    assert.ok(Array.isArray(value), path);
    value.forEach((item, index) => validate(item, rule.items, `${path}[${index}]`, rootSchema));
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
      if (Object.hasOwn(value, name)) validate(value[name], child, `${path}.${name}`, rootSchema);
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
  assert.throws(() => validate({ ...representative("event"), schema_version: 2 }, recordSchema));
  assert.throws(() => validate({ ...representative("event"), unexpected: true }, recordSchema));
  assert.throws(() => validate({
    ...representative("event"),
    attributes: [{ name: "attempt", type: "int", value: "1" }],
  }, recordSchema));
  const logWithoutMessage = representative("log");
  delete logWithoutMessage.message;
  assert.throws(() => validate(logWithoutMessage, recordSchema));
  assert.throws(() => validate({ ...representative("event"), message: "unexpected" }, recordSchema));
  const counterWithoutValue = representative("counter");
  delete counterWithoutValue.value;
  assert.throws(() => validate(counterWithoutValue, recordSchema));
  assert.throws(() => validate({ ...representative("span_start"), trace: { trace_id: "trace-1", span_id: "span-1", sampled: true } }, recordSchema));
  assert.throws(() => validate({
    ...representative("log"),
    trace: { trace_id: "", span_id: "span-1", sampled: true },
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("log"),
    trace: { trace_id: "trace-1", span_id: "", sampled: true },
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("span_start"),
    span: { trace_id: "trace-1", span_id: "span-2", parent_span_id: "" },
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("span_start"),
    links: [{ trace_id: "", span_id: "span-3", sampled: false }],
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("event"),
    source: { ...representative("event").source, start_line: -1 },
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("event"),
    source: { ...representative("event").source, end_column: -1 },
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("event"),
    resource: { name: "", attributes: [] },
  }, recordSchema));
  assert.throws(() => validate({
    ...representative("event"),
    instrumentation_scope: { name: "", version: "1" },
  }, recordSchema));
});

test("six captured JVM encoder records satisfy schema, branch, framing, and path contracts", async () => {
  const raw = await readFile(encoderRecordsUrl, "utf8");
  assert.equal(raw.includes("\r"), false);
  assert.equal(raw.endsWith("\n"), true);
  const parts = raw.split("\n");
  assert.equal(parts.length, 7);
  assert.equal(parts[6], "");
  const records = parts.slice(0, 6).map((line) => {
    assert.notEqual(line, "");
    assert.equal(line.includes("\n"), false);
    const record = JSON.parse(line);
    validate(record, recordSchema);
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
    package: "acme:lib",
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
  assert.equal(raw.includes("acme:lib"), true);
  assert.equal(raw.includes("../private"), false);
  assert.equal(raw.includes("CON.veln"), false);
});

test("the captured exporter health summary has one closed int64 record and LF", async () => {
  const raw = await readFile(hostHealthUrl, "utf8");
  assert.equal(raw.includes("\r"), false);
  assert.equal(raw.endsWith("\n"), true);
  const parts = raw.split("\n");
  assert.equal(parts.length, 2);
  assert.notEqual(parts[0], "");
  assert.equal(parts[1], "");

  const health = parseLosslessJson(parts[0]);
  validate(health, healthSchema, "health");
  assert.throws(() => validate(health, recordSchema));
  assert.throws(() => validate(representative("event"), healthSchema, "health"));

  const maximum = { ...health, failed_exports: 9223372036854775807n };
  validate(maximum, healthSchema, "health");
  assert.throws(() => validate({ ...maximum, failed_exports: 9223372036854775808n }, healthSchema, "health"));
  assert.throws(() => validate({ ...health, schema_version: 2n }, healthSchema, "health"));
  assert.throws(() => validate({ ...health, kind: "event" }, healthSchema, "health"));
  assert.throws(() => validate({ ...health, failed_exports: -1n }, healthSchema, "health"));
  assert.throws(() => validate({ ...health, unexpected: 0n }, healthSchema, "health"));
  const missing = { ...health };
  delete missing.redacted_attributes;
  assert.throws(() => validate(missing, healthSchema, "health"));
});
