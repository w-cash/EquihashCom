use crate::data::Data;
use crate::fmt;
use maud::{html, Markup};

fn zcash_points(d: &Data) -> Vec<&crate::data::MarketHistoryPoint> {
    let mut points: Vec<_> = d
        .market_history
        .iter()
        .filter(|point| point.coin_id == "zcash")
        .collect();
    points.sort_by(|a, b| a.observed_at.cmp(&b.observed_at));
    points
}

pub fn zcash_network(d: &Data) -> Markup {
    let points = zcash_points(d);
    let max_hashrate = points
        .iter()
        .filter_map(|point| point.network_hashrate_sol_s)
        .fold(0.0_f64, f64::max);
    let span_days = match (points.first(), points.last()) {
        (Some(first), Some(last)) => first
            .observed_at
            .as_deref()
            .and_then(|start| chrono::DateTime::parse_from_rfc3339(start).ok())
            .zip(
                last.observed_at
                    .as_deref()
                    .and_then(|end| chrono::DateTime::parse_from_rfc3339(end).ok()),
            )
            .map(|(start, end)| (end - start).num_days().max(0)),
        _ => None,
    };
    html! {
        section class="market-history" aria-labelledby="network-history-title" {
            header class="section-head" {
                div {
                    p class="eyebrow" { "FIRST-PARTY SNAPSHOT HISTORY" }
                    h2 id="network-history-title" { "Zcash hashrate and difficulty history" }
                    p { "Each point is retained from a published Equihash.com snapshot. Missing intervals stay missing; the chart does not interpolate or backfill." }
                }
                div class="history-range" aria-label="History range" {
                    button type="button" data-history-days="7" aria-pressed="true" { "7D" }
                    button type="button" data-history-days="30" aria-pressed="false" { "30D" }
                    button type="button" data-history-days="90" aria-pressed="false" { "90D" }
                }
            }
            @if points.is_empty() {
                p class="na" { "No retained Zcash history is available yet." }
            } @else {
                div class="history-bars" data-history-series {
                    @for point in &points {
                        @let width = point.network_hashrate_sol_s.map(|value| if max_hashrate > 0.0 { value / max_hashrate * 100.0 } else { 0.0 }).unwrap_or(0.0);
                        div class="history-bar-row" data-history-at=[point.observed_at.as_deref()] {
                            time datetime=[point.observed_at.as_deref()] { (fmt::date(point.observed_at.as_deref())) }
                            div class="history-track" aria-hidden="true" { i style={"width:" (format!("{width:.1}%"))} {} }
                            b class="mono" { (fmt::hashrate(point.network_hashrate_sol_s, "Sol/s")) }
                            span class="mono" { "difficulty " (fmt::compact(point.difficulty)) }
                        }
                    }
                }
                p class="small history-status" role="status" {
                    (points.len()) " retained observations"
                    @if let Some(days) = span_days { " across " (days) " days" }
                    ". The 30- and 90-day views will fill as scheduled snapshots accumulate."
                }
            }
            p class="small" { a href="/data/market-history.json" { "Download the observations and assumptions →" } }
        }
    }
}

pub fn z15_economics(d: &Data) -> Markup {
    let points: Vec<_> = zcash_points(d)
        .into_iter()
        .filter(|point| point.miner_id == "antminer-z15-pro")
        .collect();
    let offers: Vec<_> = d
        .offer_history
        .iter()
        .filter(|point| point.miner_id == "antminer-z15-pro")
        .rev()
        .take(12)
        .collect();
    html! {
        section class="market-history" aria-labelledby="z15-history-title" {
            header class="section-head" {
                div {
                    p class="eyebrow" { "OBSERVED, NOT FORECAST" }
                    h2 id="z15-history-title" { "Z15 Pro revenue and seller history" }
                    p { "Revenue uses the manufacturer-rated 840 kSol/s and 2,780 W, a 1% pool fee and $0.08/kWh. Every row keeps its network, price and source time." }
                }
                div class="history-range" aria-label="History range" {
                    button type="button" data-history-days="7" aria-pressed="true" { "7D" }
                    button type="button" data-history-days="30" aria-pressed="false" { "30D" }
                    button type="button" data-history-days="90" aria-pressed="false" { "90D" }
                }
            }
            @if points.is_empty() {
                p class="na" { "No retained economics history is available yet." }
            } @else {
                div class="table-scroll" { table class="data history-table" data-history-series {
                    thead { tr { th { "Observed" } th class="num" { "Network" } th class="num" { "ZEC price" } th class="num" { "Gross / day" } th class="num" { "Margin / day" } } }
                    tbody { @for point in &points { tr data-history-at=[point.observed_at.as_deref()] {
                        td { time datetime=[point.observed_at.as_deref()] { (fmt::utc(point.observed_at.as_deref())) } }
                        td class="num mono" { (fmt::hashrate(point.network_hashrate_sol_s, "Sol/s")) }
                        td class="num mono" { (point.price_usd.map(|value| format!("${value:.2}")).unwrap_or_else(|| "n/a".into())) }
                        td class="num mono" { (point.revenue_day_usd.map(|value| format!("${value:.2}")).unwrap_or_else(|| "n/a".into())) }
                        td class="num mono" { (point.operating_margin_day_usd.map(|value| format!("${value:.2}")).unwrap_or_else(|| "n/a".into())) }
                    } } }
                } }
            }
            @if !offers.is_empty() {
                details class="offer-history" {
                    summary { "Latest retained seller observations (" (offers.len()) ")" }
                    div class="table-scroll" { table class="data mini" {
                        thead { tr { th { "Seller record" } th class="num" { "Price" } th { "State" } th { "Observed" } } }
                        tbody { @for offer in offers { tr {
                            td { (offer.vendor_id.as_str()) }
                            td class="num mono" { (offer.price_amount.map(|value| format!("{} {value:.0}", offer.price_currency.as_deref().unwrap_or(""))).unwrap_or_else(|| "n/a".into())) }
                            td { (offer.availability.as_deref().unwrap_or("unknown").replace('_', " ")) }
                            td { time datetime=[offer.observed_at.as_deref()] { (fmt::utc(offer.observed_at.as_deref())) } }
                        } } }
                    } }
                }
            }
            p class="small" { "Seller observations are attributed page claims, not physical inventory checks. Currencies and tax treatment are not normalized. " a href="/data/market-history.json" { "Open JSON →" } }
        }
    }
}
