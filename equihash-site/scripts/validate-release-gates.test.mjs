import test from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const script = path.join(root, "scripts", "validate-release-gates.mjs");

test("gate report is valid and does not pretend the release is approved", () => {
  const report = spawnSync(process.execPath, [script], { encoding: "utf8" });
  assert.equal(report.status, 0, report.stderr);
  assert.match(report.stdout, /required gates remain open/);

  const release = spawnSync(process.execPath, [script, "--release"], { encoding: "utf8" });
  assert.equal(release.status, 2, "release mode must fail closed while evidence is missing");
  assert.match(release.stdout, /source-rights: blocked/);
  assert.match(release.stdout, /comprehension-8-of-10: blocked/);
});
