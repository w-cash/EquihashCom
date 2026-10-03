// Tests for static/shared.js, the pure helpers app.js uses for the live re-ranking and the
// calculator. Run with: node --test scripts/
import { test } from "node:test";
import assert from "node:assert/strict";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const S = require("../static/shared.js");

// Same literal as calc.rs (GUARD_840), so the server and the browser say the same thing.
const GUARD_840 = "Your hashrate is 1.61× the network estimate. At 10% or more of the network a share-based estimate isn't meaningful: adding it would itself move the network hashrate and difficulty. Coins per day, revenue and profit are shown as n/a.";

test("rankOrder follows the server's order and keeps unknown ids, stably, at the end", () => {
  assert.deepEqual(S.rankOrder(["zcash", "piratechain", "komodo", "wcash"], ["zcash", "piratechain", "wcash", "komodo"]), ["zcash", "piratechain", "wcash", "komodo"]);
  assert.deepEqual(S.rankOrder(["a", "x", "b", "y"], ["b", "a"]), ["b", "a", "x", "y"]);
  assert.deepEqual(S.rankOrder([], ["a"]), []);
});

// A container with just enough of the DOM: children, getAttribute and appendChild (which moves).
function fakeList(ids, lead = []) {
  const el = (id, attr = true) => ({ id, getAttribute: (n) => (n === "data-rank-id" && attr ? id : null) });
  const list = { children: [...lead.map((h) => el(h, false)), ...ids.map((id) => el(id))], moves: 0 };
  list.appendChild = (node) => { list.children = list.children.filter((c) => c !== node).concat(node); list.moves++; return node; };
  return list;
}
const order = (l) => l.children.map((c) => c.id);

test("applyRank reorders the ranked children in place and leaves a group heading first", () => {
  const nodesBefore = fakeList(["zcash", "piratechain", "komodo", "wcash", "kerrigan"], ["heading"]);
  const same = new Set(nodesBefore.children);
  assert.equal(S.applyRank(nodesBefore, ["zcash", "piratechain", "wcash", "komodo", "kerrigan"]), true);
  assert.deepEqual(order(nodesBefore), ["heading", "zcash", "piratechain", "wcash", "komodo", "kerrigan"]);
  assert.ok(nodesBefore.children.every((c) => same.has(c)), "the same nodes are moved, not rebuilt");
  // Already in order: nothing moves (so focus and selection are untouched).
  const l = fakeList(["a", "b"]);
  assert.equal(S.applyRank(l, ["a", "b"]), false);
  assert.equal(l.moves, 0);
});

test("calculator guard: a Z15 Pro against a ~522 kSol/s WEC network is n/a, never a capped share", () => {
  const e = S.estimate({ hr: 840, w: 2780, pw: 0.08, fee: 1, price: null, rw: 8.9, net: 522000, bt: 75 });
  assert.equal(e.coinsDay, null);
  assert.equal(e.sharePct, null);
  assert.equal(e.rev, null);
  assert.equal(e.guard, GUARD_840);
  assert.ok(Math.abs(e.powDay - 5.3376) < 1e-9, "electricity still computed");
  assert.equal(S.shareGuard(840, 522000), GUARD_840);
  assert.match(S.shareGuard(52.2, 522000), /10\.0% of the network estimate/, "10% is inclusive");
  assert.equal(S.shareGuard(52.1, 522000), null);
});

test("calculator: below the guard the share-based estimate is unchanged", () => {
  const e = S.estimate({ hr: 840, w: 2780, pw: 0.08, fee: 1, price: 1300, rw: 1.25, net: 30e9, bt: 75 });
  const share = (840 * 1000) / 30e9;
  assert.equal(e.guard, null);
  assert.ok(Math.abs(e.coinsDay - share * (86400 / 75) * 1.25 * 0.99) < 1e-12);
  assert.ok(Math.abs(e.sharePct - share * 100) < 1e-12);
  assert.ok(Math.abs(e.profit - (e.coinsDay * 1300 - e.powDay)) < 1e-9);
  assert.equal(S.estimate({ hr: 840, rw: 1, net: 0, bt: 75 }).coinsDay, null, "no network figure, no estimate");
});
