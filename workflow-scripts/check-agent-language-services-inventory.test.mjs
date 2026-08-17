import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import test from "node:test";
import {
  buildInventory,
  INVENTORY_PATH,
  SOURCE_PATH,
  validateDiffScope,
  validateInventory,
  validateLedger,
  validateRepository,
} from "./check-agent-language-services-inventory.mjs";

const source = fs.readFileSync(SOURCE_PATH, "utf8");
const frozen = JSON.parse(fs.readFileSync(INVENTORY_PATH, "utf8"));

test("accepts the committed frozen inventory and ledger schema", () => {
  assert.deepEqual(validateRepository(process.cwd()).errors, []);
});

test("inventory retains every Q01 through Q22 identity", () => {
  const identities = new Set(frozen.items.flatMap((item) => item.identity));
  for (let index = 1; index <= 22; index += 1) {
    assert.equal(identities.has(`Q${String(index).padStart(2, "0")}`), true);
  }
});

test("inventory retains the closed matrices and named conformance inputs", () => {
  assert.equal(records("Definition And Reference Coverage", "list-item").length, 11);
  assert.equal(records("Topic Catalog", "list-item").length, 10);
  assert.equal(records("Next Slice: Saved Workspace Function References", "table-row").length, 14);
  assert.equal(records("Tools", "table-row").length, 8);
  assert.equal(records("Resources", "list-item").length, 4);
  for (const [heading, rowCount] of [
    ["Server And Project Selection", 13],
    ["Diagnostics And Navigation", 13],
    ["Virtual Locations And Package Documentation", 11],
    ["Published Language Reference", 10],
    ["Plugin", 6],
  ]) {
    assert.equal(records(heading, "table-row").length, rowCount);
  }

  const text = frozen.items.map((item) => item.text).join("\n");
  for (const identity of [
    "invalid_path",
    "invalid_position",
    "invalid_query",
    "source_required",
    "project_not_selected",
    "project_ambiguous",
    "snapshot_changed",
    "invalid_cursor",
    "stale_snapshot",
    "resource_not_found",
    "generation_failed",
    "resource_capacity",
    "incompatible_version",
    "UTF-8",
    "UTF-16",
    "UTF-32",
    "compatibility.toml",
  ]) {
    assert.equal(text.includes(identity), true, `missing named input ${identity}`);
  }
});

test("rejects a changed source digest", () => {
  const changed = source.replace("Add a local MCP server", "Add one local MCP server");
  assert.match(validateInventory(changed, frozen).join("\n"), /source digest changed/u);
});

test("rejects a missing or duplicate inventory item", () => {
  const missing = clone(frozen);
  missing.items.splice(2, 1);
  assert.match(validateInventory(source, missing).join("\n"), /missing inventory item ALS-0003/u);

  const duplicate = clone(frozen);
  duplicate.items.push(clone(duplicate.items[0]));
  assert.match(validateInventory(source, duplicate).join("\n"), /duplicate inventory item ALS-0001/u);
});

test("rejects a missing child and mismatched exact child count", () => {
  const inventory = clone(frozen);
  const parent = mixedParent(inventory);
  parent.children.splice(0, 1);
  assert.match(
    validateInventory(source, inventory).join("\n"),
    /child count does not match|missing or non-contiguous child/u,
  );
});

test("rejects gap, overlap, and out-of-range child spans", async (t) => {
  await t.test("gap", () => {
    const inventory = clone(frozen);
    const parent = mixedParent(inventory);
    parent.children[1].spans[0][0] += 1;
    assert.match(validateInventory(source, inventory).join("\n"), /uncovered source scalar/u);
  });
  await t.test("overlap", () => {
    const inventory = clone(frozen);
    const parent = mixedParent(inventory);
    parent.children[1].spans[0][0] -= 1;
    assert.match(validateInventory(source, inventory).join("\n"), /overlapping child spans/u);
  });
  await t.test("out of range", () => {
    const inventory = clone(frozen);
    const parent = mixedParent(inventory);
    parent.children.at(-1).spans[0][1] += 1;
    assert.match(validateInventory(source, inventory).join("\n"), /out-of-range scalar span/u);
  });
});

test("rejects a wrong lifecycle and a child containing mixed lifecycle text", () => {
  const wrong = clone(frozen);
  const parent = mixedParent(wrong);
  parent.children[0].lifecycle = otherLifecycle(parent.children[0].lifecycle);
  assert.match(validateInventory(source, wrong).join("\n"), /wrong lifecycle/u);

  const mixed = clone(frozen);
  const mixedItem = mixedParent(mixed);
  mixedItem.children[0].text = mixedItem.text;
  assert.match(validateInventory(source, mixed).join("\n"), /mixes lifecycle statements/u);
});

test("rejects an uncovered parent lifecycle statement", () => {
  const inventory = clone(frozen);
  const parent = mixedParent(inventory);
  const absent = parent.children[1].lifecycle;
  for (const child of parent.children) {
    if (child.lifecycle === absent) child.lifecycle = parent.children[0].lifecycle;
  }
  assert.match(validateInventory(source, inventory).join("\n"), /uncovered .* statement/u);
});

test("accepts one explicit valid ledger mapping per inventory leaf", () => {
  assert.deepEqual(validateLedger(frozen, validLedger(frozen)), []);
});

test("rejects direct parent, missing, duplicate, wildcard, and removed ledger mappings", async (t) => {
  await t.test("direct parent", () => {
    const ledger = validLedger(frozen);
    ledger.entries[0].source_id = mixedParent(frozen).id;
    assert.match(validateLedger(frozen, ledger).join("\n"), /maps parent .* directly/u);
  });
  await t.test("missing", () => {
    const ledger = validLedger(frozen);
    const removed = ledger.entries.pop();
    assert.match(validateLedger(frozen, ledger).join("\n"), new RegExp(`missing leaf ${removed.source_id}`, "u"));
  });
  await t.test("duplicate", () => {
    const ledger = validLedger(frozen);
    ledger.entries.push(clone(ledger.entries[0]));
    assert.match(validateLedger(frozen, ledger).join("\n"), /more than once/u);
  });
  await t.test("wildcard", () => {
    const ledger = validLedger(frozen);
    ledger.entries[0].source_id = "ALS-*";
    assert.match(validateLedger(frozen, ledger).join("\n"), /range, wildcard, or catch-all/u);
  });
  await t.test("removed", () => {
    const ledger = validLedger(frozen);
    ledger.entries[0].lifecycle = "removed";
    assert.match(validateLedger(frozen, ledger).join("\n"), /removes frozen leaf/u);
  });
});

test("rejects invalid ledger lifecycle and destination", () => {
  const ledger = validLedger(frozen);
  ledger.entries[0].lifecycle = "unknown";
  ledger.entries[1].destination.path = "outside.md";
  const errors = validateLedger(frozen, ledger).join("\n");
  assert.match(errors, /invalid lifecycle/u);
  assert.match(errors, /invalid destination path/u);
});

test("diff guard permits inventory work and rejects frozen or executable scopes", () => {
  assert.deepEqual(
    validateDiffScope([
      INVENTORY_PATH,
      "workflow-scripts/check-agent-language-services-inventory.mjs",
    ]),
    [],
  );
  const errors = validateDiffScope([
    SOURCE_PATH,
    "crates/veln-mcp/src/main.rs",
    "crates/veln-cli/tests/toolchain-case-semantics.baseline",
    "examples/specification/mcp/workspace-lifecycle/case.toml",
  ]).join("\n");
  assert.match(errors, /changes frozen umbrella proposal/u);
  assert.match(errors, /changes protected MCP or semantic evidence/u);
});

test("inventory construction is deterministic", () => {
  assert.deepEqual(buildInventory(source), buildInventory(source));
});

function validLedger(inventory) {
  const entries = [];
  for (const item of inventory.items) {
    for (const leaf of inventoryLeaves(item)) {
      const lifecycle = leaf.lifecycle === "removed" ? "planned" : leaf.lifecycle;
      const kinds = {
        current: "specification",
        completed: "implementation-record",
        planned: "proposal",
      };
      const directories = {
        current: "specification",
        completed: "reference/implemented-proposals",
        planned: "proposals",
      };
      entries.push({
        source_id: leaf.id,
        lifecycle,
        destination: {
          kind: kinds[lifecycle],
          path: `docs/${directories[lifecycle]}/destination.md`,
          ...(lifecycle === "current" ? { evidence: [`test:${leaf.id}`] } : {}),
        },
      });
    }
  }
  return { format: 1, entries };
}

function inventoryLeaves(item) {
  if (item.children === undefined) return [item];
  return item.children.flatMap(inventoryLeaves);
}

function mixedParent(inventory) {
  const parent = inventory.items.find((item) => item.children?.length >= 2);
  assert.ok(parent, "fixture needs a mixed-lifecycle parent");
  return parent;
}

function records(heading, kind) {
  return frozen.items.filter((item) => item.heading === heading && item.kind === kind);
}

function otherLifecycle(lifecycle) {
  return lifecycle === "planned" ? "current" : "planned";
}

function clone(value) {
  return structuredClone(value);
}
