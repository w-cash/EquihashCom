/* equihash.com — progressive enhancement. The site works without this file. */
(() => {
  "use strict";
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
  const NA = "n/a";
  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const json = (id) => { try { return JSON.parse($(id)?.textContent || "null"); } catch { return null; } };

  // ---------- formatting (mirrors src/fmt.rs) ----------
  const trim = (v) => (v === 0 ? "0" : Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2));
  const scale = (h) => { const a = Math.abs(h); return a >= 1e15 ? [h / 1e15, "P"] : a >= 1e12 ? [h / 1e12, "T"] : a >= 1e9 ? [h / 1e9, "G"] : a >= 1e6 ? [h / 1e6, "M"] : a >= 1e3 ? [h / 1e3, "k"] : [h, ""]; };
  const fmtHash = (v, unit = "Sol/s") => { if (v == null || !isFinite(v)) return NA; const [x, p] = scale(v); return `${trim(x)} ${p}${unit.replace("/s", "")}/s`; };
  const fmtPct = (p) => (p == null || !isFinite(p) ? NA : p === 0 ? "0%" : p < 0.01 ? "<0.01%" : p < 10 ? p.toFixed(2) + "%" : p.toFixed(1) + "%");
  const fmtInt = (v) => (v == null || !isFinite(v) ? NA : Math.round(v).toLocaleString("en-US"));
  const short = (v) => String(+(+v).toFixed(4));
  const feeRange = (p) => { const v = (p.schemes || []).map((s) => s.fee_pct).filter((x) => x != null); if (p.fee_pct != null) v.push(p.fee_pct); if (!v.length) return NA; const lo = Math.min(...v), hi = Math.max(...v); return lo === hi ? short(lo) + "%" : `${short(lo)}–${short(hi)}%`; };
  const fmtUtc = (t) => { if (!t) return NA; const d = new Date(t); return isNaN(d) ? NA : d.toISOString().replace("T", " ").slice(0, 16) + " UTC"; };
  const host = (u) => (u ? u.replace(/^https?:\/\//, "").replace(/^www\./, "").replace(/\/$/, "") : NA);
  const ago = (t) => { const s = (Date.now() - new Date(t).getTime()) / 1000; if (!isFinite(s)) return ""; if (s < 90) return "just now"; if (s < 3600) return Math.round(s / 60) + " min ago"; if (s < 172800) return Math.round(s / 3600) + " h ago"; return Math.round(s / 86400) + " days ago"; };

  // ---------- relative times ----------
  function relTimes(scope) { $$("time.ago[datetime]", scope).forEach((el) => { const a = ago(el.getAttribute("datetime")); if (a) { el.title = el.textContent; el.textContent = a; } }); }
  relTimes(document);

  // ---------- copy buttons ----------
  $$("[data-copy]").forEach((b) => b.addEventListener("click", async () => {
    const t = $(b.dataset.copy)?.textContent || "";
    try { await navigator.clipboard.writeText(t); b.textContent = "Copied ✓"; } catch { b.textContent = "Select & copy"; }
    setTimeout(() => (b.textContent = "Copy"), 1800);
  }));

  // ---------- pool table: filter + sort ----------
  const table = $("#pool-table");
  const form = $("#filters");
  const pools = json("#pool-data") || [];
  const coins = json("#coin-data") || [];
  const bySlug = Object.fromEntries(pools.map((p) => [p.slug, p]));

  function readFilters() {
    const f = {};
    if (!form) return f;
    for (const el of $$("[data-filter]", form)) f[el.name] = el.type === "checkbox" ? (el.checked ? "1" : "") : el.value.trim();
    return f;
  }
  function rowMatches(tr, f) {
    const d = tr.dataset;
    if (f.coin && f.coin !== "all" && d.coin !== f.coin) return false;
    if (f.scheme && !d.schemes.split(",").some((s) => s.toLowerCase() === f.scheme.toLowerCase())) return false;
    if (f.region && !d.regions.split(",").includes(f.region)) return false;
    if (f.fee !== "" && f.fee != null) { const fee = d.fee === undefined ? null : +d.fee; if (fee == null || fee > +f.fee + 1e-9) return false; }
    if (f.hr) { const h = d.hashrate === undefined ? null : +d.hashrate; if (h == null || h < +f.hr) return false; }
    if (f.merged === "1" && d.merged !== "1") return false;
    if (f.hide_empty === "1" && !(+d.hashrate > 0)) return false;
    if (f.q && !d.search.includes(f.q.toLowerCase())) return false;
    return true;
  }
  function applyFilters(push = true) {
    if (!table) return;
    const f = readFilters();
    let n = 0;
    for (const tr of $$("tbody tr", table)) { const ok = rowMatches(tr, f); tr.hidden = !ok; if (ok) n++; }
    $("#pool-count").textContent = n;
    $("#empty").hidden = n > 0;
    if (push) {
      const qs = new URLSearchParams();
      for (const [k, v] of Object.entries(f)) if (v) qs.set(k, v);
      const s = sortState(); if (s.key !== "hashrate" || s.dir !== "desc") { qs.set("sort", s.key); qs.set("dir", s.dir); }
      history.replaceState(null, "", "/?" + qs.toString() + "#pools");
    }
    if (f.coin && f.coin !== "all") selectTab(f.coin, false);
  }
  let t;
  if (form) {
    form.addEventListener("input", (e) => { if (e.target.matches("[data-filter]")) { clearTimeout(t); t = setTimeout(applyFilters, e.target.type === "search" ? 120 : 0); } });
    form.addEventListener("submit", (e) => { e.preventDefault(); applyFilters(); });
    const reset = $(".filter-actions .ghost", form);
    reset?.addEventListener("click", (e) => { e.preventDefault(); for (const el of $$("[data-filter]", form)) { if (el.type === "checkbox") el.checked = false; else if (el.name === "coin") el.value = "all"; else el.value = ""; } applyFilters(); });
  }
  function sortState() {
    const th = table && $("th[aria-sort=ascending], th[aria-sort=descending]", table);
    return { key: th?.dataset.sort || "hashrate", dir: th?.getAttribute("aria-sort") === "ascending" ? "asc" : "desc" };
  }
  const textKeys = { name: "name", coin: "coinlabel", region: "region" };
  const numKeys = { hashrate: "hashrate", share: "share", miners: "miners", fee: "fee", minpay: "minpay", blocks: "blocks" };
  function sortBy(key, dir) {
    const tbody = $("tbody", table);
    const rows = $$("tr", tbody);
    const val = (tr) => { if (textKeys[key]) { const v = tr.dataset[textKeys[key]]; return v ? v : null; } const v = tr.dataset[numKeys[key]]; return v === undefined ? null : +v; };
    rows.sort((a, b) => {
      const va = val(a), vb = val(b);
      if (va == null && vb == null) return 0; if (va == null) return 1; if (vb == null) return -1;
      const o = typeof va === "number" ? va - vb : va.localeCompare(vb);
      return dir === "asc" ? o : -o;
    });
    rows.forEach((r) => tbody.appendChild(r));
    $$("th[data-sort]", table).forEach((th) => th.setAttribute("aria-sort", th.dataset.sort === key ? (dir === "asc" ? "ascending" : "descending") : "none"));
    const hs = form && $("input[name=sort]", form), hd = form && $("input[name=dir]", form);
    if (hs) hs.value = key; if (hd) hd.value = dir;
  }
  if (table) {
    $$("th[data-sort] a", table).forEach((a) => a.addEventListener("click", (e) => {
      e.preventDefault();
      const th = a.closest("th"); const key = th.dataset.sort;
      const cur = th.getAttribute("aria-sort");
      const dir = cur === "descending" ? "asc" : cur === "ascending" ? "desc" : (textKeys[key] ? "asc" : "desc");
      sortBy(key, dir); applyFilters();
    }));
    table.addEventListener("click", (e) => {
      const tr = e.target.closest("tr.pool-row"); if (!tr) return;
      const a = e.target.closest("a");
      if (a && !a.classList.contains("pool-link")) return; // external links behave normally
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.button === 1) return;
      e.preventDefault(); openDrawer(tr.dataset.slug);
    });
  }

  // ---------- chart tabs ----------
  function selectTab(id, scroll) {
    const panel = $(`.chart-panel[data-chart="${CSS.escape(id)}"]`); if (!panel) return;
    $$(".chart-panel").forEach((p) => p.classList.toggle("is-hidden", p !== panel));
    $$(".tab[data-tab]").forEach((t) => { const on = t.dataset.tab === id; t.classList.toggle("active", on); t.setAttribute("aria-selected", on); });
    panel.querySelectorAll(".bar-fill").forEach((b) => { b.style.animation = "none"; void b.offsetWidth; b.style.animation = ""; });
    if (scroll) $("#share")?.scrollIntoView({ block: "start" });
  }
  $$(".tab[data-tab]").forEach((t) => t.addEventListener("click", (e) => { e.preventDefault(); selectTab(t.dataset.tab, false); }));
  $$(".bar-row[data-pool]").forEach((r) => {
    r.addEventListener("click", () => openDrawer(r.dataset.pool));
    r.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); openDrawer(r.dataset.pool); } });
  });
  $$("[data-open]").forEach((a) => a.addEventListener("click", (e) => { if (bySlug[a.dataset.open]) { e.preventDefault(); openDrawer(a.dataset.open); } }));

  // ---------- mobile filter toggle ----------
  const ft = $(".filters-toggle");
  if (ft) {
    const form = ft.closest("form");
    const count = () => {
      const n = ["scheme", "fee", "region", "hr"].filter((k) => form.elements[k]?.value).length + ["merged", "hide_empty"].filter((k) => form.elements[k]?.checked).length;
      $(".ft-count", ft).textContent = n ? String(n) : "";
    };
    ft.addEventListener("click", () => { const open = form.classList.toggle("expanded"); ft.setAttribute("aria-expanded", String(open)); });
    form.addEventListener("change", count); count();
  }

  // ---------- drawer ----------
  let lastFocus = null;
  function kv(label, value) { return `<div class="kv"><dt>${esc(label)}</dt><dd>${value}</dd></div>`; }
  const link = (u, l) => (u ? `<a href="${esc(u)}" target="_blank" rel="noopener nofollow">${esc(l || host(u))}</a>` : NA);
  function openDrawer(slug) {
    const p = bySlug[slug]; if (!p) return;
    const c = coins.find((x) => x.id === p.coin_id) || {};
    const unit = p.hashrate_unit || "Sol/s";
    const over = !!p.share_flag;
    const byPools = p.share_basis === "pools";
    const sh = Math.max(0, Math.min(100, p.network_share_pct || 0));
    const R = 26, C = 2 * Math.PI * R;
    const ring = p.network_share_pct == null ? "" : `<svg class="ring" viewBox="0 0 64 64" aria-hidden="true"><circle cx="32" cy="32" r="${R}" class="ring-bg"/><circle cx="32" cy="32" r="${R}" class="ring-30" stroke-dasharray="1.2 ${C}" transform="rotate(${-90 + 108} 32 32)"/><circle cx="32" cy="32" r="${R}" class="ring-fg" stroke-dasharray="0 ${C}" data-dash="${(sh / 100) * C} ${C}" transform="rotate(-90 32 32)"/></svg>`;
    const chips = (p.schemes || []).map((s) => `<span class="chip chip-${esc(s.scheme.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/-$/, ""))}">${esc(s.scheme)}${s.fee_pct != null ? " " + short(s.fee_pct) + "%" : ""}</span>`).join("") || NA;
    const root = $("#drawer-root");
    root.innerHTML = `
      <div class="drawer-backdrop" data-close></div>
      <aside class="drawer" role="dialog" aria-modal="true" aria-labelledby="drawer-title">
        <div class="drawer-head">
          <button class="drawer-close" data-close aria-label="Close">×</button>
          <span class="ticker sm">${esc(p.coin)}</span> <span class="muted sm">${esc(p.coin_label)} · Equihash ${esc(c.params || NA)}</span>
          <h3 id="drawer-title">${esc(p.name)}</h3>
          <div class="muted sm">${link(p.url)}</div>
        </div>
        <div class="drawer-body">
          ${over ? `<div class="warn"><strong>Decentralisation warning:</strong> this pool has ${fmtPct(p.network_share_pct)} of ${esc(p.coin)} ${byPools ? "hashrate reported by listed pools" : "network hashrate"}. Consider a smaller pool.</div>` : ""}
          <div class="drawer-stats">
            <div><span>Hashrate</span><strong>${fmtHash(p.hashrate, unit)}${p.hashrate_is_reported ? "†" : ""}</strong></div>
            <div class="share-stat ${over ? "over" : ""}">${ring}<span>${byPools ? "% listed pools" : "% network"}</span><strong>${fmtPct(p.network_share_pct)}</strong></div>
            <div><span>Fee</span><strong>${feeRange(p)}</strong></div>
          </div>
          <dl class="kv-grid">
            ${kv("Miners", fmtInt(p.miners))}
            ${kv("Workers", fmtInt(p.workers))}
            ${kv("Payout schemes", chips)}
            ${kv("Min payout", p.min_payout != null ? short(p.min_payout) + " " + esc(p.min_payout_unit || "") : NA)}
            ${kv("Blocks (last 1,000)", fmtInt(p.blocks_last_1000))}
            ${kv("Last block", p.last_block_height != null ? fmtInt(p.last_block_height) + " · " + fmtUtc(p.last_block_time) : NA)}
            ${kv("Region", esc(p.region || NA))}
            ${kv("Merged mining", p.merged_mining?.supported ? "Yes: " + esc(p.merged_mining.coins.join(", ")) + (p.merged_mining.note ? `<div class="muted sm">${esc(p.merged_mining.note)}</div>` : "") : "None reported")}
            ${kv("Hashrate note", p.hashrate_is_reported ? "† As reported by the pool operator; not independently measured." : "Measured by the source")}
            ${kv("Source", link(p.source_url, p.source_name))}
            ${kv("Raw data", link(p.data_url))}
            ${kv("On miningpoolstats", p.from_miningpoolstats ? "Yes" : "No (verified directly)")}
            ${kv("Fetched at", p.fetched_at ? `<time class="ago" datetime="${esc(p.fetched_at)}">${esc(fmtUtc(p.fetched_at))}</time> <span class="muted sm">(${esc(fmtUtc(p.fetched_at))})</span>` : NA)}
          </dl>
          ${p.notes ? `<p class="note">${esc(p.notes)}</p>` : ""}
          <p><a class="btn sm" href="/pool/${esc(p.slug)}">Permalink</a></p>
        </div>
      </aside>`;
    lastFocus = document.activeElement;
    document.body.classList.add("drawer-lock");
    requestAnimationFrame(() => requestAnimationFrame(() => {
      root.classList.add("drawer-open"); $(".drawer-close", root).focus();
      $$(".ring-fg", root).forEach((c) => c.setAttribute("stroke-dasharray", c.dataset.dash));
      relTimes(root);
    }));
    root.querySelectorAll("[data-close]").forEach((el) => el.addEventListener("click", closeDrawer));
    // Swipe down on the sheet header to close (mobile bottom sheet).
    const sheet = $(".drawer", root), head = $(".drawer-head", root);
    let y0 = null, dy = 0;
    head.addEventListener("touchstart", (e) => { if (matchMedia("(max-width: 720px)").matches) { y0 = e.touches[0].clientY; dy = 0; sheet.style.transition = "none"; } }, { passive: true });
    head.addEventListener("touchmove", (e) => { if (y0 === null) return; dy = Math.max(0, e.touches[0].clientY - y0); sheet.style.transform = `translateY(${dy}px)`; }, { passive: true });
    head.addEventListener("touchend", () => { if (y0 === null) return; sheet.style.transition = ""; sheet.style.transform = ""; y0 = null; if (dy > 90) closeDrawer(); });
    if (location.hash !== "#pool=" + slug) history.replaceState(null, "", location.pathname + location.search + "#pool=" + slug);
  }
  function closeDrawer() {
    const root = $("#drawer-root");
    if (!root.classList.contains("drawer-open")) return;
    root.classList.remove("drawer-open");
    document.body.classList.remove("drawer-lock");
    setTimeout(() => { if (!root.classList.contains("drawer-open")) root.innerHTML = ""; }, 350);
    history.replaceState(null, "", location.pathname + location.search + "#pools");
    lastFocus?.focus?.();
  }
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") closeDrawer();
    if (e.key === "Tab" && $("#drawer-root.drawer-open")) {
      const f = $$("#drawer-root a, #drawer-root button"); if (!f.length) return;
      const first = f[0], last = f[f.length - 1];
      if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
      else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
    }
  });
  const m = location.hash.match(/^#pool=(.+)$/);
  if (m && bySlug[m[1]]) setTimeout(() => openDrawer(m[1]), 150);

  // ---------- calculator ----------
  const calc = $("#calc");
  if (calc) {
    const cdata = json("#calc-data") || [];
    const g = (n) => { const v = parseFloat($("#c-" + n)?.value); return isFinite(v) ? v : null; };
    const money = (v) => (v == null ? NA : (v < 0 ? "-$" : "$") + Math.abs(v).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }));
    function compute() {
      const hr = g("hashrate"), w = g("watts") || 0, pw = g("power") || 0, fee = g("fee") || 0, price = g("price"), rw = g("reward"), net = g("nethash"), bt = g("blocktime");
      const c = cdata.find((x) => x.id === $("#c-coin").value) || {};
      $("#o-sym").textContent = c.symbol || "";
      let coinsDay = null;
      if (hr > 0 && net > 0 && bt > 0 && rw > 0) coinsDay = (hr * 1000 / net) * (86400 / bt) * rw * (1 - fee / 100);
      const powDay = w / 1000 * 24 * pw;
      const rev = coinsDay != null && price != null ? coinsDay * price : null;
      const profit = rev != null ? rev - powDay : null;
      $("#o-coins").textContent = coinsDay != null ? coinsDay.toFixed(6) : NA;
      $("#o-rev").textContent = money(rev);
      $("#o-pow").textContent = money(powDay);
      const pe = $("#o-profit"); pe.textContent = money(profit); pe.classList.toggle("neg", profit != null && profit < 0);
      $("#o-month").textContent = profit != null ? money(profit * 30) : NA;
      $("#o-share").textContent = hr > 0 && net > 0 ? fmtPct(hr * 1000 / net * 100) : NA;
      $("#o-be").textContent = rev != null && w > 0 ? "$" + (rev / (w / 1000 * 24)).toFixed(3) + "/kWh" : NA;
    }
    function setHint(n, html) { const h = $("#h-" + n); if (h) h.innerHTML = html; }
    function prefill() {
      const c = cdata.find((x) => x.id === $("#c-coin").value); if (!c) return;
      const set = (n, v) => { $("#c-" + n).value = v == null ? "" : +(+v).toPrecision(10); };
      set("price", c.price); set("reward", c.reward); set("nethash", c.nethash); set("blocktime", c.blocktime);
      $$(".input-unit .unit").forEach((u) => { if (u.previousElementSibling?.id === "c-reward") u.textContent = c.symbol; });
      setHint("price", c.price != null ? "miningpoolstats price feed · editable" : "No sourced price: enter one");
      setHint("reward", c.reward != null ? `From <a href="${esc(c.reward_source)}" target="_blank" rel="noopener">${esc(host(c.reward_source).split("/")[0])}</a>` : "Not sourced for this coin: enter it");
      setHint("nethash", c.nethash != null ? "miningpoolstats" : "Not published: enter it");
      compute();
    }
    $("#c-coin").addEventListener("change", prefill);
    calc.addEventListener("input", (e) => { if (e.target.id !== "c-coin") compute(); });
    calc.addEventListener("submit", (e) => { e.preventDefault(); compute(); });
    $$(".chip-btn[data-hr]").forEach((b) => b.addEventListener("click", () => { $("#c-hashrate").value = b.dataset.hr; $("#c-watts").value = b.dataset.w; compute(); }));
    compute();
  }

  // ---------- guide: checklist persistence + active TOC ----------
  const checks = $$(".checklist input[type=checkbox]");
  if (checks.length) {
    let saved = []; try { saved = JSON.parse(localStorage.getItem("eq-mm-checklist") || "[]"); } catch {}
    const progress = () => {
      const n = checks.filter((x) => x.checked).length;
      const fill = $(".ck-fill"), count = $(".ck-count");
      if (fill) fill.style.width = (n / checks.length) * 100 + "%";
      if (count) count.textContent = `${n} / ${checks.length} done${n === checks.length ? " ✓ ready" : ""}`;
      $(".ck-progress")?.classList.toggle("complete", n === checks.length);
    };
    const save = () => { try { localStorage.setItem("eq-mm-checklist", JSON.stringify(checks.map((x) => x.checked))); } catch {} progress(); };
    checks.forEach((c, i) => { c.checked = !!saved[i]; c.addEventListener("change", save); });
    $(".ck-reset")?.addEventListener("click", () => { checks.forEach((c) => (c.checked = false)); save(); });
    progress();
  }
  const tocLinks = $$(".toc a");
  if (tocLinks.length && "IntersectionObserver" in window) {
    const map = new Map(tocLinks.map((a) => [a.getAttribute("href").slice(1), a]));
    const io = new IntersectionObserver((ents) => ents.forEach((en) => { if (en.isIntersecting) { tocLinks.forEach((a) => a.classList.remove("active")); map.get(en.target.id)?.classList.add("active"); } }), { rootMargin: "-20% 0px -70% 0px" });
    map.forEach((_, id) => { const el = document.getElementById(id); if (el) io.observe(el); });
  }
})();
