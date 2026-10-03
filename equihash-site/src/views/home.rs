use crate::data::{Coin, Data, Pool};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default, Clone)]
#[serde(default)]
pub struct Filters {
    pub coin: Option<String>,
    pub scheme: Option<String>,
    pub region: Option<String>,
    pub q: Option<String>,
    pub fee: Option<String>,
    pub hr: Option<String>,
    pub merged: Option<String>,
    pub hide_empty: Option<String>,
    pub sort: Option<String>,
    pub dir: Option<String>,
}

impl Filters {
    fn coin(&self) -> String {
        self.coin.clone().filter(|s| !s.is_empty()).unwrap_or_else(|| "zcash".into())
    }
    fn s(v: &Option<String>) -> String {
        v.clone().unwrap_or_default()
    }
}

pub const SCHEMES: &[&str] = &["SOLO", "PPLNS", "PPS", "PPS+", "FPPS", "PROP", "D-PPS"];
pub const REGIONS: &[&str] = &["Global", "North America", "Europe", "Asia", "Russia", "South America", "Middle East"];

/// Server-side filter, mirrored exactly by static/app.js for instant filtering.
pub fn matches(p: &Pool, f: &Filters) -> bool {
    let coin = f.coin();
    if coin != "all" && p.coin_id != coin {
        return false;
    }
    if let Some(s) = f.scheme.as_deref().filter(|s| !s.is_empty()) {
        if !p.payout_schemes.iter().any(|x| x.eq_ignore_ascii_case(s)) {
            return false;
        }
    }
    if let Some(r) = f.region.as_deref().filter(|s| !s.is_empty()) {
        if !p.region_tags.iter().any(|x| x == r) {
            return false;
        }
    }
    if let Some(max) = f.fee.as_deref().and_then(|s| s.parse::<f64>().ok()) {
        match p.min_fee() {
            Some(fee) if fee <= max + 1e-9 => {}
            _ => return false,
        }
    }
    if let Some(min) = f.hr.as_deref().and_then(|s| s.parse::<f64>().ok()) {
        match p.hashrate {
            Some(h) if h >= min => {}
            _ => return false,
        }
    }
    if f.merged.as_deref() == Some("1") && !p.merged() {
        return false;
    }
    if f.hide_empty.as_deref() == Some("1") && !p.hashrate.map(|h| h > 0.0).unwrap_or(false) {
        return false;
    }
    if let Some(q) = f.q.as_deref().map(|q| q.trim().to_lowercase()).filter(|q| !q.is_empty()) {
        let hay = format!("{} {} {} {} {}", p.name, p.coin, p.coin_label, p.url.clone().unwrap_or_default(), p.region.clone().unwrap_or_default()).to_lowercase();
        if !hay.contains(&q) {
            return false;
        }
    }
    true
}

fn sort_key(p: &Pool, key: &str) -> (i32, f64, String) {
    // (has value, numeric, text) — missing values always sort last.
    let n = |v: Option<f64>| match v {
        Some(x) => (1, x, String::new()),
        None => (0, 0.0, String::new()),
    };
    match key {
        "name" => (1, 0.0, p.name.to_lowercase()),
        "coin" => (1, 0.0, p.coin_label.to_lowercase()),
        "share" => n(p.network_share_pct),
        "miners" => n(p.miners.or(p.workers)),
        "fee" => n(p.min_fee()),
        "minpay" => n(p.min_payout),
        "blocks" => n(p.blocks_last_1000),
        "region" => (if p.region.is_some() { 1 } else { 0 }, 0.0, p.region.clone().unwrap_or_default().to_lowercase()),
        _ => n(p.hashrate),
    }
}

pub fn sort_pools<'a>(mut v: Vec<&'a Pool>, key: &str, dir: &str) -> Vec<&'a Pool> {
    let desc = dir != "asc";
    v.sort_by(|a, b| {
        let (ha, na, ta) = sort_key(a, key);
        let (hb, nb, tb) = sort_key(b, key);
        if ha != hb {
            return hb.cmp(&ha);
        }
        let o = na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal).then(ta.cmp(&tb));
        if desc { o.reverse() } else { o }
    });
    v
}

fn coin_card(d: &Data, c: &Coin) -> Markup {
    let unit = c.network.unit.clone().unwrap_or("Sol/s".into());
    let top = d
        .pools
        .iter()
        .filter(|p| p.coin_id == c.id && p.network_share_pct.unwrap_or(0.0) > 0.0)
        .max_by(|a, b| a.network_share_pct.partial_cmp(&b.network_share_pct).unwrap());
    html! {
        a class="coin-card" href={"/?coin=" (c.id) "#pools"} {
            div class="coin-card-top" {
                span class="ticker" { (c.symbol) }
                span class="coin-name" { (c.name) }
            }
            div class="coin-badges" {
                @if c.z15_compatible == Some(true) { span class="badge badge-z15" title="Equihash 200,9: Antminer Z15-series compatible" { "200,9 · Z15" } }
                @else { span class="badge" title="Equihash parameters (n,k)" { (c.params()) } }
                span class="coin-pools" { (c.pool_count) " pools" }
            }
            div class="coin-hash" { (fmt::hashrate(c.network.hashrate, &unit)) }
            dl class="coin-stats" {
                div { dt { "Difficulty" } dd { (fmt::compact(c.network.difficulty)) } }
                div { dt { "Height" } dd { (fmt::int(c.network.height)) } }
                div title={"Target " (fmt::seconds(c.network.block_time_target_s))} { dt { "Block time" } dd { (fmt::seconds(c.network.block_time_avg_s)) } }
                div { dt { "Price" } dd { (fmt::price(c.price_usd)) } }
            }
            @if let Some(t) = top {
                @let s = t.network_share_pct.unwrap_or(0.0);
                div class={"coin-top" @if t.share_flag { " over" }} {
                    div class="coin-top-label" { "Largest pool" }
                    div class="coin-top-row" { strong { (t.name) } span class="mono" { (fmt::pct(Some(s))) } }
                    span class="meter" { span style={"width:" (format!("{:.1}", s.min(100.0))) "%"} {} }
                }
            }
        }
    }
}

fn share_chart(d: &Data, c: &Coin, visible: bool) -> Markup {
    let mut pools: Vec<&Pool> = d.pools.iter().filter(|p| p.coin_id == c.id && p.network_share_pct.map(|s| s > 0.0).unwrap_or(false)).collect();
    pools.sort_by(|a, b| b.network_share_pct.partial_cmp(&a.network_share_pct).unwrap());
    let top: Vec<&Pool> = pools.iter().take(10).cloned().collect();
    let shown: f64 = top.iter().filter_map(|p| p.network_share_pct).sum();
    let all: f64 = pools.iter().filter_map(|p| p.network_share_pct).sum();
    let rest = (all - shown).max(0.0);
    let by_network = c.share_basis == "network";
    let unknown = if by_network { (100.0 - all).max(0.0) } else { 0.0 };
    let over: Vec<&&Pool> = pools.iter().filter(|p| p.share_flag).collect();
    let max = top.first().and_then(|p| p.network_share_pct).unwrap_or(1.0).max(35.0);
    let of = if by_network { "network hashrate" } else { "hashrate reported by listed pools" };
    html! {
        div class={"chart-panel" @if !visible { " is-hidden" }} data-chart=(c.id) {
            div class="chart-meta" {
                div { span class="ticker" { (c.symbol) } " " strong { (c.label) } span class="muted" { " · Equihash " (c.params()) } }
                div class="chart-kpis" {
                    span { span class="muted" { "Network " } strong class="mono" { (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) } }
                    span { span class="muted" { "Pools reporting " } strong class="mono" { (pools.len()) } }
                    span { span class="muted" { "Top 3 " } strong class={"mono" @if pools.len() > 3 && top.iter().take(3).filter_map(|p| p.network_share_pct).sum::<f64>() > 51.0 { " red" }} { (fmt::pct(Some(top.iter().take(3).filter_map(|p| p.network_share_pct).sum()))) } }
                }
            }
            @if !over.is_empty() {
                div class="warn" role="alert" {
                    strong { "Decentralisation warning: " }
                    @for (i, p) in over.iter().enumerate() {
                        @if i > 0 { ", " }
                        (p.name) " has " (fmt::pct(p.network_share_pct))
                    }
                    " of " (c.symbol) " " (of) " (above the 30% line). Miners can help by choosing smaller pools."
                }
            }
            @if pools.is_empty() {
                p class="muted" { "No pool publishes hashrate for " (c.label) "." }
            } @else {
                div class="bars" style={"--t:" (format!("{:.3}", 30.0 / max * 100.0)) "%"} {
                    div class="bar-row axis" aria-hidden="true" { span {} span class="axis-track" { span class="axis-mark" { "30%" } } span {} }
                    @for (i, p) in top.iter().enumerate() {
                        @let s = p.network_share_pct.unwrap_or(0.0);
                        div class={"bar-row" @if p.share_flag { " over" }} data-pool=(p.slug) style={"--i:" (i)} tabindex="0" role="button" aria-label={(p.name) ", " (fmt::pct(Some(s))) ". Open details"} {
                            span class="bar-label" { span class="rank" { (i + 1) } (p.name) }
                            span class="bar-track" { span class="bar-fill" style={"width:" (format!("{:.3}", s / max * 100.0)) "%"} {} }
                            span class="bar-val" { (fmt::pct(Some(s))) }
                        }
                    }
                    @if rest > 0.0 {
                        div class="bar-row other" style={"--i:" (top.len())} {
                            span class="bar-label" { span class="rank" { "+" } "Other pools (" (pools.len() - top.len()) ")" }
                            span class="bar-track" { span class="bar-fill" style={"width:" (format!("{:.3}", rest / max * 100.0)) "%"} {} }
                            span class="bar-val" { (fmt::pct(Some(rest))) }
                        }
                    }
                    @if unknown > 0.5 {
                        div class="bar-row unknown" style={"--i:" (top.len() + 1)} {
                            span class="bar-label" { span class="rank" { "?" } "Unattributed" }
                            span class="bar-track" { span class="bar-fill" style={"width:" (format!("{:.3}", unknown.min(max) / max * 100.0)) "%"} {} }
                            span class="bar-val" { (fmt::pct(Some(unknown))) }
                        }
                    }
                }
                p class="fine" {
                    @if by_network {
                        "Share = pool hashrate ÷ network hashrate (" (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) ") as published by "
                        @if let Some(u) = &c.source_url { (ext(u, &fmt::host(Some(u)))) } @else { "the source" }
                        ". Unattributed = hashrate not reported by any listed pool (solo miners, private farms, pools hiding stats)."
                    } @else {
                        "Share = pool hashrate ÷ total hashrate reported by listed pools ("
                        (fmt::hashrate(c.share_denominator, c.network.unit.as_deref().unwrap_or("Sol/s"))) "). "
                        @if c.network.hashrate.is_some() {
                            "The published network estimate (" (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) ") is below what pools report, so it isn't used here."
                        } @else { "No network hashrate is published for this coin." }
                    }
                }
            }
        }
    }
}

fn sel(name: &str, current: &str, options: &[(String, String)], label: &str) -> Markup {
    html! {
        label class="field" {
            span { (label) }
            select name=(name) data-filter=(name) {
                @for (v, l) in options {
                    option value=(v) selected[v == current] { (l) }
                }
            }
        }
    }
}

fn th(f: &Filters, key: &str, label: &str, num: bool) -> Markup {
    let cur = f.sort.clone().unwrap_or("hashrate".into());
    let dir = f.dir.clone().unwrap_or("desc".into());
    let next = if cur == key && dir == "desc" { "asc" } else { "desc" };
    let mut qs = vec![("coin", f.coin())];
    for (k, v) in [("scheme", &f.scheme), ("region", &f.region), ("q", &f.q), ("fee", &f.fee), ("hr", &f.hr), ("merged", &f.merged), ("hide_empty", &f.hide_empty)] {
        if let Some(v) = v.as_ref().filter(|v| !v.is_empty()) {
            qs.push((k, v.clone()));
        }
    }
    qs.push(("sort", key.into()));
    qs.push(("dir", next.into()));
    let href = format!("/?{}#pools", qs.iter().map(|(k, v)| format!("{k}={}", urlenc(v))).collect::<Vec<_>>().join("&"));
    let aria = if cur == key { if dir == "asc" { "ascending" } else { "descending" } } else { "none" };
    html! {
        th class={@if num { "num" }} aria-sort=(aria) data-sort=(key) {
            a href=(href) { (label) span class="sort-ind" {} }
        }
    }
}

pub fn urlenc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => o.push(b as char),
            b' ' => o.push('+'),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

pub fn scheme_chips(p: &Pool) -> Markup {
    html! {
        @if p.schemes.is_empty() { span class="muted" { "n/a" } }
        @for s in &p.schemes {
            span class={"chip chip-" (crate::data::slugify(&s.scheme))} title=[s.fee_pct.map(|f| format!("{} fee {}%", s.scheme, fmt::num_short(f)))] { (s.scheme) }
        }
    }
}

fn pool_row(p: &Pool, hidden: bool) -> Markup {
    let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
    let share = p.network_share_pct;
    html! {
        tr class={"pool-row" @if p.share_flag { " over30" }} hidden[hidden]
            data-slug=(p.slug) data-coin=(p.coin_id)
            data-schemes=(p.payout_schemes.join(",")) data-regions=(p.region_tags.join(","))
            data-fee=[p.min_fee()] data-hashrate=[p.hashrate] data-share=[share]
            data-miners=[p.miners.or(p.workers)] data-minpay=[p.min_payout] data-blocks=[p.blocks_last_1000]
            data-merged=(if p.merged() { "1" } else { "0" })
            data-search=(format!("{} {} {} {} {}", p.name, p.coin, p.coin_label, p.url.clone().unwrap_or_default(), p.region.clone().unwrap_or_default()).to_lowercase())
            data-name=(p.name.to_lowercase()) data-coinlabel=(p.coin_label.to_lowercase()) data-region=(p.region.clone().unwrap_or_default().to_lowercase())
        {
            td class="c-name" {
                a class="pool-link" href={"/pool/" (p.slug)} { (p.name) }
                div class="sub" { (fmt::host(p.url.as_deref())) }
            }
            td class="c-coin" { span class="ticker sm" { (p.coin) } }
            td class="num c-hash" {
                (fmt::hashrate(p.hashrate, &unit))
                @if p.hashrate_is_reported == Some(true) { sup title="As reported by the pool operator; not independently measured" { "†" } }
            }
            td class="num c-share" {
                @if let Some(s) = share {
                    div class="share" { span class="share-bar" { span style={"width:" (format!("{:.2}", s.min(100.0))) "%"} {} } span { (fmt::pct(Some(s))) } }
                } @else { span class="muted" { "n/a" } }
            }
            td class="num c-miners" {
                @if p.miners.is_some() { (fmt::int(p.miners)) }
                @else if p.workers.is_some() { (fmt::int(p.workers)) span class="muted sm" title="workers (miner count not published)" { " w" } }
                @else { span class="muted" { "n/a" } }
            }
            td class="num c-fee" { (fmt::fee(p.fee_range())) }
            td class="c-scheme" { (scheme_chips(p)) }
            td class="num c-minpay" { @if let Some(m) = p.min_payout { (fmt::num_short(m)) " " span class="muted sm" { (p.min_payout_unit.clone().unwrap_or_default()) } } @else { span class="muted" { "n/a" } } }
            td class="num c-blocks" { (fmt::int(p.blocks_last_1000)) }
            td class="c-region" title=[p.region.clone()] {
                @let r = p.region.clone().unwrap_or_default();
                @if r.is_empty() { span class="muted" { "n/a" } }
                @else if r.chars().count() <= 22 { (r) }
                @else {
                    @let parts: Vec<&str> = r.split(',').map(|x| x.trim()).filter(|x| !x.is_empty()).collect();
                    (parts.first().copied().unwrap_or("")) " " span class="chip chip-more" { "+" (parts.len().saturating_sub(1)) }
                }
            }
            td class="c-mm" { @if p.merged() { span class="chip chip-mm" title=[p.merged_mining.note.clone()] { "+" (p.merged_mining.coins.join(", ")) } } @else { span class="muted" { "—" } } }
            td class="c-src" {
                @if p.from_miningpoolstats { span class="src-badge mps" title="From miningpoolstats.stream" { "MPS" } }
                @else { span class="src-badge direct" title="Verified from the pool's own page/API" { "Direct" } }
            }
        }
    }
}

pub fn render(d: &Data, f: &Filters) -> Markup {
    let coin = f.coin();
    let sort = f.sort.clone().unwrap_or("hashrate".into());
    let dir = f.dir.clone().unwrap_or("desc".into());
    let live: Vec<&Pool> = d.live_pools().collect();
    let rows = sort_pools(live.clone(), &sort, &dir);
    let visible = rows.iter().filter(|p| matches(p, f)).count();
    let active_coins: Vec<&Coin> = d.coins.iter().filter(|c| c.active()).collect();
    let hero_coins: Vec<&Coin> = active_coins.iter().filter(|c| c.network.hashrate.is_some()).take(6).cloned().collect();
    let chart_coins: Vec<&Coin> = active_coins.iter().filter(|c| d.pools.iter().any(|p| p.coin_id == c.id && p.network_share_pct.unwrap_or(0.0) > 0.0)).cloned().collect();
    let chart_default = if chart_coins.iter().any(|c| c.id == coin) { coin.clone() } else { "zcash".into() };
    // Hero alert: only shares measured against a published network hashrate, in coin order.
    let mut over30: Vec<&Pool> = Vec::new();
    for c in &active_coins {
        let mut v: Vec<&Pool> = live.iter().filter(|p| p.coin_id == c.id && p.share_flag && p.share_basis == "network").cloned().collect();
        v.sort_by(|a, b| b.network_share_pct.partial_cmp(&a.network_share_pct).unwrap());
        over30.extend(v);
    }
    let flagged_total = live.iter().filter(|p| p.share_flag).count();
    let non_mps = live.iter().filter(|p| !p.from_miningpoolstats).count();

    let mut coin_opts = vec![("all".to_string(), format!("All coins ({})", live.len()))];
    for c in &active_coins {
        coin_opts.push((c.id.clone(), format!("{} · {} ({})", c.symbol, c.label, c.pool_count)));
    }
    let mut scheme_opts = vec![(String::new(), "Any scheme".to_string())];
    scheme_opts.extend(SCHEMES.iter().map(|s| (s.to_string(), s.to_string())));
    let mut region_opts = vec![(String::new(), "Any region".to_string())];
    region_opts.extend(REGIONS.iter().map(|s| (s.to_string(), s.to_string())));
    let fee_opts: Vec<(String, String)> = vec![("", "Any fee"), ("0", "0% (free)"), ("0.5", "≤ 0.5%"), ("1", "≤ 1%"), ("2", "≤ 2%"), ("3", "≤ 3%")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let hr_opts: Vec<(String, String)> = vec![("", "Any hashrate"), ("1", "> 0 (reporting)"), ("1000000", "≥ 1 MSol/s"), ("100000000", "≥ 100 MSol/s"), ("1000000000", "≥ 1 GSol/s")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();

    // Embedded JSON for the drawer (progressive enhancement; each row also links to /pool/<slug>).
    let pool_json = serde_json::to_string(&live).unwrap_or("[]".into()).replace("</", "<\\/");
    let coins_json = serde_json::to_string(&d.coins.iter().map(|c| serde_json::json!({"id": c.id, "label": c.label, "symbol": c.symbol, "params": c.params(), "z15": c.z15_compatible, "net": c.network.hashrate, "basis": c.share_basis})).collect::<Vec<_>>()).unwrap_or("[]".into());

    let body = html! {
        section class="hero" {
            div class="wrap" {
                div class="hero-head" {
                    div {
                        p class="eyebrow" { span class="dot live" {} "Live data · updated " time class="ago" datetime=[d.last_updated.as_deref()] { (fmt::utc(d.last_updated.as_deref())) } }
                        h1 { "Every Equihash pool," br; span class="grad" { "in one neutral place." } }
                        p class="lede" {
                            (live.len()) " pools across " (active_coins.len()) " active Equihash coins, from miningpoolstats plus "
                            (non_mps) " pools verified directly. Sourced numbers only: if a pool doesn't publish it, you'll see n/a."
                        }
                        div class="hero-cta" {
                            a class="btn primary" href="#pools" { "Browse pools" }
                            a class="btn" href="/merged-mining" { "Merged-mining guide" }
                        }
                    }
                    @if !over30.is_empty() {
                        aside class="hero-alert" {
                            div class="alert-title" { "⚠ Concentration" }
                            p class="alert-sub" { "Pools above 30% of a network's hashrate" }
                            @for p in over30.iter().take(4) {
                                a class="alert-line" href={"#pool=" (p.slug)} data-open=(p.slug) {
                                    span class="ticker sm" { (p.coin) }
                                    span class="alert-name" { (p.name) }
                                    span class="meter" { span style={"width:" (format!("{:.1}", p.network_share_pct.unwrap_or(0.0).min(100.0))) "%"} {} }
                                    strong class="red mono" { (fmt::pct(p.network_share_pct)) }
                                }
                            }
                            @if flagged_total > over30.len().min(4) {
                                p class="fine" { "+" (flagged_total - over30.len().min(4)) " more on smaller coins" }
                            }
                            a class="alert-cta" href="#share" { "See hashrate distribution →" }
                        }
                    }
                }
                div class="coin-grid" {
                    @for c in &hero_coins { (coin_card(d, c)) }
                }
                p class="fine center" { "Network stats from miningpoolstats coin JSON (hashrate source shown per coin on " a href="/sources" { "sources" } "). " a href="#networks" { "All " (active_coins.len()) " coins →" } }
            }
        }

        section class="wrap section" id="share" {
            div class="section-head" {
                div { h2 { "Hashrate distribution" } p class="muted" { "Pools above 30% of a network are highlighted in red. No single pool should approach 51%." } }
                div class="tabs" role="tablist" {
                    @for c in chart_coins.iter().take(12) {
                        a role="tab" class={"tab" @if c.id == chart_default { " active" }} href={"/?coin=" (c.id) "#share"} data-tab=(c.id) aria-selected=(if c.id == chart_default { "true" } else { "false" }) { (c.symbol) @if c.label.contains('(') { span class="muted sm" { " " (c.params()) } } }
                    }
                }
            }
            div class="card" {
                @for c in &chart_coins { (share_chart(d, c, c.id == chart_default)) }
            }
        }

        section class="wrap section" id="pools" {
            div class="section-head" {
                div { h2 { "Pools" } p class="muted" { span id="pool-count" { (visible) } " of " (live.len()) " pools shown. Click a row for full details and sources." } }
            }
            form class="filters card" method="get" action="/#pools" id="filters" {
                label class="field grow" {
                    span { "Search" }
                    input type="search" name="q" value=(Filters::s(&f.q)) placeholder="Pool, coin, domain, region…" data-filter="q" autocomplete="off";
                }
                (sel("coin", &coin, &coin_opts, "Coin"))
                button type="button" class="btn filters-toggle" aria-expanded="false" aria-controls="filters" { "Filters" span class="ft-count" {} }
                (sel("scheme", &Filters::s(&f.scheme), &scheme_opts, "Payout"))
                (sel("fee", &Filters::s(&f.fee), &fee_opts, "Max fee"))
                (sel("region", &Filters::s(&f.region), &region_opts, "Region"))
                (sel("hr", &Filters::s(&f.hr), &hr_opts, "Hashrate"))
                div class="checks" {
                    label class="check" { input type="checkbox" name="merged" value="1" checked[f.merged.as_deref() == Some("1")] data-filter="merged"; span { "Merged mining" } }
                    label class="check" { input type="checkbox" name="hide_empty" value="1" checked[f.hide_empty.as_deref() == Some("1")] data-filter="hide_empty"; span { "Hide 0 / unreported hashrate" } }
                }
                input type="hidden" name="sort" value=(sort);
                input type="hidden" name="dir" value=(dir);
                div class="filter-actions" {
                    button class="btn primary nojs-only" type="submit" { "Apply" }
                    a class="btn ghost" href="/?coin=all#pools" { "Reset" }
                }
            }
            div class="table-wrap card" {
                table class="pool-table" id="pool-table" {
                    thead { tr {
                        (th(f, "name", "Pool", false))
                        (th(f, "coin", "Coin", false))
                        (th(f, "hashrate", "Hashrate", true))
                        (th(f, "share", "% network", true))
                        (th(f, "miners", "Miners", true))
                        (th(f, "fee", "Fee", true))
                        th { "Payout" }
                        (th(f, "minpay", "Min payout", true))
                        (th(f, "blocks", "Blocks /1k", true))
                        (th(f, "region", "Region", false))
                        th title="Merged-mining support" { "MM" }
                        th { "Source" }
                    } }
                    tbody {
                        @for p in &rows { (pool_row(p, !matches(p, f))) }
                    }
                }
                p class="empty" id="empty" hidden[visible > 0] { "No pools match these filters." }
            }
            p class="fine" {
                "Miners column shows miners where published, otherwise workers (w). Blocks /1k = blocks the pool found in the last 1,000 network blocks (miningpoolstats). "
                "† = hashrate as reported by the operator. MPS = miningpoolstats.stream; Direct = verified from the pool's own page/API."
            }
        }

        section class="wrap section" id="networks" {
            div class="section-head" { div { h2 { "Equihash networks" } p class="muted" { "Parameters, hardware class and network stats per coin. 200,9 coins can be mined with Antminer Z15-series ASICs." } } }
            div class="table-wrap card" {
                table class="net-table" {
                    thead { tr { th { "Coin" } th { "Equihash" } th { "Hardware" } th class="num" { "Network hashrate" } th class="num" { "Difficulty" } th class="num" { "Height" } th class="num" { "Block time (avg / target)" } th class="num" { "Pools" } th { "Source" } } }
                    tbody {
                        @for c in &d.coins {
                            tr class=@if !c.active() { "ended" } {
                                td { span class="ticker sm" { (c.symbol) } " " (c.label)
                                    @if !c.active() { " " span class="chip chip-ended" title=[c.status_note.clone()] { "PoW ended" } }
                                }
                                td { (c.params()) @if c.z15_compatible == Some(true) { " " span class="badge badge-z15 sm" { "Z15" } } }
                                td { (c.hardware.clone().unwrap_or("n/a".into())) }
                                td class="num" { (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) }
                                td class="num" { (fmt::compact(c.network.difficulty)) }
                                td class="num" { (fmt::int(c.network.height)) }
                                td class="num" { (fmt::seconds(c.network.block_time_avg_s)) " / " (fmt::seconds(c.network.block_time_target_s)) }
                                td class="num" { a href={"/?coin=" (c.id) "#pools"} { (c.pool_count) } }
                                td { @if let Some(u) = &c.source_url { (ext(u, &fmt::host(Some(u)))) } }
                            }
                        }
                    }
                }
            }
            p class="fine" { "Equihash parameters come from the miningpoolstats algorithm label (plain \"Equihash\" = 200,9, its ASIC-Equihash group). Wcash parameters come from its protocol specification. Ended coins are kept for reference; see the " a href="/archive" { "archive" } "." }
        }
        script type="application/json" id="pool-data" { (PreEscaped(pool_json)) }
        script type="application/json" id="coin-data" { (PreEscaped(coins_json)) }
    };
    layout(
        d,
        Page {
            title: "equihash.com · Equihash mining pool directory (Zcash, Komodo, Pirate, Bitcoin Gold…)",
            description: "Neutral, miner-first directory of every Equihash mining pool: hashrate share, fees, payout schemes, regions and sources for Zcash, Komodo, Pirate Chain, Bitcoin Gold and more.",
            path: "/",
            nav: "pools",
        },
        body,
    )
}
