//! Pages for finding a coin, pool or machine and following it to the relevant data.
//! The full pool table remains in `home.rs`.

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

fn coin_rows(d: &Data, active_only: bool, limit: Option<usize>) -> Markup {
    let groups = d.param_groups(active_only);
    html! {
        @for (group, coins) in groups {
            section class="coin-group" id=[(group == "Equihash 200,9").then_some("equihash-200-9")] {
                div class="section-head compact" {
                    div { h2 { (group) } p { @if let Some(max) = limit { (coins.len().min(max)) " of " (coins.len()) " active coins shown" } @else { (coins.len()) @if coins.len() == 1 { " listed coin" } @else { " listed coins" } } } }
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
                            @for c in coins.iter().take(limit.unwrap_or(coins.len())) {
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
    let z15 = d
        .coins
        .iter()
        .filter(|c| c.active() && c.nk() == Some((200, 9)))
        .count();
    let merged_pools = d
        .live_pools()
        .filter(|p| p.coin_id == "zcash" && p.merged())
        .count();
    let featured: Vec<_> = [
        (
            "antminer-z15-pro",
            "/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp",
            "Bitmain Antminer Z15 Pro ASIC miner",
            1200,
            1200,
        ),
        (
            "antminer-z15",
            "/static/shop/machines/antminer-z15.8dc9fd7a98.webp",
            "Bitmain Antminer Z15 ASIC miner",
            924,
            1000,
        ),
        (
            "antminer-z11",
            "/static/shop/machines/antminer-z11.1c5e36ca87.webp",
            "Bitmain Antminer Z11 ASIC miner",
            1200,
            630,
        ),
        (
            "innosilicon-a9pp-zmaster",
            "/static/shop/machines/innosilicon-a9pp-zmaster.e4b128723d.webp",
            "Innosilicon A9++ ZMaster ASIC miner",
            1000,
            680,
        ),
    ]
    .into_iter()
    .filter_map(|(id, image, alt, width, height)| {
        d.miners
            .iter()
            .find(|m| m.id == id)
            .map(|m| (m, image, alt, width, height))
    })
    .collect();
    layout(d, Page {
        title: "Equihash mining: coins, pools, hardware and guides · equihash.com",
        description: "Look up Equihash coins and mining pools, see which networks support a Z15 Pro, and estimate mining costs and output.",
        path: "/",
        nav: "home",
    }, html! {
        div class="ed-home" {
            section class="ed-hero" aria-labelledby="ed-title" {
                div class="wrap ed-hero-grid" {
                    div class="ed-hero-copy" {
                        p class="ed-edition" { "COINS / POOLS / HARDWARE / MERGED MINING" }
                        h1 id="ed-title" { "Equihash mining." }
                        p class="ed-lede" { "Find coins your hardware can mine, compare pools, estimate electricity costs and check current Z15 Pro offers." }
                        (search_form("", "Search coins, pools or hardware"))
                        div class="ed-hero-links" {
                            a class="ed-primary" href="/pools" { "Compare pools" }
                            a href="/hashpower" { "Hashpower market" }
                            a href="/hardware/antminer-z15-pro" { "Z15 Pro profile" }
                            a href="/buy" { "Vendor listings" }
                        }
                        p class="ed-method" { "Figures from public pool and project pages · no paid listings · " a href="/about" { "ownership and method" } }
                    }
                    aside class="ed-hero-machine" data-machine-carousel role="region" aria-roledescription="carousel" aria-label="Featured Equihash hardware" {
                        div class="ed-machine-slides" {
                            @for (i, (m, image, alt, width, height)) in featured.iter().enumerate() {
                                @let calculator = format!("/calculator?hashrate={}&watts={}", fmt::opt_num(m.hashrate_ksol), fmt::opt_num(m.watts));
                                @let has_listing = d.listings.iter().any(|l| l.miner_id == m.id);
                                article class={"ed-machine-slide" @if i == 0 { " is-active" }} data-machine-slide aria-hidden=(if i == 0 { "false" } else { "true" }) role="group" aria-roledescription="slide" aria-label={(i + 1) " of " (featured.len()) ": " (m.maker) " " (m.model)} {
                                    div class="ed-hero-machine-image" {
                                        img src=(image) alt=(alt) width=(width) height=(height) loading=[(i > 0).then_some("lazy")] decoding="async";
                                    }
                                    div class="ed-hero-machine-info" {
                                        p { (m.maker.to_uppercase()) " · EQUIHASH " (m.equihash) }
                                        h2 { (m.model) }
                                        dl { div { dt { "Hashrate" } dd { (fmt::opt_num(m.hashrate_ksol)) " kSol/s" } } div { dt { "Power" } dd { (fmt::int(m.watts)) " W" } } }
                                        nav aria-label={(m.model) " actions"} {
                                            a href={"/hardware/" (m.id)} { "Specifications" }
                                            a href=(calculator) { "Calculate" }
                                            @if has_listing { a href={"/buy?machine=" (m.id)} { "Where to buy" } }
                                            @else { a href="/hardware" { "Compare hardware" } }
                                        }
                                    }
                                }
                            }
                        }
                        @if featured.len() > 1 {
                            nav class="ed-machine-switcher" data-machine-controls aria-label="Choose featured machine" hidden {
                                button type="button" data-machine-prev aria-label="Previous machine" { "←" }
                                span class="ed-machine-position" aria-hidden="true" { strong data-machine-current { "1" } " / " (featured.len()) }
                                button class="ed-machine-toggle" type="button" data-machine-toggle aria-label="Pause carousel" aria-pressed="false" { "Pause" }
                                button type="button" data-machine-next aria-label="Next machine" { "→" }
                            }
                        }
                    }
                }
            }

            section class="ed-snapshot" aria-label="Current directory coverage" {
                div class="wrap" {
                    p {
                        a class="ed-snapshot-link" href="/sources" {
                            span { "Updated" }
                            time class="ago" datetime=[d.last_updated.as_deref()] { (fmt::utc(d.last_updated.as_deref())) }
                            em { "See sources" }
                        }
                    }
                    dl {
                        div { dt { a class="ed-snapshot-link" href="/coins" { "Active networks" } } dd { (active) } }
                        div { dt { a class="ed-snapshot-link" href="/pools" { "Pool listings" } } dd { (pools) } }
                        div { dt { a class="ed-snapshot-link" href="/coins#equihash-200-9" { "200,9 coins" } } dd { (z15) } }
                        div { dt { a class="ed-snapshot-link" href="/pools?coin=zcash&merged=1#pools" { "Merged-mining pools" } } dd { (merged_pools) } }
                    }
                }
            }

            section class="wrap ed-directory" aria-labelledby="coins-title" {
                header class="ed-section-head" {
                    div { h2 id="coins-title" { "Active Equihash networks" } p { "Coins are grouped by their exact Equihash parameters. A Z15 works on 200,9; other parameter sets need different hardware." } }
                    a href="/coins" { "All active and historical coins →" }
                }
                (coin_rows(d, true, Some(4)))
            }

            (crate::views::hashpower::home_market(d))

            section class="ed-merge" aria-labelledby="merge-feature-title" {
                div class="wrap ed-merge-grid" {
                    div class="ed-merge-copy" {
                        p class="ed-section-index" { "ZEC + WEC / MERGED MINING" }
                        h2 id="merge-feature-title" { "Mine Zcash and Wcash together." }
                        p { "On a pool that supports merged mining, a Z15 does the same work it would do for Zcash alone. The pool also submits qualifying work to Wcash and pays WEC separately." }
                        div class="ed-merge-actions" {
                            a class="ed-light-button" href="/pools?coin=zcash&merged=1#pools" { "See supporting pools" }
                            a href="/merged-mining" { "How it works →" }
                        }
                        p class="ed-disclosure" { "The same person maintains Wcash and equihash.com. Wcash receives work only from pools that support it." }
                    }
                    div class="ed-flow" role="img" aria-label="Z15 Pro work goes to a participating pool, then to independent Zcash and Wcash targets" {
                        div class="ed-flow-source" { small { "EQUIHASH 200,9" } strong { "Z15 PRO" } span { "same work" } }
                        i aria-hidden="true" {}
                        div class="ed-flow-pool" { small { "PARTICIPATING" } strong { "POOL" } span { "routes shares" } }
                        i aria-hidden="true" {}
                        div class="ed-flow-targets" {
                            div { small { "PARENT" } strong { "ZCASH" } span { "ZEC" } }
                            div { small { "AUXILIARY" } strong { "WCASH" } span { "WEC" } }
                        }
                    }
                }
            }

            aside class="wrap ed-contribute" {
                div { h2 { "Run a pool, coin project or ASIC shop?" } p { "Send a public page or API. Listings and corrections are free." } }
                div { a class="ed-dark-button" href="/contribute" { "Send a listing or correction" } a href="https://t.me/EquihashCom" rel="noopener" { "Telegram →" } }
            }
        }
    })
}

pub fn coins(d: &Data) -> Markup {
    layout(d, Page {
        title: "Equihash coins by parameter set",
        description: "Equihash coins grouped by their exact n,k parameters, with matching hardware, mining pools and network figures.",
        path: "/coins",
        nav: "coins",
    }, html! {
        div class="wrap page directory-page" {
            header class="page-head split-head" {
                div { p class="eyebrow" { "COIN DIRECTORY" } h1 { "Equihash coins" } p class="lede" { "Choose the parameter set first. Hardware built for 200,9 cannot mine a 144,5 or 192,7 network." } }
                (search_form("", "Search the directory"))
            }
            (coin_rows(d, false, None))
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
                        div class="section-head" { div { h2 id="pool-title" { "Mining pools" } p { "Up to 12 active listings are shown here. Open the full table for fees, payout rules, regions and sources." } } a href={"/pools?coin=" (c.id) "#pools"} { "Full pool comparison →" } }
                        @if pools.is_empty() {
                            div class="empty-state" { h3 { "No active pool is listed" } p { "We have not found an active public pool for this coin. If one exists, send its pool page or API." } a href="/contribute#pool" { "Submit a pool" } }
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
                            div class="section-head" { div { h2 id="qa-title" { "Common questions" } p { "Answers for this coin." } } a href="/contribute#question" { "Ask or improve an answer" } }
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
                            @if miners.is_empty() { p class="na" { "None of the manufacturer specifications we have checked match this parameter set." } }
                            @for m in miners.iter().take(5) { a class="side-row" href={"/hardware/" (m.id)} { span { (m.maker) " " strong { (m.model) } } small { (fmt::opt_num(m.hashrate_ksol)) " kSol/s · " (fmt::int(m.watts)) " W" } } }
                            a class="more-link" href="/hardware" { "All hardware →" }
                        }
                        section class="side-card action-card" { h2 { "Calculate output" } p { "The calculator fills in this coin's network figures. Enter your fee, electricity price and miner." } a class="btn" href={"/calculator?coin=" (c.id)} { "Open calculator" } }
                        section class="side-card" { h2 { "Sources" } p { "Each network and pool figure has a public source and a time checked. The same figures are available as JSON." } a href="/sources" { "Method and limitations" } br; a href="/data/network.json" { "Download network data" } }
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
                    p class="lede" { "Hashrate and power from the manufacturer page, followed by coins that use the same Equihash parameters." }
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
                        @if d.listings.iter().any(|l| l.miner_id == m.id) { section class="side-card action-card" { h2 { "Where to buy" } p { "Compare the price, stock, VAT and shipping shown on each vendor’s product page." } a class="btn" href={"/buy?machine=" (m.id)} { "View vendor listings" } } }
                        @if m.equihash == "200,9" { section class="side-card" { h2 { "Merged mining" } p { "Some Zcash pools can reuse the same work for Wcash without splitting Zcash hashrate." } a href="/merged-mining" { "How ZEC + WEC mining works →" } } }
                        section class="side-card" { h2 { "Setup path" } ol { li { "Choose one exact-match coin." } li { "Compare active pools and regions." } li { "Verify the stratum address at the pool." } li { "Enter worker and wallet details in the miner UI." } li { "Check accepted shares and payout threshold." } } }
                    }
                }
            }
        },
    )
}

pub fn guides(d: &Data) -> Markup {
    layout(d, Page { title: "Equihash mining guides", description: "Guides to Equihash parameters, Z15 Pro setup, pool selection, mining costs and merged mining.", path: "/guides", nav: "guides" }, html! {
        div class="wrap page guides-page" {
            header class="page-head" { p class="eyebrow" { "SETUP AND REFERENCE" } h1 { "Equihash guides" } p class="lede" { "Set up a Z15 Pro, compare pools, check mining costs or learn how Zcash + Wcash merged mining works." } }
            div class="guide-grid featured-guides" {
                article { p class="step" { "START HERE · 7 MIN" } h2 { a href="/hardware/antminer-z15-pro" { "Z15 Pro: from power-on to a compatible pool" } } p { "Check exact parameters, choose a coin, compare regions and fees, calculate power cost, and verify accepted shares." } }
                article { p class="step" { "POOL CHOICE · 5 MIN" } h2 { a href="/pools?coin=zcash#pools" { "How to compare Zcash pools" } } p { "Read hashrate, concentration, PPLNS/PPS variants, fees, minimum payouts, regions and observation times." } }
                article { p class="step" { "MERGED MINING · 10 MIN" } h2 { a href="/merged-mining" { "How Zcash + Wcash merged mining works" } } p { "See what the miner sends, what the pool adds and how qualifying work reaches Zcash and Wcash." } }
                article { p class="step" { "PARAMETERS · 4 MIN" } h2 { a href="/coins" { "Why Equihash n,k decides compatibility" } } p { "Understand why 200,9 hardware cannot mine every coin carrying the Equihash name." } }
                article { p class="step" { "ECONOMICS · 5 MIN" } h2 { a href="/calculator" { "Estimate revenue and electricity cost" } } p { "Start with the listed network figures, then enter your hashrate, watts, fee and electricity price." } }
                article { p class="step" { "VERIFY · 6 MIN" } h2 { a href="/sources" { "Where the pool and network figures come from" } } p { "See which pages and APIs are read, when each field was checked and how missing values are shown." } }
            }
            aside class="listing-callout" { div { p class="eyebrow" { "MISSING A QUESTION?" } h2 { "Couldn’t find your question?" } p { "Send the miner, pool or coin name and a link that helps answer it. We will add the answer to the relevant page." } } a class="btn" href="/contribute#question" { "Ask a question" } }
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
