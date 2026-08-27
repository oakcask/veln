import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import { mergeLcovDirectory, mergeLcovText } from "./merge-lcov.mjs";

const shardOne = `TN:
SF:crates/example/src/lib.rs
FN:2,example::covered
FN:8,example::uncovered
FNDA:1,example::covered
FNDA:0,example::uncovered
FNF:2
FNH:1
DA:2,1,checksum
DA:3,0
LF:2
LH:1
BRDA:3,0,0,0
BRDA:3,0,1,-
BRF:2
BRH:0
end_of_record
`;

const shardTwo = `TN:
SF:crates/example/src/lib.rs
FN:2,example::covered
FN:8,example::uncovered
FNDA:2,example::covered
FNDA:1,example::uncovered
FNF:2
FNH:2
DA:2,2,checksum
DA:3,1
LF:2
LH:2
BRDA:3,0,0,4
BRDA:3,0,1,1
BRF:2
BRH:2
end_of_record
`;

test("merges shard counts and recalculates summaries", () => {
  assert.equal(
    mergeLcovText([
      { name: "shard-1.info", text: shardOne },
      { name: "shard-2.info", text: shardTwo },
    ]),
    `TN:
SF:crates/example/src/lib.rs
FN:2,example::covered
FN:8,example::uncovered
FNDA:3,example::covered
FNDA:1,example::uncovered
FNF:2
FNH:2
BRDA:3,0,0,4
BRDA:3,0,1,1
BRF:2
BRH:2
DA:2,3,checksum
DA:3,1
LF:2
LH:2
end_of_record
`,
  );
});

test("writes reports discovered below artifact directories", (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "merge-lcov-"));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.mkdirSync(path.join(root, "shard-1"));
  fs.mkdirSync(path.join(root, "shard-2"));
  fs.writeFileSync(path.join(root, "shard-1", "lcov.info"), shardOne);
  fs.writeFileSync(path.join(root, "shard-2", "lcov.info"), shardTwo);

  const output = path.join(root, "merged", "lcov.info");
  mergeLcovDirectory(root, output);

  assert.match(fs.readFileSync(output, "utf8"), /FNDA:3,example::covered/u);
});

test("rejects conflicting line checksums", () => {
  const conflicting = shardTwo.replace("DA:2,2,checksum", "DA:2,2,different");
  assert.throws(
    () =>
      mergeLcovText([
        { name: "shard-1.info", text: shardOne },
        { name: "shard-2.info", text: conflicting },
      ]),
    /Regenerate the shard coverage reports; shard-2\.info:9 has conflicting checksums\./u,
  );
});

test("requires at least one downloaded report", (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "merge-lcov-empty-"));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));

  assert.throws(
    () => mergeLcovDirectory(root, path.join(root, "lcov.info")),
    /Download every nextest coverage artifact before merging/u,
  );
});
