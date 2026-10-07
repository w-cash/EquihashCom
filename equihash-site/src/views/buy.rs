//! Source-backed Equihash ASIC shop directory. This is an outbound directory only: no checkout,
//! paid placement or inferred stock. Every commercial claim keeps its source and observation time.

use crate::data::{Data, Listing, Vendor};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use crate::views::logo;
use maud::{html, Markup};
use serde::Deserialize;

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
        span class={"logo mono" @if head { " lg-head" } @else { " lg-list" }} aria-hidden="true" { (logo::monogram(&v.name)) }
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
    let region_rank = if l.shipping_regions.iter().any(|r| r == "UK") {
        0
    } else {
        1
    };
    html! {
        article class="buy-card" data-machine=(l.miner_id) data-vendor=(l.vendor_id) data-region=(l.shipping_regions.join(",")) data-price=[l.price_amount.map(|x| x.to_string())] data-stock=(l.availability.as_deref().unwrap_or("unknown")) data-stock-rank=(stock_rank) data-region-rank=(region_rank) {
            div class="buy-card-media" {
                div class="miner-plate" aria-hidden="true" { span { "EQUIHASH 200,9" } strong { "Z15 PRO" } small { "ASIC MINER" } }
            }
            div class="buy-card-body" {
                header class="buy-card-head" {
                    h3 class="buy-card-title" { (l.title) }
                    p class="buy-card-shop" {
                        @if let Some(v) = v { (vendor_mark(v, false)) }
                        @if !slug.is_empty() { a href={"/buy/vendor/" (slug)} { (vendor_name) } } @else { (vendor_name) }
                    }
                }
                dl class="buy-specs" {
                    div { dt { "Shop hashrate" } dd class="mono" { (fmt::opt_num(l.shop_hashrate_ksol)) " kSol/s" } }
                    div {
                        dt { "Price" }
                        dd {
                            span class="buy-price mono" { (currency(l.price_amount, l.price_currency.as_deref())) }
                            @if l.price_includes_vat == Some(false) {
                                span class="buy-vat" { "ex VAT" }
                            }
                        }
                    }
                    div { dt { "Availability" } dd { (l.availability_label.as_deref().unwrap_or("n/a")) } }
                    div { dt { "Ships" } dd { @if l.shipping_regions.is_empty() { "n/a" } @else { (l.shipping_regions.join(", ")) } } }
                }
                @if let Some(n) = &l.hashrate_note { p class="buy-note" { (n) } }
                @if let Some(n) = &l.shipping_note { p class="buy-note" { (n) } }
                p class="buy-prov" { "Observed " (fmt::utc(l.observed_at.as_deref())) @if let Some(u) = &l.source_url { " from " (ext(u, &fmt::host(Some(u)))) } "." }
                div class="buy-actions" {
                    @if let Some(u) = &l.product_url { a class="buy-cta" href=(u) rel="noopener nofollow" target="_blank" { "View at " (vendor_name) } }
                    a class="buy-quiet" href={"/hardware/" (l.miner_id)} { "Manufacturer specs" }
                }
            }
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
                        .any(|r| r.eq_ignore_ascii_case(region)))
        })
        .collect();
    listings.sort_by(|a, b| match sort {
        "price" => a
            .price_amount
            .partial_cmp(&b.price_amount)
            .unwrap_or(std::cmp::Ordering::Equal),
        "stock" => (a.availability.as_deref() != Some("in_stock"))
            .cmp(&(b.availability.as_deref() != Some("in_stock"))),
        _ => (!a.shipping_regions.iter().any(|r| r == "UK"))
            .cmp(&(!b.shipping_regions.iter().any(|r| r == "UK")))
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
    layout(d, Page { title: "Buy Equihash ASICs: Z15 Pro shop directory", description: "Where to buy Equihash ASICs including the Antminer Z15 Pro. Compare sourced prices, stock and shipping, then buy on the vendor's own site.", path: "/buy", nav: "buy" }, html! {
        div class="wrap page buy-page" {
            header class="page-head buy-head" {
                p class="eyebrow" { "SOURCE-BACKED SHOP DIRECTORY" }
                h1 { "Where to buy Equihash ASICs" }
                p class="lede" { "Compare sourced vendor listings, then leave to buy on the shop's own site. " strong { "equihash.com never sells hardware" } " and does not take payment for placement." }
            }
            form class="filters buy-filters" id="buy-filters" action="/buy" method="get" {
                div class="f" { label for="buy-machine" { "Machine" } select id="buy-machine" name="machine" data-buy-filter="machine" { option value="" selected[machine.is_empty()] { "All machines" } @for m in &machines { option value=(m.id) selected[m.id == machine] { (m.maker) " " (m.model) } } } }
                div class="f" { label for="buy-region" { "Region" } select id="buy-region" name="region" data-buy-filter="region" { option value="" selected[region.is_empty()] { "All regions" } @for r in &regions { option value=(r) selected[r == region] { (r) } } } }
                div class="f f-sort" { label for="buy-sort" { "Sort" } select id="buy-sort" name="sort" data-buy-sort { option value="region" selected[sort == "region"] { "Region (UK first)" } option value="price" selected[sort == "price"] { "Price (low → high)" } option value="stock" selected[sort == "stock"] { "Availability" } } }
                button class="buy-apply" type="submit" { "Apply" }
                p class="buy-count" id="buy-count" aria-live="polite" { (listings.len()) @if listings.len() == 1 { " listing" } @else { " listings" } @if let Some(t) = d.listings_verified_at.as_deref().or(d.vendors_verified_at.as_deref()) { " · checked " (fmt::utc(Some(t))) } }
            }
            @if listings.is_empty() { div class="empty-state" { h2 { "No listing matches those filters" } p { "Clear a filter or send a public product page for review." } a href="/add-vendor" { "Add a vendor" } } }
            @for m in &machines {
                @let rows: Vec<_> = listings.iter().filter(|l| l.miner_id == m.id).cloned().collect();
                @if !rows.is_empty() { section class="buy-machine" id={"machine-" (m.id)} data-machine=(m.id) {
                    header class="buy-machine-head" { h2 { (m.maker) " " (m.model) } p class="section-sub" { "Manufacturer specs on " a href={"/hardware/" (m.id)} { "Hardware" } ". Shop-stated hashrate is shown on each card." } }
                    div class="buy-gallery" data-buy-gallery { @for l in rows { (listing_card(d, l)) } }
                } }
            }
            section class="buy-vendors" id="vendors" {
                header class="buy-machine-head" { h2 { "Vendors" } p class="section-sub" { "Shops need clear Equihash ASIC stock, a public product page and a shipping region. " a href="/add-vendor" { "Add a vendor" } "." } }
                div class="buy-vendor-grid" { @for v in &d.vendors { @let n = d.listings.iter().filter(|l| l.vendor_id == v.id).count(); article class="buy-vendor-card" {
                    a class="buy-vendor-link" href={"/buy/vendor/" (v.slug)} { (vendor_mark(v, true)) span class="buy-vendor-meta" { strong { (v.name) } span class="buy-vendor-regions" { (v.regions.join(" · ")) } span class="buy-vendor-count" { (n) @if n == 1 { " listing" } @else { " listings" } } } }
                    p class="buy-vendor-site" { @if let Some(u) = &v.url { (ext(u, &fmt::host(Some(u)))) } }
                } } }
            }
            footer class="buy-honesty" { p { "Every “View at …” button opens the vendor's product page. This site has no checkout, cart or paid placement. Prices and stock are copied from the linked page with the time checked; n/a means not published." } }
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
            header class="page-head" { div class="title-row" { h1 { (vendor_mark(v, true)) (v.name) } (crate::views::pages::copy_link(&path, "Copy link to this vendor")) } p class="lede" { (v.region_note.as_deref().unwrap_or("Sourced vendor listing.")) @if let Some(u) = &v.url { " Site: " (ext(u, &fmt::host(Some(u)))) "." } } }
                dl class="buy-vendor-kv" { div { dt { "Listings here" } dd { (rows.len()) } } div { dt { "Regions" } dd { (v.regions.join(", ")) } } div { dt { "Checked" } dd { (fmt::utc(v.observed_at.as_deref())) @if let Some(u) = &v.source_url { " · " (ext(u, &fmt::host(Some(u)))) } } } }
                @if let Some(n) = &v.notes { p class="small" { (n) } }
                div class="buy-gallery" data-buy-gallery { @for l in rows { (listing_card(d, l)) } }
                p class="buy-honesty" { "To buy, open the listing on " (v.name) "'s own site. equihash.com does not sell hardware." }
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
                ol { li { strong { "Publish a product page. " } "Price, stock and shipping must be visible without an account. Anything you do not publish shows as n/a." } li { strong { "Send the details. " } "Copy the template and send it to " a href="https://x.com/RustDev_" rel="noopener" { "@RustDev_" } ". It is checked against your public pages before listing." } }
                div class="tpl" { div class="tpl-head" { span { "Vendor template" } button class="btn small" type="button" data-copy="#tpl-vendor" { "Copy" } } pre id="tpl-vendor" { (template) } }
                p class="small" { "Manufacturer specifications stay on " a href="/hardware" { "Hardware" } "; this directory only indexes where to buy." }
            }
        },
    )
}
