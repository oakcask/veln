import assert from "node:assert/strict";
import test from "node:test";

import {
  failureMessage,
  unguardedRustTestCommands,
  workflowRunCommands,
} from "./check-rust-test-memory-guards.mjs";

test("extracts inline and block workflow commands with source lines", () => {
  const commands = workflowRunCommands(`steps:
  - run: cargo test --workspace
  - run: |
      prepare
      cargo nextest run --workspace
  - uses: actions/checkout@v4
`);

  assert.deepEqual(commands, [
    { line: 2, command: "cargo test --workspace" },
    { line: 4, command: "prepare" },
    { line: 5, command: "cargo nextest run --workspace" },
  ]);
});

test("accepts guarded Cargo and nextest test commands", () => {
  const source = `steps:
  - run: bash scripts/ci-run cargo test --workspace
  - run: bash scripts/ci-run cargo nextest run --workspace
  - run: bash scripts/ci-run cargo llvm-cov --no-report nextest --workspace
`;

  assert.deepEqual(unguardedRustTestCommands(source), []);
});

test("rejects the local-only runner because it does not fix CI concurrency", () => {
  const source = `steps:
  - run: bash scripts/agent-run cargo test --workspace
`;

  assert.deepEqual(unguardedRustTestCommands(source), [
    {
      line: 2,
      command: "bash scripts/agent-run cargo test --workspace",
    },
  ]);
});

test("rejects each unguarded Rust test command", () => {
  const source = `steps:
  - run: cargo test --workspace
  - run: cargo nextest run --workspace
  - run: cargo llvm-cov --no-report nextest --workspace
`;

  assert.deepEqual(unguardedRustTestCommands(source), [
    { line: 2, command: "cargo test --workspace" },
    { line: 3, command: "cargo nextest run --workspace" },
    { line: 4, command: "cargo llvm-cov --no-report nextest --workspace" },
  ]);
});

test("ignores non-test Cargo commands and YAML prose", () => {
  const source = `description: run cargo test locally when debugging
steps:
  - run: cargo run --workspace
  - name: cargo test documentation
    uses: actions/checkout@v4
`;

  assert.deepEqual(unguardedRustTestCommands(source), []);
});

test("failure tells maintainers how and why to guard the command", () => {
  const message = failureMessage({ file: "test.yaml", line: 2 });

  assert.match(message, /run this Rust test command through `bash scripts\/ci-run`/);
  assert.match(message, /cannot exhaust the host before the job timeout/);
});
