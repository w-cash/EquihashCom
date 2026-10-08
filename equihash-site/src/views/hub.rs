//! Pages for finding a coin, pool or machine and following it to the relevant data.
//! The full pool table remains in `home.rs`.

use crate::data::{Coin, Data, Miner, Pool};
use crate::fmt;
use crate::views::home::schemes_text;
use crate::views::layout::{layout, Page};
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

fn coin_table_row(c: &Coin) -> Markup {
    html! {
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

fn lead_coin_ids(group: &str) -> &'static [&'static str] {
    match group {
        "Equihash 200,9" => &["zcash", "piratechain", "wcash"],
        "Equihash 192,7" => &["ycash", "zclassic"],
        "Equihash 144,5" => &["bitcoingold"],
        _ => &[],
    }
}

fn coin_group(group: &str, coins: &[&Coin]) -> Markup {
    let lead_ids = lead_coin_ids(group);
    let mut lead: Vec<&Coin> = lead_ids
        .iter()
        .filter_map(|id| coins.iter().copied().find(|c| c.id == *id))
        .collect();
    if lead.is_empty() {
        lead.extend(coins.iter().take(1).copied());
    }
    let other: Vec<&Coin> = coins
        .iter()
        .copied()
        .filter(|c| !lead.iter().any(|shown| shown.id == c.id))
        .collect();
    let suffix = coins
        .first()
        .and_then(|c| c.nk())
        .map(|(n, k)| format!("{n}-{k}"))
        .unwrap_or_else(|| "unknown".into());
    let section_id = format!("equihash-{suffix}");
    let more_id = format!("coin-more-{suffix}");
    html! {
        section class="coin-group" id=(section_id) {
            div class="section-head compact" {
                div {
                    h2 { (group) }
                    p {
                        @if other.is_empty() {
                            (coins.len()) @if coins.len() == 1 { " listed network" } @else { " listed networks" }
                        } @else {
                            (lead.len()) " shown first · " (other.len()) " more"
                        }
                    }
                }
                @if group == "Equihash 200,9" { a href="/asics/antminer-z15-pro" { "Z15 Pro compatible →" } }
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
                    tbody { @for c in &lead { (coin_table_row(c)) } }
                    @if !other.is_empty() {
                        tbody class="coin-more-control" hidden {
                            tr { td colspan="6" {
                                button type="button" class="coin-more-button" data-coin-more data-count=(other.len()) aria-expanded="false" aria-controls=(more_id) {
                                    "Show " (other.len()) " other " @if other.len() == 1 { "network" } @else { "networks" }
                                }
                            } }
                        }
                        tbody class="coin-secondary-rows" id=(more_id) { @for c in &other { (coin_table_row(c)) } }
                    }
                }
            }
        }
    }
}

fn coin_rows(d: &Data, active_only: bool) -> Markup {
    let groups = d.param_groups(active_only);
    let (priority, other): (Vec<_>, Vec<_>) = groups.into_iter().partition(|(group, _)| {
        matches!(
            group.as_str(),
            "Equihash 200,9" | "Equihash 192,7" | "Equihash 144,5"
        )
    });
    html! {
        @for (group, coins) in priority { (coin_group(&group, &coins)) }
        @if !other.is_empty() {
            details class="coin-other-params" {
                summary { "Other parameter sets" span { (other.len()) } }
                div class="coin-other-params-body" { @for (group, coins) in other { (coin_group(&group, &coins)) } }
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
    let zcash = d.coin("zcash");
    let zcash_network = zcash
        .map(|c| {
            fmt::hashrate(
                c.network.hashrate,
                c.network.unit.as_deref().unwrap_or("Sol/s"),
            )
        })
        .unwrap_or_else(|| "n/a".into());
    let zcash_reported = zcash
        .map(|c| fmt::reported(c).0)
        .unwrap_or_else(|| "n/a".into());
    let zcash_observed = zcash
        .and_then(|c| c.network.hashrate_observed_at.as_deref())
        .map(|v| fmt::utc(Some(v)))
        .unwrap_or_else(|| "n/a".into());
    let mut zcash_pools: Vec<&Pool> = d
        .live_pools()
        .filter(|p| p.coin_id == "zcash" && p.hashrate.unwrap_or(0.0) > 0.0)
        .collect();
    zcash_pools.sort_by(|a, b| {
        b.hashrate
            .partial_cmp(&a.hashrate)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    zcash_pools.truncate(3);
    // This seller rates its current Z15 Pro offer at 860 kSol/s and 2,847 W. Keep it distinct
    // from BITMAIN's manufacturer specification (840 kSol/s typical), which is the next slide.
    let offer_860 = d
        .listings
        .iter()
        .find(|l| l.id == "hashlabs-antminer-z15-pro-860");
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
            "antminer-z15j",
            "/static/shop/machines/antminer-z15.8dc9fd7a98.webp",
            "Bitmain Antminer Z15 series ASIC miner",
            924,
            1000,
        ),
        (
            "antminer-z15e",
            "/static/shop/machines/antminer-z15.8dc9fd7a98.webp",
            "Bitmain Antminer Z15 series ASIC miner",
            924,
            1000,
        ),
        (
            "innosilicon-a9pp-zmaster",
            "/static/shop/machines/innosilicon-a9pp-zmaster.e4b128723d.webp",
            "Innosilicon A9++ ZMaster ASIC miner",
            1000,
            680,
        ),
        (
            "antminer-z11",
            "/static/shop/machines/antminer-z11.1c5e36ca87.webp",
            "Bitmain Antminer Z11 ASIC miner",
            1200,
            630,
        ),
        (
            "antminer-z9",
            "/static/shop/machines/antminer-z9.4e480dbe7c.webp",
            "Bitmain Antminer Z9 ASIC miner",
            1140,
            586,
        ),
        (
            "antminer-z9-mini",
            "/static/shop/machines/antminer-z9-mini.712b4180db.webp",
            "Bitmain Antminer Z9 Mini ASIC miner",
            1140,
            525,
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
    let slide_count = featured.len() + usize::from(offer_860.is_some());
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
                        dl class="ed-hero-signal" aria-label="Latest Zcash mining snapshot" {
                            div { dt { "Zcash network" } dd { (&zcash_network) } }
                            div { dt { "Listed pools report" } dd { (&zcash_reported) } }
                            div { dt { "Observed" } dd { (&zcash_observed) } }
                        }
                        (search_form("", "Search coins, pools or hardware"))
                        div class="ed-hero-links" {
                            a class="ed-primary" href="/pools" { "Compare pools" }
                            a href="/zcash-mining" { "Zcash mining guide" }
                            a href="/hashpower" { "Hashpower market" }
                            a href="/asics/antminer-z15-pro" { "Z15 Pro profile" }
                            a href="/vendors" { "Vendor directory" }
                        }
                        p class="ed-method" { "Figures from public pool and project pages · no paid listings · " a href="/about" { "ownership and method" } }
                    }
                    aside class="ed-hero-machine" data-machine-carousel role="region" aria-roledescription="carousel" aria-label="Featured Equihash hardware" {
                        @if slide_count > 1 {
                            nav class="ed-machine-switcher" data-machine-controls aria-label="Featured hardware controls" hidden {
                                button type="button" data-machine-prev aria-label="Previous machine" { "←" }
                                span class="ed-machine-position" aria-hidden="true" { strong data-machine-current { "1" } " / " (slide_count) }
                                button class="ed-machine-toggle" type="button" data-machine-toggle aria-label="Pause carousel" aria-pressed="false" { "Ⅱ" }
                                button type="button" data-machine-next aria-label="Next machine" { "→" }
                            }
                            button class="ed-machine-peek ed-machine-peek-prev" type="button" data-machine-peek-prev aria-label="Show previous machine" hidden {
                                span aria-hidden="true" { "‹" }
                            }
                            button class="ed-machine-peek ed-machine-peek-next" type="button" data-machine-peek-next aria-label="Show next machine" hidden {
                                span aria-hidden="true" { "›" }
                            }
                        }
                        div class="ed-machine-slides" {
                            @if let Some(offer) = offer_860 {
                                article class="ed-machine-slide is-active" data-machine-slide aria-hidden="false" role="group" aria-roledescription="slide" aria-label={"1 of " (slide_count) ": Antminer Z15 Pro 860"} {
                                    header class="ed-machine-card-top" { span { "Featured hardware" } span class="ed-machine-record" { "Vendor listing" } }
                                    div class="ed-hero-machine-image" {
                                        img src="/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp" alt="Bitmain Antminer Z15 Pro 860 kSol/s offer" width="1200" height="1200" decoding="async";
                                    }
                                    div class="ed-hero-machine-info" {
                                        p { "VENDOR-RATED · EQUIHASH 200,9" }
                                        h2 { "Antminer Z15 Pro 860" }
                                        p class="ed-machine-note" { "Offer-page rating; BITMAIN publishes 840 kSol/s as the typical specification." }
                                        dl { div { dt { "Hashrate" } dd { (fmt::opt_num(offer.shop_hashrate_ksol)) " kSol/s" } } div { dt { "Power" } dd { "2,847 W" } } }
                                        nav aria-label="Antminer Z15 Pro 860 actions" {
                                            a href="/asics/antminer-z15-pro" { "Official 840 spec" }
                                            a href="/calculator?hashrate=860&watts=2847" { "Calculate" }
                                            a href="/vendors?machine=antminer-z15-pro" { "Vendor records" }
                                        }
                                    }
                                }
                            }
                            @for (j, (m, image, alt, width, height)) in featured.iter().enumerate() {
                                @let i = j + usize::from(offer_860.is_some());
                                @let calculator = format!("/calculator?hashrate={}&watts={}", fmt::opt_num(m.hashrate_ksol), fmt::opt_num(m.watts));
                                @let has_listing = d.listings.iter().any(|l| l.miner_id == m.id);
                                article class={"ed-machine-slide" @if i == 0 { " is-active" }} data-machine-slide aria-hidden=(if i == 0 { "false" } else { "true" }) role="group" aria-roledescription="slide" aria-label={(i + 1) " of " (slide_count) ": " (m.maker) " " (m.model)} {
                                    header class="ed-machine-card-top" { span { "Featured hardware" } span class="ed-machine-record" { "Manufacturer spec" } }
                                    div class="ed-hero-machine-image" {
                                        img src=(image) alt=(alt) width=(width) height=(height) loading=[(i > 0).then_some("lazy")] decoding="async";
                                    }
                                    div class="ed-hero-machine-info" {
                                        p { (m.maker.to_uppercase()) " · EQUIHASH " (m.equihash) }
                                        h2 { (m.model) }
                                        dl { div { dt { "Hashrate" } dd { (fmt::opt_num(m.hashrate_ksol)) " kSol/s" } } div { dt { "Power" } dd { (fmt::int(m.watts)) " W" } } }
                                        nav aria-label={(m.model) " actions"} {
                                            a href={"/asics/" (m.id)} { "Specifications" }
                                            a href=(calculator) { "Calculate" }
                                            @if has_listing { a href={"/vendors?machine=" (m.id)} { "Vendor records" } }
                            @else { a href="/asics" { "Compare ASICs" } }
                                        }
                                    }
                                }
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

            section class="wrap ed-workspace" aria-labelledby="workspace-title" {
                header class="ed-section-head" {
                    div {
                        p class="ed-section-index" { "ZCASH / START HERE" }
                        h2 id="workspace-title" { "Choose a pool, then check the power cost." }
                        p { "The three largest Zcash pool listings with reported hashrate. Check the full comparison before pointing your hardware." }
                    }
                    a href="/pools?coin=zcash#pools" { "Compare all Zcash pools →" }
                }
                div class="ed-workspace-grid" {
                    div class="ed-workspace-pools" aria-label="Largest listed Zcash pool rows" {
                        @for (i, p) in zcash_pools.iter().enumerate() {
                            a href={"/pool/" (&p.slug)} {
                                span class="ed-workspace-rank" { (i + 1) }
                                span class="ed-workspace-name" { (logo::chip(&p.logo, &p.name, At::Row, true)) strong { (&p.name) } }
                                span { small { "Hashrate" } b class="mono" { (fmt::hashrate(p.hashrate, p.hashrate_unit.as_deref().unwrap_or("Sol/s"))) } }
                                span { small { "Fee" } b class="mono" { (fmt::fee(p.fee_range())) } }
                                span { small { "Payout" } b { @if p.payout_schemes.is_empty() { "n/a" } @else { (p.payout_schemes.join(", ")) } } }
                                span class="ed-workspace-arrow" aria-hidden="true" { "→" }
                            }
                        }
                    }
                    aside class="ed-workspace-calc" {
                        p class="ed-section-index" { "Z15 PRO / 840 kSol/s" }
                        h3 { "What does 2,780 W cost where you mine?" }
                        p { "Start with the manufacturer-rated Z15 Pro figures, then enter your electricity price and pool fee." }
                        dl {
                            div { dt { "Hashrate" } dd { "840 kSol/s" } }
                            div { dt { "Power" } dd { "2,780 W" } }
                        }
                        a class="ed-dark-button" href="/calculator?hashrate=840&watts=2780" { "Open the calculator" }
                    }
                }
            }

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

            section class="wrap ed-directory" aria-labelledby="coins-title" {
                header class="ed-section-head" {
                    div { h2 id="coins-title" { "Active Equihash networks" } p { "Coins are grouped by their exact Equihash parameters. A Z15 works on 200,9; other parameter sets need different hardware." } }
                    a href="/coins" { "All active and historical coins →" }
                }
                (coin_rows(d, true))
            }

            (crate::views::hashpower::home_market(d))

            (crate::views::research::home_note(d))

            aside class="wrap ed-contribute" {
                div { h2 { "Run a pool, coin project or ASIC shop?" } p { "Send a public page or API. Listings and corrections are free." } }
                div { a class="ed-dark-button" href="/contribute" { "Send a listing or correction" } }
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
            (coin_rows(d, false))
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
    let is_zcash = c.id == "zcash";
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
                    @if is_zcash { p { "New to Zcash proof of work? " a href="/zcash-mining" { "Read the mining setup, pool and privacy guide" } "." } }
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
                        @if is_zcash { (crate::views::research::coin_note(d)) }
                        section class="side-card" { h2 { "Compatible hardware" }
                            @if miners.is_empty() { p class="na" { "None of the manufacturer specifications we have checked match this parameter set." } }
                            @for m in miners.iter().take(5) { a class="side-row" href={"/asics/" (m.id)} { span { (m.maker) " " strong { (m.model) } } small { (fmt::opt_num(m.hashrate_ksol)) " kSol/s · " (fmt::int(m.watts)) " W" } } }
                            a class="more-link" href="/asics" { "All ASICs →" }
                        }
                        section class="side-card action-card" { h2 { "Calculate output" } p { "The calculator fills in this coin's network figures. Enter your fee, electricity price and miner." } a class="btn" href={"/calculator?coin=" (c.id)} { "Open calculator" } }
                        section class="side-card" { h2 { "Sources" } p { "Each network and pool figure has a public source and a time checked. The same figures are available as JSON." } a href="/sources" { "Method and limitations" } br; a href="/data/network.json" { "Download network data" } }
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
                article { p class="step" { "ZCASH MINING · 8 MIN" } h2 { a href="/zcash-mining" { "How to mine Zcash with an Equihash ASIC" } } p { "Choose compatible hardware, compare pool evidence, configure a worker and separate mining from Zcash transaction privacy." } }
                article { p class="step" { "START HERE · 7 MIN" } h2 { a href="/asics/antminer-z15-pro" { "Z15 Pro: from power-on to a compatible pool" } } p { "Check exact parameters, choose a coin, compare regions and fees, calculate power cost, and verify accepted shares." } }
                article { p class="step" { "POOL CHOICE · 5 MIN" } h2 { a href="/pools#pools" { "How to compare Zcash pools" } } p { "Read hashrate, concentration, PPLNS/PPS variants, fees, minimum payouts, regions and observation times." } }
                article { p class="step" { "MERGED MINING · 10 MIN" } h2 { a href="/merged-mining" { "How Zcash + Wcash merged mining works" } } p { "See what the miner sends, what the pool adds and how qualifying work reaches Zcash and Wcash." } }
                article { p class="step" { "PARAMETERS · 4 MIN" } h2 { a href="/coins" { "Why Equihash n,k decides compatibility" } } p { "Understand why 200,9 hardware cannot mine every coin carrying the Equihash name." } }
                article { p class="step" { "ECONOMICS · 5 MIN" } h2 { a href="/calculator" { "Estimate revenue and electricity cost" } } p { "Start with the listed network figures, then enter your hashrate, watts, fee and electricity price." } }
                article { p class="step" { "VERIFY · 6 MIN" } h2 { a href="/sources" { "Where the pool and network figures come from" } } p { "See which pages and APIs are read, when each field was checked and how missing values are shown." } }
            }
            aside class="listing-callout" { div { p class="eyebrow" { "MISSING A QUESTION?" } h2 { "Couldn’t find your question?" } p { "Send the miner, pool or coin name and a link that helps answer it. We will add the answer to the relevant page." } } a class="btn" href="/contribute#question" { "Ask a question" } }
        }
    })
}

pub fn zcash_mining(d: &Data) -> Markup {
    let zcash = d.coin("zcash");
    let z15 = d.miners.iter().find(|m| m.id == "antminer-z15-pro");
    let pools = zcash.map(|c| coin_pools(d, c)).unwrap_or_default();
    let network = zcash
        .map(|c| {
            fmt::hashrate(
                c.network.hashrate,
                c.network.unit.as_deref().unwrap_or("Sol/s"),
            )
        })
        .unwrap_or_else(|| "n/a".into());
    layout(d, Page {
        title: "Zcash mining guide: pools, Z15 Pro setup and costs",
        description: "A source-backed Zcash mining guide: Equihash 200,9 hardware, Antminer Z15 Pro specifications, pool selection, setup, electricity costs and merged mining.",
        path: "/zcash-mining",
        nav: "guides",
    }, html! {
        div class="wrap page narrow prose zcash-guide" {
            p class="crumb" { a href="/guides" { "All guides" } " / Zcash mining" }
            header class="page-head" {
                p class="eyebrow" { "ZCASH · EQUIHASH 200,9" }
                h1 { "How to mine Zcash" }
                p class="lede" { "Zcash uses Equihash 200,9 proof of work. A compatible ASIC such as the Antminer Z15 Pro sends shares to a Zcash pool, and the pool pays according to its published fee and payout rules." }
                p class="small" { "Maintained by " a href="https://x.com/MykytaSamardak" rel="me noopener" { "Mykyta Samardak" } " · network and pool data refreshed " time datetime=[d.last_updated.as_deref()] { (fmt::utc(d.last_updated.as_deref())) } " · " a href="/sources" { "method and sources" } }
            }

            section aria-labelledby="zcash-answer" {
                h2 id="zcash-answer" { "What you need" }
                p { "You need an Equihash 200,9 miner, a Zcash address accepted by the pool, a pool endpoint near the miner, and enough electrical capacity for the machine. A Bitcoin SHA-256 ASIC cannot mine Zcash, and a Zcash Z15 cannot mine Bitcoin: the proof-of-work algorithms are different." }
                dl class="fact-strip" {
                    div { dt { "Algorithm" } dd { "Equihash 200,9" } }
                    div { dt { "Network estimate" } dd { (network) } }
                    div { dt { "Listed pools" } dd { (pools.len()) } }
                    div { dt { "Z15 Pro typical" } dd { @if let Some(m) = z15 { (fmt::opt_num(m.hashrate_ksol)) " kSol/s" } @else { "840 kSol/s" } } }
                    div { dt { "Z15 Pro power" } dd { @if let Some(m) = z15 { (fmt::int(m.watts)) " W" } @else { "2,780 W" } } }
                }
                p class="small" { "Network and pool values are snapshots, not forecasts. Hardware figures come from the linked manufacturer specification." }
            }

            section aria-labelledby="zcash-steps" {
                h2 id="zcash-steps" { "Zcash mining setup, step by step" }
                ol {
                    li { strong { "Check the machine and power circuit. " } "Confirm Equihash 200,9 compatibility, input voltage, plug, airflow and the published wattage. " a href="/asics/antminer-z15-pro" { "Open the Z15 Pro specification record" } "." }
                    li { strong { "Create a Zcash address. " } "Check the pool's own payout instructions before choosing an address type. Mining secures Zcash consensus; transaction privacy depends on the wallet and whether funds move through shielded addresses." }
                    li { strong { "Compare pools. " } "Look at reported hashrate, network concentration, payout method, fee, minimum payout, server region and the age of each source. No single pool is best for every miner. " a href="/pools#pools" { "Compare Zcash pools" } "." }
                    li { strong { "Copy the pool's current endpoint. " } "Take the stratum host, port, worker format and password from the pool's own setup page. Do not copy an endpoint from an undated forum post." }
                    li { strong { "Configure the miner. " } "Enter the pool endpoint, worker or wallet identifier and password in the miner interface. Save the primary pool and at least one fallback operated independently." }
                    li { strong { "Verify accepted shares. " } "After startup, check the miner and pool dashboards for accepted shares, rejects, hashrate and payout progress. Investigate sustained rejects before treating the machine as stable." }
                    li { strong { "Run the cost estimate. " } "Enter the machine's actual wall power and your delivered electricity rate. Hardware cost, tax, downtime, pool luck, stale shares, hosting and import charges remain outside the basic estimate. " a href="/calculator?coin=zcash&hashrate=840&watts=2780" { "Open the Zcash calculator" } "." }
                }
            }

            section aria-labelledby="zcash-pools" {
                div class="section-head" { div { h2 id="zcash-pools" { "Zcash pool snapshot" } p { "These are the largest currently listed rows by reported hashrate. Open the comparison for sources, payout details and all active rows." } } a href="/pools#pools" { "Full comparison →" } }
                @if pools.is_empty() {
                    p { "No active Zcash pool is currently listed." }
                } @else {
                    div class="table-scroll" { table class="data mini" {
                        thead { tr { th scope="col" { "Pool" } th class="num" scope="col" { "Reported hashrate" } th scope="col" { "Payout" } th class="num" scope="col" { "Fee" } th scope="col" { "Region" } } }
                        tbody { @for p in pools.iter().take(8) { tr {
                            td { a href={"/pool/" (p.slug)} { strong { (p.name) } } }
                            td class="num" { (fmt::hashrate(p.hashrate, p.hashrate_unit.as_deref().unwrap_or("Sol/s"))) }
                            td { (schemes_text(p)) }
                            td class="num" { (fmt::fee(p.fee_range())) }
                            td { (p.region.as_deref().unwrap_or("n/a")) }
                        } } }
                    } }
                }
            }

            section aria-labelledby="zcash-z15" {
                h2 id="zcash-z15" { "Z15 Pro: 840 kSol/s manufacturer specification" }
                p { "BITMAIN publishes 840 kSol/s typical and 2,780 W typical for the Antminer Z15 Pro under its stated operating conditions. Some retailer pages advertise 860 kSol/s; equihash.com keeps that seller claim separate from the manufacturer's typical specification." }
                p { a href="/asics/antminer-z15-pro" { "Read the sourced Z15 Pro specifications" } " · " a href="/vendors?machine=antminer-z15-pro" { "Review vendor records" } }
            }

            (crate::views::research::guide_note(d))

            section aria-labelledby="zcash-privacy" {
                h2 id="zcash-privacy" { "Mining and Zcash privacy are separate" }
                p { "Proof of work orders blocks and secures consensus. Zcash privacy comes from shielded transactions and compatible wallet software; mining with Equihash does not by itself make a payout private. Check the address types supported by your wallet and pool before mining." }
            }

            section aria-labelledby="zcash-merged" {
                h2 id="zcash-merged" { "Merged mining" }
                p { "A participating Zcash pool can reuse qualifying Equihash work for Wcash without splitting the miner's Zcash hashrate. Wcash is secured only by work routed through participating pools, not by all Zcash hashrate. Pools account for ZEC and WEC separately." }
                p { a href="/merged-mining" { "Read the Zcash + Wcash merged-mining guide" } }
            }

            aside class="listing-callout" { div { h2 { "Check the live record" } p { "Pool conditions, network hashrate and coin price change. Use the current directory and then verify the chosen pool's own page before pointing a miner." } } a class="btn" href="/coin/zcash" { "Open the Zcash mining record" } }
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
    let vendor_research: Vec<_> = if needle.is_empty() {
        Vec::new()
    } else {
        d.vendor_research
            .iter()
            .filter(|v| {
                v.trust_tier != "N/A"
                    && matches(
                        &needle,
                        &[
                            &v.vendor,
                            &v.country,
                            &v.region,
                            &v.vendor_type,
                            &v.equihash_z15,
                        ],
                    )
            })
            .take(30)
            .collect()
    };
    let vendors: Vec<_> = if needle.is_empty() || !vendor_research.is_empty() {
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
    let industry: Vec<(&str, &str, &str)> = if needle.is_empty() {
        Vec::new()
    } else {
        [
            (
                "/industry/cypherpunk-zcash-mining",
                "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet",
                "Cypherpunk CYPH Zcash mining 4902 Z15 Pro hashrate fleet Winklevoss",
            ),
            (
                "/industry/grayscale-zcash-etf",
                "From OTC trust to ZCSH: Grayscale’s Zcash vehicle",
                "Grayscale ZCSH Zcash ETF trust NYSE Arca institutional capital custody",
            ),
            (
                "/industry/winklevoss-zcash-etf",
                "WINK filed: the preliminary Winklevoss Zcash ETF",
                "Winklevoss WINK Zcash ETF Gemini Cypherpunk filing Nasdaq",
            ),
        ]
        .into_iter()
        .filter(|(_, title, terms)| matches(&needle, &[title, terms, "industry briefing"]))
        .collect()
    };
    let count = coins.len()
        + miners.len()
        + pools.len()
        + vendor_research.len()
        + vendors.len()
        + industry.len()
        + usize::from(merged)
        + usize::from(buying);
    layout(d, Page { title: "Search Equihash coins, pools, hardware, vendors and industry briefings", description: "Search the equihash.com directory for coins, mining pools, hardware, vendors, guides and sourced industry briefings.", path: "/search", nav: "" }, html! {
        div class="wrap page search-page" {
            header class="page-head" { h1 { "Search equihash.com" } (search_form(raw, "Coin, pool, hardware or parameter set")) }
            @if raw.is_empty() { div class="empty-state" { h2 { "Search the whole directory" } p { "Try “Z15 Pro”, “Zcash”, “Wcash”, “merged mining”, “Cypherpunk”, “Grayscale”, a vendor, pool or region." } } }
            @else if count == 0 { div class="empty-state" { h2 { "No results for “" (raw) "”" } p { "Try a coin symbol, miner model, exact parameter set or shorter pool name." } a href="/contribute" { "Suggest a missing listing" } } }
            @else {
                p class="result-count" { (count) " result" @if count != 1 { "s" } " for “" (raw) "”" }
                div class="search-results" {
                    @if !coins.is_empty() { section { h2 { "Coins" } div class="result-list" { @for c in coins { a href={"/coin/" (c.id)} { span { (logo::chip(&c.logo, &c.name, At::List, true)) strong { (c.name) } " " span class="sym" { (c.symbol) } } small { "Equihash " (c.params()) " · " (c.pool_count) " pools" } } } } } }
                    @if !miners.is_empty() { section { h2 { "ASICs" } div class="result-list" { @for m in miners { a href={"/asics/" (m.id)} { span { strong { (m.maker) " " (m.model) } } small { "Equihash " (m.equihash) " · " (fmt::opt_num(m.hashrate_ksol)) " kSol/s" } } } } } }
                    @if !pools.is_empty() { section { h2 { "Pools" } div class="result-list" { @for p in pools { a href={"/pool/" (p.slug)} { span { (logo::chip(&p.logo, &p.name, At::List, true)) strong { (p.name) } } small { (p.coin_label) " · " (p.region.as_deref().unwrap_or("region n/a")) } } } } } }
                    @if !vendor_research.is_empty() { section { h2 { "Vendor research" } div class="result-list" { @for v in vendor_research { a href={"/vendors#vendor-" (&v.id)} { span { strong { (&v.vendor) } @if v.trust_tier == "D" { " · Warning record" } @else { " · Tier " (&v.trust_tier) } } small { (&v.country) " · " (&v.region) " · " (&v.equihash_z15) } } } } } }
                    @if !vendors.is_empty() { section { h2 { "Vendors" } div class="result-list" { @for v in vendors { a href={"/vendors/" (v.slug)} { span { strong { (v.name) } } small { (v.region_focus.as_deref().unwrap_or("Region not stated")) " · " (v.regions.join(", ")) } } } } } }
                    @if !industry.is_empty() { section { h2 { "Industry" } div class="result-list" { @for (href, title, _) in industry { a href=(href) { span { strong { (title) } } small { "Source-backed Equihash industry briefing" } } } } } }
                    @if merged || buying { section { h2 { "Guides and tools" } div class="result-list" {
                        @if merged { a href="/merged-mining" { span { strong { "Zcash + Wcash merged mining" } } small { "Miner overview and pool operator guide" } } }
                        @if buying { a href="/vendors" { span { strong { "Equihash ASIC vendor directory" } } small { "Seller identity checks and dated listing snapshots" } } }
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
            header class="page-head" { p class="eyebrow" { "CONTRIBUTE" } h1 { "Add what miners need" } p class="lede" { "Listings are free. Public evidence is required for factual claims, and accepted fields keep their source. Send a completed template to " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak on X" } "." } }
            nav class="contribute-nav" aria-label="Contribution types" { a href="#pool" { "Pool" } a href="#project" { "Coin or hardware" } a href="/add-vendor" { "Vendor" } a href="#question" { "Question or correction" } }
            section id="pool" { h2 { "Pool listing or correction" } p { "A useful pool listing needs a public website plus enough evidence to verify fee, payout, endpoints and current activity. Public machine-readable stats give miners the best record." } div class="tpl" { div class="tpl-head" { span { "Pool template" } button class="btn small" type="button" data-copy="#pool-template" { "Copy" } } pre id="pool-template" { (pool) } } }
            section id="project" { h2 { "Coin or hardware listing" } p { "Compatibility is based on exact n,k parameters. Link an official protocol or manufacturer specification wherever possible." } div class="tpl" { div class="tpl-head" { span { "Project template" } button class="btn small" type="button" data-copy="#project-template" { "Copy" } } pre id="project-template" { (project) } } }
            section id="question" { h2 { "Question, guide idea or correction" } p { "Ask something other miners will search for. Answers are reviewed and attached to the relevant coin, pool, hardware or guide page so they remain useful." } div class="tpl" { div class="tpl-head" { span { "Question template" } button class="btn small" type="button" data-copy="#question-template" { "Copy" } } pre id="question-template" { (question) } } }
            p class="small" { "Submissions are reviewed for relevance and verifiability. Listings are never sold and payment cannot change their order." }
        }
    })
}
