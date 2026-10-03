// Tests for scripts/snapshot.mjs (atomic publish, validation, rollback) and the refresh's
// permalink assignment. Run with: node --test scripts/
import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import crypto from "node:crypto";
import * as snap from "./snapshot.mjs";
import { assignPermalinks, computedSlug, slugify } from "./refresh-data.mjs";

const tmp = async () => fs.mkdtemp(path.join(os.tmpdir(), "eq-snap-"));
const pool = (id, coin, h = 1000, extra = {}) => ({ id, name: id.split(":")[1] || id, coin_id: coin, coin: coin.toUpperCase(), url: `https://${id.split(":")[1] || "x.example"}/`, hashrate: h, ...extra });
function dataset(nPools = 20, extra = {}) {
  const coins = [{ id: "zcash", name: "Zcash", symbol: "ZEC", status: "active", network: { hashrate: 1e9 } }, { id: "komodo", name: "Komodo", symbol: "KMD", status: "active", network: { hashrate: 1e6 } }];
  const pools = Array.from({ length: nPools }, (_, i) => pool(`${i % 2 ? "komodo" : "zcash"}:pool${i}.example`, i % 2 ? "komodo" : "zcash", 1000 + i));
  const meta = { generated_at: "2026-10-03T14:00:00Z", pool_count: pools.length, coin_count: coins.length, errors: [], ...extra.meta };
  return { "pools.json": { generated_at: meta.generated_at, pools: extra.pools || pools }, "network.json": { generated_at: meta.generated_at, coins }, "meta.json": { ...meta, pool_count: (extra.pools || pools).length } };
}
const text = (d) => Object.fromEntries(Object.entries(d).map(([k, v]) => [k, JSON.stringify(v, null, 2)]));

test("publish writes a complete snapshot, then swaps current.json with sizes and SHA-256", async () => {
  const dir = await tmp();
  const r = await snap.publish(dir, text(dataset()), { id: "20261003T140000Z" });
  assert.deepEqual([r.published, r.id, r.problems], [true, "20261003T140000Z", []]);
  const m = JSON.parse(await fs.readFile(path.join(dir, "current.json"), "utf8"));
  assert.equal(m.snapshot, "20261003T140000Z");
  for (const f of snap.GENERATED) {
    const buf = await fs.readFile(path.join(dir, "snapshots", m.snapshot, f));
    assert.deepEqual(m.files[f], { bytes: buf.length, sha256: crypto.createHash("sha256").update(buf).digest("hex") }, f);
  }
  const left = await fs.readdir(path.join(dir, "snapshots"));
  assert.deepEqual(left, ["20261003T140000Z"], "no staging directory left behind");
  assert.ok(!(await fs.readdir(dir)).some((n) => n.includes(".tmp-")), "no temp manifest left behind");
});

test("a sharp drop or a critical source failure is refused and the previous snapshot stays live", async () => {
  const dir = await tmp();
  await snap.publish(dir, text(dataset(20)), { id: "20261003T130000Z" });
  const before = await fs.readFile(path.join(dir, "current.json"), "utf8");
  const previous = await snap.readCurrent(dir);
  const small = dataset(10);
  let r = await snap.publish(dir, text(small), { id: "20261003T140000Z", previous });
  assert.equal(r.published, false);
  assert.ok(r.problems.some((p) => /pool rows dropped from 20 to 10/.test(p)), r.problems.join("; "));
  assert.equal(await fs.readFile(path.join(dir, "current.json"), "utf8"), before, "manifest untouched");
  assert.deepEqual(await snap.listSnapshots(dir), ["20261003T130000Z"], "rejected snapshot not kept");
  r = await snap.publish(dir, text(dataset(20, { meta: { errors: ["mps zcash: HTTP 503", "price bitmark-equihash: HTTP 404"] } })), { previous });
  assert.equal(r.published, false);
  assert.deepEqual(r.problems, ["critical source failed: mps zcash: HTTP 503"], "a price 404 is not critical");
  // --force publishes anyway, and records why.
  r = await snap.publish(dir, text(small), { id: "20261003T150000Z", previous, force: true });
  assert.equal(r.published, true);
  assert.ok(JSON.parse(await fs.readFile(path.join(dir, "current.json"), "utf8")).forced.length > 0);
});

test("schema checks: unknown coin, duplicate ids, non-numbers and unsafe URLs are refused", () => {
  const d = dataset(4);
  d["pools.json"].pools.push(pool("zcash:pool0.example", "zcash"));
  d["pools.json"].pools.push(pool("dogecoin:x.example", "dogecoin"));
  d["pools.json"].pools.push(pool("zcash:evil.example", "zcash", "12", { url: "javascript:alert(1)" }));
  d["meta.json"].pool_count = d["pools.json"].pools.length;
  const p = snap.validate(d);
  for (const re of [/duplicate pool id zcash:pool0/, /unknown coin "dogecoin"/, /evil.example hashrate is not a number/, /evil.example url is not an http\(s\) URL/]) assert.ok(p.some((x) => re.test(x)), `${re} in ${p.join("; ")}`);
  assert.deepEqual(snap.validate(dataset(4)), []);
});

test("sanitizeUrls drops javascript:, data: and friends but keeps http(s) and plain words", () => {
  const pools = [{ id: "a", url: "JavaScript:alert(1)", source_url: "https://ok.example/", data_url: "data:text/html,<script>", hashrate_source: " http://fine.example/x " }];
  const coins = [{ id: "c", source_url: "vbscript:x", network: { hashrate_upstream: "explorer", height_source: "//evil.example" }, status_sources: [{ label: "x", url: "javascript:1" }, { label: "y", url: "https://y.example" }] }];
  const warn = snap.sanitizeUrls(pools, coins, []);
  assert.deepEqual([pools[0].url, pools[0].source_url, pools[0].data_url, pools[0].hashrate_source], [null, "https://ok.example/", null, " http://fine.example/x "]);
  assert.deepEqual([coins[0].source_url, coins[0].network.hashrate_upstream, coins[0].network.height_source], [null, "explorer", null]);
  assert.deepEqual(coins[0].status_sources.map((l) => l.url), ["https://y.example"]);
  assert.equal(warn.length, 5);
  for (const bad of ["javascript:alert(1)", "data:x", "https://", "https://a b", "https://x/<", "ftp://x", "/relative"]) assert.equal(snap.safeUrl(bad), false, bad);
});

test("rollback points current.json at the previous snapshot; prune keeps current and previous", async () => {
  const dir = await tmp();
  for (const h of ["10", "11", "12"]) await snap.publish(dir, text(dataset()), { id: `20261003T${h}0000Z` });
  assert.equal(await snap.rollback(dir), "20261003T110000Z");
  let m = JSON.parse(await fs.readFile(path.join(dir, "current.json"), "utf8"));
  assert.deepEqual([m.snapshot, m.rolled_back_from], ["20261003T110000Z", "20261003T120000Z"]);
  assert.equal(await snap.rollback(dir, "20261003T100000Z"), "20261003T100000Z");
  await assert.rejects(snap.rollback(dir, "nope"));
  // Prune to one: the newest (12:00) plus the current (10:00) and the previous (11:00) survive.
  await snap.prune(dir, 1);
  assert.deepEqual(await snap.listSnapshots(dir), ["20261003T100000Z", "20261003T110000Z", "20261003T120000Z"]);
  // Publishing again: 12:00 is now neither newest, current nor previous, so it goes.
  await snap.publish(dir, text(dataset()), { id: "20261003T130000Z", keep: 1 });
  assert.deepEqual(await snap.listSnapshots(dir), ["20261003T100000Z", "20261003T130000Z"]);
  m = JSON.parse(await fs.readFile(path.join(dir, "current.json"), "utf8"));
  assert.deepEqual([m.snapshot, m.previous], ["20261003T130000Z", "20261003T100000Z"]);
});

test("the flat layout is imported as the first snapshot and the flat files removed", async () => {
  const dir = await tmp();
  for (const [f, v] of Object.entries(text(dataset()))) await fs.writeFile(path.join(dir, f), v);
  const id = await snap.importFlat(dir);
  assert.equal(id, "20261003T140000Z-imported");
  assert.equal(JSON.parse(await fs.readFile(path.join(dir, "current.json"), "utf8")).snapshot, id);
  for (const f of snap.GENERATED) await assert.rejects(fs.stat(path.join(dir, f)), f);
  assert.equal(await snap.importFlat(dir), null, "only once");
  const cur = await snap.readCurrent(dir);
  assert.equal(cur["pools.json"].pools.length, 20);
});

test("one refresh at a time", async () => {
  const dir = await tmp();
  let inner;
  await snap.withLock(dir, async () => { inner = await snap.withLock(dir, async () => "nested").catch((e) => e.message); });
  assert.match(inner, /another refresh is running/);
  assert.equal(await snap.withLock(dir, async () => "free again"), "free again");
});

test("permalinks: assigned once, never changed by a rename or new domain, unique, reserved after removal", () => {
  const file = { pools: {}, aliases: {} };
  const pools = [
    { id: "zcash:f2pool.com:1", coin_id: "zcash", name: "F2Pool", url: "https://www.f2pool.com/" },
    { id: "zcash:f2pool.com:2", coin_id: "zcash", name: "F2Pool", url: "https://www.f2pool.com/" },
  ];
  assert.deepEqual(assignPermalinks(pools, file), ["zcash:f2pool.com:1", "zcash:f2pool.com:2"]);
  assert.deepEqual(file.pools, { "zcash:f2pool.com:1": "zcash-f2pool-f2pool-com", "zcash:f2pool.com:2": "zcash-f2pool-f2pool-com-2" });
  // Renamed and moved: same id, same permalink.
  pools[0].name = "F2Pool Global"; pools[0].url = "https://f2pool.io/";
  assert.deepEqual(assignPermalinks(pools, file), []);
  assert.equal(file.pools["zcash:f2pool.com:1"], "zcash-f2pool-f2pool-com");
  // A pool that left keeps its slug reserved; a newcomer with the same computed slug gets another.
  const later = [{ id: "zcash:f2pool.com:3", coin_id: "zcash", name: "F2Pool", url: "https://www.f2pool.com/" }];
  assignPermalinks(later, file);
  assert.equal(file.pools["zcash:f2pool.com:3"], "zcash-f2pool-f2pool-com-3");
  assert.equal(slugify("Hello,  World!--"), "hello-world");
  assert.equal(computedSlug({ coin_id: "wcash", name: "ZecWec", url: "https://pool.zecwec.com" }), "wcash-zecwec-pool-zecwec-com");
});
