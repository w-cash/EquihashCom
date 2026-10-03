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
 * Rules: never invent numbers. Unknown => null (rendered as "n/a").
 * Usage: npm run refresh        (node scripts/refresh-data.mjs)
 */
import { pathToFileURL } from "node:url";
import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DATA = path.join(ROOT, "data");
const UA =
  "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36";
const MPS = "https://miningpoolstats.stream";
const MPS_DATA = "https://data.miningpoolstats.stream/data";
// Pages that are no longer in the MPS coin index but still have a page (PoW ended).
const EXTRA_MPS_PAGES = ["horizen", "flux"];

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
        name: idx?.name ?? d.symbol,
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
          hashrate_source: d.hashrate_src ?? null,
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
      }
      console.log(`  mps ${page}: ${d.data?.length ?? 0} pools`);
    } catch (e) {
      errors.push(`mps ${page}: ${e.message}`);
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
    delete row.verified_at; delete row.live;
    row.coin_id = coin?.id ?? m.coin_id ?? m.coin.toLowerCase();
    row.fetched_at = m.verified_at;
    if (m.live === "zecwec" && zecwec) {
      const fee = m.coin === "WEC" ? num(zecwec.body.wec_fee_bps) : num(zecwec.body.zec_fee_bps);
      if (fee !== null) {
        row.fee_pct = fee / 100;
        row.schemes = (row.schemes || []).map((s) => ({ ...s, fee_pct: fee / 100 }));
      }
      row.fetched_at = zecwec.fetched_at;
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
    network: { hashrate: null, unit: "Sol/s", hashrate_source: null, difficulty: null, height: null, block_time_target_s: 75, block_time_avg_s: null, pools_hashrate: null },
    price_usd: null,
    price_source: null,
    block_reward_miner: null,
    mps_pool_count: 0,
    source_url: "https://wcashexplorer.com/",
    data_url: `${base}/status`,
    fetched_at: null,
    status: "active",
    status_note: "Not listed on miningpoolstats. Network hashrate is not published by the explorer; the only pool (ZecWec) is reported at ~0.44 MSol/s.",
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
    const { body: bl, fetched_at: bAt } = await get(`${base}/blocks?limit=1`);
    const b = bl.data?.[0];
    if (b?.reward?.decimal) {
      coin.block_reward_miner = {
        value: Number(b.reward.decimal),
        unit: "WEC",
        note: `Coinbase reward of block ${b.height} as shown by the explorer (subsidy changes every block; see the spec's emission section).`,
        source_url: `${base}/blocks?limit=1`,
        fetched_at: bAt,
      };
    }
    addSource("wcashexplorer", "Wcash explorer API (height, difficulty, block spacing)", `${base}/status`);
  } catch (e) {
    errors.push(`wcash explorer: ${e.message}`);
  }
  if (zecwec?.body?.wcash_height && !coin.network.height) coin.network.height = num(zecwec.body.wcash_height);
  coins.push(coin);
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
    for (const p of own) p.network_share_pct = p.hashrate !== null && p.hashrate !== undefined && denom ? (p.hashrate / denom) * 100 : null;
  }
}

async function main() {
  const started = nowIso();
  console.log("Refreshing equihash.com data…");
  const { coins, pools } = await refreshMps();
  await refresh2Miners(coins);
  await refreshZpool(coins, pools);
  await refreshZergpool(coins, pools);
  await addCuratedCoins(coins);
  const zecwec = await refreshManual(coins, pools);
  await refreshWcash(coins, zecwec);
  await applyCoinStatus(coins);

  // Coin order: by network hashrate share of attention (ZEC first), then name.
  const order = ["ZEC", "WEC", "KMD", "ARRR", "BTG"];
  coins.sort((a, b) => {
    const ia = order.indexOf(a.symbol), ib = order.indexOf(b.symbol);
    if (ia !== -1 || ib !== -1) return (ia === -1 ? 99 : ia) - (ib === -1 ? 99 : ib);
    if (a.status !== b.status) return a.status === "active" ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  for (const c of coins) c.pool_count = pools.filter((p) => p.coin_id === c.id).length;
  normaliseShares(coins, pools);
  pools.sort((a, b) => (b.hashrate ?? -1) - (a.hashrate ?? -1));

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
  await fs.writeFile(path.join(DATA, "pools.json"), JSON.stringify({ generated_at: finished, pools }, null, 2));
  await fs.writeFile(path.join(DATA, "network.json"), JSON.stringify({ generated_at: finished, coins }, null, 2));
  await fs.writeFile(path.join(DATA, "meta.json"), JSON.stringify(meta, null, 2));
  console.log(`Done: ${pools.length} pools across ${coins.length} coins (${meta.non_mps_pool_count} not from MPS). ${errors.length} warnings.`);
  if (errors.length) console.log(errors.map((e) => "  - " + e).join("\n"));
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) main().catch((e) => {
  console.error(e);
  process.exit(1);
});
