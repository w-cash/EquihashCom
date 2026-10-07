use crate::data::Data;
use crate::fmt;
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub const SITE: &str = "https://equihash.com";
/// Bump when static/app.css or static/app.js change so browsers don't keep a stale copy.
pub const ASSET_V: &str = "16";

/// The one inline script (swaps the no-js class before first paint). Its SHA-256 is allowed by
/// the Content-Security-Policy (see `csp`), so no other inline script can run.
pub const INLINE_SCRIPT: &str = "document.documentElement.classList.replace('no-js','js');";

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for ch in bytes.chunks(3) {
        let b = [ch[0], *ch.get(1).unwrap_or(&0), *ch.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            if i <= ch.len() {
                out.push(T[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// Content-Security-Policy for every response: same-origin scripts plus the hashed inline
/// script, same-origin fetches (/api/live), fonts and images; inline style attributes (bar
/// widths) are allowed; no plugins, no framing, no <base>, forms only to this site.
pub fn csp() -> String {
    let h = base64(ring::digest::digest(&ring::digest::SHA256, INLINE_SCRIPT.as_bytes()).as_ref());
    format!("default-src 'self'; script-src 'self' 'sha256-{h}'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; manifest-src 'self'; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'")
}

pub struct Page<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub path: &'a str,
    pub nav: &'a str,
}

/// Main navigation follows the four things people come here to find. Calculator stays visible as
/// the primary working tools; About, Contribute, Sources and the archive live in the footer.
const NAV: &[(&str, &str, &str)] = &[
    ("coins", "/coins", "Coins"),
    ("pools", "/pools", "Pools"),
    ("hardware", "/hardware", "Hardware"),
    ("buy", "/buy", "Buy"),
    ("guides", "/guides", "Guides"),
    ("calculator", "/calculator", "Calculator"),
    ("merged-mining", "/merged-mining", "Merged mining"),
];

pub fn layout(d: &Data, p: Page, body: Markup) -> Markup {
    layout_at(d, p, body, chrono::Utc::now())
}

/// `layout` with an explicit clock, so the stale notice can be tested.
pub fn layout_at(d: &Data, p: Page, body: Markup, now: chrono::DateTime<chrono::Utc>) -> Markup {
    let canonical = format!("{SITE}{}", p.path);
    let full_title = if p.path == "/" || p.title.ends_with("· equihash.com") {
        p.title.to_string()
    } else {
        format!("{} · equihash.com", p.title)
    };
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
                script { (PreEscaped(INLINE_SCRIPT)) }
                script type="application/ld+json" {
                    (PreEscaped(format!(r#"{{"@context":"https://schema.org","@type":"WebSite","name":"equihash.com","url":"{SITE}","description":"Equihash coins, mining pools, ASIC hardware, mining calculator and setup guides.","potentialAction":{{"@type":"SearchAction","target":"{SITE}/search?q={{search_term_string}}","query-input":"required name=search_term_string"}}}}"#)))
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
                            strong { "equihash.com" } " is maintained by " a href="https://x.com/MykytaSamardak" rel="noopener" { "@MykytaSamardak" } ". Community updates: " a href="https://t.me/EquihashCom" rel="noopener" { "t.me/EquihashCom" } ". "
                            "Pool figures come from miningpoolstats and the pools' own public APIs; last refresh "
                            time class="ago" datetime=[updated.as_deref()] { (fmt::utc(updated.as_deref())) } ". "
                            "Nothing on this site is paid for, and none of it is financial advice."
                        }
                        p class="foot-links" {
                            a href="/contribute" { "Add a listing or correction" }
                            a href="/add-vendor" { "Add a vendor" }
                            a href="/archive" { "Archive" }
                            a href="/sources" { "Sources and method" }
                            a href="/about" { "About" }
                            span { "Raw data: " a href="/data/pools.json" { "pools" } ", " a href="/data/network.json" { "networks" } ", " a href="/data/miners.json" { "hardware" } ", " a href="/data/vendors.json" { "vendors" } ", " a href="/data/listings.json" { "listings" } }
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

/// An outbound link. Only http(s) URLs become links (data::safe_url); anything else is shown as
/// plain text, so a bad upstream value can never become a javascript: or data: link.
pub fn ext(url: &str, label: &str) -> Markup {
    match crate::data::safe_url(url) {
        Some(u) => html! { a href=(u) rel="noopener nofollow" target="_blank" { (label) } },
        None => html! { span class="na" { (label) } },
    }
}
