use crate::data::Data;
use crate::fmt;
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub const SITE: &str = "https://equihash.com";

pub struct Page<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub path: &'a str,
    pub nav: &'a str,
}

const NAV: &[(&str, &str, &str)] = &[
    ("pools", "/", "Pools"),
    ("miners", "/miners", "Miners"),
    ("calculator", "/calculator", "Calculator"),
    ("merged-mining", "/merged-mining", "Merged mining"),
    ("archive", "/archive", "Archive"),
    ("add-pool", "/add-pool", "Add pool"),
    ("about", "/about", "About"),
];

pub fn logo() -> Markup {
    html! {
        svg class="logo-mark" viewBox="0 0 32 32" aria-hidden="true" {
            defs { linearGradient id="lg" x1="0" y1="0" x2="1" y2="1" { stop offset="0" stop-color="#f5c451" {} stop offset="1" stop-color="#f08a24" {} } }
            rect x="1" y="1" width="30" height="30" rx="8" fill="url(#lg)" {}
            path d="M9 10h14M9 16h10M9 22h14" stroke="#14110a" stroke-width="3.2" stroke-linecap="round" fill="none" {}
        }
    }
}

pub fn layout(d: &Data, p: Page, body: Markup) -> Markup {
    let canonical = format!("{SITE}{}", p.path);
    let full_title = if p.path == "/" { p.title.to_string() } else { format!("{} · equihash.com", p.title) };
    let updated = d.last_updated.clone();
    html! {
        (DOCTYPE)
        html lang="en" class="no-js" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                title { (full_title) }
                meta name="description" content=(p.description);
                link rel="canonical" href=(canonical);
                meta name="theme-color" content="#0b0d12";
                meta name="color-scheme" content="dark light";
                meta property="og:type" content="website";
                meta property="og:site_name" content="equihash.com";
                meta property="og:title" content=(full_title);
                meta property="og:description" content=(p.description);
                meta property="og:url" content=(canonical);
                meta property="og:image" content=(format!("{SITE}/static/og.png"));
                meta property="og:image:width" content="1200";
                meta property="og:image:height" content="630";
                meta name="twitter:card" content="summary_large_image";
                meta name="twitter:title" content=(full_title);
                meta name="twitter:description" content=(p.description);
                meta name="twitter:image" content=(format!("{SITE}/static/og.png"));
                link rel="icon" href="/favicon.ico" sizes="32x32";
                link rel="icon" href="/static/favicon.svg" type="image/svg+xml";
                link rel="apple-touch-icon" href="/static/apple-touch-icon.png";
                link rel="stylesheet" href="/static/app.css?v=1";
                script { (PreEscaped("document.documentElement.classList.replace('no-js','js');")) }
                script type="application/ld+json" {
                    (PreEscaped(format!(r#"{{"@context":"https://schema.org","@type":"WebSite","name":"equihash.com","url":"{SITE}","description":"Neutral, sourced directory of Equihash mining pools, ASICs and network stats."}}"#)))
                }
            }
            body data-page=(p.nav) {
                a class="skip" href="#main" { "Skip to content" }
                header class="site-header" {
                    div class="wrap header-inner" {
                        a class="brand" href="/" aria-label="equihash.com home" { (logo()) span { "equihash" b { ".com" } } }
                        nav class="nav-desktop" aria-label="Main" {
                            @for (key, href, label) in NAV {
                                a href=(href) class=@if *key == p.nav { "active" } { (label) }
                            }
                        }
                        details class="nav-mobile" {
                            summary aria-label="Menu" { span {} span {} span {} }
                            nav aria-label="Mobile" {
                                @for (key, href, label) in NAV {
                                    a href=(href) class=@if *key == p.nav { "active" } { (label) }
                                }
                            }
                        }
                    }
                }
                main id="main" { (body) }
                footer class="site-footer" {
                    div class="wrap footer-grid" {
                        div {
                            a class="brand small" href="/" { (logo()) span { "equihash" b { ".com" } } }
                            p class="muted" { "A neutral, miner-first directory of Equihash mining pools. Every number links to its source; unknown values are shown as n/a, never estimated." }
                        }
                        div {
                            h4 { "Data" }
                            ul {
                                li { "Last updated: " time class="ago" datetime=[updated.as_deref()] { (fmt::utc(updated.as_deref())) } }
                                li { a href="/sources" { "Sources & methodology" } }
                                li { a href="/data/pools.json" { "pools.json" } " · " a href="/data/network.json" { "network.json" } " · " a href="/data/miners.json" { "miners.json" } }
                            }
                        }
                        div {
                            h4 { "Site" }
                            ul {
                                li { a href="/add-pool" { "Add or update your pool" } }
                                li { a href="/merged-mining" { "Merged-mining guide for pools" } }
                                li { a href="/about" { "About" } }
                            }
                        }
                    }
                    div class="wrap fine" { "Not financial advice. Pool data from miningpoolstats.stream and pools' own public pages/APIs; see " a href="/sources" { "sources" } "." }
                }
                div id="drawer-root" {}
                script src="/static/app.js?v=1" defer {}
            }
        }
    }
}

pub fn ext(url: &str, label: &str) -> Markup {
    html! { a href=(url) rel="noopener nofollow" target="_blank" { (label) } }
}
