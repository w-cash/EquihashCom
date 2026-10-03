// Tests for the pure parts of the refresh script. Run: node --test scripts/
import test from "node:test";
import assert from "node:assert/strict";
import { applyLiveFee, stamp, rankCoins, reportedHashrate, hashrateBasis, parseLiveReading, applyLiveReading, keepPreviousLive, normaliseShares, livePoolIds, applyLiveResult, wcashRewardNote } from "./refresh-data.mjs";

test("a live fee refresh never moves the hashrate timestamp", () => {
  const row = { hashrate: 440000, hashrate_is_reported: true, hashrate_observed_at: "2026-10-02T23:29:00.000Z", fetched_at: "2026-10-02T23:29:00.000Z", fee_pct: 0, schemes: [{ scheme: "PPLNS", fee_pct: 0 }] };
  applyLiveFee(row, 0.5, "https://pool.zecwec.com/api/v1/overview", "2026-10-03T10:00:00.000Z");
  assert.equal(row.fee_pct, 0.5);
  assert.equal(row.schemes[0].fee_pct, 0.5);
  assert.equal(row.fee_observed_at, "2026-10-03T10:00:00.000Z");
  assert.equal(row.hashrate_observed_at, "2026-10-02T23:29:00.000Z");
  assert.equal(row.fetched_at, "2026-10-02T23:29:00.000Z");
});

test("a missing live fee leaves the row untouched", () => {
  const row = { fee_pct: 0, fee_observed_at: "a" };
  applyLiveFee(row, null, "x", "b");
  assert.deepEqual(row, { fee_pct: 0, fee_observed_at: "a" });
});

test("stamp only touches the named field", () => {
  const n = { hashrate_observed_at: "old", height: 5 };
  stamp(n, "height", "src", "new");
  assert.equal(n.height_observed_at, "new");
  assert.equal(n.hashrate_observed_at, "old");
});

test("basis keeps n/a distinct from zero", () => {
  assert.equal(hashrateBasis({ hashrate: null }), null);
  assert.equal(hashrateBasis({ hashrate: 0 }), "listed_pools");
  assert.equal(hashrateBasis({ hashrate: 5, hashrate_is_reported: true }), "operator_reported");
  assert.equal(reportedHashrate({ id: "a" }, [{ coin_id: "a", hashrate: null }]), null);
  assert.equal(reportedHashrate({ id: "a" }, [{ coin_id: "a", hashrate: 0 }]), 0);
});

test("ranking stays inside each parameter set", () => {
  const coins = [
    { id: "big144", name: "Big", equihash: { n: 144, k: 5 } },
    { id: "zec", name: "Zcash", equihash: { n: 200, k: 9 } },
    { id: "zero", name: "Alpha", equihash: { n: 200, k: 9 } },
    { id: "kmd", name: "Komodo", equihash: { n: 200, k: 9 } },
    { id: "unk", name: "Unknown", equihash: null },
  ];
  const pools = [
    { coin_id: "big144", hashrate: 1e12 }, { coin_id: "zec", hashrate: 3e10 }, { coin_id: "kmd", hashrate: 1.5e5 },
    { coin_id: "zero", hashrate: 0 }, { coin_id: "unk", hashrate: 9e15 },
  ];
  assert.deepEqual(rankCoins(coins, pools).map((c) => c.id), ["zec", "kmd", "zero", "big144", "unk"]);
});

const poolSrc = { id: "zecwec-pool", target: "pool", pool_id: "wcash:zecwec.com", url: "https://pool.zecwec.com/api/v1/hashrate/pool" };
const netSrc = { id: "zecwec-wcash-network", target: "network", coin_id: "wcash", url: "https://pool.zecwec.com/api/v1/hashrate/network" };
const NOW = 1791025800 * 1000;

test("live readings are used only when available is true", () => {
  assert.equal(parseLiveReading(poolSrc, { available: false, hashrate_sol_s: 9, updated_at: 1791025763 }, NOW).status, "unavailable");
  assert.equal(parseLiveReading(poolSrc, { hashrate_sol_s: 9, updated_at: 1791025763 }, NOW).status, "unavailable");
  assert.equal(parseLiveReading(poolSrc, { available: true, hashrate_sol_s: -1, updated_at: 1791025763 }, NOW).status, "error");
  assert.equal(parseLiveReading(poolSrc, { available: true, hashrate_sol_s: 5, updated_at: 1891025763 }, NOW).status, "error");
  const ok = parseLiveReading(poolSrc, { available: true, hashrate_sol_s: 553587, window_seconds: 1200, updated_at: 1791025763 }, NOW);
  assert.equal(ok.status, "ok");
  assert.deepEqual(ok.reading, { hashrate: 553587, observed_at: "2026-10-03T11:09:23Z", window_seconds: 1200, sample_blocks: null, height: null });
});

test("live source field mappings accept nested dotted paths and preserve an explicit zero", () => {
  const src = {
    id: "zprominers-pool",
    fields: { available: "ok", hashrate: "all.solRate", updated_at: "t", window_seconds: "" },
  };
  const out = parseLiveReading(src, { ok: true, t: 1791025763, all: { solRate: 0 } }, NOW);
  assert.equal(out.status, "ok");
  assert.deepEqual(out.reading, { hashrate: 0, observed_at: "2026-10-03T11:09:23Z", window_seconds: null, sample_blocks: null, height: null });
});

test("a live reading sets basis, source and its own time; the previous one survives an outage", () => {
  const row = { id: "wcash:zecwec.com", hashrate: null, basis: null, fee_observed_at: "2026-10-03T10:00:00Z" };
  const r = parseLiveReading(poolSrc, { available: true, hashrate_sol_s: 553587, window_seconds: 1200, updated_at: 1791025763 }, NOW).reading;
  applyLiveReading(row, poolSrc, r);
  assert.equal(row.basis, "pool_api");
  assert.equal(row.hashrate_source, poolSrc.url);
  assert.equal(row.hashrate_observed_at, "2026-10-03T11:09:23Z");
  assert.equal(row.hashrate_window_s, 1200);
  assert.equal(row.fee_observed_at, "2026-10-03T10:00:00Z", "the fee time is untouched");
  const fresh = { id: "wcash:zecwec.com", hashrate: null };
  assert.equal(keepPreviousLive(fresh, row, poolSrc), true);
  assert.equal(fresh.hashrate, 553587);
  assert.equal(fresh.hashrate_observed_at, "2026-10-03T11:09:23Z", "kept value keeps its real age");
  const coin = { id: "wcash", network: { hashrate: null, height: 16000, height_observed_at: "2026-10-03T11:00:00Z" } };
  applyLiveReading(coin, netSrc, parseLiveReading(netSrc, { available: true, hashrate_sol_s: 507977, sample_blocks: 120, height: 16143, updated_at: 1791025763 }, NOW).reading);
  assert.equal(coin.network.basis, "network");
  assert.equal(coin.network.hashrate_sample_blocks, 120);
  assert.equal(coin.network.height, 16000, "the explorer's height keeps its own source");
  const bare = { id: "wcash", network: { hashrate: null, height: null } };
  applyLiveReading(bare, netSrc, parseLiveReading(netSrc, { available: true, hashrate_sol_s: 507977, sample_blocks: 120, height: 16143, updated_at: 1791025763 }, NOW).reading);
  assert.equal(bare.network.height, 16143, "fills a height nobody else supplied");
  assert.equal(bare.network.height_source, netSrc.url);
});

test("shares never exceed 100%", () => {
  const coins = [{ id: "wcash", network: { hashrate: 507977 } }];
  const pools = [{ coin_id: "wcash", hashrate: 553587 }];
  normaliseShares(coins, pools);
  assert.ok(pools[0].network_share_pct <= 100);
  const c2 = [{ id: "x", network: { hashrate: 100 } }];
  const p2 = [{ coin_id: "x", hashrate: 101.5 }];
  normaliseShares(c2, p2);
  assert.equal(p2[0].network_share_pct, 100);
});

test("one pool endpoint feeding the ZEC and WEC rows gives both the same value", () => {
  const src = { id: "zecwec-pool", target: "pool", pool_ids: ["zcash:zecwec.com", "wcash:zecwec.com"], url: "https://pool.zecwec.com/api/v1/hashrate/pool" };
  assert.deepEqual(livePoolIds(src), ["zcash:zecwec.com", "wcash:zecwec.com"]);
  assert.deepEqual(livePoolIds({ pool_id: "a" }), ["a"], "single pool_id still works");
  const pools = [{ id: "zcash:zecwec.com", hashrate: 440000, basis: "operator_reported", hashrate_is_reported: true }, { id: "wcash:zecwec.com", hashrate: null }];
  const res = parseLiveReading(src, { available: true, hashrate_sol_s: 561866, window_seconds: 1200, updated_at: 1791025763 }, NOW);
  const out = applyLiveResult(src, res, [], pools, { pools: [], coins: [] });
  assert.deepEqual(out.missing, []);
  const keys = ["hashrate", "basis", "hashrate_source", "hashrate_observed_at", "hashrate_window_s", "hashrate_is_reported", "live_source"];
  const pick = (p) => keys.map((k) => p[k]);
  assert.deepEqual(pick(pools[0]), pick(pools[1]));
  assert.equal(pools[0].hashrate, 561866);
  assert.equal(pools[0].basis, "pool_api");
  // Endpoint down: both rows keep the same previous reading.
  const prev = { pools: structuredClone(pools), coins: [] };
  const fresh = [{ id: "zcash:zecwec.com", hashrate: null }, { id: "wcash:zecwec.com", hashrate: null }];
  const down = applyLiveResult(src, { status: "unavailable" }, [], fresh, prev);
  assert.deepEqual(down.kept, ["zcash:zecwec.com", "wcash:zecwec.com"]);
  assert.deepEqual(pick(fresh[0]), pick(fresh[1]));
  assert.equal(fresh[0].hashrate, 561866);
});

test("the WEC reward note says the subsidy ramps until height 40,000, and never states a supply", () => {
  const n = wcashRewardNote(16186);
  assert.match(n, /^Coinbase reward of block 16186 /);
  assert.match(n, /ramps up every block until height 40,000/);
  assert.doesNotMatch(n, /supply|cap|21/i);
  assert.match(wcashRewardNote(50000), /changes every block after height 40,000/);
});
