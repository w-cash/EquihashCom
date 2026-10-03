/**
 * Published data snapshots (used by refresh-data.mjs; mirrors src/data.rs Snapshot).
 *
 *   data/snapshots/<id>/pools.json, network.json, meta.json   one complete refresh, never edited
 *   data/current.json                                          manifest: which snapshot is live,
 *                                                              with each file's size and SHA-256
 *
 * Publishing: write the files into data/snapshots/.staging-<id>/, fsync them, validate what is on
 * disk, rename the directory to data/snapshots/<id>/, then write data/.current.json.tmp-<pid>,
 * fsync and rename it over data/current.json. A reader (the server) sees either the old manifest
 * or the new one, and every file a manifest lists is complete. Old snapshots are kept for
 * rollback (`KEEP`); `rollback()` points the manifest back at an earlier one.
 */
import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";

export const GENERATED = ["pools.json", "network.json", "meta.json"];
export const MANIFEST = "current.json";
/** Snapshots kept on disk (the current one and the one before it are never pruned). */
export const KEEP = 10;
/** Refuse to publish when a count falls below this share of the previous snapshot's. */
export const MIN_KEEP_RATIO = { pools: 0.8, coins: 0.9, positive: 0.6 };

const sha256 = (buf) => crypto.createHash("sha256").update(buf).digest("hex");
export const validId = (id) => typeof id === "string" && /^[A-Za-z0-9_-][A-Za-z0-9._-]{0,99}$/.test(id);

/** 2026-10-03T14:11:50.123Z -> 20261003T141150Z */
export function snapshotId(date = new Date()) {
  return date.toISOString().replace(/[-:]/g, "").replace(/\.\d+Z$/, "Z");
}

async function writeDurable(file, data) {
  const fh = await fs.open(file, "w");
  try { await fh.writeFile(data); await fh.sync(); } finally { await fh.close(); }
}
async function syncDir(dir) {
  let fh;
  try { fh = await fs.open(dir, "r"); await fh.sync(); } catch { /* not supported everywhere */ } finally { await fh?.close(); }
}
const exists = (p) => fs.stat(p).then(() => true, () => false);

/** The current manifest, or null (flat layout: the files sit in data/ itself). */
export async function readManifest(dataDir) {
  try { return JSON.parse(await fs.readFile(path.join(dataDir, MANIFEST), "utf8")); }
  catch (e) { if (e.code === "ENOENT") return null; throw new Error(`${MANIFEST}: ${e.message}`); }
}

/** Directory holding the current generated files. */
export async function currentDir(dataDir) {
  const m = await readManifest(dataDir);
  if (!m) return dataDir;
  if (!validId(m.snapshot)) throw new Error(`${MANIFEST}: bad snapshot id ${JSON.stringify(m.snapshot)}`);
  return path.join(dataDir, "snapshots", m.snapshot);
}

/** The current generated files, parsed ({} for a file that is missing). */
export async function readCurrent(dataDir) {
  const dir = await currentDir(dataDir).catch(() => dataDir);
  const out = {};
  for (const f of GENERATED) {
    try { out[f] = JSON.parse(await fs.readFile(path.join(dir, f), "utf8")); } catch { out[f] = null; }
  }
  return out;
}

// ---------- validation ----------

/** Same rule as data::safe_url: http(s), a host, no whitespace/control chars, <, >, \ or `. */
export function safeUrl(u) {
  if (typeof u !== "string") return false;
  const t = u.trim();
  if (!t || t.length > 2048 || /[\s\u0000-\u001f\u007f<>\\`]/.test(t)) return false;
  const m = t.match(/^https?:\/\/([^/?#]*)/i);
  if (!m) return false;
  const host = m[1].split("@").pop();
  return host !== "" && !host.startsWith(".") && !host.startsWith(":");
}
const POOL_URLS = ["url", "source_url", "data_url", "hashrate_source", "fee_source", "miners_source", "blocks_source", "min_payout_source"];
const COIN_URLS = ["source_url", "data_url", "price_source"];
const NET_URLS = ["hashrate_source", "difficulty_source", "height_source", "block_time_source"];
const POOL_NUMS = ["hashrate", "network_share_pct", "miners", "workers", "fee_pct", "min_payout", "blocks_last_1000", "last_block_height"];

/** Null every upstream URL that isn't http(s) (javascript:, data:, ...), noting each in `warn`.
 *  Runs on the refreshed rows before they are published; the server applies the same rule. */
export function sanitizeUrls(pools, coins, warn = []) {
  const fix = (obj, k, what) => {
    if (obj && obj[k] != null && !safeUrl(obj[k])) { warn.push(`unsafe URL dropped: ${what} ${k} ${JSON.stringify(String(obj[k]).slice(0, 80))}`); obj[k] = null; }
  };
  for (const p of pools) for (const k of POOL_URLS) fix(p, k, `pool ${p.id}`);
  for (const c of coins) {
    for (const k of COIN_URLS) fix(c, k, `coin ${c.id}`);
    for (const k of NET_URLS) fix(c.network, k, `coin ${c.id} network`);
    if (c.network && typeof c.network.hashrate_upstream === "string" && /^[a-z][a-z0-9+.-]*:|\/\//i.test(c.network.hashrate_upstream.trim())) fix(c.network, "hashrate_upstream", `coin ${c.id} network`);
    if (c.block_reward_miner) fix(c.block_reward_miner, "source_url", `coin ${c.id} block_reward_miner`);
    for (const key of ["status_sources", "cross_checks"]) {
      if (!Array.isArray(c[key])) continue;
      c[key] = c[key].filter((l) => { const ok = safeUrl(l?.url); if (!ok) warn.push(`unsafe URL dropped: coin ${c.id} ${key} ${JSON.stringify(String(l?.url).slice(0, 80))}`); return ok; });
    }
  }
  return warn;
}

/** Errors from sources the snapshot can't do without: a miningpoolstats coin page (its pool rows
 *  would silently disappear). Prices, 2Miners cross-checks, zergpool and live sources are not. */
export const isCritical = (e) => /^mps [\w.-]+: /.test(e);

/** Schema and sanity checks. Returns a list of problems; empty means it may be published. */
export function validate(files, previous = null) {
  const problems = [];
  const pf = files["pools.json"], nf = files["network.json"], meta = files["meta.json"];
  if (!pf || !Array.isArray(pf.pools)) return ["pools.json: no pools array"];
  if (!nf || !Array.isArray(nf.coins)) return ["network.json: no coins array"];
  if (!meta || typeof meta !== "object") return ["meta.json: missing"];
  const pools = pf.pools, coins = nf.coins;
  if (!pools.length) problems.push("pools.json: no pool rows");
  if (!coins.length) problems.push("network.json: no coins");
  const coinIds = new Set();
  for (const c of coins) {
    if (!c || typeof c.id !== "string" || !c.id) { problems.push("network.json: coin without an id"); continue; }
    if (coinIds.has(c.id)) problems.push(`network.json: duplicate coin id ${c.id}`);
    coinIds.add(c.id);
    if (typeof c.name !== "string" || !c.name) problems.push(`network.json: coin ${c.id} has no name`);
    if (typeof c.symbol !== "string" || !c.symbol) problems.push(`network.json: coin ${c.id} has no symbol`);
    if (c.network != null && typeof c.network !== "object") problems.push(`network.json: coin ${c.id} network is not an object`);
    const h = c.network?.hashrate;
    if (h != null && !(typeof h === "number" && Number.isFinite(h) && h >= 0)) problems.push(`network.json: coin ${c.id} network.hashrate ${h}`);
    for (const k of COIN_URLS) if (c[k] != null && !safeUrl(c[k])) problems.push(`network.json: coin ${c.id} ${k} is not an http(s) URL`);
    for (const k of NET_URLS) if (c.network?.[k] != null && !safeUrl(c.network[k])) problems.push(`network.json: coin ${c.id} network.${k} is not an http(s) URL`);
  }
  const poolIds = new Set();
  for (const p of pools) {
    if (!p || typeof p.id !== "string" || !p.id) { problems.push("pools.json: row without an id"); continue; }
    if (poolIds.has(p.id)) problems.push(`pools.json: duplicate pool id ${p.id}`);
    poolIds.add(p.id);
    if (typeof p.name !== "string" || !p.name) problems.push(`pools.json: ${p.id} has no name`);
    if (!coinIds.has(p.coin_id)) problems.push(`pools.json: ${p.id} names unknown coin ${JSON.stringify(p.coin_id)}`);
    for (const k of POOL_NUMS) if (p[k] != null && !(typeof p[k] === "number" && Number.isFinite(p[k]))) problems.push(`pools.json: ${p.id} ${k} is not a number`);
    if (typeof p.hashrate === "number" && p.hashrate < 0) problems.push(`pools.json: ${p.id} negative hashrate`);
    for (const k of POOL_URLS) if (p[k] != null && !safeUrl(p[k])) problems.push(`pools.json: ${p.id} ${k} is not an http(s) URL`);
  }
  if (typeof meta.generated_at !== "string" || isNaN(Date.parse(meta.generated_at))) problems.push("meta.json: generated_at missing or not a date");
  if (meta.pool_count !== pools.length) problems.push(`meta.json: pool_count ${meta.pool_count} but ${pools.length} rows`);
  if (meta.coin_count !== coins.length) problems.push(`meta.json: coin_count ${meta.coin_count} but ${coins.length} coins`);
  for (const e of (meta.errors || []).filter(isCritical)) problems.push(`critical source failed: ${e}`);
  // Sharp drops against the snapshot being replaced.
  const prevPools = previous?.["pools.json"]?.pools, prevCoins = previous?.["network.json"]?.coins;
  if (Array.isArray(prevPools) && prevPools.length) {
    const pos = (list) => list.filter((p) => typeof p.hashrate === "number" && p.hashrate > 0).length;
    if (pools.length < prevPools.length * MIN_KEEP_RATIO.pools) problems.push(`pool rows dropped from ${prevPools.length} to ${pools.length}`);
    if (pos(pools) < pos(prevPools) * MIN_KEEP_RATIO.positive) problems.push(`rows with positive hashrate dropped from ${pos(prevPools)} to ${pos(pools)}`);
    const had = new Map();
    for (const p of prevPools) had.set(p.coin_id, (had.get(p.coin_id) || 0) + 1);
    for (const [cid, n] of had) {
      const coin = coins.find((c) => c.id === cid);
      if (n >= 3 && coin && coin.status === "active" && !pools.some((p) => p.coin_id === cid)) problems.push(`coin ${cid} lost all ${n} of its pool rows`);
    }
  }
  if (Array.isArray(prevCoins) && prevCoins.length && coins.length < prevCoins.length * MIN_KEEP_RATIO.coins) problems.push(`coins dropped from ${prevCoins.length} to ${coins.length}`);
  return problems;
}

// ---------- publish / rollback ----------

async function swapManifest(dataDir, manifest) {
  const tmp = path.join(dataDir, `.${MANIFEST}.tmp-${process.pid}`);
  await writeDurable(tmp, JSON.stringify(manifest, null, 2) + "\n");
  await fs.rename(tmp, path.join(dataDir, MANIFEST));
  await syncDir(dataDir);
}

async function entries(dir) {
  const out = {};
  for (const f of GENERATED) {
    const buf = await fs.readFile(path.join(dir, f));
    out[f] = { bytes: buf.length, sha256: sha256(buf) };
  }
  return out;
}

/** Snapshot ids on disk, oldest first. */
export async function listSnapshots(dataDir) {
  const dir = path.join(dataDir, "snapshots");
  const names = await fs.readdir(dir).catch(() => []);
  return names.filter((n) => validId(n) && !n.startsWith(".")).sort();
}

/** Keep the newest `keep` snapshots, plus the current one and the one before it. */
export async function prune(dataDir, keep = KEEP) {
  const m = await readManifest(dataDir);
  const protect = new Set([m?.snapshot, m?.previous].filter(Boolean));
  const ids = await listSnapshots(dataDir);
  const removed = [];
  for (const id of ids.slice(0, Math.max(0, ids.length - keep))) {
    if (protect.has(id)) continue;
    await fs.rm(path.join(dataDir, "snapshots", id), { recursive: true, force: true });
    removed.push(id);
  }
  // Staging directories left by an interrupted run.
  for (const n of await fs.readdir(path.join(dataDir, "snapshots")).catch(() => [])) {
    if (!n.startsWith(".staging-")) continue;
    const st = await fs.stat(path.join(dataDir, "snapshots", n)).catch(() => null);
    if (st && Date.now() - st.mtimeMs > 3600e3) await fs.rm(path.join(dataDir, "snapshots", n), { recursive: true, force: true });
  }
  return removed;
}

/** Turn the flat layout (data/pools.json, …) into the first snapshot, so it can be rolled back to. */
export async function importFlat(dataDir) {
  if (await readManifest(dataDir)) return null;
  const have = await Promise.all(GENERATED.map((f) => exists(path.join(dataDir, f))));
  if (!have.every(Boolean)) return null;
  const meta = JSON.parse(await fs.readFile(path.join(dataDir, "meta.json"), "utf8"));
  const id = `${snapshotId(new Date(Date.parse(meta.generated_at) || Date.now()))}-imported`;
  const files = {};
  for (const f of GENERATED) files[f] = await fs.readFile(path.join(dataDir, f), "utf8");
  await publish(dataDir, files, { id, force: true });
  for (const f of GENERATED) await fs.rm(path.join(dataDir, f), { force: true });
  await syncDir(dataDir);
  return id;
}

/**
 * Validate and publish one snapshot. `files` maps each GENERATED name to its JSON text.
 * Returns { published, id, problems }. With problems and no `force`, nothing changes on disk
 * apart from a removed staging directory: the previous snapshot stays live.
 */
export async function publish(dataDir, files, { id = snapshotId(), previous = null, force = false, keep = KEEP, now = new Date() } = {}) {
  if (!validId(id)) throw new Error(`bad snapshot id ${id}`);
  const root = path.join(dataDir, "snapshots");
  await fs.mkdir(root, { recursive: true });
  let final = path.join(root, id);
  for (let i = 2; await exists(final); i++) { id = `${id.replace(/-\d+$/, "")}-${i}`; final = path.join(root, id); }
  const stage = path.join(root, `.staging-${id}-${process.pid}`);
  await fs.rm(stage, { recursive: true, force: true });
  await fs.mkdir(stage);
  for (const f of GENERATED) {
    if (typeof files[f] !== "string") throw new Error(`publish: ${f} missing`);
    await writeDurable(path.join(stage, f), files[f]);
  }
  await syncDir(stage);
  // Validate what is actually on disk.
  const parsed = {};
  const problems = [];
  for (const f of GENERATED) {
    try { parsed[f] = JSON.parse(await fs.readFile(path.join(stage, f), "utf8")); } catch (e) { problems.push(`${f}: ${e.message}`); }
  }
  if (!problems.length) problems.push(...validate(parsed, previous));
  if (problems.length && !force) {
    await fs.rm(stage, { recursive: true, force: true });
    return { published: false, id: null, problems };
  }
  await fs.rename(stage, final);
  await syncDir(root);
  const prev = await readManifest(dataDir).catch(() => null);
  await swapManifest(dataDir, {
    snapshot: id,
    published_at: now.toISOString().replace(/\.\d+Z$/, "Z"),
    generated_at: parsed["meta.json"]?.generated_at ?? null,
    previous: prev?.snapshot ?? null,
    forced: problems.length ? problems : undefined,
    files: await entries(final),
  });
  await prune(dataDir, keep);
  return { published: true, id, problems };
}

/** Point data/current.json at an earlier snapshot (`toId`, or the newest one before the current). */
export async function rollback(dataDir, toId = null, now = new Date()) {
  const m = await readManifest(dataDir);
  const ids = await listSnapshots(dataDir);
  const target = toId ?? [...ids].reverse().find((id) => m && id < m.snapshot);
  if (!target || !ids.includes(target)) throw new Error(`no snapshot ${target ?? "before " + (m?.snapshot ?? "(none)")} to roll back to; have: ${ids.join(", ")}`);
  const dir = path.join(dataDir, "snapshots", target);
  const meta = JSON.parse(await fs.readFile(path.join(dir, "meta.json"), "utf8"));
  await swapManifest(dataDir, { snapshot: target, published_at: now.toISOString().replace(/\.\d+Z$/, "Z"), generated_at: meta.generated_at ?? null, previous: m?.snapshot ?? null, rolled_back_from: m?.snapshot ?? null, files: await entries(dir) });
  return target;
}

/** One refresh at a time (data/.refresh.lock; a lock older than two hours is taken over). */
export async function withLock(dataDir, fn) {
  const lock = path.join(dataDir, ".refresh.lock");
  try { await fs.writeFile(lock, String(process.pid), { flag: "wx" }); }
  catch (e) {
    const st = await fs.stat(lock).catch(() => null);
    if (e.code !== "EEXIST" || (st && Date.now() - st.mtimeMs < 2 * 3600e3)) throw new Error(`another refresh is running (${lock})`);
    await fs.writeFile(lock, String(process.pid));
  }
  try { return await fn(); } finally { await fs.rm(lock, { force: true }); }
}
