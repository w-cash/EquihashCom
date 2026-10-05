//! Equihash ASIC marketplace directory (/buy). equihash.com never sells hardware:
//! every CTA is an outbound link to the vendor product page.
use crate::data::{Data, Listing, Vendor};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use crate::views::logo::{self, At};
use crate::views::pages::copy_link;
use maud::{html, Markup};
use serde::Deserialize;
use std::cmp::Ordering;

#[derive(Debug, Deserialize, Default)]
pub struct BuyQuery {
    /// Filter to one miners.json id, e.g. antminer-z15-pro.
    pub machine: Option<String>,
    /// Region filter: UK, EU, US, … (empty = all).
    pub region: Option<String>,
    /// Stable directory order: region (default), price or stock.
    pub sort: Option<String>,
}

fn stock_rank(a: &str) -> u8 {
    match a {
        "in_stock" => 0,
        "low_stock" => 1,
        "preorder" => 2,
        "out_of_stock" => 3,
        _ => 4,
    }
}

fn region_rank(regions: &[String], focus: Option<&str>) -> u8 {
    let f = focus.unwrap_or("").to_ascii_uppercase();
    if f == "UK"
        || regions
            .iter()
            .any(|r| r.eq_ignore_ascii_case("UK") || r.eq_ignore_ascii_case("GB"))
    {
        return 0;
    }
    if regions.iter().any(|r| r.eq_ignore_ascii_case("EU")) {
        return 1;
    }
    if regions
        .iter()
        .any(|r| r.eq_ignore_ascii_case("US") || r.eq_ignore_ascii_case("USA"))
    {
        return 2;
    }
    3
}

fn listing_matches(l: &Listing, region: &str) -> bool {
    if region.is_empty() || region == "all" {
        return true;
    }
    l.shipping_regions
        .iter()
        .any(|r| r.eq_ignore_ascii_case(region))
}

fn cta(l: &Listing) -> Markup {
    let label = format!("View at {}", l.vendor_name);
    match crate::data::safe_url(&l.product_url) {
        Some(u) => html! {
            a class="buy-cta" href=(u) rel="noopener" target="_blank" { (label) }
        },
        None => html! { span class="na" { "Listing unavailable" } },
    }
}

fn price_block(l: &Listing) -> Markup {
    html! {
        span class="buy-price mono" {
            (fmt::money(l.price_amount, l.price_currency.as_deref()))
        }
        @if l.price_amount.is_some() {
            @match l.price_includes_vat {
                Some(false) => { span class="buy-vat" { "ex VAT" } }
                Some(true) => { span class="buy-vat" { "inc VAT" } }
                None => {}
            }
        }
    }
}

fn listing_card(d: &Data, l: &Listing, hidden: bool) -> Markup {
    let vendor = d.vendor(&l.vendor_id);
    let logo = vendor
        .map(|v| &v.logo)
        .cloned()
        .unwrap_or_else(|| d.logos.vendor(&l.vendor_id, &l.vendor_name));
    html! {
        article class="buy-card" data-machine=(l.miner_id) data-vendor=(l.vendor_id)
            data-region=(l.shipping_regions.join(","))
            data-price=(l.price_amount.map(|p| format!("{p}")).unwrap_or_default())
            data-stock=(l.availability.as_deref().unwrap_or("unknown"))
            data-stock-rank=(stock_rank(l.availability.as_deref().unwrap_or("unknown")))
            data-region-rank=(region_rank(&l.shipping_regions, vendor.and_then(|v| v.region_focus.as_deref())))
            hidden[hidden]
        {
            div class="buy-card-media" {
                @if let Some(src) = &l.image_src {
                    img src=(src) alt=(l.title) width="480" height="480" loading="lazy" decoding="async";
                } @else {
                    div class="buy-card-placeholder" aria-hidden="true" { "No photo" }
                }
            }
            div class="buy-card-body" {
                header class="buy-card-head" {
                    h3 class="buy-card-title" { (l.title) }
                    p class="buy-card-shop" {
                        (logo::chip(&logo, &l.vendor_name, At::List, true))
                        a href={"/buy/vendor/" (l.vendor_slug)} { (l.vendor_name) }
                    }
                }
                dl class="buy-specs" {
                    div {
                        dt { "Shop hashrate" }
                        dd class="mono" {
                            @if let Some(h) = l.shop_hashrate_ksol {
                                (fmt::num_short(h)) " kSol/s"
                            } @else { span class="na" { "n/a" } }
                        }
                    }
                    div {
                        dt { "Price" }
                        dd { (price_block(l)) }
                    }
                    div {
                        dt { "Availability" }
                        dd {
                            @if let Some(label) = &l.availability_label {
                                (label)
                            } @else if let Some(a) = &l.availability {
                                (a.replace('_', " "))
                            } @else { span class="na" { "n/a" } }
                        }
                    }
                    div {
                        dt { "Ships" }
                        dd {
                            @if l.shipping_regions.is_empty() {
                                span class="na" { "n/a" }
                            } @else {
                                (l.shipping_regions.join(", "))
                            }
                        }
                    }
                }
                @if let Some(n) = &l.hashrate_note {
                    p class="buy-note" { (n) }
                }
                @if let Some(n) = &l.shipping_note {
                    p class="buy-note" { (n) }
                }
                p class="buy-prov" {
                    "Observed "
                    (fmt::utc(l.observed_at.as_deref()))
                    " from "
                    (ext(l.source_url.as_deref().unwrap_or(&l.product_url), &fmt::host(Some(l.source_url.as_deref().unwrap_or(&l.product_url)))))
                    "."
                }
                div class="buy-actions" {
                    (cta(l))
                    a class="buy-quiet" href={"/miners#machine-" (l.miner_id)} { "Manufacturer specs" }
                }
            }
        }
    }
}

fn vendor_card(v: &Vendor) -> Markup {
    html! {
        article class="buy-vendor-card" {
            a class="buy-vendor-link" href={"/buy/vendor/" (v.slug)} {
                (logo::chip(&v.logo, &v.name, At::Head, false))
                span class="buy-vendor-meta" {
                    strong { (v.name) }
                    span class="buy-vendor-regions" {
                        @if v.regions.is_empty() { "Regions n/a" } @else { (v.regions.join(" · ")) }
                    }
                    span class="buy-vendor-count" {
                        @if v.listing_count == 1 { "1 listing" } @else { (v.listing_count) " listings" }
                    }
                }
            }
            @if let Some(u) = &v.url {
                p class="buy-vendor-site" { (ext(u, &fmt::host(Some(u)))) }
            }
        }
    }
}

/// Main /buy directory: machines as gallery groups, shops underneath.
pub fn index(d: &Data, q: &BuyQuery) -> Markup {
    let machine_filter = q.machine.as_deref().unwrap_or("").trim().to_string();
    let region_filter = q.region.as_deref().unwrap_or("").trim().to_string();
    let sort = match q.sort.as_deref() {
        Some("price") => "price",
        Some("stock") => "stock",
        _ => "region",
    };
    let mut groups = d.buy_machine_groups();
    for (_, _, listings) in &mut groups {
        listings.sort_by(|a, b| {
            let price = || match (a.price_amount, b.price_amount) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };
            let primary = match sort {
                "price" => price(),
                "stock" => stock_rank(a.availability.as_deref().unwrap_or("unknown"))
                    .cmp(&stock_rank(b.availability.as_deref().unwrap_or("unknown"))),
                _ => {
                    let ar = region_rank(
                        &a.shipping_regions,
                        d.vendor(&a.vendor_id)
                            .and_then(|v| v.region_focus.as_deref()),
                    );
                    let br = region_rank(
                        &b.shipping_regions,
                        d.vendor(&b.vendor_id)
                            .and_then(|v| v.region_focus.as_deref()),
                    );
                    ar.cmp(&br).then_with(price)
                }
            };
            primary
                .then_with(|| a.vendor_name.cmp(&b.vendor_name))
                .then_with(|| a.id.cmp(&b.id))
        });
    }
    let regions: Vec<String> = {
        let mut r: Vec<String> = d
            .listings
            .iter()
            .flat_map(|l| l.shipping_regions.iter().cloned())
            .collect();
        r.sort();
        r.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
        r
    };
    let visible_count = d
        .listings
        .iter()
        .filter(|l| {
            (machine_filter.is_empty() || l.miner_id == machine_filter)
                && listing_matches(l, &region_filter)
        })
        .count();

    layout(
        d,
        Page {
            title: "Buy Equihash ASICs: Z15 Pro and Z-series shop directory",
            description: "Where to buy Equihash ASICs (Antminer Z15 Pro and related). Prices and stock from vendor sites; equihash.com never sells hardware.",
            path: "/buy",
            nav: "buy",
        },
        html! {
            div class="wrap page buy-page" {
                header class="page-head buy-head" {
                    h1 { "Buy" }
                    p class="lede" {
                        "A calm directory of Equihash ASIC shops. Compare machines and vendors, then leave to buy on the shop's own site. "
                        strong { "equihash.com never sells hardware" }
                        " and does not take payment for placement."
                    }
                }

                form class="filters buy-filters" id="buy-filters" action="/buy" method="get" {
                    div class="f" {
                        label for="buy-machine" { "Machine" }
                        select id="buy-machine" name="machine" data-buy-filter="machine" {
                            option value="" selected[machine_filter.is_empty()] { "All machines" }
                            @for (id, label, _) in &groups {
                                option value=(id) selected[machine_filter == *id] { (label) }
                            }
                        }
                    }
                    div class="f" {
                        label for="buy-region" { "Region" }
                        select id="buy-region" name="region" data-buy-filter="region" {
                            option value="" selected[region_filter.is_empty()] { "All regions" }
                            @for r in &regions {
                                option value=(r) selected[region_filter.eq_ignore_ascii_case(r)] { (r) }
                            }
                        }
                    }
                    div class="f f-sort" {
                        label for="buy-sort" { "Sort" }
                        select id="buy-sort" name="sort" data-buy-sort {
                            option value="region" selected[sort == "region"] { "Region (UK first)" }
                            option value="price" selected[sort == "price"] { "Price (low → high)" }
                            option value="stock" selected[sort == "stock"] { "Availability" }
                        }
                    }
                    button class="buy-apply" type="submit" { "Apply" }
                    p class="buy-count" id="buy-count" aria-live="polite" {
                        @if visible_count == 1 { "1 listing" } @else { (visible_count) " listings" }
                        @if let Some(t) = &d.listings_verified_at {
                            " · checked " (fmt::utc(Some(t)))
                        }
                    }
                }

                @if groups.is_empty() {
                    p class="buy-empty" {
                        "No listings yet. Vendors can ask to be added via "
                        a href="/add-vendor" { "Add a vendor" }
                        "."
                    }
                } @else {
                    @for (miner_id, label, listings) in &groups {
                        @let show_group = machine_filter.is_empty() || machine_filter == *miner_id;
                        @let group_visible = show_group && listings.iter().any(|l| listing_matches(l, &region_filter));
                        section class="buy-machine" id={(format!("machine-{miner_id}"))} data-machine=(miner_id) hidden[!group_visible] {
                                header class="buy-machine-head" {
                                    h2 { (label) }
                                    p class="section-sub" {
                                        "Manufacturer specs on "
                                        a href={"/miners#machine-" (miner_id)} { "Hardware" }
                                        ". Shop-stated hashrate is shown on each card."
                                    }
                                }
                                div class="buy-gallery" data-buy-gallery {
                                    @for l in listings {
                                        @let card_visible = show_group && listing_matches(l, &region_filter);
                                        (listing_card(d, l, !card_visible))
                                    }
                                }
                            }
                    }
                }

                section class="buy-vendors" id="vendors" {
                    header class="buy-machine-head" {
                        h2 { "Vendors" }
                        p class="section-sub" {
                            "Shops that want a place here: clear Equihash ASIC stock, a public product page, and a region. "
                            a href="/add-vendor" { "Add a vendor" } "."
                            @if let Some(t) = &d.vendors_verified_at {
                                " Directory checked " (fmt::utc(Some(t))) "."
                            }
                        }
                    }
                    div class="buy-vendor-grid" {
                        @for v in &d.vendors {
                            (vendor_card(v))
                        }
                    }
                }

                footer class="buy-honesty" {
                    p {
                        "Every “View at …” button opens the vendor's product page in a new tab. "
                        "This site has no checkout, no cart and no paid placement. "
                        "Prices and stock are copied from the linked page with the time they were checked; "
                        "n/a means not published — never a guessed zero."
                    }
                }
            }
        },
    )
}

pub fn vendor(d: &Data, slug: &str) -> Option<Markup> {
    let v = d.vendor(slug)?;
    let listings: Vec<&Listing> = d.listings.iter().filter(|l| l.vendor_id == v.id).collect();
    let path = format!("/buy/vendor/{}", v.slug);
    let title = format!("{} — Equihash ASIC vendor", v.name);
    let desc = format!(
        "Equihash ASIC listings from {}. Open each product on the vendor site to buy.",
        v.name
    );
    Some(layout(
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
                header class="page-head" {
                    div class="title-row" {
                        h1 { (logo::chip(&v.logo, &v.name, At::Head, false)) (v.name) }
                        (copy_link(&path, "Copy link to this vendor"))
                    }
                    p class="lede" {
                        @if let Some(n) = &v.region_note { (n) " " }
                        @else if !v.regions.is_empty() { "Regions: " (v.regions.join(", ")) ". " }
                        @if let Some(u) = &v.url {
                            "Site: " (ext(u, &fmt::host(Some(u)))) "."
                        }
                    }
                }
                dl class="kv buy-vendor-kv" {
                    div { dt { "Listings here" } dd { (v.listing_count) } }
                    div { dt { "Regions" } dd { @if v.regions.is_empty() { span class="na" { "n/a" } } @else { (v.regions.join(", ")) } } }
                    div {
                        dt { "Checked" }
                        dd {
                            (fmt::utc(v.observed_at.as_deref()))
                            @if let Some(u) = &v.source_url {
                                " · " (ext(u, &fmt::host(Some(u))))
                            }
                        }
                    }
                }
                @if let Some(n) = &v.notes { p class="small" { (n) } }
                @if listings.is_empty() {
                    p { "No listings from this vendor yet." }
                } @else {
                    div class="buy-gallery" data-buy-gallery {
                        @for l in &listings {
                            (listing_card(d, l, false))
                        }
                    }
                }
                p class="buy-honesty" {
                    "To buy, open the listing on " (v.name) "'s own site. equihash.com does not sell hardware."
                }
            }
        },
    ))
}

pub fn add_vendor(d: &Data) -> Markup {
    let template = "Vendor / shop name:\nWebsite:\nRegions you ship to:\nEquihash ASIC product page URL(s):\nMachine model(s) (e.g. Antminer Z15 Pro):\nPrice, currency, VAT included? (as shown on your page):\nStock / availability wording:\nShipping notes (as shown on your page):\nContact for verification:\n";
    layout(
        d,
        Page {
            title: "Add a vendor",
            description:
                "How Equihash ASIC shops can ask to be listed on the equihash.com Buy directory.",
            path: "/add-vendor",
            nav: "buy",
        },
        html! {
            div class="wrap page narrow prose" {
                header class="page-head" {
                    h1 { "Add a vendor" }
                    p class="lede" {
                        "Listing is free. equihash.com does not sell hardware and does not take payment for placement. "
                        "A row only shows what can be checked on your public product page."
                    }
                }
                h2 { "If you sell Equihash ASICs" }
                ol {
                    li {
                        strong { "Publish a product page. " }
                        "Price, stock and shipping must be visible without an account. Anything you don't publish shows as n/a."
                    }
                    li {
                        strong { "Send the details. " }
                        "Copy the template below and send it to "
                        a href="https://x.com/RustDev_" rel="noopener" { "@RustDev_" }
                        ". It is checked against your public pages before a listing is added, and the row cites its source and the time it was checked."
                    }
                }
                div class="tpl" {
                    div class="tpl-head" { span { "Template" } button class="btn small" type="button" data-copy="#tpl-vendor" { "Copy" } }
                    pre id="tpl-vendor" { (template) }
                }
                p class="small" {
                    "Hardware manufacturer specs stay on "
                    a href="/miners" { "Hardware" }
                    "; this directory only indexes where to buy."
                }
            }
        },
    )
}
