//! Source-linked Equihash ASIC vendor directory. This is an editorial register, not a shop:
//! there is no checkout, paid placement, inferred stock or fulfillment endorsement.

use crate::data::{Data, Listing, Vendor, VendorResearch};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};
use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct BuyQuery {
    pub machine: Option<String>,
    pub base: Option<String>,
    pub region: Option<String>,
    pub state: Option<String>,
    pub vendor_q: Option<String>,
    pub vendor_region: Option<String>,
    pub availability: Option<String>,
    pub sort: Option<String>,
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

fn record_type_label(kind: &str) -> &'static str {
    match kind {
        "manufacturer_direct" => "Manufacturer direct",
        "reseller_or_broker" => "Reseller / broker",
        "marketplace" => "Marketplace",
        "hardware_and_hosting" => "Hardware / hosting",
        "public_warning_record" => "Sourced public-warning record",
        _ => "Public record",
    }
}

fn availability_label(state: &str) -> &'static str {
    match state {
        "seller_declared_spot_or_near_term" => "Seller declares spot or near-term dispatch",
        "preorder_or_future_batch" => "Preorder or future batch",
        "historical_or_used" => "Historical or used listing",
        "unknown_or_quote_required" => "Unknown or quote required",
        "sold_out_or_no_current_listing" => "Sold out or no current Equihash listing",
        "not_applicable_warning_record" => "Availability not applicable",
        _ => "Not stated",
    }
}

fn availability_rank(state: &str) -> u8 {
    match state {
        "seller_declared_spot_or_near_term" => 0,
        "preorder_or_future_batch" => 1,
        "historical_or_used" => 2,
        "unknown_or_quote_required" => 3,
        "sold_out_or_no_current_listing" => 4,
        "not_applicable_warning_record" => 5,
        _ => 6,
    }
}

fn profile_for_research<'a>(d: &'a Data, record: &VendorResearch) -> Option<&'a Vendor> {
    let key = site_key(record.website.as_deref());
    (!key.is_empty() && key != "n/a").then_some(())?;
    d.vendors
        .iter()
        .find(|vendor| site_key(vendor.url.as_deref()) == key)
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

#[derive(Clone, Copy)]
struct VendorTheme {
    accent: &'static str,
    deep: &'static str,
    tint: &'static str,
    shape: &'static str,
}

/// A small, restrained identity system for vendor profiles. These colours are taken from the
/// vendors' public marks or sites; they identify the profile without turning the directory into a
/// copy of each shop. Every value is a compile-time CSS token, never vendor-supplied text.
fn vendor_theme(id: &str) -> VendorTheme {
    match id {
        "bitmain" => VendorTheme {
            accent: "#d95d26",
            deep: "#66260e",
            tint: "#f4e6dc",
            shape: "wedge",
        },
        "the-mining-shop-uk" => VendorTheme {
            accent: "#d77818",
            deep: "#713603",
            tint: "#f6eadb",
            shape: "arc",
        },
        "antminer-distribution-europe" => VendorTheme {
            accent: "#356ea5",
            deep: "#173b5d",
            tint: "#e7eff7",
            shape: "rail",
        },
        "apexto-mining" => VendorTheme {
            accent: "#4f873d",
            deep: "#254a1e",
            tint: "#e8f0e3",
            shape: "orbit",
        },
        "hashlabs" => VendorTheme {
            accent: "#7c4097",
            deep: "#3c1e4b",
            tint: "#eee5f2",
            shape: "orbit",
        },
        "bt-miners" => VendorTheme {
            accent: "#b65b49",
            deep: "#5c271d",
            tint: "#f2e4e0",
            shape: "wedge",
        },
        "oneminers" => VendorTheme {
            accent: "#d79a0d",
            deep: "#664600",
            tint: "#f6edd4",
            shape: "arc",
        },
        "the-bitcoin-miner-uk" => VendorTheme {
            accent: "#dd7b16",
            deep: "#693606",
            tint: "#f5e8d9",
            shape: "arc",
        },
        "805-mining" => VendorTheme {
            accent: "#811d23",
            deep: "#400c10",
            tint: "#f1e1e2",
            shape: "rail",
        },
        "crypto-miner-bros" => VendorTheme {
            accent: "#30343a",
            deep: "#15171a",
            tint: "#e6e8ea",
            shape: "wedge",
        },
        "mineshop-eu" => VendorTheme {
            accent: "#27765b",
            deep: "#123e2e",
            tint: "#e2eee9",
            shape: "rail",
        },
        _ => VendorTheme {
            accent: "#697068",
            deep: "#30352f",
            tint: "#e8ebe5",
            shape: "wedge",
        },
    }
}

fn vendor_brand_field(v: &Vendor) -> Markup {
    html! {
        div class="vendor-brand-field" aria-hidden="true" {
            @if let Some(src) = &v.logo.src {
                img src=(src) alt="" width="360" height="180" decoding="async";
            } @else {
                span { (v.logo.mono.as_str()) }
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
        _ => "hashrate —".into(),
    }
}

fn vendor_offer_table(d: &Data, rows: &[&Listing]) -> Markup {
    let groups = vendor_model_groups(rows);
    html! {
        p class="vendor-offer-swipe" { "Swipe to compare all columns →" }
        div class="vendor-offer-table-wrap" {
            table class="vendor-offer-table" id="vendor-offer-table" {
                thead { tr { th scope="col" { "Model / option" } th scope="col" { "Price" } th scope="col" { "Seller-declared status" } th scope="col" { "Ships to" } th scope="col" { "Checked" } th scope="col" { "Source" } } }
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
                                    strong { (row.availability_label.as_deref().unwrap_or("—")) }
                                    @if let Some(condition) = &row.condition { small { (condition.replace('_', " ")) } }
                                }
                                td data-label="Price" class="mono" { strong { (currency(row.price_amount, row.price_currency.as_deref())) } @if row.price_includes_vat == Some(false) { small { "ex VAT" } } }
                                td data-label="Seller-declared status" { span class={"vendor-market-state state-" (row.availability.as_deref().unwrap_or("unknown"))} { (row.availability.as_deref().unwrap_or("unknown").replace('_', " ")) } }
                                td data-label="Ships to" { @if row.shipping_regions.is_empty() { "—" } @else { (row.shipping_regions.join(", ")) } }
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
            p class="vendor-product-method" { "Prices, stock, batches and delivery wording are seller-declared page claims captured on the shown date. We do not verify inventory or fulfillment. Check the source before paying." }
        }
    }
}

fn public_vendor_row(d: &Data, record: &VendorResearch) -> Markup {
    let profile = profile_for_research(d, record);
    html! {
        tr id={"vendor-" (&record.id)} class="vendor-directory-public-row" data-vendor-record=(&record.id) {
            th scope="row" class="vendor-directory-name" {
                @if let Some(vendor) = profile {
                    a href={"/vendors/" (&vendor.slug)} { (&record.vendor) }
                } @else if let Some(url) = &record.website {
                    (ext(url, &record.vendor))
                } @else { (&record.vendor) }
                small { (record_type_label(&record.record_type)) }
            }
            td data-label="Location" { strong { (&record.country) } small { (&record.region) } }
            td data-label="Declared availability" class="vendor-directory-availability" { (availability_label(&record.declared_availability)) }
            td data-label="Third-party review profile" class="vendor-directory-review" {
                @if let Some(url) = &record.review_source_url {
                    (ext(url, "Open profile ↗"))
                    @if let Some(date) = &record.review_snapshot_date { small { (record.review_platform.as_deref().unwrap_or("Third-party review")) " · link checked " (date) } }
                } @else { "—" }
            }
            td data-label="Verified company year" class="mono" {
                @if let Some(year) = record.company_incorporated_year_verified { (year) }
                @else { "—" }
            }
            td data-label="Checked" class="mono" { (record.last_verified.as_deref().unwrap_or("—")) }
            td data-label="Record" class="vendor-directory-links" {
                @if let Some(vendor) = profile { a href={"/vendors/" (&vendor.slug)} { "Profile" } }
                @if let Some(url) = &record.website { (ext(url, "Website ↗")) }
                details class="vendor-directory-sources" {
                    summary { "Sources " (record.source_urls.len()) }
                    (source_list(record))
                }
                a href={"/add-vendor?record=" (&record.id)} { "Correction" }
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
    let needle = q.vendor_q.as_deref().unwrap_or("").trim().to_lowercase();
    let vendor_region = q.vendor_region.as_deref().unwrap_or("");
    let availability = q.availability.as_deref().unwrap_or("");
    let sort = match q.sort.as_deref().unwrap_or("alphabetical") {
        "declared_availability" | "company_age" | "manufacturer_direct" | "last_verified" => {
            q.sort.as_deref().unwrap()
        }
        _ => "alphabetical",
    };
    let legacy_filter =
        !machine.is_empty() || !base.is_empty() || !region.is_empty() || !state.is_empty();
    let mut visible: Vec<&VendorResearch> = d
        .vendor_research
        .iter()
        .filter(|record| record.record_type != "coverage_gap")
        .filter(|record| {
            needle.is_empty()
                || [
                    record.vendor.as_str(),
                    record.country.as_str(),
                    record.region.as_str(),
                    record.equihash_z15_claim.as_str(),
                ]
                .iter()
                .any(|value| value.to_lowercase().contains(&needle))
        })
        .filter(|record| vendor_region.is_empty() || record.region == vendor_region)
        .filter(|record| availability.is_empty() || record.declared_availability == availability)
        .filter(|record| {
            if !legacy_filter {
                return true;
            }
            let Some(vendor) = profile_for_research(d, record) else {
                return false;
            };
            (base.is_empty() || seller_region(vendor) == base)
                && d.listings.iter().any(|listing| {
                    listing.vendor_id == vendor.id && matches(listing, machine, region, state)
                })
        })
        .collect();
    let alpha = |a: &&VendorResearch, b: &&VendorResearch| {
        a.vendor
            .to_lowercase()
            .cmp(&b.vendor.to_lowercase())
            .then_with(|| a.id.cmp(&b.id))
    };
    visible.sort_by(|a, b| {
        let primary = match sort {
            "declared_availability" => availability_rank(&a.declared_availability)
                .cmp(&availability_rank(&b.declared_availability)),
            "company_age" => match (
                a.company_incorporated_year_verified,
                b.company_incorporated_year_verified,
            ) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                _ => Ordering::Equal,
            },
            "manufacturer_direct" => b.manufacturer_direct.cmp(&a.manufacturer_direct),
            "last_verified" => match (&a.last_verified, &b.last_verified) {
                (Some(a), Some(b)) => b.cmp(a),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                _ => Ordering::Equal,
            },
            _ => Ordering::Equal,
        };
        primary.then_with(|| alpha(a, b))
    });
    let regions = d
        .vendor_research
        .iter()
        .filter(|record| record.record_type != "coverage_gap")
        .map(|record| record.region.as_str())
        .collect::<BTreeSet<_>>();
    let public_records = d
        .vendor_research
        .iter()
        .filter(|record| record.record_type != "coverage_gap")
        .count();
    let review_profiles = d
        .vendor_research
        .iter()
        .filter(|record| record.record_type != "coverage_gap" && record.review_source_url.is_some())
        .count();
    let (sort_label, sort_rule) = match sort {
        "declared_availability" => ("Seller-declared availability", "Documented state order; seller/public claims first, unknown and non-current records later. No stock is physically audited unless expressly stated."),
        "company_age" => ("Verified company incorporation year", "Oldest verified incorporation year first; missing years last. Company age is not a quality guarantee."),
        "manufacturer_direct" => ("Manufacturer direct", "Manufacturer-operated shops first, then alphabetical. This does not imply current stock or destination support."),
        "last_verified" => ("Most recently verified", "Newest evidence date first; missing dates last."),
        _ => ("Alphabetical", "Vendor name A–Z. No Equihash.com recommendation or quality judgment is implied."),
    };
    layout(d, Page {
        title: "Equihash ASIC vendor directory",
        description: "Alphabetical ASIC vendor directory with seller-attributed availability, company records, source links and transparent sorting rules.",
        path: "/vendors",
        nav: "vendors",
    }, html! {
        div class="wrap page buy-page vendor-directory" {
            header class="page-head buy-head" {
                p class="eyebrow" { "EQUIHASH VENDOR DIRECTORY" }
                h1 { "ASIC vendors" }
                p class="lede" { "One record per vendor. Alphabetical by default, with optional sorting by a single disclosed factor." }
            }
            dl class="vendor-directory-stats" {
                div { dt { "Public records" } dd { (public_records) } }
                div { dt { "Review profiles" } dd { (review_profiles) } }
                div { dt { "Copied ratings" } dd { "Not published" } }
                div { dt { "Evidence snapshot" } dd { (d.vendor_research_as_of.as_deref().unwrap_or("—")) } }
            }
            aside class="vendor-directory-notice" aria-labelledby="vendor-directory-notice-title" {
                strong id="vendor-directory-notice-title" { "How this directory works" }
                p { (d.vendor_research_disclaimer.as_deref().unwrap_or("Informational directory only. Inclusion, position and seller claims are not endorsements or guarantees.")) }
                p { "Review profile links are provided as external references. Scores, review counts and review-based sorting are not republished." }
                p { "Availability is seller-declared unless expressly stated otherwise. Paid listings and affiliate relationships cannot influence the default order. " a href="/add-vendor#correction" { "Request a correction" } "." }
            }
            form class="filters vendor-directory-filters" action="/vendors" method="get" {
                div class="f" { label for="vendor-q" { "Search" } input id="vendor-q" type="search" name="vendor_q" value=(q.vendor_q.as_deref().unwrap_or("")) placeholder="Vendor or country"; }
                div class="f" { label for="vendor-region" { "Region" } select id="vendor-region" name="vendor_region" { option value="" selected[vendor_region.is_empty()] { "All regions" } @for item in &regions { option value=(item) selected[*item == vendor_region] { (item) } } } }
                div class="f" { label for="vendor-availability" { "Declared availability" } select id="vendor-availability" name="availability" { option value="" selected[availability.is_empty()] { "All states" } @for item in ["seller_declared_spot_or_near_term", "preorder_or_future_batch", "historical_or_used", "unknown_or_quote_required", "sold_out_or_no_current_listing", "not_applicable_warning_record"] { option value=(item) selected[item == availability] { (availability_label(item)) } } } }
                div class="f" { label for="vendor-sort" { "Sort by" } select id="vendor-sort" name="sort" { option value="alphabetical" selected[sort == "alphabetical"] { "Alphabetical" } option value="declared_availability" selected[sort == "declared_availability"] { "Seller-declared availability" } option value="company_age" selected[sort == "company_age"] { "Verified company age" } option value="domain_age" disabled { "Verified domain age — unavailable" } option value="manufacturer_direct" selected[sort == "manufacturer_direct"] { "Manufacturer direct" } option value="last_verified" selected[sort == "last_verified"] { "Most recently verified" } } }
                button class="buy-apply" type="submit" { "Apply" }
                p class="buy-count" aria-live="polite" { (visible.len()) @if visible.len() == 1 { " record" } @else { " records" } }
            }
            div class="vendor-sort-status" { strong { "Sorted by: " (sort_label) } span { (sort_rule) } }
            p class="vendor-domain-note" { "Domain-age sorting is disabled: none of the 87 source records has a verified domain-registration year." }
            @if visible.is_empty() { div class="empty-state" { h2 { "No vendor matches those filters" } p { "Clear a filter or submit a sourced correction." } a href="/add-vendor" { "Add or correct a vendor" } } }
            @if !visible.is_empty() {
                div class="vendor-market-table-wrap" {
                    table class="vendor-market-table" {
                        thead { tr { th scope="col" { "Vendor" } th scope="col" { "Location" } th scope="col" { "Declared Equihash availability" } th scope="col" { "Third-party review profile" } th scope="col" { "Verified company year" } th scope="col" { "Checked" } th scope="col" { "Record" } } }
                        tbody {
                            @for record in &visible {
                                (public_vendor_row(d, record))
                            }
                        }
                    }
                }
                p class="vendor-market-disclosure" { "One row per public record. Blank values were not established in the cited evidence and are not inferred." }
            }
            footer class="vendor-directory-footer" { a href="/add-vendor#correction" { "Add a vendor or request a correction" } span { "No paid position, affiliate relationship or house score changes this order." } }
        }
    })
}

pub fn vendor_page(d: &Data, v: &Vendor) -> Markup {
    let research = research_for(d, v);
    let warning = research
        .map(|record| record.record_type == "public_warning_record")
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
    let title = format!("{} Equihash miners and listings", v.name);
    let desc = format!(
        "Equihash miner models, seller-listed prices and source dates for {}.",
        v.name
    );
    let path = format!("/vendors/{}", v.slug);
    let theme = vendor_theme(&v.id);
    let theme_style = format!(
        "--vendor-accent:{};--vendor-deep:{};--vendor-tint:{}",
        theme.accent, theme.deep, theme.tint
    );
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
            div class="wrap page buy-page buy-vendor-page" data-brand-shape=(theme.shape) style=(theme_style) {
                p class="crumb" { a href="/vendors" { "Vendor directory" } }
                section class="vendor-identity" aria-labelledby="vendor-name" {
                    (vendor_brand_field(v))
                    header class="page-head vendor-profile-head" {
                        (vendor_mark(v, true))
                        div {
                            p class="vendor-profile-kicker" { (channel_label(v)) }
                            h1 id="vendor-name" { (v.name) }
                            div class="vendor-profile-actions" {
                                @if !warning { @if let Some(url) = &v.url { (ext(url, "Visit vendor ↗")) } }
                                (crate::views::pages::copy_link(&path, "Copy link to this vendor"))
                                a href={"/add-vendor?record=" (&v.id) "#correction"} { "Request a correction" }
                            }
                        }
                    }
                    dl class="vendor-profile-metrics" {
                        div { dt { "Models" } dd { (model_count) } }
                        div { dt { "Listings" } dd { (rows.len()) } }
                        div { dt { "Vendor base" } dd { (v.base_region.as_deref().unwrap_or("—")) } }
                        div { dt { "Checked" } dd { (fmt::utc(latest_check)) } }
                    }
                }
                aside class="vendor-profile-directory-note" {
                    strong { "Seller record" }
                    p { "Prices, stock, batches and delivery wording below are seller-declared page claims captured on the shown date. Inclusion is not an endorsement or a fulfillment guarantee." }
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
                            div { dt { "Vendor base" } dd { (v.base_region.as_deref().unwrap_or("—")) } }
                            div { dt { "Legal entity" } dd { (v.legal_name.as_deref().unwrap_or("—")) } }
                            div { dt { "Registration" } dd { (v.registration.as_deref().unwrap_or("—")) } }
                            div { dt { "Ships to" } dd { @if v.regions.is_empty() { "—" } @else { (v.regions.join(", ")) } } }
                            div { dt { "Record checked" } dd { (fmt::utc(v.observed_at.as_deref())) } }
                        }
                        @if let Some(label) = &v.verification_label { p class="vendor-evidence-line" { strong { "Identity source: " } (label) } }
                        @if let Some(record) = research {
                            p class="vendor-evidence-line" { (&record.legal_identity_and_location_summary) }
                            dl class="vendor-research-facts" {
                                div { dt { "Equihash" } dd { (&record.equihash_z15_claim) } }
                                div { dt { "Declared availability" } dd { (availability_label(&record.declared_availability)) } }
                                div { dt { "Payments" } dd { (&record.payment_methods_and_protection) } }
                                div { dt { "Shipping" } dd { (&record.shipping_customs) } }
                                div { dt { "Warranty / RMA" } dd { (&record.warranty_rma) } }
                                div { dt { "Review profile" } dd { @if let Some(url) = &record.review_source_url { (ext(url, "Open third-party profile ↗")) @if let Some(date) = &record.review_snapshot_date { " · link checked " (date) } } @else { "—" } } }
                                div { dt { "Record checked" } dd { (record.last_verified.as_deref().unwrap_or("—")) } }
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
        div class="wrap page narrow prose" id="correction" {
            header class="page-head" { h1 { "Add or correct a vendor" } p class="lede" { "Directory inclusion and factual corrections are free. Submit public company and listing pages that readers can inspect without an account." } }
            p { "For a correction, include the record ID, the field that is wrong, the proposed wording and a primary source. Clear factual errors are corrected without changing a vendor's position for commercial reasons." }
            h2 { "What to send" }
            ol { li { strong { "Publish the facts. " } "Company identity, seller location, availability, price and delivery wording must be visible on public pages." } li { strong { "Send the sources. " } "Copy the template and send it to " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak on X" } "." } }
            div class="tpl" { div class="tpl-head" { span { "Vendor template" } button class="btn small" type="button" data-copy="#tpl-vendor" { "Copy" } } pre id="tpl-vendor" { (template) } }
            p class="small" { "Machine specifications stay in the " a href="/asics" { "ASIC index" } "; the vendor directory records seller-published listings and source checks." }
        }
    })
}
