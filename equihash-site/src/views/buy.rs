//! Equihash ASIC shop directory. This is an outbound directory only: no checkout,
//! paid placement or inferred stock. Every commercial claim keeps its source and observation time.

use crate::data::{Data, Listing, Vendor};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use crate::views::logo;
use maud::{html, Markup};
use serde::Deserialize;

const SELLER_REGIONS: &[&str] = &[
    "Manufacturer direct",
    "United Kingdom",
    "United States",
    "Europe",
    "China & Hong Kong",
    "Asia-Pacific",
    "Other",
];

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct BuyQuery {
    pub machine: Option<String>,
    pub region: Option<String>,
    pub sort: Option<String>,
}

fn vendor<'a>(d: &'a Data, id: &str) -> Option<&'a Vendor> {
    d.vendors.iter().find(|v| v.id == id)
}

fn seller_region(v: Option<&Vendor>) -> &str {
    let Some(v) = v else { return "Other" };
    if v.channel.as_deref() == Some("manufacturer") {
        "Manufacturer direct"
    } else {
        v.base_region.as_deref().unwrap_or("Other")
    }
}

fn seller_region_name_rank(region: &str) -> usize {
    match region {
        "Manufacturer direct" => 0,
        "United Kingdom" => 1,
        "United States" => 2,
        "Europe" => 3,
        "China & Hong Kong" => 4,
        "Asia-Pacific" => 5,
        _ => 6,
    }
}

fn seller_region_rank(v: Option<&Vendor>) -> usize {
    seller_region_name_rank(seller_region(v))
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
    html! {
        span class={"logo mono" @if head { " lg-head" } @else { " lg-list" }} aria-hidden="true" {
            @if v.slug == "the-mining-shop-uk" {
                img src="/static/logos/vendors/the-mining-shop-uk.239a526176.png" alt="" width="512" height="512";
            } @else {
                (logo::monogram(&v.name))
            }
        }
    }
}

fn channel_label(v: &Vendor) -> &str {
    match v.channel.as_deref() {
        Some("manufacturer") => "Official manufacturer",
        Some("broker_hosting") => "Broker & hosting",
        _ => "Independent retailer",
    }
}

fn channel_class(v: &Vendor) -> &str {
    if v.channel.as_deref() == Some("manufacturer") {
        " official"
    } else {
        ""
    }
}

fn listing_card(d: &Data, l: &Listing) -> Markup {
    let v = vendor(d, &l.vendor_id);
    let vendor_name = v.map(|x| x.name.as_str()).unwrap_or("Vendor");
    let slug = v.map(|x| x.slug.as_str()).unwrap_or("");
    let stock_rank = if l.availability.as_deref() == Some("in_stock") {
        0
    } else {
        1
    };
    let region_rank = seller_region_rank(v);
    html! {
        article class="seller-row buy-card" data-machine=(l.miner_id) data-vendor=(l.vendor_id) data-region=(l.shipping_regions.join(",")) data-seller-region=(seller_region(v)) data-price=[l.price_amount.map(|x| x.to_string())] data-stock=(l.availability.as_deref().unwrap_or("unknown")) data-stock-rank=(stock_rank) data-region-rank=(region_rank) {
            header class="seller-org" {
                @if let Some(v) = v { span class={"vendor-badge" (channel_class(v))} { (channel_label(v)) } }
                h3 { @if !slug.is_empty() { a href={"/buy/vendor/" (slug)} { (vendor_name) } } @else { (vendor_name) } }
                @if let Some(v) = v {
                    @if let Some(n) = &v.legal_name { small { (n) } }
                    @if let Some(r) = &v.registration { span class="mono seller-registration" { (r) } }
                }
            }
            div class="seller-offer" {
                p class="seller-label" { "PUBLIC OFFER" }
                strong class="seller-price mono" { (currency(l.price_amount, l.price_currency.as_deref())) }
                @if l.price_includes_vat == Some(false) { span { "ex VAT" } }
                span class="seller-status" { (l.availability_label.as_deref().unwrap_or("n/a")) }
            }
            dl class="seller-facts" {
                div { dt { "Hashrate" } dd class="mono" { (fmt::opt_num(l.shop_hashrate_ksol)) " kSol/s" } }
                div { dt { "Ships to" } dd { @if l.shipping_regions.is_empty() { "n/a" } @else { (l.shipping_regions.join(", ")) } } }
                div { dt { "Condition" } dd { (l.condition.as_deref().unwrap_or("n/a")) } }
            }
            div class="seller-evidence" {
                p class="seller-label" { "EVIDENCE" }
                @if let Some(v) = v {
                    strong { (v.verification_label.as_deref().unwrap_or("Public seller page checked")) }
                    p { @if let Some(u) = &v.registry_url { (ext(u, "Registry record")) " · " } @if let Some(u) = &l.source_url { (ext(u, "Offer page")) } }
                }
                small { "Checked " (fmt::utc(l.observed_at.as_deref())) }
            }
            div class="seller-action" {
                @if let Some(u) = &l.product_url { a class="buy-cta" href=(u) rel="noopener nofollow" target="_blank" { "Open offer" } }
                a class="buy-quiet" href={"/hardware/" (l.miner_id)} { "Specifications" }
            }
            details class="seller-notes" { summary { "Offer notes" } @if let Some(n) = &l.hashrate_note { p { (n) } } @if let Some(n) = &l.shipping_note { p { (n) } } }
        }
    }
}

pub fn index(d: &Data, q: &BuyQuery) -> Markup {
    let machine = q.machine.as_deref().unwrap_or("");
    let region = q.region.as_deref().unwrap_or("");
    let sort = q.sort.as_deref().unwrap_or("region");
    let mut listings: Vec<_> = d
        .listings
        .iter()
        .filter(|l| {
            (machine.is_empty() || l.miner_id == machine)
                && (region.is_empty()
                    || l.shipping_regions
                        .iter()
                        .any(|r| r.eq_ignore_ascii_case(region) || r == "Global"))
        })
        .collect();
    listings.sort_by(|a, b| match sort {
        "stock" => (a.availability.as_deref() != Some("in_stock"))
            .cmp(&(b.availability.as_deref() != Some("in_stock"))),
        _ => seller_region_rank(vendor(d, &a.vendor_id))
            .cmp(&seller_region_rank(vendor(d, &b.vendor_id)))
            .then_with(|| {
                a.price_amount
                    .partial_cmp(&b.price_amount)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
    });
    let mut machines: Vec<_> = d
        .miners
        .iter()
        .filter(|m| d.listings.iter().any(|l| l.miner_id == m.id))
        .collect();
    machines.sort_by(|a, b| a.model.cmp(&b.model));
    let mut regions: Vec<String> = d
        .listings
        .iter()
        .flat_map(|l| l.shipping_regions.iter().cloned())
        .collect();
    regions.sort();
    regions.dedup();
    layout(d, Page { title: "Buy Equihash ASICs: Z15 Pro shop directory", description: "Where to buy Equihash ASICs including the Antminer Z15 Pro. Prices, stock and shipping are copied from each vendor’s public product page.", path: "/buy", nav: "buy" }, html! {
        div class="wrap page buy-page" {
            header class="page-head buy-head" {
                p class="eyebrow" { "WHERE TO BUY" }
                h1 { "Antminer Z15 Pro sellers" }
                p class="lede" { "Current public offers from BITMAIN and independent sellers, grouped by the seller’s disclosed operating region. Each row links the company record, product page and the details observed there." }
            }
            aside class="official-channel" aria-labelledby="official-channel-title" {
                div {
                    p class="eyebrow" { "START HERE" }
                    h2 id="official-channel-title" { "BITMAIN sells direct. Its current Z15 Pro batch is sold out." }
                    p { "BITMAIN says shop.bitmain.com is its only sales channel and that it has no official distributors or resellers. Every other name below is an independent seller." }
                }
                div class="official-channel-actions" {
                    a class="buy-cta" href="https://shop.bitmain.com/" rel="noopener nofollow" target="_blank" { "Open BITMAIN Shop" }
                    a href="https://support.bitmain.com/hc/en-us/articles/4563169497497-Is-there-any-recommended-dealers-Are-there-any-distributors-overseas" rel="noopener" target="_blank" { "BITMAIN’s own statement" }
                }
            }
            form class="filters buy-filters" id="buy-filters" action="/buy" method="get" {
                div class="f" { label for="buy-machine" { "Machine" } select id="buy-machine" name="machine" data-buy-filter="machine" { option value="" selected[machine.is_empty()] { "All machines" } @for m in &machines { option value=(m.id) selected[m.id == machine] { (m.maker) " " (m.model) } } } }
                div class="f" { label for="buy-region" { "Ships to" } select id="buy-region" name="region" data-buy-filter="region" { option value="" selected[region.is_empty()] { "All destinations" } @for r in &regions { option value=(r) selected[r == region] { (r) } } } }
                div class="f f-sort" { label for="buy-sort" { "Sort" } select id="buy-sort" name="sort" data-buy-sort { option value="region" selected[sort == "region"] { "Channel and region" } option value="stock" selected[sort == "stock"] { "Availability" } } }
                button class="buy-apply" type="submit" { "Apply" }
                p class="buy-count" id="buy-count" aria-live="polite" { (listings.len()) @if listings.len() == 1 { " listing" } @else { " listings" } @if let Some(t) = d.listings_verified_at.as_deref().or(d.vendors_verified_at.as_deref()) { " · checked " (fmt::utc(Some(t))) } }
            }
            @if listings.is_empty() { div class="empty-state" { h2 { "No listing matches those filters" } p { "Clear a filter or send a public product page for review." } a href="/add-vendor" { "Add a vendor" } } }
            @for m in &machines {
                @let rows: Vec<_> = listings.iter().filter(|l| l.miner_id == m.id).cloned().collect();
                @if !rows.is_empty() { section class="buy-machine" id={"machine-" (m.id)} data-machine=(m.id) {
                    header class="buy-machine-head seller-sheet-head" { h2 { (m.maker) " " (m.model) " offers" } p class="section-sub" { "Seller base and shipping destinations are separate. Public prices are snapshots; availability is the seller’s claim. " a href={"/hardware/" (m.id)} { "Check BITMAIN specifications" } "." } }
                    @for group in SELLER_REGIONS {
                        @let group_rows: Vec<_> = rows.iter().filter(|l| seller_region(vendor(d, &l.vendor_id)) == *group).cloned().collect();
                        @if !group_rows.is_empty() {
                            section class="seller-region-group" data-buy-region-group {
                                header class="seller-region-head" { h3 { (*group) } p { (group_rows.len()) @if group_rows.len() == 1 { " seller" } @else { " sellers" } } }
                                div class="seller-sheet-labels" aria-hidden="true" { span { "Seller" } span { "Offer" } span { "Machine" } span { "Evidence" } span { "Link" } }
                                div class="buy-gallery seller-sheet" data-buy-gallery { @for l in group_rows { (listing_card(d, l)) } }
                            }
                        }
                    }
                } }
            }
            section class="buy-standard" aria-labelledby="buy-standard-title" {
                header { p class="eyebrow" { "LISTING METHOD" } h2 id="buy-standard-title" { "How listings are checked" } }
                div class="buy-standard-grid" {
                    article { span class="audit-number" { "01" } h3 { "Seller identity" } p { "The seller’s legal name and registration number were matched to a public company record where one was available." } }
                    article { span class="audit-number" { "02" } h3 { "Published offer" } p { "Price, stated availability, hashrate and shipping claims were copied from the linked offer page at the checked time." } }
                    article { span class="audit-number" { "03" } h3 { "Before payment" } p { "Stock, serial numbers, warranty coverage and final delivered cost are not independently confirmed. Get them in writing before payment." } }
                }
                p class="buy-submit" { "Sell Equihash hardware? " a href="/add-vendor" { "Submit the company record and public offer page" } "." }
            }
            footer class="buy-honesty" { p { "equihash.com does not sell hardware, collect payment, receive referral fees or guarantee a seller. A listing records public evidence; it is not an endorsement." } }
        }
    })
}

pub fn vendor_page(d: &Data, v: &Vendor) -> Markup {
    let rows: Vec<_> = d.listings.iter().filter(|l| l.vendor_id == v.id).collect();
    let title = format!("{} — Equihash ASIC vendor", v.name);
    let desc = format!(
        "Equihash ASIC listings from {}. Open each product on the vendor site to buy.",
        v.name
    );
    let path = format!("/buy/vendor/{}", v.slug);
    layout(
        d,
        Page {
            title: &title,
            description: &desc,
            path: &path,
            nav: "buy",
        },
        html! {
            div class="wrap page buy-page buy-vendor-page" {
                p class="crumb" { a href="/buy" { "All Buy listings" } }
            header class="page-head" { div class="title-row" { h1 { (vendor_mark(v, true)) (v.name) } (crate::views::pages::copy_link(&path, "Copy link to this vendor")) } p class="lede" { (v.region_note.as_deref().unwrap_or("Vendor listing.")) @if let Some(u) = &v.url { " Site: " (ext(u, &fmt::host(Some(u)))) "." } } }
                dl class="vendor-dossier" { div { dt { "Relationship" } dd { (channel_label(v)) } } div { dt { "Seller base" } dd { (v.base_region.as_deref().unwrap_or("Not established")) } } div { dt { "Legal entity" } dd { (v.legal_name.as_deref().unwrap_or("Not established")) } } div { dt { "Registration" } dd { (v.registration.as_deref().unwrap_or("Not established")) } } div { dt { "Ships to" } dd { (v.regions.join(", ")) } } div { dt { "Checked" } dd { (fmt::utc(v.observed_at.as_deref())) } } }
                @if let Some(label) = &v.verification_label { p class="vendor-verification" { strong { (label) } @if let Some(u) = &v.registry_url { " · " (ext(u, "Company record")) } @if let Some(u) = &v.verification_url { " · " (ext(u, "Relationship source")) } } }
                @if let Some(n) = &v.notes { p class="small" { (n) } }
                div class="buy-gallery seller-sheet vendor-offers" data-buy-gallery { @for l in rows { (listing_card(d, l)) } }
                p class="buy-honesty" { "This page records public evidence about " (v.name) ". It does not confirm inventory or guarantee fulfillment. equihash.com does not sell hardware." }
            }
        },
    )
}

pub fn add_vendor(d: &Data) -> Markup {
    let template = "Vendor / shop name:\nWebsite:\nRegions you ship to:\nEquihash ASIC product page URL(s):\nMachine model(s) (e.g. Antminer Z15 Pro):\nPrice, currency, VAT included? (as shown on your page):\nStock / availability wording:\nShipping notes (as shown on your page):\nContact for verification:\n";
    layout(
        d,
        Page {
            title: "Add a vendor",
            description:
                "How Equihash ASIC shops can ask to be listed in the equihash.com Buy directory.",
            path: "/add-vendor",
            nav: "buy",
        },
        html! {
            div class="wrap page narrow prose" {
                header class="page-head" { h1 { "Add a vendor" } p class="lede" { "Listing is free. equihash.com does not sell hardware or take payment for placement. A row only shows what can be checked on a public product page." } }
                h2 { "If you sell Equihash ASICs" }
                ol { li { strong { "Publish a product page. " } "Price, stock and shipping must be visible without an account. Anything you do not publish shows as n/a." } li { strong { "Send the details. " } "Copy the template and send it to " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak on X" } " or " a href="https://t.me/EquihashCom" rel="noopener" { "EquihashCom on Telegram" } ". It is checked against your public pages before listing." } }
                div class="tpl" { div class="tpl-head" { span { "Vendor template" } button class="btn small" type="button" data-copy="#tpl-vendor" { "Copy" } } pre id="tpl-vendor" { (template) } }
                p class="small" { "Manufacturer specifications stay on " a href="/hardware" { "Hardware" } "; this directory only indexes where to buy." }
            }
        },
    )
}
