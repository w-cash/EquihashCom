import test from "node:test";
import assert from "node:assert/strict";
import { resolveMpsFeed, mpsPriceUrl } from "./mps-source.mjs";
import { parseMpsPools } from "./refresh-data.mjs";

test("official index and coin preloads follow the migrated feed host and cache token", () => {
  const index = '<link rel="preload" href="https://server.miningpoolstats.stream/data/coins_data.js?t=1793015756" as="fetch" type=”application/json”>';
  const coin = '<script>var last_time = "1793015756";</script><LINK AS=\'fetch\' HREF=\'https://server.miningpoolstats.stream/data/zcash.js?t=1793015756&amp;v=1\' REL=\'preload\'>';
  assert.equal(resolveMpsFeed(index).url, "https://server.miningpoolstats.stream/data/coins_data.js?t=1793015756");
  const feed = resolveMpsFeed(coin, "zcash");
  assert.equal(feed.url, "https://server.miningpoolstats.stream/data/zcash.js?t=1793015756&v=1");
  assert.equal(mpsPriceUrl(feed.url, "zcash"), "https://server.miningpoolstats.stream/data/price/zcash.js?t=1793015756&v=1");
});

test("only an uncommented matching fetch preload selects the source", () => {
  const html = `
    <!-- <link rel="preload" as="fetch" href="https://data.miningpoolstats.stream/data/zcash.js?t=old"> -->
    <link rel="preload" as="fetch" href="https://server.miningpoolstats.stream/data/komodo.js?t=wrong">
    <link rel="preload" as="image" href="https://server.miningpoolstats.stream/data/zcash.js?t=image">
    <link rel="preload" as="fetch" href="https://data.miningpoolstats.stream/data/zcash.js?t=legacy">
  `;
  assert.equal(resolveMpsFeed(html, "zcash").url, "https://data.miningpoolstats.stream/data/zcash.js?t=legacy");
  assert.equal(resolveMpsFeed('var last_time = "123";', "zcash").url, "https://server.miningpoolstats.stream/data/zcash.js?t=123");
  assert.equal(resolveMpsFeed("", "zcash").url, "https://server.miningpoolstats.stream/data/zcash.js");
});

test("page content cannot authorize a new host or weaken the HTTPS source policy", () => {
  for (const url of [
    "https://evil.example/data/zcash.js",
    "http://server.miningpoolstats.stream/data/zcash.js",
    "https://user:secret@server.miningpoolstats.stream/data/zcash.js",
    "https://server.miningpoolstats.stream:8443/data/zcash.js",
    "https://127.0.0.1/data/zcash.js",
    "https://server.miningpoolstats.stream/data/zcash.js#fragment",
  ]) {
    assert.throws(() => resolveMpsFeed(`<link rel="preload" as="fetch" href="${url}">`, "zcash"), url);
  }
  assert.throws(() => resolveMpsFeed("", "../zcash"));
  assert.throws(() => mpsPriceUrl("https://evil.example/data/zcash.js", "zcash"));
  assert.throws(() => mpsPriceUrl("https://server.miningpoolstats.stream/data/komodo.js", "zcash"));
});

test("a fresh SoloMiners feed preserves its stable identity, positive rate and actual provenance", () => {
  const source = "https://server.miningpoolstats.stream/data/zcash.js?t=1793015756";
  const at = "2026-10-26T11:55:56Z";
  const [pool] = parseMpsPools("zcash", {
    symbol: "ZEC", unit: "Sol/s", hashrate: 30_000_000_000,
    data: [{ id: 143, pool_id: "solominers.space", url: "https://solominers.space/zec", hashrate: 3_690_980, miners: 1, feetype: "SOLO", country: "US|United States", minpay: -1 }],
  }, source, at);
  assert.equal(pool.id, "zcash:solominers.space:143");
  assert.equal(pool.hashrate, 3_690_980);
  assert.equal(pool.hashrate_unit, "Sol/s");
  assert.equal(pool.active, true);
  assert.equal(pool.miners, 1);
  assert.equal(pool.min_payout, null);
  assert.equal(pool.data_url, source);
  for (const field of ["hashrate", "miners", "fee", "blocks", "min_payout"]) {
    assert.equal(pool[`${field}_source`], source);
    assert.equal(pool[`${field}_observed_at`], at);
  }
});

test("MPS rates retain fractional native units and distinguish zero from hidden or missing", () => {
  const values = [1234.567, "87.38", 0, -1, -2, null];
  const pools = parseMpsPools("zcash", { symbol: "ZEC", unit: "Sol/s", data: values.map((hashrate, id) => ({ id, pool_id: `pool${id}.example`, url: `https://pool${id}.example/`, hashrate })) }, "https://server.miningpoolstats.stream/data/zcash.js", "2026-10-26T12:00:00Z");
  assert.deepEqual(pools.map((p) => p.hashrate), [1234.567, 87.38, 0, null, null, null]);
  assert.deepEqual(pools.map((p) => p.active), [true, true, false, false, false, false]);
  assert.deepEqual(pools.map((p) => p.basis), ["listed_pools", "listed_pools", "listed_pools", null, null, null]);
});
