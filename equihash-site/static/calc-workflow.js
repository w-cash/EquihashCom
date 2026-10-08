/* Calculator setup persistence and explanation workflow.
   Pure helpers are exported for Node tests; browser setup stores only an explicit allow-list and
   never puts pool-result amounts, pool choice or credentials in the URL. */
(function (root, factory) {
  const api = factory();
  if (typeof module === "object" && module.exports) module.exports = api;
  else root.EqCalcWorkflow = api;
  if (typeof document !== "undefined") {
    const start = () => api.init(document, root);
    if (document.readyState === "loading") document.addEventListener("DOMContentLoaded", start);
    else start();
  }
})(typeof self !== "undefined" ? self : this, function () {
  "use strict";

  const STORAGE_KEY = "eq-mining-setup-v1";
  const PRICE_MAX_AGE_SECONDS = 30 * 60;
  const NETWORK_MAX_AGE_SECONDS = 15 * 60;
  const POOL_POLICY_MAX_AGE_SECONDS = 7 * 24 * 60 * 60;
  const ALLOWED = [
    "coin", "hashrate", "watts", "power", "fee", "price", "reward", "nethash", "blocktime",
    "rateBasis", "poolId", "productId",
    "periodDays", "credited", "matured", "withdrawable",
  ];

  function numberOrNull(value) {
    if (value == null || String(value).trim() === "") return null;
    const n = Number(value);
    return Number.isFinite(n) ? n : null;
  }

  function sourceState(observedAt, userEntered, nowMs, maxAgeSeconds) {
    if (userEntered) return "user-entered";
    if (!observedAt) return "missing-time";
    const at = Date.parse(observedAt);
    if (!Number.isFinite(at)) return "missing-time";
    const age = (nowMs - at) / 1000;
    if (age < 0) return "future-time";
    return age <= maxAgeSeconds ? "fresh" : "stale";
  }

  function sourceUsable(state) {
    return state === "fresh" || state === "user-entered";
  }

  function cleanSetup(value) {
    const raw = value && typeof value === "object" && !Array.isArray(value) ? value : {};
    const out = { version: 1 };
    for (const key of ALLOWED) {
      if (!(key in raw)) continue;
      const v = raw[key];
      if (v == null || typeof v === "boolean") continue;
      const text = String(v).trim().slice(0, 120);
      if (text) out[key] = text;
    }
    return out;
  }

  function parseStoredSetup(text) {
    try { return cleanSetup(JSON.parse(text || "{}")); }
    catch (_) { return { version: 1 }; }
  }

  function estimateScenario(input, shared) {
    const hr = numberOrNull(input.hr), watts = numberOrNull(input.watts);
    const powerRate = numberOrNull(input.powerRate), fee = numberOrNull(input.fee);
    const price = numberOrNull(input.price), reward = numberOrNull(input.reward);
    const net = numberOrNull(input.net), bt = numberOrNull(input.bt);
    const powerDay = watts != null && powerRate != null ? (watts / 1000) * 24 * powerRate : null;
    const out = { coinsDay: null, revenueDay: null, powerDay, marginDay: null, sharePct: null, guard: null };
    if (!(hr > 0 && reward > 0 && net > 0 && bt > 0) || fee == null || fee < 0 || fee > 100) return out;
    const base = shared && shared.estimate
      ? shared.estimate({ hr, w: 0, pw: 0, fee, price, rw: reward, net, bt })
      : null;
    if (base) {
      out.coinsDay = base.coinsDay;
      out.revenueDay = base.rev;
      out.sharePct = base.sharePct;
      out.guard = base.guard;
    } else {
      const ratio = (hr * 1000) / net;
      if (ratio >= 0.1) { out.guard = "Hashrate is 10% or more of the network estimate."; return out; }
      out.sharePct = ratio * 100;
      out.coinsDay = ratio * (86400 / bt) * reward * (1 - fee / 100);
      out.revenueDay = price == null ? null : out.coinsDay * price;
    }
    out.marginDay = out.revenueDay != null && powerDay != null ? out.revenueDay - powerDay : null;
    return out;
  }

  function comparison(expectedPerDay, periodDays, credited, matured, withdrawable, symbol) {
    const days = numberOrNull(periodDays);
    const expected = expectedPerDay != null && days > 0 ? expectedPerDay * days : null;
    const fmt = (v) => v == null ? "n/a" : `${v >= 100 ? v.toFixed(2) : v >= 1 ? v.toFixed(4) : v.toFixed(6)} ${symbol || "coins"}`;
    return {
      expected: fmt(expected),
      credited: fmt(numberOrNull(credited)),
      matured: fmt(numberOrNull(matured)),
      withdrawable: fmt(numberOrNull(withdrawable)),
      attributionSupported: false,
      note: "Time-aligned network, accepted-work, pool-accounting and payout-window history is unavailable. A difference alone does not establish underpayment, fee error or pool fault.",
    };
  }

  function init(doc, win) {
    const form = doc.querySelector("#calc");
    if (!form || form.dataset.workflowReady === "1") return;
    form.dataset.workflowReady = "1";
    const byId = (id) => doc.getElementById(id);
    const readJson = (id) => { try { return JSON.parse(byId(id)?.textContent || "[]"); } catch (_) { return []; } };
    const coins = readJson("calc-data"), pools = readJson("calc-pool-data");
    const query = new URLSearchParams(win.location?.search || "");
    let saved = { version: 1 };
    try { saved = parseStoredSetup(win.localStorage?.getItem(STORAGE_KEY)); } catch (_) {}

    const setupFields = {
      coin: byId("c-coin"), hashrate: byId("c-hashrate"), watts: byId("c-watts"),
      power: byId("c-power"), fee: byId("c-fee"), price: byId("c-price"),
      reward: byId("c-reward"), nethash: byId("c-nethash"), blocktime: byId("c-blocktime"),
      rateBasis: byId("c-rate-basis"),
      poolId: byId("c-pool"), productId: byId("c-product"), periodDays: byId("c-period-days"),
      credited: byId("c-credited"), matured: byId("c-matured"), withdrawable: byId("c-withdrawable"),
    };
    for (const [key, el] of Object.entries(setupFields)) {
      if (!el || saved[key] == null || query.has(key)) continue;
      if (!["poolId", "productId"].includes(key)) el.value = saved[key];
    }
    for (const key of ["price", "nethash", "reward", "blocktime", "fee"]) {
      if (query.has(key)) byId("c-" + key)?.setAttribute("data-user-entered", "1");
    }

    function currentCoin() { return coins.find((c) => c.id === setupFields.coin?.value) || {}; }
    function currentPool() { return pools.find((p) => p.id === setupFields.poolId?.value && p.coin === setupFields.coin?.value); }
    function productRows(pool) {
      if (!pool) return [];
      const rows = (pool.schemes || []).map((s) => ({ id: s.name, name: s.name, fee: s.fee }));
      for (const name of pool.payout_schemes || []) if (!rows.some((r) => r.id === name)) rows.push({ id: name, name, fee: null });
      if (!rows.length) rows.push({ id: "headline", name: "Pool headline terms", fee: pool.fee });
      return rows;
    }
    function fillProducts(preferred) {
      const select = setupFields.productId, pool = currentPool(); if (!select) return;
      select.replaceChildren();
      const rows = productRows(pool);
      if (!rows.length) {
        select.add(new Option("Select a pool first", "")); select.disabled = true; return;
      }
      select.disabled = false;
      select.add(new Option("Choose a pool product", ""));
      for (const row of rows) select.add(new Option(row.name, row.id));
      if (preferred && rows.some((r) => r.id === preferred)) select.value = preferred;
    }
    function fillPools() {
      const select = setupFields.poolId; if (!select) return;
      const preferred = saved.coin === setupFields.coin?.value ? saved.poolId : null;
      select.replaceChildren(new Option("No pool selected", ""));
      for (const pool of pools.filter((p) => p.coin === setupFields.coin?.value).sort((a, b) => a.name.localeCompare(b.name))) {
        select.add(new Option(pool.name, pool.id));
      }
      if (preferred && [...select.options].some((o) => o.value === preferred)) select.value = preferred;
      fillProducts(select.value ? saved.productId : null);
      if (select.value && setupFields.productId?.value) applyProduct();
    }
    function applyProduct() {
      const pool = currentPool(), product = productRows(pool).find((p) => p.id === setupFields.productId?.value);
      const fee = product?.fee ?? pool?.fee ?? null;
      if (setupFields.fee) {
        setupFields.fee.value = fee == null ? "" : String(fee);
        setupFields.fee.dataset.userEntered = "0";
        setupFields.fee.dataset.sourceObservedAt = pool?.fee_observed_at || "";
      }
      const h = byId("h-product");
      if (h) h.textContent = pool
        ? `${product?.name || "Product not selected"}. Minimum payout ${pool.minimum_payout == null ? "not published" : pool.minimum_payout + " " + (pool.minimum_payout_unit || "")}.`
        : "Payout scheme, fee and threshold are separate pool terms. Missing terms remain unknown.";
    }
    fillPools();

    function coinSources() {
      const coin = currentCoin();
      const map = [["price", "price_observed_at"], ["nethash", "nethash_observed_at"]];
      for (const [field, source] of map) {
        const el = byId("c-" + field); if (!el) continue;
        el.dataset.sourceObservedAt = coin[source] || "";
        const restoredForThisCoin = saved.coin === setupFields.coin?.value && saved[field] != null;
        el.dataset.userEntered = query.has(field) || restoredForThisCoin ? "1" : "0";
      }
    }
    coinSources();
    if (setupFields.fee && !currentPool() && saved.fee != null) setupFields.fee.dataset.userEntered = "1";

    const money = (v) => v == null ? "n/a" : (v <= -0.005 ? "−$" : "$") + Math.abs(v).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 });
    const coinFmt = (v) => v == null ? "n/a" : v >= 100 ? v.toFixed(2) : v >= 1 ? v.toFixed(4) : v.toFixed(6);
    const set = (id, text) => { const el = byId(id); if (el) el.textContent = text; };
    function statusText(state, limit) {
      if (state === "user-entered") return "User-entered scenario; source age is not asserted.";
      if (state === "fresh") return `Within the ${limit} current-data window.`;
      if (state === "stale") return `Outside the ${limit} current-data window; current economics withheld.`;
      if (state === "future-time") return "Observation time is in the future; current economics withheld.";
      return "Observation time unavailable; current economics withheld.";
    }
    function reconcile() {
      const now = Date.now(), priceEl = byId("c-price"), netEl = byId("c-nethash"), feeEl = setupFields.fee;
      const priceState = sourceState(priceEl?.dataset.sourceObservedAt, priceEl?.dataset.userEntered === "1", now, PRICE_MAX_AGE_SECONDS);
      const netState = sourceState(netEl?.dataset.sourceObservedAt, netEl?.dataset.userEntered === "1", now, NETWORK_MAX_AGE_SECONDS);
      const feeMaxAge = currentPool()?.fee_max_age_seconds || POOL_POLICY_MAX_AGE_SECONDS;
      const feeState = feeEl?.dataset.userEntered === "1"
        ? "user-entered"
        : sourceState(feeEl?.dataset.sourceObservedAt, false, now, feeMaxAge);
      set("s-price", statusText(priceState, "30-minute"));
      set("s-nethash", statusText(netState, "15-minute"));
      set("s-fee", feeEl?.value.trim() ? statusText(feeState, "seven-day pool-policy") : "Unknown until entered or a dated pool product is selected.");
      const values = {
        hr: byId("c-hashrate")?.value, watts: byId("c-watts")?.value, powerRate: byId("c-power")?.value,
        fee: sourceUsable(feeState) ? feeEl?.value : null,
        price: sourceUsable(priceState) ? priceEl?.value : null,
        reward: byId("c-reward")?.value,
        net: sourceUsable(netState) ? netEl?.value : null,
        bt: byId("c-blocktime")?.value,
      };
      const result = estimateScenario(values, win.EqShared);
      const x30 = (v) => v == null ? null : v * 30;
      set("o-coins", coinFmt(result.coinsDay)); set("o-coins30", coinFmt(x30(result.coinsDay)));
      set("o-rev", money(result.revenueDay)); set("o-rev30", money(x30(result.revenueDay)));
      set("o-pow", money(result.powerDay == null ? null : -result.powerDay)); set("o-pow30", money(result.powerDay == null ? null : -result.powerDay * 30));
      set("o-profit", money(result.marginDay)); set("o-profit30", money(x30(result.marginDay)));
      set("o-share", result.sharePct == null ? "n/a" : result.sharePct < 0.01 ? "<0.01%" : result.sharePct.toFixed(result.sharePct < 10 ? 2 : 1) + "%");
      const watts = numberOrNull(values.watts);
      set("o-be", result.revenueDay != null && watts > 0 ? "$" + (result.revenueDay / ((watts / 1000) * 24)).toFixed(3) + "/kWh" : "n/a");
      const missing = [];
      if (numberOrNull(values.hr) == null) missing.push("hashrate");
      if (numberOrNull(values.fee) == null) missing.push("current pool fee");
      if (numberOrNull(values.reward) == null) missing.push("miner reward");
      if (numberOrNull(values.net) == null) missing.push("current network hashrate");
      if (numberOrNull(values.bt) == null) missing.push("block time");
      if (numberOrNull(values.price) == null) missing.push("current price for fiat results");
      if (numberOrNull(values.watts) == null) missing.push("wall power for electricity");
      if (numberOrNull(values.powerRate) == null) missing.push("electricity rate");
      const warning = byId("o-missing");
      if (warning) { warning.hidden = !missing.length; warning.textContent = missing.length ? `Estimate incomplete: ${missing.join(", ")}. Unknown inputs are not treated as zero.` : ""; }
      const cmp = comparison(result.coinsDay, setupFields.periodDays?.value, setupFields.credited?.value, setupFields.matured?.value, setupFields.withdrawable?.value, currentCoin().symbol);
      set("x-expected", cmp.expected); set("x-credited", cmp.credited); set("x-matured", cmp.matured); set("x-withdrawable", cmp.withdrawable);
      const attr = byId("x-attribution"); if (attr) attr.textContent = "Attribution unavailable: " + cmp.note;
      return result;
    }
    function save() {
      const value = { version: 1 };
      for (const [key, el] of Object.entries(setupFields)) if (el && el.value.trim()) value[key] = el.value.trim();
      saved = cleanSetup(value);
      try { win.localStorage?.setItem(STORAGE_KEY, JSON.stringify(saved)); } catch (_) {}
    }
    // app.js still owns legacy progressive enhancement. It is loaded after this page-specific
    // module, so reconcile on the next task and make the stricter missingness/source contract the
    // final rendered state without coupling the two files.
    let pending = null;
    function scheduleReconcile() {
      if (pending != null) win.clearTimeout(pending);
      pending = win.setTimeout(() => { pending = null; reconcile(); }, 0);
    }
    form.addEventListener("input", (event) => {
      if (["c-price", "c-nethash", "c-fee", "c-reward", "c-blocktime"].includes(event.target?.id)) event.target.dataset.userEntered = "1";
      save(); scheduleReconcile();
    });
    form.addEventListener("change", (event) => {
      if (event.target === setupFields.coin) { coinSources(); fillPools(); }
      if (event.target === setupFields.poolId) fillProducts(null);
      if (event.target === setupFields.productId) applyProduct();
      save(); scheduleReconcile();
    });
    form.addEventListener("submit", scheduleReconcile);
    byId("calc-reconciliation")?.addEventListener("input", () => { save(); scheduleReconcile(); });
    byId("clear-local-setup")?.addEventListener("click", () => {
      try { win.localStorage?.removeItem(STORAGE_KEY); } catch (_) {}
      for (const key of ["periodDays", "credited", "matured", "withdrawable"]) if (setupFields[key]) setupFields[key].value = "";
      if (setupFields.poolId) setupFields.poolId.value = "";
      fillProducts(null);
      const status = byId("setup-storage-status");
      if (status) status.textContent = "Saved local setup removed. Values already present on this page remain until it is reloaded.";
      scheduleReconcile();
    });
    scheduleReconcile();
  }

  return {
    STORAGE_KEY, PRICE_MAX_AGE_SECONDS, NETWORK_MAX_AGE_SECONDS, POOL_POLICY_MAX_AGE_SECONDS,
    numberOrNull, sourceState, sourceUsable, cleanSetup, parseStoredSetup, estimateScenario,
    comparison, init,
  };
});
