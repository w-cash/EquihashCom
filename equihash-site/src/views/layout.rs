use crate::data::Data;
use crate::fmt;
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub const SITE: &str = "https://equihash.com";
/// Bump when static/app.css or static/app.js change so browsers don't keep a stale copy.
pub const ASSET_V: &str = "5";

pub struct Page<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub path: &'a str,
    pub nav: &'a str,
}

/// Main navigation: five places a miner actually goes. About, Add a pool and Sources live in the footer.
const NAV: &[(&str, &str, &str)] = &[
    ("pools", "/", "Pools"),
    ("miners", "/miners", "Hardware"),
    ("calculator", "/calculator", "Calculator"),
    ("merged-mining", "/merged-mining", "Merged mining"),
    ("archive", "/archive", "Archive"),
];

pub fn layout(d: &Data, p: Page, body: Markup) -> Markup {
    layout_at(d, p, body, chrono::Utc::now())
}

/// `layout` with an explicit clock, so the stale notice can be tested.
pub fn layout_at(d: &Data, p: Page, body: Markup, now: chrono::DateTime<chrono::Utc>) -> Markup {
    let canonical = format!("{SITE}{}", p.path);
    let full_title = if p.path == "/" || p.title.ends_with("· equihash.com") { p.title.to_string() } else { format!("{} · equihash.com", p.title) };
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
                meta name="theme-color" content="#f7f5ef" media="(prefers-color-scheme: light)";
                meta name="theme-color" content="#161614" media="(prefers-color-scheme: dark)";
                meta name="color-scheme" content="light dark";
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
                link rel="preload" href="/static/fonts/ibm-plex-sans-latin-400-normal.woff2" as="font" type="font/woff2" crossorigin;
                link rel="preload" href="/static/fonts/ibm-plex-mono-latin-400-normal.woff2" as="font" type="font/woff2" crossorigin;
                link rel="stylesheet" href={"/static/app.css?v=" (ASSET_V)};
                script { (PreEscaped("document.documentElement.classList.replace('no-js','js');")) }
                script type="application/ld+json" {
                    (PreEscaped(format!(r#"{{"@context":"https://schema.org","@type":"WebSite","name":"equihash.com","url":"{SITE}","description":"Equihash mining pools, hardware and network stats, with sources."}}"#)))
                }
            }
            body data-page=(p.nav) {
                a class="skip" href="#main" { "Skip to content" }
                header class="masthead" {
                    div class="wrap masthead-inner" {
                        a class="wordmark" href="/" { "equihash" span { ".com" } }
                        nav class="nav" aria-label="Main" {
                            @for (key, href, label) in NAV {
                                a href=(href) aria-current=[(*key == p.nav).then_some("page")] { (label) }
                            }
                        }
                    }
                }
                (stale_notice(updated.as_deref(), now))
                main id="main" { (body) }
                footer class="foot" {
                    div class="wrap foot-inner" {
                        p {
                            strong { "equihash.com" } " is maintained by " a href="https://x.com/RustDev_" rel="noopener" { "@RustDev_" } ". "
                            "Pool figures come from miningpoolstats and the pools' own public APIs; last refresh "
                            time class="ago" datetime=[updated.as_deref()] { (fmt::utc(updated.as_deref())) } ". "
                            "Nothing on this site is paid for, and none of it is financial advice."
                        }
                        p class="foot-links" {
                            a href="/add-pool" { "Add or correct a pool" }
                            a href="/sources" { "Sources and method" }
                            a href="/about" { "About" }
                            span { "Raw data: " a href="/data/pools.json" { "pools" } ", " a href="/data/network.json" { "networks" } ", " a href="/data/miners.json" { "hardware" } }
                        }
                    }
                }
                div id="drawer-root" {}
                script src={"/static/shared.js?v=" (ASSET_V)} defer {}
                script src={"/static/app.js?v=" (ASSET_V)} defer {}
            }
        }
    }
}

/// Site-wide notice when the newest refresh is older than `data::STALE_AFTER_SECS`.
pub fn stale_notice(ts: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> Markup {
    if !crate::data::is_stale(ts, now) {
        return html! {};
    }
    let age = fmt::age(crate::data::age_secs(ts, now));
    html! {
        div class="stale-note" role="status" {
            div class="wrap" {
                p {
                    strong { "Stale data. " }
                    @match age {
                        Some(a) => { "The newest pool figures here are " (a) " old (" (fmt::utc(ts)) "). " }
                        None => { "The age of these figures is unknown. " }
                    }
                    "Treat them as a snapshot and check the pool's own page before you switch."
                }
            }
        }
    }
}

pub fn ext(url: &str, label: &str) -> Markup {
    html! { a href=(url) rel="noopener nofollow" target="_blank" { (label) } }
}
