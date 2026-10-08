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

            aside class="wrap ed-affiliated-reference" aria-labelledby="merge-reference-title" {
                div {
                    p class="ed-section-index" { "MERGED-MINING REFERENCE" }
                    h2 id="merge-reference-title" { "Zcash + Wcash" }
                    p { (merged_pools) @if merged_pools == 1 { " listed Zcash pool declares merged-mining support." } @else { " listed Zcash pools declare merged-mining support." } " Pool support and WEC payouts depend on that pool's published terms." }
                    p class="ed-disclosure" { "Affiliation: Wcash and equihash.com share a maintainer. This is a factual protocol reference; no comparative return claim is made." }
                }
                nav aria-label="Merged-mining references" {
                    a href="/pools?coin=zcash&merged=1#pools" { "Pool records →" }
                    a href="/merged-mining" { "Technical guide →" }
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
                p class="small" { "Network and pool data refreshed " time datetime=[d.last_updated.as_deref()] { (fmt::utc(d.last_updated.as_deref())) } " · " a href="/sources" { "method and sources" } }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum SearchKind {
    Coin,
    Asic,
    Pool,
    Vendor,
    Industry,
    Tool,
}

impl SearchKind {
    fn heading(self) -> &'static str {
        match self {
            Self::Coin => "Coins",
            Self::Asic => "ASICs",
            Self::Pool => "Pools",
            Self::Vendor => "Vendor directory",
            Self::Industry => "Industry",
            Self::Tool => "Guides and tools",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AvailabilityIntent {
    Current,
    Preorder,
    SoldOut,
    Quote,
    Used,
}

impl AvailabilityIntent {
    fn label(self) -> &'static str {
        match self {
            Self::Current => "seller-declared available",
            Self::Preorder => "preorder or future batch",
            Self::SoldOut => "sold out",
            Self::Quote => "quote required",
            Self::Used => "used or historical",
        }
    }

    fn vendor_query_value(self) -> &'static str {
        match self {
            Self::Current => "seller_declared_spot_or_near_term",
            Self::Preorder => "preorder_or_future_batch",
            Self::SoldOut => "sold_out_or_no_current_listing",
            Self::Quote => "unknown_or_quote_required",
            Self::Used => "historical_or_used",
        }
    }

    fn listing_query_value(self) -> &'static str {
        match self {
            Self::Current => "in_stock",
            Self::Preorder => "preorder",
            Self::SoldOut => "sold_out",
            Self::Quote => "quote",
            Self::Used => "used",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CoinStateIntent {
    Active,
    Historical,
}

#[derive(Debug, Clone)]
struct LocationIntent {
    label: &'static str,
    listing_region: &'static str,
    pool_region: &'static str,
    aliases: &'static [&'static str],
}

#[derive(Debug, Clone)]
struct ParsedSearch {
    terms: Vec<String>,
    kind: Option<SearchKind>,
    location: Option<LocationIntent>,
    availability: Option<AvailabilityIntent>,
    coin_state: Option<CoinStateIntent>,
    merged: bool,
    payout: Option<String>,
}

#[derive(Debug, Clone)]
struct SearchDocument {
    id: String,
    kind: SearchKind,
    /// A tool can belong to an entity-specific search, such as the pool comparison.
    scope: Option<SearchKind>,
    title: String,
    subtitle: String,
    href: String,
    canonical: String,
    aliases: Vec<String>,
    search_text: String,
    locations: Vec<String>,
    availability: Vec<AvailabilityIntent>,
    coin_state: Option<CoinStateIntent>,
}

#[derive(Debug, Clone)]
struct SearchHit {
    doc: SearchDocument,
    score: i32,
}

fn normalize_search(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut separated = true;
    for ch in value.chars().flat_map(char::to_lowercase) {
        if ch.is_alphanumeric() {
            out.push(ch);
            separated = false;
        } else if !separated {
            out.push(' ');
            separated = true;
        }
    }
    out.trim().to_string()
}

fn replace_phrase(value: &str, from: &str, to: &str) -> String {
    let padded = format!(" {value} ");
    padded
        .replace(&format!(" {from} "), &format!(" {to} "))
        .trim()
        .to_string()
}

fn remove_phrase(tokens: &mut Vec<String>, phrase: &str) -> bool {
    let wanted: Vec<&str> = phrase.split_whitespace().collect();
    if wanted.is_empty() || wanted.len() > tokens.len() {
        return false;
    }
    if let Some(start) = tokens
        .windows(wanted.len())
        .position(|window| window.iter().map(String::as_str).eq(wanted.iter().copied()))
    {
        tokens.drain(start..start + wanted.len());
        true
    } else {
        false
    }
}

fn parse_search(raw: &str) -> ParsedSearch {
    let mut normalized = normalize_search(raw);
    for (alias, canonical) in [
        ("z15pro", "z15 pro"),
        ("z15p", "z15 pro"),
        ("nice hash", "nicehash"),
        ("piratechain", "pirate chain"),
        ("bitcoin gold", "bitcoingold"),
        ("bitcoin z", "bitcoinz"),
    ] {
        normalized = replace_phrase(&normalized, alias, canonical);
    }
    let mut tokens: Vec<String> = normalized
        .split_whitespace()
        .map(ToOwned::to_owned)
        .collect();

    let location_defs: &[(&str, &str, &str, &str, &[&str])] = &[
        (
            "united kingdom",
            "United Kingdom",
            "UK",
            "Europe",
            &["united kingdom", "uk", "gb", "great britain", "britain"],
        ),
        (
            "great britain",
            "United Kingdom",
            "UK",
            "Europe",
            &["united kingdom", "uk", "gb", "great britain", "britain"],
        ),
        (
            "uk",
            "United Kingdom",
            "UK",
            "Europe",
            &["united kingdom", "uk", "gb", "great britain", "britain"],
        ),
        (
            "united states",
            "United States",
            "USA",
            "North America",
            &["united states", "usa", "us", "america"],
        ),
        (
            "usa",
            "United States",
            "USA",
            "North America",
            &["united states", "usa", "us", "america"],
        ),
        (
            "us",
            "United States",
            "USA",
            "North America",
            &["united states", "usa", "us", "america"],
        ),
        (
            "european union",
            "European Union",
            "EU",
            "Europe",
            &["european union", "eu", "europe"],
        ),
        (
            "eu",
            "European Union",
            "EU",
            "Europe",
            &["european union", "eu", "europe"],
        ),
        (
            "europe",
            "Europe",
            "EU",
            "Europe",
            &["europe", "european union", "eu"],
        ),
        (
            "canada",
            "Canada",
            "Canada",
            "North America",
            &["canada", "canadian"],
        ),
        (
            "australia",
            "Australia",
            "Australia",
            "Asia-Pacific",
            &["australia", "australian"],
        ),
        (
            "china",
            "China",
            "China",
            "Asia-Pacific",
            &["china", "chinese", "hong kong"],
        ),
        (
            "hong kong",
            "Hong Kong",
            "Hong Kong",
            "Asia-Pacific",
            &["hong kong", "china"],
        ),
        (
            "germany",
            "Germany",
            "Germany",
            "Europe",
            &["germany", "german"],
        ),
        (
            "france",
            "France",
            "France",
            "Europe",
            &["france", "french"],
        ),
        ("spain", "Spain", "Spain", "Europe", &["spain", "spanish"]),
        (
            "netherlands",
            "Netherlands",
            "Netherlands",
            "Europe",
            &["netherlands", "dutch"],
        ),
        (
            "switzerland",
            "Switzerland",
            "Switzerland",
            "Europe",
            &["switzerland", "swiss"],
        ),
        (
            "ireland",
            "Ireland",
            "Ireland",
            "Europe",
            &["ireland", "irish"],
        ),
        (
            "united arab emirates",
            "United Arab Emirates",
            "UAE",
            "Middle East",
            &["united arab emirates", "uae", "dubai"],
        ),
        (
            "uae",
            "United Arab Emirates",
            "UAE",
            "Middle East",
            &["united arab emirates", "uae", "dubai"],
        ),
        (
            "india",
            "India",
            "India",
            "Asia-Pacific",
            &["india", "indian"],
        ),
        (
            "singapore",
            "Singapore",
            "Singapore",
            "Asia-Pacific",
            &["singapore"],
        ),
        (
            "south africa",
            "South Africa",
            "South Africa",
            "Africa",
            &["south africa"],
        ),
        (
            "brazil",
            "Brazil",
            "Brazil",
            "Latin America",
            &["brazil", "brazilian"],
        ),
        (
            "argentina",
            "Argentina",
            "Argentina",
            "Latin America",
            &["argentina", "argentinian"],
        ),
        (
            "global",
            "Global delivery",
            "Global",
            "",
            &["global", "worldwide"],
        ),
        (
            "worldwide",
            "Global delivery",
            "Global",
            "",
            &["global", "worldwide"],
        ),
    ];
    let mut location = None;
    for (phrase, label, listing_region, pool_region, aliases) in location_defs {
        if remove_phrase(&mut tokens, phrase) {
            location = Some(LocationIntent {
                label,
                listing_region,
                pool_region,
                aliases,
            });
            break;
        }
    }

    let availability_defs = [
        ("in stock", AvailabilityIntent::Current),
        ("available now", AvailabilityIntent::Current),
        ("available", AvailabilityIntent::Current),
        ("ready to ship", AvailabilityIntent::Current),
        ("pre order", AvailabilityIntent::Preorder),
        ("preorder", AvailabilityIntent::Preorder),
        ("future batch", AvailabilityIntent::Preorder),
        ("batch", AvailabilityIntent::Preorder),
        ("sold out", AvailabilityIntent::SoldOut),
        ("out of stock", AvailabilityIntent::SoldOut),
        ("quote required", AvailabilityIntent::Quote),
        ("quote", AvailabilityIntent::Quote),
        ("historical listing", AvailabilityIntent::Used),
        ("used", AvailabilityIntent::Used),
    ];
    let mut availability = None;
    for (phrase, value) in availability_defs {
        if remove_phrase(&mut tokens, phrase) {
            availability = Some(value);
            break;
        }
    }

    let kind_defs = [
        ("where to buy", SearchKind::Vendor),
        ("vendor", SearchKind::Vendor),
        ("vendors", SearchKind::Vendor),
        ("seller", SearchKind::Vendor),
        ("sellers", SearchKind::Vendor),
        ("shop", SearchKind::Vendor),
        ("buy", SearchKind::Vendor),
        ("mining pool", SearchKind::Pool),
        ("pools", SearchKind::Pool),
        ("pool", SearchKind::Pool),
        ("asics", SearchKind::Asic),
        ("asic", SearchKind::Asic),
        ("hardware", SearchKind::Asic),
        ("miners", SearchKind::Asic),
        ("miner", SearchKind::Asic),
        ("coins", SearchKind::Coin),
        ("coin", SearchKind::Coin),
        ("networks", SearchKind::Coin),
        ("network", SearchKind::Coin),
        ("news", SearchKind::Industry),
        ("industry", SearchKind::Industry),
    ];
    let mut kind = None;
    for (phrase, value) in kind_defs {
        if remove_phrase(&mut tokens, phrase) {
            kind = Some(value);
            break;
        }
    }

    let merged = ["merged mining", "merge mining", "auxpow"]
        .into_iter()
        .any(|phrase| {
            let found = remove_phrase(&mut tokens, phrase);
            found
        });
    let payout = [
        ("pps plus", "PPS+"),
        ("ppsplus", "PPS+"),
        ("pplns", "PPLNS"),
        ("pps", "PPS"),
        ("solo", "SOLO"),
    ]
    .into_iter()
    .find_map(|(phrase, value)| remove_phrase(&mut tokens, phrase).then(|| value.to_string()));

    let coin_state = if remove_phrase(&mut tokens, "active") {
        Some(CoinStateIntent::Active)
    } else if remove_phrase(&mut tokens, "ended")
        || remove_phrase(&mut tokens, "inactive")
        || remove_phrase(&mut tokens, "historical")
    {
        Some(CoinStateIntent::Historical)
    } else {
        None
    };

    tokens.retain(|token| {
        !matches!(
            token.as_str(),
            "a" | "an" | "the" | "for" | "to" | "show" | "find" | "me" | "all" | "please"
        )
    });

    ParsedSearch {
        terms: tokens,
        kind,
        location,
        availability,
        coin_state,
        merged,
        payout,
    }
}

fn aliases_for_coin(id: &str) -> &'static [&'static str] {
    match id {
        "zcash" => &["zec"],
        "wcash" => &["wec", "w cash"],
        "piratechain" => &["arrr", "pirate", "piratechain"],
        "bitcoingold" => &["btg", "bitcoin gold"],
        "bitcoinz" => &["btcz", "bitcoin z"],
        "ycash" => &["yec"],
        "zclassic" => &["zcl"],
        "horizen" => &["zen"],
        _ => &[],
    }
}

fn aliases_for_miner(id: &str) -> &'static [&'static str] {
    match id {
        "antminer-z15-pro" => &[
            "z15 pro",
            "z15pro",
            "z15p",
            "bitmain z15 pro",
            "840 ksol",
            "860 ksol",
        ],
        "antminer-z15k-565" => &["z15k", "z15k 565", "565 ksol"],
        "antminer-z15k-525" => &["z15k", "z15k 525", "525 ksol"],
        "innosilicon-a9pp-zmaster" => &["a9++", "a9 plus plus", "zmaster"],
        "antminer-z9-mini" => &["z9 mini"],
        _ => &[],
    }
}

fn host_text(url: Option<&str>) -> String {
    url.and_then(|url| url.split("//").nth(1))
        .unwrap_or("")
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim_start_matches("www.")
        .to_string()
}

fn availability_terms(state: &str) -> Vec<AvailabilityIntent> {
    match state {
        "seller_declared_spot_or_near_term" | "in_stock" => vec![AvailabilityIntent::Current],
        "preorder_or_future_batch" | "preorder" | "future_batch" => {
            vec![AvailabilityIntent::Preorder]
        }
        "sold_out_or_no_current_listing" | "sold_out" => vec![AvailabilityIntent::SoldOut],
        "unknown_or_quote_required" | "quote" | "quote_required" => vec![AvailabilityIntent::Quote],
        "historical_or_used" | "used" => vec![AvailabilityIntent::Used],
        _ => Vec::new(),
    }
}

fn make_doc(
    id: impl Into<String>,
    kind: SearchKind,
    scope: Option<SearchKind>,
    title: impl Into<String>,
    subtitle: impl Into<String>,
    href: impl Into<String>,
    aliases: &[&str],
    terms: impl Into<String>,
) -> SearchDocument {
    let title = title.into();
    let aliases: Vec<String> = aliases.iter().map(|v| normalize_search(v)).collect();
    let canonical = normalize_search(&title);
    let search_text =
        normalize_search(&format!("{} {} {}", title, aliases.join(" "), terms.into()));
    SearchDocument {
        id: id.into(),
        kind,
        scope,
        title,
        subtitle: subtitle.into(),
        href: href.into(),
        canonical,
        aliases,
        search_text,
        locations: Vec::new(),
        availability: Vec::new(),
        coin_state: None,
    }
}

fn vendor_availability(d: &Data, vendor_id: &str, declared: &str) -> Vec<AvailabilityIntent> {
    let mut values = availability_terms(declared);
    for listing in d
        .listings
        .iter()
        .filter(|listing| listing.vendor_id == vendor_id)
    {
        for value in availability_terms(listing.availability.as_deref().unwrap_or("")) {
            if !values.contains(&value) {
                values.push(value);
            }
        }
    }
    values
}

fn build_search_index(d: &Data) -> Vec<SearchDocument> {
    let mut docs = Vec::new();
    for coin in &d.coins {
        let mut doc = make_doc(
            format!("coin:{}", coin.id),
            SearchKind::Coin,
            None,
            &coin.name,
            format!(
                "{} · Equihash {} · {} · {} pools · checked {}",
                coin.symbol,
                coin.params(),
                if coin.active() {
                    "active PoW"
                } else {
                    "PoW ended"
                },
                coin.pool_count,
                fmt::utc(coin.fetched_at.as_deref())
            ),
            format!("/coin/{}", coin.id),
            aliases_for_coin(&coin.id),
            format!(
                "{} {} {} {} {}",
                coin.symbol,
                coin.id,
                coin.label,
                coin.params(),
                coin.status_note.as_deref().unwrap_or("")
            ),
        );
        doc.coin_state = Some(if coin.active() {
            CoinStateIntent::Active
        } else {
            CoinStateIntent::Historical
        });
        docs.push(doc);
    }
    for miner in &d.miners {
        docs.push(make_doc(
            format!("asic:{}", miner.id),
            SearchKind::Asic,
            None,
            format!("{} {}", miner.maker, miner.model),
            format!(
                "Equihash {} · {} kSol/s · {} W · {} · checked {}",
                miner.equihash,
                fmt::opt_num(miner.hashrate_ksol),
                fmt::int(miner.watts),
                match miner.spec_basis.as_deref() {
                    Some("manufacturer") => "manufacturer specification",
                    Some("market_reported") => "market-reported specification",
                    _ => "specification basis not stated",
                },
                fmt::utc(miner.fetched_at.as_deref())
            ),
            format!("/asics/{}", miner.id),
            aliases_for_miner(&miner.id),
            format!(
                "{} {} {} {} {}",
                miner.id,
                miner.equihash,
                miner
                    .hashrate_ksol
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
                miner.released.as_deref().unwrap_or(""),
                miner.notes.as_deref().unwrap_or("")
            ),
        ));
    }
    for pool in &d.pools {
        let mut doc = make_doc(
            format!("pool:{}", pool.slug),
            SearchKind::Pool,
            None,
            &pool.name,
            format!(
                "{} · {} · {} · {} · checked {}",
                pool.coin_label,
                pool.payout_schemes.join(" / "),
                pool.region.as_deref().unwrap_or("region not stated"),
                if pool.active {
                    "active listing"
                } else {
                    "inactive listing"
                },
                fmt::utc(pool.fetched_at.as_deref())
            ),
            format!("/pool/{}", pool.slug),
            &[],
            format!(
                "{} {} {} {} {} {} {}",
                pool.id,
                pool.coin,
                pool.coin_id,
                host_text(pool.url.as_deref()),
                pool.region.as_deref().unwrap_or(""),
                pool.regions.join(" "),
                pool.payout_schemes.join(" ")
            ),
        );
        doc.locations
            .extend(pool.region_tags.iter().map(|v| normalize_search(v)));
        doc.locations
            .extend(pool.regions.iter().map(|v| normalize_search(v)));
        if let Some(region) = pool.region.as_deref() {
            doc.locations.push(normalize_search(region));
        }
        docs.push(doc);
    }

    let mut represented_hosts = std::collections::HashSet::new();
    for record in d
        .vendor_research
        .iter()
        .filter(|record| record.record_type != "coverage_gap")
    {
        let host = host_text(record.website.as_deref());
        if !host.is_empty() {
            represented_hosts.insert(host.clone());
        }
        let profile = d.vendors.iter().find(|vendor| {
            !host.is_empty() && host_text(vendor.url.as_deref()).eq_ignore_ascii_case(&host)
        });
        let vendor_id = profile.map(|vendor| vendor.id.as_str()).unwrap_or("");
        let listings: Vec<&crate::data::Listing> = d
            .listings
            .iter()
            .filter(|listing| listing.vendor_id == vendor_id)
            .collect();
        let listing_terms = listings
            .iter()
            .map(|listing| {
                let miner = d.miners.iter().find(|miner| miner.id == listing.miner_id);
                format!(
                    "{} {} {} {} {} {}",
                    listing.title,
                    listing.miner_id,
                    miner.map(|miner| miner.model.as_str()).unwrap_or(""),
                    listing.shipping_regions.join(" "),
                    listing.availability.as_deref().unwrap_or(""),
                    listing.availability_label.as_deref().unwrap_or("")
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let href = profile
            .map(|vendor| format!("/vendors/{}", vendor.slug))
            .unwrap_or_else(|| format!("/vendors#vendor-{}", record.id));
        let mut doc = make_doc(
            format!("vendor:{}", record.id),
            SearchKind::Vendor,
            None,
            &record.vendor,
            format!(
                "{} · {} · {} · checked {}",
                record.country,
                record.region,
                record.declared_availability.replace('_', " "),
                fmt::utc(record.last_verified.as_deref())
            ),
            href,
            &[],
            format!(
                "{} {} {} {} {} {} {} {}",
                host,
                record.country,
                record.region,
                record.record_type,
                record.equihash_z15_claim,
                record.legal_identity_and_location_summary,
                record.availability_basis,
                listing_terms
            ),
        );
        doc.locations = vec![
            normalize_search(&record.country),
            normalize_search(&record.region),
        ];
        if let Some(profile) = profile {
            doc.locations
                .extend(profile.regions.iter().map(|v| normalize_search(v)));
            if let Some(base) = profile.base_region.as_deref() {
                doc.locations.push(normalize_search(base));
            }
        }
        doc.availability = vendor_availability(d, vendor_id, &record.declared_availability);
        docs.push(doc);
    }
    for vendor in &d.vendors {
        let host = host_text(vendor.url.as_deref());
        if !host.is_empty() && represented_hosts.contains(&host) {
            continue;
        }
        let listings: Vec<&crate::data::Listing> = d
            .listings
            .iter()
            .filter(|listing| listing.vendor_id == vendor.id)
            .collect();
        let listing_terms = listings
            .iter()
            .map(|listing| {
                format!(
                    "{} {} {}",
                    listing.title,
                    listing.miner_id,
                    listing.shipping_regions.join(" ")
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let mut doc = make_doc(
            format!("vendor-profile:{}", vendor.id),
            SearchKind::Vendor,
            None,
            &vendor.name,
            format!(
                "{} · {}",
                vendor
                    .base_region
                    .as_deref()
                    .unwrap_or("Location not stated"),
                vendor
                    .region_focus
                    .as_deref()
                    .unwrap_or("Delivery region not stated")
            ),
            format!("/vendors/{}", vendor.slug),
            &[],
            format!(
                "{} {} {} {}",
                host,
                vendor.regions.join(" "),
                vendor.region_note.as_deref().unwrap_or(""),
                listing_terms
            ),
        );
        doc.locations
            .extend(vendor.regions.iter().map(|v| normalize_search(v)));
        if let Some(base) = vendor.base_region.as_deref() {
            doc.locations.push(normalize_search(base));
        }
        for listing in listings {
            for value in availability_terms(listing.availability.as_deref().unwrap_or("")) {
                if !doc.availability.contains(&value) {
                    doc.availability.push(value);
                }
            }
        }
        docs.push(doc);
    }

    for (id, href, title, terms) in [
        (
            "cypherpunk",
            "/industry/cypherpunk-zcash-mining",
            "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet",
            "Cypherpunk CYPH Zcash mining 4902 Z15 Pro hashrate fleet Winklevoss",
        ),
        (
            "grayscale",
            "/industry/grayscale-zcash-etf",
            "From OTC trust to ZCSH: Grayscale’s Zcash vehicle",
            "Grayscale ZCSH Zcash ETF trust NYSE Arca institutional capital custody",
        ),
        (
            "winklevoss",
            "/industry/winklevoss-zcash-etf",
            "WINK filed: the preliminary Winklevoss Zcash ETF",
            "Winklevoss WINK Zcash ETF Gemini Cypherpunk filing Nasdaq",
        ),
    ] {
        docs.push(make_doc(
            format!("industry:{id}"),
            SearchKind::Industry,
            None,
            title,
            "Source-backed Equihash industry briefing",
            href,
            &[],
            terms,
        ));
    }

    let tools: &[(&str, &str, &str, &str, &[&str], Option<SearchKind>)] = &[
        (
            "calculator",
            "/calculator",
            "Mining calculator",
            "Zcash Equihash profitability revenue income earnings electricity fee costs",
            &["profit calculator", "earnings calculator"],
            None,
        ),
        (
            "vendors",
            "/vendors",
            "ASIC vendor directory",
            "Z15 Pro sellers shops buy available stock preorder batch countries",
            &["where to buy", "seller directory"],
            Some(SearchKind::Vendor),
        ),
        (
            "asics",
            "/asics",
            "Equihash ASIC index",
            "Bitmain Antminer Z15 Pro Z15K Z15 Z11 Z9 Innosilicon hardware specifications compare directory",
            &["miner list", "hardware comparison"],
            Some(SearchKind::Asic),
        ),
        (
            "pools",
            "/pools",
            "Mining pool comparison",
            "Zcash Wcash Equihash pools PPLNS PPS solo fee payout region compare",
            &["best pool", "pool directory"],
            Some(SearchKind::Pool),
        ),
        (
            "coins",
            "/coins",
            "Equihash coin directory",
            "networks coins parameters 200 9 192 7 144 5 active historical",
            &["network directory"],
            Some(SearchKind::Coin),
        ),
        (
            "hashpower",
            "/hashpower",
            "Equihash hashpower market",
            "NiceHash nice hash rent buy sell hashrate order book EQUIHASH BTC",
            &["nicehash", "hashrate rental"],
            None,
        ),
        (
            "merged",
            "/merged-mining",
            "Zcash + Wcash merged mining",
            "merged mining merge mining auxpow ZEC WEC pool operator guide",
            &["auxpow", "zec wec"],
            None,
        ),
        (
            "zcash-guide",
            "/zcash-mining",
            "How to mine Zcash",
            "Zcash mining setup Z15 Pro wallet pool electricity privacy guide",
            &["zcash mining guide"],
            None,
        ),
        (
            "guides",
            "/guides",
            "Equihash mining guides",
            "setup learn how tutorial mining hardware pools costs",
            &["guides"],
            None,
        ),
        (
            "sources",
            "/sources",
            "Sources and method",
            "data sources evidence methodology freshness provenance corrections API JSON",
            &["method", "evidence"],
            None,
        ),
        (
            "industry",
            "/industry",
            "Equihash industry briefings",
            "news research companies institutions ETF mining fleet",
            &["industry news"],
            Some(SearchKind::Industry),
        ),
    ];
    for (id, href, title, terms, aliases, scope) in tools {
        docs.push(make_doc(
            format!("tool:{id}"),
            SearchKind::Tool,
            *scope,
            *title,
            "Equihash.com guide or decision tool",
            *href,
            aliases,
            *terms,
        ));
    }
    docs
}

fn phrase_matches(haystack: &str, needle: &str) -> bool {
    format!(" {haystack} ").contains(&format!(" {needle} "))
}

fn location_matches(doc: &SearchDocument, location: &LocationIntent) -> bool {
    doc.locations.iter().any(|value| {
        location.aliases.iter().any(|alias| {
            let alias = normalize_search(alias);
            phrase_matches(value, &alias) || phrase_matches(&alias, value)
        })
    })
}

fn score_document(doc: &SearchDocument, parsed: &ParsedSearch) -> Option<i32> {
    if let Some(kind) = parsed.kind {
        if doc.kind != kind && !(doc.kind == SearchKind::Tool && doc.scope == Some(kind)) {
            return None;
        }
    }
    if let Some(state) = parsed.coin_state {
        if doc.kind == SearchKind::Coin && doc.coin_state != Some(state) {
            return None;
        }
    }
    if let Some(location) = &parsed.location {
        if matches!(doc.kind, SearchKind::Vendor | SearchKind::Pool)
            && !location_matches(doc, location)
        {
            return None;
        }
    }
    if let Some(availability) = parsed.availability {
        if doc.kind == SearchKind::Vendor && !doc.availability.contains(&availability) {
            return None;
        }
    }
    if parsed.merged && !doc.search_text.contains("merged") && !doc.search_text.contains("auxpow") {
        return None;
    }
    if let Some(payout) = parsed.payout.as_deref() {
        let payout = normalize_search(payout);
        if !payout
            .split_whitespace()
            .all(|term| doc.search_text.split_whitespace().any(|word| word == term))
        {
            return None;
        }
    }

    if parsed.terms.is_empty() {
        if parsed.kind.is_none()
            && parsed.location.is_none()
            && parsed.availability.is_none()
            && parsed.coin_state.is_none()
            && !parsed.merged
            && parsed.payout.is_none()
        {
            return None;
        }
    } else if !parsed.terms.iter().all(|term| {
        doc.search_text
            .split_whitespace()
            .any(|word| word == term || (term.len() >= 3 && word.starts_with(term)))
    }) {
        return None;
    }

    let free_phrase = parsed.terms.join(" ");
    let mut score = 0;
    if !free_phrase.is_empty() {
        if doc.canonical == free_phrase {
            score += 10_000;
        } else if doc.aliases.iter().any(|alias| alias == &free_phrase) {
            score += 9_000;
        } else if phrase_matches(&doc.canonical, &free_phrase) {
            score += 4_000;
        } else if doc.search_text.contains(&free_phrase) {
            score += 2_000;
        }
        for term in &parsed.terms {
            if doc.canonical.split_whitespace().any(|word| word == term) {
                score += 500;
            } else if doc
                .aliases
                .iter()
                .any(|alias| alias.split_whitespace().any(|word| word == term))
            {
                score += 400;
            } else {
                score += 100;
            }
        }
    }
    if parsed.location.is_some() && matches!(doc.kind, SearchKind::Vendor | SearchKind::Pool) {
        score += 700;
    }
    if parsed.availability.is_some() && doc.kind == SearchKind::Vendor {
        score += 700;
    }
    if parsed.kind == Some(doc.kind) {
        score += 500;
    }
    if doc.kind == SearchKind::Tool {
        score -= 25;
    }
    Some(score)
}

fn rank_documents(docs: &[SearchDocument], parsed: &ParsedSearch) -> Vec<SearchHit> {
    let mut hits: Vec<SearchHit> = docs
        .iter()
        .filter_map(|doc| {
            score_document(doc, parsed).map(|score| SearchHit {
                doc: doc.clone(),
                score,
            })
        })
        .collect();
    hits.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| a.doc.kind.cmp(&b.doc.kind))
            .then_with(|| a.doc.title.to_lowercase().cmp(&b.doc.title.to_lowercase()))
            .then_with(|| a.doc.id.cmp(&b.doc.id))
    });
    hits
}

fn selected_entity_id(hits: &[SearchHit], kind: SearchKind) -> Option<String> {
    hits.iter()
        .find(|hit| hit.doc.kind == kind)
        .and_then(|hit| hit.doc.id.split_once(':').map(|(_, id)| id.to_string()))
}

fn encode_query_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            out.push(byte as char);
        } else {
            use std::fmt::Write;
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

fn append_query(path: &str, pairs: &[(&str, String)]) -> String {
    let active: Vec<_> = pairs
        .iter()
        .filter(|(_, value)| !value.is_empty())
        .collect();
    if active.is_empty() {
        return path.to_string();
    }
    let query = active
        .iter()
        .map(|(key, value)| format!("{key}={}", encode_query_component(value)))
        .collect::<Vec<_>>()
        .join("&");
    format!("{path}?{query}")
}

fn apply_continuity(d: &Data, hits: &mut [SearchHit], parsed: &ParsedSearch) {
    // An explicit entity filter can hide the entity that supplies continuity. For example,
    // “where to buy Z15 Pro” asks for vendors, but the vendor-directory URL still needs the
    // canonical machine id. Resolve that context against the same index without the output-type
    // and field filters; this changes the link state, not the visible result set.
    let inferred = if selected_entity_id(hits, SearchKind::Asic).is_none()
        || selected_entity_id(hits, SearchKind::Coin).is_none()
    {
        let relaxed = ParsedSearch {
            terms: parsed.terms.clone(),
            kind: None,
            location: None,
            availability: None,
            coin_state: None,
            merged: false,
            payout: None,
        };
        rank_documents(&build_search_index(d), &relaxed)
    } else {
        Vec::new()
    };
    let miner_id = selected_entity_id(hits, SearchKind::Asic)
        .or_else(|| selected_entity_id(&inferred, SearchKind::Asic));
    let coin_id = selected_entity_id(hits, SearchKind::Coin)
        .or_else(|| selected_entity_id(&inferred, SearchKind::Coin));
    let miner = miner_id
        .as_deref()
        .and_then(|id| d.miners.iter().find(|miner| miner.id == id));
    for hit in hits {
        if hit.doc.kind != SearchKind::Tool {
            continue;
        }
        hit.doc.href = match hit.doc.id.as_str() {
            "tool:vendors" => append_query(
                "/vendors",
                &[
                    ("machine", miner_id.clone().unwrap_or_default()),
                    (
                        "region",
                        parsed
                            .location
                            .as_ref()
                            .map(|location| location.listing_region.to_string())
                            .unwrap_or_default(),
                    ),
                    (
                        "state",
                        parsed
                            .availability
                            .map(|availability| availability.listing_query_value().to_string())
                            .unwrap_or_default(),
                    ),
                    (
                        "availability",
                        parsed
                            .availability
                            .map(|availability| availability.vendor_query_value().to_string())
                            .unwrap_or_default(),
                    ),
                ],
            ),
            "tool:pools" => {
                append_query(
                    "/pools",
                    &[
                        ("coin", coin_id.clone().unwrap_or_default()),
                        (
                            "region",
                            parsed
                                .location
                                .as_ref()
                                .map(|location| location.pool_region.to_string())
                                .unwrap_or_default(),
                        ),
                        ("scheme", parsed.payout.clone().unwrap_or_default()),
                        (
                            "merged",
                            if parsed.merged {
                                "1".into()
                            } else {
                                String::new()
                            },
                        ),
                    ],
                ) + "#pools"
            }
            "tool:calculator" => append_query(
                "/calculator",
                &[
                    ("coin", coin_id.clone().unwrap_or_default()),
                    (
                        "hashrate",
                        miner
                            .and_then(|miner| miner.hashrate_ksol)
                            .map(|value| fmt::num_short(value))
                            .unwrap_or_default(),
                    ),
                    (
                        "watts",
                        miner
                            .and_then(|miner| miner.watts)
                            .map(|value| fmt::num_short(value))
                            .unwrap_or_default(),
                    ),
                ],
            ),
            _ => hit.doc.href.clone(),
        };
    }
}

fn interpretation(parsed: &ParsedSearch) -> Vec<String> {
    let mut values = Vec::new();
    if let Some(kind) = parsed.kind {
        values.push(kind.heading().to_string());
    }
    if let Some(location) = &parsed.location {
        values.push(format!("delivery/location: {}", location.label));
    }
    if let Some(availability) = parsed.availability {
        values.push(format!("availability: {}", availability.label()));
    }
    if let Some(state) = parsed.coin_state {
        values.push(match state {
            CoinStateIntent::Active => "active networks".into(),
            CoinStateIntent::Historical => "historical networks".into(),
        });
    }
    if parsed.merged {
        values.push("merged mining".into());
    }
    if let Some(payout) = &parsed.payout {
        values.push(format!("payout: {payout}"));
    }
    values
}

pub fn search(d: &Data, q: &SearchQuery) -> Markup {
    let raw = q.q.as_deref().unwrap_or("").trim();
    let parsed = parse_search(raw);
    let mut hits = if raw.is_empty() {
        Vec::new()
    } else {
        rank_documents(&build_search_index(d), &parsed)
    };
    apply_continuity(d, &mut hits, &parsed);
    let understood = interpretation(&parsed);
    let visible_count = hits.len();
    layout(d, Page { title: "Search Equihash coins, pools, hardware, vendors and industry briefings", description: "Search the equihash.com directory for coins, mining pools, hardware, vendors, guides and sourced industry briefings.", path: "/search", nav: "" }, html! {
        div class="wrap page search-page" {
            header class="page-head" { h1 { "Search equihash.com" } (search_form(raw, "Coin, pool, hardware, vendor, country or payout")) }
            @if raw.is_empty() {
                div class="empty-state" { h2 { "Search the whole directory" } p { "Try “Z15 Pro UK in stock”, “Zcash PPLNS pools”, “flypool.org”, “KRGN”, “merged mining” or “earnings calculator”." } }
            } @else if visible_count == 0 {
                div class="empty-state" {
                    h2 { "No results for “" (raw) "”" }
                    @if !understood.is_empty() { p { "Recognised filters: " (understood.join(" · ")) "." } }
                    p { "Remove one filter, try a coin symbol or model name, or send the missing public source." }
                    a href="/contribute" { "Suggest a missing listing" }
                }
            } @else {
                p class="result-count" { (visible_count) " result" @if visible_count != 1 { "s" } " for “" (raw) "”" }
                @if !understood.is_empty() {
                    p class="source-line" { strong { "Understood as: " } (understood.join(" · ")) ". Filters apply only where that field exists; for example, country and availability narrow sellers while the matching ASIC record remains visible." }
                }
                div class="search-results" {
                    @for kind in [SearchKind::Coin, SearchKind::Asic, SearchKind::Pool, SearchKind::Vendor, SearchKind::Industry, SearchKind::Tool] {
                        @let group: Vec<&SearchHit> = hits.iter().filter(|hit| hit.doc.kind == kind).collect();
                        @if !group.is_empty() {
                            section {
                                h2 { (kind.heading()) }
                                div class="result-list" {
                                    @for hit in group {
                                        a href=(&hit.doc.href) { span { strong { (&hit.doc.title) } } small { (&hit.doc.subtitle) } }
                                    }
                                }
                            }
                        }
                    }
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
            header class="page-head" { p class="eyebrow" { "CONTRIBUTE" } h1 { "Add what miners need" } p class="lede" { "Listings are free. Public evidence is required for factual claims, and accepted fields keep their source. Send a completed template to " a href="https://x.com/EquihashCom" rel="noopener" { "@EquihashCom on X" } "." } }
            nav class="contribute-nav" aria-label="Contribution types" { a href="#pool" { "Pool" } a href="#project" { "Coin or hardware" } a href="/add-vendor" { "Vendor" } a href="#question" { "Question or correction" } }
            section id="pool" { h2 { "Pool listing or correction" } p { "A useful pool listing needs a public website plus enough evidence to verify fee, payout, endpoints and current activity. Public machine-readable stats give miners the best record." } div class="tpl" { div class="tpl-head" { span { "Pool template" } button class="btn small" type="button" data-copy="#pool-template" { "Copy" } } pre id="pool-template" { (pool) } } }
            section id="project" { h2 { "Coin or hardware listing" } p { "Compatibility is based on exact n,k parameters. Link an official protocol or manufacturer specification wherever possible." } div class="tpl" { div class="tpl-head" { span { "Project template" } button class="btn small" type="button" data-copy="#project-template" { "Copy" } } pre id="project-template" { (project) } } }
            section id="question" { h2 { "Question, guide idea or correction" } p { "Ask something other miners will search for. Answers are reviewed and attached to the relevant coin, pool, hardware or guide page so they remain useful." } div class="tpl" { div class="tpl-head" { span { "Question template" } button class="btn small" type="button" data-copy="#question-template" { "Copy" } } pre id="question-template" { (question) } } }
            p class="small" { "Submissions are reviewed for relevance and verifiability. Listings are never sold and payment cannot change their order." }
        }
    })
}

#[cfg(test)]
mod structured_search_tests {
    use super::*;

    fn doc(
        id: &str,
        kind: SearchKind,
        title: &str,
        terms: &str,
        aliases: &[&str],
    ) -> SearchDocument {
        make_doc(
            id,
            kind,
            None,
            title,
            "fixture",
            format!("/{id}"),
            aliases,
            terms,
        )
    }

    fn fixture_documents() -> Vec<SearchDocument> {
        let mut docs = vec![
            doc(
                "coin:zcash",
                SearchKind::Coin,
                "Zcash",
                "ZEC zcash Equihash 200 9 active",
                &["zec"],
            ),
            doc(
                "coin:wcash",
                SearchKind::Coin,
                "Wcash",
                "WEC wcash Equihash 200 9 active",
                &["wec", "w cash"],
            ),
            doc(
                "coin:piratechain",
                SearchKind::Coin,
                "Pirate Chain",
                "ARRR piratechain Equihash 200 9 active",
                &["arrr", "pirate"],
            ),
            doc(
                "coin:bitcoingold",
                SearchKind::Coin,
                "Bitcoin Gold",
                "BTG bitcoingold Equihash 144 5 active",
                &["btg"],
            ),
            doc(
                "coin:ycash",
                SearchKind::Coin,
                "Ycash",
                "YEC ycash Equihash 192 7 active",
                &["yec"],
            ),
            doc(
                "coin:zclassic",
                SearchKind::Coin,
                "Zclassic",
                "ZCL zclassic Equihash 192 7 active",
                &["zcl"],
            ),
            doc(
                "coin:kerrigan-a",
                SearchKind::Coin,
                "Kerrigan",
                "KRGN Equihash 200 9 active",
                &[],
            ),
            doc(
                "coin:kerrigan-b",
                SearchKind::Coin,
                "Kerrigan",
                "KRGN Equihash 192 7 active",
                &[],
            ),
            doc(
                "coin:horizen",
                SearchKind::Coin,
                "Horizen",
                "ZEN historical ended Equihash 200 9",
                &["zen"],
            ),
            doc(
                "asic:antminer-z15-pro",
                SearchKind::Asic,
                "Bitmain Antminer Z15 Pro",
                "antminer z15 pro bitmain 840 860 ksol Equihash 200 9",
                &["z15p", "z15pro"],
            ),
            doc(
                "asic:antminer-z15k-565",
                SearchKind::Asic,
                "Bitmain Antminer Z15K 565",
                "z15k 565 ksol Equihash 200 9",
                &["z15k"],
            ),
            doc(
                "pool:flypool",
                SearchKind::Pool,
                "Flypool",
                "flypool org zcash ZEC PPLNS Europe",
                &[],
            ),
            doc(
                "pool:zecwec",
                SearchKind::Pool,
                "ZecWec",
                "zecwec zcash wcash ZEC WEC PPLNS merged mining auxpow Europe",
                &["zec wec"],
            ),
            doc(
                "industry:cypherpunk",
                SearchKind::Industry,
                "Cypherpunk Zcash mining fleet",
                "CYPH Winklevoss Z15 Pro hashrate mining",
                &[],
            ),
            doc(
                "industry:grayscale",
                SearchKind::Industry,
                "Grayscale Zcash ETF",
                "ZCSH ETF trust NYSE Arca",
                &[],
            ),
            doc(
                "industry:winklevoss",
                SearchKind::Industry,
                "Winklevoss Zcash ETF",
                "WINK ETF Gemini filing",
                &[],
            ),
        ];
        docs[0].coin_state = Some(CoinStateIntent::Active);
        docs[1].coin_state = Some(CoinStateIntent::Active);
        docs[2].coin_state = Some(CoinStateIntent::Active);
        docs[3].coin_state = Some(CoinStateIntent::Active);
        docs[4].coin_state = Some(CoinStateIntent::Active);
        docs[5].coin_state = Some(CoinStateIntent::Active);
        docs[6].coin_state = Some(CoinStateIntent::Active);
        docs[7].coin_state = Some(CoinStateIntent::Active);
        docs[8].coin_state = Some(CoinStateIntent::Historical);
        docs[11].locations = vec!["europe".into()];
        docs[12].locations = vec!["europe".into()];

        let mut uk_vendor = doc(
            "vendor:uk-current",
            SearchKind::Vendor,
            "The Mining Shop UK",
            "United Kingdom UK Europe Z15 Pro Bitmain Antminer seller",
            &[],
        );
        uk_vendor.locations = vec!["united kingdom".into(), "uk".into(), "europe".into()];
        uk_vendor.availability = vec![AvailabilityIntent::Current];
        docs.push(uk_vendor);

        let mut eu_vendor = doc(
            "vendor:eu-preorder",
            SearchKind::Vendor,
            "European ASIC Store",
            "European Union EU Europe Z15 Pro Bitmain Antminer future batch",
            &[],
        );
        eu_vendor.locations = vec!["european union".into(), "eu".into(), "europe".into()];
        eu_vendor.availability = vec![AvailabilityIntent::Preorder];
        docs.push(eu_vendor);

        let mut sold_vendor = doc(
            "vendor:us-sold",
            SearchKind::Vendor,
            "Example ASIC USA",
            "United States USA Z15 Pro Bitmain Antminer sold out",
            &[],
        );
        sold_vendor.locations = vec!["united states".into(), "usa".into()];
        sold_vendor.availability = vec![AvailabilityIntent::SoldOut];
        docs.push(sold_vendor);

        for (id, title, terms, aliases, scope) in [
            (
                "tool:calculator",
                "Mining calculator",
                "Zcash Equihash profitability revenue income earnings electricity fee costs",
                &["profit calculator", "earnings calculator"][..],
                None,
            ),
            (
                "tool:vendors",
                "ASIC vendor directory",
                "Z15 Pro sellers shops buy available stock preorder batch countries directory",
                &["where to buy", "seller directory"][..],
                Some(SearchKind::Vendor),
            ),
            (
                "tool:asics",
                "Equihash ASIC index",
                "Bitmain Antminer Z15 Pro Z15K hardware specifications compare directory",
                &["miner list", "hardware comparison"][..],
                Some(SearchKind::Asic),
            ),
            (
                "tool:pools",
                "Mining pool comparison",
                "Zcash Wcash Equihash pools PPLNS PPS SOLO fee payout region compare directory",
                &["best pool", "pool directory"][..],
                Some(SearchKind::Pool),
            ),
            (
                "tool:coins",
                "Equihash coin directory",
                "networks coins parameters 200 9 active historical directory",
                &["network directory"][..],
                Some(SearchKind::Coin),
            ),
            (
                "tool:hashpower",
                "Equihash hashpower market",
                "NiceHash nice hash rent buy sell hashrate order book",
                &["nicehash", "hashrate rental"][..],
                None,
            ),
            (
                "tool:merged",
                "Zcash + Wcash merged mining",
                "merged mining merge mining auxpow ZEC WEC pool operator guide",
                &["auxpow", "zec wec"][..],
                None,
            ),
            (
                "tool:zcash-guide",
                "How to mine Zcash",
                "Zcash mining setup Z15 Pro wallet pool electricity privacy guide",
                &["zcash mining guide"][..],
                None,
            ),
            (
                "tool:guides",
                "Equihash mining guides",
                "setup learn how tutorial mining hardware pools costs guides",
                &[][..],
                None,
            ),
            (
                "tool:sources",
                "Sources and method",
                "data sources evidence methodology freshness provenance corrections API JSON",
                &["method", "evidence"][..],
                None,
            ),
            (
                "tool:industry",
                "Equihash industry briefings",
                "news research companies institutions ETF mining fleet",
                &["industry news"][..],
                Some(SearchKind::Industry),
            ),
        ] {
            let mut tool = doc(id, SearchKind::Tool, title, terms, aliases);
            tool.scope = scope;
            docs.push(tool);
        }
        docs
    }

    fn result_ids(query: &str) -> Vec<String> {
        let parsed = parse_search(query);
        rank_documents(&fixture_documents(), &parsed)
            .into_iter()
            .map(|hit| hit.doc.id)
            .collect()
    }

    fn assert_has(query: &str, expected: &str) {
        let ids = result_ids(query);
        assert!(
            ids.iter().any(|id| id == expected),
            "query {query:?} should include {expected:?}; got {ids:?}"
        );
    }

    macro_rules! search_case {
        ($name:ident, $query:expr, $expected:expr) => {
            #[test]
            fn $name() {
                assert_has($query, $expected);
            }
        };
    }

    search_case!(exact_coin_name, "Zcash", "coin:zcash");
    search_case!(zec_ticker_alias, "ZEC", "coin:zcash");
    search_case!(wec_ticker_alias, "WEC", "coin:wcash");
    search_case!(arrr_ticker_alias, "ARRR", "coin:piratechain");
    search_case!(pirate_short_alias, "pirate", "coin:piratechain");
    search_case!(btg_ticker_alias, "BTG", "coin:bitcoingold");
    search_case!(yec_ticker_alias, "YEC", "coin:ycash");
    search_case!(zcl_ticker_alias, "ZCL", "coin:zclassic");
    search_case!(exact_asic_name, "Antminer Z15 Pro", "asic:antminer-z15-pro");
    search_case!(compact_asic_alias, "z15pro", "asic:antminer-z15-pro");
    search_case!(short_asic_alias, "z15p", "asic:antminer-z15-pro");
    search_case!(maker_and_hashrate, "Bitmain 840", "asic:antminer-z15-pro");
    search_case!(asic_variant, "Z15K 565", "asic:antminer-z15k-565");
    search_case!(parameter_set, "200,9", "asic:antminer-z15-pro");
    search_case!(pool_payout_and_coin, "Zcash PPLNS pools", "pool:flypool");
    search_case!(pool_domain, "flypool.org", "pool:flypool");
    search_case!(merged_pool_alias, "ZecWec", "pool:zecwec");
    search_case!(
        model_country_stock,
        "Z15 Pro UK in stock",
        "vendor:uk-current"
    );
    search_case!(
        long_country_stock,
        "Z15 Pro United Kingdom available",
        "vendor:uk-current"
    );
    search_case!(
        model_eu_preorder,
        "Z15 Pro EU preorder",
        "vendor:eu-preorder"
    );
    search_case!(model_sold_out, "Z15 Pro USA sold out", "vendor:us-sold");
    search_case!(nicehash_spaced_alias, "nice hash", "tool:hashpower");
    search_case!(
        hashrate_rental_alias,
        "rent Equihash hashrate",
        "tool:hashpower"
    );
    search_case!(merged_mining_phrase, "merged mining", "tool:merged");
    search_case!(auxpow_alias, "auxpow", "tool:merged");
    search_case!(zec_wec_alias, "zec wec", "tool:merged");
    search_case!(
        profitability_calculator,
        "profitability calculator",
        "tool:calculator"
    );
    search_case!(zcash_earnings, "Zcash earnings", "tool:calculator");
    search_case!(zcash_mining_guide, "Zcash mining guide", "tool:zcash-guide");
    search_case!(source_method, "sources method", "tool:sources");
    search_case!(cypherpunk_industry, "Cypherpunk", "industry:cypherpunk");
    search_case!(grayscale_etf, "Grayscale ETF", "industry:grayscale");
    search_case!(winklevoss_ticker, "Winklevoss WINK", "industry:winklevoss");
    search_case!(asic_directory, "ASIC directory", "tool:asics");
    search_case!(vendor_directory, "vendor directory", "tool:vendors");
    search_case!(pool_comparison, "pool comparison", "tool:pools");
    search_case!(coin_directory, "coin directory", "tool:coins");
    search_case!(historical_coin, "historical coin ZEN", "coin:horizen");
    search_case!(active_coin, "active coin Zcash", "coin:zcash");
    search_case!(
        industry_filter,
        "industry Grayscale ETF",
        "industry:grayscale"
    );

    #[test]
    fn ambiguous_ticker_returns_every_matching_entity() {
        let ids = result_ids("KRGN");
        assert!(ids.contains(&"coin:kerrigan-a".to_string()));
        assert!(ids.contains(&"coin:kerrigan-b".to_string()));
    }

    #[test]
    fn unknown_query_has_an_honest_empty_result() {
        assert!(result_ids("definitely missing frobnicator").is_empty());
    }

    #[test]
    fn structured_filters_do_not_hide_the_matching_asic() {
        assert_has("Z15 Pro UK in stock", "asic:antminer-z15-pro");
    }

    #[test]
    fn query_parser_extracts_country_and_availability() {
        let parsed = parse_search("Z15 Pro UK in stock");
        assert_eq!(parsed.terms, ["z15", "pro"]);
        assert_eq!(
            parsed.location.as_ref().map(|v| v.label),
            Some("United Kingdom")
        );
        assert_eq!(parsed.availability, Some(AvailabilityIntent::Current));
    }

    #[test]
    fn continuity_links_preserve_known_filters() {
        let parsed = parse_search("Z15 Pro UK in stock");
        let mut hits = rank_documents(&fixture_documents(), &parsed);
        let data = Data {
            miners: vec![Miner {
                id: "antminer-z15-pro".into(),
                hashrate_ksol: Some(840.0),
                watts: Some(2780.0),
                ..Default::default()
            }],
            ..Default::default()
        };
        apply_continuity(&data, &mut hits, &parsed);
        let vendor = hits
            .iter()
            .find(|hit| hit.doc.id == "tool:vendors")
            .unwrap();
        assert!(vendor.doc.href.contains("machine=antminer-z15-pro"));
        assert!(vendor.doc.href.contains("region=UK"));
        assert!(vendor.doc.href.contains("state=in_stock"));
    }

    #[test]
    fn current_directory_answers_z15_pro_uk_in_stock() {
        let data = crate::data::load(std::path::Path::new("data")).unwrap();
        let parsed = parse_search("Z15 Pro UK in stock");
        let hits = rank_documents(&build_search_index(&data), &parsed);
        assert!(
            hits.iter().any(|hit| hit.doc.id == "asic:antminer-z15-pro"),
            "the manufacturer-backed ASIC record must remain visible"
        );
        assert!(
            hits.iter().any(|hit| hit.doc.kind == SearchKind::Vendor),
            "the current dataset must provide at least one UK seller result"
        );
        assert!(hits
            .iter()
            .filter(|hit| hit.doc.kind == SearchKind::Vendor)
            .all(
                |hit| location_matches(&hit.doc, parsed.location.as_ref().unwrap())
                    && hit.doc.availability.contains(&AvailabilityIntent::Current)
            ));
    }

    #[test]
    fn vendor_intent_keeps_the_machine_in_the_directory_link() {
        let data = Data {
            miners: vec![Miner {
                id: "antminer-z15-pro".into(),
                maker: "Bitmain".into(),
                model: "Antminer Z15 Pro".into(),
                hashrate_ksol: Some(840.0),
                watts: Some(2780.0),
                ..Default::default()
            }],
            ..Default::default()
        };
        let parsed = parse_search("where to buy Z15 Pro UK in stock");
        let mut hits = rank_documents(&build_search_index(&data), &parsed);
        apply_continuity(&data, &mut hits, &parsed);
        let directory = hits
            .iter()
            .find(|hit| hit.doc.id == "tool:vendors")
            .unwrap();
        assert!(directory.doc.href.contains("machine=antminer-z15-pro"));
        assert!(directory.doc.href.contains("region=UK"));
    }

    #[test]
    fn pool_intent_keeps_the_coin_and_payout_in_the_comparison_link() {
        let data = Data {
            coins: vec![Coin {
                id: "zcash".into(),
                name: "Zcash".into(),
                symbol: "ZEC".into(),
                status: "active".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let parsed = parse_search("Zcash PPLNS pools");
        let mut hits = rank_documents(&build_search_index(&data), &parsed);
        apply_continuity(&data, &mut hits, &parsed);
        let comparison = hits.iter().find(|hit| hit.doc.id == "tool:pools").unwrap();
        assert!(comparison.doc.href.contains("coin=zcash"));
        assert!(comparison.doc.href.contains("scheme=PPLNS"));
        assert!(comparison.doc.href.ends_with("#pools"));
    }
}
