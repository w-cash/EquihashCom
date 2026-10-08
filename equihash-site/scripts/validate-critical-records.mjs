#!/usr/bin/env node

import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (file) => readFile(path.join(root, file), "utf8").then(JSON.parse);
const [audit, current, minersDoc, listingsDoc, vendorsDoc] = await Promise.all([
  read("ops/critical-record-audit.json"), read("data/current.json"), read("data/miners.json"), read("data/listings.json"), read("data/vendor-directory.json"),
]);
const snapshotRoot = `data/snapshots/${current.snapshot}`;
const [networkDoc, poolsDoc] = await Promise.all([read(`${snapshotRoot}/network.json`), read(`${snapshotRoot}/pools.json`)]);
const collections = {
  network: new Map(networkDoc.coins.map((row) => [row.id, row])),
  pool: new Map(poolsDoc.pools.map((row) => [row.id, row])),
  miner: new Map(minersDoc.miners.map((row) => [row.id, row])),
  listing: new Map(listingsDoc.listings.map((row) => [row.id, row])),
  vendor: new Map(vendorsDoc.vendors.map((row) => [row.id, row])),
};
if (audit.records.length !== 20) throw new Error(`audit sample must remain 20 records, found ${audit.records.length}`);
for (const item of audit.records) {
  const row = collections[item.type]?.get(item.id);
  if (!row) throw new Error(`missing audited ${item.type}: ${item.id}`);
  if (!item.risk || !item.qualification) throw new Error(`audit rationale missing: ${item.id}`);
  if (item.type === "network") {
    if (!row.equihash?.n || !row.equihash?.k || !row.status || !row.source_url) throw new Error(`network qualification incomplete: ${item.id}`);
    if (row.status === "ended" && (!row.status_note || !row.status_sources?.length)) throw new Error(`ended-network evidence missing: ${item.id}`);
  } else if (item.type === "pool") {
    if (!row.source_url || !row.coin_id || !Array.isArray(row.schemes) || typeof row.merged_mining?.supported === "undefined") throw new Error(`pool qualification incomplete: ${item.id}`);
  } else if (item.type === "miner") {
    if (!row.source_url || !row.spec_basis || !row.equihash || !(row.hashrate_ksol > 0) || !(row.watts > 0)) throw new Error(`miner qualification incomplete: ${item.id}`);
  } else if (item.type === "listing") {
    if (!row.source_url || !row.product_url || !row.price_currency || !row.availability || !(row.price_amount > 0)) throw new Error(`listing qualification incomplete: ${item.id}`);
  } else if (item.type === "vendor") {
    if (!row.record_type || !row.legal_identity_and_location_summary || !row.source_urls?.length || !row.last_verified) throw new Error(`vendor qualification incomplete: ${item.id}`);
  }
}
console.log(`Validated frozen critical-record sample: ${audit.records.length}/20 records.`);
