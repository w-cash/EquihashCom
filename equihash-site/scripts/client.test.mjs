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

// ---------- live share-mode transitions ----------
// A small fake DOM: enough for the selectors applyShareModes uses (tag, .class, [attr], [attr="v"]),
// innerHTML / outerHTML setters (outerHTML re-parses the first tag's attributes) and classList.
function node(tag, attrs = {}, kids = [], html = "") {
  const n = { tagName: tag.toUpperCase(), attrs: { ...attrs }, children: kids, parent: null, _html: html, text: "" };
  kids.forEach((k) => (k.parent = n));
  n.getAttribute = (a) => (a in n.attrs ? n.attrs[a] : null);
  n.setAttribute = (a, v) => { n.attrs[a] = String(v); };
  n.removeAttribute = (a) => { delete n.attrs[a]; };
  n.classList = {
    has: (c) => (n.attrs.class || "").split(/\s+/).includes(c),
    toggle: (c, on) => { const s = new Set((n.attrs.class || "").split(/\s+/).filter(Boolean)); if (on) s.add(c); else s.delete(c); n.attrs.class = [...s].join(" "); },
  };
  Object.defineProperty(n, "innerHTML", { get: () => n._html, set: (h) => { n._html = h; n.children = []; } });
  Object.defineProperty(n, "textContent", { get: () => n.text || n._html.replace(/<[^>]*>/g, ""), set: (t) => { n.text = t; n._html = t; } });
  Object.defineProperty(n, "outerHTML", {
    set: (h) => {
      const m = h.match(/^<(\w+)([^>]*)>([\s\S]*)<\/\1>\s*$/);
      const attrs = {}; for (const a of m[2].matchAll(/([\w-]+)="([^"]*)"/g)) attrs[a[1]] = a[2];
      const repl = node(m[1], attrs, [], m[3]);
      const sib = n.parent.children; sib[sib.indexOf(n)] = repl; repl.parent = n.parent;
    },
  });
  const matches = (el, sel) => {
    const m = sel.match(/^(\w+)?(?:\.([\w-]+))?(?:\[([\w-]+)(?:="([^"]*)")?\])?$/);
    if (!m) throw new Error("fake DOM can't match " + sel);
    if (m[1] && el.tagName !== m[1].toUpperCase()) return false;
    if (m[2] && !el.classList.has(m[2])) return false;
    if (m[3] && el.getAttribute(m[3]) == null) return false;
    if (m[4] !== undefined && el.getAttribute(m[3]) !== m[4]) return false;
    return true;
  };
  n.querySelectorAll = (sel) => { const out = []; const walk = (e) => e.children.forEach((k) => { if (matches(k, sel)) out.push(k); walk(k); }); walk(n); return out; };
  n.querySelector = (sel) => n.querySelectorAll(sel)[0] || null;
  return n;
}

// The markup the server's templates render in each mode (src/views/home.rs share_cell / split_bar).
const NETWORK_CELL = '<span class="bar" aria-hidden="true"><span style="width:84.70%"></span></span>84.7%';
const SOLO_NOTE = "ZecWec is the only listed pool. Its 20-minute hashrate (560 kSol/s) reads above the network estimate from the last 120 blocks (511 kSol/s). The two cover different windows, so no share above 100% is shown.";
const SOLO_CELL = `<span class="share-note" title="${SOLO_NOTE}">only listed pool</span>`;
const SPLIT_NETWORK = '<figure class="split" data-split-coin="wcash"><figcaption>Share of WEC network hashrate</figcaption><div class="split-bar" role="img"></div></figure>';
const SPLIT_SOLO = `<figure class="split" data-split-coin="wcash"><figcaption>Share of WEC network hashrate</figcaption><p class="split-note solo" data-f="share-note">${SOLO_NOTE}</p></figure>`;
const CONC_NETWORK = '<div class="conc" data-conc-view="wcash"><div class="alert" role="note"><p><strong>Concentration. </strong>ZecWec has 84.7% of Wcash\'s network hashrate.</p></div></div>';
const CONC_NONE = '<div class="conc" data-conc-view="wcash"></div>';

function page() {
  const row = node("tr", { class: "pool-row over", "data-pool-id": "wcash:zecwec.com", "data-share": "84.7" }, [
    node("td", { class: "num hash" }, [node("span", { class: "m-share" }, [], "84.7%")]),
    node("td", { class: "num share", "data-share-pool": "wcash:zecwec.com" }, [], NETWORK_CELL),
  ]);
  const kv = node("tr", { "data-share-kv": "wcash:zecwec.com" }, [node("th", {}, [], "Share of network"), node("td", {}, [], '<span class="red">84.7%</span>')]);
  const th = node("th", { "data-sort": "share", title: "Share of the network's hashrate" });
  return node("main", {}, [node("p", { class: "conc-host" }, [node("div", { class: "conc", "data-conc-view": "wcash" }, [], CONC_NETWORK)]), node("figure", { class: "split", "data-split-coin": "wcash" }, [], SPLIT_NETWORK), th, row, kv]);
}
const live = (mode) => ({
  pools: {
    "wcash:zecwec.com": mode === "solo"
      ? { share_pct: null, share_status: "only_listed_pool", share_basis: "listed_pools", listed_share_pct: null, share_sort: 100, share_flag: false, share_note: SOLO_NOTE, share_short: "only listed pool", share_cell_html: SOLO_CELL, share_kv_label: "Share of pool-reported hashrate", share_kv_html: `only listed pool<br><span class="na">${SOLO_NOTE}</span>` }
      : { share_pct: 84.7, share_status: "network", share_basis: "network", listed_share_pct: null, share_sort: 84.7, share_flag: true, share_note: null, share_short: "84.7%", share_cell_html: NETWORK_CELL, share_kv_label: "Share of network", share_kv_html: '<span class="red">84.7%</span>' },
  },
  coins: { wcash: mode === "solo"
    ? { share_th_title: "Share of the hashrate the listed pools report (the network estimate reads below their sum)", split_html: SPLIT_SOLO, concentration_html: CONC_NONE }
    : { share_th_title: "Share of the network's hashrate", split_html: SPLIT_NETWORK, concentration_html: CONC_NETWORK } },
});

test("share mode: pools crossing above the network switch every share element to 'only listed pool' and back", () => {
  const root = page();
  S.applyShareModes(root, live("network"), { coin: "wcash" }); // first answer: same mode as rendered
  // Listed pool total crosses above the network estimate.
  assert.ok(S.applyShareModes(root, live("solo"), { coin: "wcash" }) > 0);
  const cell = root.querySelector("[data-share-pool]");
  assert.equal(cell.innerHTML, SOLO_CELL, "share cell shows 'only listed pool' with the reason");
  assert.equal(root.querySelector(".m-share").textContent, "only listed pool");
  const row = root.querySelector("tr[data-pool-id]");
  assert.equal(row.classList.has("over"), false, "no concentration mark against a pools-only basis");
  assert.equal(row.getAttribute("data-share"), "100");
  assert.match(root.querySelector("[data-split-coin]").innerHTML, /split-note solo/, "split bar replaced by the explanation");
  assert.equal(root.querySelector("[data-conc-view]").innerHTML, "", "concentration alert removed");
  assert.equal(root.querySelector('th[data-sort="share"]').getAttribute("title"), "Share of the hashrate the listed pools report (the network estimate reads below their sum)");
  const kv = root.querySelector("[data-share-kv]");
  assert.equal(kv.querySelector("th").textContent, "Share of pool-reported hashrate");
  assert.match(kv.querySelector("td").innerHTML, /^only listed pool/);
  // The same answer again changes nothing.
  assert.equal(S.applyShareModes(root, live("solo"), { coin: "wcash" }), 0);
  // And back below the network estimate: a percentage, the bar and the alert return.
  assert.ok(S.applyShareModes(root, live("network"), { coin: "wcash" }) > 0);
  assert.equal(root.querySelector("[data-share-pool]").innerHTML, NETWORK_CELL);
  assert.equal(root.querySelector(".m-share").textContent, "84.7%");
  assert.equal(root.querySelector("tr[data-pool-id]").classList.has("over"), true);
  assert.match(root.querySelector("[data-split-coin]").innerHTML, /split-bar/);
  assert.match(root.querySelector("[data-conc-view]").innerHTML, /Concentration/);
  assert.equal(root.querySelector('th[data-sort="share"]').getAttribute("title"), "Share of the network's hashrate");
  assert.equal(root.querySelector("[data-share-kv]").querySelector("th").textContent, "Share of network");
});

test("share mode: pools the answer doesn't mention are left alone", () => {
  const root = page();
  assert.equal(S.applyShareModes(root, { pools: {}, coins: {} }, { coin: "wcash" }), 0);
  assert.equal(root.querySelector("[data-share-pool]").innerHTML, NETWORK_CELL);
});

test("mergeSharePool keeps the drawer's record in step, with no ambiguous 100%", () => {
  const row = { id: "wcash:zecwec.com", network_share_pct: 84.7, share_basis: "network", share_flag: true, share_note: null };
  S.mergeSharePool(row, live("solo").pools["wcash:zecwec.com"]);
  assert.deepEqual([row.share_basis, row.share_status, row.share_flag, row.share_capped], ["pools", "only_listed_pool", false, false]);
  assert.equal(row.share_note, SOLO_NOTE, "the drawer shows 'only listed pool' with the reason, not 100%");
  S.mergeSharePool(row, live("network").pools["wcash:zecwec.com"]);
  assert.deepEqual([row.share_basis, row.network_share_pct, row.share_note], ["network", 84.7, null]);
});

test("copyText uses the clipboard API, falls back to execCommand, and reports failure", async () => {
  let wrote = null;
  assert.equal(await S.copyText("https://equihash.com/pool/x", null, { clipboard: { writeText: async (t) => { wrote = t; } } }), true);
  assert.equal(wrote, "https://equihash.com/pool/x");
  const made = [];
  const doc = (ok) => ({
    body: { appendChild: (el) => made.push(el) },
    createElement: () => ({ style: {}, setAttribute() {}, select() {}, remove() { this.removed = true; } }),
    execCommand: (c) => c === "copy" && ok,
  });
  const denied = { clipboard: { writeText: async () => { throw new Error("NotAllowedError"); } } };
  assert.equal(await S.copyText("u", doc(true), denied), true, "fallback copies");
  assert.ok(made[0].removed && made[0].value === "u", "temporary textarea removed");
  assert.equal(await S.copyText("u", doc(false), {}), false, "no way to copy: false, so the page offers the link to copy by hand");
});
