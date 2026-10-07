#!/usr/bin/env node
/**
 * Refresh the public NiceHash Equihash order-book snapshot used by /hashpower.
 * The file contains aggregates only; individual order and account identifiers are discarded.
 */
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT = path.join(ROOT, "data", "hashpower.json");
const API = "https://api2.nicehash.com/main/api/v2/hashpower/orderBook";
const ALGORITHMS = "https://api2.nicehash.com/main/api/v2/mining/algorithms";
const UA = "equihash.com data refresh (+https://equihash.com/sources)";

async function json(url) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(), 20_000);
  try {
    const res = await fetch(url, { headers: { Accept: "application/json", "User-Agent": UA }, signal: ctrl.signal });
    if (!res.ok) throw new Error(`${url}: HTTP ${res.status}`);
    return await res.json();
  } finally {
    clearTimeout(timer);
  }
}

const number = (value, label) => {
  const n = Number(value);
  if (!Number.isFinite(n) || n < 0) throw new Error(`invalid ${label}: ${value}`);
  return n;
};

const first = await json(`${API}?algorithm=EQUIHASH&page=0&size=100`);
const market = first?.stats?.BTC;
if (!market) throw new Error("NiceHash response has no BTC Equihash market");

const algorithmResponse = await json(ALGORITHMS);
const algorithm = algorithmResponse?.miningAlgorithms?.find((a) => a.algorithm === "EQUIHASH");
if (!algorithm?.enabled || !algorithm?.ordersEnabled) throw new Error("NiceHash Equihash market is not enabled");

const pages = Math.max(1, Number(market.pagination?.totalPageCount) || 1);
let orders = [...(market.orders || [])];
for (let page = 1; page < pages; page++) {
  const body = await json(`${API}?algorithm=EQUIHASH&page=${page}&size=100`);
  orders.push(...(body?.stats?.BTC?.orders || []));
}
orders = orders.filter((order) => order.alive === true);
const standard = orders.filter((order) => order.type === "STANDARD");
const fixed = orders.filter((order) => order.type === "FIXED");
const top = (rows) => rows.length ? Math.max(...rows.map((row) => number(row.price, "order price"))) : null;

const snapshot = {
  _doc: "Aggregate NiceHash Equihash hashpower marketplace snapshot. NiceHash is a marketplace, not a mining pool. Prices are order-book bids, not guaranteed miner revenue. Individual orders are intentionally omitted.",
  nicehash: {
    provider: "NiceHash",
    algorithm: "EQUIHASH",
    market: "BTC",
    observed_at: market.updatedTs || new Date().toISOString(),
    total_speed_gsol: number(market.totalSpeed, "total speed"),
    active_orders: orders.length,
    fixed_orders: fixed.length,
    standard_orders: standard.length,
    top_standard_btc_per_gsol_day: top(standard),
    top_fixed_btc_per_gsol_day: top(fixed),
    display_speed_unit: market.displayMarketFactor || algorithm.displayMarketFactor || "GSol/s",
    display_price_unit: `BTC/${market.displayPriceFactor || algorithm.displayPriceFactor || "GSol"}/day`,
    source_url: `${API}?algorithm=EQUIHASH&page=0&size=100`,
    marketplace_url: "https://www.nicehash.com/marketplace",
    connection_guide_url: "https://www.nicehash.com/blog/post/notice-for-equihash-miners-xnsub-support",
    stratum_url: "stratum+tcp://equihash.auto.nicehash.com:9200#xnsub",
    note: "Order-book prices move continuously. The top bid is not a forecast and does not include the seller's fees or switching effects."
  }
};

const tmp = `${OUT}.tmp-${process.pid}`;
await fs.writeFile(tmp, `${JSON.stringify(snapshot, null, 2)}\n`, "utf8");
await fs.rename(tmp, OUT);
console.log(`Wrote ${OUT}: ${snapshot.nicehash.total_speed_gsol} GSol/s across ${orders.length} active orders`);
