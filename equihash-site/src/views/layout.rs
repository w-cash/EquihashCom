use crate::data::Data;
use maud::{html, Markup, PreEscaped, DOCTYPE};

pub const SITE: &str = "https://equihash.com";
/// Bump when static CSS or JavaScript changes so browsers don't keep a stale copy.
pub const ASSET_V: &str = "63";

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

fn pool_has_indexable_evidence(pool: &crate::data::Pool) -> bool {
    let evidence = [
        pool.hashrate.filter(|value| *value > 0.0).is_some(),
        pool.fee_range().is_some(),
        !pool.payout_schemes.is_empty(),
        pool.region
            .as_deref()
            .map(str::trim)
            .is_some_and(|value| !value.is_empty()),
        pool.min_payout.is_some(),
        pool.blocks_last_1000.is_some(),
        pool.url.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count();
    pool.active && pool.source_url.is_some() && evidence >= 2
}

pub fn indexable_pool(d: &Data, pool: &crate::data::Pool) -> bool {
    if !pool_has_indexable_evidence(pool) {
        return false;
    }
    // When an upstream row and a curated row describe the same operator product, keep the richer
    // stable record indexable and leave the duplicate accessible for evidence/debugging.
    d.pools
        .iter()
        .filter(|other| {
            pool_has_indexable_evidence(other)
                && other.coin_id == pool.coin_id
                && other.name == pool.name
                && other.url == pool.url
                && other.payout_schemes == pool.payout_schemes
        })
        .max_by(|a, b| a.id.cmp(&b.id))
        .map(|canonical| canonical.id == pool.id)
        .unwrap_or(true)
}

pub fn indexable_vendor(vendor: &crate::data::Vendor) -> bool {
    vendor.channel.as_deref() != Some("warning_record")
        && vendor.url.is_some()
        && (vendor.research_id.is_some() || vendor.source_url.is_some())
}

/// Main navigation follows the four things people come here to find. Calculator stays visible as
/// the primary working tools; About, Contribute, Sources and the archive live in the footer.
const NAV: &[(&str, &str, &str)] = &[
    ("coins", "/coins", "Coins"),
    ("pools", "/pools", "Pools"),
    ("merged-mining", "/merged-mining", "Merged mining"),
    ("hashpower", "/hashpower", "Hashpower"),
    ("industry", "/industry", "Industry"),
    ("asics", "/asics", "ASICs"),
    ("vendors", "/vendors", "Vendors"),
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
    let thin_record = p
        .path
        .strip_prefix("/pool/")
        .and_then(|slug| d.pools.iter().find(|pool| pool.slug == slug))
        .map(|pool| !indexable_pool(d, pool))
        .or_else(|| {
            p.path
                .strip_prefix("/vendors/")
                .and_then(|slug| d.vendors.iter().find(|vendor| vendor.slug == slug))
                .map(|vendor| !indexable_vendor(vendor))
        })
        .unwrap_or(false);
    let robots = if thin_record
        || matches!(
            p.path,
            "/search" | "/404" | "/contribute" | "/add-vendor" | "/add-pool"
        ) {
        "noindex, follow"
    } else {
        "index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1"
    };
    let og_type = if matches!(
        p.path,
        "/zcash-mining" | "/merged-mining" | "/guides/zcash-mining-privacy"
    ) || p.path.starts_with("/industry/")
    {
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
                meta name="author" content="equihash.com";
                link rel="canonical" href=(canonical);
                link rel="alternate" type="text/plain" href="/llms.txt" title="AI-readable site guide";
                @for (href, title) in data_alternates(p.path) {
                    link rel="alternate" type="application/json" href=(href) title=(title);
                }
                meta name="theme-color" content="#11130f";
                meta name="color-scheme" content="light";
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
                link rel="preload" href="/static/fonts/ibm-plex-sans-latin-600-normal.woff2" as="font" type="font/woff2" crossorigin;
                link rel="preload" href="/static/fonts/ibm-plex-mono-latin-400-normal.woff2" as="font" type="font/woff2" crossorigin;
                link rel="stylesheet" href={"/static/app.css?v=" (ASSET_V)};
                link rel="stylesheet" href={"/static/system.css?v=" (ASSET_V)};
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
                                a class=[matches!(*key, "coins" | "pools" | "merged-mining" | "calculator").then_some("nav-primary")] href=(href) aria-current=[(*key == p.nav).then_some("page")] { (label) }
                            }
                        }
                        details class="nav-more" {
                            summary { "Menu" }
                            nav aria-label="More destinations" {
                                a href="/search" { "Search" }
                                @for (key, href, label) in NAV {
                                    @if !matches!(*key, "coins" | "pools" | "merged-mining" | "calculator") {
                                        a href=(href) aria-current=[(*key == p.nav).then_some("page")] { (label) }
                                    }
                                }
                            }
                        }
                    }
                }
                main id="main" { (body) }
                footer class="foot" {
                    div class="wrap foot-inner" {
                        p class="foot-owner" {
                            strong { "equihash.com" }
                            " · 2026 · "
                            a href="mailto:mykyta@equihash.com" { "mykyta@equihash.com" }
                            " · "
                            a href="https://x.com/EquihashCom" rel="me noopener" { "X" }
                        }
                        nav class="foot-links" aria-label="Footer" {
                            a href="/contribute" { "Corrections" }
                            a href="/sources" { "Sources" }
                            a href="/about" { "About" }
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
    if path == "/coin/zcash" {
        vec![
            ("/data/network.json", "Equihash network data"),
            ("/data/market-history.json", "Zcash network and Z15 history"),
        ]
    } else if path == "/" || path == "/coins" || path.starts_with("/coin/") {
        vec![("/data/network.json", "Equihash network data")]
    } else if path.starts_with("/pools") || path.starts_with("/pool/") || path == "/zcash-mining" {
        vec![
            ("/data/pools.json", "Equihash pool data"),
            ("/data/network.json", "Equihash network data"),
        ]
    } else if path == "/asics" || path.starts_with("/asics/") {
        vec![
            ("/data/miners.json", "Equihash ASIC data"),
            ("/data/market-history.json", "Zcash and Z15 market history"),
        ]
    } else if path == "/vendors" || path.starts_with("/vendors/") {
        vec![
            ("/data/vendors.json", "ASIC vendor data"),
            ("/data/vendor-directory.json", "Global ASIC vendor research"),
            ("/data/listings.json", "ASIC offer data"),
        ]
    } else if path == "/hashpower" {
        vec![("/data/hashpower.json", "Equihash hashpower market data")]
    } else if path == "/industry/cypherpunk-zcash-mining" {
        vec![
            (
                "/data/cypherpunk-zcash.json",
                "Cypherpunk Zcash mining research data",
            ),
            ("/data/network.json", "Equihash network data"),
        ]
    } else if path == "/industry/grayscale-zcash-etf" {
        vec![(
            "/data/grayscale-zcash.json",
            "Grayscale Zcash product research data",
        )]
    } else if path == "/industry/winklevoss-zcash-etf" {
        vec![(
            "/data/cypherpunk-zcash.json",
            "Cypherpunk and WINK filing research data",
        )]
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
        "/coins" | "/asics" | "/vendors" | "/guides" | "/archive" | "/industry" => "CollectionPage",
        "/about" => "AboutPage",
        "/calculator" => "WebApplication",
        "/zcash-mining" | "/merged-mining" | "/guides/zcash-mining-privacy" => "TechArticle",
        path if path.starts_with("/industry/") => "Article",
        path if path.starts_with("/pools") => "CollectionPage",
        path if path.starts_with("/coin/")
            || path.starts_with("/pool/")
            || path.starts_with("/asics/")
            || path.starts_with("/vendors/") =>
        {
            "ItemPage"
        }
        _ => "WebPage",
    };

    let mut graph = vec![json!({
        "@type": "Organization",
        "@id": publisher_id,
        "name": "equihash.com",
        "url": "https://equihash.com/about",
        "sameAs": ["https://x.com/EquihashCom"]
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
    if matches!(
        p.path,
        "/zcash-mining" | "/merged-mining" | "/guides/zcash-mining-privacy"
    ) {
        webpage.insert("headline".into(), json!(p.title));
        webpage.insert("author".into(), json!({"@id": publisher_id}));
        let editorial_date = if p.path == "/guides/zcash-mining-privacy" {
            "2026-10-08"
        } else {
            "2026-10-07"
        };
        webpage.insert("datePublished".into(), json!(editorial_date));
        webpage.insert("dateModified".into(), json!(editorial_date));
        webpage.insert(
            "about".into(),
            json!([
                {"@type": "Thing", "name": "Zcash mining"},
                {"@type": "Thing", "name": "Equihash"},
                {"@type": "Thing", "name": "Antminer Z15 Pro"}
            ]),
        );
        if p.path == "/guides/zcash-mining-privacy" {
            webpage.insert("citation".into(), json!([
                "https://zcash.readthedocs.io/en/latest/rtd_pages/addresses.html",
                "https://zcash.readthedocs.io/en/latest/rtd_pages/zcash_mining_guide.html",
                "https://zcash.readthedocs.io/en/latest/rtd_pages/privacy_recommendations_best_practices.html"
            ]));
        }
    } else if p.path.starts_with("/industry/") {
        webpage.insert("headline".into(), json!(p.title));
        webpage.insert("author".into(), json!({"@id": publisher_id}));
        webpage.insert("datePublished".into(), json!("2026-10-07"));
        webpage.insert("dateModified".into(), json!("2026-10-07"));
        webpage.insert("articleSection".into(), json!("Equihash industry"));
        webpage.insert(
            "about".into(),
            if p.path == "/industry/cypherpunk-zcash-mining" {
                json!([
                    {"@type": "Organization", "name": "Cypherpunk Technologies Inc."},
                    {"@type": "Thing", "name": "Zcash mining"},
                    {"@type": "Thing", "name": "Antminer Z15 Pro"},
                    {"@type": "Thing", "name": "Winklevoss Zcash ETF"}
                ])
            } else if p.path == "/industry/grayscale-zcash-etf" {
                json!([
                    {"@type": "Organization", "name": "Grayscale"},
                    {"@type": "FinancialProduct", "name": "The Zcash ETF", "tickerSymbol": "ZCSH"},
                    {"@type": "Thing", "name": "Zcash"}
                ])
            } else {
                json!([
                    {"@type": "Organization", "name": "Winklevoss Asset Services, LLC"},
                    {"@type": "FinancialProduct", "name": "Winklevoss Zcash ETF", "tickerSymbol": "WINK"},
                    {"@type": "Thing", "name": "Zcash"}
                ])
            },
        );
        let citations = if p.path == "/industry/grayscale-zcash-etf" {
            d.grayscale
                .sources
                .iter()
                .map(|s| s.url.as_str())
                .collect::<Vec<_>>()
        } else if p.path == "/industry/winklevoss-zcash-etf" {
            d.cypherpunk
                .sources
                .iter()
                .filter(|s| s.id == "wink-etf-s1")
                .map(|s| s.url.as_str())
                .collect::<Vec<_>>()
        } else {
            d.cypherpunk
                .sources
                .iter()
                .map(|s| s.url.as_str())
                .collect::<Vec<_>>()
        };
        webpage.insert("citation".into(), json!(citations));
    }
    if entity.is_some() {
        webpage.insert(
            "mainEntity".into(),
            json!({"@id": format!("{canonical}#entity")}),
        );
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
    if p.path == "/asics" {
        let mut miners: Vec<_> = d.miners.iter().collect();
        miners.sort_by(|a, b| {
            b.hashrate_ksol
                .unwrap_or_default()
                .total_cmp(&a.hashrate_ksol.unwrap_or_default())
        });
        let items = miners
            .iter()
            .enumerate()
            .map(|(position, miner)| {
                json!({
                    "@type": "ListItem",
                    "position": position + 1,
                    "url": format!("{SITE}/asics/{}", miner.id),
                    "name": format!("{} {}", miner.maker, miner.model)
                })
            })
            .collect::<Vec<_>>();
        return Some(json!({
            "@type": "ItemList", "@id": id, "name": "Equihash ASIC miners",
            "url": canonical, "numberOfItems": items.len(), "itemListOrder": "https://schema.org/ItemListOrderDescending",
            "itemListElement": items
        }));
    }
    if p.path == "/pools" {
        let mut pools: Vec<_> = d
            .live_pools()
            .filter(|pool| !pool.is_hashpower_marketplace() && indexable_pool(d, pool))
            .collect();
        pools.sort_by(|a, b| {
            b.hashrate
                .unwrap_or_default()
                .total_cmp(&a.hashrate.unwrap_or_default())
        });
        let items = pools
            .iter()
            .enumerate()
            .map(|(position, pool)| {
                json!({
                    "@type": "ListItem", "position": position + 1,
                    "url": format!("{SITE}/pool/{}", pool.slug),
                    "name": format!("{} {} mining pool", pool.name, pool.coin_label)
                })
            })
            .collect::<Vec<_>>();
        return Some(json!({
            "@type": "ItemList", "@id": id, "name": "Equihash mining pools",
            "url": canonical, "numberOfItems": items.len(), "itemListElement": items
        }));
    }
    if p.path == "/vendors" {
        let items = d
            .vendors
            .iter()
            .filter(|vendor| indexable_vendor(vendor))
            .enumerate()
            .map(|(position, vendor)| {
                json!({
                    "@type": "ListItem", "position": position + 1,
                    "url": format!("{SITE}/vendors/{}", vendor.slug), "name": vendor.name
                })
            })
            .collect::<Vec<_>>();
        return Some(json!({
            "@type": "ItemList", "@id": id, "name": "ASIC vendor directory",
            "url": canonical, "numberOfItems": items.len(), "itemListElement": items
        }));
    }
    if p.path == "/industry" {
        return Some(json!({
            "@type": "ItemList", "@id": id, "name": "Equihash industry briefings",
            "url": canonical, "numberOfItems": 3,
            "itemListElement": [
                {"@type": "ListItem", "position": 1, "url": format!("{SITE}/industry/cypherpunk-zcash-mining"), "name": "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet"},
                {"@type": "ListItem", "position": 2, "url": format!("{SITE}/industry/winklevoss-zcash-etf"), "name": "WINK filed: what the preliminary Zcash ETF prospectus says"},
                {"@type": "ListItem", "position": 3, "url": format!("{SITE}/industry/grayscale-zcash-etf"), "name": "From OTC trust to ZCSH: Grayscale’s Zcash vehicle changed shape"}
            ]
        }));
    }
    if p.path == "/sources" {
        let datasets = [
            ("Equihash pool records", "Sourced pool hashrate, fee, payout and region records.", "/data/pools.json"),
            ("Equihash network records", "Equihash parameters, network estimates, rewards and observation times.", "/data/network.json"),
            ("Equihash ASIC records", "Manufacturer and clearly labelled market-reported ASIC specifications with parameter compatibility.", "/data/miners.json"),
            ("Equihash vendor records", "Public seller identity evidence and observed sales channels.", "/data/vendors.json"),
            ("Equihash hashpower market snapshot", "Aggregate EQUIHASH order-book observations.", "/data/hashpower.json"),
            ("Zcash and Z15 market history", "Retained network, difficulty, price, machine-economics and seller observations without interpolated gaps.", "/data/market-history.json"),
            ("Cypherpunk Zcash mining record", "Company-reported fleet figures and primary filing sources.", "/data/cypherpunk-zcash.json"),
            ("Grayscale Zcash product record", "Filed product status, structure, dates and primary sources.", "/data/grayscale-zcash.json"),
        ]
        .into_iter()
        .map(|(name, description, path)| json!({
            "@type": "Dataset", "name": name, "description": description,
            "dateModified": d.last_updated,
            "measurementTechnique": "Public API, operator page or reviewed primary-source observation; field-level sources and timestamps are retained in the distribution.",
            "variableMeasured": ["hashrate", "difficulty", "fee", "payout terms", "price", "availability"],
            "license": format!("{SITE}/sources"),
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
    if let Some(miner_id) = p.path.strip_prefix("/asics/") {
        let miner = d.miners.iter().find(|x| x.id == miner_id)?;
        let mut product = json!({
            "@type": "Product", "@id": id, "name": format!("{} {}", miner.maker, miner.model),
            "model": miner.model, "brand": {"@type": "Brand", "name": miner.maker},
            "category": "Equihash ASIC miner", "url": canonical,
            "description": format!("{} {} specifications for Equihash {}.", miner.maker, miner.model, miner.equihash),
            "additionalProperty": [
                {"@type": "PropertyValue", "name": "Equihash parameters", "value": miner.equihash},
                {"@type": "PropertyValue", "name": "Hashrate", "value": miner.hashrate_ksol, "unitText": "kSol/s"},
                {"@type": "PropertyValue", "name": "Power", "value": miner.watts, "unitText": "W"}
            ]
        });
        if let Some(image) = miner
            .image
            .as_deref()
            .filter(|image| image.starts_with("/static/shop/machines/"))
        {
            product["image"] = json!(format!("{SITE}{image}"));
        }
        let offers = d.listings.iter().filter(|offer| {
            offer.miner_id == miner.id
                && offer.price_amount.is_some()
                && offer.price_currency.is_some()
                && matches!(offer.availability.as_deref(), Some("in_stock" | "dispatch_claim" | "preorder"))
        }).map(|offer| {
            let availability = match offer.availability.as_deref() {
                Some("in_stock" | "dispatch_claim") => "https://schema.org/InStock",
                Some("preorder") => "https://schema.org/PreOrder",
                _ => "https://schema.org/OutOfStock",
            };
            json!({
                "@type": "Offer", "url": offer.product_url.as_deref().or(offer.source_url.as_deref()),
                "price": offer.price_amount, "priceCurrency": offer.price_currency,
                "availability": availability,
                "description": offer.availability_label,
                "validFrom": offer.price_observed_at.as_deref().or(offer.observed_at.as_deref())
            })
        }).collect::<Vec<_>>();
        if !offers.is_empty() {
            product["offers"] = json!(offers);
        }
        return Some(product);
    }
    if let Some(slug) = p.path.strip_prefix("/vendors/") {
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
    } else if p.path.starts_with("/asics/") {
        Some(("ASICs", "/asics"))
    } else if p.path.starts_with("/vendors/") {
        Some(("Vendors", "/vendors"))
    } else if p.path.starts_with("/industry/") {
        Some(("Industry", "/industry"))
    } else if matches!(p.path, "/zcash-mining" | "/merged-mining") {
        Some(("Guides", "/guides"))
    } else {
        None
    };
    let mut items =
        vec![json!({"@type": "ListItem", "position": 1, "name": "equihash.com", "item": SITE})];
    if let Some((name, href)) = section {
        items.push(json!({"@type": "ListItem", "position": 2, "name": name, "item": format!("{SITE}{href}")}));
    }
    items.push(json!({
        "@type": "ListItem", "position": items.len() + 1,
        "name": p.title.trim_end_matches(" · equihash.com"), "item": canonical
    }));
    Some(
        json!({"@type": "BreadcrumbList", "@id": format!("{canonical}#breadcrumbs"), "itemListElement": items}),
    )
}

/// An outbound link. Only http(s) URLs become links (data::safe_url); anything else is shown as
/// plain text, so a bad upstream value can never become a javascript: or data: link.
pub fn ext(url: &str, label: &str) -> Markup {
    match crate::data::safe_url(url) {
        Some(u) => html! { a href=(u) rel="noopener" target="_blank" { (label) } },
        None => html! { span class="na" { (label) } },
    }
}
