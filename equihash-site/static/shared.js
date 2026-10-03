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

  // ---------- live share modes (server-rendered fragments from /api/live) ----------
  // When a coin's listed-pool total crosses the network estimate, what its shares mean changes:
  // a percentage of the network, "only listed pool", or a share of the listed pools' total. The
  // server sends the markup its own templates would render now, so an open page ends up exactly
  // like a fresh one. Each element remembers the markup it last got, so an unchanged answer
  // touches nothing.
  const LAST = typeof Symbol === "function" ? Symbol("eq-share-html") : "__eqShareHtml";
  function swap(el, html, outer) {
    if (html == null || el[LAST] === html) return false;
    if (outer) el.outerHTML = html; else { el.innerHTML = html; el[LAST] = html; }
    return true;
  }

  /** Apply one /api/live answer's share fields to `root` (a document or any element):
   *  share cells, the small-screen share, the row's over-30% mark and sort value, the pool page's
   *  share row, each coin's split figure, the Share column tooltip (`opts.coin` = the coin the
   *  table shows) and the concentration alert. Returns how many elements changed. */
  function applyShareModes(root, live, opts = {}) {
    const pools = (live && live.pools) || {}, coins = (live && live.coins) || {};
    let n = 0;
    for (const el of root.querySelectorAll("[data-share-pool]")) { const p = pools[el.getAttribute("data-share-pool")]; if (p && swap(el, p.share_cell_html)) n++; }
    for (const tr of root.querySelectorAll("tr[data-pool-id]")) {
      const p = pools[tr.getAttribute("data-pool-id")]; if (!p) continue;
      const m = tr.querySelector(".m-share");
      if (m && p.share_short != null && m.textContent !== p.share_short) { m.textContent = p.share_short; n++; }
      tr.classList.toggle("over", !!p.share_flag);
      if (p.share_sort == null) tr.removeAttribute("data-share"); else tr.setAttribute("data-share", String(p.share_sort));
    }
    for (const kv of root.querySelectorAll("[data-share-kv]")) {
      const p = pools[kv.getAttribute("data-share-kv")]; if (!p) continue;
      const th = kv.querySelector("th"), td = kv.querySelector("td");
      if (th && p.share_kv_label && th.textContent !== p.share_kv_label) { th.textContent = p.share_kv_label; n++; }
      if (td && swap(td, p.share_kv_html)) n++;
    }
    // Whole-element swaps: the replacement carries the same data attribute, so the next answer
    // finds it again. Remember what each one got on the root (the element itself is replaced).
    const seen = (root[LAST] = root[LAST] || {});
    const outer = (sel, attr, htmlFor) => {
      for (const el of root.querySelectorAll(sel)) {
        const key = attr + ":" + el.getAttribute(attr), html = htmlFor(el.getAttribute(attr));
        if (html == null || seen[key] === html) continue;
        seen[key] = html; el.outerHTML = html; n++;
      }
    };
    outer("[data-split-coin]", "data-split-coin", (id) => coins[id] && coins[id].split_html);
    outer("[data-conc-view]", "data-conc-view", (v) => (v === "all" ? live && live.concentration_all_html : coins[v] && coins[v].concentration_html));
    const c = opts.coin && coins[opts.coin];
    if (c && c.share_th_title) for (const th of root.querySelectorAll('th[data-sort="share"]')) { if (th.getAttribute("title") !== c.share_th_title) { th.setAttribute("title", c.share_th_title); n++; } }
    return n;
  }

  /** Bring one pool record of the page's #pool-data (what the drawer reads) up to date with its
   *  /api/live entry. `share_pct` there is null unless the share is of the network. */
  function mergeSharePool(row, p) {
    if (!row || !p) return row;
    row.network_share_pct = p.share_sort == null ? null : p.share_sort;
    row.share_note = p.share_note == null ? null : p.share_note;
    row.share_flag = !!p.share_flag;
    row.share_status = p.share_status;
    row.share_basis = p.share_basis === "listed_pools" ? "pools" : p.share_basis === "network" ? "network" : "none";
    row.share_capped = p.share_status === "pools_exceed_network" && p.share_basis === "network";
    if ("hashrate" in p) row.hashrate = p.hashrate;
    if ("observed_at" in p) row.hashrate_observed_at = p.observed_at;
    return row;
  }

  // ---------- copy link ----------
  /** Copy `text`: the async clipboard API where it's allowed, else a hidden textarea and
   *  execCommand("copy"). Resolves true when it worked. `doc`/`nav` are injectable for tests. */
  async function copyText(text, doc = typeof document !== "undefined" ? document : null, nav = typeof navigator !== "undefined" ? navigator : null) {
    try { if (nav && nav.clipboard && nav.clipboard.writeText) { await nav.clipboard.writeText(text); return true; } } catch (e) { /* fall through */ }
    if (!doc) return false;
    const ta = doc.createElement("textarea");
    ta.value = text; ta.setAttribute("readonly", ""); ta.setAttribute("aria-hidden", "true");
    ta.style.position = "fixed"; ta.style.top = "0"; ta.style.left = "0"; ta.style.opacity = "0";
    doc.body.appendChild(ta);
    let ok = false;
    try { ta.select(); ok = !!doc.execCommand("copy"); } catch (e) { ok = false; }
    ta.remove();
    return ok;
  }

  return { OUTWEIGH_SHARE, shareGuard, estimate, rankOrder, applyRank, applyShareModes, mergeSharePool, copyText };
});
