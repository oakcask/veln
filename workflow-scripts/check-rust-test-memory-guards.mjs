import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const rustTestPatterns = [
  /\bcargo\s+test(?:\s|$)/,
  /\bcargo\s+nextest\s+run(?:\s|$)/,
  /\bcargo\s+llvm-cov\b.*\bnextest(?:\s|$)/,
];
const memoryGuardPattern = /\bbash\s+scripts\/ci-run(?:\s|$)/;

function indentation(line) {
  return line.length - line.trimStart().length;
}

export function workflowRunCommands(source) {
  const lines = source.split(/\r?\n/);
  const commands = [];

  for (let index = 0; index < lines.length; index += 1) {
    const match = /^(\s*)(?:-\s+)?run:\s*(.*)$/.exec(lines[index]);
    if (!match) {
      continue;
    }

    const value = match[2].trim();
    if (!/^[|>][+-]?$/.test(value)) {
      commands.push({ line: index + 1, command: value });
      continue;
    }

    const runIndent = match[1].length;
    for (index += 1; index < lines.length; index += 1) {
      const command = lines[index];
      if (command.trim() && indentation(command) <= runIndent) {
        index -= 1;
        break;
      }
      if (command.trim()) {
        commands.push({ line: index + 1, command: command.trim() });
      }
    }
  }

  return commands;
}

export function unguardedRustTestCommands(source) {
  return workflowRunCommands(source).filter(
    ({ command }) =>
      rustTestPatterns.some((pattern) => pattern.test(command)) &&
      !memoryGuardPattern.test(command),
  );
}

function yamlFiles(target) {
  const status = fs.statSync(target);
  if (status.isFile()) {
    return /\.ya?ml$/.test(target) ? [target] : [];
  }
  return fs
    .readdirSync(target, { withFileTypes: true })
    .flatMap((entry) => yamlFiles(path.join(target, entry.name)));
}

export function checkFiles(targets) {
  const failures = [];
  for (const file of targets.flatMap(yamlFiles).sort()) {
    const source = fs.readFileSync(file, "utf8");
    for (const failure of unguardedRustTestCommands(source)) {
      failures.push({ file, ...failure });
    }
  }
  return failures;
}

export function failureMessage(failure) {
  return `${failure.file}:${failure.line}: run this Rust test command through \`bash scripts/ci-run\`; the CI runner fixes Cargo concurrency and process memory so tests cannot exhaust the host before the job timeout`;
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : "";
if (invokedPath === fileURLToPath(import.meta.url)) {
  const targets = process.argv.slice(2);
  if (targets.length === 0) {
    console.error("usage: node workflow-scripts/check-rust-test-memory-guards.mjs <workflow-or-action>...");
    process.exitCode = 64;
  } else {
    const failures = checkFiles(targets);
    for (const failure of failures) {
      console.error(failureMessage(failure));
    }
    if (failures.length > 0) {
      process.exitCode = 1;
    }
  }
}
