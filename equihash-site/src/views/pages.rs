use crate::data::{Data, Pool};
use crate::fmt;
use crate::views::home::scheme_chips;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};

fn kv(label: &str, value: Markup) -> Markup {
    html! { div class="kv" { dt { (label) } dd { (value) } } }
}

pub fn pool_fields(d: &Data, p: &Pool) -> Markup {
    let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
    let coin = d.coin(&p.coin_id);
    html! {
        dl class="kv-grid" {
            (kv("Coin", html! { span class="ticker sm" { (p.coin) } " " (p.coin_label) @if let Some(c) = coin { " · Equihash " (c.params()) } }))
            (kv("Hashrate", html! { (fmt::hashrate(p.hashrate, &unit)) @if p.hashrate_is_reported == Some(true) { " " span class="muted" { "(as reported by operator)" } } }))
            (kv(if p.share_basis == "pools" { "% of listed-pool hashrate" } else { "% of network" }, html! { (fmt::pct(p.network_share_pct)) @if p.share_basis == "pools" { div class="muted sm" { "The published network estimate is below the pools' total, so share is measured against listed pools." } } }))
            (kv("Miners", html! { (fmt::int(p.miners)) }))
            (kv("Workers", html! { (fmt::int(p.workers)) }))
            (kv("Fee", html! { (fmt::fee(p.fee_range())) }))
            (kv("Payout schemes", html! {
                (scheme_chips(p))
                @for s in &p.schemes { @if let Some(f) = s.fee_pct { span class="muted sm" { " " (s.scheme) " " (fmt::num_short(f)) "%" } } }
            }))
            (kv("Min payout", html! { @if let Some(m) = p.min_payout { (fmt::num_short(m)) " " (p.min_payout_unit.clone().unwrap_or_default()) } @else { "n/a" } }))
            (kv("Blocks (last 1,000)", html! { (fmt::int(p.blocks_last_1000)) }))
            (kv("Last block", html! { @if p.last_block_height.is_some() { (fmt::int(p.last_block_height)) " · " (fmt::utc(p.last_block_time.as_deref())) } @else { "n/a" } }))
            (kv("Region", html! { (p.region.clone().unwrap_or("n/a".into())) }))
            (kv("Merged mining", html! { @if p.merged() { "Yes: " (p.merged_mining.coins.join(", ")) @if let Some(n) = &p.merged_mining.note { div class="muted sm" { (n) } } } @else { "None reported" } }))
            (kv("Website", html! { @if let Some(u) = &p.url { (ext(u, &fmt::host(Some(u)))) } @else { "n/a" } }))
            (kv("Source", html! { @if let Some(u) = &p.source_url { (ext(u, p.source_name.as_deref().unwrap_or(u))) } @else { "n/a" } }))
            (kv("Raw data", html! { @if let Some(u) = &p.data_url { (ext(u, &fmt::host(Some(u)))) } @else { "n/a" } }))
            (kv("On miningpoolstats", html! { @if p.from_miningpoolstats { "Yes" } @else { "No (verified directly)" } }))
            (kv("Fetched at", html! { code { (p.fetched_at.clone().unwrap_or("n/a".into())) } }))
        }
        @if let Some(n) = &p.notes { p class="note" { (n) } }
    }
}

pub fn pool_page(d: &Data, p: &Pool) -> Markup {
    let title = format!("{} {} pool", p.name, p.coin);
    let desc = format!("{} mining pool for {}: hashrate, network share, fee, payout scheme and sources.", p.name, p.coin_label);
    let path = format!("/pool/{}", p.slug);
    layout(d, Page { title: &title, description: &desc, path: &path, nav: "pools" }, html! {
        section class="wrap section narrow" {
            a class="back" href={"/?coin=" (p.coin_id) "#pools"} { "← All " (p.coin) " pools" }
            h1 class="page-title" { (p.name) " " span class="muted" { (p.coin) } }
            div class="card pad" { (pool_fields(d, p)) }
        }
    })
}

pub fn archive(d: &Data) -> Markup {
    let ended = d.ended_coin_pools();
    let ended_coins: Vec<_> = d.coins.iter().filter(|c| !c.active()).collect();
    layout(d, Page { title: "Archive: retired Equihash pools", description: "Retired Equihash mining pools and coins whose proof-of-work ended, each with a citation.", path: "/archive", nav: "archive" }, html! {
        section class="wrap section" {
            h1 class="page-title" { "Archive" }
            p class="lede" { "Pools that no longer operate, and Equihash coins whose proof-of-work has ended. An entry is only added when a source confirms it." }
            div class="archive-grid" {
                @for a in &d.archive {
                    article class="card pad archive-card" {
                        div class="archive-top" { span class="ticker sm" { (a.coin) } span class="chip chip-ended" { "Retired " (a.retired_label.clone().unwrap_or("n/a".into())) } }
                        h3 { (a.name) }
                        @if let Some(o) = &a.operator { p class="muted sm" { "Operator: " (o) } }
                        @if let Some(r) = &a.reason { p { (r) } }
                        @if let Some(n) = &a.notes { p class="muted sm" { (n) } }
                        ul class="src-list" { @for s in &a.sources { li { (ext(&s.url, &s.label)) } } }
                        p class="fine" { "Verified " (fmt::utc(a.fetched_at.as_deref())) }
                    }
                }
            }
            h2 class="mt" { "Coins whose Equihash PoW ended" }
            div class="archive-grid" {
                @for c in &ended_coins {
                    article class="card pad archive-card" {
                        div class="archive-top" { span class="ticker sm" { (c.symbol) } span class="chip chip-ended" { "PoW ended" } }
                        h3 { (c.name) " " span class="muted sm" { "Equihash " (c.params()) } }
                        @if let Some(n) = &c.status_note { p { (n) } }
                        ul class="src-list" { @for s in &c.status_sources { li { (ext(&s.url, &s.label)) } } }
                        @let pools: Vec<_> = ended.iter().filter(|p| p.coin_id == c.id).collect();
                        @if !pools.is_empty() {
                            p class="muted sm" { "Still listed on miningpoolstats with no hashrate: "
                                @for (i, p) in pools.iter().enumerate() { @if i > 0 { ", " } (p.name) @if let Some(t) = &p.last_block_time { " (last block " (fmt::utc(Some(t))) ")" } }
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn miners(d: &Data) -> Markup {
    let best = d.miners.iter().filter_map(|m| m.efficiency()).fold(f64::INFINITY, f64::min);
    let worst = d.miners.iter().filter_map(|m| m.efficiency()).fold(0.0, f64::max);
    let z15_coins: Vec<_> = d.coins.iter().filter(|c| c.active() && c.z15_compatible == Some(true)).collect();
    layout(d, Page { title: "Equihash ASIC miners: Antminer Z15, Z15 Pro, Z11, Z9, Innosilicon A9++", description: "Equihash 200,9 ASIC specifications (hashrate, power, computed J/kSol efficiency) from manufacturer spec pages, with sources.", path: "/miners", nav: "miners" }, html! {
        section class="wrap section" {
            h1 class="page-title" { "Equihash ASICs" }
            p class="lede" { "Manufacturer specifications only. Efficiency is computed as watts ÷ kSol/s (lower is better); the manufacturer's own figure is shown alongside when published." }
            div class="miner-grid" {
                @for m in &d.miners {
                    @let eff = m.efficiency();
                    @let rel = eff.map(|e| if worst > best { 1.0 - (e - best) / (worst - best) } else { 1.0 }).unwrap_or(0.0);
                    article class="card pad miner-card" {
                        div class="miner-top" { span class="muted sm" { (m.maker) } span class="badge badge-z15 sm" { "Equihash " (m.equihash) } }
                        h3 { (m.model) }
                        div class="miner-stats" {
                            div { span class="big" { (fmt::opt_num(m.hashrate_ksol)) } span class="unit" { "kSol/s" } }
                            div { span class="big" { (fmt::int(m.watts)) } span class="unit" { "W" } }
                            div { span class="big accent" { (eff.map(|e| format!("{e:.2}")).unwrap_or("n/a".into())) } span class="unit" { "J/kSol" } }
                        }
                        div class="eff-meter" title="Relative efficiency among listed models" { span style={"width:" (format!("{:.0}", 8.0 + rel * 92.0)) "%"} {} }
                        p class="muted sm" { "Stated efficiency: " (m.stated_efficiency_j_per_ksol.map(|e| format!("{} J/kSol", fmt::num_short(e))).unwrap_or("n/a".into())) }
                        @if let Some(n) = &m.notes { p class="sm" { (n) } }
                        p class="sm" { "Source: " @if let Some(u) = &m.source_url { (ext(u, m.source_name.as_deref().unwrap_or(u))) } @else { "n/a" } }
                    }
                }
            }
            div class="table-wrap card mt" {
                table class="net-table" {
                    thead { tr { th { "Model" } th class="num" { "Hashrate (kSol/s)" } th class="num" { "Power (W)" } th class="num" { "Efficiency (J/kSol, computed)" } th class="num" { "Stated" } th { "Source" } } }
                    tbody { @for m in &d.miners { tr {
                        td { (m.maker) " " (m.model) }
                        td class="num" { (fmt::opt_num(m.hashrate_ksol)) }
                        td class="num" { (fmt::int(m.watts)) }
                        td class="num" { (m.efficiency().map(|e| format!("{e:.2}")).unwrap_or("n/a".into())) }
                        td class="num" { (fmt::opt_num(m.stated_efficiency_j_per_ksol)) }
                        td { @if let Some(u) = &m.source_url { (ext(u, &fmt::host(Some(u)).split('/').next().unwrap_or("").to_string())) } }
                    } } }
                }
            }
            @if !d.miners_not_listed.is_empty() {
                p class="fine" { "Specs checked against manufacturer pages " @if let Some(v) = &d.miners_verified_at { "on " (fmt::utc(Some(v))) } ". " "Not listed: " @for n in &d.miners_not_listed { (n.model) " (" (n.reason) ") " } }
            }
            div class="card pad mt" {
                h3 { "What can a Z15 mine?" }
                p { "Every active coin on Equihash 200,9 in this directory: "
                    @for (i, c) in z15_coins.iter().enumerate() { @if i > 0 { ", " } a href={"/?coin=" (c.id) "#pools"} { (c.label) " (" (c.symbol) ")" } }
                    ". Coins on 144,5, 192,7, 210,9 or 125,4 need GPUs or different hardware."
                }
            }
        }
    })
}

pub fn add_pool(d: &Data) -> Markup {
    let template = "Pool name:\nWebsite:\nCoin(s) and Equihash parameters:\nStratum URL(s) and regions:\nPayout scheme(s) and fee for each:\nMinimum payout:\nMerged mining (aux chains), if any:\nPublic stats/API URL (for hashrate, miners, blocks):\nIs the pool listed on miningpoolstats.stream? (yes/no + link)\nContact (for verification):\n";
    layout(d, Page { title: "Add or update your pool", description: "How Equihash pool operators can get their pool listed or corrected on equihash.com.", path: "/add-pool", nav: "add-pool" }, html! {
        section class="wrap section narrow" {
            h1 class="page-title" { "Add or update your pool" }
            p class="lede" { "Listing is free and neutral. There are no paid placements, badges or sponsored rows, and ranking is purely by the data." }
            div class="steps" {
                div class="step card pad" { span class="step-n" { "1" } div { h3 { "Publish your stats" } p { "We only show numbers we can fetch from a public source: a stats page or JSON API with hashrate, miners/workers, blocks, fee and payout scheme. Anything you don't publish shows as n/a." } } }
                div class="step card pad" { span class="step-n" { "2" } div { h3 { "Get on miningpoolstats (recommended)" } p { "Most rows come from " (ext("https://miningpoolstats.stream/zcash", "miningpoolstats.stream")) ". If you're listed there, you'll appear here on the next refresh automatically." } } }
                div class="step card pad" { span class="step-n" { "3" } div { h3 { "Or send us the details" } p { "Copy the template below and send it to the maintainer (contact on the " a href="/about" { "About" } " page). We verify against your public pages before adding a row, and every row cites its source and fetch time." } } }
            }
            div class="card pad mt" {
                div class="row-between" { h3 { "Submission template" } button class="btn sm" type="button" data-copy="#tpl" { "Copy" } }
                pre id="tpl" class="code" { (template) }
            }
            div class="card pad mt" {
                h3 { "Corrections" }
                p { "Spotted something wrong? Send the pool name, the field, the correct value and a public link that shows it. Pools that stop operating move to the " a href="/archive" { "archive" } " once a source confirms it." }
                h3 { "For maintainers" }
                p { "Pools without an API live in " code { "data/curated/manual-pools.json" } " (one JSON object per row, with " code { "source_url" } " and " code { "verified_at" } "). The server picks up edits automatically." }
            }
        }
    })
}

pub fn about(d: &Data) -> Markup {
    layout(d, Page { title: "About", description: "About equihash.com, a neutral directory of Equihash mining pools.", path: "/about", nav: "about" }, html! {
        section class="wrap section narrow" {
            h1 class="page-title" { "About" }
            div class="card pad about-card" {
                p class="about-line" { "Maintained by " a href="https://x.com/RustDev_" rel="noopener" target="_blank" { "@RustDev_" } ", who also builds " a href="https://w.cash" rel="noopener" target="_blank" { "Wcash" } "." }
            }
            div class="card pad mt" {
                h3 { "Principles" }
                ul class="ticks" {
                    li { "Miner-first and neutral: pools are ranked by data, never by relationship or payment." }
                    li { "Sourced: every row links to where its numbers came from, with the time they were fetched." }
                    li { "No invented numbers: unknown values are shown as n/a." }
                    li { "Decentralisation matters: pools above 30% of a network are flagged." }
                }
                h3 { "How the data works" }
                p { "A refresh script pulls miningpoolstats' public JSON for every Equihash coin, adds pools verified directly from their own pages or APIs, and writes plain JSON files that this site reads. See " a href="/sources" { "sources & methodology" } "." }
            }
        }
    })
}

pub fn sources(d: &Data) -> Markup {
    let m = &d.meta;
    layout(d, Page { title: "Sources & methodology", description: "Where every number on equihash.com comes from, and what could not be verified.", path: "/sources", nav: "" }, html! {
        section class="wrap section narrow" {
            h1 class="page-title" { "Sources & methodology" }
            p class="lede" { "Generated " (fmt::utc(m.generated_at.as_deref())) ": " (m.pool_count) " pool rows (" (m.mps_pool_count) " from miningpoolstats, " (m.non_mps_pool_count) " verified directly) across " (m.coin_count) " coins." }
            div class="card pad" {
                h3 { "Live sources (refresh script)" }
                ul class="src-list" { @for s in &m.sources { li { (ext(&s.url, &s.label)) } } }
                h3 { "Curated sources" }
                ul class="src-list" {
                    li { (ext("https://w.cash/whitepaper", "Wcash protocol specification")) " (merged-mining guide, WEC parameters)" }
                    li { (ext("https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-zcash-aux/README.md", "wcash-zcash-aux README @ 3e6b8044")) }
                    li { (ext("https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-merge-miner/README.md", "wcash-merge-miner README @ 3e6b8044")) }
                    li { "ASIC specs: Bitmain support spec pages and Innosilicon product page (linked per row on " a href="/miners" { "Miners" } ")." }
                    li { "Archive: pool/operator announcements and community threads (linked per entry on " a href="/archive" { "Archive" } ")." }
                }
                h3 { "Method" }
                ul class="ticks" {
                    li { "miningpoolstats: the coin page is loaded to read its current data timestamp, then " code { "data.miningpoolstats.stream/data/<coin>.js?t=<ts>" } " is fetched (browser User-Agent + Referer)." }
                    li { "MPS reports unknown/hidden values as -1/-2; these become n/a." }
                    li { "% of network = pool hashrate ÷ the coin's network hashrate from the same file. On small coins the network estimate is sometimes below the sum of what pools report; then shares are measured against the listed pools' total instead and labelled as such." }
                    li { "Equihash parameters: MPS label; plain \"Equihash\" is the 200,9 set (MPS ASIC-Equihash group)." }
                    li { "Prices (calculator only) come from the miningpoolstats price feed and can be overridden." }
                }
            }
            div class="card pad mt" {
                h3 { "Research log" }
                table class="net-table" {
                    thead { tr { th { "Pool" } th { "Outcome" } th { "Detail" } } }
                    tbody { @for r in &d.research { tr {
                        td { (r.pool) }
                        td { span class={"chip chip-" (r.outcome)} { (r.outcome) } }
                        td { (r.detail) @if let Some(u) = &r.url { " " (ext(u, "↗")) } }
                    } } }
                }
                h3 { "Coin notes" }
                ul class="ticks" { @for c in &d.research_coins { li { strong { (c.coin) } ": " (c.detail) } } }
            }
            @if !m.errors.is_empty() {
                div class="card pad mt" {
                    h3 { "Warnings from the last refresh" }
                    ul class="mono sm" { @for e in &m.errors { li { (e) } } }
                }
            }
        }
    })
}

pub fn not_found(d: &Data) -> Markup {
    layout(d, Page { title: "Not found", description: "Page not found.", path: "/404", nav: "" }, html! {
        section class="wrap section narrow center" {
            h1 class="page-title" { "404" }
            p class="lede" { "That page doesn't exist. " a href="/" { "Back to the pool directory →" } }
        }
    })
}
