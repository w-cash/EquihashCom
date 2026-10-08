use crate::data::{Coin, Data, Listing, Miner, Vendor};
use crate::fmt;
use crate::views::calc;
use crate::views::layout::{ext, layout, Page};
use crate::views::logo::{self, At};
use maud::{html, Markup};

const POWER_USD_KWH: f64 = 0.08;
const POOL_FEE_PCT: f64 = 1.0;

#[derive(Clone, Copy)]
struct Economics<'a> {
    coin: &'a Coin,
    coins_day: f64,
    revenue_day: f64,
    electricity_day: f64,
    margin_day: f64,
}

fn economics<'a>(coin: &'a Coin, miner: &Miner) -> Option<Economics<'a>> {
    if !coin.active() || coin.params() != miner.equihash {
        return None;
    }
    let hashrate = miner.hashrate_ksol?;
    if calc::outweighs_network(coin, hashrate) {
        return None;
    }
    let estimate = calc::estimate(
        hashrate,
        miner.watts?,
        POWER_USD_KWH,
        POOL_FEE_PCT,
        coin.price_usd,
        coin.block_reward_miner.as_ref()?.value,
        coin.network.hashrate?,
        coin.network.block_time_target_s?,
    )?;
    Some(Economics {
        coin,
        coins_day: estimate.coins_day,
        revenue_day: estimate.revenue_day?,
        electricity_day: estimate.power_day,
        margin_day: estimate.profit_day?,
    })
}

fn miner_economics<'a>(d: &'a Data, miner: &Miner) -> Vec<Economics<'a>> {
    let mut rows: Vec<_> = d
        .coins
        .iter()
        .filter_map(|coin| economics(coin, miner))
        .collect();
    rows.sort_by(|a, b| b.margin_day.total_cmp(&a.margin_day));
    rows
}

fn best_economics<'a>(d: &'a Data, miner: &Miner) -> Option<Economics<'a>> {
    miner_economics(d, miner).into_iter().next()
}

fn machine_image(id: &str) -> Option<(&'static str, u32, u32)> {
    match id {
        "antminer-z15-pro" => Some((
            "/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp",
            1200,
            1200,
        )),
        "antminer-z15" | "antminer-z15j" | "antminer-z15e" => Some((
            "/static/shop/machines/antminer-z15.8dc9fd7a98.webp",
            924,
            1000,
        )),
        "innosilicon-a9pp-zmaster" => Some((
            "/static/shop/machines/innosilicon-a9pp-zmaster.e4b128723d.webp",
            1000,
            680,
        )),
        "antminer-z11" => Some((
            "/static/shop/machines/antminer-z11.1c5e36ca87.webp",
            1200,
            630,
        )),
        "antminer-z9" => Some((
            "/static/shop/machines/antminer-z9.4e480dbe7c.webp",
            1140,
            586,
        )),
        "antminer-z9-mini" => Some((
            "/static/shop/machines/antminer-z9-mini.712b4180db.webp",
            1140,
            525,
        )),
        _ => None,
    }
}

fn money(value: Option<f64>) -> String {
    match value {
        Some(v) if v < -0.005 => format!("−${:.2}", -v),
        Some(v) => format!("${:.2}", v.max(0.0)),
        None => "n/a".into(),
    }
}

fn price(listing: &Listing) -> String {
    match (listing.price_amount, listing.price_currency.as_deref()) {
        (Some(value), Some("USD")) => format!("${}", fmt::int(Some(value))),
        (Some(value), Some("EUR")) => format!("€{}", fmt::int(Some(value))),
        (Some(value), Some("GBP")) => format!("£{}", fmt::int(Some(value))),
        (Some(value), Some(code)) => format!("{code} {}", fmt::int(Some(value))),
        (Some(value), None) => fmt::int(Some(value)),
        _ => "Quote".into(),
    }
}

fn availability_rank(value: Option<&str>) -> u8 {
    match value {
        Some("in_stock") => 0,
        Some("dispatch_claim") => 1,
        Some("quote") => 2,
        Some("backorder") => 3,
        Some("preorder") | Some("waitlist") => 4,
        Some("sold_out") => 6,
        _ => 5,
    }
}

fn vendor<'a>(d: &'a Data, listing: &Listing) -> Option<&'a Vendor> {
    d.vendors
        .iter()
        .find(|vendor| vendor.id == listing.vendor_id)
}

fn listing_rows<'a>(d: &'a Data, miner: &Miner) -> Vec<&'a Listing> {
    let mut rows: Vec<_> = d
        .listings
        .iter()
        .filter(|listing| listing.miner_id == miner.id)
        .collect();
    rows.sort_by(|a, b| {
        availability_rank(a.availability.as_deref())
            .cmp(&availability_rank(b.availability.as_deref()))
            .then_with(|| a.price_currency.cmp(&b.price_currency))
            .then_with(|| match (a.price_amount, b.price_amount) {
                (Some(x), Some(y)) => x.total_cmp(&y),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                _ => std::cmp::Ordering::Equal,
            })
    });
    rows
}

fn spec_basis(miner: &Miner) -> Markup {
    if miner.spec_basis.as_deref() == Some("market_reported") {
        html! { span class="asic-basis market" { "Market-reported spec" } }
    } else {
        html! { span class="asic-basis manufacturer" { "Manufacturer spec" } }
    }
}

fn machine_visual(miner: &Miner, class: &str) -> Markup {
    match machine_image(&miner.id) {
        Some((src, width, height)) => html! {
            div class=(class) { img src=(src) alt={(miner.maker) " " (miner.model)} width=(width) height=(height) loading="lazy" decoding="async"; }
        },
        None => html! {
            div class={(class) " asic-model-tile"} aria-label={(miner.model) " image pending manufacturer publication"} {
                span { (miner.maker.to_uppercase()) }
                strong { (miner.model.replace("Antminer ", "")) }
                small { "IMAGE PENDING" }
            }
        },
    }
}

pub fn index(d: &Data) -> Markup {
    let mut miners: Vec<_> = d.miners.iter().collect();
    miners.sort_by(|a, b| {
        b.hashrate_ksol
            .unwrap_or_default()
            .total_cmp(&a.hashrate_ksol.unwrap_or_default())
    });
    let max_margin = miners
        .iter()
        .filter_map(|miner| best_economics(d, miner).map(|e| e.margin_day.max(0.0)))
        .fold(0.0_f64, f64::max);
    let models_with_offers = miners
        .iter()
        .filter(|miner| {
            d.listings
                .iter()
                .any(|listing| listing.miner_id == miner.id)
        })
        .count();
    layout(d, Page {
        title: "Equihash ASIC miners, profitability and prices",
        description: "Compare Equihash ASIC miners by hashrate, power, efficiency, release date, estimated operating margin and current source-linked vendor offers.",
        path: "/asics",
        nav: "asics",
    }, html! {
        div class="wrap page asic-index" {
            header class="page-head split-head" {
                div { p class="eyebrow" { "EQUIHASH ASIC INDEX" } h1 { "ASIC miners" } p class="lede" { "Manufacturer and clearly labelled market records, ordered from highest to lowest hashrate. Open a machine for coin economics and current seller offers." } }
                dl class="asic-index-summary" {
                    div { dt { "Models" } dd { (miners.len()) } }
                    div { dt { "With offers" } dd { (models_with_offers) } }
                    div { dt { "Profit basis" } dd { "$0.08/kWh · 1% fee" } }
                }
            }
            div class="asic-method-bar" {
                p { strong { "Current estimate" } " uses sourced coin price, network hashrate, block time and miner reward. Electricity, pool fee and hardware specifications are shown separately." }
                a href="/calculator" { "Change assumptions →" }
            }
            div class="table-scroll asic-table-wrap" {
                table class="asic-table" {
                    thead { tr {
                        th scope="col" { "#" }
                        th scope="col" { "Model" }
                        th class="num" scope="col" { "Hashrate" }
                        th class="num" scope="col" { "Power" }
                        th class="num" scope="col" { "Efficiency" }
                        th scope="col" { "Ann. / spec date" }
                        th scope="col" { "Coin / algo" }
                        th class="num" scope="col" { "Gross / day" }
                        th scope="col" { "Operating margin / day" }
                        th class="num" scope="col" { "Offers" }
                    } }
                    tbody {
                        @for (rank, miner) in miners.iter().enumerate() {
                            @let best = best_economics(d, miner);
                            @let offers = d.listings.iter().filter(|listing| listing.miner_id == miner.id).count();
                            @let bar = best.map(|e| if max_margin > 0.0 { (e.margin_day.max(0.0) / max_margin * 100.0).clamp(0.0, 100.0) } else { 0.0 }).unwrap_or(0.0);
                            tr {
                                td class="asic-rank mono" { (rank + 1) }
                                th scope="row" class="asic-machine" {
                                    (machine_visual(miner, "asic-thumb"))
                                    span { a href={"/asics/" (miner.id)} { strong { (miner.maker) " " (miner.model) } } (spec_basis(miner)) }
                                }
                                td class="num mono" { strong { (fmt::opt_num(miner.hashrate_ksol)) } " kSol/s" }
                                td class="num mono" { (fmt::int(miner.watts)) " W" }
                                td class="num mono" { (miner.efficiency().map(|v| format!("{v:.2}")).unwrap_or("n/a".into())) " J/kSol" }
                                td class="mono" { (miner.released.as_deref().unwrap_or("n/a")) }
                                td { @if let Some(e) = best { (logo::chip(&e.coin.logo, &e.coin.name, At::Row, true)) strong { (e.coin.symbol.as_str()) } br; small { "Equihash " (miner.equihash.as_str()) } } @else { span class="na" { "No complete estimate" } } }
                                td class="num mono" { (money(best.map(|e| e.revenue_day))) }
                                td class="asic-margin" {
                                    strong class="mono" { (money(best.map(|e| e.margin_day))) }
                                    div class="asic-margin-track" aria-hidden="true" { i style={"width:" (format!("{bar:.1}%"))} {} }
                                }
                                td class="num" { @if offers > 0 { a href={"/asics/" (miner.id) "#offers"} { (offers) } } @else { span class="na" { "0" } } }
                            }
                        }
                    }
                }
            }
            p class="asic-table-note" { "Operating margin = estimated gross revenue − electricity at $0.08/kWh − 1% pool fee. It excludes hardware cost, cooling, tax, downtime, rejects, hosting and merged-mined rewards. Values move with price and network difficulty. Market-reported machines remain separate from manufacturer specifications." }
            @if !d.miners_not_listed.is_empty() {
                details class="asic-omitted" {
                    summary { "Records still awaiting a usable specification" }
                    ul { @for record in &d.miners_not_listed { li { strong { (record.model.as_str()) } " — " (record.reason.as_str()) } } }
                }
            }
        }
    })
}

pub fn detail(d: &Data, miner: &Miner) -> Markup {
    let economics = miner_economics(d, miner);
    let best = economics.first().copied();
    let offers = listing_rows(d, miner);
    let coins: Vec<_> = d
        .coins
        .iter()
        .filter(|coin| coin.active() && coin.params() == miner.equihash)
        .collect();
    let max_network = coins
        .iter()
        .filter_map(|coin| coin.network.hashrate)
        .fold(0.0_f64, f64::max);
    let min_network = coins
        .iter()
        .filter_map(|coin| coin.network.hashrate.filter(|value| *value > 0.0))
        .fold(f64::INFINITY, f64::min);
    let max_margin = economics
        .iter()
        .map(|row| row.margin_day.max(0.0))
        .fold(0.0_f64, f64::max);
    let available = offers
        .iter()
        .filter(|offer| {
            matches!(
                offer.availability.as_deref(),
                Some("in_stock" | "dispatch_claim" | "quote")
            )
        })
        .count();
    let title = format!(
        "{} {} profitability, coins and prices",
        miner.maker, miner.model
    );
    let desc = format!("{} {} Equihash {} specifications, current coin economics and source-linked seller offers sorted by availability and price.", miner.maker, miner.model, miner.equihash);
    let path = format!("/asics/{}", miner.id);
    layout(
        d,
        Page {
            title: &title,
            description: &desc,
            path: &path,
            nav: "asics",
        },
        html! {
            div class="wrap page asic-detail" {
                p class="crumb" { a href="/asics" { "All ASICs" } " / " (miner.maker) }
                section class="asic-hero" {
                    div class="asic-hero-copy" {
                        (spec_basis(miner))
                        h1 { (miner.maker) " " (miner.model) }
                        p class="lede" { "Equihash " (miner.equihash.as_str()) " machine record with sourced specifications, compatible network economics and current seller listings." }
                        div class="asic-hero-actions" {
                            a class="btn" href={"/calculator?hashrate=" (fmt::opt_num(miner.hashrate_ksol)) "&watts=" (fmt::opt_num(miner.watts))} { "Calculate with my power rate" }
                            @if !offers.is_empty() { a href="#offers" { "Compare " (offers.len()) " offers ↓" } }
                        }
                    }
                    (machine_visual(miner, "asic-hero-image"))
                }
                dl class="fact-strip asic-facts" {
                    div { dt { "Hashrate" } dd { (fmt::opt_num(miner.hashrate_ksol)) " kSol/s" } }
                    div { dt { "Power" } dd { (fmt::int(miner.watts)) " W" } }
                    div { dt { "Efficiency" } dd { (miner.efficiency().map(|v| format!("{v:.2} J/kSol")).unwrap_or("n/a".into())) } }
                    div { dt { "Ann. / spec date" } dd { (miner.released.as_deref().unwrap_or("n/a")) } }
                    div { dt { "Current offers" } dd { (offers.len()) " · " (available) " active/quote" } }
                }
                @if miner.spec_basis.as_deref() == Some("market_reported") {
                    aside class="asic-verification-note" { strong { "Specification status" } p { (miner.notes.as_deref().unwrap_or("This record is supported by public market pages; a manufacturer specification has not been located.")) } }
                }
                section class="asic-economics" aria-labelledby="economics-title" {
                    header class="section-head" {
                        div { p class="eyebrow" { "CURRENT NETWORK SNAPSHOT" } h2 id="economics-title" { "Coins and operating economics" } p { "One machine, $0.08/kWh electricity and a 1% pool fee. Network figures and coin prices use the current Equihash.com snapshot." } }
                        a href={"/calculator?hashrate=" (fmt::opt_num(miner.hashrate_ksol)) "&watts=" (fmt::opt_num(miner.watts))} { "Change assumptions →" }
                    }
                    @if let Some(row) = best {
                        dl class="asic-profit-strip" {
                            div { dt { "Highest complete estimate" } dd { (logo::chip(&row.coin.logo, &row.coin.name, At::Row, true)) (row.coin.name.as_str()) } }
                            div { dt { "Gross / day" } dd class="mono" { (money(Some(row.revenue_day))) } }
                            div { dt { "Electricity / day" } dd class="mono" { (money(Some(row.electricity_day))) } }
                            div { dt { "Operating margin / day" } dd class="mono positive" { (money(Some(row.margin_day))) } }
                        }
                    }
                    div class="asic-chart-grid" {
                        figure class="asic-chart" {
                            figcaption { strong { "Compatible network hashrate" } span { "Relative logarithmic scale" } }
                            @for coin in &coins {
                                @let value = coin.network.hashrate.unwrap_or(0.0);
                                @let width = if value > 0.0 && max_network > min_network && min_network.is_finite() { 12.0 + 88.0 * ((value.log10() - min_network.log10()) / (max_network.log10() - min_network.log10())).clamp(0.0, 1.0) } else if value > 0.0 { 100.0 } else { 0.0 };
                                div class="asic-chart-row" {
                                    span { (logo::chip(&coin.logo, &coin.name, At::Row, true)) a href={"/coin/" (coin.id.as_str())} { (coin.symbol.as_str()) } }
                                    div aria-hidden="true" { i style={"width:" (format!("{width:.1}%"))} {} }
                                    b class="mono" { (fmt::hashrate(coin.network.hashrate, coin.network.unit.as_deref().unwrap_or("Sol/s"))) }
                                }
                            }
                        }
                        figure class="asic-chart profitability" {
                            figcaption { strong { "Estimated operating margin" } span { "USD per machine per day" } }
                            @if economics.is_empty() { p class="na" { "No compatible coin has every input required for an estimate." } }
                            @for row in &economics {
                                @let width = if max_margin > 0.0 { (row.margin_day.max(0.0) / max_margin * 100.0).clamp(0.0, 100.0) } else { 0.0 };
                                div class="asic-chart-row" {
                                    span { (logo::chip(&row.coin.logo, &row.coin.name, At::Row, true)) (row.coin.symbol.as_str()) }
                                    div aria-hidden="true" { i class=[(row.margin_day < 0.0).then_some("negative")] style={"width:" (format!("{width:.1}%"))} {} }
                                    b class="mono" { (money(Some(row.margin_day))) }
                                }
                            }
                        }
                    }
                    @if !economics.is_empty() {
                        div class="table-scroll asic-coin-table-wrap" { table class="data asic-coin-table" {
                            thead { tr { th scope="col" { "Coin" } th class="num" scope="col" { "Network" } th class="num" scope="col" { "Coins / day" } th class="num" scope="col" { "Gross / day" } th class="num" scope="col" { "Electricity" } th class="num" scope="col" { "Margin / day" } th scope="col" {} } }
                            tbody { @for row in &economics { tr {
                                td { a href={"/coin/" (row.coin.id.as_str())} { (logo::chip(&row.coin.logo, &row.coin.name, At::List, true)) strong { (row.coin.name.as_str()) } } }
                                td class="num mono" { (fmt::hashrate(row.coin.network.hashrate, row.coin.network.unit.as_deref().unwrap_or("Sol/s"))) }
                                td class="num mono" { (format!("{:.6} {}", row.coins_day, row.coin.symbol)) }
                                td class="num mono" { (money(Some(row.revenue_day))) }
                                td class="num mono" { (money(Some(row.electricity_day))) }
                                td class="num mono" { strong { (money(Some(row.margin_day))) } }
                                td { a href={"/calculator?coin=" (row.coin.id.as_str()) "&hashrate=" (fmt::opt_num(miner.hashrate_ksol)) "&watts=" (fmt::opt_num(miner.watts))} { "Adjust" } }
                            } } }
                        } }
                    }
                    p class="asic-table-note" { "A missing coin estimate means at least one required price, reward, block-time or network input is unavailable, or one machine would be at least 10% of that network. Merged-mined rewards are excluded." }
                }
                section class="asic-offers" id="offers" aria-labelledby="offers-title" {
                    header class="section-head" {
                        div { p class="eyebrow" { "SOURCE-LINKED SELLER RECORDS" } h2 id="offers-title" { "Where this machine is listed" } p { "Active and quote-based offers first; prices are ordered within each currency because unlike currencies are not directly comparable." } }
                        a href={"/vendors?machine=" (miner.id.as_str())} { "Open vendor market →" }
                    }
                    @if offers.is_empty() {
                        div class="empty-state" { h3 { "No approved offer in the current snapshot" } p { "A model can appear in the ASIC index before a seller page passes the vendor and listing checks." } a href="/add-vendor" { "Submit a public listing" } }
                    } @else {
                        div class="table-scroll" { table class="asic-offer-table" {
                            thead { tr { th scope="col" { "Seller" } th scope="col" { "Price" } th scope="col" { "Availability / batch" } th scope="col" { "Ships to" } th scope="col" { "Checked" } th scope="col" { "Source" } } }
                            tbody { @for offer in &offers { @let seller = vendor(d, offer); tr {
                                td { @if let Some(seller) = seller { a href={"/vendors/" (seller.slug.as_str())} { strong { (seller.name.as_str()) } } @if let Some(base) = &seller.base_region { small { (base) } } } @else { strong { (offer.vendor_id.as_str()) } } }
                                td class="mono" { strong { (price(offer)) } @if offer.price_includes_vat == Some(false) { small { "ex VAT" } } }
                                td { strong { (offer.availability_label.as_deref().unwrap_or("Not stated")) } span class={"vendor-market-state state-" (offer.availability.as_deref().unwrap_or("unknown"))} { (offer.availability.as_deref().unwrap_or("unknown").replace('_', " ")) } }
                                td { (if offer.shipping_regions.is_empty() { "Not stated".into() } else { offer.shipping_regions.join(", ") }) }
                                td { time class="ago" datetime=[offer.observed_at.as_deref()] { (fmt::utc(offer.observed_at.as_deref())) } }
                                td { @if let Some(url) = &offer.source_url { (ext(url, "Open ↗")) } @else { "—" } }
                            } } }
                        } }
                        p class="asic-table-note" { "Seller pages can be stale. Equihash.com records the page claim and observation time; it does not verify physical inventory or fulfillment. Confirm model, serials, beneficiary, taxes, freight and warranty before paying." }
                    }
                }
                section class="asic-source-record" {
                    div { p class="eyebrow" { "SPECIFICATION RECORD" } h2 { "Source and status" } p { (miner.notes.as_deref().unwrap_or("No additional specification note.")) } }
                    dl {
                        div { dt { "Basis" } dd { @if miner.spec_basis.as_deref() == Some("market_reported") { "Public market record" } @else { "Manufacturer publication" } } }
                        div { dt { "Source" } dd { @if let Some(url) = &miner.source_url { (ext(url, miner.source_name.as_deref().unwrap_or("Open source"))) } @else { "n/a" } } }
                        div { dt { "Checked" } dd { (fmt::utc(miner.fetched_at.as_deref())) } }
                    }
                }
            }
        },
    )
}
