//! Source-linked Equihash ASIC vendor directory. This is an editorial register, not a shop:
//! there is no checkout, paid placement, inferred stock or fulfillment endorsement.

use crate::data::{Data, Listing, Vendor, VendorResearch};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};
use serde::Deserialize;
use std::collections::BTreeSet;

const SELLER_REGIONS: &[&str] = &[
    "Manufacturer",
    "Asia-Pacific",
    "China & Hong Kong",
    "Europe",
    "United Kingdom",
    "United States",
    "Other",
];

const RESEARCH_REGIONS: &[&str] = &[
    "Asia-Pacific",
    "Middle East",
    "Europe",
    "North America",
    "Africa",
    "Latin America",
];

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct BuyQuery {
    pub machine: Option<String>,
    pub base: Option<String>,
    pub region: Option<String>,
    pub state: Option<String>,
}

fn seller_region(v: &Vendor) -> &str {
    if v.channel.as_deref() == Some("manufacturer") {
        "Manufacturer"
    } else {
        v.base_region.as_deref().unwrap_or("Other")
    }
}

fn site_key(url: Option<&str>) -> String {
    fmt::host(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn research_for<'a>(d: &'a Data, vendor: &Vendor) -> Option<&'a VendorResearch> {
    let key = site_key(vendor.url.as_deref());
    (!key.is_empty()).then_some(())?;
    d.vendor_research
        .iter()
        .find(|record| site_key(record.website.as_deref()) == key)
}

fn trust_label(tier: &str) -> &'static str {
    match tier {
        "A" => "Strongest evidence",
        "B" => "Credible, with caveats",
        "C" => "High diligence",
        "D" => "Warning only",
        _ => "Coverage gap",
    }
}

fn vendor_type_label(kind: &str) -> String {
    kind.replace('_', " ")
}

fn source_list(record: &VendorResearch) -> Markup {
    html! {
        ul class="vendor-evidence-sources" {
            @for url in &record.source_urls {
                li { (ext(url, &fmt::host(Some(url)).split('/').next().unwrap_or("Source"))) }
            }
        }
    }
}

fn research_record(record: &VendorResearch) -> Markup {
    let rank = record
        .global_trust_rank
        .map(|rank| format!("#{rank}"))
        .unwrap_or_else(|| "—".into());
    let score = record
        .editorial_trust_score_100
        .map(|score| format!("{score}/100"))
        .unwrap_or_else(|| "n/a".into());
    html! {
        article id={"vendor-" (&record.id)} class="vendor-research-card" data-vendor-research data-region=(&record.region) data-tier=(&record.trust_tier) data-search={(record.vendor) " " (record.country) " " (record.equihash_z15)} {
            header class="vendor-research-head" {
                span class="vendor-rank mono" { (rank) }
                div {
                    p class="seller-label" { (vendor_type_label(&record.vendor_type)) " · " (&record.country) }
                    h3 {
                        @if let Some(url) = &record.website { (ext(url, &record.vendor)) }
                        @else { (&record.vendor) }
                    }
                }
                div class={"vendor-tier tier-" (record.trust_tier.to_ascii_lowercase())} {
                    span { "TIER " (&record.trust_tier) }
                    strong { (trust_label(&record.trust_tier)) }
                    small { "Evidence score " (score) }
                }
            }
            p class="vendor-research-note" { (&record.notes) }
            div class="vendor-z15-record" {
                strong { "Equihash / Z15" }
                p { (&record.equihash_z15) }
            }
            details class="vendor-evidence" {
                summary { "Identity, payment, shipping and warranty evidence" }
                dl {
                    div { dt { "Legal identity and location" } dd { (&record.legal_identity_location) } }
                    div { dt { "History and reputation" } dd { (&record.history_and_reputation) } }
                    div { dt { "Payment protection" } dd { (&record.payment_protection) } }
                    div { dt { "Shipping and customs" } dd { (&record.shipping_customs) } }
                    div { dt { "Pickup" } dd { (&record.pickup) } }
                    div { dt { "Warranty / RMA" } dd { (&record.warranty_rma) } }
                    div { dt { "Delivery assessment" } dd { (&record.estimated_delivery_probability) } }
                    div { dt { "Last verified" } dd { (record.last_verified.as_deref().unwrap_or("n/a")) } }
                }
                h4 { "Evidence links" }
                (source_list(record))
            }
        }
    }
}

fn warning_record(record: &VendorResearch) -> Markup {
    html! {
        article id={"vendor-" (&record.id)} class="vendor-warning-card" data-vendor-warning {
            header {
                div { p class="seller-label" { (&record.country) " · WARNING RECORD" } h3 { (&record.vendor) } }
                span class="vendor-tier tier-d" { "TIER D" }
            }
            p { (&record.notes) }
            details class="vendor-evidence" {
                summary { "Why this record is excluded" }
                p { strong { "Identity and history: " } (&record.legal_identity_location) " " (&record.history_and_reputation) }
                p { strong { "Payment risk: " } (&record.payment_protection) }
                p { strong { "Equihash / Z15: " } (&record.equihash_z15) }
                h4 { "Evidence links" }
                (source_list(record))
            }
        }
    }
}

fn currency(amount: Option<f64>, code: Option<&str>) -> String {
    let Some(n) = amount.filter(|n| n.is_finite()) else {
        return "n/a".into();
    };
    let whole = fmt::group(n.round() as i128);
    match code.unwrap_or("") {
        "GBP" => format!("£{whole}"),
        "USD" => format!("${whole}"),
        "EUR" => format!("€{whole}"),
        c if !c.is_empty() => format!("{c} {whole}"),
        _ => whole,
    }
}

fn vendor_mark(v: &Vendor, head: bool) -> Markup {
    let class = if head {
        "vendor-logo vendor-logo-head"
    } else {
        "vendor-logo"
    };
    html! {
        @if let Some(src) = &v.logo.src {
            span class={(class) @if v.logo.tile { " tile" } @if v.logo.light { " light" }} data-kind=(v.logo.kind) {
                img src=(src) alt=[head.then(|| format!("{} logo", v.name))] width="112" height="64" decoding="async";
            }
        } @else {
            span class=(class) data-kind="fallback" role=[head.then_some("img")] aria-label=[head.then(|| format!("{} monogram", v.name))] {
                (v.logo.mono)
            }
        }
    }
}

fn channel_label(v: &Vendor) -> &str {
    match v.channel.as_deref() {
        Some("manufacturer") => "Manufacturer",
        Some("broker_hosting") => "Broker & hosting",
        _ => "Independent retailer",
    }
}

fn machine_name<'a>(d: &'a Data, id: &str) -> String {
    d.miners
        .iter()
        .find(|m| m.id == id)
        .map(|m| format!("{} {}", m.maker, m.model))
        .unwrap_or_else(|| id.to_string())
}

fn listing_priority(l: &Listing) -> u8 {
    match l.availability.as_deref() {
        Some("in_stock") => 0,
        Some("dispatch_claim") => 1,
        Some("quote") => 2,
        Some("backorder") => 3,
        Some("preorder") | Some("waitlist") => 4,
        Some("sold_out") => 6,
        _ => 5,
    }
}

fn listing_image(l: &Listing) -> (&str, u32, u32) {
    if let Some(src) = l.image.as_deref().filter(|src| src.starts_with("/static/")) {
        return (src, 1200, 1200);
    }
    match l.miner_id.as_str() {
        "antminer-z15-pro" => (
            "/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp",
            1200,
            1200,
        ),
        "antminer-z15" | "antminer-z15j" | "antminer-z15e" => (
            "/static/shop/machines/antminer-z15.8dc9fd7a98.webp",
            924,
            1000,
        ),
        "innosilicon-a9pp-zmaster" => (
            "/static/shop/machines/innosilicon-a9pp-zmaster.e4b128723d.webp",
            1000,
            680,
        ),
        "antminer-z11" => (
            "/static/shop/machines/antminer-z11.1c5e36ca87.webp",
            1200,
            630,
        ),
        "antminer-z9" => (
            "/static/shop/machines/antminer-z9.4e480dbe7c.webp",
            1140,
            586,
        ),
        "antminer-z9-mini" => (
            "/static/shop/machines/antminer-z9-mini.712b4180db.webp",
            1140,
            525,
        ),
        _ => (
            "/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp",
            1200,
            1200,
        ),
    }
}

fn vendor_model_groups<'a>(rows: &[&'a Listing]) -> Vec<Vec<&'a Listing>> {
    let mut groups: Vec<Vec<&'a Listing>> = Vec::new();
    for row in rows {
        if let Some(group) = groups
            .iter_mut()
            .find(|group| group.first().map(|item| &item.miner_id) == Some(&row.miner_id))
        {
            group.push(*row);
        } else {
            groups.push(vec![*row]);
        }
    }
    groups
}

fn vendor_model_slide(
    d: &Data,
    v: &Vendor,
    rows: &[&Listing],
    index: usize,
    count: usize,
) -> Markup {
    let listing = rows[0];
    let miner = d.miners.iter().find(|miner| miner.id == listing.miner_id);
    let (image, width, height) = listing_image(listing);
    let machine = machine_name(d, &listing.miner_id);
    let hashrate = miner
        .and_then(|miner| miner.hashrate_ksol)
        .or(listing.shop_hashrate_ksol);
    let watts = miner.and_then(|miner| miner.watts);
    let calculator = format!(
        "/calculator?hashrate={}&watts={}",
        fmt::opt_num(hashrate),
        fmt::opt_num(watts)
    );
    let all_sold_out = rows
        .iter()
        .all(|row| row.availability.as_deref() == Some("sold_out"));
    let lowest = rows
        .iter()
        .filter_map(|row| {
            row.price_amount
                .map(|price| (price, row.price_currency.as_deref()))
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    html! {
        article class={"ed-machine-slide vendor-offer-slide" @if index == 0 { " is-active" }} data-machine-slide aria-hidden=(if index == 0 { "false" } else { "true" }) role="group" aria-roledescription="slide" aria-label={(index + 1) " of " (count) ": " (&machine)} {
            header class="ed-machine-card-top" {
                span { "EQUIHASH MINER" }
                span class="ed-machine-record" { (rows.len()) @if rows.len() == 1 { " listing" } @else { " listings" } }
            }
            div class="ed-hero-machine-image vendor-offer-image" {
                img src=(image) alt={(machine) " listed by " (&v.name)} width=(width) height=(height) loading=[(index > 0).then_some("lazy")] decoding="async";
            }
            div class="ed-hero-machine-info vendor-offer-info" {
                p { "EQUIHASH " (miner.map(|m| m.equihash.as_str()).unwrap_or("—")) }
                h2 { (&machine) }
                dl class="vendor-offer-facts" {
                    div { dt { "Hashrate" } dd { (fmt::opt_num(hashrate)) " kSol/s" } }
                    div { dt { "Power" } dd { (fmt::opt_num(watts)) " W" } }
                    div { dt { "Efficiency" } dd { (fmt::opt_num(miner.and_then(|m| m.stated_efficiency_j_per_ksol))) " J/kSol" } }
                    div { dt { @if all_sold_out { "Last page price" } @else { "Lowest page price" } } dd { @if let Some((price, code)) = lowest { (currency(Some(price), code)) } @else { "n/a" } } }
                }
                nav aria-label={(&machine) " actions"} {
                    a href={"/asics/" (&listing.miner_id)} { "Specifications" }
                    a href=(calculator) { "Calculator" }
                    a href={"#vendor-listings-" (&listing.miner_id)} { "View listings" }
                }
            }
        }
    }
}

fn seller_hashrate_range(rows: &[&Listing]) -> String {
    let mut values = rows
        .iter()
        .filter_map(|row| row.shop_hashrate_ksol)
        .collect::<Vec<_>>();
    values.sort_by(f64::total_cmp);
    values.dedup_by(|a, b| a.total_cmp(b).is_eq());
    match (values.first().copied(), values.last().copied()) {
        (Some(low), Some(high)) if low != high => {
            format!(
                "{}–{} kSol/s",
                fmt::opt_num(Some(low)),
                fmt::opt_num(Some(high))
            )
        }
        (Some(value), _) => format!("{} kSol/s", fmt::opt_num(Some(value))),
        _ => "rating not stated".into(),
    }
}

fn vendor_offer_table(d: &Data, rows: &[&Listing]) -> Markup {
    let groups = vendor_model_groups(rows);
    html! {
        p class="vendor-offer-swipe" { "Swipe to compare all columns →" }
        div class="vendor-offer-table-wrap" {
            table class="vendor-offer-table" id="vendor-offer-table" {
                thead { tr { th scope="col" { "Model / option" } th scope="col" { "Price" } th scope="col" { "Seller status" } th scope="col" { "Ships to" } th scope="col" { "Checked" } th scope="col" { "Source" } } }
                @for group in groups {
                    @let machine = machine_name(d, &group[0].miner_id);
                    tbody id={"vendor-listings-" (&group[0].miner_id)} {
                        tr class="vendor-offer-model-row" {
                            th colspan="6" scope="rowgroup" { a href={"/asics/" (&group[0].miner_id)} { (&machine) } span { "Seller-listed " (seller_hashrate_range(&group)) }
                            }
                        }
                        @for row in group {
                            tr {
                                td data-label="Option" {
                                    strong { (row.availability_label.as_deref().unwrap_or("Not stated")) }
                                    @if let Some(condition) = &row.condition { small { (condition.replace('_', " ")) } }
                                }
                                td data-label="Price" class="mono" { strong { (currency(row.price_amount, row.price_currency.as_deref())) } @if row.price_includes_vat == Some(false) { small { "ex VAT" } } }
                                td data-label="Seller status" { span class={"vendor-market-state state-" (row.availability.as_deref().unwrap_or("unknown"))} { (row.availability.as_deref().unwrap_or("unknown").replace('_', " ")) } }
                                td data-label="Ships to" { @if row.shipping_regions.is_empty() { "Not stated" } @else { (row.shipping_regions.join(", ")) } }
                                td data-label="Checked" { time class="ago" datetime=[row.observed_at.as_deref()] { (fmt::utc(row.observed_at.as_deref())) } }
                                td data-label="Source" { @if let Some(url) = &row.source_url { (ext(url, "Open ↗")) } @else { "—" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn vendor_product_market(d: &Data, v: &Vendor, rows: &[&Listing]) -> Markup {
    let groups = vendor_model_groups(rows);
    let count = groups.len();
    html! {
        section class="vendor-product-stage" aria-labelledby="vendor-products-title" {
            header class="vendor-product-intro" {
                p class="eyebrow" { "EQUIHASH CATALOG" }
                h2 id="vendor-products-title" { (count) @if count == 1 { " miner" } @else { " miners" } " · " (rows.len()) @if rows.len() == 1 { " listing" } @else { " listings" } }
            }
            aside class="ed-hero-machine vendor-offer-carousel" data-machine-carousel=[(count > 1).then_some("")] data-carousel-noun="model" data-carousel-first-delay="2200" data-carousel-delay="5800" role="region" aria-roledescription=[(count > 1).then_some("carousel")] aria-label={(&v.name) " Equihash models"} {
                @if count > 1 {
                    nav class="ed-machine-switcher" data-machine-controls aria-label="Model carousel controls" hidden {
                        button type="button" data-machine-prev aria-label="Previous model" { "←" }
                        span class="ed-machine-position" aria-hidden="true" { strong data-machine-current { "1" } " / " (count) }
                        button class="ed-machine-toggle" type="button" data-machine-toggle aria-label="Pause carousel" aria-pressed="false" { "Ⅱ" }
                        button type="button" data-machine-next aria-label="Next model" { "→" }
                    }
                    button class="ed-machine-peek ed-machine-peek-prev" type="button" data-machine-peek-prev aria-label="Show previous model" hidden { span aria-hidden="true" { "‹" } }
                    button class="ed-machine-peek ed-machine-peek-next" type="button" data-machine-peek-next aria-label="Show next model" hidden { span aria-hidden="true" { "›" } }
                }
                div class="ed-machine-slides" {
                    @for (index, group) in groups.iter().enumerate() {
                        (vendor_model_slide(d, v, group, index, count))
                    }
                }
            }
            (vendor_offer_table(d, rows))
            p class="vendor-product-method" { "Seller pages can be stale. We record page claims; we do not verify inventory or fulfillment. Check the source and date before paying." }
        }
    }
}

fn market_offer_row(d: &Data, vendor: &Vendor, listing: &Listing, rank: usize) -> Markup {
    let (image, width, height) = listing_image(listing);
    let machine = machine_name(d, &listing.miner_id);
    html! {
        tr class="vendor-market-row" data-machine=(&listing.miner_id) data-base=(seller_region(vendor)) data-region=(listing.shipping_regions.join(",")) data-listing-state=(listing.availability.as_deref().unwrap_or("unknown")) data-vendor=(&vendor.id) {
            td class="vendor-market-rank mono" { (rank) }
            th scope="row" class="vendor-market-machine" {
                img src=(image) alt="" width=(width) height=(height) loading="lazy" decoding="async";
                span { a href={"/asics/" (&listing.miner_id)} { (&machine) } small { (fmt::opt_num(listing.shop_hashrate_ksol)) " kSol/s" } }
            }
            td data-label="Seller" class="vendor-market-seller" { a href={"/vendors/" (&vendor.slug)} { (&vendor.name) } small { (vendor.base_region.as_deref().unwrap_or("Base not stated")) } }
            td data-label="Price" class="vendor-market-price mono" { strong { (currency(listing.price_amount, listing.price_currency.as_deref())) } @if listing.price_includes_vat == Some(false) { small { "ex VAT" } } }
            td data-label="Seller status" { strong { (listing.availability_label.as_deref().unwrap_or("Not stated")) } span class={"vendor-market-state state-" (listing.availability.as_deref().unwrap_or("unknown"))} { (listing.availability.as_deref().unwrap_or("unknown").replace('_', " ")) } }
            td data-label="Ships to" { (if listing.shipping_regions.is_empty() { "Not stated".into() } else { listing.shipping_regions.join(", ") }) }
            td data-label="Checked" { time class="ago" datetime=[listing.observed_at.as_deref()] { (fmt::utc(listing.observed_at.as_deref())) } }
            td data-label="Source" { @if let Some(url) = &listing.source_url { (ext(url, "Open ↗")) } @else { "—" } }
        }
    }
}

fn matches(l: &Listing, machine: &str, region: &str, state: &str) -> bool {
    (machine.is_empty() || l.miner_id == machine)
        && (region.is_empty()
            || l.shipping_regions
                .iter()
                .any(|r| r.eq_ignore_ascii_case(region) || r == "Global"))
        && (state.is_empty() || l.availability.as_deref() == Some(state))
}

pub fn index(d: &Data, q: &BuyQuery) -> Markup {
    let machine = q.machine.as_deref().unwrap_or("");
    let base = q.base.as_deref().unwrap_or("");
    let region = q.region.as_deref().unwrap_or("");
    let state = q.state.as_deref().unwrap_or("");
    let visible: Vec<(&Vendor, Vec<&Listing>)> = d
        .vendors
        .iter()
        .filter_map(|v| {
            if research_for(d, v)
                .map(|record| record.trust_tier == "D")
                .unwrap_or(false)
            {
                return None;
            }
            if !base.is_empty() && seller_region(v) != base {
                return None;
            }
            let mut rows: Vec<&Listing> = d
                .listings
                .iter()
                .filter(|l| l.vendor_id == v.id && matches(l, machine, region, state))
                .collect();
            rows.sort_by(|a, b| {
                a.miner_id.cmp(&b.miner_id).then_with(|| {
                    a.price_amount
                        .partial_cmp(&b.price_amount)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
            });
            (!rows.is_empty()).then_some((v, rows))
        })
        .collect();
    let mut machines: Vec<_> = d
        .miners
        .iter()
        .filter(|m| d.listings.iter().any(|l| l.miner_id == m.id))
        .collect();
    machines.sort_by(|a, b| a.model.cmp(&b.model));
    let mut destinations: Vec<String> = d
        .listings
        .iter()
        .flat_map(|l| l.shipping_regions.iter().cloned())
        .collect();
    destinations.sort();
    destinations.dedup();
    let mut states: Vec<String> = d
        .listings
        .iter()
        .filter_map(|l| l.availability.clone())
        .collect();
    states.sort();
    states.dedup();
    let offers = visible.iter().map(|(_, rows)| rows.len()).sum::<usize>();
    let offer_models = visible
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|row| row.miner_id.as_str()))
        .collect::<BTreeSet<_>>()
        .len();
    let mut market_rows: Vec<(&Vendor, &Listing)> = visible
        .iter()
        .flat_map(|(vendor, rows)| rows.iter().map(move |listing| (*vendor, *listing)))
        .collect();
    market_rows.sort_by(|(vendor_a, listing_a), (vendor_b, listing_b)| {
        listing_priority(listing_a)
            .cmp(&listing_priority(listing_b))
            .then_with(|| {
                listing_b
                    .shop_hashrate_ksol
                    .partial_cmp(&listing_a.shop_hashrate_ksol)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| {
                listing_a
                    .price_amount
                    .partial_cmp(&listing_b.price_amount)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| vendor_a.name.cmp(&vendor_b.name))
    });
    let published: Vec<_> = d
        .vendor_research
        .iter()
        .filter(|record| matches!(record.trust_tier.as_str(), "A" | "B" | "C"))
        .collect();
    let warnings: Vec<_> = d
        .vendor_research
        .iter()
        .filter(|record| record.trust_tier == "D")
        .collect();
    layout(d, Page {
        title: "Equihash miner prices and vendors",
        description: "Compare source-linked Equihash ASIC prices, seller status, delivery regions and checked dates across miner models and vendors.",
        path: "/vendors",
        nav: "vendors",
    }, html! {
        div class="wrap page buy-page vendor-directory" {
            header class="page-head buy-head" {
                p class="eyebrow" { "EQUIHASH MINER MARKET" }
                h1 { "Miner prices by seller" }
                p class="lede" { "Compare seller-listed price, batch, delivery region and source date across Equihash ASIC models." }
            }
            dl class="vendor-directory-stats" {
                div { dt { "Sellers" } dd { (visible.len()) } }
                div { dt { "Listings" } dd { (offers) } }
                div { dt { "Models" } dd { (offer_models) } }
                div { dt { "Listings checked" } dd { (fmt::utc(d.listings_verified_at.as_deref())) } }
            }
            form class="filters buy-filters" id="buy-filters" action="/vendors" method="get" {
                div class="f" { label for="buy-machine" { "Machine" } select id="buy-machine" name="machine" data-buy-filter="machine" { option value="" selected[machine.is_empty()] { "All machines" } @for m in &machines { option value=(m.id) selected[m.id == machine] { (m.maker) " " (m.model) } } } }
                div class="f" { label for="buy-base" { "Seller base" } select id="buy-base" name="base" data-buy-filter="base" { option value="" selected[base.is_empty()] { "All seller regions" } @for r in SELLER_REGIONS { @if d.vendors.iter().any(|v| seller_region(v) == *r) { option value=(r) selected[*r == base] { (r) } } } } }
                div class="f" { label for="buy-region" { "Ships to" } select id="buy-region" name="region" data-buy-filter="region" { option value="" selected[region.is_empty()] { "All destinations" } @for r in &destinations { option value=(r) selected[r == region] { (r) } } } }
                div class="f" { label for="buy-state" { "Listing state" } select id="buy-state" name="state" data-buy-filter="state" { option value="" selected[state.is_empty()] { "All states" } @for s in &states { option value=(s) selected[s == state] { (s.replace('_', " ")) } } } }
                button class="buy-apply" type="submit" { "Apply" }
                p class="buy-count" id="buy-count" aria-live="polite" { (offers) @if offers == 1 { " listing" } @else { " listings" } " · " (visible.len()) @if visible.len() == 1 { " seller" } @else { " sellers" } }
            }
            @if visible.is_empty() { div class="empty-state" { h2 { "No vendor matches those filters" } p { "Clear a filter or send a public product page for review." } a href="/add-vendor" { "Add a vendor" } } }
            @if !market_rows.is_empty() {
                div class="vendor-market-table-wrap" {
                    table class="vendor-market-table" {
                        thead { tr { th scope="col" { "#" } th scope="col" { "Miner" } th scope="col" { "Seller" } th scope="col" { "Price" } th scope="col" { "Batch / status" } th scope="col" { "Ships to" } th scope="col" { "Checked" } th scope="col" { "Source" } } }
                        tbody {
                            @for (index, (vendor, listing)) in market_rows.iter().enumerate() {
                                (market_offer_row(d, vendor, listing, index + 1))
                            }
                        }
                    }
                }
                p class="vendor-market-disclosure" { "Seller pages can be stale. We record page claims; we do not verify inventory or fulfillment. Check the source and date before paying." }
            }
            details class="vendor-global-directory" {
                summary { strong { "Vendor research" } span { (published.len()) " companies · checked " (d.vendor_research_as_of.as_deref().unwrap_or("n/a")) } }
                div class="vendor-register-body" {
                header class="vendor-directory-section-head" {
                    p class="eyebrow" { "VENDOR RESEARCH" }
                    h2 id="global-vendors-title" { "Manufacturers and sellers by region" }
                }
                form class="vendor-research-filters" id="vendor-research-filters" role="search" {
                    div class="f" { label for="vendor-research-q" { "Search" } input id="vendor-research-q" type="search" name="vendor_q" placeholder="Vendor, country or Z15" autocomplete="off"; }
                    div class="f" { label for="vendor-research-region" { "Region" } select id="vendor-research-region" name="vendor_region" { option value="" { "All regions" } @for region in RESEARCH_REGIONS { option value=(region) { (region) } } } }
                    div class="f" { label for="vendor-research-tier" { "Evidence tier" } select id="vendor-research-tier" name="vendor_tier" { option value="" { "Tiers A–C" } option value="A" { "Tier A" } option value="B" { "Tier B" } option value="C" { "Tier C" } } }
                    button class="buy-apply" type="submit" { "Apply" }
                    p class="buy-count" id="vendor-research-count" aria-live="polite" { (published.len()) " researched vendors" }
                }
                nav class="vendor-region-index vendor-research-index" aria-label="Global research regions" {
                    @for region in RESEARCH_REGIONS {
                        @let count = published.iter().filter(|record| record.region == *region).count();
                        @if count > 0 { a href={"#research-region-" (region.to_ascii_lowercase().replace(' ', "-"))} { (region) span { (count) } } }
                    }
                }
                div id="vendor-research-empty" class="empty-state" hidden { h3 { "No research record matches those filters" } p { "Try a shorter name or clear a filter." } }
                @for region in RESEARCH_REGIONS {
                    @let records: Vec<_> = published.iter().filter(|record| record.region == *region).collect();
                    @if !records.is_empty() {
                        section id={"research-region-" (region.to_ascii_lowercase().replace(' ', "-"))} class="vendor-research-region" data-vendor-research-region {
                            header class="seller-region-head" { h3 { (region) } p { (records.len()) " records" } }
                            @for record in records { (research_record(record)) }
                        }
                    }
                }
                }
            }
            details class="vendor-warning-register" {
                summary { strong { "Warning records" } span { (warnings.len()) " excluded sellers" } }
                div class="vendor-register-body" {
                header class="vendor-directory-section-head" {
                    p class="eyebrow" { "WARNING REGISTER" }
                    h2 id="vendor-warnings-title" { "Sellers excluded from listing results" }
                }
                div class="vendor-warning-list" { @for record in &warnings { (warning_record(record)) } }
                }
            }
            footer class="vendor-directory-footer" { a href="/add-vendor" { "Add a vendor or correct a listing" } span { "Listings are free; ranking cannot be purchased." } }
        }
    })
}

pub fn vendor_page(d: &Data, v: &Vendor) -> Markup {
    let research = research_for(d, v);
    let warning = research
        .map(|record| record.trust_tier == "D")
        .unwrap_or(false);
    let mut rows: Vec<_> = if warning {
        Vec::new()
    } else {
        d.listings.iter().filter(|l| l.vendor_id == v.id).collect()
    };
    rows.sort_by(|a, b| {
        listing_priority(a)
            .cmp(&listing_priority(b))
            .then_with(|| {
                b.shop_hashrate_ksol
                    .partial_cmp(&a.shop_hashrate_ksol)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .then_with(|| a.title.cmp(&b.title))
    });
    let model_count = rows
        .iter()
        .map(|row| row.miner_id.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let latest_check = rows
        .iter()
        .filter_map(|row| row.observed_at.as_deref())
        .max()
        .or(v.observed_at.as_deref());
    let evidence_label = research.map(|record| match record.trust_tier.as_str() {
        "A" => "Extensive identity record",
        "B" => "Identity record reviewed",
        "C" => "Limited identity record",
        "D" => "Warning record",
        _ => "Coverage record",
    });
    let title = format!("{} Equihash miners and listings", v.name);
    let desc = format!(
        "Equihash miner models, seller-listed prices and source dates for {}.",
        v.name
    );
    let path = format!("/vendors/{}", v.slug);
    let mut profile_sources: Vec<(String, String)> = Vec::new();
    if let Some(url) = &v.registry_url {
        profile_sources.push(("Company record".into(), url.clone()));
    }
    if let Some(url) = &v.verification_url {
        if !profile_sources.iter().any(|(_, seen)| seen == url) {
            profile_sources.push(("Operator source".into(), url.clone()));
        }
    }
    if !warning {
        if let Some(url) = &v.url {
            if !profile_sources.iter().any(|(_, seen)| seen == url) {
                profile_sources.push(("Vendor website".into(), url.clone()));
            }
        }
    }
    if let Some(record) = research {
        for url in &record.source_urls {
            if !profile_sources.iter().any(|(_, seen)| seen == url) {
                let label = fmt::host(Some(url))
                    .split('/')
                    .next()
                    .unwrap_or("Evidence")
                    .to_string();
                profile_sources.push((label, url.clone()));
            }
        }
    }
    layout(
        d,
        Page {
            title: &title,
            description: &desc,
            path: &path,
            nav: "vendors",
        },
        html! {
            div class="wrap page buy-page buy-vendor-page" {
                p class="crumb" { a href="/vendors" { "Vendor directory" } }
                header class="page-head vendor-profile-head" {
                    (vendor_mark(v, true))
                    div {
                        p class="vendor-profile-kicker" { (channel_label(v)) @if let Some(label) = evidence_label { " · " (label) } }
                        h1 { (v.name) }
                        div class="vendor-profile-actions" {
                            @if !warning { @if let Some(url) = &v.url { (ext(url, "Visit vendor ↗")) } }
                            (crate::views::pages::copy_link(&path, "Copy link to this vendor"))
                            a href="/contribute" { "Report a correction" }
                        }
                    }
                }
                dl class="vendor-profile-metrics" {
                    div { dt { "Models" } dd { (model_count) } }
                    div { dt { "Listings" } dd { (rows.len()) } }
                    div { dt { "Seller base" } dd { (v.base_region.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Checked" } dd { (fmt::utc(latest_check)) } }
                }
                @if warning {
                    aside class="vendor-profile-warning" { strong { "Warning record" } p { "Product links are withheld. This entry is kept so readers can identify the seller and inspect the evidence." } }
                }
                @if !warning && !rows.is_empty() { (vendor_product_market(d, v, &rows)) }
                @if !warning && rows.is_empty() {
                    div class="empty-state vendor-profile-empty" { h2 { "No Equihash listing recorded" } p { "The seller record remains available below." } }
                }
                details class={"vendor-profile-evidence" @if warning { " is-warning" }} id="vendor-source-checks" {
                    summary { "Vendor details and sources" }
                    div class="vendor-profile-evidence-body" {
                        dl class="vendor-dossier" {
                            div { dt { "Relationship" } dd { (channel_label(v)) } }
                            div { dt { "Seller base" } dd { (v.base_region.as_deref().unwrap_or("Not established")) } }
                            div { dt { "Legal entity" } dd { (v.legal_name.as_deref().unwrap_or("Not established")) } }
                            div { dt { "Registration" } dd { (v.registration.as_deref().unwrap_or("Not established")) } }
                            div { dt { "Ships to" } dd { @if v.regions.is_empty() { "Not stated" } @else { (v.regions.join(", ")) } } }
                            div { dt { "Record checked" } dd { (fmt::utc(v.observed_at.as_deref())) } }
                        }
                        @if let Some(label) = &v.verification_label { p class="vendor-evidence-line" { strong { "Identity check: " } (label) } }
                        @if let Some(record) = research {
                            p class="vendor-evidence-line" { (&record.notes) }
                            dl class="vendor-research-facts" {
                                div { dt { "Equihash" } dd { (&record.equihash_z15) } }
                                div { dt { "History" } dd { (&record.history_and_reputation) } }
                                div { dt { "Payments" } dd { (&record.payment_protection) } }
                                div { dt { "Shipping" } dd { (&record.shipping_customs) } }
                                div { dt { "Warranty / RMA" } dd { (&record.warranty_rma) } }
                                div { dt { "Research checked" } dd { (record.last_verified.as_deref().unwrap_or("n/a")) } }
                            }
                        }
                        nav class="vendor-profile-source-links" aria-label="Vendor evidence links" {
                            @for (label, url) in &profile_sources {
                                (ext(url, label))
                            }
                        }
                    }
                }
            }
        },
    )
}

pub fn add_vendor(d: &Data) -> Markup {
    let template = "Vendor / shop name:\nWebsite:\nLegal entity and public company record:\nSeller base:\nRegions you ship to:\nEquihash ASIC source listing URL(s):\nMachine model(s):\nPrice, currency and tax wording shown:\nAvailability wording shown:\nContact for verification:\n";
    layout(d, Page {
        title: "Add a vendor",
        description: "How Equihash ASIC businesses can submit a source-linked record to the equihash.com vendor directory.",
        path: "/add-vendor",
        nav: "vendors",
    }, html! {
        div class="wrap page narrow prose" {
            header class="page-head" { h1 { "Add a vendor" } p class="lede" { "Directory inclusion is free. Submit public company and listing pages that readers can inspect without an account." } }
            h2 { "What to send" }
            ol { li { strong { "Publish the facts. " } "Company identity, seller location, availability, price and delivery wording must be visible on public pages." } li { strong { "Send the sources. " } "Copy the template and send it to " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak on X" } "." } }
            div class="tpl" { div class="tpl-head" { span { "Vendor template" } button class="btn small" type="button" data-copy="#tpl-vendor" { "Copy" } } pre id="tpl-vendor" { (template) } }
            p class="small" { "Machine specifications stay in the " a href="/asics" { "ASIC index" } "; the vendor directory records seller-published listings and source checks." }
        }
    })
}
