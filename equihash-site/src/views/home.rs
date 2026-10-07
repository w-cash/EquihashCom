use crate::data::{age_secs, is_stale, Coin, Data, Pool};
use crate::fmt;
use crate::views::calc;
use crate::views::layout::{ext, layout_at, Page};
use crate::views::links;
use crate::views::logo::{self, At};
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
    pub fn coin(&self) -> String {
        self.coin
            .clone()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "zcash".into())
    }
    fn s(v: &Option<String>) -> String {
        v.clone().unwrap_or_default()
    }
}

/// Payout filter options, built from the data so every scheme a pool lists is selectable.
pub fn scheme_options(d: &Data) -> Vec<(String, String)> {
    let mut v = vec![(String::new(), "Any payout".to_string())];
    v.extend(
        d.payout_schemes()
            .into_iter()
            .map(|(s, n)| (s.clone(), format!("{s} ({n})"))),
    );
    v
}

/// Region filter options, built from the normalised region buckets present in the data.
pub fn region_options(d: &Data) -> Vec<(String, String)> {
    let mut v = vec![(String::new(), "Any region".to_string())];
    v.extend(d.regions().into_iter().map(|r| (r.clone(), r)));
    v
}

/// Server-side filter, mirrored by static/app.js for instant filtering.
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
    if let Some(q) =
        f.q.as_deref()
            .map(|q| q.trim().to_lowercase())
            .filter(|q| !q.is_empty())
    {
        if !search_text(p).contains(&q) {
            return false;
        }
    }
    true
}

fn search_text(p: &Pool) -> String {
    format!(
        "{} {} {} {} {}",
        p.name,
        p.coin,
        p.coin_label,
        p.url.clone().unwrap_or_default(),
        p.region.clone().unwrap_or_default()
    )
    .to_lowercase()
}

fn sort_key(p: &Pool, key: &str) -> (i32, f64, String) {
    // (has value, numeric, text): missing values always sort last.
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
        "region" => (
            if p.region.is_some() { 1 } else { 0 },
            0.0,
            p.region.clone().unwrap_or_default().to_lowercase(),
        ),
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
        let o = na
            .partial_cmp(&nb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(ta.cmp(&tb));
        if desc {
            o.reverse()
        } else {
            o
        }
    });
    v
}

pub fn urlenc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                o.push(b as char)
            }
            b' ' => o.push('+'),
            _ => o.push_str(&format!("%{b:02X}")),
        }
    }
    o
}

/// Payout schemes as plain text, with each scheme's own fee in the tooltip.
pub fn schemes_text(p: &Pool) -> Markup {
    html! {
        @if p.payout_schemes.is_empty() { span class="na" { "n/a" } }
        @for (i, s) in p.payout_schemes.iter().enumerate() {
            @if i > 0 { ", " }
            @let fee = p.schemes.iter().find(|x| &x.scheme == s).and_then(|x| x.fee_pct);
            span title=[fee.map(|f| format!("{s}: {}% fee", fmt::num_short(f)))] { (s) }
        }
    }
}

fn median(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

/// Fee a pool charges on one of the given schemes (scheme fee if published, else headline fee).
fn scheme_fee(p: &Pool, family: &[&str]) -> Option<f64> {
    let fees: Vec<f64> = p
        .schemes
        .iter()
        .filter(|s| family.iter().any(|f| s.scheme.eq_ignore_ascii_case(f)))
        .filter_map(|s| s.fee_pct.or(p.fee_pct))
        .collect();
    if fees.is_empty() {
        if p.payout_schemes
            .iter()
            .any(|s| family.iter().any(|f| s.eq_ignore_ascii_case(f)))
        {
            return p.fee_pct;
        }
        return None;
    }
    Some(fees.iter().cloned().fold(f64::INFINITY, f64::min))
}

const PPS_FAMILY: &[&str] = &["PPS", "PPS+", "FPPS"];
const SHARED_FAMILY: &[&str] = &["PPLNS", "PPLNT", "PPLNSBF", "PROP"];

fn th(f: &Filters, key: &str, label: &str, class: &str, title: Option<&str>) -> Markup {
    let cur = f.sort.clone().unwrap_or("hashrate".into());
    let dir = f.dir.clone().unwrap_or("desc".into());
    let text_key = matches!(key, "name" | "coin" | "region");
    let next = if cur == key {
        if dir == "desc" {
            "asc"
        } else {
            "desc"
        }
    } else if text_key {
        "asc"
    } else {
        "desc"
    };
    let mut qs = vec![("coin", f.coin())];
    for (k, v) in [
        ("scheme", &f.scheme),
        ("region", &f.region),
        ("q", &f.q),
        ("fee", &f.fee),
        ("hr", &f.hr),
        ("merged", &f.merged),
        ("hide_empty", &f.hide_empty),
    ] {
        if let Some(v) = v.as_ref().filter(|v| !v.is_empty()) {
            qs.push((k, v.clone()));
        }
    }
    qs.push(("sort", key.into()));
    qs.push(("dir", next.into()));
    let href = format!(
        "/pools?{}#pools",
        qs.iter()
            .map(|(k, v)| format!("{k}={}", urlenc(v)))
            .collect::<Vec<_>>()
            .join("&")
    );
    let aria = if cur == key {
        if dir == "asc" {
            "ascending"
        } else {
            "descending"
        }
    } else {
        "none"
    };
    html! {
        th class=(class) aria-sort=(aria) data-sort=(key) title=[title] scope="col" {
            a href=(href) { (label) }
        }
    }
}

fn sel(
    name: &str,
    current: &str,
    options: &[(String, String)],
    label: &str,
    class: &str,
) -> Markup {
    html! {
        label class={"f " (class)} {
            span { (label) }
            select name=(name) data-filter=(name) {
                @for (v, l) in options {
                    option value=(v) selected[v == current] { (l) }
                }
            }
        }
    }
}

fn region_short(r: &str) -> Markup {
    let parts: Vec<&str> = r
        .split(',')
        .map(|x| x.trim())
        .filter(|x| !x.is_empty())
        .collect();
    html! {
        @if r.chars().count() <= 20 { (r) }
        @else { (parts.first().copied().unwrap_or("")) " " span class="more" { "+" (parts.len().saturating_sub(1)) } }
    }
}

fn pool_row(p: &Pool, rank: usize, hidden: bool, show_coin: bool) -> Markup {
    let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
    let share = p.network_share_pct;
    html! {
        tr class={"pool-row" @if p.share_flag { " over" }} hidden[hidden]
            data-slug=(p.slug) data-coin=(p.coin_id) data-pool-id=(p.id)
            data-schemes=(p.payout_schemes.join(",")) data-regions=(p.region_tags.join(","))
            data-fee=[p.min_fee()] data-hashrate=[p.hashrate] data-share=[share]
            data-miners=[p.miners.or(p.workers)] data-minpay=[p.min_payout] data-blocks=[p.blocks_last_1000]
            data-merged=(if p.merged() { "1" } else { "0" })
            data-search=(search_text(p))
            data-name=(p.name.to_lowercase()) data-coinlabel=(p.coin_label.to_lowercase()) data-region=(p.region.clone().unwrap_or_default().to_lowercase())
        {
            td class="rank" { (rank) }
            td class="name" {
                a class="pool-link" href={"/pool/" (p.slug)} { (logo::chip(&p.logo, &p.name, At::Row, rank > 12)) (p.name) }
                @if p.merged() { " " span class="mm" title=[p.merged_mining.note.clone()] { "+" (p.merged_mining.coins.join(", ")) } }
                span class="host" title=(fmt::host(p.url.as_deref())) { (fmt::host(p.url.as_deref())) }
                span class="m-meta" {
                    @if !p.payout_schemes.is_empty() { (p.payout_schemes.join(", ")) }
                    @if let Some(r) = &p.region { @if !p.payout_schemes.is_empty() { " · " } (region_short(r)) }
                }
            }
            @if show_coin { td class="coin" { (p.coin) } }
            td class="num hash" title=(hashrate_title(p)) data-live-pool=[p.live_source.as_ref().map(|_| &p.id)] {
                @if p.hashrate.is_some() { span data-f="hashrate" { (fmt::hashrate(p.hashrate, &unit)) } } @else { span class="na" { "n/a" } }
                @if p.operator_reported() { span class="dag" { "†" } }
                span class="m-share" { (m_share(p)) }
            }
            td class="num share" data-share-pool=(p.id) { (share_cell(p)) }
            td class="num miners" {
                @if p.miners.is_some() { (fmt::int(p.miners)) }
                @else if p.workers.is_some() { (fmt::int(p.workers)) span class="na" title="Workers; the pool doesn't publish a miner count" { " w" } }
                @else { span class="na" { "n/a" } }
            }
            td class="num fee" { @if p.fee_range().is_some() { (fmt::fee(p.fee_range())) } @else { span class="na" { "n/a" } } }
            td class="payout" { (schemes_text(p)) }
            td class="num minpay" { @if let Some(m) = p.min_payout { (fmt::num_short(m)) } @else { span class="na" { "n/a" } } }
            td class="num blocks" { @if p.blocks_last_1000.is_some() { (fmt::int(p.blocks_last_1000)) } @else { span class="na" { "n/a" } } }
            td class="region" title=[p.region.clone()] {
                @match &p.region { Some(r) if !r.is_empty() => { (region_short(r)) } _ => { span class="na" { "n/a" } } }
            }
            td class="src" {
                @if p.from_miningpoolstats { span title="Figures from miningpoolstats.stream" { "MPS" } }
                @else { span title="Figures from the pool's own page or API" { "pool" } }
            }
        }
    }
}

/// What the Share cell shows: a bar and percentage, "only listed pool" (with the reason in the
/// tooltip) or n/a. Also sent in /api/live, so an open page switches modes without a reload.
pub fn share_cell(p: &Pool) -> Markup {
    html! {
        @if let (Some(note), false) = (&p.share_note, p.share_capped) {
            span class="share-note" title=(note) { "only listed pool" }
        } @else if let Some(s) = p.network_share_pct {
            span class="bar" aria-hidden="true" { span style={"width:" (format!("{:.2}", s.clamp(0.0, 100.0))) "%"} {} }
            (fmt::pct(Some(s)))
        } @else { span class="na" { "n/a" } }
    }
}

/// The share under the hashrate on small screens.
pub fn m_share(p: &Pool) -> String {
    if p.share_note.is_some() {
        share_text(p)
    } else {
        p.network_share_pct
            .map(|s| fmt::pct(Some(s)))
            .unwrap_or_default()
    }
}

/// Tooltip for a hashrate cell: where the figure came from and when it was observed.
/// The Share column's tooltip names the basis the shares are actually computed on.
pub fn share_th_title(cur: Option<&Coin>) -> &'static str {
    match cur.map(|c| c.share_basis.as_str()) {
        Some("network") => "Share of the network's hashrate",
        Some(_) => "Share of the hashrate the listed pools report (the network estimate reads below their sum)",
        None => "Share of the coin's network hashrate, or of the pools' reported sum where the network estimate reads below it",
    }
}

pub fn hashrate_title(p: &Pool) -> String {
    let basis = match p.basis.as_deref() {
        Some("operator_reported") => "Reported by the pool operator, not independently measured",
        Some("estimated") => "Estimated",
        Some("pool_api") => "From the pool's public hashrate API",
        _ if p.hashrate.is_none() => "Not published",
        _ => "Published by the pool",
    };
    let src = p.hashrate_source.as_deref().map(|u| {
        fmt::host(Some(u))
            .split('/')
            .next()
            .unwrap_or("")
            .to_string()
    });
    match (src, p.hashrate_observed_at.as_deref()) {
        (Some(s), Some(t)) => format!("{basis}; {s}, observed {}", fmt::utc(Some(t))),
        (Some(s), None) => format!("{basis}; {s}"),
        _ => basis.to_string(),
    }
}

/// One stacked bar: who holds the coin's hashrate. Red for any pool over 30%, hatched for hashrate
/// no listed pool reports. Every variant carries data-split-coin, so /api/live can swap it whole.
pub fn split_bar(d: &Data, c: &Coin) -> Markup {
    let mut pools: Vec<&Pool> = d
        .pools
        .iter()
        .filter(|p| p.coin_id == c.id && p.network_share_pct.map(|s| s > 0.0).unwrap_or(false))
        .collect();
    pools.sort_by(|a, b| {
        b.network_share_pct
            .partial_cmp(&a.network_share_pct)
            .unwrap()
    });
    if pools.is_empty() {
        return html! { p class="split-none" data-split-coin=(c.id) { "No pool on " (c.label) " publishes its hashrate, so there is no split to show." } };
    }
    // One pool reading above a network estimate taken over another window: show both figures, not a bar.
    if let [p] = pools.as_slice() {
        if let (Some(note), false) = (&p.share_note, p.share_capped) {
            return html! {
                figure class="split" data-split-coin=(c.id) {
                    figcaption { "Share of " (c.symbol) " network hashrate" }
                    p class="split-note solo" data-f="share-note" { (note) }
                }
            };
        }
    }
    let by_network = c.share_basis == "network";
    let all: f64 = pools.iter().filter_map(|p| p.network_share_pct).sum();
    let unknown = if by_network {
        (100.0 - all).max(0.0)
    } else {
        0.0
    };
    let named: Vec<&Pool> = pools.iter().take(3).cloned().collect();
    let rest: f64 = pools
        .iter()
        .skip(3)
        .filter_map(|p| p.network_share_pct)
        .sum();
    let unit = c.network.unit.as_deref().unwrap_or("Sol/s");
    // Pools under 2% are drawn as one segment so the bar stays readable.
    let big: Vec<&Pool> = pools
        .iter()
        .filter(|p| p.network_share_pct.unwrap_or(0.0) >= 2.0 || p.share_flag)
        .cloned()
        .collect();
    let small: f64 = pools
        .iter()
        .filter(|p| !(p.network_share_pct.unwrap_or(0.0) >= 2.0 || p.share_flag))
        .filter_map(|p| p.network_share_pct)
        .sum();
    html! {
        figure class="split" data-split-coin=(c.id) {
            figcaption {
                @if by_network { "Share of " (c.symbol) " network hashrate" }
                @else { "Share of hashrate reported by " (c.symbol) " pools" }
            }
            div class="split-bar" role="img" aria-label={"Hashrate split: " @for p in &named { (p.name) " " (fmt::pct(p.network_share_pct)) ", " } "others " (fmt::pct(Some(rest)))} {
                @for (i, p) in big.iter().enumerate() {
                    @let s = p.network_share_pct.unwrap_or(0.0);
                    span class={"seg" @if p.share_flag { " over" } @else if i % 2 == 1 { " alt" }} style={"width:" (format!("{:.3}", s.min(100.0))) "%"} title={(p.name) " " (fmt::pct(Some(s)))} {}
                }
                @if small > 0.0 { span class="seg small" style={"width:" (format!("{:.3}", small.min(100.0))) "%"} title={(pools.len() - big.len()) " smaller pools, " (fmt::pct(Some(small)))} {} }
                @if unknown > 0.3 { span class="seg unk" style={"width:" (format!("{:.3}", unknown)) "%"} title={"Not attributed to a listed pool: " (fmt::pct(Some(unknown)))} {} }
                span class="tick t30" style="left:30%" {}
                span class="tick t51" style="left:51%" {}
            }
            div class="split-scale" aria-hidden="true" { span style="left:30%" { "30%" } span style="left:51%" { "51%" } }
            p class="split-key" {
                @for p in &named {
                    span class={@if p.share_flag { "over" }} { (p.name) " " b { (fmt::pct(p.network_share_pct)) } }
                }
                @if pools.len() > 3 { span { (pools.len() - 3) " others " b { (fmt::pct(Some(rest))) } } }
                @if unknown > 0.3 { span class="unk" { "not attributed to a listed pool " b { (fmt::pct(Some(unknown))) } } }
            }
            @if !by_network {
                p class="split-note" {
                    @if c.network.hashrate.is_some() {
                        "The published network estimate (" (fmt::hashrate(c.network.hashrate, unit)) ") is below what the pools report, so shares here are of the pools' total ("
                        (fmt::hashrate(c.share_denominator, unit)) ")."
                    } @else {
                        "No network estimate is published, so shares here are of what the listed pools report (" (fmt::hashrate(c.share_denominator, unit)) ")."
                    }
                }
            }
        }
    }
}

/// "Network estimate" and "Reported by listed pools" side by side, never merged.
pub fn hashrate_pair(c: &Coin) -> Markup {
    let unit = c.network.unit.as_deref().unwrap_or("Sol/s");
    let (rep, dag) = fmt::reported(c);
    let net_title = match (
        c.network.hashrate_source.as_deref(),
        c.network.hashrate_observed_at.as_deref(),
    ) {
        (Some(s), Some(t)) => {
            let host = |u: &str| {
                fmt::host(Some(u))
                    .split('/')
                    .next()
                    .unwrap_or("")
                    .to_string()
            };
            let via = match c.network.hashrate_upstream.as_deref() {
                Some(u) if u.starts_with("http") => format!(", which cites {}", host(u)),
                Some(u) => format!(", which gives its basis as \"{u}\""),
                None => String::new(),
            };
            format!("Read from {}{via}, observed {}", host(s), fmt::utc(Some(t)))
        }
        _ => "Not published by any source we read".into(),
    };
    let live = c.network.live_source.is_some();
    html! {
        p class="coin-hr" data-live-coin=[live.then_some(&c.id)] {
            span title=(net_title) {
                "Network estimate: "
                @if c.network.hashrate.is_some() { b data-f="network" { (fmt::hashrate(c.network.hashrate, unit)) } } @else { b class="na" data-f="network" { "unavailable" } }
                @if let Some(n) = network_note(c) { " " span class="hr-note" data-f="network-note" { (n) } }
            }
            span title="Sum of the positive hashrates the listed pools publish" { "Reported by listed pools: " b data-f="reported" { (rep) @if dag { span class="dag" { "†" } } } }
        }
        @if let Some(n) = fmt::operator_note(c) { p class="footnote" { (n) } }
        @if let Some(n) = fmt::discrepancy_note(c) { p class="dq-note" { strong { "Data check. " } (n) } }
    }
}

/// Age of each live figure on this coin ("network estimate updated 40 seconds ago · pool hashrate
/// updated 1 minute ago"), and a plain note when an endpoint is down and the last good value shows.
pub fn live_line(d: &Data, c: &Coin, now: chrono::DateTime<chrono::Utc>) -> Markup {
    let mut items: Vec<(String, String, Option<String>, bool, Option<String>)> = Vec::new();
    let stale_after = d.live_stale_after_secs;
    for s in d.live.iter() {
        let (what, ts) = match s.target.as_str() {
            "network" if s.coin_id.as_deref() == Some(c.id.as_str()) => (
                "network estimate".to_string(),
                c.network.hashrate_observed_at.clone(),
            ),
            // Name the pool: on a coin with many pools, "pool hashrate" alone would read as all of them.
            "pool" => match d
                .pools
                .iter()
                .find(|p| s.pool_ids.contains(&p.id) && p.coin_id == c.id)
            {
                Some(p) => (
                    format!("{} hashrate", p.name),
                    p.hashrate_observed_at.clone(),
                ),
                None => continue,
            },
            _ => continue,
        };
        let stale = age_secs(ts.as_deref(), now)
            .map(|a| a > stale_after)
            .unwrap_or(true);
        let down = matches!(s.status.as_str(), "unavailable" | "error")
            .then(|| s.error.clone().unwrap_or_default());
        items.push((s.id.clone(), what, ts, stale, down));
    }
    if items.is_empty() {
        return html! {};
    }
    html! {
        p class="fresh live-age" {
            "Live: "
            @for (i, (id, what, ts, stale, _)) in items.iter().enumerate() {
                @if i > 0 { " · " }
                (what) " updated "
                time class=[stale.then_some("is-stale")] data-live-age=(id) datetime=[ts.as_deref()] title=(fmt::utc(ts.as_deref())) {
                    (fmt::age(age_secs(ts.as_deref(), now)).map(|a| format!("{a} ago")).unwrap_or("n/a".into()))
                }
            }
            "."
            @if items.iter().any(|x| x.4.is_some()) { " " span class="stale-tag" { "endpoint down" } " Showing the last good reading." }
            " Polled by this server every " (d.live.first().map(|s| s.poll_seconds).unwrap_or(45)) " s."
        }
    }
}

/// "(estimated from the last 120 blocks)" for network figures that say how they were averaged.
pub fn network_note(c: &Coin) -> Option<String> {
    c.network
        .hashrate_sample_blocks
        .filter(|_| c.network.hashrate.is_some())
        .map(|b| format!("(estimated from the last {b} blocks)"))
}

/// Share cell text: a percentage, or "only listed pool" when the pool reads above a network
/// estimate taken over a different window (see Pool::share_note).
pub fn share_text(p: &Pool) -> String {
    match (&p.share_note, p.share_capped) {
        (Some(_), false) => "only listed pool".into(),
        (_, true) => "100% (capped)".into(),
        _ => fmt::pct(p.network_share_pct),
    }
}

/// "Figures observed … ago", with a stale marker past the threshold.
pub fn freshness(ts: Option<&str>, now: chrono::DateTime<chrono::Utc>, what: &str) -> Markup {
    let stale = is_stale(ts, now);
    html! {
        p class={"fresh" @if stale { " is-stale" }} {
            @match fmt::age(age_secs(ts, now)) {
                Some(a) => { (what) " observed " time datetime=[ts] title=(fmt::utc(ts)) { (a) " ago" } }
                None => { (what) ": age unknown" }
            }
            @if stale { " " span class="stale-tag" { "stale" } }
        }
    }
}

/// What the coin's age line is about: automatic pool figures when there are any, else the chain.
fn freshness_label(d: &Data, c: &Coin) -> &'static str {
    if d.pools.iter().any(|p| {
        p.coin_id == c.id
            && !p.operator_reported()
            && p.live_source.is_none()
            && p.hashrate_observed_at.is_some()
    }) {
        "Pool figures"
    } else if c.network.hashrate_observed_at.is_some() {
        "Network figures"
    } else {
        "Chain height"
    }
}

fn coin_head(d: &Data, c: &Coin, now: chrono::DateTime<chrono::Utc>) -> Markup {
    let pools_on = d.pools.iter().filter(|p| p.coin_id == c.id).count();
    html! {
        div class="coin-head" {
            div class="coin-title" {
                div class="title-row" {
                    h2 { (logo::chip(&c.logo, &c.label, At::Head, false)) (c.label) " " span class="sym" { (c.symbol) } }
                    (super::pages::copy_link(&format!("/coin/{}", c.id), &format!("Copy link to {}", c.label)))
                }
                (links::render(&c.links, &format!("{} links", c.label), "coin-links"))
                (hashrate_pair(c))
                (live_line(d, c, now))
                p class="coin-facts" {
                    span { "Equihash " (c.params()) @if c.z15_compatible == Some(true) { ", Z15 can mine it" } }
                    span title=(field_title(c.network.difficulty_source.as_deref(), c.network.difficulty_observed_at.as_deref())) { "difficulty " b { (fmt::compact(c.network.difficulty)) } }
                    span title=(field_title(c.network.height_source.as_deref(), c.network.height_observed_at.as_deref())) { "height " b { (fmt::int(c.network.height)) } }
                    span title={"Target " (fmt::seconds(c.network.block_time_target_s))} { "blocks every " b { (fmt::seconds(c.network.block_time_avg_s.or(c.network.block_time_target_s))) } }
                    span title=(field_title(c.price_source.as_deref(), None)) { "price " b { (fmt::price(c.price_usd)) } }
                    span { b { (pools_on) } @if pools_on == 1 { " pool row, " } @else { " pool rows, " } b { (c.reported.positive_pools) } " with hashrate" }
                }
                // The live line covers live-fed figures. Any other automatic figures on the coin
                // (e.g. the miningpoolstats rows next to ZecWec on Zcash) keep their own age line.
                @if d.has_static_figures(c) {
                    (freshness(d.coin_observed_at(c).as_deref(), now, freshness_label(d, c)))
                }
            }
            (split_bar(d, c))
        }
    }
}

/// The concentration alert for one coin view (or all coins), in a wrapper /api/live can refill
/// when a live reading moves a pool over or under 30%, or changes what shares are measured against.
pub fn concentration_view(d: &Data, cur: Option<&Coin>) -> Markup {
    let mut flagged: Vec<&Pool> = d
        .live_pools()
        .filter(|p| cur.map(|c| p.coin_id == c.id).unwrap_or(true))
        .filter(|p| p.share_flag && (cur.is_some() || p.share_basis == "network"))
        .collect();
    flagged.sort_by(|a, b| {
        b.network_share_pct
            .partial_cmp(&a.network_share_pct)
            .unwrap()
    });
    html! { div class="conc" data-conc-view=(cur.map(|c| c.id.as_str()).unwrap_or("all")) { (concentration(cur, &flagged)) } }
}

fn concentration(c: Option<&Coin>, flagged: &[&Pool]) -> Markup {
    if flagged.is_empty() {
        return html! {};
    }
    html! {
        div class="alert" role="note" {
            @match c {
                Some(c) => {
                    p {
                        strong { "Concentration. " }
                        @for (i, p) in flagged.iter().enumerate() {
                            @if i > 0 { @if i + 1 == flagged.len() { " and " } @else { ", " } }
                            (p.name) " has " (fmt::pct(p.network_share_pct))
                        }
                        " of " (c.label) "'s " @if c.share_basis == "network" { "network hashrate" } @else { "pool-reported hashrate" } ". "
                        "Anything over 30% is flagged here; at 51% one pool could rewrite the chain on its own."
                    }
                }
                None => {
                    p {
                        strong { "Over 30% of a network: " }
                        @for (i, p) in flagged.iter().enumerate() {
                            @if i > 0 { "; " }
                            a href={"/pools?coin=" (p.coin_id)} { (p.name) " on " (p.coin) } " " (fmt::pct(p.network_share_pct))
                        }
                        "."
                    }
                }
            }
        }
    }
}

/// Sub-label for a parameter group heading.
pub fn group_hint(label: &str) -> &'static str {
    match label {
        "Equihash 200,9" => "Z15, Z11, Z9",
        "Parameters not published" => "",
        _ => "GPU",
    }
}

/// "≥440 kSol/s† · 1 pool": a coin's listed-pool total and row count, as plain text.
pub fn coin_summary(c: &Coin) -> String {
    let (rep, dag) = fmt::reported(c);
    format!(
        "{rep}{} · {} {}",
        if dag { "†" } else { "" },
        c.pool_count,
        if c.pool_count == 1 { "pool" } else { "pools" }
    )
}

/// The sidebar link's tooltip: "Wcash (WEC): 586 kSol/s · 1 pool".
pub fn coin_title(c: &Coin) -> String {
    format!("{} ({}): {}", c.label, c.symbol, coin_summary(c))
}

/// The coin selector's option text: "Wcash WEC · 586 kSol/s · 1 pool".
pub fn coin_option_text(c: &Coin) -> String {
    format!("{} {} · {}", c.name, c.symbol, coin_summary(c))
}

fn pools_text(n: usize) -> String {
    format!("{n} {}", if n == 1 { "pool" } else { "pools" })
}

fn coin_nav(d: &Data, current: &str, live_total: usize) -> Markup {
    let item = |c: &Coin| {
        let (rep, dag) = fmt::reported(c);
        html! {
            li class=[(c.pool_count == 0).then_some("none")] data-rank-id=(c.id) { a href={"/pools?coin=" (c.id)} aria-current=[(c.id == current).then_some("true")] title=(coin_title(c)) data-rank-title {
                span class="cn" { (logo::chip(&c.logo, &c.name, At::Nav, false)) (c.name) } span class="ch" { span data-rank-f="reported" { (rep) } @if dag { span class="dag" { "†" } } } span class="cc" { span class="sep" { "· " } (pools_text(c.pool_count as usize)) }
            } }
        }
    };
    let any_dag = d
        .coins
        .iter()
        .any(|c| c.active() && c.reported.operator_pools > 0);
    html! {
        nav class="coin-nav" aria-label="Coins" {
            div class="cn-head" aria-hidden="true" { span class="cn" { "Coin" } span class="ch" title="Sum of what the listed pools report" { "Hashrate" } span class="cc" { "Pools" } }
            ul { li { a href="/pools?coin=all" aria-current=[(current == "all").then_some("true")] { span class="cn" { "All coins" } span class="ch" {} span class="cc" { span class="sep" { "· " } (pools_text(live_total)) } } } }
            @for (label, coins) in d.param_groups(true) {
                h3 { (label) " " span { (group_hint(&label)) } }
                ul data-rank-list { @for c in &coins { (item(c)) } }
            }
            p class="coin-nav-foot" {
                "Hashrate is the sum of what each coin's listed pools report, ranked within each parameter set only. "
                "≥ means no network estimate is published. "
                @if any_dag { "† includes an operator-reported figure. " }
                "Coins whose proof of work ended are in the " a href="/archive" { "archive" } "."
            }
        }
    }
}

fn how_to_pick(d: &Data, c: &Coin) -> Markup {
    let pools: Vec<&Pool> = d.pools.iter().filter(|p| p.coin_id == c.id).collect();
    let pps: Vec<f64> = pools
        .iter()
        .filter_map(|p| scheme_fee(p, PPS_FAMILY))
        .collect();
    let shared: Vec<f64> = pools
        .iter()
        .filter_map(|p| scheme_fee(p, SHARED_FAMILY))
        .collect();
    let pro = d.miners.iter().find(|m| m.model.ends_with("Z15 Pro"));
    let z15 = c.z15_compatible == Some(true);
    let pro_hr = pro.and_then(|m| m.hashrate_ksol).filter(|_| z15);
    // Expected days between blocks for one Z15 Pro solo: 1 / (share × blocks per day).
    let solo_days = match (pro_hr, c.network.hashrate, c.network.block_time_target_s) {
        (Some(h), Some(n), Some(bt)) if n > 0.0 && bt > 0.0 && !calc::outweighs_network(c, h) => {
            Some(1.0 / ((h * 1000.0 / n) * 86400.0 / bt))
        }
        _ => None,
    };
    let per_day = pro_hr.and_then(|h| calc::coins_per_day(c, h, 1.0));
    let minpay = median(
        pools
            .iter()
            .filter_map(|p| p.min_payout)
            .filter(|m| *m > 0.0)
            .collect(),
    );
    let days = |d: f64| -> String {
        if d < 1.0 / 24.0 {
            format!("{:.0} minutes", (d * 1440.0).max(1.0))
        } else if d < 2.0 {
            format!("{:.0} hours", d * 24.0)
        } else if d < 60.0 {
            format!("{:.0} days", d)
        } else {
            format!("{:.0} months", d / 30.4)
        }
    };
    html! {
        section class="prose how" id="how" {
            h2 { "How to pick a pool" }
            p class="how-intro" { "Most of what matters is in the table above. In rough order of how much it moves your income:" }
            h3 { span class="step" { "01" } "Fee and payout scheme go together" }
            p {
                "On PPS, PPS+ and FPPS the pool pays a fixed amount for every share and carries the luck itself, so it charges more for it. "
                "On PPLNS, PPLNT and PROP you share the pool's luck: lower fees, lumpier income, and PPLNS pays less to anyone who hops in and out. "
                @if pps.len() >= 2 && shared.len() >= 2 {
                    "On " (c.label) " right now the PPS-type pools listed charge a median " b { (fmt::num_short(median(pps).unwrap())) "%" }
                    " and the shared-luck ones " b { (fmt::num_short(median(shared).unwrap())) "%" } "."
                }
            }
            h3 { span class="step" { "02" } "Solo is a lottery unless you are large" }
            p {
                "SOLO pays you the whole block, but only for blocks you find yourself. "
                @if let (Some(dd), Some(h)) = (solo_days, pro_hr) {
                    "At today's " (c.symbol) " network hashrate one Z15 Pro (" (fmt::num_short(h)) " kSol/s) would find a block about once every "
                    b { (days(dd)) } " on average, and a bad run can be several times longer. "
                } @else if let (Some(h), Some(n)) = (pro_hr.filter(|h| calc::outweighs_network(c, *h)), c.network.hashrate) {
                    "One Z15 Pro (" (fmt::num_short(h)) " kSol/s) would be " @if h * 1000.0 >= n { "more than the whole " } @else { "a large part of the " } (c.symbol) " network estimate (" (fmt::hashrate(Some(n), "Sol/s")) "), so per-machine block odds aren't meaningful here. "
                }
                "On small coins the same machine can be a real share of the network, which is where solo starts to make sense."
            }
            h3 { span class="step" { "03" } "Check the minimum payout against what you earn" }
            p {
                @if let Some(pd) = per_day {
                    "One Z15 Pro makes about " b { (fmt::num_short((pd * 10000.0).round() / 10000.0)) " " (c.symbol) } " a day on " (c.label) " at the moment"
                    @if let Some(m) = minpay {
                        @if pd > 0.0 {
                            ", so the median minimum payout here (" (fmt::num_short(m)) " " (c.symbol) ") comes round every " (days(m / pd)) "."
                        } @else { "." }
                    } @else { "." }
                    " "
                }
                "A high threshold on a small operation means coins sit on the pool for a long time. Some pools let you lower it in your account settings."
            }
            h3 { span class="step" { "04" } "Pick a stratum close to you" }
            p { "Distance costs you stale shares. Most of the large pools run servers on several continents; the Region column and the region filter use what each pool says about itself." }
            h3 { span class="step" { "05" } "Spread the hashrate" }
            p { "If one pool holds a large part of a network, that is everyone's problem. Pools over 30% are marked in red. A smaller pool on the same scheme pays the same on average; it just pays less evenly." }
            h3 { span class="step" { "06" } "Open the pool's own page before you switch" }
            p { "These numbers are a snapshot. Confirm the fee, the stratum host and port for ASICs, and that the pool is actually finding blocks. The Blocks column (blocks found in the last 1,000) is a quick check on that." }
        }
    }
}

fn z15_box(d: &Data) -> Markup {
    // d.coins is already in rank order within each parameter set.
    let coins: Vec<&Coin> = d
        .coins
        .iter()
        .filter(|c| c.active() && c.nk() == Some((200, 9)) && c.pool_count > 0)
        .collect();
    let mut other_params: Vec<String> = d
        .coins
        .iter()
        .filter(|c| c.active() && c.z15_compatible != Some(true))
        .map(|c| c.params())
        .filter(|p| p != "n/a")
        .collect();
    other_params.sort();
    other_params.dedup();
    html! {
        aside class="z15" id="z15" {
            h2 { "What can a Z15 mine?" }
            p { "Antminer Z15, Z15 Pro, Z15e, Z15j, Z11 and Z9 all run Equihash 200,9. These 200,9 coins have pools listed here:" }
            table class="mini" {
                thead { tr { th scope="col" { "Coin" } th class="num" scope="col" title="Sum of what the listed pools report" { "Reported by pools" } th class="num" scope="col" { "Pools" } } }
                tbody data-rank-list {
                    @for c in &coins {
                        @let (rep, dag) = fmt::reported(c);
                        tr data-rank-id=(c.id) {
                            td { a href={"/coin/" (c.id)} { (logo::chip(&c.logo, &c.name, At::List, true)) (c.name) } " " span class="sym" { (c.symbol) } }
                            td class="num" { span data-rank-f="reported" { (rep) } @if dag { span class="dag" { "†" } } }
                            td class="num" { (c.pool_count) }
                        }
                    }
                }
            }
            @for c in coins.iter().filter(|c| c.reported.operator_pools > 0) {
                @if let Some(n) = fmt::operator_note(c) { p class="footnote" { (c.name) ": " (n) "." } }
            }
            p class="small" {
                "The rest use other parameter sets (" (other_params.join(", ")) ") and need GPUs or different hardware. "
                a href="/hardware" { "Machine specs" } " and " a href="/calculator" { "earnings estimate" } "."
            }
        }
    }
}

fn networks(d: &Data, now: chrono::DateTime<chrono::Utc>) -> Markup {
    let cols = 11;
    html! {
        section class="networks" id="networks" {
            h2 { "All Equihash networks" }
            p class="section-sub" { "Grouped by parameter set and ranked within each group by what the listed pools report. Hashrates on different parameter sets measure different work and are never compared." }
            div class="table-scroll" {
                table class="data nets" {
                    thead { tr {
                        th scope="col" { "Coin" } th scope="col" { "Hardware" } th class="num" scope="col" { "Network estimate" }
                        th class="num" scope="col" title="Sum of what the listed pools report" { "Reported by listed pools" }
                        th class="num" scope="col" { "Difficulty" } th class="num" scope="col" { "Height" } th class="num" scope="col" { "Block time, avg / target" }
                        th class="num" scope="col" { "Price" } th class="num" scope="col" { "Pools" } th class="num" scope="col" { "As of" } th scope="col" { "Source" }
                    } }
                    @for (label, coins) in d.param_groups(false) {
                        tbody data-rank-list {
                            tr class="group-row" { th colspan=(cols) scope="rowgroup" { (label) " " span { (group_hint(&label)) } } }
                            @for c in &coins {
                                @let (rep, dag) = fmt::reported(c);
                                @let seen = c.network.hashrate_observed_at.clone().or(c.network.height_observed_at.clone()).or(c.fetched_at.clone());
                                @let dq = c.discrepancy().is_some();
                                tr class=[(!c.active()).then_some("ended")] data-rank-id=(c.id) {
                                    td { (logo::chip(&c.logo, &c.name, At::Row, true)) @if c.active() { a href={"/coin/" (c.id)} { (c.name) } } @else { (c.name) } " " span class="sym" { (c.symbol) }
                                        @if !c.active() { " " span class="tag" title=[c.status_note.clone()] { "PoW ended" } } }
                                    td { (c.hardware.clone().unwrap_or("n/a".into())) }
                                    td class="num" title=(field_title(c.network.hashrate_source.as_deref(), c.network.hashrate_observed_at.as_deref())) {
                                        @if c.network.hashrate.is_some() { span data-rank-f="network" { (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) } } @else { span class="na" data-rank-f="network" { "unavailable" } }
                                        @if dq { a class="dq-mark" href="#dq-notes" title=[fmt::discrepancy_note(c)] aria-label="Data-quality warning, see the note below the table" { "‡" } }
                                    }
                                    td class="num" title="Sum of what the listed pools report" { span data-rank-f="reported" { (rep) } @if dag { span class="dag" { "†" } } }
                                    td class="num" title=(field_title(c.network.difficulty_source.as_deref(), c.network.difficulty_observed_at.as_deref())) { (fmt::compact(c.network.difficulty)) }
                                    td class="num" title=(field_title(c.network.height_source.as_deref(), c.network.height_observed_at.as_deref())) { (fmt::int(c.network.height)) }
                                    td class="num" title=(field_title(c.network.block_time_source.as_deref(), c.network.block_time_observed_at.as_deref())) { (fmt::seconds(c.network.block_time_avg_s)) " / " (fmt::seconds(c.network.block_time_target_s)) }
                                    td class="num" title=(field_title(c.price_source.as_deref(), None)) { (fmt::price(c.price_usd)) }
                                    td class="num" { (c.pool_count) }
                                    td class={"num asof" @if is_stale(seen.as_deref(), now) { " is-stale" }} title=(fmt::utc(seen.as_deref())) { (fmt::age(age_secs(seen.as_deref(), now)).map(|a| format!("{a} ago")).unwrap_or("n/a".into())) }
                                    td class="src" { (field_sources(c)) }
                                }
                            }
                        }
                    }
                }
            }
            @for c in d.coins.iter().filter(|c| c.reported.operator_pools > 0) {
                @if let Some(n) = fmt::operator_note(c) { p class="footnote" { (c.name) ": " (n) "." } }
            }
            @let gaps: Vec<&Coin> = d.coins.iter().filter(|c| c.discrepancy().is_some()).collect();
            @if !gaps.is_empty() {
                div class="dq-notes" id="dq-notes" {
                    @for c in &gaps { p class="footnote" { "‡ " strong { (c.name) } ": " (fmt::discrepancy_note(c).unwrap_or_default()) } }
                }
            }
            p class="small" { "Source names where each figure was read: one host when it supplied them all, otherwise each host with its fields. Hover a figure for its source and time. Parameters come from the miningpoolstats algorithm label (plain \"Equihash\" is 200,9). Wcash parameters come from its protocol specification. Coins whose proof of work ended stay here for reference; see the " a href="/archive" { "archive" } "." }
        }
    }
}

fn host_of(u: &str) -> String {
    fmt::host(Some(u))
        .split('/')
        .next()
        .unwrap_or("")
        .to_string()
}

/// Tooltip naming where one figure was read and when.
pub fn field_title(src: Option<&str>, at: Option<&str>) -> String {
    match (src.filter(|u| u.starts_with("http")), at) {
        (Some(u), Some(t)) => format!("Source: {}, observed {}", host_of(u), fmt::utc(Some(t))),
        (Some(u), None) => format!("Source: {}", host_of(u)),
        (None, _) => "No source: not published by any source we read".into(),
    }
}

/// Each network-table figure that has a value, with the URL it was read from.
pub fn field_source_list(c: &Coin) -> Vec<(&'static str, String)> {
    let n = &c.network;
    let mut v: Vec<(&'static str, String)> = Vec::new();
    for (label, has, src) in [
        ("hashrate", n.hashrate.is_some(), n.hashrate_source.as_ref()),
        (
            "difficulty",
            n.difficulty.is_some(),
            n.difficulty_source.as_ref(),
        ),
        ("height", n.height.is_some(), n.height_source.as_ref()),
        (
            "block time",
            n.block_time_avg_s.is_some(),
            n.block_time_source.as_ref(),
        ),
        ("price", c.price_usd.is_some(), c.price_source.as_ref()),
    ] {
        if let (true, Some(u)) = (has, src.filter(|u| u.starts_with("http"))) {
            v.push((label, u.clone()));
        }
    }
    v
}

/// The networks table's Source cell: one host when it supplied every figure, otherwise each host
/// with the figures it supplied (e.g. Wcash: hashrate from pool.zecwec.com, height and difficulty
/// from wcashexplorer.com).
fn field_sources(c: &Coin) -> Markup {
    let mut by_host: Vec<(String, String, Vec<String>)> = Vec::new();
    for (label, u) in field_source_list(c) {
        let h = host_of(&u);
        let label = match (label, c.network.hashrate_sample_blocks) {
            ("hashrate", Some(b)) => format!("hashrate, {b}-block estimate"),
            (l, _) => l.to_string(),
        };
        match by_host.iter_mut().find(|(x, _, _)| *x == h) {
            Some(e) => e.2.push(label),
            None => by_host.push((h, u, vec![label])),
        }
    }
    html! {
        @if by_host.is_empty() {
            @if let Some(u) = &c.source_url { (ext(u, &host_of(u))) } @else { span class="na" { "n/a" } }
        } @else if by_host.len() == 1 {
            (ext(&by_host[0].1, &by_host[0].0))
        } @else {
            @for (i, (h, u, fields)) in by_host.iter().enumerate() {
                @if i > 0 { br; }
                (ext(u, h)) " " span class="src-fields" { "(" (fields.join(", ")) ")" }
            }
        }
    }
}

/// Coin selector with one optgroup per exact parameter set, in rank order.
fn coin_select(d: &Data, current: &str, live_total: usize) -> Markup {
    html! {
        label class={"f f-coin" @if d.coin(current).is_some() { " has-logo" }} {
            span { "Coin" }
            @if let Some(c) = d.coin(current) { (logo::chip(&c.logo, &c.name, At::List, false)) }
            select name="coin" data-filter="coin" {
                option value="all" selected[current == "all"] { "All coins (" (live_total) " pool rows)" }
                @for (label, coins) in d.param_groups(true) {
                    optgroup label=(label) data-rank-list {
                        @for c in &coins { option value=(c.id) selected[c.id == current] data-rank-id=(c.id) data-rank-f="option" { (coin_option_text(c)) } }
                    }
                }
            }
        }
    }
}

/// The lede's counts, computed from the data.
pub fn headline_text(d: &Data) -> String {
    let h = d.headline();
    format!(
        "{} tracked pool rows on {} Equihash coins, {} reporting positive hashrate ({} report zero, {} publish none). {} rows come from miningpoolstats; {} were checked by hand on the pool's own site.",
        fmt::group(h.rows as i128),
        h.coins_with_rows,
        h.positive,
        h.zero,
        h.unavailable,
        h.from_mps,
        h.rows - h.from_mps
    )
}

pub fn render(d: &Data, f: &Filters) -> Markup {
    render_at(d, f, chrono::Utc::now())
}

pub fn render_at(d: &Data, f: &Filters, now: chrono::DateTime<chrono::Utc>) -> Markup {
    let mut coin = f.coin();
    let live: Vec<&Pool> = d.live_pools().collect();
    let active_coins: Vec<&Coin> = d.coins.iter().filter(|c| c.active()).collect();
    if coin != "all" && !active_coins.iter().any(|c| c.id == coin) {
        coin = "all".into();
    }
    let f = &Filters {
        coin: Some(coin.clone()),
        ..f.clone()
    };
    let cur: Option<&Coin> = active_coins.iter().find(|c| c.id == coin).cloned();
    let sort = f.sort.clone().unwrap_or("hashrate".into());
    let dir = f.dir.clone().unwrap_or("desc".into());
    let in_scope: Vec<&Pool> = live
        .iter()
        .filter(|p| coin == "all" || p.coin_id == coin)
        .cloned()
        .collect();
    let rows = sort_pools(in_scope.clone(), &sort, &dir);
    let show_coin = cur.is_none();
    let help_coin: &Coin = cur.unwrap_or_else(|| {
        active_coins
            .iter()
            .find(|c| c.id == "zcash")
            .or(active_coins.first())
            .unwrap()
    });

    let scheme_opts = scheme_options(d);
    let region_opts = region_options(d);
    let fee_opts: Vec<(String, String)> = vec![
        ("", "Any fee"),
        ("0", "No fee"),
        ("0.5", "0.5% or less"),
        ("1", "1% or less"),
        ("2", "2% or less"),
        ("3", "3% or less"),
    ]
    .into_iter()
    .map(|(a, b)| (a.to_string(), b.to_string()))
    .collect();
    // Size thresholds only within one coin: across coins they would compare different parameter sets.
    let mut hr_opts: Vec<(String, String)> = vec![("", "Any size"), ("1", "Reporting hashrate")]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    if cur.is_some() {
        hr_opts.extend(
            [
                ("1000000", "1 MSol/s or more"),
                ("100000000", "100 MSol/s or more"),
                ("1000000000", "1 GSol/s or more"),
            ]
            .into_iter()
            .map(|(a, b)| (a.to_string(), b.to_string())),
        );
    }
    let hr_cur = if cur.is_none() && f.hr.as_deref().map(|v| v != "1").unwrap_or(false) {
        String::new()
    } else {
        Filters::s(&f.hr)
    };
    let f = &Filters {
        hr: Some(hr_cur.clone()).filter(|v| !v.is_empty()),
        ..f.clone()
    };
    let visible = rows.iter().filter(|p| matches(p, f)).count();
    // All-coins view: one table section per parameter set, ranked inside it.
    let groups: Vec<(String, Vec<&Pool>)> = if cur.is_some() {
        vec![(String::new(), rows.clone())]
    } else {
        d.param_groups(true)
            .into_iter()
            .map(|(label, cs)| {
                let ids: Vec<&str> = cs.iter().map(|c| c.id.as_str()).collect();
                (
                    label,
                    rows.iter()
                        .filter(|p| ids.contains(&p.coin_id.as_str()))
                        .cloned()
                        .collect::<Vec<&Pool>>(),
                )
            })
            .filter(|(_, v)| !v.is_empty())
            .collect()
    };

    // Escaped for the <script> context (see views::script_json): upstream names can't close it.
    let pool_json = super::script_json(&in_scope);
    let coins_json = super::script_json(&d.coins.iter().map(|c| serde_json::json!({"id": c.id, "label": c.label, "symbol": c.symbol, "params": c.params(), "z15": c.z15_compatible, "net": c.network.hashrate, "reported": c.reported, "basis": c.share_basis})).collect::<Vec<_>>());
    let site_ts = d.last_updated.as_deref();

    let body = html! {
        div class="wrap home" {
            header class="intro" {
                    h1 { "Equihash mining pools" }
                    p class="lede" {
                        (headline_text(d)) " "
                        @match fmt::age(age_secs(site_ts, now)) {
                            Some(a) => { "Last refreshed " time datetime=[site_ts] title=(fmt::utc(site_ts)) { (a) " ago" } "." }
                            None => { "Refresh time unknown." }
                        }
                    }
            }
            div class="home-grid" {
                (coin_nav(d, &coin, live.len()))
                section class="pools" id="pools" {
                    @match cur {
                        Some(c) => { (coin_head(d, c, now)) }
                        None => {
                            div class="coin-head" { div class="coin-title" {
                                h2 { "All coins" }
                                p class="coin-facts" { span { "Every tracked pool row, grouped by Equihash parameter set and ranked by hashrate inside each group. Pick a coin for its network figures and hashrate split." } }
                                (freshness(site_ts, now, "Pool figures"))
                            } }
                        }
                    }
                    (concentration_view(d, cur))
                    form class="filters" method="get" action="/pools#pools" id="filters" role="search" {
                        (coin_select(d, &coin, live.len()))
                        label class="f f-q" {
                            span { "Search" }
                            input type="search" name="q" value=(Filters::s(&f.q)) placeholder="Pool name, domain or region" data-filter="q" autocomplete="off";
                        }
                        button type="button" class="filters-toggle" aria-expanded="false" aria-controls="filters" { "More filters" span class="ft-count" {} }
                        (sel("scheme", &Filters::s(&f.scheme), &scheme_opts, "Payout", "f-more"))
                        (sel("fee", &Filters::s(&f.fee), &fee_opts, "Fee", "f-more"))
                        (sel("region", &Filters::s(&f.region), &region_opts, "Region", "f-more"))
                        (sel("hr", &hr_cur, &hr_opts, "Size", "f-more"))
                        div class="f-checks f-more" {
                            label { input type="checkbox" name="merged" value="1" checked[f.merged.as_deref() == Some("1")] data-filter="merged"; " Merged mining" }
                            label { input type="checkbox" name="hide_empty" value="1" checked[f.hide_empty.as_deref() == Some("1")] data-filter="hide_empty"; " Hide pools with no hashrate" }
                        }
                        input type="hidden" name="sort" value=(sort);
                        input type="hidden" name="dir" value=(dir);
                        div class="f-actions" {
                            button class="btn nojs-only" type="submit" { "Apply" }
                            a class="reset" href={"/pools?coin=" (coin) "#pools"} { "Clear" }
                        }
                    }
                    p class="count" { span id="pool-count" { (visible) } " of " (in_scope.len()) " pool rows. Click a pool for every field and its source." }
                    div class="table-scroll" {
                        table class={"pools-table" @if show_coin { " with-coin" }} id="pool-table" {
                            thead { tr {
                                th class="rank" scope="col" { "#" }
                                (th(f, "name", "Pool", "name", None))
                                @if show_coin { (th(f, "coin", "Coin", "coin", None)) }
                                (th(f, "hashrate", "Hashrate", "num hash", None))
                                (th(f, "share", "Share", "num share", Some(share_th_title(cur))))
                                (th(f, "miners", "Miners", "num miners", None))
                                (th(f, "fee", "Fee", "num fee", None))
                                th class="payout" scope="col" { "Payout" }
                                (th(f, "minpay", "Min. pay", "num minpay", Some("Minimum payout, in the coin")))
                                (th(f, "blocks", "Blocks", "num blocks", Some("Blocks found in the last 1,000 network blocks")))
                                (th(f, "region", "Region", "region", None))
                                th class="src" scope="col" title="Where the figures come from" { "Data" }
                            } }
                            @for (label, rs) in &groups {
                                @let shown = rs.iter().filter(|p| matches(p, f)).count();
                                tbody class="pool-group" hidden[shown == 0] {
                                    @if !label.is_empty() { tr class="group-row" { th colspan="12" scope="rowgroup" { (label) " " span { (group_hint(label)) } } } }
                                    @for (i, p) in rs.iter().enumerate() { (pool_row(p, i + 1, !matches(p, f), show_coin)) }
                                }
                            }
                        }
                    }
                    p class="empty" id="empty" hidden[visible > 0] { "No pool matches these filters. " a href={"/pools?coin=" (coin) "#pools"} { "Clear them" } "." }
                    div class="notes" {
                        h3 { "About these numbers" }
                        ul {
                            li { b { "Hashrate" } " is what each pool reports about itself, read from miningpoolstats or the pool's API. " b { "Share" } " is that divided by the network hashrate, which is itself an estimate from difficulty, so on small coins it jumps around." }
                            li { b { "Miners" } " is the miner count where a pool publishes one; " i { "w" } " means it only publishes workers. " b { "Blocks" } " is how many of the last 1,000 network blocks the pool found." }
                            li { b { "Min. pay" } " is in the coin itself. " b { "Data" } ": MPS means the row comes from miningpoolstats.stream; " i { "pool" } " means it was read from the pool's own site." @if in_scope.iter().any(|p| p.operator_reported()) { " † marks hashrate the operator told us, not a measured figure." } }
                            li { b { "n/a" } " means the pool doesn't publish it. Gaps are left as gaps. The full method is on the " a href="/sources" { "sources page" } "." }
                        }
                    }
                }
            }
            div class="lower" {
                (how_to_pick(d, help_coin))
                (z15_box(d))
            }
            (networks(d, now))
        }
        script type="application/json" id="pool-data" { (PreEscaped(pool_json)) }
        script type="application/json" id="coin-data" { (PreEscaped(coins_json)) }
    };
    let title = match cur {
        Some(c) => format!("{} ({}) mining pools: fees, payout and network share · equihash.com", c.label, c.symbol),
        None => "Equihash mining pools: Zcash, Komodo, Pirate Chain, Bitcoin Gold and more · equihash.com".to_string(),
    };
    let path = match cur {
        Some(c) if c.id != "zcash" => format!("/pools?coin={}", c.id),
        _ => "/pools".to_string(),
    };
    layout_at(
        d,
        Page {
            title: &title,
            description: "Every Equihash mining pool we can source, with hashrate share, fees, payout schemes, minimum payout and region for Zcash, Komodo, Pirate Chain, Bitcoin Gold and the smaller Equihash coins.",
            path: &path,
            nav: "pools",
        },
        body,
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn real() -> Data {
        crate::data::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).expect("data/ loads")
    }

    #[test]
    fn payout_filter_lists_every_scheme_in_the_data() {
        let d = real();
        let opts: Vec<String> = scheme_options(&d).into_iter().map(|(v, _)| v).collect();
        for p in d.live_pools() {
            for s in &p.payout_schemes {
                assert!(
                    opts.contains(s),
                    "scheme {s} on {} missing from the payout filter",
                    p.name
                );
            }
        }
        assert!(opts.iter().any(|o| o == "PPLNT"), "PPLNT missing");
        assert!(opts.iter().any(|o| o == "PPLNSBF"), "PPLNSBF missing");
    }

    #[test]
    fn every_scheme_option_finds_at_least_one_pool() {
        let d = real();
        for (v, _) in scheme_options(&d)
            .into_iter()
            .filter(|(v, _)| !v.is_empty())
        {
            let f = Filters {
                coin: Some("all".into()),
                scheme: Some(v.clone()),
                ..Default::default()
            };
            assert!(
                d.live_pools().any(|p| matches(p, &f)),
                "payout option {v} matches nothing"
            );
        }
    }

    #[test]
    fn every_region_option_finds_at_least_one_pool() {
        let d = real();
        for (v, _) in region_options(&d)
            .into_iter()
            .filter(|(v, _)| !v.is_empty())
        {
            let f = Filters {
                coin: Some("all".into()),
                region: Some(v.clone()),
                ..Default::default()
            };
            assert!(
                d.live_pools().any(|p| matches(p, &f)),
                "region option {v} matches nothing"
            );
        }
    }

    #[test]
    fn scheme_filter_is_case_insensitive() {
        let p = Pool {
            coin_id: "zcash".into(),
            payout_schemes: vec!["PPLNT".into()],
            ..Default::default()
        };
        let f = Filters {
            coin: Some("zcash".into()),
            scheme: Some("pplnt".into()),
            ..Default::default()
        };
        assert!(matches(&p, &f));
    }

    #[test]
    fn median_works() {
        assert_eq!(median(vec![3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(median(vec![1.0, 2.0]), Some(1.5));
        assert_eq!(median(vec![]), None);
    }

    // ---- round 3: data presentation ----

    use crate::data::{Pool, SocialLink};

    fn snap() -> Data {
        crate::data::load(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata")
                .join("snapshot-2026-10-02"),
        )
        .expect("snapshot loads")
    }
    fn at(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s)
            .unwrap()
            .with_timezone(&chrono::Utc)
    }
    /// Positions of each needle in the haystack, in order; panics when one is missing.
    fn order(h: &str, needles: &[&str]) -> Vec<usize> {
        needles
            .iter()
            .map(|n| h.find(n).unwrap_or_else(|| panic!("{n} missing")))
            .collect()
    }
    fn ascending(v: &[usize]) -> bool {
        v.windows(2).all(|w| w[0] < w[1])
    }
    const ORDER: [&str; 7] = [
        "Zcash",
        "Pirate Chain",
        "Wcash",
        "Kerrigan",
        "Komodo",
        "Buck",
        "BitMark",
    ];

    #[test]
    fn wcash_keeps_network_and_reported_apart() {
        let d = snap();
        let w = d.coin("wcash").unwrap();
        let h = hashrate_pair(w).into_string();
        assert!(h.contains("Network estimate: "), "{h}");
        assert!(h.contains("unavailable"));
        assert!(h.contains("Reported by listed pools: "));
        assert!(h.contains("≥440 kSol/s"));
        assert!(h.contains("†"));
        assert!(
            h.contains("† Operator-reported; one listed pool; verified 2 Oct 2026"),
            "{h}"
        );
        assert_eq!(coin_summary(w), "≥440 kSol/s† · 1 pool");
        // A coin with a network estimate shows both, and no ≥.
        let z = hashrate_pair(d.coin("zcash").unwrap()).into_string();
        assert!(!z.contains("unavailable") && !z.contains('≥'), "{z}");
    }

    #[test]
    fn ranking_is_the_same_in_sidebar_selector_z15_and_networks() {
        let d = snap();
        let now = at("2026-10-03T00:00:00Z");
        let names: Vec<String> = ORDER.iter().map(|n| format!(">{n}<")).collect();
        let needles: Vec<&str> = names.iter().map(String::as_str).collect();
        let nav = coin_nav(&d, "zcash", 117).into_string();
        assert!(ascending(&order(&nav, &needles)), "sidebar");
        let z15 = z15_box(&d).into_string();
        assert!(ascending(&order(&z15, &needles)), "z15 box");
        let net = networks(&d, now).into_string();
        assert!(ascending(&order(&net, &needles)), "networks table");
        let sel = coin_select(&d, "zcash", 117).into_string();
        let opts: Vec<String> = ORDER.iter().map(|n| format!(">{n} ")).collect();
        let o: Vec<&str> = opts.iter().map(String::as_str).collect();
        assert!(ascending(&order(&sel, &o)), "selector");
        // The sidebar labels its columns and spells out the pool count.
        assert!(nav.contains(">Hashrate<") && nav.contains(">Pools<"));
        assert!(nav.contains("≥440 kSol/s") && nav.contains("1 pool<"));
        // 200,9 rows come before any other parameter set's rows, which sit in their own group.
        let first_other = net.find("Equihash 144,5").unwrap();
        assert!(net.find(">BitMark<").unwrap() < first_other);
        let hw = crate::views::pages::miners(&d).into_string();
        assert!(ascending(&order(&hw, &needles)), "hardware page");
    }

    #[test]
    fn zero_and_na_render_differently() {
        let mut p = Pool {
            name: "P".into(),
            slug: "p".into(),
            hashrate: Some(0.0),
            ..Default::default()
        };
        let zero = pool_row(&p, 1, false, false).into_string();
        assert!(zero.contains("0 Sol/s"), "{zero}");
        assert!(zero.contains("data-hashrate=\"0\""));
        p.hashrate = None;
        let na = pool_row(&p, 1, false, false).into_string();
        assert!(
            !na.contains("0 Sol/s") && !na.contains("data-hashrate"),
            "{na}"
        );
        assert!(na.contains("<span class=\"na\">n/a</span>"));
        // Coin totals: n/a when nothing is published, 0 when zero is.
        let mut c = crate::data::Coin::default();
        assert_eq!(fmt::reported(&c).0, "n/a");
        c.reported.hashrate = Some(0.0);
        assert_eq!(fmt::reported(&c).0, "0 Sol/s");
        let d = snap();
        assert_eq!(
            fmt::reported(d.coin("hush").unwrap()).0,
            if d.coin("hush").unwrap().reported.hashrate.is_some() {
                "0 Sol/s"
            } else {
                "n/a"
            }
        );
    }

    #[test]
    fn headline_is_computed_not_claimed() {
        let d = snap();
        let t = headline_text(&d);
        assert!(t.starts_with("117 tracked pool rows on "), "{t}");
        assert!(t.contains("63 reporting positive hashrate"), "{t}");
        let page = render_at(&d, &Filters::default(), at("2026-10-03T00:00:00Z")).into_string();
        assert!(
            !page.contains("active pools"),
            "no '117 active pools' claim"
        );
        assert!(!page.to_lowercase().contains("every 30 minutes") && !page.contains("30 min"));
    }

    #[test]
    fn stale_notice_appears_only_past_two_hours() {
        let d = snap();
        let gen = d.meta.generated_at.clone().unwrap();
        let g = at(&gen);
        let fresh =
            render_at(&d, &Filters::default(), g + chrono::Duration::minutes(30)).into_string();
        let i = fresh
            .find("stale-tag")
            .map(|i| fresh[i.saturating_sub(400)..i + 50].to_string());
        assert!(
            !fresh.contains("stale-note") && i.is_none(),
            "fresh data shows no warning: {i:?}"
        );
        let old = render_at(&d, &Filters::default(), g + chrono::Duration::hours(5)).into_string();
        assert!(old.contains("class=\"stale-note\"") && old.contains("Stale data."));
        assert!(old.contains("stale-tag"));
    }

    #[test]
    fn coin_view_renders_only_verified_coin_links() {
        let mut d = snap();
        let mk = |kind: &str, url: &str, status: &str| SocialLink {
            kind: kind.into(),
            url: url.into(),
            label: "L".into(),
            status: status.into(),
            ..Default::default()
        };
        let raw = vec![
            mk("website", "https://z.cash/", "verified"),
            mk("x", "https://x.com/zcash", "unverified"),
            mk("discord", "https://discord.gg/x", "dead"),
        ];
        let z = d.coins.iter_mut().find(|c| c.id == "zcash").unwrap();
        z.links = crate::data::verified_links(&raw);
        let f = Filters {
            coin: Some("zcash".into()),
            ..Default::default()
        };
        let page = render_at(&d, &f, at("2026-10-03T00:00:00Z")).into_string();
        assert!(
            page.contains("href=\"https://z.cash/\" target=\"_blank\" rel=\"noopener noreferrer\""),
            "verified link with safe rel"
        );
        assert!(!page.contains("x.com/zcash") && !page.contains("discord.gg/x"));
        assert!(
            !page.contains("#i-x\"") && !page.contains("#i-discord\""),
            "no empty icons for missing kinds"
        );
    }

    #[test]
    fn live_wcash_shows_both_numbers_without_dagger_or_overflow() {
        let dir = std::env::temp_dir().join(format!("eq-live-view-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("curated")).unwrap();
        let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        for sub in ["", "curated"] {
            for e in std::fs::read_dir(real.join(sub)).unwrap().flatten() {
                if e.path().is_file() && e.file_name() != crate::data::MANIFEST {
                    std::fs::copy(e.path(), dir.join(sub).join(e.file_name())).unwrap();
                }
            }
        }
        for f in crate::data::GENERATED {
            std::fs::copy(crate::data::generated_path(&real, f), dir.join(f)).unwrap();
        }
        use crate::live::{LiveState, Parsed, Reading};
        let now = chrono::Utc::now();
        let at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let mut st = LiveState::default();
        st.record(
            "zecwec-pool",
            Ok(Parsed::Ok(Reading {
                hashrate: 553_587.0,
                observed_at: at.clone(),
                window_seconds: Some(1200),
                sample_blocks: None,
                height: None,
            })),
            now,
        );
        st.record(
            "zecwec-wcash-network",
            Ok(Parsed::Ok(Reading {
                hashrate: 507_977.0,
                observed_at: at,
                window_seconds: None,
                sample_blocks: Some(120),
                height: Some(16143),
            })),
            now,
        );
        let d = crate::data::load_with_live(&dir, Some(&st)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        let w = d.coin("wcash").unwrap();
        let pair = hashrate_pair(w).into_string();
        assert!(
            pair.contains("508 kSol/s") && pair.contains("(estimated from the last 120 blocks)"),
            "{pair}"
        );
        assert!(
            pair.contains("554 kSol/s")
                && !pair.contains('†')
                && !pair.contains('≥')
                && !pair.contains("unavailable"),
            "{pair}"
        );
        let f = Filters {
            coin: Some("wcash".into()),
            ..Default::default()
        };
        let page = render_at(&d, &f, now).into_string();
        assert!(page.contains("only listed pool"));
        assert!(
            !page.contains("109.0%") && !page.contains("109."),
            "no share above 100%"
        );
        assert!(
            page.contains("data-live-coin=\"wcash\"")
                && page.contains("data-live-pool=\"wcash:zecwec.com\"")
        );
        assert!(page.contains("Live: ") && page.contains("data-live-age=\"zecwec-wcash-network\""));
        assert!(
            !page.contains("Operator-reported; one listed pool"),
            "the † footnote is gone for Wcash"
        );
        // /api/live carries the same figures, already formatted.
        let j = crate::views::live_json(&d, now);
        assert_eq!(j["pools"]["wcash:zecwec.com"]["hashrate"], 553_587.0);
        assert_eq!(
            j["pools"]["wcash:zecwec.com"]["share_text"],
            "only listed pool"
        );
        assert_eq!(j["coins"]["wcash"]["network_text"], "508 kSol/s");
        assert_eq!(j["coins"]["wcash"]["reported_dagger"], false);
        // Hardware: no per-machine WEC figure when one Z15 Pro outweighs the network.
        let hw = crate::views::pages::miners(&d).into_string();
        assert!(hw.contains("10% or more of the network estimate"));
    }

    #[test]
    fn networks_table_names_a_source_for_every_figure() {
        let d = real();
        for c in &d.coins {
            let n = &c.network;
            let present = [
                n.hashrate.is_some(),
                n.difficulty.is_some(),
                n.height.is_some(),
                n.block_time_avg_s.is_some(),
                c.price_usd.is_some(),
            ]
            .iter()
            .filter(|x| **x)
            .count();
            assert_eq!(
                field_source_list(c).len(),
                present,
                "{}: every figure shown needs its own source",
                c.id
            );
        }
        // Wcash: hashrate from ZecWec's 120-block estimate, chain figures from the explorer.
        let w = d.coin("wcash").unwrap();
        let cell = field_sources(w).into_string();
        assert!(
            cell.contains("pool.zecwec.com") && cell.contains("hashrate, 120-block estimate"),
            "{cell}"
        );
        assert!(
            cell.contains("wcashexplorer.com") && cell.contains("difficulty, height"),
            "{cell}"
        );
        assert!(cell.find("pool.zecwec.com").unwrap() < cell.find("wcashexplorer.com").unwrap());
        assert_eq!(
            field_title(w.network.hashrate_source.as_deref(), None),
            "Source: pool.zecwec.com"
        );
        assert!(field_title(
            w.network.height_source.as_deref(),
            w.network.height_observed_at.as_deref()
        )
        .starts_with("Source: wcashexplorer.com, observed "));
        // A coin read from one file gets one link.
        let z = field_sources(d.coin("zcash").unwrap()).into_string();
        assert!(
            z.contains("data.miningpoolstats.stream") && !z.contains("src-fields"),
            "{z}"
        );
    }

    #[test]
    fn data_check_note_shows_on_coin_header_and_networks_table() {
        let mut d = real();
        let now = chrono::Utc::now();
        // Whatever the live data says, a coin made 1,000× apart is flagged in both places...
        let k = d.coins.iter_mut().find(|c| c.id == "komodo").unwrap();
        k.network.hashrate = Some(k.reported.hashrate.unwrap() * 1000.0);
        let k = d.coin("komodo").unwrap().clone();
        assert!(hashrate_pair(&k)
            .into_string()
            .contains("Data check. </strong>The network estimate"));
        let net = networks(&d, now).into_string();
        assert!(
            net.contains("id=\"dq-notes\"")
                && net.contains("<strong>Komodo</strong>")
                && net.contains("class=\"dq-mark\""),
            "networks"
        );
        // ...and a coin with ordinary figures is not.
        let z = d.coin("zcash").unwrap();
        assert!(
            z.discrepancy().is_none() && !hashrate_pair(z).into_string().contains("Data check.")
        );
    }

    #[test]
    fn price_notes_say_what_each_coin_actually_shows() {
        use crate::data::Coin;
        let mk = |id: &str, sym: &str, price: Option<f64>, src: Option<&str>| Coin {
            id: id.into(),
            label: id.into(),
            name: id.into(),
            symbol: sym.into(),
            status: "active".into(),
            price_usd: price,
            price_source: src.map(String::from),
            ..Default::default()
        };
        let mut d = Data::default();
        d.coins = vec![
            mk(
                "zcash",
                "ZEC",
                Some(1300.0),
                Some("https://data.miningpoolstats.stream/data/price/zcash.js"),
            ),
            mk(
                "kerrigan-equihash",
                "KRGN",
                Some(0.00916),
                Some("https://data.miningpoolstats.stream/data/kerrigan-equihash.js"),
            ),
            mk("bitmark-equihash", "MARKS", None, None),
            mk("buck", "BUCK", None, None),
        ];
        d.meta.errors = vec![
            "price bitmark-equihash: https://data.miningpoolstats.stream/data/price/bitmark-equihash.js?t=1: HTTP 404".into(),
            "price kerrigan-equihash: https://data.miningpoolstats.stream/data/price/kerrigan-equihash.js?t=1: HTTP 404".into(),
        ];
        let h = crate::views::pages::price_notes(&d).into_string();
        assert!(
            h.contains("1 active coin uses miningpoolstats' dedicated price endpoint"),
            "{h}"
        );
        assert!(h.contains("kerrigan-equihash (KRGN) shows $0.00916 from the fallback price field in its main miningpoolstats coin file, because its dedicated price endpoint failed at the last refresh (HTTP 404)"), "{h}");
        assert!(h.contains("No price (n/a) for bitmark-equihash (MARKS): the dedicated price endpoint failed at the last refresh (HTTP 404) and the coin file has no fallback price"), "{h}");
        assert!(h.contains("No price is published for buck (BUCK)"), "{h}");
    }
}
