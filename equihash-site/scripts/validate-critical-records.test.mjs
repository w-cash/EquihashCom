import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

test("the frozen 20-record audit resolves against the published candidate", () => {
  const run = spawnSync(process.execPath, [path.join(root, "scripts", "validate-critical-records.mjs")], { encoding: "utf8" });
  assert.equal(run.status, 0, run.stderr);
  assert.match(run.stdout, /20\/20 records/);
});
