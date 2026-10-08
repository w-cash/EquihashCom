#!/usr/bin/env node
/** Classify the structured origin log emitted by src/main.rs.
 * Usage: node scripts/classify-http-status.mjs /var/log/equihash/access.log
 */
import fs from "node:fs";

const file = process.argv[2];
const raw = file ? fs.readFileSync(file, "utf8") : fs.readFileSync(0, "utf8");
const suspicious = /(?:\.env|wp-admin|wp-login|phpmyadmin|\.git|cgi-bin|vendor\/phpunit|actuator|boaform|HNAP1|\.php(?:$|\?))/i;
const rows = [];
for (const line of raw.split(/\r?\n/)) {
  const match = line.match(/"(?:GET|HEAD|POST|PUT|DELETE|OPTIONS)\s+([^\s"]+)[^"]*"\s+(301|308|404)\b.*?host="([^"]*)"\s+ref="([^"]*)"\s+ua="([^"]*)"/);
  if (!match) continue;
  let path = match[1];
  try { path = new URL(path, "https://equihash.com").pathname; } catch {}
  rows.push({ path, status: match[2], host: match[3], referrer: match[4], userAgent: match[5], probe: suspicious.test(path) });
}

function top(items, key, limit = 30) {
  const counts = new Map();
  for (const item of items) counts.set(item[key] || "(empty)", (counts.get(item[key] || "(empty)") || 0) + 1);
  return [...counts].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, limit).map(([value, count]) => ({ value, count }));
}

for (const status of ["404", "301", "308"]) {
  const selected = rows.filter((row) => row.status === status);
  if (!selected.length) continue;
  console.log(`\n${status}: ${selected.length} requests (${selected.filter((row) => row.probe).length} recognized probes)`);
  console.log("Legitimate-candidate paths:");
  console.table(top(selected.filter((row) => !row.probe), "path"));
  console.log("User agents:");
  console.table(top(selected, "userAgent", 15));
  console.log("Hosts:");
  console.table(top(selected, "host", 15));
  console.log("Referrers:");
  console.table(top(selected.filter((row) => !row.probe), "referrer", 15));
}
