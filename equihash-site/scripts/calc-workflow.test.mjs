import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const C = require("../static/calc-workflow.js");
const S = require("../static/shared.js");

test("calculator preserves zero separately from missing inputs", () => {
  assert.equal(C.numberOrNull(""), null);
  assert.equal(C.numberOrNull(null), null);
  assert.equal(C.numberOrNull("0"), 0);
  const unknown = C.estimateScenario({ hr: 840, watts: 2780, powerRate: null, fee: 0, price: 100, reward: 1.25, net: 30e9, bt: 75 }, S);
  assert.equal(unknown.powerDay, null);
  assert.equal(unknown.marginDay, null);
  assert.ok(unknown.coinsDay > 0, "known zero fee does not suppress expected coins");
  const freePower = C.estimateScenario({ hr: 840, watts: 2780, powerRate: 0, fee: 0, price: 100, reward: 1.25, net: 30e9, bt: 75 }, S);
  assert.equal(freePower.powerDay, 0);
  assert.equal(freePower.marginDay, freePower.revenueDay);
});

test("source ages gate automatic economics and reject future timestamps", () => {
  const now = Date.parse("2026-10-08T12:00:00Z");
  assert.equal(C.sourceState("2026-10-08T11:45:00Z", false, now, C.PRICE_MAX_AGE_SECONDS), "fresh");
  assert.equal(C.sourceState("2026-10-08T11:29:59Z", false, now, C.PRICE_MAX_AGE_SECONDS), "stale");
  assert.equal(C.sourceState(null, false, now, C.NETWORK_MAX_AGE_SECONDS), "missing-time");
  assert.equal(C.sourceState("2026-10-08T12:01:00Z", false, now, C.NETWORK_MAX_AGE_SECONDS), "future-time");
  assert.equal(C.sourceState("bad", true, now, C.NETWORK_MAX_AGE_SECONDS), "user-entered");
  assert.equal(C.sourceUsable("stale"), false);
  assert.equal(C.sourceUsable("user-entered"), true);
});

test("local setup storage is allow-listed and cannot retain credentials", () => {
  const setup = C.parseStoredSetup(JSON.stringify({
    coin: "zcash", hashrate: 840, power: 0, poolId: "pool-1", productId: "PPLNS",
    credited: 1.2, wallet: "t1-private", password: "secret", apiKey: "secret",
  }));
  assert.deepEqual(setup, {
    version: 1, coin: "zcash", hashrate: "840", power: "0", poolId: "pool-1",
    productId: "PPLNS", credited: "1.2",
  });
  assert.equal(C.parseStoredSetup("not json").version, 1);
});

test("expected, credited, matured and withdrawable remain distinct without causal claims", () => {
  const x = C.comparison(0.25, 7, "1.5", "1.25", "0", "ZEC");
  assert.equal(x.expected, "1.7500 ZEC");
  assert.equal(x.credited, "1.5000 ZEC");
  assert.equal(x.matured, "1.2500 ZEC");
  assert.equal(x.withdrawable, "0.000000 ZEC");
  assert.equal(x.attributionSupported, false);
  assert.match(x.note, /does not establish underpayment/);
});

test("units use kSol/s against Sol/s exactly once", () => {
  const e = C.estimateScenario({ hr: 840, watts: 2780, powerRate: 0.08, fee: 1, price: null, reward: 1.25, net: 30e9, bt: 75 }, S);
  const expected = (840 * 1000 / 30e9) * (86400 / 75) * 1.25 * 0.99;
  assert.ok(Math.abs(e.coinsDay - expected) < 1e-12);
  assert.ok(Math.abs(e.powerDay - 2.78 * 24 * 0.08) < 1e-12);
  assert.equal(e.revenueDay, null, "absent price does not become zero revenue");
  assert.equal(e.marginDay, null, "absent price does not imply a loss equal to power cost");
});
