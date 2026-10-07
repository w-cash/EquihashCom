/* equihash.com: progressive enhancement only. Every page works without this file. */
(() => {
  "use strict";
  const $ = (s, r = document) => r.querySelector(s);
  const $$ = (s, r = document) => Array.from(r.querySelectorAll(s));
  const NA = "n/a";
  const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
  const S = window.EqShared || null; // static/shared.js (pure helpers, also used by the node tests)
  const json = (id) => { try { return JSON.parse($(id)?.textContent || "null"); } catch { return null; } };

  // ---------- formatting (mirrors src/fmt.rs) ----------
  const trim = (v) => (v === 0 ? "0" : Math.abs(v) >= 100 ? v.toFixed(0) : Math.abs(v) >= 10 ? v.toFixed(1) : v.toFixed(2));
  const scale = (h) => { const a = Math.abs(h); return a >= 1e15 ? [h / 1e15, "P"] : a >= 1e12 ? [h / 1e12, "T"] : a >= 1e9 ? [h / 1e9, "G"] : a >= 1e6 ? [h / 1e6, "M"] : a >= 1e3 ? [h / 1e3, "k"] : [h, ""]; };
  const fmtHash = (v, unit = "Sol/s") => S ? S.fmtHash(v, unit) : (v == null || !isFinite(v) ? NA : (() => { const [x, p] = scale(v); return `${trim(x)} ${p}${esc(String(unit ?? "Sol/s").replace("/s", ""))}/s`; })());
  const fmtPct = (p) => (p == null || !isFinite(p) ? NA : p === 0 ? "0%" : p < 0.01 ? "<0.01%" : p < 10 ? p.toFixed(2) + "%" : p.toFixed(1) + "%");
  const fmtInt = (v) => (v == null || !isFinite(v) ? NA : Math.round(v).toLocaleString("en-US"));
  const short = (v) => String(+(+v).toFixed(4));
  const feeRange = (p) => { const v = (p.schemes || []).map((s) => s.fee_pct).filter((x) => x != null); if (p.fee_pct != null) v.push(p.fee_pct); if (!v.length) return NA; const lo = Math.min(...v), hi = Math.max(...v); return lo === hi ? short(lo) + "%" : `${short(lo)}–${short(hi)}%`; };
  const fmtUtc = (t) => { if (!t) return NA; const d = new Date(t); return isNaN(d) ? NA : d.toISOString().replace("T", " ").slice(0, 16) + " UTC"; };
  const host = (u) => (u ? u.replace(/^https?:\/\//, "").replace(/^www\./, "").replace(/\/$/, "") : NA);
  const ago = (t) => { const s = (Date.now() - new Date(t).getTime()) / 1000; if (!isFinite(s)) return ""; if (s < 90) return "under a minute ago"; if (s < 5400) return Math.round(s / 60) + " minutes ago"; if (s < 172800) return Math.round(s / 3600) + " hours ago"; return Math.round(s / 86400) + " days ago"; };

  // ---------- relative times (exact UTC stays in the tooltip) ----------
  function relTimes(scope) { $$("time.ago[datetime]", scope).forEach((el) => { const a = ago(el.getAttribute("datetime")); if (a) { el.title = el.textContent; el.textContent = a; } }); }
  relTimes(document);

  // ---------- homepage hardware carousel ----------
  const machineCarousel = $("[data-machine-carousel]");
  if (machineCarousel) {
    const slides = $$('[data-machine-slide]', machineCarousel);
    const controls = $('[data-machine-controls]', machineCarousel);
    if (slides.length > 1 && controls) {
      const current = $('[data-machine-current]', controls);
      const toggle = $('[data-machine-toggle]', controls);
      const peekPrev = $('[data-machine-peek-prev]', machineCarousel);
      const peekNext = $('[data-machine-peek-next]', machineCarousel);
      const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)');
      let index = 0, timer = null, paused = reduceMotion.matches, hovering = false, focused = false, quickStart = true;
      controls.hidden = false;
      if (peekPrev) peekPrev.hidden = false;
      if (peekNext) peekNext.hidden = false;
      const paint = () => {
        const prevIndex = (index - 1 + slides.length) % slides.length;
        const nextIndex = (index + 1) % slides.length;
        slides.forEach((slide, i) => {
          const active = i === index;
          slide.classList.toggle('is-active', active);
          slide.classList.toggle('is-prev', i === prevIndex);
          slide.classList.toggle('is-next', i === nextIndex);
          slide.setAttribute('aria-hidden', String(!active));
          $$('a, button, input, select, textarea', slide).forEach((el) => { if (active) el.removeAttribute('tabindex'); else el.setAttribute('tabindex', '-1'); });
        });
        current.textContent = String(index + 1);
        if (peekPrev) peekPrev.setAttribute('aria-label', `Show previous machine: ${slides[prevIndex].getAttribute('aria-label') || `slide ${prevIndex + 1}`}`);
        if (peekNext) peekNext.setAttribute('aria-label', `Show next machine: ${slides[nextIndex].getAttribute('aria-label') || `slide ${nextIndex + 1}`}`);
      };
      const stop = () => { clearTimeout(timer); timer = null; };
      const schedule = () => {
        stop();
        if (!paused && !hovering && !focused && !document.hidden) {
          const delay = quickStart ? 1500 : 6500;
          timer = setTimeout(() => { quickStart = false; index = (index + 1) % slides.length; paint(); schedule(); }, delay);
        }
      };
      const go = (step) => { quickStart = false; index = (index + step + slides.length) % slides.length; paint(); schedule(); };
      $('[data-machine-prev]', controls).addEventListener('click', () => go(-1));
      $('[data-machine-next]', controls).addEventListener('click', () => go(1));
      if (peekPrev) peekPrev.addEventListener('click', () => go(-1));
      if (peekNext) peekNext.addEventListener('click', () => go(1));
      toggle.addEventListener('click', () => {
        quickStart = false;
        paused = !paused;
        toggle.textContent = paused ? '▶' : 'Ⅱ';
        toggle.setAttribute('aria-label', paused ? 'Play carousel' : 'Pause carousel');
        toggle.setAttribute('aria-pressed', String(paused));
        schedule();
      });
      machineCarousel.addEventListener('mouseenter', () => { hovering = true; stop(); });
      machineCarousel.addEventListener('mouseleave', () => { hovering = false; schedule(); });
      machineCarousel.addEventListener('focusin', () => { focused = true; stop(); });
      machineCarousel.addEventListener('focusout', (e) => { if (!machineCarousel.contains(e.relatedTarget)) { focused = false; schedule(); } });
      machineCarousel.addEventListener('keydown', (e) => { if (e.key === 'ArrowLeft' || e.key === 'ArrowRight') { e.preventDefault(); go(e.key === 'ArrowLeft' ? -1 : 1); } });
      document.addEventListener('visibilitychange', schedule);
      if (paused) { toggle.textContent = '▶'; toggle.setAttribute('aria-label', 'Play carousel'); toggle.setAttribute('aria-pressed', 'true'); }
      paint(); schedule();
    }
  }

  // ---------- coin directory: keep lower-priority networks available without crowding the scan ----------
  $$('[data-coin-more]').forEach((button) => {
    const rows = document.getElementById(button.getAttribute('aria-controls'));
    const control = button.closest('.coin-more-control');
    if (!rows || !control) return;
    control.hidden = false;
    const count = +button.dataset.count;
    button.addEventListener('click', () => {
      const open = button.getAttribute('aria-expanded') !== 'true';
      button.setAttribute('aria-expanded', String(open));
      rows.classList.toggle('is-open', open);
      button.textContent = open ? 'Hide other networks' : `Show ${count} other network${count === 1 ? '' : 's'}`;
    });
  });

  // ---------- copy buttons ----------
  $$("[data-copy]").forEach((b) => b.addEventListener("click", async () => {
    const t = $(b.dataset.copy)?.textContent || "";
    try { await navigator.clipboard.writeText(t); b.textContent = "Copied"; } catch { b.textContent = "Select and copy"; }
    setTimeout(() => (b.textContent = "Copy"), 1800);
  }));
  // "Copy link" (pool page, drawer, coin header): the absolute permanent URL, with a visible and
  // announced "Copied". Without clipboard access the link is offered in a prompt to copy by hand.
  document.addEventListener("click", async (e) => {
    const b = e.target.closest && e.target.closest(".copy-link[data-copy-url]"); if (!b) return;
    e.preventDefault();
    const url = new URL(b.dataset.copyUrl, location.origin).href;
    const ok = S ? await S.copyText(url) : false;
    const st = b.parentElement && $(".copy-status", b.parentElement);
    if (ok) { b.textContent = "Copied"; if (st) st.textContent = "Link copied to the clipboard."; }
    else { if (st) st.textContent = ""; window.prompt("Copy this link:", url); }
    b.focus({ preventScroll: true });
    clearTimeout(b._t); b._t = setTimeout(() => { b.textContent = "Copy link"; if (st) st.textContent = ""; }, 2000);
  });

  // ---------- pool table: filter + sort ----------
  const table = $("#pool-table");
  const form = $("#filters");
  const pools = json("#pool-data") || [];
  const coins = json("#coin-data") || [];
  const bySlug = Object.fromEntries(pools.map((p) => [p.slug, p]));

  // Old homepage drawer links used /#pool=<slug>. The discovery homepage no longer carries the
  // full pool payload, so take those permanent links straight to the pool page.
  if (location.pathname === "/" && location.hash.startsWith("#pool=")) {
    const slug = decodeURIComponent(location.hash.slice(6));
    if (/^[a-z0-9][a-z0-9-]{0,199}$/.test(slug)) location.replace("/pool/" + slug);
  } else if (location.pathname === "/" && location.hash === "#pools") {
    location.replace("/pools");
  }

  function readFilters() {
    const f = {};
    if (!form) return f;
    for (const el of $$("[data-filter]", form)) f[el.name] = el.type === "checkbox" ? (el.checked ? "1" : "") : el.value.trim();
    return f;
  }
  function rowMatches(tr, f) {
    const d = tr.dataset;
    if (f.scheme && !d.schemes.split(",").some((s) => s.toLowerCase() === f.scheme.toLowerCase())) return false;
    if (f.region && !d.regions.split(",").includes(f.region)) return false;
    if (f.fee !== "" && f.fee != null) { const fee = d.fee === undefined ? null : +d.fee; if (fee == null || fee > +f.fee + 1e-9) return false; }
    if (f.hr) { const h = d.hashrate === undefined ? null : +d.hashrate; if (h == null || h < +f.hr) return false; }
    if (f.merged === "1" && d.merged !== "1") return false;
    if (f.hide_empty === "1" && !(+d.hashrate > 0)) return false;
    if (f.q && !d.search.includes(f.q.toLowerCase())) return false;
    return true;
  }
  // Ranks restart in every parameter group (one tbody each): hashrates on different
  // Equihash parameters are never ranked against each other.
  function renumber() {
    $$("tbody", table).forEach((tb) => {
      let i = 0;
      $$("tr.pool-row", tb).forEach((tr) => { if (!tr.hidden) $(".rank", tr).textContent = ++i; });
      if (tb.classList.contains("pool-group")) tb.hidden = i === 0;
    });
  }
  function applyFilters(push = true) {
    if (!table) return;
    const f = readFilters();
    let n = 0;
    for (const tr of $$("tbody tr.pool-row", table)) { const ok = rowMatches(tr, f); tr.hidden = !ok; if (ok) n++; }
    renumber();
    $("#pool-count").textContent = n;
    $("#empty").hidden = n > 0;
    if (push) {
      const qs = new URLSearchParams();
      for (const [k, v] of Object.entries(f)) if (v) qs.set(k, v);
      const s = sortState(); if (s.key !== "hashrate" || s.dir !== "desc") { qs.set("sort", s.key); qs.set("dir", s.dir); }
      history.replaceState(null, "", "/pools?" + qs.toString() + location.hash);
    }
  }
  let t;
  if (form) {
    form.addEventListener("input", (e) => {
      if (e.target.name === "coin") return;
      if (e.target.matches("[data-filter]")) { clearTimeout(t); t = setTimeout(applyFilters, e.target.type === "search" ? 120 : 0); }
    });
    form.addEventListener("change", (e) => { if (e.target.name === "coin") location.href = "/pools?coin=" + encodeURIComponent(e.target.value) + "#pools"; });
    form.addEventListener("submit", (e) => { e.preventDefault(); applyFilters(); });
    $(".reset", form)?.addEventListener("click", (e) => {
      e.preventDefault();
      for (const el of $$("[data-filter]", form)) { if (el.name === "coin") continue; if (el.type === "checkbox") el.checked = false; else el.value = ""; }
      applyFilters(); countFilters();
    });
  }
  function sortState() {
    const th = table && $("th[aria-sort=ascending], th[aria-sort=descending]", table);
    return { key: th?.dataset.sort || "hashrate", dir: th?.getAttribute("aria-sort") === "ascending" ? "asc" : "desc" };
  }
  const textKeys = { name: "name", coin: "coinlabel", region: "region" };
  const numKeys = { hashrate: "hashrate", share: "share", miners: "miners", fee: "fee", minpay: "minpay", blocks: "blocks" };
  function sortBy(key, dir) {
    const val = (tr) => { if (textKeys[key]) { const v = tr.dataset[textKeys[key]]; return v ? v : null; } const v = tr.dataset[numKeys[key]]; return v === undefined ? null : +v; };
    // n/a always sorts last, in either direction; 0 is a value and sorts as one.
    const cmp = (a, b) => {
      const va = val(a), vb = val(b);
      if (va == null && vb == null) return 0; if (va == null) return 1; if (vb == null) return -1;
      const o = typeof va === "number" ? va - vb : va.localeCompare(vb);
      return dir === "asc" ? o : -o;
    };
    // Sort inside each tbody so rows never cross parameter groups.
    $$("tbody", table).forEach((tbody) => { $$("tr.pool-row", tbody).sort(cmp).forEach((r) => tbody.appendChild(r)); });
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
      if (a && !a.classList.contains("pool-link")) return;
      if (e.metaKey || e.ctrlKey || e.shiftKey || e.button === 1) return;
      if (window.getSelection && String(window.getSelection()).length > 0 && !a) return; // let people copy numbers
      e.preventDefault(); openDrawer(tr.dataset.slug);
    });
  }

  // ---------- small-screen filter toggle ----------
  const ft = $(".filters-toggle");
  function countFilters() {
    if (!ft || !form) return;
    const n = ["scheme", "fee", "region", "hr"].filter((k) => form.elements[k]?.value).length + ["merged", "hide_empty"].filter((k) => form.elements[k]?.checked).length;
    $(".ft-count", ft).textContent = n ? String(n) : "";
  }
  if (ft) {
    ft.addEventListener("click", () => { const open = form.classList.toggle("expanded"); ft.setAttribute("aria-expanded", String(open)); });
    form.addEventListener("change", countFilters); countFilters();
    if (["scheme", "fee", "region", "hr"].some((k) => form.elements[k]?.value)) { form.classList.add("expanded"); ft.setAttribute("aria-expanded", "true"); }
  }

  // ---------- pool drawer ----------
  let lastFocus = null;
  const kv = (label, value) => `<tr><th scope="row">${esc(label)}</th><td>${value}</td></tr>`;
  // Only http(s) URLs become links (the server drops anything else at load time, too).
  const safeUrl = (u) => S ? S.safeUrl(u) : (() => { try { const x = new URL(String(u).trim()); return /^https?:$/.test(x.protocol) && !!x.hostname && !x.username && !x.password; } catch { return false; } })();
  const link = (u, l) => (u ? (safeUrl(u) ? `<a href="${esc(u)}" target="_blank" rel="noopener noreferrer nofollow">${esc(l || host(u))}</a>` : esc(l || NA)) : NA);
  // Per-field provenance: where a figure came from and when that figure (not the row) was observed.
  const prov = (src, at) => (src || at ? `<br><span class="prov">${src ? link(src, host(src).split("/")[0]) : ""}${src && at ? " · " : ""}${at ? `<time class="ago" datetime="${esc(at)}">${esc(fmtUtc(at))}</time>` : ""}</span>` : "");
  const KIND = { website: "Website", explorer: "Explorer", docs: "Docs", github: "GitHub", forum: "Forum", x: "X", discord: "Discord", telegram: "Telegram", reddit: "Reddit", status: "Status", support: "Support" };
  const ICONS = "/static/icons.svg?v=3";
  // Links arrive already filtered server-side (status=verified, known kind, http(s) only).
  const oneLink = (l, withHost) => `<a href="${esc(l.url)}" target="_blank" rel="noopener noreferrer" title="${esc((l.label || KIND[l.kind]) + " · " + host(l.url).split("/")[0])}" data-kind="${esc(l.kind)}"><svg class="ic" aria-hidden="true" focusable="false"><use href="${ICONS}#i-${esc(l.kind)}"></use></svg><span>${esc(KIND[l.kind])}${withHost ? " · " + esc(host(l.url).split("/")[0]) : ""}</span></a>`;
  // Same markup as src/views/links.rs: one per kind, the rest behind "+N more".
  const links = (ls, aria) => {
    const ok = (ls || []).filter((l) => KIND[l.kind] && /^https?:\/\/\S+$/.test(l.url || ""));
    if (!ok.length) return "";
    const seen = new Set(), first = [], more = [];
    for (const l of ok) { if (seen.has(l.kind)) more.push(l); else { seen.add(l.kind); first.push(l); } }
    return `<ul class="links pool-links" aria-label="${esc(aria)}">${first.map((l) => `<li>${oneLink(l, false)}</li>`).join("")}${more.length ? `<li class="more"><details><summary>+${more.length} more</summary><ul class="links-more">${more.map((l) => `<li>${oneLink(l, true)}</li>`).join("")}</ul></details></li>` : ""}</ul>`;
  };
  const basisNote = (p) => (p.basis === "operator_reported" || p.hashrate_is_reported ? ' <span class="na">(reported by the operator, not independently measured)</span>'
    : p.basis === "pool_api" ? ` <span class="na">(from the pool's public hashrate API${p.hashrate_window_s ? `, accepted work over the previous ${Math.round(p.hashrate_window_s / 60)} minutes` : ""})</span>` : "");
  // Pool logo, same markup as src/views/logo.rs (At::Head); only local /static/logos/ files.
  const logoChip = (l, name) => {
    if (!l) return "";
    if (l.src && /^\/static\/logos\/(coins|pools)\/[a-z0-9._-]+\.(svg|webp|png)$/.test(l.src)) {
      const px = l.tile ? 38 : 30;
      return `<span class="logo lg-head${l.tile ? " tile" : ""}${l.light ? " light" : ""}" data-kind="${esc(l.kind)}"><img src="${esc(l.src)}" alt="${esc(name)} logo" width="${px}" height="${px}" decoding="async"></span>`;
    }
    return `<span class="logo lg-head mono" data-kind="fallback" role="img" aria-label="${esc(name)} (no logo published)" title="No logo found; monogram">${esc(l.mono || "")}</span>`;
  };
  function openDrawer(slug) {
    const p = bySlug[slug]; if (!p) return;
    const c = coins.find((x) => x.id === p.coin_id) || {};
    const unit = p.hashrate_unit || "Sol/s";
    const byPools = p.share_basis === "pools";
    const per = (p.schemes || []).filter((s) => s.fee_pct != null).map((s) => `${esc(s.scheme)} ${short(s.fee_pct)}%`).join(", ");
    const root = $("#drawer-root");
    root.innerHTML = `
      <div class="drawer-backdrop" data-close></div>
      <aside class="drawer" role="dialog" aria-modal="true" aria-labelledby="drawer-title">
        <div class="drawer-head">
          <div class="grab" aria-hidden="true"></div>
          <button class="drawer-close" data-close aria-label="Close">Close</button>
          <div class="sub">${esc(p.coin_label)} <span class="sym">${esc(p.coin)}</span>, Equihash ${esc(c.params || NA)}</div>
          <h3 id="drawer-title">${logoChip(p.logo, p.name)}${esc(p.name)}</h3>
          <div class="sub">${link(p.url)}</div>
          ${links(p.links, p.name + " links")}
        </div>
        <div class="drawer-body">
          ${p.share_flag ? `<p class="alert"><strong>Concentration. </strong>This pool has ${fmtPct(p.network_share_pct)} of ${esc(p.coin_label)} ${byPools ? "pool-reported hashrate" : "network hashrate"}.</p>` : ""}
          <table class="kv"><tbody>
            ${kv("Hashrate", fmtHash(p.hashrate, unit) + basisNote(p) + prov(p.hashrate_source, p.hashrate_observed_at))}
            ${p.share_note ? kv("Share of network", `${p.share_capped ? "100% (capped)" : "only listed pool"}<br><span class="na">${esc(p.share_note)}</span>`)
              : kv(byPools ? "Share of pool-reported hashrate" : "Share of network", `<span class="${p.share_flag ? "red" : ""}">${fmtPct(p.network_share_pct)}</span>`)}
            ${kv("Miners", fmtInt(p.miners) + prov(p.miners_source, p.miners_observed_at))}
            ${kv("Workers", fmtInt(p.workers))}
            ${kv("Fee", feeRange(p) + prov(p.fee_source, p.fee_observed_at))}
            ${kv("Payout", (p.payout_schemes || []).length ? esc(p.payout_schemes.join(", ")) + (per ? `<br><span class="na">${per}</span>` : "") : NA)}
            ${kv("Minimum payout", p.min_payout != null ? short(p.min_payout) + " " + esc(p.min_payout_unit || "") + prov(p.min_payout_source, p.min_payout_observed_at) : NA)}
            ${kv("Blocks in last 1,000", fmtInt(p.blocks_last_1000) + (p.blocks_last_1000 != null ? prov(p.blocks_source, p.blocks_observed_at) : ""))}
            ${kv("Last block", p.last_block_height != null ? fmtInt(p.last_block_height) + ", " + fmtUtc(p.last_block_time) : NA)}
            ${kv("Region", esc(p.region || NA))}
            ${kv("Merged mining", p.merged_mining?.supported ? esc(p.merged_mining.coins.join(", ")) + (p.merged_mining.note ? `<br><span class="na">${esc(p.merged_mining.note)}</span>` : "") : "None reported")}
            ${kv("Source", link(p.source_url, p.source_name))}
            ${kv("Raw data", link(p.data_url))}
            ${kv(p.from_miningpoolstats ? "Row fetched" : "Row checked by hand", p.fetched_at ? `<time class="ago" datetime="${esc(p.fetched_at)}">${esc(fmtUtc(p.fetched_at))}</time>` : NA)}
          </tbody></table>
          ${p.notes ? `<p class="small">${esc(p.notes)}</p>` : ""}
          <p class="perma"><a href="/pool/${esc(p.slug)}">Open pool page</a><span class="copy-link-wrap"><button type="button" class="copy-link" data-copy-url="/pool/${esc(p.slug)}" aria-label="Copy link to ${esc(p.name)}">Copy link</button><span class="copy-status" role="status" aria-live="polite"></span></span></p>
        </div>
      </aside>`;
    lastFocus = document.activeElement;
    document.body.classList.add("drawer-lock");
    requestAnimationFrame(() => requestAnimationFrame(() => { root.classList.add("drawer-open"); $(".drawer-close", root).focus(); relTimes(root); }));
    root.querySelectorAll("[data-close]").forEach((el) => el.addEventListener("click", closeDrawer));
    // Swipe down on the sheet header to close (small screens).
    const sheet = $(".drawer", root), head = $(".drawer-head", root);
    let y0 = null, dy = 0;
    head.addEventListener("touchstart", (e) => { if (matchMedia("(max-width: 680px)").matches) { y0 = e.touches[0].clientY; dy = 0; sheet.style.transition = "none"; } }, { passive: true });
    head.addEventListener("touchmove", (e) => { if (y0 === null) return; dy = Math.max(0, e.touches[0].clientY - y0); sheet.style.transform = `translateY(${dy}px)`; }, { passive: true });
    head.addEventListener("touchend", () => { if (y0 === null) return; sheet.style.transition = ""; sheet.style.transform = ""; y0 = null; if (dy > 90) closeDrawer(); });
    if (location.hash !== "#pool=" + slug) history.replaceState(null, "", location.pathname + location.search + "#pool=" + slug);
  }
  function closeDrawer() {
    const root = $("#drawer-root");
    if (!root.classList.contains("drawer-open")) return;
    root.classList.remove("drawer-open");
    document.body.classList.remove("drawer-lock");
    setTimeout(() => { if (!root.classList.contains("drawer-open")) root.innerHTML = ""; }, 250);
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
  if (m && bySlug[m[1]]) setTimeout(() => openDrawer(m[1]), 100);

  // ---------- calculator ----------
  // Bounds come from the input's min/max/data-min-exclusive attributes, which the server fills from
  // the same table it validates against (src/views/calc.rs FIELDS).
  const calc = $("#calc");
  if (calc) {
    const cdata = json("#calc-data") || [];
    const input = (n) => $("#c-" + n);
    function check(n) {
      const el = input(n); if (!el) return { ok: true, v: null };
      const raw = el.value.trim();
      let msg = "";
      let v = null;
      if (raw !== "") {
        v = Number(raw);
        const min = +el.min, max = +el.max, excl = el.dataset.minExclusive === "1", unit = el.dataset.unit || "";
        if (!isFinite(v)) msg = "Enter a number.";
        else if (n === "fee" && (v < 0 || v > 100)) msg = "Pool fee must be between 0% and 100%.";
        else if (v < 0 && min >= 0) msg = "Can't be negative.";
        else if (excl && v <= min) msg = `Must be more than ${min}.`;
        else if (v < min) msg = `Must be at least ${min} ${unit}.`;
        else if (v > max) msg = `Must be ${max >= 1e6 ? max.toExponential().replace("e+", "×10^") : max} ${unit} or less.`;
      }
      const row = el.closest(".row"), err = $("#e-" + n);
      row?.classList.toggle("invalid", !!msg);
      if (msg) el.setAttribute("aria-invalid", "true"); else el.removeAttribute("aria-invalid");
      if (err) { err.textContent = msg; err.hidden = !msg; }
      return { ok: !msg, v: msg ? null : v };
    }
    const money = (v) => (v == null ? NA : (v <= -0.005 ? "−$" : "$") + Math.abs(v).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 }));
    const coinsFmt = (v) => (v == null ? NA : v >= 100 ? v.toFixed(2) : v >= 1 ? v.toFixed(4) : v.toFixed(6));
    const set = (id, txt) => { const el = $("#" + id); if (el) el.textContent = txt; };
    function compute() {
      const names = ["hashrate", "watts", "power", "fee", "price", "reward", "nethash", "blocktime"];
      const r = Object.fromEntries(names.map((n) => [n, check(n)]));
      const bad = names.some((n) => !r[n].ok);
      $("#o-bad").hidden = !bad;
      const c = cdata.find((x) => x.id === $("#c-coin").value) || {};
      $$(".sym", $("#calc-out")).forEach((s) => (s.textContent = c.symbol || ""));
      set("o-name", c.label || "");
      $("#o-note").hidden = !!c.z15;
      if (!c.z15) $("#o-note").textContent = `${c.label} uses Equihash ${c.params}. The Antminer presets are 200,9 machines and won't mine it.`;
      const hr = r.hashrate.v, w = r.watts.v || 0, pw = r.power.v || 0, fee = r.fee.v || 0, price = r.price.v, rw = r.reward.v, net = r.nethash.v, bt = r.blocktime.v;
      // Same rule as the server: at 10% or more of the network estimate, no share-based figure.
      const e = !bad && S ? S.estimate({ hr, w, pw, fee, price, rw, net, bt }) : { coinsDay: null, powDay: null, rev: null, profit: null, sharePct: null, guard: null };
      const coinsDay = e.coinsDay, powDay = e.powDay, rev = e.rev, profit = e.profit;
      const g = $("#o-guard"); if (g) { g.textContent = e.guard || ""; g.hidden = !e.guard; }
      const x30 = (v) => (v == null ? null : v * 30);
      set("o-coins", coinsFmt(coinsDay)); set("o-coins30", coinsFmt(x30(coinsDay)));
      set("o-rev", money(rev)); set("o-rev30", money(x30(rev)));
      set("o-pow", money(powDay == null ? null : -powDay)); set("o-pow30", money(powDay == null ? null : -powDay * 30));
      set("o-profit", money(profit)); set("o-profit30", money(x30(profit)));
      $("#o-profit").classList.toggle("neg", profit != null && profit < 0); $("#o-profit30").classList.toggle("neg", profit != null && profit < 0);
      set("o-share", e.sharePct != null ? fmtPct(e.sharePct) : NA);
      set("o-be", rev != null && w > 0 ? "$" + (rev / ((w / 1000) * 24)).toFixed(3) + "/kWh" : NA);
    }
    function setHint(n, html) { const h = $("#h-" + n); if (h) h.innerHTML = html; }
    function prefill() {
      const c = cdata.find((x) => x.id === $("#c-coin").value); if (!c) return;
      const put = (n, v) => { input(n).value = v == null ? "" : +(+v).toPrecision(10); };
      put("price", c.price); put("reward", c.reward); put("nethash", c.nethash); put("blocktime", c.blocktime);
      const ru = input("reward")?.parentElement.querySelector(".unit"); if (ru) ru.textContent = c.symbol;
      setHint("price", c.price != null ? "miningpoolstats price feed" : "No sourced price for this coin. Enter one.");
      setHint("reward", c.reward != null ? `Miner share of the block reward, from ${link(c.reward_source, host(c.reward_source).split("/")[0])}.${c.reward_note ? ` <span class="reward-note" id="o-reward-note">${esc(c.reward_note)}</span>` : ""}` : "Not sourced for this coin. Enter it from the coin's explorer.");
      setHint("nethash", c.nethash == null ? "Not published. Enter it." : safeUrl(c.nethash_source) ? `Network estimate from ${link(c.nethash_source, host(c.nethash_source).split("/")[0])}${c.nethash_blocks ? `, over the previous ${+c.nethash_blocks} blocks` : ""}` : "Network estimate");
      if (!c.z15) { input("hashrate").value = ""; input("watts").value = ""; }
      const qs = new URLSearchParams(location.search); qs.set("coin", c.id); history.replaceState(null, "", "/calculator?" + qs.toString());
      compute();
    }
    $("#c-coin").addEventListener("change", prefill);
    calc.addEventListener("input", (e) => { if (e.target.id !== "c-coin") { $$(".preset").forEach((b) => b.setAttribute("aria-pressed", "false")); compute(); } });
    calc.addEventListener("submit", (e) => { e.preventDefault(); compute(); });
    $$(".preset[data-hr]").forEach((b) => b.addEventListener("click", () => {
      input("hashrate").value = b.dataset.hr; input("watts").value = b.dataset.w;
      $$(".preset").forEach((x) => x.setAttribute("aria-pressed", String(x === b)));
      compute();
    }));
    compute();
  }

  // ---------- guide: checklist persistence + active contents entry ----------
  const checks = $$(".checklist input[type=checkbox]");
  if (checks.length) {
    let saved = []; try { saved = JSON.parse(localStorage.getItem("eq-mm-checklist") || "[]"); } catch {}
    const progress = () => { const n = checks.filter((x) => x.checked).length; const c = $(".ck-count"); if (c) c.textContent = `${n} of ${checks.length} done`; };
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

  // ---------- live figures (server-polled endpoints; see /api/live) ----------
  // The server renders the latest reading; this only swaps text in place every 60 s while the
  // page is open. Browsers never call the pools' endpoints themselves.
  const liveEls = () => $$("[data-live-coin], [data-live-pool], [data-share-pool], [data-share-kv], [data-split-coin], [data-rank-list]");
  // Ranking: /api/live carries the server's order (the same rank_cmp the page was rendered with)
  // and each coin's ranked figures. Lists marked data-rank-list are reordered in the DOM, so the
  // reading order and tab order follow the visual order; a polite status line announces a change.
  function rankStatus(msg) {
    let el = $("#rank-status");
    if (!el) { el = document.createElement("p"); el.id = "rank-status"; el.className = "sr-only"; el.setAttribute("role", "status"); document.body.appendChild(el); }
    el.textContent = msg;
  }
  function applyRanking(ranking) {
    if (!S || !Array.isArray(ranking) || !ranking.length) return;
    const ids = ranking.map((r) => r.id), by = new Map(ranking.map((r) => [r.id, r]));
    const before = $$("[data-rank-list]").map((l) => $$(":scope > [data-rank-id]", l).map((x) => x.dataset.rankId).join(","));
    const focused = document.activeElement;
    for (const list of $$("[data-rank-list]")) {
      const sel = list.closest("select");
      if (sel && sel === focused) continue; // never shuffle an open or focused menu; next poll catches up
      const val = sel ? sel.value : null;
      S.applyRank(list, ids);
      if (sel && sel.value !== val) sel.value = val;
    }
    if (focused && focused !== document.activeElement && document.contains(focused)) focused.focus({ preventScroll: true });
    // Figures that move with the ranking.
    for (const item of $$("[data-rank-id]")) {
      const r = by.get(item.dataset.rankId); if (!r) continue;
      const rep = $('[data-rank-f="reported"]', item); if (rep && r.reported_text) rep.textContent = r.reported_text;
      const net = $('[data-rank-f="network"]', item); if (net && r.network_text) { net.textContent = r.network_text; net.classList.toggle("na", r.network_hashrate == null); }
      if (item.matches('[data-rank-f="option"]') && r.option_text) item.textContent = r.option_text;
      const a = $("[data-rank-title]", item); if (a && r.title) a.title = r.title;
      const day = $('[data-rank-f="z15-day"]', item);
      if (day && "z15_day" in r) {
        day.textContent = "";
        if (r.z15_day != null) { day.append(r.z15_day + " "); const sym = document.createElement("span"); sym.className = "sym"; sym.textContent = r.symbol; day.append(sym); }
        else { const na = document.createElement("span"); na.className = "na"; na.title = r.z15_na_reason || ""; na.textContent = NA; day.append(na); }
      }
      const usd = $('[data-rank-f="z15-usd"]', item); if (usd && "z15_usd" in r) usd.textContent = r.z15_usd ?? NA;
    }
    const after = $$("[data-rank-list]").map((l) => $$(":scope > [data-rank-id]", l).map((x) => x.dataset.rankId).join(","));
    if (after.some((v, i) => v !== before[i])) {
      const top = ranking.filter((r) => r.group === ranking[0].group).map((r) => r.name).slice(0, 4).join(", ");
      rankStatus(`Coin ranking updated from the latest live readings. ${ranking[0].group}: ${top}…`);
    }
  }
  async function pollLive() {
    if (document.hidden || !liveEls().length) return;
    let j;
    try { const r = await fetch("/api/live", { headers: { accept: "application/json" } }); if (!r.ok) return; j = await r.json(); } catch { return; }
    for (const el of $$("[data-live-pool]")) {
      const p = j.pools?.[el.dataset.livePool]; if (!p) continue;
      const f = $('[data-f="hashrate"]', el); if (f && p.hashrate_text) f.textContent = p.hashrate_text;
      if (p.title) el.title = p.title;
    }
    // Shares: switch every share cell, split bar, pool-page share row and alert to the mode the
    // server now computes (network share, only listed pool, or share of the listed pools).
    if (S) {
      S.applyShareModes(document, j, { coin: form?.elements?.coin?.value });
      for (const row of pools) if (j.pools?.[row.id]) S.mergeSharePool(row, j.pools[row.id]);
    }
    for (const el of $$("[data-live-coin]")) {
      const c = j.coins?.[el.dataset.liveCoin]; if (!c) continue;
      const n = $('[data-f="network"]', el); if (n) { n.textContent = c.network_text; n.classList.toggle("na", c.network_hashrate == null); }
      const r = $('[data-f="reported"]', el); if (r) r.firstChild.textContent = c.reported_text;
    }
    applyRanking(j.ranking);
    for (const el of $$("[data-live-age]")) {
      const src = (j.sources || []).find((s) => s.id === el.dataset.liveAge); if (!src?.reading) continue;
      el.setAttribute("datetime", src.reading.observed_at);
      el.textContent = ago(src.reading.observed_at);
      el.classList.toggle("is-stale", !!src.stale);
    }
  }
  if (liveEls().length) { setInterval(pollLive, 60000); document.addEventListener("visibilitychange", () => { if (!document.hidden) pollLive(); }); }

  // ---------- Buy directory: filter + sort, still fully usable without JavaScript ----------
  (function buyDir() {
    const f = $("#buy-filters"), galleries = $$('[data-buy-gallery]');
    if (!f || !galleries.length) return;
    const cards = () => $$(".buy-card");
    function apply() {
      const machine = f.elements.machine?.value || "", region = f.elements.region?.value || "", sort = f.elements.sort?.value || "region";
      let count = 0;
      for (const card of cards()) {
        const regions = (card.dataset.region || "").split(",").map((x) => x.trim().toLowerCase());
        card.hidden = !!((machine && card.dataset.machine !== machine) || (region && !regions.includes("global") && !regions.includes(region.toLowerCase())));
        if (!card.hidden) count++;
      }
      for (const gallery of galleries) {
        const number = (c, key, fallback = 9) => c.dataset[key] === "" || c.dataset[key] == null ? fallback : +c.dataset[key];
        $$('.buy-card', gallery).sort((a, b) => sort === "price" ? number(a, "price", Infinity) - number(b, "price", Infinity) : sort === "stock" ? number(a, "stockRank") - number(b, "stockRank") : number(a, "regionRank") - number(b, "regionRank")).forEach((c) => gallery.appendChild(c));
      }
      for (const group of $$('[data-buy-region-group]')) group.hidden = !$$('.buy-card', group).some((c) => !c.hidden);
      for (const section of $$('.buy-machine')) section.hidden = !$$('.buy-card', section).some((c) => !c.hidden);
      const out = $("#buy-count");
      if (out) { const checked = out.textContent.split("·").slice(1).join("·").trim(); out.textContent = `${count} listing${count === 1 ? "" : "s"}${checked ? " · " + checked : ""}`; }
      const qs = new URLSearchParams(); if (machine) qs.set("machine", machine); if (region) qs.set("region", region); if (sort !== "region") qs.set("sort", sort);
      history.replaceState(null, "", qs.toString() ? "/buy?" + qs : "/buy");
    }
    f.addEventListener("change", apply); apply();
  })();
})();
