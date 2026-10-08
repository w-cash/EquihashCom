#!/usr/bin/env node
/** Build the append-only first-party market history used by the Zcash and Z15 pages.
 * Historical source snapshots remain authoritative; missing days stay missing.
 */
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

async function json(file, fallback = null) {
  try { return JSON.parse(await fs.readFile(file, "utf8")); } catch { return fallback; }
}

export async function captureMarketHistory(dataDir = process.env.DATA_DIR || path.join(ROOT, "data")) {
  const dir = path.resolve(dataDir);
  const snapshotsDir = path.join(dir, "snapshots");
  const ids = (await fs.readdir(snapshotsDir, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory())
    .map((entry) => entry.name)
    .sort();
  const miners = await json(path.join(dir, "miners.json"), { miners: [] });
  const z15 = miners.miners.find((miner) => miner.id === "antminer-z15-pro");
  const hashrate = z15?.hashrate_ksol ?? null;
  const watts = z15?.watts ?? null;
  const previous = await json(path.join(dir, "market-history.json"), { network_and_economics: [], offer_observations: [] });
  // Preserve observations even after old publication snapshots are pruned. Snapshot id plus
  // observation time is the immutable identity; a rerun replaces the same point, never invents one.
  const pointsByKey = new Map((previous.network_and_economics || []).map((point) => [
    `${point.snapshot}\u0000${point.observed_at || ""}`,
    point,
  ]));
  for (const snapshot of ids) {
    const network = await json(path.join(snapshotsDir, snapshot, "network.json"));
    const coin = network?.coins?.find((row) => row.id === "zcash");
    if (!coin) continue;
    const observedAt = coin.network?.hashrate_observed_at || network.generated_at || null;
    const net = coin.network?.hashrate ?? null;
    const difficulty = coin.network?.difficulty ?? null;
    const price = coin.price_usd ?? null;
    const reward = coin.block_reward_miner?.value ?? null;
    const blocktime = coin.network?.block_time_target_s ?? null;
    let revenue = null;
    let margin = null;
    if ([hashrate, net, price, reward, blocktime].every((value) => Number.isFinite(value) && value > 0)) {
      const coinsDay = (hashrate * 1000 / net) * (86400 / blocktime) * reward * 0.99;
      revenue = coinsDay * price;
      if (Number.isFinite(watts)) margin = revenue - (watts / 1000) * 24 * 0.08;
    }
    const point = {
      snapshot,
      observed_at: observedAt,
      coin_id: "zcash",
      network_hashrate_sol_s: net,
      difficulty,
      price_usd: price,
      miner_id: "antminer-z15-pro",
      miner_hashrate_ksol_s: hashrate,
      miner_watts: watts,
      pool_fee_pct: 1,
      electricity_usd_kwh: 0.08,
      revenue_day_usd: revenue,
      operating_margin_day_usd: margin,
      network_source_url: coin.network?.hashrate_source ?? null,
      price_source_url: coin.price_source ?? null,
    };
    pointsByKey.set(`${point.snapshot}\u0000${point.observed_at || ""}`, point);
  }

  const points = [...pointsByKey.values()].sort((a, b) =>
    String(a.observed_at).localeCompare(String(b.observed_at)) || String(a.snapshot).localeCompare(String(b.snapshot))
  );
  const listings = await json(path.join(dir, "listings.json"), { listings: [] });
  const observed = [...(previous.offer_observations || [])];
  for (const listing of listings.listings || []) {
    if (listing.miner_id !== "antminer-z15-pro") continue;
    const at = listing.price_observed_at || listing.observed_at;
    if (!at || observed.some((row) => row.listing_id === listing.id && row.observed_at === at)) continue;
    observed.push({
      observed_at: at,
      listing_id: listing.id,
      vendor_id: listing.vendor_id,
      miner_id: listing.miner_id,
      price_amount: listing.price_amount ?? null,
      price_currency: listing.price_currency ?? null,
      vat_included: listing.price_includes_vat ?? null,
      availability: listing.availability ?? null,
      source_url: listing.source_url ?? listing.product_url ?? null,
    });
  }
  observed.sort((a, b) => String(a.observed_at).localeCompare(String(b.observed_at)));
  const output = {
    schema_version: "1.0",
    _doc: "First-party observations retained from published Equihash.com snapshots. Gaps are not interpolated. Z15 economics use the assumptions stored on every point.",
    generated_at: new Date().toISOString(),
    network_and_economics: points,
    offer_observations: observed,
  };
  const destination = path.join(dir, "market-history.json");
  const temporary = `${destination}.tmp-${process.pid}-${Date.now()}`;
  await fs.writeFile(temporary, JSON.stringify(output, null, 2) + "\n");
  await fs.rename(temporary, destination);
  return output;
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  captureMarketHistory().then((result) => {
    console.log(`Captured ${result.network_and_economics.length} network snapshots and ${result.offer_observations.length} offer observations.`);
  }).catch((error) => { console.error(error); process.exit(1); });
}
