import fs from "node:fs";
import path from "node:path";
import { pathToFileURL } from "node:url";

function parseCount(value, context) {
  if (!/^\d+$/u.test(value)) {
    throw new Error(`Regenerate the shard coverage reports; ${context} has invalid count ${value}.`);
  }
  return BigInt(value);
}

function splitOnce(value, separator, context) {
  const index = value.indexOf(separator);
  if (index === -1) {
    throw new Error(`Regenerate the shard coverage reports; ${context} is malformed.`);
  }
  return [value.slice(0, index), value.slice(index + separator.length)];
}

function coverageFor(records, source) {
  let coverage = records.get(source);
  if (!coverage) {
    coverage = {
      functions: new Map(),
      functionCounts: new Map(),
      lines: new Map(),
      branches: new Map(),
    };
    records.set(source, coverage);
  }
  return coverage;
}

function addFunction(coverage, value, context) {
  const [lineValue, name] = splitOnce(value, ",", context);
  const line = Number(lineValue);
  if (!Number.isSafeInteger(line) || line < 1 || name === "") {
    throw new Error(`Regenerate the shard coverage reports; ${context} is malformed.`);
  }
  const previous = coverage.functions.get(name);
  if (previous !== undefined && previous !== line) {
    throw new Error(`Regenerate the shard coverage reports; ${context} defines ${name} on conflicting lines.`);
  }
  coverage.functions.set(name, line);
}

function addFunctionCount(coverage, value, context) {
  const [countValue, name] = splitOnce(value, ",", context);
  if (name === "") {
    throw new Error(`Regenerate the shard coverage reports; ${context} is malformed.`);
  }
  const count = parseCount(countValue, context);
  coverage.functionCounts.set(name, (coverage.functionCounts.get(name) ?? 0n) + count);
}

function addLine(coverage, value, context) {
  const fields = value.split(",");
  const line = Number(fields[0]);
  if (fields.length < 2 || fields.length > 3 || !Number.isSafeInteger(line) || line < 1) {
    throw new Error(`Regenerate the shard coverage reports; ${context} is malformed.`);
  }
  const count = parseCount(fields[1], context);
  const checksum = fields[2] ?? null;
  const previous = coverage.lines.get(line);
  if (previous && previous.checksum !== checksum) {
    throw new Error(`Regenerate the shard coverage reports; ${context} has conflicting checksums.`);
  }
  coverage.lines.set(line, { count: (previous?.count ?? 0n) + count, checksum });
}

function addBranch(coverage, value, context) {
  const fields = value.split(",");
  const line = Number(fields[0]);
  if (fields.length !== 4 || !Number.isSafeInteger(line) || line < 1) {
    throw new Error(`Regenerate the shard coverage reports; ${context} is malformed.`);
  }
  const key = fields.slice(0, 3).join(",");
  const count = fields[3] === "-" ? null : parseCount(fields[3], context);
  const previous = coverage.branches.get(key);
  coverage.branches.set(
    key,
    previous === undefined || previous === null
      ? count
      : count === null
        ? previous
        : previous + count,
  );
}

export function mergeLcovText(inputs) {
  const records = new Map();
  for (const { name, text } of inputs) {
    let coverage = null;
    for (const [index, line] of text.split(/\r?\n/u).entries()) {
      if (line === "" || line.startsWith("TN:")) {
        continue;
      }
      const context = `${name}:${index + 1}`;
      if (line.startsWith("SF:")) {
        const source = line.slice(3);
        if (source === "") {
          throw new Error(`Regenerate the shard coverage reports; ${context} has an empty source path.`);
        }
        coverage = coverageFor(records, source);
      } else if (line === "end_of_record") {
        coverage = null;
      } else if (/^(?:FNF|FNH|LF|LH|BRF|BRH):/u.test(line)) {
        continue;
      } else if (!coverage) {
        throw new Error(`Regenerate the shard coverage reports; ${context} appears outside a source record.`);
      } else if (line.startsWith("FN:")) {
        addFunction(coverage, line.slice(3), context);
      } else if (line.startsWith("FNDA:")) {
        addFunctionCount(coverage, line.slice(5), context);
      } else if (line.startsWith("DA:")) {
        addLine(coverage, line.slice(3), context);
      } else if (line.startsWith("BRDA:")) {
        addBranch(coverage, line.slice(5), context);
      } else {
        throw new Error(`Regenerate the shard coverage reports; ${context} uses unsupported record ${line}.`);
      }
    }
    if (coverage) {
      throw new Error(`Regenerate the shard coverage reports; ${name} is missing end_of_record.`);
    }
  }

  if (records.size === 0) {
    throw new Error("Regenerate the shard coverage reports; no source records were found.");
  }

  const output = [];
  for (const source of [...records.keys()].sort()) {
    const coverage = records.get(source);
    for (const name of coverage.functionCounts.keys()) {
      if (!coverage.functions.has(name)) {
        throw new Error(
          `Regenerate the shard coverage reports; ${source} has execution data for undefined function ${name}.`,
        );
      }
    }
    output.push("TN:", `SF:${source}`);
    for (const [name, line] of [...coverage.functions].sort((left, right) => {
      return left[1] - right[1] || left[0].localeCompare(right[0]);
    })) {
      output.push(`FN:${line},${name}`);
    }
    for (const [name, count] of [...coverage.functionCounts].sort(([left], [right]) => {
      return left.localeCompare(right);
    })) {
      output.push(`FNDA:${count},${name}`);
    }
    output.push(
      `FNF:${coverage.functions.size}`,
      `FNH:${[...coverage.functionCounts.values()].filter((count) => count > 0n).length}`,
    );
    for (const [key, count] of [...coverage.branches].sort(([left], [right]) => left.localeCompare(right))) {
      output.push(`BRDA:${key},${count === null ? "-" : count}`);
    }
    output.push(
      `BRF:${coverage.branches.size}`,
      `BRH:${[...coverage.branches.values()].filter((count) => count !== null && count > 0n).length}`,
    );
    for (const [line, { count, checksum }] of [...coverage.lines].sort((left, right) => left[0] - right[0])) {
      output.push(`DA:${line},${count}${checksum === null ? "" : `,${checksum}`}`);
    }
    output.push(
      `LF:${coverage.lines.size}`,
      `LH:${[...coverage.lines.values()].filter(({ count }) => count > 0n).length}`,
      "end_of_record",
    );
  }
  return `${output.join("\n")}\n`;
}

function collectLcovFiles(root) {
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectLcovFiles(entryPath));
    } else if (entry.isFile() && entry.name.endsWith(".info")) {
      files.push(entryPath);
    }
  }
  return files;
}

export function mergeLcovDirectory(inputRoot, outputPath) {
  const files = collectLcovFiles(inputRoot).sort();
  if (files.length === 0) {
    throw new Error(
      `Download every nextest coverage artifact before merging; no LCOV reports were found under ${inputRoot}.`,
    );
  }
  const inputs = files.map((file) => ({ name: file, text: fs.readFileSync(file, "utf8") }));
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, mergeLcovText(inputs));
}

function main() {
  const [inputRoot, outputPath] = process.argv.slice(2);
  if (!inputRoot || !outputPath) {
    throw new Error(
      "Pass the downloaded shard report directory and merged LCOV output path so coverage can be combined.",
    );
  }
  mergeLcovDirectory(inputRoot, outputPath);
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
