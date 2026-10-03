/* equihash.com: pure helpers used by app.js and by the node tests (scripts/client.test.mjs).
   No DOM access at load time; the DOM helpers only touch the nodes they are given. */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.EqShared = api;
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  // ---------- calculator (mirrors src/views/calc.rs) ----------
  // At this share of the network estimate or more, a share-based figure isn't meaningful.
  const OUTWEIGH_SHARE = 0.1;

  /** Reason coins per day is n/a, or null. Same wording as calc::share_guard. */
  function shareGuard(hr, net) {
    if (!(net > 0 && hr > 0) || hr * 1000 < OUTWEIGH_SHARE * net) return null;
    const r = (hr * 1000) / net;
    const how = r >= 1 ? `${r.toFixed(2)}× the network estimate` : `${(r * 100).toFixed(1)}% of the network estimate`;
    return `Your hashrate is ${how}. At 10% or more of the network a share-based estimate isn't meaningful: adding it would itself move the network hashrate and difficulty. Coins per day, revenue and profit are shown as n/a.`;
  }

  /** Daily estimate from already-validated inputs. Never caps the share: past the guard it's n/a. */
  function estimate({ hr, w = 0, pw = 0, fee = 0, price = null, rw, net, bt }) {
    const out = { coinsDay: null, powDay: null, rev: null, profit: null, sharePct: null, guard: null };
    if (!(hr > 0 && net > 0 && bt > 0 && rw > 0)) return out;
    out.powDay = (w / 1000) * 24 * pw;
    out.guard = shareGuard(hr, net);
    if (out.guard) return out;
    out.sharePct = ((hr * 1000) / net) * 100;
    out.coinsDay = ((hr * 1000) / net) * (86400 / bt) * rw * (1 - fee / 100);
    out.rev = price != null ? out.coinsDay * price : null;
    out.profit = out.rev != null ? out.rev - out.powDay : null;
    return out;
  }

  // ---------- live re-ranking (order from /api/live "ranking") ----------
  /** The ids in `current`, sorted by their position in `rank`. Ids the server didn't rank keep
   *  their relative order after the ranked ones. Stable. */
  function rankOrder(current, rank) {
    const pos = new Map(rank.map((id, i) => [id, i]));
    return current
      .map((id, i) => ({ id, i, p: pos.has(id) ? pos.get(id) : Infinity }))
      .sort((a, b) => a.p - b.p || a.i - b.i)
      .map((x) => x.id);
  }

  /** Reorder the [data-rank-id] children of one container to match `rank`. Children without the
   *  attribute (a group heading row) stay in front. Moves nodes rather than rebuilding them, so
   *  listeners, selection and focus survive; returns true when anything moved. */
  function applyRank(container, rank) {
    const kids = Array.from(container.children);
    const items = kids.filter((el) => el.getAttribute && el.getAttribute("data-rank-id") != null);
    const cur = items.map((el) => el.getAttribute("data-rank-id"));
    const next = rankOrder(cur, rank);
    if (next.every((id, i) => id === cur[i])) return false;
    const byId = new Map(items.map((el) => [el.getAttribute("data-rank-id"), el]));
    for (const id of next) container.appendChild(byId.get(id));
    return true;
  }

  return { OUTWEIGH_SHARE, shareGuard, estimate, rankOrder, applyRank };
});
