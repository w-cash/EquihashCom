use crate::data::Data;
use crate::fmt;
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub const SITE: &str = "https://equihash.com";
/// Bump when static/app.css or static/app.js change so browsers don't keep a stale copy.
pub const ASSET_V: &str = "29";

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
    ("merged-mining", "/merged-mining", "Merged mining"),
    ("hashpower", "/hashpower", "Hashpower"),
    ("hardware", "/hardware", "Hardware"),
    ("buy", "/buy", "Buy"),
    ("guides", "/guides", "Guides"),
    ("calculator", "/calculator", "Calculator"),
];

pub fn layout(d: &Data, p: Page, body: Markup) -> Markup {
    layout_at(d, p, body, chrono::Utc::now())
}

/// `layout` with an explicit clock for deterministic page rendering in tests.
pub fn layout_at(d: &Data, p: Page, body: Markup, _now: chrono::DateTime<chrono::Utc>) -> Markup {
    let canonical = format!("{SITE}{}", p.path);
    let full_title = if p.path == "/" || p.title.ends_with("· equihash.com") {
        p.title.to_string()
    } else {
        format!("{} · equihash.com", p.title)
    };
    let updated = d.last_updated.clone();
    let robots = if matches!(p.path, "/search" | "/404" | "/contribute" | "/add-vendor" | "/add-pool") {
        "noindex, follow"
    } else {
        "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1"
    };
    let og_type = if matches!(p.path, "/zcash-mining" | "/merged-mining") {
        "article"
    } else {
        "website"
    };
    let json_ld = structured_data(d, &p, &full_title, &canonical);
    html! {
        (DOCTYPE)
        html lang="en" class="no-js" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover";
                title { (full_title) }
                meta name="description" content=(p.description);
                meta name="robots" content=(robots);
                meta name="author" content="Mykyta Samardak";
                link rel="canonical" href=(canonical);
                link rel="alternate" type="text/plain" href="/llms.txt" title="AI-readable site guide";
                @for (href, title) in data_alternates(p.path) {
                    link rel="alternate" type="application/json" href=(href) title=(title);
                }
                meta name="theme-color" content="#f7f5ef" media="(prefers-color-scheme: light)";
                meta name="theme-color" content="#161614" media="(prefers-color-scheme: dark)";
                meta name="color-scheme" content="light dark";
                meta property="og:type" content=(og_type);
                meta property="og:locale" content="en_US";
                meta property="og:site_name" content="equihash.com";
                meta property="og:title" content=(full_title);
                meta property="og:description" content=(p.description);
                meta property="og:url" content=(canonical);
                meta property="og:image" content=(format!("{SITE}/static/og.png"));
                meta property="og:image:alt" content="equihash.com mining directory";
                meta property="og:image:width" content="1200";
                meta property="og:image:height" content="630";
                meta name="twitter:card" content="summary_large_image";
                meta name="twitter:title" content=(full_title);
                meta name="twitter:description" content=(p.description);
                meta name="twitter:image" content=(format!("{SITE}/static/og.png"));
                meta name="twitter:image:alt" content="equihash.com mining directory";
                link rel="icon" href="/favicon.ico" sizes="32x32";
                link rel="icon" href="/static/favicon.svg" type="image/svg+xml";
                link rel="apple-touch-icon" href="/static/apple-touch-icon.png";
                link rel="preload" href="/static/fonts/ibm-plex-sans-latin-400-normal.woff2" as="font" type="font/woff2" crossorigin;
                link rel="preload" href="/static/fonts/ibm-plex-mono-latin-400-normal.woff2" as="font" type="font/woff2" crossorigin;
                link rel="stylesheet" href={"/static/app.css?v=" (ASSET_V)};
                script { (PreEscaped(INLINE_SCRIPT)) }
                script type="application/ld+json" {
                    (PreEscaped(json_ld))
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
                main id="main" { (body) }
                footer class="foot" {
                    div class="wrap foot-inner" {
                        p {
                            strong { "equihash.com" } " is maintained by " a href="https://x.com/MykytaSamardak" rel="me noopener" { "@MykytaSamardak" } ". Community updates: " a href="https://t.me/EquihashCom" rel="noopener" { "t.me/EquihashCom" } ". "
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
                            span { "Raw data: " a href="/data/pools.json" { "pools" } ", " a href="/data/network.json" { "networks" } ", " a href="/data/miners.json" { "hardware" } ", " a href="/data/vendors.json" { "vendors" } ", " a href="/data/listings.json" { "listings" } ", " a href="/data/hashpower.json" { "hashpower" } }
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

fn data_alternates(path: &str) -> Vec<(&'static str, &'static str)> {
    if path == "/" || path == "/coins" || path.starts_with("/coin/") {
        vec![("/data/network.json", "Equihash network data")]
    } else if path.starts_with("/pools") || path.starts_with("/pool/") || path == "/zcash-mining" {
        vec![("/data/pools.json", "Equihash pool data"), ("/data/network.json", "Equihash network data")]
    } else if path == "/hardware" || path.starts_with("/hardware/") {
        vec![("/data/miners.json", "Equihash hardware data")]
    } else if path == "/buy" || path.starts_with("/buy/vendor/") {
        vec![("/data/vendors.json", "ASIC vendor data"), ("/data/listings.json", "ASIC offer data")]
    } else if path == "/hashpower" {
        vec![("/data/hashpower.json", "Equihash hashpower market data")]
    } else {
        Vec::new()
    }
}

fn structured_data(d: &Data, p: &Page<'_>, full_title: &str, canonical: &str) -> String {
    use serde_json::{json, Map, Value};

    let website_id = format!("{SITE}/#website");
    let publisher_id = format!("{SITE}/#publisher");
    let webpage_id = format!("{canonical}#webpage");
    let page_type = match p.path {
        "/coins" | "/hardware" | "/buy" | "/guides" | "/archive" => "CollectionPage",
        "/about" => "AboutPage",
        "/calculator" => "WebApplication",
        "/zcash-mining" | "/merged-mining" => "TechArticle",
        path if path.starts_with("/pools") => "CollectionPage",
        path if path.starts_with("/coin/")
            || path.starts_with("/pool/")
            || path.starts_with("/hardware/")
            || path.starts_with("/buy/vendor/") => "ItemPage",
        _ => "WebPage",
    };

    let mut graph = vec![json!({
        "@type": "Person",
        "@id": publisher_id,
        "name": "Mykyta Samardak",
        "url": "https://equihash.com/about",
        "sameAs": ["https://x.com/MykytaSamardak", "https://t.me/EquihashCom"]
    })];
    if p.path == "/" {
        graph.insert(0, json!({
            "@type": "WebSite",
            "@id": website_id,
            "url": SITE,
            "name": "equihash.com",
            "alternateName": "Equihash",
            "description": "Source-backed Equihash mining directory for Zcash and other Equihash networks, pools, ASIC hardware, sellers, calculators and guides.",
            "inLanguage": "en",
            "publisher": {"@id": publisher_id},
            "potentialAction": {
                "@type": "SearchAction",
                "target": {"@type": "EntryPoint", "urlTemplate": format!("{SITE}/search?q={{search_term_string}}")},
                "query-input": "required name=search_term_string"
            }
        }));
    }

    let entity = structured_entity(d, p, canonical);
    let mut webpage = Map::new();
    webpage.insert("@type".into(), json!(page_type));
    webpage.insert("@id".into(), json!(webpage_id));
    webpage.insert("url".into(), json!(canonical));
    webpage.insert("name".into(), json!(full_title));
    webpage.insert("description".into(), json!(p.description));
    if p.path == "/" {
        webpage.insert("isPartOf".into(), json!({"@id": website_id}));
    }
    webpage.insert("inLanguage".into(), json!("en"));
    webpage.insert("publisher".into(), json!({"@id": publisher_id}));
    if let Some(ts) = d.last_updated.as_deref() {
        webpage.insert("dateModified".into(), json!(ts));
    }
    if matches!(p.path, "/zcash-mining" | "/merged-mining") {
        webpage.insert("headline".into(), json!(p.title));
        webpage.insert("author".into(), json!({"@id": publisher_id}));
        webpage.insert("datePublished".into(), json!("2026-10-07"));
        webpage.insert("dateModified".into(), json!("2026-10-07"));
        webpage.insert("about".into(), json!([
            {"@type": "Thing", "name": "Zcash mining"},
            {"@type": "Thing", "name": "Equihash"},
            {"@type": "Thing", "name": "Antminer Z15 Pro"}
        ]));
    }
    if entity.is_some() {
        webpage.insert("mainEntity".into(), json!({"@id": format!("{canonical}#entity")}));
    }
    graph.push(Value::Object(webpage));
    if let Some(entity) = entity {
        graph.push(entity);
    }
    if let Some(crumbs) = breadcrumbs(p, canonical) {
        graph.push(crumbs);
    }

    serde_json::to_string(&json!({"@context": "https://schema.org", "@graph": graph}))
        .unwrap_or_else(|_| "{}".into())
        .replace('<', "\\u003c")
        .replace('>', "\\u003e")
        .replace('&', "\\u0026")
}

fn structured_entity(d: &Data, p: &Page<'_>, canonical: &str) -> Option<serde_json::Value> {
    use serde_json::json;
    let id = format!("{canonical}#entity");
    if p.path == "/sources" {
        let datasets = [
            ("Equihash pool records", "Sourced pool hashrate, fee, payout and region records.", "/data/pools.json"),
            ("Equihash network records", "Equihash parameters, network estimates, rewards and observation times.", "/data/network.json"),
            ("Equihash hardware records", "Manufacturer ASIC specifications and parameter compatibility.", "/data/miners.json"),
            ("Equihash vendor records", "Public seller identity evidence and observed sales channels.", "/data/vendors.json"),
            ("Equihash hashpower market snapshot", "Aggregate EQUIHASH order-book observations.", "/data/hashpower.json"),
        ]
        .into_iter()
        .map(|(name, description, path)| json!({
            "@type": "Dataset", "name": name, "description": description,
            "distribution": {"@type": "DataDownload", "encodingFormat": "application/json", "contentUrl": format!("{SITE}{path}")}
        }))
        .collect::<Vec<_>>();
        return Some(json!({
            "@type": "DataCatalog", "@id": id, "name": "equihash.com public mining data",
            "url": canonical, "description": p.description, "dataset": datasets
        }));
    }
    if let Some(coin_id) = p.path.strip_prefix("/coin/") {
        let c = d.coin(coin_id)?;
        return Some(json!({
            "@type": "Thing", "@id": id, "name": c.name, "alternateName": c.symbol,
            "identifier": c.id, "url": canonical,
            "description": format!("{} ({}) is an Equihash {} network with sourced mining pool and network data.", c.name, c.symbol, c.params())
        }));
    }
    if let Some(slug) = p.path.strip_prefix("/pool/") {
        let pool = d.pools.iter().find(|x| x.slug == slug)?;
        return Some(json!({
            "@type": "Organization", "@id": id, "name": pool.name,
            "url": pool.url.as_deref().unwrap_or(canonical),
            "description": format!("{} mining pool record for {} with sourced fee, payout and hashrate data.", pool.name, pool.coin_label),
            "subjectOf": {"@id": format!("{canonical}#webpage")}
        }));
    }
    if let Some(miner_id) = p.path.strip_prefix("/hardware/") {
        let miner = d.miners.iter().find(|x| x.id == miner_id)?;
        return Some(json!({
            "@type": "Product", "@id": id, "name": format!("{} {}", miner.maker, miner.model),
            "model": miner.model, "brand": {"@type": "Brand", "name": miner.maker},
            "category": "Equihash ASIC miner", "url": canonical,
            "description": format!("{} {} specifications for Equihash {}.", miner.maker, miner.model, miner.equihash),
            "additionalProperty": [
                {"@type": "PropertyValue", "name": "Equihash parameters", "value": miner.equihash},
                {"@type": "PropertyValue", "name": "Hashrate", "value": miner.hashrate_ksol, "unitText": "kSol/s"},
                {"@type": "PropertyValue", "name": "Power", "value": miner.watts, "unitText": "W"}
            ]
        }));
    }
    if let Some(slug) = p.path.strip_prefix("/buy/vendor/") {
        let vendor = d.vendors.iter().find(|x| x.slug == slug)?;
        return Some(json!({
            "@type": "Organization", "@id": id, "name": vendor.name,
            "url": vendor.url.as_deref().unwrap_or(canonical),
            "description": p.description,
            "subjectOf": {"@id": format!("{canonical}#webpage")}
        }));
    }
    None
}

fn breadcrumbs(p: &Page<'_>, canonical: &str) -> Option<serde_json::Value> {
    use serde_json::json;
    if matches!(p.path, "/" | "/search" | "/404") {
        return None;
    }
    let section = if p.path.starts_with("/coin/") {
        Some(("Coins", "/coins"))
    } else if p.path.starts_with("/pool/") {
        Some(("Pools", "/pools"))
    } else if p.path.starts_with("/hardware/") {
        Some(("Hardware", "/hardware"))
    } else if p.path.starts_with("/buy/vendor/") {
        Some(("Buy", "/buy"))
    } else if matches!(p.path, "/zcash-mining" | "/merged-mining") {
        Some(("Guides", "/guides"))
    } else {
        None
    };
    let mut items = vec![json!({"@type": "ListItem", "position": 1, "name": "equihash.com", "item": SITE})];
    if let Some((name, href)) = section {
        items.push(json!({"@type": "ListItem", "position": 2, "name": name, "item": format!("{SITE}{href}")}));
    }
    items.push(json!({
        "@type": "ListItem", "position": items.len() + 1,
        "name": p.title.trim_end_matches(" · equihash.com"), "item": canonical
    }));
    Some(json!({"@type": "BreadcrumbList", "@id": format!("{canonical}#breadcrumbs"), "itemListElement": items}))
}

/// An outbound link. Only http(s) URLs become links (data::safe_url); anything else is shown as
/// plain text, so a bad upstream value can never become a javascript: or data: link.
pub fn ext(url: &str, label: &str) -> Markup {
    match crate::data::safe_url(url) {
        Some(u) => html! { a href=(u) rel="noopener" target="_blank" { (label) } },
        None => html! { span class="na" { (label) } },
    }
}
