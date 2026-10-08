import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { captureMarketHistory } from "./capture-market-history.mjs";

test("market history retains real snapshots, does not invent gaps, and deduplicates offers", async () => {
  const dir = await fs.mkdtemp(path.join(os.tmpdir(), "equihash-history-"));
  await fs.mkdir(path.join(dir, "snapshots", "20261001T000000Z"), { recursive: true });
  await fs.mkdir(path.join(dir, "snapshots", "20261003T000000Z"), { recursive: true });
  const network = (at, net) => ({ generated_at: at, coins: [{
    id: "zcash", price_usd: 100, price_source: "https://example.test/price",
    block_reward_miner: { value: 1.25 },
    network: { hashrate: net, difficulty: 10, block_time_target_s: 75, hashrate_observed_at: at, hashrate_source: "https://example.test/network" },
  }] });
  await fs.writeFile(path.join(dir, "snapshots", "20261001T000000Z", "network.json"), JSON.stringify(network("2026-10-01T00:00:00Z", 30e9)));
  await fs.writeFile(path.join(dir, "snapshots", "20261003T000000Z", "network.json"), JSON.stringify(network("2026-10-03T00:00:00Z", 31e9)));
  await fs.writeFile(path.join(dir, "miners.json"), JSON.stringify({ miners: [{ id: "antminer-z15-pro", hashrate_ksol: 840, watts: 2780 }] }));
  await fs.writeFile(path.join(dir, "listings.json"), JSON.stringify({ listings: [{ id: "offer", vendor_id: "seller", miner_id: "antminer-z15-pro", price_amount: 5000, price_currency: "USD", observed_at: "2026-10-03T00:00:00Z" }] }));

  const first = await captureMarketHistory(dir);
  const second = await captureMarketHistory(dir);
  assert.equal(first.network_and_economics.length, 2, "the absent 2 October snapshot stays absent");
  assert.equal(first.offer_observations.length, 1);
  assert.equal(second.offer_observations.length, 1, "same listing observation is not duplicated");
  assert.equal(second.schema_version, "1.0");
  assert.ok(second.network_and_economics.every((point) => Number.isFinite(point.revenue_day_usd)));

  await fs.rm(path.join(dir, "snapshots", "20261001T000000Z"), { recursive: true });
  const afterPrune = await captureMarketHistory(dir);
  assert.equal(afterPrune.network_and_economics.length, 2, "published observations survive snapshot pruning");
});
