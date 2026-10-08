#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const file = path.join(root, "ops", "release-gates.json");
const document = JSON.parse(await readFile(file, "utf8"));
const valid = new Set(["pass", "pending", "blocked", "not_applicable"]);
const ids = new Set();
for (const gate of document.gates || []) {
  if (!gate.id || ids.has(gate.id)) throw new Error(`invalid or duplicate gate id: ${gate.id}`);
  ids.add(gate.id);
  if (!valid.has(gate.status)) throw new Error(`invalid status for ${gate.id}: ${gate.status}`);
  if (gate.required && gate.status === "not_applicable") throw new Error(`required gate cannot be not_applicable: ${gate.id}`);
  if (!Array.isArray(gate.evidence)) throw new Error(`evidence must be an array: ${gate.id}`);
}
const open = document.gates.filter((gate) => gate.required && gate.status !== "pass");
console.log(`${document.gates.length} gates checked; ${open.length} required gates remain open.`);
for (const gate of open) console.log(`- ${gate.id}: ${gate.status}`);
if (process.argv.includes("--release") && open.length) process.exitCode = 2;
