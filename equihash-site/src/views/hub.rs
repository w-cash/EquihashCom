//! The discovery layer: a fast way from a coin or machine name to compatible hardware, pools,
//! calculations and source-backed guides. The detailed pool table remains in `home.rs`.

use crate::data::{Coin, Data, Miner, Pool};
use crate::fmt;
use crate::views::home::schemes_text;
use crate::views::layout::{ext, layout, Page};
use crate::views::links;
use crate::views::logo::{self, At};
use maud::{html, Markup};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct SearchQuery {
    pub q: Option<String>,
}

fn search_form(value: &str, label: &str) -> Markup {
    html! {
        form class="hub-search" action="/search" method="get" role="search" {
            label for="site-q" { (label) }
            div class="hub-search-row" {
                input id="site-q" type="search" name="q" value=(value) placeholder="Try Z15 Pro, Zcash, Wcash or a pool name" autocomplete="off";
                button class="btn" type="submit" { "Search" }
            }
        }
    }
}

fn status(c: &Coin) -> Markup {
    html! {
        span class={"status " @if c.active() { "active" } @else { "ended" }} {
            @if c.active() { "Active PoW" } @else { "PoW ended" }
        }
    }
}

fn coin_rows(d: &Data, active_only: bool, featured_only: bool) -> Markup {
    let groups: Vec<_> = d
        .param_groups(active_only)
        .into_iter()
        .filter(|(group, _)| !featured_only || *group == "Equihash 200,9")
        .collect();
    html! {
        @for (group, coins) in groups {
            section class="coin-group" {
                div class="section-head compact" {
                    div { h2 { (group) } p { (coins.len()) @if coins.len() == 1 { " listed coin" } @else { " listed coins" } } }
                    @if group == "Equihash 200,9" { a href="/hardware/antminer-z15-pro" { "Z15 Pro compatible →" } }
                }
                div class="table-scroll" {
                    table class="data directory coin-directory" {
                        thead { tr {
                            th scope="col" { "Coin" }
                            th scope="col" { "Status" }
                            th class="num" scope="col" { "Network" }
                            th class="num" scope="col" { "Listed pools" }
                            th class="num" scope="col" { "Price" }
                            th scope="col" { "Next step" }
                        } }
                        tbody {
                            @for c in coins {
                                tr {
                                    td { a class="entity-link" href={"/coin/" (c.id)} { (logo::chip(&c.logo, &c.name, At::Row, true)) strong { (c.name) } } " " span class="sym" { (c.symbol) } }
                                    td { (status(c)) }
                                    td class="num" { (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) }
                                    td class="num" { (c.pool_count) }
                                    td class="num" { (fmt::price(c.price_usd)) }
                                    td class="nowrap" { @if c.active() && c.pool_count > 0 { a href={"/pools?coin=" (c.id) "#pools"} { "Compare pools" } } @else { a href={"/coin/" (c.id)} { "View record" } } }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn index(d: &Data) -> Markup {
    let active = d.coins.iter().filter(|c| c.active()).count();
    let pools = d.live_pools().count();
    let reporting = d
        .live_pools()
        .filter(|p| p.hashrate.unwrap_or(0.0) > 0.0)
        .count();
    let z15 = d
        .coins
        .iter()
        .filter(|c| c.active() && c.nk() == Some((200, 9)))
        .count();
    let zcash = d.coins.iter().find(|c| c.id == "zcash");
    let merged_pools = d
        .live_pools()
        .filter(|p| p.coin_id == "zcash" && p.merged())
        .count();
    layout(d, Page {
        title: "Equihash mining: coins, pools, hardware and guides · equihash.com",
        description: "Find Equihash coins, compare mining pools, check Z15 Pro compatibility and calculate mining output from source-backed live data.",
        path: "/",
        nav: "home",
    }, html! {
        div class="ih-home" {
            section class="ih-hero" aria-labelledby="ih-title" {
                div class="ih-hero-image" role="img" aria-label="Illustration of industrial ASIC mining infrastructure" {}
                div class="ih-hero-shade" {}
                div class="wrap ih-hero-inner" {
                    div class="ih-hero-copy" {
                        p class="ih-kicker" { span class="ih-live-dot" aria-hidden="true" {} "THE OPEN EQUIHASH DIRECTORY" }
                        h1 id="ih-title" { "The Equihash mining network, " span { "mapped." } }
                        p { "Compare coins, pools and ASIC hardware. Move from a Z15 Pro to a supporting pool, a sourced calculation and ZEC + WEC merged mining." }
                        div class="ih-actions" {
                            a class="ih-btn primary" href="/pools" { "Compare mining pools" span aria-hidden="true" { "↗" } }
                            a class="ih-btn secondary" href="/hardware/antminer-z15-pro" { "Explore the Z15 Pro" span aria-hidden="true" { "→" } }
                        }
                    }
                    div class="ih-hero-utility" {
                        (search_form("", "Search a coin, pool or miner"))
                        p { "Independent directory · no paid placement · public sources" }
                    }
                }
                p class="ih-image-note" { "Original illustrative infrastructure visual" }
            }

            section class="wrap ih-overview" aria-labelledby="overview-title" {
                header class="ih-section-head" {
                    div { p class="ih-overline" { "CURRENT COVERAGE" } h2 id="overview-title" { "A live view of the ecosystem" } }
                    p { "Latest directory snapshot " time class="ago" datetime=[d.last_updated.as_deref()] { (fmt::utc(d.last_updated.as_deref())) } " · " a href="/sources" { "Sources and method" } }
                }
                dl class="ih-metrics" {
                    div { dt { "Active networks" } dd { (active) } small { "across exact n,k groups" } }
                    div { dt { "Pool listings" } dd { (pools) } small { (reporting) " publish hashrate" } }
                    div { dt { "Z15-ready" } dd { (z15) } small { "active 200,9 networks" } }
                    div { dt { "ZEC network" } dd { @if let Some(c) = zcash { (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) } @else { "n/a" } } small { @if let Some(c) = zcash { (fmt::price(c.price_usd)) } @else { "price n/a" } } }
                }
            }

            section class="wrap ih-directory" aria-labelledby="coins-title" {
                div class="ih-section-head directory-head" {
                    div { p class="ih-overline" { "THE DIRECTORY" } h2 id="coins-title" { "Find your network" } p { "Compatibility starts with the exact Equihash parameters. Figures are never compared across different n,k groups." } }
                    a class="ih-text-link" href="/coins" { "All coin records →" }
                }
                (coin_rows(d, true, true))
            }

            section class="ih-route-band" aria-labelledby="routes-title" {
                div class="wrap" {
                    div class="ih-section-head on-dark" {
                        div { p class="ih-overline" { "OPERATING PATHS" } h2 id="routes-title" { "Start with the work in front of you" } }
                    }
                    div class="ih-routes" {
                        a href="/pools" { span class="ih-route-no" { "01" } h3 { "Compare pools" } p { "Fees, payouts, regions, hashrate and concentration." } strong { "Open pool intelligence →" } }
                        a href="/hardware" { span class="ih-route-no" { "02" } h3 { "Choose hardware" } p { "Exact parameter compatibility and operating inputs." } strong { "Explore ASIC hardware →" } }
                        a href="/buy" { span class="ih-route-no" { "03" } h3 { "Find a vendor" } p { "Dated offers with VAT, stock and shipping context." } strong { "Compare vendor offers →" } }
                    }
                }
            }

            section class="wrap ih-product" aria-labelledby="z15-feature-title" {
                div class="ih-product-visual" {
                    img src="/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp" alt="Bitmain Antminer Z15 Pro ASIC miner" width="1200" height="1200";
                    span { "FLAGSHIP EQUIHASH 200,9 ASIC" }
                }
                div class="ih-product-copy" {
                    p class="ih-overline" { "HARDWARE PROFILE" }
                    h2 id="z15-feature-title" { "Antminer Z15 Pro" }
                    p class="ih-product-lede" { "One machine profile connects manufacturer specifications, compatible networks, pool comparison, operating-cost calculations and vendor offers." }
                    dl class="ih-specs" {
                        div { dt { "Typical hashrate" } dd { "840 kSol/s" } }
                        div { dt { "Power" } dd { "2,780 W" } }
                        div { dt { "Algorithm" } dd { "Equihash 200,9" } }
                    }
                    div class="ih-actions light" {
                        a class="ih-btn primary" href="/hardware/antminer-z15-pro" { "View machine profile" }
                        a class="ih-btn secondary" href="/buy?machine=antminer-z15-pro" { "Where to buy" }
                        a class="ih-text-link" href="/calculator?hashrate=840&watts=2780" { "Calculate output →" }
                    }
                    p class="ih-source-note" { "Specifications use the manufacturer-sourced profile. Vendor claims remain separately labelled." }
                }
            }

            section class="ih-merge-feature" aria-labelledby="merge-feature-title" {
                div class="wrap ih-merge-inner" {
                    div class="ih-merge-copy" {
                        p class="ih-overline" { "ZEC + WEC MERGED MINING" }
                        h2 id="merge-feature-title" { "One miner. Two independent targets." }
                        p { "A participating pool can submit the same Equihash 200,9 work to Zcash and Wcash. Your ASIC keeps mining normally; the pool handles the auxiliary chain and its separate payouts." }
                        p class="ih-merge-fact" { strong { (merged_pools) } " currently listed supporting pool" @if merged_pools != 1 { "s" } }
                        div class="ih-actions" {
                            a class="ih-btn primary" href="/pools?coin=zcash&merged=1#pools" { "See supporting pools" }
                            a class="ih-btn secondary" href="/merged-mining" { "How merged mining works" }
                        }
                        p class="ih-disclosure" { "Wcash and equihash.com share a maintainer. Only work from participating pools secures Wcash." }
                    }
                    div class="ih-flow" role="img" aria-label="A Z15 Pro sends work to a compatible pool, which checks independent Zcash and Wcash targets" {
                        div class="ih-flow-node miner" { span { "EQUIHASH 200,9" } strong { "Z15 PRO" } small { "one work stream" } }
                        div class="ih-flow-line" aria-hidden="true" {}
                        div class="ih-flow-node pool" { span { "SUPPORTING" } strong { "POOL" } small { "routes valid work" } }
                        div class="ih-flow-branches" aria-hidden="true" { i {} i {} }
                        div class="ih-flow-targets" {
                            div { span { "PARENT" } strong { "ZCASH" } small { "ZEC target" } }
                            div { span { "AUXILIARY" } strong { "WCASH" } small { "WEC target" } }
                        }
                    }
                }
            }

            aside class="wrap ih-contribute" {
                div { p class="ih-overline" { "INDUSTRY PARTICIPATION" } h2 { "Make the directory more complete." } p { "Pool operators, coin teams and hardware vendors can add public facts or correct an existing record. Listings remain free and source-backed." } }
                div class="ih-actions light" { a class="ih-btn primary" href="/contribute" { "Add or correct a listing" } a class="ih-text-link" href="https://t.me/EquihashCom" rel="noopener" { "Contact on Telegram →" } }
            }
        }
    })
}

pub fn coins(d: &Data) -> Markup {
    layout(d, Page {
        title: "Equihash coins by parameter set",
        description: "Active and historical Equihash coins grouped by exact n,k parameters, with compatible hardware, mining pools and sourced network data.",
        path: "/coins",
        nav: "coins",
    }, html! {
        div class="wrap page directory-page" {
            header class="page-head split-head" {
                div { p class="eyebrow" { "COIN DIRECTORY" } h1 { "Equihash coins" } p class="lede" { "Browse active and historical networks by exact parameters. Open a coin for compatible hardware, pools, calculator inputs, official links and data sources." } }
                (search_form("", "Search the directory"))
            }
            (coin_rows(d, false, false))
        }
    })
}

fn coin_pools<'a>(d: &'a Data, c: &Coin) -> Vec<&'a Pool> {
    let mut rows: Vec<_> = d.live_pools().filter(|p| p.coin_id == c.id).collect();
    rows.sort_by(|a, b| {
        b.hashrate
            .partial_cmp(&a.hashrate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn compatible_miners<'a>(d: &'a Data, c: &Coin) -> Vec<&'a Miner> {
    d.miners
        .iter()
        .filter(|m| m.equihash == c.params())
        .collect()
}

pub fn coin(d: &Data, c: &Coin) -> Markup {
    let title = format!(
        "{} ({}) mining: pools, hardware and network",
        c.name, c.symbol
    );
    let desc = format!("{} Equihash {} mining overview: compatible hardware, pool fees and payouts, network data, calculator and sources.", c.name, c.params());
    let path = format!("/coin/{}", c.id);
    let pools = coin_pools(d, c);
    let miners = compatible_miners(d, c);
    let is_wcash = c.id == "wcash";
    layout(
        d,
        Page {
            title: &title,
            description: &desc,
            path: &path,
            nav: "coins",
        },
        html! {
            div class="wrap page entity-page" {
                p class="crumb" { a href="/coins" { "All coins" } " / " (c.group_label()) }
                header class="entity-hero" {
                    div class="entity-title" { (logo::chip(&c.logo, &c.name, At::Head, false)) div { h1 { (c.name) " " span class="sym" { (c.symbol) } } p { "Equihash " span class="mono" { (c.params()) } " · " (status(c)) } } }
                    (links::render(&c.links, &format!("{} official and community links", c.name), "coin-links"))
                    @if is_wcash { p class="disclosure" { strong { "Disclosure: " } "equihash.com and Wcash share a maintainer. Wcash receives merged-mined work only through pools that explicitly support it; the pool directory uses the same source and ordering rules for every coin." } }
                }

                dl class="fact-strip" {
                    div { dt { "Network estimate" } dd { (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) } }
                    div { dt { "Listed-pool total" } dd { @let (rep, dag) = fmt::reported(c); (rep) @if dag { "†" } } }
                    div { dt { "Active pool rows" } dd { (c.pool_count) } }
                    div { dt { "Price" } dd { (fmt::price(c.price_usd)) } }
                    div { dt { "Block target" } dd { (fmt::seconds(c.network.block_time_target_s)) } }
                }

                div class="entity-layout" {
                    section class="entity-main" aria-labelledby="pool-title" {
                        div class="section-head" { div { h2 id="pool-title" { "Mining pools" } p { "A concise view of active listings. Open the full comparison for all fields and filters." } } a href={"/pools?coin=" (c.id) "#pools"} { "Full pool comparison →" } }
                        @if pools.is_empty() {
                            div class="empty-state" { h3 { "No active pool is listed" } p { "The network record remains available for research. If a public pool exists, send its evidence for review." } a href="/contribute#pool" { "Submit a pool" } }
                        } @else {
                            div class="table-scroll" { table class="data directory pool-summary" {
                                thead { tr { th scope="col" { "Pool" } th class="num" scope="col" { "Hashrate" } th class="num" scope="col" { "Fee" } th scope="col" { "Payout" } th scope="col" { "Region" } th scope="col" { "Merged" } } }
                                tbody { @for p in pools.iter().take(12) { tr {
                                    td { a class="entity-link" href={"/pool/" (p.slug)} { (logo::chip(&p.logo, &p.name, At::Row, true)) (p.name) } }
                                    td class="num" { (fmt::hashrate(p.hashrate, p.hashrate_unit.as_deref().unwrap_or("Sol/s"))) }
                                    td class="num" { (fmt::fee(p.fee_range())) }
                                    td { (schemes_text(p)) }
                                    td { (p.region.as_deref().unwrap_or("n/a")) }
                                    td { @if p.merged() { "Yes" } @else { "—" } }
                                } } }
                            } }
                        }

                        section class="qa" aria-labelledby="qa-title" {
                            div class="section-head" { div { h2 id="qa-title" { "Common questions" } p { "Short answers tied to this network record." } } a href="/contribute#question" { "Ask or improve an answer" } }
                            details open {
                                summary { "Can an Antminer Z15 Pro mine " (c.name) "?" }
                                p { @if c.nk() == Some((200, 9)) { "Yes. The Z15 Pro runs Equihash 200,9, which exactly matches this network. Use the hardware page to compare output and power assumptions." } @else { "No. The Z15 Pro runs Equihash 200,9, while this network uses " (c.params()) ". Matching the Equihash name alone is not enough; n and k must match." } }
                            }
                            details {
                                summary { "How should I choose a pool?" }
                                p { "Check the pool's current hashrate, fee, payout method, minimum payout, server region and source freshness. Avoid choosing from hashrate alone; concentration and latency matter too." }
                            }
                            @if c.id == "zcash" {
                                details { summary { "Can Zcash work also secure Wcash?" } p { "Yes, when the Zcash pool runs the Wcash auxiliary mining software. The miner submits ordinary Zcash work; the participating pool can reuse valid proof for Wcash without splitting the miner's Zcash hashrate. " a href="/merged-mining" { "Read the merged-mining guide and find supporting pools." } } }
                            }
                            @if c.id == "wcash" { details { summary { "Where does Wcash's mining work come from?" } p { "Only work routed through participating merged-mining pools secures Wcash. " a href="/merged-mining" { "See the miner and pool-operator flow." } } } }
                        }
                    }

                    aside class="entity-side" {
                        section class="side-card" { h2 { "Compatible hardware" }
                            @if miners.is_empty() { p class="na" { "No verified hardware spec in the directory matches this parameter set." } }
                            @for m in miners.iter().take(5) { a class="side-row" href={"/hardware/" (m.id)} { span { (m.maker) " " strong { (m.model) } } small { (fmt::opt_num(m.hashrate_ksol)) " kSol/s · " (fmt::int(m.watts)) " W" } } }
                            a class="more-link" href="/hardware" { "All hardware →" }
                        }
                        section class="side-card action-card" { h2 { "Calculate output" } p { "Use this network's sourced values, then edit fee, power rate or hardware assumptions." } a class="btn" href={"/calculator?coin=" (c.id)} { "Open calculator" } }
                        section class="side-card" { h2 { "Source trail" } p { "Network and pool values keep their public source and observation time. Raw JSON is available for reuse." } a href="/sources" { "Method and limitations" } br; a href="/data/network.json" { "Download network data" } }
                    }
                }
            }
        },
    )
}

pub fn hardware_detail(d: &Data, m: &Miner) -> Markup {
    let coins: Vec<_> = d
        .coins
        .iter()
        .filter(|c| c.active() && c.params() == m.equihash)
        .collect();
    let title = format!(
        "{} {}: compatible Equihash coins and mining calculator",
        m.maker, m.model
    );
    let desc = format!("{} {} specifications, efficiency, compatible Equihash {} coins, pools and prefilled mining calculator.", m.maker, m.model, m.equihash);
    let path = format!("/hardware/{}", m.id);
    layout(
        d,
        Page {
            title: &title,
            description: &desc,
            path: &path,
            nav: "hardware",
        },
        html! {
            div class="wrap page entity-page" {
                p class="crumb" { a href="/hardware" { "All hardware" } " / " (m.maker) }
                header class="entity-hero hardware-hero" {
                    p class="eyebrow" { "VERIFIED MANUFACTURER SPEC" }
                    h1 { (m.maker) " " (m.model) }
                    p class="lede" { "A direct path from this machine to compatible coins, current pools and editable mining estimates." }
                }
                dl class="fact-strip" {
                    div { dt { "Parameters" } dd class="mono" { (m.equihash) } }
                    div { dt { "Hashrate" } dd { (fmt::opt_num(m.hashrate_ksol)) " kSol/s" } }
                    div { dt { "Power" } dd { (fmt::int(m.watts)) " W" } }
                    div { dt { "Computed efficiency" } dd { (m.efficiency().map(|v| format!("{v:.2} J/kSol")).unwrap_or("n/a".into())) } }
                    div { dt { "Source" } dd { @if let Some(u) = &m.source_url { (ext(u, m.source_name.as_deref().unwrap_or("Manufacturer page"))) } @else { "n/a" } } }
                }
                div class="entity-layout" {
                    section class="entity-main" {
                        div class="section-head" { div { h2 { "Compatible active coins" } p { "Exact parameter match: Equihash " (m.equihash) "." } } }
                        div class="entity-grid" {
                            @for c in &coins { article {
                                div class="entity-title small" { a href={"/coin/" (c.id)} { (logo::chip(&c.logo, &c.name, At::List, true)) strong { (c.name) } } span class="sym" { (c.symbol) } }
                                p { (c.pool_count) " listed pool" @if c.pool_count != 1 { "s" } " · " (fmt::hashrate(c.network.hashrate, c.network.unit.as_deref().unwrap_or("Sol/s"))) " network" }
                                div class="card-actions" { a href={"/pools?coin=" (c.id) "#pools"} { "Pools" } a href={"/calculator?coin=" (c.id) "&hashrate=" (fmt::opt_num(m.hashrate_ksol)) "&watts=" (fmt::opt_num(m.watts))} { "Calculate" } }
                            } }
                        }
                        section class="qa" { div class="section-head" { div { h2 { "Before you point the miner" } } }
                            details open { summary { "Does every Equihash coin work on this machine?" } p { "No. A miner must match the exact n,k parameters. This device runs " (m.equihash) "; the compatible list above is generated from that exact match." } }
                            details { summary { "Which pool should I use?" } p { "Start with a nearby server and a payout method you understand, then compare fee, minimum payout, current hashrate and concentration. Confirm the stratum address on the pool's own site before configuring the miner." } }
                        }
                    }
                    aside class="entity-side" {
                        section class="side-card action-card" { h2 { "Run the numbers" } p { "The calculator opens with this machine's hashrate and power. Choose a coin and enter your electricity rate." } a class="btn" href={"/calculator?hashrate=" (fmt::opt_num(m.hashrate_ksol)) "&watts=" (fmt::opt_num(m.watts))} { "Use " (m.model) " defaults" } }
                        @if d.listings.iter().any(|l| l.miner_id == m.id) { section class="side-card action-card" { h2 { "Where to buy" } p { "Compare sourced vendor price, stock, VAT and shipping claims for this exact model." } a class="btn" href={"/buy?machine=" (m.id)} { "View vendor listings" } } }
                        @if m.equihash == "200,9" { section class="side-card" { h2 { "Merged mining" } p { "Some Zcash pools can reuse the same work for Wcash without splitting Zcash hashrate." } a href="/merged-mining" { "How ZEC + WEC mining works →" } } }
                        section class="side-card" { h2 { "Setup path" } ol { li { "Choose one exact-match coin." } li { "Compare active pools and regions." } li { "Verify the stratum address at the pool." } li { "Enter worker and wallet details in the miner UI." } li { "Check accepted shares and payout threshold." } } }
                    }
                }
            }
        },
    )
}

pub fn guides(d: &Data) -> Markup {
    layout(d, Page { title: "Equihash mining guides", description: "Practical, source-backed guides for Equihash parameters, Antminer Z15 Pro setup, pool selection, calculators and merged mining.", path: "/guides", nav: "guides" }, html! {
        div class="wrap page guides-page" {
            header class="page-head" { p class="eyebrow" { "PRACTICAL KNOWLEDGE" } h1 { "Equihash guides" } p class="lede" { "Start with a concrete mining job. Each guide connects to live listings and shows where its facts came from." } }
            div class="guide-grid featured-guides" {
                article { p class="step" { "START HERE · 7 MIN" } h2 { a href="/hardware/antminer-z15-pro" { "Z15 Pro: from power-on to a compatible pool" } } p { "Check exact parameters, choose a coin, compare regions and fees, calculate power cost, and verify accepted shares." } }
                article { p class="step" { "POOL CHOICE · 5 MIN" } h2 { a href="/pools?coin=zcash#pools" { "How to compare Zcash pools" } } p { "Read hashrate, concentration, PPLNS/PPS variants, fees, minimum payouts, regions and observation times." } }
                article { p class="step" { "MERGED MINING · 10 MIN" } h2 { a href="/merged-mining" { "How Zcash + Wcash merged mining works" } } p { "Follow the work from a Zcash miner through a participating pool to the Wcash auxiliary chain, with protocol sources." } }
                article { p class="step" { "PARAMETERS · 4 MIN" } h2 { a href="/coins" { "Why Equihash n,k decides compatibility" } } p { "Understand why 200,9 hardware cannot mine every coin carrying the Equihash name." } }
                article { p class="step" { "ECONOMICS · 5 MIN" } h2 { a href="/calculator" { "Calculate output without trusting a ranking" } } p { "Use current network inputs, edit uncertain values and include fee, watts and electricity cost." } }
                article { p class="step" { "VERIFY · 6 MIN" } h2 { a href="/sources" { "How equihash.com checks and labels data" } } p { "See refresh sources, per-field timestamps, live endpoints, unknown-value rules and downloadable JSON." } }
            }
            aside class="listing-callout" { div { p class="eyebrow" { "COMMUNITY QUESTIONS" } h2 { "What should the next answer cover?" } p { "Send a specific miner, pool or coin question with a public source. Useful answers will be attached to the relevant directory page." } } a class="btn" href="/contribute#question" { "Ask a question" } }
        }
    })
}

fn matches(needle: &str, fields: &[&str]) -> bool {
    fields
        .iter()
        .any(|v| v.to_ascii_lowercase().contains(needle))
}

pub fn search(d: &Data, q: &SearchQuery) -> Markup {
    let raw = q.q.as_deref().unwrap_or("").trim();
    let needle = raw.to_ascii_lowercase();
    let coins: Vec<_> = if needle.is_empty() {
        Vec::new()
    } else {
        d.coins
            .iter()
            .filter(|c| matches(&needle, &[&c.name, &c.symbol, &c.id, &c.params()]))
            .take(20)
            .collect()
    };
    let miners: Vec<_> = if needle.is_empty() {
        Vec::new()
    } else {
        d.miners
            .iter()
            .filter(|m| matches(&needle, &[&m.maker, &m.model, &m.equihash]))
            .take(20)
            .collect()
    };
    let pools: Vec<_> = if needle.is_empty() {
        Vec::new()
    } else {
        d.pools
            .iter()
            .filter(|p| {
                matches(
                    &needle,
                    &[
                        &p.name,
                        &p.coin,
                        &p.coin_label,
                        p.region.as_deref().unwrap_or(""),
                    ],
                )
            })
            .take(30)
            .collect()
    };
    let vendors: Vec<_> = if needle.is_empty() {
        Vec::new()
    } else {
        d.vendors
            .iter()
            .filter(|v| {
                matches(
                    &needle,
                    &[
                        &v.name,
                        &v.slug,
                        &v.regions.join(" "),
                        v.region_focus.as_deref().unwrap_or(""),
                    ],
                )
            })
            .take(20)
            .collect()
    };
    let merged = !needle.is_empty()
        && [
            "merged mining",
            "merge mining",
            "auxpow",
            "zec wec",
            "zcash wcash",
            "wcash zcash",
        ]
        .iter()
        .any(|term| term.contains(&needle) || needle.contains(term));
    let buying = !needle.is_empty()
        && ["buy", "vendor", "shop", "seller", "where to buy"]
            .iter()
            .any(|term| term.contains(&needle) || needle.contains(term));
    let count = coins.len()
        + miners.len()
        + pools.len()
        + vendors.len()
        + usize::from(merged)
        + usize::from(buying);
    layout(d, Page { title: "Search Equihash coins, pools, hardware and vendors", description: "Search the equihash.com directory for coins, mining pools, hardware, vendors and guides.", path: "/search", nav: "" }, html! {
        div class="wrap page search-page" {
            header class="page-head" { h1 { "Search equihash.com" } (search_form(raw, "Coin, pool, hardware or parameter set")) }
            @if raw.is_empty() { div class="empty-state" { h2 { "Search the whole directory" } p { "Try “Z15 Pro”, “Zcash”, “Wcash”, “merged mining”, a vendor, pool or region." } } }
            @else if count == 0 { div class="empty-state" { h2 { "No results for “" (raw) "”" } p { "Try a coin symbol, miner model, exact parameter set or shorter pool name." } a href="/contribute" { "Suggest a missing listing" } } }
            @else {
                p class="result-count" { (count) " result" @if count != 1 { "s" } " for “" (raw) "”" }
                div class="search-results" {
                    @if !coins.is_empty() { section { h2 { "Coins" } div class="result-list" { @for c in coins { a href={"/coin/" (c.id)} { span { (logo::chip(&c.logo, &c.name, At::List, true)) strong { (c.name) } " " span class="sym" { (c.symbol) } } small { "Equihash " (c.params()) " · " (c.pool_count) " pools" } } } } } }
                    @if !miners.is_empty() { section { h2 { "Hardware" } div class="result-list" { @for m in miners { a href={"/hardware/" (m.id)} { span { strong { (m.maker) " " (m.model) } } small { "Equihash " (m.equihash) " · " (fmt::opt_num(m.hashrate_ksol)) " kSol/s" } } } } } }
                    @if !pools.is_empty() { section { h2 { "Pools" } div class="result-list" { @for p in pools { a href={"/pool/" (p.slug)} { span { (logo::chip(&p.logo, &p.name, At::List, true)) strong { (p.name) } } small { (p.coin_label) " · " (p.region.as_deref().unwrap_or("region n/a")) } } } } } }
                    @if !vendors.is_empty() { section { h2 { "Vendors" } div class="result-list" { @for v in vendors { a href={"/buy/vendor/" (v.slug)} { span { strong { (v.name) } } small { (v.region_focus.as_deref().unwrap_or("Region not stated")) " · " (v.regions.join(", ")) } } } } } }
                    @if merged || buying { section { h2 { "Guides and tools" } div class="result-list" {
                        @if merged { a href="/merged-mining" { span { strong { "Zcash + Wcash merged mining" } } small { "Miner overview and pool operator guide" } } }
                        @if buying { a href="/buy" { span { strong { "Where to buy Equihash miners" } } small { "Sourced prices, stock claims, VAT and shipping notes" } } }
                    } } }
                }
            }
        }
    })
}

pub fn contribute(d: &Data) -> Markup {
    let pool = "Type: pool listing or correction\nPool name:\nWebsite:\nCoin(s) and exact Equihash parameters:\nStratum hosts, ports and regions:\nPayout schemes and fee for each:\nMinimum payout:\nMerged-mined coins, if any:\nPublic stats page or JSON API:\nContact for verification:\n";
    let project = "Type: coin or hardware listing/correction\nName and symbol/model:\nOfficial website:\nExact Equihash parameters:\nPublic specification or source:\nWhat should be added or corrected:\nContact for verification:\n";
    let question = "Type: question or guide correction\nPage or entity:\nQuestion/correction:\nPublic source, if applicable:\nHow may we credit you? (optional):\n";
    layout(d, Page { title: "Add a listing, correction or question", description: "Submit an Equihash pool, coin, hardware listing, correction or community question to equihash.com.", path: "/contribute", nav: "" }, html! {
        div class="wrap page narrow prose contribute-page" {
            header class="page-head" { p class="eyebrow" { "CONTRIBUTE" } h1 { "Add what miners need" } p class="lede" { "Listings are free. Public evidence is required for factual claims, and accepted fields keep their source. Send a completed template to " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak on X" } " or " a href="https://t.me/EquihashCom" rel="noopener" { "EquihashCom on Telegram" } "." } }
            nav class="contribute-nav" aria-label="Contribution types" { a href="#pool" { "Pool" } a href="#project" { "Coin or hardware" } a href="/add-vendor" { "Vendor" } a href="#question" { "Question or correction" } }
            section id="pool" { h2 { "Pool listing or correction" } p { "A useful pool listing needs a public website plus enough evidence to verify fee, payout, endpoints and current activity. Public machine-readable stats give miners the best record." } div class="tpl" { div class="tpl-head" { span { "Pool template" } button class="btn small" type="button" data-copy="#pool-template" { "Copy" } } pre id="pool-template" { (pool) } } }
            section id="project" { h2 { "Coin or hardware listing" } p { "Compatibility is based on exact n,k parameters. Link an official protocol or manufacturer specification wherever possible." } div class="tpl" { div class="tpl-head" { span { "Project template" } button class="btn small" type="button" data-copy="#project-template" { "Copy" } } pre id="project-template" { (project) } } }
            section id="question" { h2 { "Question, guide idea or correction" } p { "Ask something other miners will search for. Answers are reviewed and attached to the relevant coin, pool, hardware or guide page so they remain useful." } div class="tpl" { div class="tpl-head" { span { "Question template" } button class="btn small" type="button" data-copy="#question-template" { "Copy" } } pre id="question-template" { (question) } } }
            p class="small" { "Submissions are reviewed for relevance and verifiability. Listings are never sold and payment cannot change their order." }
        }
    })
}
