//! Source-linked Equihash ASIC vendor directory. This is an editorial register, not a shop:
//! there is no checkout, paid placement, inferred stock or fulfillment endorsement.

use crate::data::{Data, Listing, Vendor};
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
            div class="vendor-snapshots" { @for l in rows { (listing_snapshot(d, l)) } }
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
    layout(d, Page {
        title: "Equihash ASIC vendor directory",
        description: "A source-linked directory of businesses publishing Equihash ASIC listings, grouped by seller base with dated price and availability snapshots.",
        path: "/vendors",
        nav: "vendors",
    }, html! {
        div class="wrap page buy-page vendor-directory" {
            header class="page-head buy-head" {
                p class="eyebrow" { "VENDOR DIRECTORY" }
                h1 { "Equihash ASIC vendors" }
                p class="lede" { "A source-linked directory of businesses publishing Equihash ASIC listings. Company identity, location, delivery claims, price and availability are recorded separately. Inclusion is not an endorsement." }
            }
            dl class="vendor-directory-stats" {
                div { dt { "Vendors" } dd { (d.vendors.len()) } }
                div { dt { "Seller regions" } dd { (SELLER_REGIONS.iter().filter(|r| d.vendors.iter().any(|v| seller_region(v) == **r)).count()) } }
                div { dt { "Listing snapshots" } dd { (d.listings.len()) } }
                div { dt { "Last checked" } dd { (fmt::utc(d.listings_verified_at.as_deref().or(d.vendors_verified_at.as_deref()))) } }
            }
            aside class="vendor-reading-note" {
                strong { "What a check means" }
                p { "Identity checks establish who operates a site. They do not confirm inventory, delivery, warranty handling or refund performance." }
                a href="#method" { "Read the method" }
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
            section class="buy-standard" id="method" aria-labelledby="buy-standard-title" {
                header { p class="eyebrow" { "DIRECTORY METHOD" } h2 id="buy-standard-title" { "What is checked — and what is not" } }
                div class="buy-standard-grid" {
                    article { span class="audit-number" { "01" } h3 { "Seller identity" } p { "A legal name and public company record are linked where they can be independently matched. Seller-published registration details stay labelled as seller-published." } }
                    article { span class="audit-number" { "02" } h3 { "Listing snapshot" } p { "Price, availability wording, hashrate and delivery claims are copied from the source page with an observation time." } }
                    article { span class="audit-number" { "03" } h3 { "Not verified" } p { "Inventory, serial numbers, fulfillment, warranty handling, refunds and final delivered cost are not independently checked." } }
                }
                p class="buy-submit" { "Publish Equihash hardware? " a href="/add-vendor" { "Submit the company record and source listing" } "." }
            }
            footer class="buy-honesty" { p { "equihash.com does not sell hardware or receive referral fees. Listings document public claims and company records; they do not endorse a seller or guarantee fulfillment." } }
        }
    })
}

pub fn vendor_page(d: &Data, v: &Vendor) -> Markup {
    let rows: Vec<_> = d.listings.iter().filter(|l| l.vendor_id == v.id).collect();
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
                    div { div class="title-row" { h1 { (v.name) } (crate::views::pages::copy_link(&path, "Copy link to this vendor")) } p class="lede" { (v.region_note.as_deref().unwrap_or("Vendor record.")) } }
                }
                dl class="vendor-dossier" {
                    div { dt { "Relationship" } dd { (channel_label(v)) } }
                    div { dt { "Seller base" } dd { (v.base_region.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Legal entity" } dd { (v.legal_name.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Registration" } dd { (v.registration.as_deref().unwrap_or("Not established")) } }
                    div { dt { "Ships to" } dd { @if v.regions.is_empty() { "Not stated" } @else { (v.regions.join(", ")) } } }
                    div { dt { "Checked" } dd { (fmt::utc(v.observed_at.as_deref())) } }
                }
                section class="vendor-profile-checks" {
                    h2 { "Source checks" }
                    @if let Some(label) = &v.verification_label { p { strong { (label) } } }
                    p { @if let Some(u) = &v.registry_url { (ext(u, "Company record")) " · " } @if let Some(u) = &v.verification_url { (ext(u, "Operator source")) } @if let Some(u) = &v.url { " · " (ext(u, "Vendor website")) } }
                    @if let Some(n) = &v.notes { p class="small" { (n) } }
                }
                section class="vendor-profile-listings" { h2 { "Observed listings" } div class="vendor-snapshots" { @for l in rows { (listing_snapshot(d, l)) } } }
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
            ol { li { strong { "Publish the facts. " } "Company identity, seller location, availability, price and delivery wording must be visible on public pages." } li { strong { "Send the sources. " } "Copy the template and send it to " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak on X" } " or " a href="https://t.me/EquihashCom" rel="noopener" { "EquihashCom on Telegram" } "." } }
            div class="tpl" { div class="tpl-head" { span { "Vendor template" } button class="btn small" type="button" data-copy="#tpl-vendor" { "Copy" } } pre id="tpl-vendor" { (template) } }
            p class="small" { "Manufacturer specifications stay on " a href="/hardware" { "Hardware" } "; the vendor directory records seller-published listings and source checks." }
        }
    })
}
