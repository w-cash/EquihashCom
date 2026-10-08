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

fn listing_snapshot(d: &Data, l: &Listing) -> Markup {
    html! {
        article class="vendor-snapshot" data-machine=(l.miner_id) data-region=(l.shipping_regions.join(",")) data-listing-state=(l.availability.as_deref().unwrap_or("unknown")) {
            div class="vendor-snapshot-machine" {
                p class="seller-label" { "LISTING SNAPSHOT" }
                h4 { a href={"/hardware/" (l.miner_id)} { (machine_name(d, &l.miner_id)) } }
                span class="mono" { (fmt::opt_num(l.shop_hashrate_ksol)) " kSol/s" }
            }
            div {
                p class="seller-label" { "PRICE SHOWN" }
                strong class="seller-price mono" { (currency(l.price_amount, l.price_currency.as_deref())) }
                @if l.price_includes_vat == Some(false) { small { "ex VAT" } }
            }
            div {
                p class="seller-label" { "AVAILABILITY STATED" }
                strong class="vendor-state" { (l.availability_label.as_deref().unwrap_or("Not stated")) }
                small { "Observed " (fmt::utc(l.observed_at.as_deref())) }
            }
            div class="vendor-snapshot-links" {
                @if let Some(u) = &l.source_url { (ext(u, "View source listing ↗")) }
                a href={"/hardware/" (l.miner_id)} { "Specifications" }
            }
            details class="seller-notes" {
                summary { "Listing notes" }
                @if let Some(n) = &l.hashrate_note { p { (n) } }
                @if let Some(n) = &l.shipping_note { p { (n) } }
            }
        }
    }
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

fn vendor_offer_slide(d: &Data, v: &Vendor, l: &Listing, index: usize, count: usize) -> Markup {
    let (image, width, height) = listing_image(l);
    let machine = machine_name(d, &l.miner_id);
    let calculator = format!(
        "/calculator?hashrate={}",
        fmt::opt_num(l.shop_hashrate_ksol)
    );
    html! {
        article class={"ed-machine-slide vendor-offer-slide" @if index == 0 { " is-active" }} data-machine-slide aria-hidden=(if index == 0 { "false" } else { "true" }) role="group" aria-roledescription="slide" aria-label={(index + 1) " of " (count) ": " (&l.title)} {
            header class="ed-machine-card-top" {
                span { (&v.name) }
                span class="ed-machine-record" { (l.availability_label.as_deref().unwrap_or("Listing observed")) }
            }
            div class="ed-hero-machine-image vendor-offer-image" {
                img src=(image) alt={(machine) " shown for " (&v.name)} width=(width) height=(height) loading=[(index > 0).then_some("lazy")] decoding="async";
            }
            div class="ed-hero-machine-info vendor-offer-info" {
                p { "VENDOR LISTING · EQUIHASH 200,9" }
                h2 { (&l.title) }
                dl class="vendor-offer-facts" {
                    div { dt { "Price shown" } dd { (currency(l.price_amount, l.price_currency.as_deref())) @if l.price_includes_vat == Some(false) { small { " ex VAT" } } } }
                    div { dt { "Hashrate" } dd { (fmt::opt_num(l.shop_hashrate_ksol)) " kSol/s" } }
                    div { dt { "Batch / availability" } dd { (l.availability_label.as_deref().unwrap_or("Not stated")) } }
                    div { dt { "Delivery scope" } dd { @if l.shipping_regions.is_empty() { "Not stated" } @else { (l.shipping_regions.join(", ")) } } }
                }
                p class="vendor-offer-observed" { "Page checked " time class="ago" datetime=[l.observed_at.as_deref()] { (fmt::utc(l.observed_at.as_deref())) } }
                nav aria-label={(&l.title) " actions"} {
                    a href={"/hardware/" (&l.miner_id)} { "Machine specifications" }
                    a href=(calculator) { "Calculate power cost" }
                    a href="#vendor-source-checks" { "Vendor checks" }
                }
                details class="vendor-offer-source" {
                    summary { "Delivery notes and source" }
                    @if let Some(note) = &l.shipping_note { p { (note) } }
                    @if let Some(note) = &l.hashrate_note { p { (note) } }
                    @if let Some(url) = &l.source_url { p { (ext(url, "Open the original product page ↗")) } }
                }
            }
        }
    }
}

fn vendor_offer_carousel(d: &Data, v: &Vendor, rows: &[&Listing]) -> Markup {
    let count = rows.len();
    html! {
        section class="vendor-product-stage" aria-labelledby="vendor-products-title" {
            header class="vendor-product-intro" {
                p class="eyebrow" { "PRODUCTS OBSERVED ON THE VENDOR SITE" }
                h2 id="vendor-products-title" { "Compare without opening every shop page" }
                p { "Price, delivery and batch wording are copied from the public product page. They are refreshed daily when the page can be parsed safely." }
            }
            aside class="ed-hero-machine vendor-offer-carousel" data-machine-carousel data-carousel-noun="offer" data-carousel-first-delay="2200" data-carousel-delay="5800" role="region" aria-roledescription="carousel" aria-label={(&v.name) " observed products"} {
                @if count > 1 {
                    nav class="ed-machine-switcher" data-machine-controls aria-label="Product carousel controls" hidden {
                        button type="button" data-machine-prev aria-label="Previous offer" { "←" }
                        span class="ed-machine-position" aria-hidden="true" { strong data-machine-current { "1" } " / " (count) }
                        button class="ed-machine-toggle" type="button" data-machine-toggle aria-label="Pause carousel" aria-pressed="false" { "Ⅱ" }
                        button type="button" data-machine-next aria-label="Next offer" { "→" }
                    }
                    button class="ed-machine-peek ed-machine-peek-prev" type="button" data-machine-peek-prev aria-label="Show previous offer" hidden { span aria-hidden="true" { "‹" } }
                    button class="ed-machine-peek ed-machine-peek-next" type="button" data-machine-peek-next aria-label="Show next offer" hidden { span aria-hidden="true" { "›" } }
                }
                div class="ed-machine-slides" {
                    @for (index, listing) in rows.iter().enumerate() {
                        (vendor_offer_slide(d, v, listing, index, count))
                    }
                }
            }
            p class="vendor-product-method" { "A seller's stock label is a public claim, not proof of a reserved unit. Confirm serials, invoice beneficiary, dispatch deadline and non-dispatch remedy before payment." }
        }
    }
}

fn vendor_record(d: &Data, v: &Vendor, rows: &[&Listing]) -> Markup {
    let machines = rows
        .iter()
        .map(|l| l.miner_id.as_str())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    let destinations = rows
        .iter()
        .flat_map(|l| l.shipping_regions.iter().map(String::as_str))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    let states = rows
        .iter()
        .filter_map(|l| l.availability.as_deref())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    html! {
        article class="vendor-record buy-card" data-machine=(machines) data-region=(destinations) data-base=(seller_region(v)) data-state=(states) {
            header class="vendor-record-head" {
                (vendor_mark(v, false))
                div class="vendor-record-title" {
                    p class="vendor-badge" { (channel_label(v)) }
                    h3 { a href={"/vendors/" (v.slug)} { (v.name) } }
                    p { (v.legal_name.as_deref().unwrap_or("Legal entity not established")) }
                }
                dl class="vendor-record-meta" {
                    div { dt { "Seller base" } dd { (v.base_region.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Ships to" } dd { @if v.regions.is_empty() { "Not stated" } @else { (v.regions.join(", ")) } } }
                }
            }
            div class="vendor-checks" {
                div {
                    p class="seller-label" { "SOURCE CHECKS" }
                    strong { (v.verification_label.as_deref().unwrap_or("Public seller page checked")) }
                    p {
                        @if let Some(u) = &v.registry_url { (ext(u, "Company record")) }
                        @if v.registry_url.is_some() && v.verification_url.is_some() { " · " }
                        @if let Some(u) = &v.verification_url { (ext(u, "Operator source")) }
                    }
                }
                div class="vendor-record-actions" {
                    a href={"/vendors/" (v.slug)} { "Vendor profile" }
                    @if let Some(u) = &v.url { (ext(u, "Vendor website ↗")) }
                }
            }
            div class="vendor-snapshots" {
                @for l in rows.iter().take(2) { (listing_snapshot(d, l)) }
                @if rows.len() > 2 {
                    p class="vendor-more-offers" { a href={"/vendors/" (v.slug) "#vendor-products-title"} { "Compare all " (rows.len()) " observed options on this vendor profile →" } }
                }
            }
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
    let country_groups = d
        .vendor_research
        .iter()
        .map(|record| record.country.as_str())
        .collect::<BTreeSet<_>>()
        .len();
    let fast_z15 = d
        .vendor_research
        .iter()
        .find(|record| record.vendor == "Antminer Distribution Europe");
    let mineshop = d
        .vendor_research
        .iter()
        .find(|record| record.vendor == "Mineshop.eu");
    layout(d, Page {
        title: "Global ASIC vendor directory and Equihash listings",
        description: "An evidence-graded global ASIC vendor directory with country and region coverage, warning records, and dated Equihash Z15 listing snapshots.",
        path: "/vendors",
        nav: "vendors",
    }, html! {
        div class="wrap page buy-page vendor-directory" {
            header class="page-head buy-head" {
                p class="eyebrow" { "GLOBAL ASIC VENDOR RESEARCH" }
                h1 { "ASIC vendors, checked country by country" }
                p class="lede" { "A public-evidence register of ASIC manufacturers, retailers, marketplaces and warning records. Equihash relevance, vendor trust and a specific stock claim are assessed separately." }
            }
            dl class="vendor-directory-stats" {
                div { dt { "Research records" } dd { (published.len() + warnings.len()) } }
                div { dt { "Country / region groups" } dd { (country_groups) } }
                div { dt { "Tier A–C records" } dd { (published.len()) } }
                div { dt { "Warning records" } dd { (warnings.len()) } }
            }
            aside class="vendor-reading-note" {
                strong { "Research date" }
                p { (d.vendor_research_as_of.as_deref().unwrap_or("n/a")) ". An evidence tier orders the strength of public records; it is not a delivery guarantee." }
                a href="#method" { "How to read the tiers" }
            }
            section class="vendor-z15-brief" aria-labelledby="z15-research-title" {
                header {
                    p class="eyebrow" { "Z15 / CURRENT RESEARCH NOTES" }
                    h2 id="z15-research-title" { "Stock wording needs a second check" }
                    p { "A product page, schema.org stock flag and physical allocation are different things. These two records show why." }
                }
                div class="vendor-z15-brief-grid" {
                    @if let Some(record) = fast_z15 {
                        article {
                            span class="vendor-tier tier-a" { "TIER A · FASTEST EXTERNAL LEAD" }
                            h3 { a href={"#vendor-" (&record.id)} { (&record.vendor) } }
                            p { (&record.equihash_z15) }
                        }
                    }
                    @if let Some(record) = mineshop {
                        article {
                            span class="vendor-tier tier-a" { "TIER A · FUTURE BATCH" }
                            h3 { a href={"#vendor-" (&record.id)} { (&record.vendor) } }
                            p { (&record.equihash_z15) }
                        }
                    }
                }
            }
            section class="vendor-offers" aria-labelledby="vendor-offers-title" {
                header class="vendor-directory-section-head" {
                    p class="eyebrow" { "EQUIHASH OFFER SNAPSHOTS" }
                    h2 id="vendor-offers-title" { "Public Z15 listings" }
                    p { "These dated observations record what a seller page stated. Recheck the page, allocation, serials, invoice beneficiary and delivery terms before payment." }
                }
            }
            form class="filters buy-filters" id="buy-filters" action="/vendors" method="get" {
                div class="f" { label for="buy-machine" { "Machine" } select id="buy-machine" name="machine" data-buy-filter="machine" { option value="" selected[machine.is_empty()] { "All machines" } @for m in &machines { option value=(m.id) selected[m.id == machine] { (m.maker) " " (m.model) } } } }
                div class="f" { label for="buy-base" { "Seller base" } select id="buy-base" name="base" data-buy-filter="base" { option value="" selected[base.is_empty()] { "All seller regions" } @for r in SELLER_REGIONS { @if d.vendors.iter().any(|v| seller_region(v) == *r) { option value=(r) selected[*r == base] { (r) } } } } }
                div class="f" { label for="buy-region" { "Ships to" } select id="buy-region" name="region" data-buy-filter="region" { option value="" selected[region.is_empty()] { "All destinations" } @for r in &destinations { option value=(r) selected[r == region] { (r) } } } }
                div class="f" { label for="buy-state" { "Listing state" } select id="buy-state" name="state" data-buy-filter="state" { option value="" selected[state.is_empty()] { "All states" } @for s in &states { option value=(s) selected[s == state] { (s.replace('_', " ")) } } } }
                button class="buy-apply" type="submit" { "Apply" }
                p class="buy-count" id="buy-count" aria-live="polite" { (visible.len()) " vendors · " (offers) " listing snapshots" }
            }
            nav class="vendor-region-index" aria-label="Seller regions" {
                @for group in SELLER_REGIONS { @let count = visible.iter().filter(|(v, _)| seller_region(v) == *group).count(); @if count > 0 { a href={"#region-" (group.to_ascii_lowercase().replace([' ', '&'], "-"))} { (group) span { (count) } } } }
            }
            @if visible.is_empty() { div class="empty-state" { h2 { "No vendor matches those filters" } p { "Clear a filter or send a public product page for review." } a href="/add-vendor" { "Add a vendor" } } }
            @for group in SELLER_REGIONS {
                @let mut records: Vec<_> = visible.iter().filter(|(v, _)| seller_region(v) == *group).collect();
                @if !records.is_empty() {
                    @let section_id = format!("region-{}", group.to_ascii_lowercase().replace([' ', '&'], "-"));
                    section class="seller-region-group" id=(section_id) data-buy-region-group data-base=(group) {
                        header class="seller-region-head" { h2 { (group) } p { (records.len()) @if records.len() == 1 { " vendor" } @else { " vendors" } } }
                        @for (v, rows) in records.drain(..) { (vendor_record(d, v, rows)) }
                    }
                }
            }
            section class="vendor-global-directory" aria-labelledby="global-vendors-title" {
                header class="vendor-directory-section-head" {
                    p class="eyebrow" { "GLOBAL RESEARCH REGISTER" }
                    h2 id="global-vendors-title" { "Manufacturers and sellers by region" }
                    p { "Tier A–C records are ordered by the strength of public identity, operating-history, payment, inventory and service evidence. Many are general ASIC companies with no current Equihash miner; the Z15 field says so directly." }
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
            section class="vendor-warning-register" aria-labelledby="vendor-warnings-title" {
                header class="vendor-directory-section-head" {
                    p class="eyebrow" { "WARNING / EXCLUSION REGISTER" }
                    h2 id="vendor-warnings-title" { (warnings.len()) " records kept out of the trusted directory" }
                    p { "These names remain visible so miners can recognize identity gaps, complaint patterns, implausible prices and clone-domain risk. They do not receive purchase or vendor-site buttons." }
                }
                div class="vendor-warning-list" { @for record in &warnings { (warning_record(record)) } }
            }
            section class="buy-standard" id="method" aria-labelledby="buy-standard-title" {
                header { p class="eyebrow" { "DIRECTORY METHOD" } h2 id="buy-standard-title" { "Evidence tier, vendor record and stock claim are separate" } }
                div class="buy-standard-grid" {
                    article { span class="audit-number" { "01" } h3 { "Evidence tier" } p { "Tier A is the strongest public evidence found; Tier B has material caveats; Tier C calls for high diligence; Tier D is warning-only. A tier is not a delivery probability." } }
                    article { span class="audit-number" { "02" } h3 { "Listing snapshot" } p { "Price, availability wording, hashrate and delivery claims are copied from a public page with an observation time. Structured-data stock alone is never treated as physical inventory." } }
                    article { span class="audit-number" { "03" } h3 { "Before payment" } p { "For a high-value order, require dated serial evidence, a legal-entity invoice, exact Incoterm, insured freight, a dispatch deadline and a written non-dispatch remedy." } }
                }
                p class="buy-submit" { "Publish Equihash hardware? " a href="/add-vendor" { "Submit the company record and source listing" } "." }
            }
            footer class="buy-honesty" { p { "equihash.com does not sell hardware or receive referral fees. Listings document public claims and company records; they do not endorse a seller or guarantee fulfillment." } }
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
    let title = format!("{} — Equihash ASIC vendor record", v.name);
    let desc = format!(
        "Source checks and dated Equihash ASIC listing snapshots for {}.",
        v.name
    );
    let path = format!("/vendors/{}", v.slug);
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
                        @if let Some(record) = research { p class={"vendor-profile-tier tier-" (record.trust_tier.to_ascii_lowercase())} { "TIER " (&record.trust_tier) " · " (trust_label(&record.trust_tier)) } }
                        div class="title-row" { h1 { (v.name) } (crate::views::pages::copy_link(&path, "Copy link to this vendor")) }
                        p class="lede" { (v.region_note.as_deref().unwrap_or("Vendor record.")) }
                    }
                }
                @if !warning && !rows.is_empty() { (vendor_offer_carousel(d, v, &rows)) }
                dl class="vendor-dossier" {
                    div { dt { "Relationship" } dd { (channel_label(v)) } }
                    div { dt { "Seller base" } dd { (v.base_region.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Legal entity" } dd { (v.legal_name.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Registration" } dd { (v.registration.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Ships to" } dd { @if v.regions.is_empty() { "Not stated" } @else { (v.regions.join(", ")) } } }
                    div { dt { "Checked" } dd { (fmt::utc(v.observed_at.as_deref())) } }
                }
                @if let Some(record) = research {
                    section class={"vendor-profile-research" @if warning { " is-warning" }} {
                        p class="seller-label" { "GLOBAL VENDOR RESEARCH · VERIFIED " (record.last_verified.as_deref().unwrap_or("n/a")) }
                        h2 { @if warning { "Warning record" } @else { "Research dossier" } }
                        p class="vendor-profile-research-note" { (&record.notes) }
                        dl {
                            div { dt { "Equihash / Z15" } dd { (&record.equihash_z15) } }
                            div { dt { "History and reputation" } dd { (&record.history_and_reputation) } }
                            div { dt { "Payments" } dd { (&record.payment_protection) } }
                            div { dt { "Shipping and customs" } dd { (&record.shipping_customs) } }
                            div { dt { "Warranty / RMA" } dd { (&record.warranty_rma) } }
                        }
                        (source_list(record))
                    }
                }
                section class="vendor-profile-checks" id="vendor-source-checks" {
                    h2 { "Source checks" }
                    @if let Some(label) = &v.verification_label { p { strong { (label) } } }
                    p { @if let Some(u) = &v.registry_url { (ext(u, "Company record")) " · " } @if let Some(u) = &v.verification_url { (ext(u, "Operator source")) } @if !warning { @if let Some(u) = &v.url { " · " (ext(u, "Vendor website")) } } }
                    @if let Some(n) = &v.notes { p class="small" { (n) } }
                }
                @if warning {
                    aside class="vendor-reading-note vendor-warning-note" { strong { "Listing links withheld" } p { "This seller is retained as a warning record. Its product snapshot is not presented as a purchase lead." } }
                }
                aside class="vendor-reading-note" { strong { "Not checked by equihash.com" } p { "Inventory, fulfillment, warranty handling, refund performance and final delivered cost." } }
                p class="buy-honesty" { "This record documents public evidence. It is not an endorsement, and equihash.com does not sell hardware." }
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
            p class="small" { "Manufacturer specifications stay on " a href="/hardware" { "Hardware" } "; the vendor directory records seller-published listings and source checks." }
        }
    })
}
