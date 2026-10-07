//! Equihash hashpower marketplace coverage. Kept separate from the pool directory because the
//! marketplace buyer, rather than the seller, chooses which pool and coin receive the work.

use crate::data::Data;
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct HashpowerQuery {
    pub speed_msol: Option<String>,
    pub hours: Option<String>,
    pub bid: Option<String>,
}

fn input(value: &Option<String>, default: f64, min: f64, max: f64) -> f64 {
    value
        .as_deref()
        .and_then(|v| v.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v >= min && *v <= max)
        .unwrap_or(default)
}

fn speed(d: &Data) -> String {
    fmt::hashrate(
        d.hashpower.total_speed_gsol.map(|v| v * 1_000_000_000.0),
        "Sol/s",
    )
}

fn orders(value: Option<u32>) -> String {
    value
        .map(|v| fmt::group(v as i128))
        .unwrap_or_else(|| fmt::NA.into())
}

fn bid(value: Option<f64>) -> String {
    value
        .map(|v| format!("{} BTC/GSol/day", fmt::num_short(v)))
        .unwrap_or_else(|| fmt::NA.into())
}

pub fn home_market(d: &Data) -> Markup {
    html! {
        section class="ed-market" aria-labelledby="market-title" {
            div class="wrap ed-market-grid" {
                header {
                    p class="ed-section-index" { "HASHPOWER MARKET" }
                    h2 id="market-title" { "Sell Equihash work or mine a pool directly." }
                    p { "NiceHash connects Equihash miners with buyers. The buyer chooses the destination; a direct pool account pays the coin that pool mines." }
                    a href="/hashpower" { "Compare both routes →" }
                }
                dl class="ed-market-tape" {
                    div { dt { "NiceHash Equihash speed" } dd { (speed(d)) } }
                    div { dt { "Active buy orders" } dd { (orders(d.hashpower.active_orders)) } }
                    div { dt { "Top standard bid" } dd { (bid(d.hashpower.top_standard_btc_per_gsol_day)) } }
                    div { dt { "Observed" } dd { (fmt::utc(d.hashpower.observed_at.as_deref())) } }
                }
                p class="ed-market-note" { "Marketplace snapshot from the NiceHash public order book. Bid prices are not guaranteed miner revenue. Selling hashpower does not itself create a WEC payout." }
            }
        }
    }
}

pub fn render(d: &Data, q: &HashpowerQuery) -> Markup {
    let default_bid = d.hashpower.top_standard_btc_per_gsol_day.unwrap_or(0.0);
    let speed_msol = input(&q.speed_msol, 0.84, 0.001, 1_000_000.0);
    let hours = input(&q.hours, 24.0, 0.1, 8_760.0);
    let order_bid = input(&q.bid, default_bid, 0.0, 1_000_000.0);
    let base_cost = order_bid * (speed_msol / 1_000.0) * (hours / 24.0);
    let service_fee = base_cost * 0.03;
    let order_fee = 0.00001;
    let total_cost = base_cost + service_fee + order_fee;
    layout(
        d,
        Page {
            title: "Equihash hashpower market: NiceHash and direct pool mining",
            description: "Current NiceHash Equihash order-book data, Z15 connection details and a plain comparison with direct pool and merged mining.",
            path: "/hashpower",
            nav: "hashpower",
        },
        html! {
            div class="wrap page market-page" {
                header class="page-head market-head" {
                    div {
                        p class="eyebrow" { "EQUIHASH HASHPOWER" }
                        h1 { "Marketplace or pool?" }
                        p class="lede" { "A Z15 can sell Equihash work to NiceHash or point at a mining pool. These are different products, with different payouts and control." }
                    }
                    p class="market-definition" { strong { "NiceHash is a marketplace." } " Buyers pay for Equihash hashrate and choose the pool receiving it. It is listed here separately from mining pools." }
                }

                section aria-labelledby="snapshot-title" {
                    div class="section-head" {
                        div { h2 id="snapshot-title" { "NiceHash order-book snapshot" } p { "Public BTC market for the EQUIHASH algorithm. Values can move between refreshes." } }
                        @if let Some(url) = d.hashpower.source_url.as_deref() { (ext(url, "Open source API ↗")) }
                    }
                    dl class="market-facts" {
                        div { dt { "Connected speed" } dd { (speed(d)) } small { (fmt::num_short(d.hashpower.total_speed_gsol.unwrap_or(0.0))) " GSol/s" } }
                        div { dt { "Active buy orders" } dd { (orders(d.hashpower.active_orders)) } small { (orders(d.hashpower.standard_orders)) " standard · " (orders(d.hashpower.fixed_orders)) " fixed" } }
                        div { dt { "Top standard bid" } dd { (bid(d.hashpower.top_standard_btc_per_gsol_day)) } small { "highest visible order price" } }
                        div { dt { "Top fixed bid" } dd { (bid(d.hashpower.top_fixed_btc_per_gsol_day)) } small { "highest visible order price" } }
                    }
                    p class="source-line" { "Observed " time datetime=[d.hashpower.observed_at.as_deref()] { (fmt::utc(d.hashpower.observed_at.as_deref())) } ". " (d.hashpower.note.as_deref().unwrap_or("Order-book prices change continuously.")) }
                }

                section class="market-compare" aria-labelledby="compare-title" {
                    header { p class="eyebrow" { "DECISION TABLE" } h2 id="compare-title" { "Choose the payout you want." } }
                    div class="table-scroll" { table class="data decision-table" {
                        thead { tr { th scope="col" { "Route" } th scope="col" { "You receive" } th scope="col" { "Who chooses the pool" } th scope="col" { "Merged-mining result" } th scope="col" { "Use it when" } } }
                        tbody {
                            tr { th scope="row" { "Direct pool" } td { "The pool's coin payout" } td { "You" } td { "A supporting ZEC pool can also pay WEC" } td { "You want coin-level control and pool terms" } }
                            tr { th scope="row" { "NiceHash seller" } td { "BTC from sold hashpower" } td { "The buyer" } td { "No separate WEC payout to you" } td { "You prefer a BTC-denominated marketplace payout" } }
                            tr { th scope="row" { "NiceHash buyer" } td { "The target pool's payout" } td { "You, as buyer" } td { "Depends on the destination pool and account" } td { "You want temporary Equihash capacity" } }
                        }
                    } }
                    p class="market-caution" { strong { "Do not add the NiceHash bid to a pool revenue estimate." } " They are alternative routes for the same hashrate. Compare net payouts over the same time window after fees, rejects and downtime." }
                }

                section class="rental-estimator" aria-labelledby="rental-title" {
                    header {
                        p class="eyebrow" { "BUY HASHPOWER" }
                        h2 id="rental-title" { "Estimate an Equihash order" }
                        p { "This calculates the cost of the requested hashrate at one bid price. It does not estimate the coins a destination pool will return." }
                    }
                    form action="/hashpower#rental-title" method="get" class="rental-form" {
                        label { span { "Hashrate" } div { input name="speed_msol" type="number" min="0.001" max="1000000" step="0.001" value=(fmt::num_short(speed_msol)); b { "MSol/s" } } }
                        label { span { "Duration" } div { input name="hours" type="number" min="0.1" max="8760" step="0.1" value=(fmt::num_short(hours)); b { "hours" } } }
                        label { span { "Order price" } div { input name="bid" type="number" min="0" max="1000000" step="0.0001" value=(fmt::num_short(order_bid)); b { "BTC/GSol/day" } } }
                        button class="btn" type="submit" { "Calculate cost" }
                    }
                    dl class="rental-output" {
                        div { dt { "Hashpower at bid" } dd { (format!("{base_cost:.8} BTC")) } }
                        div { dt { "Buyer service fee" } dd { (format!("{service_fee:.8} BTC")) } small { "3% of delivered hashpower cost" } }
                        div { dt { "New order fee" } dd { "0.00001000 BTC" } small { "non-refundable" } }
                        div class="rental-total" { dt { "Estimated funds" } dd { (format!("{total_cost:.8} BTC")) } }
                    }
                    p class="source-line" { "Formula: bid × GSol/s × days, plus NiceHash's published 3% buyer service fee and 0.00001 BTC new-order fee. Unused funds from unfilled or cancelled orders are returned without a service fee. " (ext("https://www.nicehash.com/support/hash-power-marketplace/general-help/what-is-the-service-fee-for-buyers", "Read NiceHash fee policy ↗")) }
                }

                section class="market-connect" aria-labelledby="connect-title" {
                    header { p class="eyebrow" { "Z15 / SELL HASHPOWER" } h2 id="connect-title" { "NiceHash connection record" } p { "Use the current endpoint from NiceHash and verify it against their generator before changing a running miner." } }
                    ol class="connect-ledger" {
                        li { span { "01" } div { h3 { "Create or sign in to a NiceHash account" } p { "The mining username is normally the BTC mining address shown by NiceHash, with an optional worker suffix." } } }
                        li { span { "02" } div { h3 { "Select the EQUIHASH algorithm" } p { "This is the original Equihash market used by Zcash-class 200,9 ASICs, including the Antminer Z15 family." } } }
                        li { span { "03" } div { h3 { "Set the stratum endpoint" } code { (d.hashpower.stratum_url.as_deref().unwrap_or("See the NiceHash stratum generator")) } p { "XNSUB support depends on miner firmware. NiceHash says to append " code { "#xnsub" } " and update firmware if the worker does not report XNSUB." } } }
                        li { span { "04" } div { h3 { "Verify the worker before moving the farm" } p { "Start with one machine. Check accepted shares, reported hashrate, payout address and XNSUB status, then compare its net result with direct pool mining." } } }
                    }
                    div class="market-actions" {
                        @if let Some(url) = d.hashpower.marketplace_url.as_deref() { (ext(url, "Open NiceHash marketplace ↗")) }
                        @if let Some(url) = d.hashpower.connection_guide_url.as_deref() { (ext(url, "Read NiceHash XNSUB notice ↗")) }
                        a href="/pools?coin=zcash#pools" { "Compare Zcash pools →" }
                    }
                }

                aside class="market-wec" {
                    div { p class="eyebrow" { "ZEC + WEC" } h2 { "Merged mining stays a pool decision." } }
                    p { "To receive both ZEC and WEC as a miner, connect to a pool that explicitly supports both and enter the required payout addresses. When you sell hashpower, the buyer controls the destination and NiceHash pays you in BTC." }
                    a href="/merged-mining" { "Read the merged-mining guide →" }
                }
            }
        },
    )
}
