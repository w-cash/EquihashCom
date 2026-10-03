#!/usr/bin/env node
/**
 * equihash.com data refresh.
 *
 * Pulls live, public data and writes:
 *   data/pools.json    – every pool row (miningpoolstats + verified non-MPS pools)
 *   data/network.json  – per-coin network stats + Equihash parameters
 *   data/meta.json     – run metadata (fetched_at, sources, errors)
 *
 * Curated (hand-verified, never overwritten here) inputs live in data/curated/:
 *   manual-pools.json  – pools that have no machine-readable public API
 *   coin-status.json   – coins whose PoW has ended (with citations)
 * and the static files data/archive.json, data/miners.json, data/research.json.
 *
 * Rules: never invent numbers. Unknown => null (rendered as "n/a"); zero stays zero.
 *
 * Provenance: every pool row carries <field>_source and <field>_observed_at for hashrate, fee,
 * miners, blocks and min_payout, plus `basis` for the hashrate (listed_pools | operator_reported |
 * estimated). Network rows carry the same for hashrate, difficulty and height, with basis
 * "network". A field's timestamp only moves when that field was actually fetched: refreshing a
 * fee or a block height never makes a hashrate look newer than it is.
 * Publishing: the three generated files are built in memory, written to a staging directory,
 * validated (schema, upstream URLs, critical sources, sharp drops against the current snapshot)
 * and only then published atomically as data/snapshots/<id>/ plus a swapped data/current.json
 * (see scripts/snapshot.mjs). A failed validation keeps the previous snapshot live and exits 2.
 *
 * Usage: npm run refresh                      (node scripts/refresh-data.mjs)
 *        node scripts/refresh-data.mjs --live-only   re-read only the live sources
 *        node scripts/refresh-data.mjs --force       publish even if validation fails
 *        node scripts/refresh-data.mjs --list        list snapshots
 *        node scripts/refresh-data.mjs --rollback [id]   point current.json at an earlier snapshot
 */
import { pathToFileURL } from "node:url";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import * as snap from "./snapshot.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
// DATA_DIR overrides where data lives (e.g. /srv/equihash/data, shared by every release).
const DATA = process.env.DATA_DIR ? path.resolve(process.env.DATA_DIR) : path.join(ROOT, "data");
const UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
const MPS = "https://miningpoolstats.stream";
const MPS_DATA = "https://data.miningpoolstats.stream/data";
// Pages that are no longer in the MPS coin index but still have a page (PoW ended).
const EXTRA_MPS_PAGES = ["horizen", "flux"];
// These two left the miningpoolstats index when their PoW ended, so the index can't name them.
const EXTRA_MPS_NAMES = { horizen: "Horizen", flux: "Flux" };

const errors = [];
const sources = new Map();
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const nowIso = () => new Date().toISOString();

function addSource(id, label, url) {
  sources.set(id, { id, label, url });
}

async function get(url, { referer, json = true, timeout = 25000, retries = 2 } = {}) {
  for (let attempt = 0; attempt <= retries; attempt++) {
    const ctrl = new AbortController();
    const t = setTimeout(() => ctrl.abort(), timeout);
    try {
      const res = await fetch(url, {
        headers: { "User-Agent": UA, Accept: json ? "application/json,*/*" : "text/html,*/*", ...(referer ? { Referer: referer } : {}) },
        signal: ctrl.signal,
      });
      clearTimeout(t);
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      const text = await res.text();
      return { body: json ? JSON.parse(text) : text, fetched_at: nowIso() };
    } catch (e) {
      clearTimeout(t);
      if (attempt === retries) throw new Error(`${url}: ${e.message}`);
      await sleep(1500 * (attempt + 1));
    }
  }
}

const num = (v) => {
  if (v === null || v === undefined || v === "") return null;
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : null;
};
/** MPS uses -1 / -2 as "unknown / hidden". */
const mpsNum = (v) => {
  const n = num(v);
  return n === null || n < 0 ? null : n;
};

/** Record where one field came from and when it was observed. Touches nothing else on the row. */
export function stamp(row, field, source, at) {
  row[`${field}_source`] = source ?? null;
  row[`${field}_observed_at`] = at ?? null;
  return row;
}
const POOL_FIELDS = ["hashrate", "fee", "miners", "blocks", "min_payout"];
/** Basis of a pool row's hashrate figure. */
export function hashrateBasis(row) {
  if (row.hashrate === null || row.hashrate === undefined) return null;
  return row.hashrate_is_reported ? "operator_reported" : "listed_pools";
}
/** A live fee update (e.g. the ZecWec overview API): fee and its provenance only. The row's
 * hashrate, its timestamp and the row-level fetched_at are left exactly as they were. */
export function applyLiveFee(row, feePct, source, at) {
  if (feePct === null || feePct === undefined || !Number.isFinite(feePct)) return row;
  row.fee_pct = feePct;
  row.schemes = (row.schemes || []).map((s) => ({ ...s, fee_pct: feePct }));
  return stamp(row, "fee", source, at);
}

function parseEquihash(algo) {
  const a = String(algo || "");
  if (!/equihash/i.test(a)) return null;
  if (/\+/.test(a)) return { n: null, k: null, label: a };
  const m = a.match(/(\d{2,3})\s*,\s*(\d)/);
  if (m) return { n: Number(m[1]), k: Number(m[2]), label: a };
  // MPS labels plain "Equihash" for the original (200,9) parameter set (its "ASIC - Equihash" group).
  return { n: 200, k: 9, label: a };
}

const PRETTY = {
  "viabtc.com": "ViaBTC", "f2pool.com": "F2Pool", "antpool.com": "AntPool", "luxor.tech": "Luxor",
  "foundrydigital.com": "Foundry", "2miners.com": "2Miners", "pool.kryptex.com": "Kryptex", "kryptex.com": "Kryptex",
  "binance.com": "Binance Pool", "poolin.com": "Poolin", "mining-dutch.nl": "Mining-Dutch", "nicehash.com": "NiceHash",
  "kupool.com": "KuPool", "trustpool.cc": "TrustPool", "tpool.io": "TPool", "zpool.ca": "zpool", "zergpool.com": "ZergPool",
  "suprnova.cc": "Suprnova", "herominers.com": "HeroMiners", "cruxpool.com": "CruxPool", "solopool.org": "SoloPool.org",
};

function hostOf(url) {
  try { return new URL(url).hostname.replace(/^www\./, ""); } catch { return null; }
}
function baseDomain(host) {
  if (!host) return null;
  const parts = host.split(".");
  return parts.slice(-2).join(".");
}
function prettyName(p) {
  const host = hostOf(p.url);
  const id = p.pool_id || host;
  let name = PRETTY[id] || PRETTY[host] || PRETTY[baseDomain(host)] || id || host || "Unknown";
  const solo = /solo/i.test(p.url) || /solo/i.test(p.jsonpool_id || "");
  if (solo && !/solo/i.test(name)) name += " (solo)";
  return name;
}

function parseSchemes(feetype, fee) {
  // e.g. "PPS+%4|PPLNS%2", "PPS%3|PPS+%3", "SOLO", "PPS|", "D-PPS%1.00|PROP%1.00|SOLO%1.00"
  if (!feetype) return [];
  return String(feetype)
    .split("|")
    .map((s) => s.trim())
    .filter(Boolean)
    .map((s) => {
      const [scheme, f] = s.split("%");
      return { scheme: scheme.trim().toUpperCase(), fee_pct: f !== undefined && f !== "" ? num(f) : (num(fee) ?? null) };
    });
}

function parseRegion(country) {
  if (!country) return { region: null, regions: [], country_code: null };
  const [code, label] = String(country).split("|");
  const regions = (label || "").split(",").map((s) => s.trim()).filter(Boolean);
  return { region: label ? label.replace(/\s+,\s+/g, ", ").trim() : null, regions, country_code: code || null };
}

async function mpsPage(page) {
  const { body: html } = await get(`${MPS}/${page}`, { json: false });
  const lt = html.match(/var last_time = "(\d+)"/);
  const grp = html.match(/var coin_grup = "([^"]*)"/);
  return { last_time: lt ? lt[1] : null, coin_group: grp ? grp[1] : null };
}

async function refreshMps() {
  addSource("mps", "miningpoolstats.stream (pool + network JSON)", `${MPS}/`);
  const home = await mpsPage("");
  const { body: coinsIdx, fetched_at: idxAt } = await get(`${MPS_DATA}/coins_data.js?t=${home.last_time}`, { referer: `${MPS}/` });
  const eqCoins = coinsIdx.data.filter((c) => /equihash/i.test(String(c.algo)));
  const pages = [...new Set([...eqCoins.map((c) => c.page), ...EXTRA_MPS_PAGES])];
  const coins = [];
  const pools = [];
  for (const page of pages) {
    try {
      const meta = await mpsPage(page);
      await sleep(400);
      const ref = `${MPS}/${page}`;
      const { body: d, fetched_at } = await get(`${MPS_DATA}/${page}.js?t=${meta.last_time}`, { referer: ref });
      await sleep(400);
      let price = null;
      try {
        const { body: p } = await get(`${MPS_DATA}/price/${page}.js?t=${meta.last_time}`, { referer: ref });
        price = num(p.price_USD);
      } catch (e) { errors.push(`price ${page}: ${e.message}`); }
      await sleep(400);
      const idx = coinsIdx.data.find((c) => c.page === page);
      const eq = parseEquihash(d.algo);
      const netHash = mpsNum(d.hashrate);
      const coin = {
        id: page,
        name: idx?.name ?? EXTRA_MPS_NAMES[page] ?? d.symbol,
        symbol: d.symbol,
        algo_label: d.algo ?? null,
        equihash: eq ? { n: eq.n, k: eq.k } : null,
        z15_compatible: eq && eq.n === 200 && eq.k === 9 ? true : eq && eq.n === null ? null : false,
        hardware: idx?.typ ?? null,
        mps_group: meta.coin_group,
        in_mps_index: Boolean(idx),
        network: {
          hashrate: netHash && netHash > 0 ? netHash : null,
          unit: d.unit ?? "Sol/s",
          // Where the number was read (set by stamp() below); miningpoolstats' own attribution
          // is kept beside it, since its figure can differ from that upstream's current one.
          hashrate_source: null,
          hashrate_upstream: d.hashrate_src ?? null,
          difficulty: mpsNum(d.difficulty) || null,
          height: mpsNum(d.height),
          block_time_target_s: mpsNum(d.block_time_target),
          block_time_avg_s: mpsNum(d.block_time_average) || null,
          pools_hashrate: mpsNum(d.poolshash) || null,
        },
        price_usd: price ?? (num(d.price) || null),
        // Price file first; otherwise the "price" field in the coin's own pool file.
        price_source: price !== null ? `${MPS_DATA}/price/${page}.js` : num(d.price) ? `${MPS_DATA}/${page}.js` : null,
        block_reward_miner: null,
        mps_pool_count: Array.isArray(d.data) ? d.data.length : 0,
        source_url: ref,
        data_url: `${MPS_DATA}/${page}.js?t=${meta.last_time}`,
        fetched_at,
        status: "active",
        status_note: null,
        status_sources: [],
        cross_checks: [],
      };
      const ndu = `${MPS_DATA}/${page}.js?t=${meta.last_time}`;
      const nw = coin.network;
      stamp(nw, "hashrate", nw.hashrate !== null ? ndu : null, nw.hashrate !== null ? fetched_at : null);
      nw.basis = nw.hashrate !== null ? "network" : null;
      stamp(nw, "difficulty", nw.difficulty !== null ? ndu : null, nw.difficulty !== null ? fetched_at : null);
      stamp(nw, "height", nw.height !== null ? ndu : null, nw.height !== null ? fetched_at : null);
      stamp(nw, "block_time", nw.block_time_avg_s !== null ? ndu : null, nw.block_time_avg_s !== null ? fetched_at : null);
      coins.push(coin);
      for (const p of d.data || []) {
        const hr = mpsNum(p.hashrate);
        const schemes = parseSchemes(p.feetype, p.fee);
        const reg = parseRegion(p.country);
        const lbt = num(p.lastblocktime);
        pools.push({
          id: `${page}:${p.pool_id || hostOf(p.url)}:${p.id ?? p.url}`,
          name: prettyName(p),
          coin: d.symbol,
          coin_id: page,
          url: p.url,
          hashrate: hr,
          hashrate_unit: d.unit ?? "Sol/s",
          network_share_pct: hr !== null && netHash ? (hr / netHash) * 100 : null,
          miners: mpsNum(p.miners),
          workers: mpsNum(p.workers),
          fee_pct: num(p.fee) ?? (schemes.length === 1 ? schemes[0].fee_pct : null),
          schemes,
          payout_schemes: [...new Set(schemes.map((s) => s.scheme))],
          min_payout: mpsNum(p.minpay),
          min_payout_unit: d.symbol,
          blocks_last_1000: mpsNum(p.blocks_1000),
          last_block_height: mpsNum(p.lastblock),
          last_block_time: lbt ? new Date(lbt * 1000).toISOString() : null,
          ...reg,
          // MPS "merge" = merged coin(s); "mergetype" alone (e.g. "DUAL") names no coin, so it isn't treated as merged mining.
          merged_mining: p.merge
            ? { supported: true, coins: String(p.merge).split(/[,|\s]+/).filter(Boolean), note: `as reported by miningpoolstats${p.mergetype ? ` (type ${p.mergetype})` : ""}` }
            : { supported: null, coins: [], note: p.mergetype ? `miningpoolstats lists merge type "${p.mergetype}" with no merged coin` : null },
          solo: schemes.some((s) => s.scheme === "SOLO") || /solo/i.test(p.url),
          active: hr !== null && hr > 0,
          source_name: "miningpoolstats.stream",
          source_url: ref,
          data_url: `${MPS_DATA}/${page}.js`,
          from_miningpoolstats: true,
          fetched_at,
          notes: hr === null ? "Hashrate not published (MPS reports it as hidden/unknown)." : null,
        });
        const row = pools[pools.length - 1];
        for (const f of POOL_FIELDS) stamp(row, f, `${MPS_DATA}/${page}.js`, fetched_at);
        row.basis = hashrateBasis(row);
      }
      console.log(`  mps ${page}: ${d.data?.length ?? 0} pools`);
    } catch (e) {
      // A coin page is a critical source (its pool rows would vanish); pages kept only for coins
      // whose PoW ended are not (see snapshot.isCritical).
      errors.push(`${EXTRA_MPS_PAGES.includes(page) ? "mps (ended coin)" : "mps"} ${page}: ${e.message}`);
      console.warn(`  ! mps ${page}: ${e.message}`);
    }
  }
  return { coins, pools, idxAt };
}

/** zpool.ca – multi-coin pool with a public currencies API (hashrate in Sol/s for Equihash algos). */
async function refreshZpool(coins, pools) {
  const url = "https://zpool.ca/api/currencies";
  addSource("zpool", "zpool.ca public API", url);
  try {
    const { body, fetched_at } = await get(url);
    const { body: status } = await get("https://zpool.ca/api/status");
    for (const [sym, c] of Object.entries(body)) {
      if (!/^equihash/i.test(String(c.algo))) continue;
      const coin = coins.find((x) => x.symbol === sym.split("-")[0] && (sym.includes("eq192") ? x.equihash?.n === 192 : sym.includes("eq200") ? x.equihash?.n === 200 : true));
      if (!coin) continue; // only coins we can tie to a tracked Equihash coin
      const already = pools.some((p) => p.coin_id === coin.id && /zpool\.ca/.test(p.url || ""));
      if (already) continue;
      const algoFee = num(status?.[c.algo]?.fees);
      const hr = num(c.hashrate);
      pools.push({
        id: `${coin.id}:zpool.ca:${sym}`,
        name: "zpool",
        coin: coin.symbol,
        coin_id: coin.id,
        url: "https://zpool.ca/",
        hashrate: hr,
        hashrate_unit: "Sol/s",
        network_share_pct: hr !== null && coin.network.hashrate ? (hr / coin.network.hashrate) * 100 : null,
        miners: null,
        workers: num(c.workers),
        fee_pct: algoFee,
        schemes: [],
        payout_schemes: [],
        min_payout: null,
        min_payout_unit: null,
        blocks_last_1000: null,
        last_block_height: null,
        last_block_time: null,
        region: "Global (multi-region)",
        regions: [],
        country_code: null,
        merged_mining: { supported: null, coins: [], note: null },
        solo: false,
        active: hr !== null && hr > 0,
        source_name: "zpool.ca API",
        source_url: url,
        data_url: url,
        from_miningpoolstats: false,
        fetched_at,
        notes: `Multi-coin pool (algo '${c.algo}', coin key '${sym}'). Payouts can be auto-exchanged into the coin you select; payout scheme not published in the API. ${c["24h_blocks"] !== undefined ? `Blocks in last 24h: ${c["24h_blocks"]}.` : ""}`.trim(),
      });
      const row = pools[pools.length - 1];
      for (const f of POOL_FIELDS) stamp(row, f, url, fetched_at);
      row.basis = hashrateBasis(row);
    }
    console.log("  zpool ok");
  } catch (e) {
    errors.push(`zpool: ${e.message}`);
    console.warn(`  ! zpool: ${e.message}`);
  }
}

/** ZergPool – public API (was returning Cloudflare 522 during v1 research). Added only if reachable. */
async function refreshZergpool(coins, pools) {
  const url = "https://zergpool.com/api/currencies";
  try {
    const { body, fetched_at } = await get(url, { retries: 1, timeout: 15000 });
    addSource("zergpool", "ZergPool public API", url);
    for (const [sym, c] of Object.entries(body)) {
      if (!/^equihash/i.test(String(c.algo))) continue;
      const coin = coins.find((x) => x.symbol === sym.split("-")[0]);
      if (!coin || pools.some((p) => p.coin_id === coin.id && /zergpool/.test(p.url || ""))) continue;
      const hr = num(c.hashrate);
      pools.push({
        id: `${coin.id}:zergpool.com:${sym}`, name: "ZergPool", coin: coin.symbol, coin_id: coin.id, url: "https://zergpool.com/",
        hashrate: hr, hashrate_unit: "Sol/s", network_share_pct: hr !== null && coin.network.hashrate ? (hr / coin.network.hashrate) * 100 : null,
        miners: null, workers: num(c.workers), fee_pct: num(c.fees), schemes: [], payout_schemes: [],
        min_payout: null, min_payout_unit: null, blocks_last_1000: null, last_block_height: null, last_block_time: null,
        region: "Global (multi-region)", regions: [], country_code: null, merged_mining: { supported: null, coins: [], note: null },
        solo: false, active: hr !== null && hr > 0, source_name: "ZergPool API", source_url: url, data_url: url, from_miningpoolstats: false, fetched_at,
        notes: `Multi-coin pool (algo '${c.algo}').`,
      });
      const row = pools[pools.length - 1];
      for (const f of POOL_FIELDS) stamp(row, f, url, fetched_at);
      row.basis = hashrateBasis(row);
    }
    console.log("  zergpool ok");
  } catch (e) {
    errors.push(`zergpool (optional): ${e.message}`);
    console.warn(`  ! zergpool unreachable (optional): ${e.message}`);
  }
}

/** Manual (hand-verified) pools, enriched with live public endpoints where they exist. */
async function refreshManual(coins, pools) {
  const manual = JSON.parse(await fs.readFile(path.join(DATA, "curated", "manual-pools.json"), "utf8"));
  // ZecWec public overview endpoint: fee policy + chain heights (hashrate is not public without login).
  let zecwec = null;
  try {
    const r = await get("https://pool.zecwec.com/api/v1/overview");
    zecwec = r;
    addSource("zecwec", "ZecWec public overview API", "https://pool.zecwec.com/api/v1/overview");
  } catch (e) { errors.push(`zecwec overview: ${e.message}`); }
  for (const m of manual.pools) {
    const coin = coins.find((c) => c.symbol === m.coin);
    const row = { ...m };
    delete row.verified_at; delete row.live; delete row.live_fields;
    const liveFields = m.live_fields || (m.live ? ["fee"] : []);
    row.coin_id = coin?.id ?? m.coin_id ?? m.coin.toLowerCase();
    // The row as a whole was hand-checked at verified_at; that stays its fetched_at.
    row.fetched_at = m.verified_at;
    // Hand-verified fields default to the verification time unless the curated row says otherwise.
    for (const f of POOL_FIELDS) {
      if (liveFields.includes(f) && f !== "fee") continue; // filled by its live source below
      if (row[`${f}_source`] === undefined) row[`${f}_source`] = m.source_url ?? null;
      if (row[`${f}_observed_at`] === undefined) row[`${f}_observed_at`] = m.verified_at ?? null;
    }
    if (row.basis === undefined) row.basis = hashrateBasis(row);
    if (m.live === "zecwec" && zecwec) {
      const fee = m.coin === "WEC" ? num(zecwec.body.wec_fee_bps) : num(zecwec.body.zec_fee_bps);
      // Only the fee is fetched here. The hashrate (if live) comes from live-sources.json and keeps its own time.
      applyLiveFee(row, fee === null ? null : fee / 100, "https://pool.zecwec.com/api/v1/overview", zecwec.fetched_at);
      row.data_url = "https://pool.zecwec.com/api/v1/overview";
    }
    row.network_share_pct = row.hashrate !== null && coin?.network?.hashrate ? (row.hashrate / coin.network.hashrate) * 100 : null;
    pools.push(row);
  }
  console.log(`  manual: ${manual.pools.length} rows`);
  return zecwec;
}

/** Cross-checks + block rewards from 2Miners public APIs (used only as cited secondary data). */
async function refresh2Miners(coins) {
  const targets = [
    { symbol: "ZEC", base: "https://zec.2miners.com" },
    { symbol: "BTG", base: "https://btg.2miners.com" },
  ];
  for (const t of targets) {
    const coin = coins.find((c) => c.symbol === t.symbol);
    if (!coin) continue;
    try {
      const { body: s, fetched_at } = await get(`${t.base}/api/stats`);
      const node = s.nodes?.[0];
      if (node) {
        coin.cross_checks.push({
          label: `2Miners node (${t.base.replace("https://", "")})`,
          url: `${t.base}/api/stats`,
          network_hashrate: num(node.networkhashps),
          difficulty: num(node.difficulty),
          height: num(node.height),
          avg_block_time_s: num(node.avgBlockTime),
          fetched_at,
        });
      }
      const { body: b, fetched_at: bAt } = await get(`${t.base}/api/blocks`);
      const last = (b.matured || []).find((x) => !x.orphan);
      if (last && num(last.reward)) {
        coin.block_reward_miner = {
          value: num(last.reward) / 1e8,
          unit: coin.symbol,
          note: `Reward credited to the pool for block ${last.height} (miner share of subsidy plus fees), from 2Miners' matured block list.`,
          source_url: `${t.base}/api/blocks`,
          fetched_at: bAt,
        };
      }
      addSource(`2miners-${t.symbol}`, `2Miners ${t.symbol} public API`, `${t.base}/api/stats`);
    } catch (e) {
      errors.push(`2miners ${t.symbol}: ${e.message}`);
    }
  }
}

/** The calculator's note on the WEC block reward. The subsidy is S(h) = floor(2,199,023,255 × h /
 *  40,000) atoms up to height 40,000 (Wcash protocol specification, "Monetary policy"), so the
 *  last block's reward is already out of date one block later. Never states a supply figure. */
export function wcashRewardNote(height) {
  const h = Number(height);
  const ramp = Number.isFinite(h) && h < 40000;
  return `Coinbase reward of block ${height} as shown by the explorer (subsidy plus fees). ` +
    (ramp
      ? "The subsidy ramps up every block until height 40,000 (by about 0.00055 WEC per block), so a later block pays more; see the spec's emission section."
      : "The subsidy changes every block after height 40,000; see the spec's emission section.");
}

/** Wcash (WEC) network: explorer status + latest block. Only neutral chain stats are kept. */
async function refreshWcash(coins, zecwec) {
  const base = "https://wcashexplorer.com/api/v1";
  const coin = {
    id: "wcash",
    name: "Wcash",
    symbol: "WEC",
    algo_label: "Equihash (200,9) via Zcash AuxPoW v2 (merged-mined, Zcash parent)",
    equihash: { n: 200, k: 9 },
    z15_compatible: true,
    hardware: "ASIC",
    mps_group: null,
    in_mps_index: false,
    // The explorer publishes no network hashrate, so it stays null (shown as "Network estimate:
    // unavailable"). What the listed pool reports is a separate figure, derived from the pool rows.
    network: { hashrate: null, unit: "Sol/s", hashrate_source: null, hashrate_observed_at: null, basis: null, difficulty: null, height: null, block_time_target_s: 75, block_time_avg_s: null, pools_hashrate: null },
    price_usd: null,
    price_source: null,
    block_reward_miner: null,
    mps_pool_count: 0,
    source_url: "https://wcashexplorer.com/",
    data_url: `${base}/status`,
    fetched_at: null,
    status: "active",
    status_note: "Not listed on miningpoolstats. The explorer publishes no network hashrate; the network estimate comes from ZecWec's public endpoint (previous 120 blocks). ZecWec is the only listed pool.",
    status_sources: [{ label: "Wcash protocol specification", url: "https://w.cash/whitepaper" }],
    cross_checks: [],
    merged_mining_parent: "ZEC",
  };
  try {
    const { body: st, fetched_at } = await get(`${base}/status`);
    const s = st.data || {};
    coin.network.height = num(s.nodeHeight ?? s.indexedHeight);
    coin.network.difficulty = num(s.difficulty);
    coin.network.block_time_avg_s = num(s.observedSpacingSeconds);
    coin.network.block_time_target_s = num(s.targetSpacingSeconds) ?? 75;
    coin.fetched_at = fetched_at;
    if (coin.network.difficulty !== null) stamp(coin.network, "difficulty", `${base}/status`, fetched_at);
    if (coin.network.height !== null) stamp(coin.network, "height", `${base}/status`, fetched_at);
    if (coin.network.block_time_avg_s !== null) stamp(coin.network, "block_time", `${base}/status`, fetched_at);
    const { body: bl, fetched_at: bAt } = await get(`${base}/blocks?limit=1`);
    const b = bl.data?.[0];
    if (b?.reward?.decimal) {
      coin.block_reward_miner = {
        value: Number(b.reward.decimal),
        unit: "WEC",
        note: wcashRewardNote(b.height),
        source_url: `${base}/blocks?limit=1`,
        fetched_at: bAt,
      };
    }
    addSource("wcashexplorer", "Wcash explorer API (height, difficulty, block spacing)", `${base}/status`);
  } catch (e) {
    errors.push(`wcash explorer: ${e.message}`);
  }
  if (zecwec?.body?.wcash_height && !coin.network.height) {
    coin.network.height = num(zecwec.body.wcash_height);
    stamp(coin.network, "height", "https://pool.zecwec.com/api/v1/overview", zecwec.fetched_at);
  }
  coins.push(coin);
}

// ---------- live sources (data/curated/live-sources.json; the server polls the same list) ----------

/** Parse one live-source response. Mirrors src/live.rs: only available === true with a finite,
 *  non-negative hashrate and a sane unix updated_at counts. */
export function parseLiveReading(src, body, nowMs = Date.now()) {
  const f = { available: "available", hashrate: "hashrate_sol_s", updated_at: "updated_at", window_seconds: "window_seconds", sample_blocks: "sample_blocks", height: "height", ...(src.fields || {}) };
  if (!body || body[f.available] !== true) return { status: "unavailable" };
  const h = body[f.hashrate];
  if (typeof h !== "number" || !isFinite(h) || h < 0) return { status: "error", error: `${src.id}: no valid ${f.hashrate}` };
  const ts = body[f.updated_at];
  if (typeof ts !== "number" || !isFinite(ts)) return { status: "error", error: `${src.id}: no valid ${f.updated_at}` };
  if (ts * 1000 > nowMs + 5 * 60 * 1000) return { status: "error", error: `${src.id}: ${f.updated_at} is in the future` };
  const u = (k) => (typeof body[k] === "number" && isFinite(body[k]) && body[k] >= 0 ? Math.floor(body[k]) : null);
  return {
    status: "ok",
    reading: { hashrate: h, observed_at: new Date(Math.floor(ts) * 1000).toISOString().replace(".000Z", "Z"), window_seconds: u(f.window_seconds), sample_blocks: u(f.sample_blocks), height: u(f.height) },
  };
}

/** Write one good reading into its pool row or coin, with its own provenance. */
export function applyLiveReading(target, src, r) {
  if (src.target === "pool") {
    Object.assign(target, {
      hashrate: r.hashrate, hashrate_unit: src.unit || "Sol/s", hashrate_is_reported: false, basis: "pool_api",
      hashrate_source: src.url, hashrate_observed_at: r.observed_at, hashrate_window_s: r.window_seconds, live_source: src.id,
    });
  } else if (src.target === "network") {
    const n = (target.network ||= {});
    Object.assign(n, {
      hashrate: r.hashrate, unit: src.unit || "Sol/s", basis: "network", hashrate_source: src.url, hashrate_upstream: null,
      hashrate_observed_at: r.observed_at, hashrate_sample_blocks: r.sample_blocks, live_source: src.id,
    });
    // Height stays credited to the chain source (explorer) when it has one; see src/live.rs.
    if (r.height !== null && (n.height === null || n.height === undefined) && (!n.height_observed_at || new Date(r.observed_at) >= new Date(n.height_observed_at))) {
      n.height = r.height;
      stamp(n, "height", src.url, r.observed_at);
    }
  }
}

/** Carry the previous good live values over when the endpoint is unavailable this time. */
export function keepPreviousLive(target, prev, src) {
  if (!prev) return false;
  if (src.target === "pool" && prev.live_source === src.id && prev.hashrate != null) {
    for (const k of ["hashrate", "hashrate_unit", "hashrate_is_reported", "basis", "hashrate_source", "hashrate_observed_at", "hashrate_window_s", "live_source"]) target[k] = prev[k];
    return true;
  }
  if (src.target === "network" && prev.network?.live_source === src.id && prev.network.hashrate != null) {
    target.network ||= {};
    for (const k of ["hashrate", "unit", "basis", "hashrate_source", "hashrate_upstream", "hashrate_observed_at", "hashrate_sample_blocks", "live_source"]) target.network[k] = prev.network[k];
    return true;
  }
  return false;
}

async function readJson(file, fallback) {
  try { return JSON.parse(await fs.readFile(file, "utf8")); } catch { return fallback; }
}

/** Pool rows a live source feeds: `pool_ids` (one endpoint, several rows, e.g. a merge-mining
 *  pool whose single hashrate covers ZEC and WEC), plus `pool_id` for older configs. Mirrors
 *  Source::pool_ids in src/live.rs. */
export function livePoolIds(src) {
  const ids = [...(Array.isArray(src.pool_ids) ? src.pool_ids : [])];
  if (src.pool_id && !ids.includes(src.pool_id)) ids.push(src.pool_id);
  return ids;
}

/** Apply one live result to every target it feeds. One reading per source, so rows fed by the same
 *  endpoint always carry the same value and time. Returns the ids it could not find. */
export function applyLiveResult(src, res, coins, pools, previous) {
  const ids = src.target === "pool" ? livePoolIds(src) : [src.coin_id];
  const missing = [], kept = [];
  for (const id of ids) {
    const target = src.target === "pool" ? pools.find((p) => p.id === id) : coins.find((c) => c.id === id);
    if (!target) { missing.push(id); continue; }
    if (res.status === "ok") applyLiveReading(target, src, res.reading);
    else {
      const prev = src.target === "pool" ? previous.pools.find((p) => p.id === id) : previous.coins.find((c) => c.id === id);
      if (keepPreviousLive(target, prev, src)) kept.push(id);
    }
  }
  return { missing, kept };
}

async function refreshLiveSources(coins, pools, previous) {
  const cfg = await readJson(path.join(DATA, "curated", "live-sources.json"), { sources: [] });
  for (const src of cfg.sources || []) {
    let res;
    try { const { body } = await get(src.url, { retries: 1, timeout: 15000 }); res = parseLiveReading(src, body); }
    catch (e) { res = { status: "error", error: `${src.id}: ${e.message}` }; }
    const { missing, kept } = applyLiveResult(src, res, coins, pools, previous);
    for (const id of missing) errors.push(`live ${src.id}: no ${src.target} ${id}`);
    if (res.status === "ok") addSource(src.id, src.label || src.id, src.url);
    else errors.push(`live ${src.id}: ${res.error || "available: false"}${kept.length ? ` (kept previous reading for ${kept.join(", ")})` : ""}`);
  }
}

async function readPrevious() {
  const cur = await snap.readCurrent(DATA);
  const pf = cur["pools.json"] || { pools: [] };
  const nf = cur["network.json"] || { coins: [] };
  return { pools: pf.pools || [], coins: nf.coins || [], pf, nf, meta: cur["meta.json"], files: cur };
}

// ---------- pool permalinks (data/curated/permalinks.json; read by src/data.rs) ----------

/** Same as data::slugify: ASCII letters and digits, lowercased; any run of other characters is one dash. */
export function slugify(s) {
  let out = "", dash = false;
  for (const ch of String(s)) {
    if (/^[A-Za-z0-9]$/.test(ch)) { out += ch.toLowerCase(); dash = false; }
    else if (!dash && out) { out += "-"; dash = true; }
  }
  return out.replace(/-+$/, "");
}
/** Same as data::computed_slug: the slug a pool's URL was built from before permalinks. */
export function computedSlug(p) {
  const host = String(p.url || "").replace(/^(https:\/\/)+/, "").replace(/^(http:\/\/)+/, "").replace(/^(www\.)+/, "");
  return slugify(`${p.coin_id}-${p.name}-${host}`);
}
/** Give each pool id that has none a permanent slug, once. Existing entries are never changed or
 *  removed (a retired pool's slug stays reserved), so a rename or a new domain keeps the URL.
 *  Returns the ids that were added. */
export function assignPermalinks(pools, file) {
  file.pools ||= {};
  file.aliases ||= {};
  const taken = new Set([...Object.values(file.pools), ...Object.keys(file.aliases)]);
  const added = [];
  for (const p of pools) {
    if (!p.id || file.pools[p.id]) continue;
    const base = computedSlug(p) || slugify(p.id) || "pool";
    let s = base;
    for (let i = 2; taken.has(s); i++) s = `${base}-${i}`;
    taken.add(s);
    file.pools[p.id] = s;
    added.push(p.id);
  }
  return added;
}
async function updatePermalinks(pools) {
  const file = path.join(DATA, "curated", "permalinks.json");
  const cur = await readJson(file, null);
  const data = cur || { _note: "Permanent pool URLs: /pool/<slug>. Assigned once per pool id by the refresh and never changed; add \"aliases\": {\"old-slug\": \"slug\"} to redirect an old URL.", pools: {}, aliases: {} };
  const added = assignPermalinks(pools, data);
  if (added.length || !cur) {
    const tmp = `${file}.tmp-${process.pid}`;
    await fs.writeFile(tmp, JSON.stringify(data, null, 2) + "\n");
    await fs.rename(tmp, file);
    console.log(`  permalinks: ${added.length} new`);
  }
  return added;
}

/** Validate and publish one refresh as a new snapshot; exit 2 (previous snapshot kept) on problems. */
async function publishSnapshot(pf, nf, meta, previous) {
  const files = { "pools.json": JSON.stringify(pf, null, 2), "network.json": JSON.stringify(nf, null, 2), "meta.json": JSON.stringify(meta, null, 2) };
  const res = await snap.publish(DATA, files, { previous: previous.files, force: process.argv.includes("--force") });
  if (!res.published) {
    console.error("Not published: the new data failed validation, so the previous snapshot stays live.\n" + res.problems.map((p) => "  - " + p).join("\n") + "\nRe-run later, or with --force after checking.");
    process.exitCode = 2;
    return null;
  }
  if (res.problems.length) console.warn("Published with --force despite:\n" + res.problems.map((p) => "  - " + p).join("\n"));
  console.log(`Published snapshot ${res.id}.`);
  return res.id;
}

// data/curated/coins.json: coins miningpoolstats doesn't cover, added verbatim (same shape as network.json).
async function addCuratedCoins(coins) {
  let extra = [];
  try { extra = JSON.parse(await fs.readFile(path.join(DATA, "curated", "coins.json"), "utf8")).coins || []; } catch {}
  for (const c of extra) {
    if (!c.id || coins.some((x) => x.id === c.id)) continue;
    coins.push({ status: "active", status_note: null, status_sources: [], cross_checks: [], network: {}, ...c });
  }
  if (extra.length) console.log(`  curated coins: ${extra.length}`);
}

async function applyCoinStatus(coins) {
  const cs = JSON.parse(await fs.readFile(path.join(DATA, "curated", "coin-status.json"), "utf8"));
  for (const s of cs.coins) {
    const c = coins.find((x) => x.id === s.id);
    if (!c) continue;
    c.status = s.status;
    c.status_note = s.note;
    c.status_sources = s.sources;
    if (s.status !== "active") c.network.hashrate = null;
  }
}

// Network share: pool hashrate / network hashrate. Small coins' network estimates are noisy and can
// be below the sum of what pools report (shares > 100%). Then shares are measured against the
// listed pools' total and the coin is marked share_basis = "pools". Mirrors src/data.rs.
export function normaliseShares(coins, pools) {
  for (const c of coins) {
    const own = pools.filter((p) => p.coin_id === c.id);
    const sum = own.reduce((a, p) => a + (p.hashrate > 0 ? p.hashrate : 0), 0);
    const net = c.network?.hashrate > 0 ? c.network.hashrate : null;
    let basis = "none", denom = null;
    if (net !== null && sum <= net * 1.02) { basis = "network"; denom = net; }
    else if (sum > 0) { basis = "pools"; denom = sum; }
    c.share_basis = basis;
    // Never above 100%: pool and network figures can cover different windows.
    for (const p of own) p.network_share_pct = p.hashrate !== null && p.hashrate !== undefined && denom ? Math.min(100, (p.hashrate / denom) * 100) : null;
  }
}

/** Parameter-set key: 200,9 first (Z15-series), then other exact (n,k) sets, unknown last. */
export function paramKey(c) {
  const n = c.equihash?.n, k = c.equihash?.k;
  if (n == null || k == null) return [2, 0, 0];
  return n === 200 && k === 9 ? [0, n, k] : [1, n, k];
}
/** Sum of positive pool-reported hashrates for a coin, or null when no row has a number. */
export function reportedHashrate(coin, pools) {
  const own = pools.filter((p) => p.coin_id === coin.id && p.hashrate !== null && p.hashrate !== undefined);
  if (!own.length) return null;
  return own.reduce((a, p) => a + (p.hashrate > 0 ? p.hashrate : 0), 0);
}
/** Ranking within each exact (n,k) group by pool-reported hashrate, zero/unavailable last, name as
 * tie-break. Hashrates are never compared across parameter sets. Mirrors src/data.rs rank_coins. */
export function rankCoins(coins, pools) {
  const sum = new Map(coins.map((c) => [c.id, reportedHashrate(c, pools)]));
  coins.sort((a, b) => {
    const ka = paramKey(a), kb = paramKey(b);
    for (let i = 0; i < 3; i++) if (ka[i] !== kb[i]) return ka[i] - kb[i];
    const sa = sum.get(a.id) > 0 ? sum.get(a.id) : 0, sb = sum.get(b.id) > 0 ? sum.get(b.id) : 0;
    if ((sa > 0) !== (sb > 0)) return sa > 0 ? -1 : 1;
    if (sa !== sb) return sb - sa;
    return a.name.localeCompare(b.name) || a.id.localeCompare(b.id);
  });
  return coins;
}

/** `--live-only`: re-read just the live sources into the existing pools.json / network.json. */
async function mainLiveOnly() {
  const previous = await readPrevious();
  const pools = structuredClone(previous.pools), coins = structuredClone(previous.coins);
  if (!pools.length || !coins.length) { console.error("No existing data to update; run a full refresh first."); process.exit(1); }
  await refreshLiveSources(coins, pools, previous);
  rankCoins(coins, pools);
  normaliseShares(coins, pools);
  pools.sort((a, b) => (b.hashrate ?? -1) - (a.hashrate ?? -1));
  snap.sanitizeUrls(pools, coins, errors);
  // generated_at stays: only the live figures moved, and they carry their own observed_at.
  // The run's warnings are not merged into meta.errors (that list describes the full refresh).
  await publishSnapshot({ ...previous.pf, pools }, { ...previous.nf, coins }, previous.meta, previous);
  console.log(`Live sources updated. ${errors.length} warnings.`);
  if (errors.length) console.log(errors.map((e) => "  - " + e).join("\n"));
}

async function main() {
  if (process.argv.includes("--list")) {
    const m = await snap.readManifest(DATA);
    for (const id of await snap.listSnapshots(DATA)) console.log(`${id}${m?.snapshot === id ? "  (current)" : ""}`);
    return;
  }
  if (process.argv.includes("--rollback")) {
    const i = process.argv.indexOf("--rollback");
    const to = process.argv[i + 1] && !process.argv[i + 1].startsWith("--") ? process.argv[i + 1] : null;
    return snap.withLock(DATA, async () => console.log(`current.json now points at snapshot ${await snap.rollback(DATA, to)}.`));
  }
  return snap.withLock(DATA, async () => {
    // The layout before snapshots: keep those files as the first snapshot (for rollback).
    const imported = await snap.importFlat(DATA);
    if (imported) console.log(`Imported the existing data files as snapshot ${imported}.`);
    return process.argv.includes("--live-only") ? mainLiveOnly() : mainFull();
  });
}

async function mainFull() {
  const started = nowIso();
  const previous = await readPrevious();
  console.log("Refreshing equihash.com data…");
  const { coins, pools } = await refreshMps();
  await refresh2Miners(coins);
  await refreshZpool(coins, pools);
  await refreshZergpool(coins, pools);
  await addCuratedCoins(coins);
  const zecwec = await refreshManual(coins, pools);
  await refreshWcash(coins, zecwec);
  await refreshLiveSources(coins, pools, previous);
  await applyCoinStatus(coins);

  for (const c of coins) c.pool_count = pools.filter((p) => p.coin_id === c.id).length;
  rankCoins(coins, pools);
  normaliseShares(coins, pools);
  pools.sort((a, b) => (b.hashrate ?? -1) - (a.hashrate ?? -1));
  // Upstream URLs become links: anything that isn't http(s) is dropped here (and again at load).
  snap.sanitizeUrls(pools, coins, errors);

  const finished = nowIso();
  if (coins.length === 0 || pools.length === 0) {
    console.error("Refusing to write empty data set. Errors:\n" + errors.join("\n"));
    process.exit(1);
  }
  const meta = {
    generated_at: finished,
    started_at: started,
    pool_count: pools.length,
    coin_count: coins.length,
    mps_pool_count: pools.filter((p) => p.from_miningpoolstats).length,
    non_mps_pool_count: pools.filter((p) => !p.from_miningpoolstats).length,
    sources: [...sources.values()],
    errors,
  };
  await updatePermalinks(pools);
  const id = await publishSnapshot({ generated_at: finished, pools }, { generated_at: finished, coins }, meta, previous);
  console.log(`${id ? "Done" : "Refresh finished but not published"}: ${pools.length} pools across ${coins.length} coins (${meta.non_mps_pool_count} not from MPS). ${errors.length} warnings.`);
  if (errors.length) console.log(errors.map((e) => "  - " + e).join("\n"));
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) main().catch((e) => {
  console.error(e);
  process.exit(1);
});
