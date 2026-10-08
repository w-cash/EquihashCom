//! equihash.com: a small Actix Web server that renders the pool directory from data/*.json.
//!
//! Run:  cargo run --release            (serves http://127.0.0.1:8080, PORT/HOST/DATA_DIR/STATIC_DIR override)
//! Data: npm run refresh                (publishes a new snapshot; the server reloads it automatically)

mod data;
mod fmt;
mod live;
mod views;

use actix_files::{Files, NamedFile};
use actix_web::{
    guard, http::header, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

struct AppState {
    data: RwLock<Arc<data::Data>>,
    data_dir: PathBuf,
    /// Last good reading of every live source (see src/live.rs).
    live: RwLock<live::LiveState>,
    live_enabled: bool,
    /// Why the newest files on disk don't load (the previous data keeps serving), if they don't.
    reload_error: RwLock<Option<String>>,
}

impl AppState {
    fn new(d: data::Data, data_dir: PathBuf, live_enabled: bool) -> Self {
        AppState {
            data: RwLock::new(Arc::new(d)),
            data_dir,
            live: RwLock::new(live::LiveState::default()),
            live_enabled,
            reload_error: RwLock::new(None),
        }
    }
    fn get(&self) -> Arc<data::Data> {
        self.data.read().unwrap().clone()
    }
    /// Rebuild the served data from the files plus the latest live readings.
    fn rebuild(&self) -> Result<(), String> {
        let l = self.live.read().unwrap().clone();
        let d = data::load_with_live(&self.data_dir, Some(&l))?;
        *self.data.write().unwrap() = Arc::new(d);
        Ok(())
    }
}

fn html(m: maud::Markup) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "public, max-age=60"))
        .body(m.into_string())
}

// Every page route answers GET and HEAD (HEAD gets the same status and headers, no body).

async fn home(s: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    // Preserve old shared/indexed pool-filter URLs while making `/` the discovery homepage.
    let legacy_pool_query = req
        .query_string()
        .split('&')
        .filter_map(|part| part.split_once('=').map(|x| x.0))
        .any(|key| {
            matches!(
                key,
                "coin"
                    | "scheme"
                    | "region"
                    | "q"
                    | "fee"
                    | "hr"
                    | "merged"
                    | "hide_empty"
                    | "sort"
                    | "dir"
            )
        });
    if legacy_pool_query {
        return HttpResponse::MovedPermanently()
            .insert_header((header::LOCATION, format!("/pools?{}", req.query_string())))
            .finish();
    }
    html(views::hub::index(&s.get()))
}

async fn pools(s: web::Data<AppState>, q: web::Query<views::home::Filters>) -> impl Responder {
    html(views::home::render(&s.get(), &q))
}

async fn coins(s: web::Data<AppState>) -> impl Responder {
    html(views::hub::coins(&s.get()))
}

async fn coin(s: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let d = s.get();
    match d.coin(path.as_str()) {
        Some(c) => html(views::hub::coin(&d, c)),
        None => HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(views::pages::not_found(&d).into_string()),
    }
}

/// Pool pages live at their permanent slug (data/curated/permalinks.json). An old URL (the slug
/// once computed from coin, name and domain, or a curated alias) redirects there with a 301.
async fn pool(s: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let d = s.get();
    if let Some(p) = d.pools.iter().find(|p| p.slug == *path) {
        if p.is_hashpower_marketplace() {
            return HttpResponse::MovedPermanently()
                .insert_header((header::LOCATION, "/hashpower"))
                .finish();
        }
        return html(views::pages::pool_page(&d, p));
    }
    if let Some(to) = d.slug_redirects.get(path.as_str()) {
        return HttpResponse::MovedPermanently()
            .insert_header((header::LOCATION, format!("/pool/{to}")))
            .finish();
    }
    HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(views::pages::not_found(&d).into_string())
}

async fn archive(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::archive(&s.get()))
}
async fn miners(s: web::Data<AppState>) -> impl Responder {
    html(views::asics::index(&s.get()))
}
async fn miners_legacy() -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "/asics"))
        .finish()
}
async fn hardware_detail(s: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let d = s.get();
    match d.miners.iter().find(|m| m.id == *path) {
        Some(m) => html(views::asics::detail(&d, m)),
        None => HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(views::pages::not_found(&d).into_string()),
    }
}
async fn hardware_legacy() -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "/asics"))
        .finish()
}
async fn hardware_detail_legacy(path: web::Path<String>) -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, format!("/asics/{path}")))
        .finish()
}
async fn calculator(
    s: web::Data<AppState>,
    q: web::Query<views::calc::CalcQuery>,
) -> impl Responder {
    html(views::calc::render(&s.get(), &q))
}
async fn guide(s: web::Data<AppState>) -> impl Responder {
    html(views::guide::render(&s.get()))
}
async fn zcash_mining(s: web::Data<AppState>) -> impl Responder {
    html(views::hub::zcash_mining(&s.get()))
}
async fn hashpower(
    s: web::Data<AppState>,
    q: web::Query<views::hashpower::HashpowerQuery>,
) -> impl Responder {
    html(views::hashpower::render(&s.get(), &q))
}
async fn add_pool_legacy() -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, "/contribute#pool"))
        .finish()
}
async fn about(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::about(&s.get()))
}
async fn sources(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::sources(&s.get()))
}
async fn guides(s: web::Data<AppState>) -> impl Responder {
    html(views::hub::guides(&s.get()))
}
async fn industry(s: web::Data<AppState>) -> impl Responder {
    html(views::research::index(&s.get()))
}
async fn cypherpunk_research(s: web::Data<AppState>) -> impl Responder {
    html(views::research::cypherpunk(&s.get()))
}
async fn wink_research(s: web::Data<AppState>) -> impl Responder {
    html(views::research::wink(&s.get()))
}
async fn grayscale_research(s: web::Data<AppState>) -> impl Responder {
    html(views::research::grayscale(&s.get()))
}
async fn research_legacy() -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, views::research::CYPHERPUNK_PATH))
        .finish()
}
async fn search(s: web::Data<AppState>, q: web::Query<views::hub::SearchQuery>) -> impl Responder {
    html(views::hub::search(&s.get(), &q))
}
async fn contribute(s: web::Data<AppState>) -> impl Responder {
    html(views::hub::contribute(&s.get()))
}
async fn vendors(s: web::Data<AppState>, q: web::Query<views::buy::BuyQuery>) -> impl Responder {
    html(views::buy::index(&s.get(), &q))
}
async fn vendor(s: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let d = s.get();
    match d.vendors.iter().find(|v| v.slug == *path) {
        Some(v) => html(views::buy::vendor_page(&d, v)),
        None => HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(views::pages::not_found(&d).into_string()),
    }
}

async fn buy_legacy(req: HttpRequest) -> HttpResponse {
    let to = if req.query_string().is_empty() {
        "/vendors".to_string()
    } else {
        format!("/vendors?{}", req.query_string())
    };
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, to))
        .finish()
}

async fn buy_vendor_legacy(path: web::Path<String>) -> HttpResponse {
    HttpResponse::MovedPermanently()
        .insert_header((header::LOCATION, format!("/vendors/{}", path.into_inner())))
        .finish()
}
async fn add_vendor(s: web::Data<AppState>) -> impl Responder {
    html(views::buy::add_vendor(&s.get()))
}

/// Live figures for the page to poll (every 60 s). Read from the server's own last good readings,
/// so browsers never call the pool's endpoints.
async fn api_live(s: web::Data<AppState>) -> HttpResponse {
    let d = s.get();
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "public, max-age=15"))
        .insert_header(("X-Robots-Tag", "noindex"))
        .json(views::live_json(&d, chrono::Utc::now()))
}

/// Default for `HEALTH_MAX_DATA_AGE_SECS`: three missed hourly refreshes.
const HEALTH_MAX_DATA_AGE_SECS: i64 = 3 * 3600;

/// Body and status of /healthz. 503 when the data is older than `max_age` (or its age is unknown);
/// "degraded" (still 200) when a reload is failing or a live source is down or stale.
fn health(
    d: &data::Data,
    now: chrono::DateTime<chrono::Utc>,
    max_age: i64,
    reload_error: Option<&str>,
    live_enabled: bool,
) -> (u16, serde_json::Value) {
    let generated = d.meta.generated_at.clone().or(d.last_updated.clone());
    let age = data::age_secs(generated.as_deref(), now);
    let data_ok = age.map(|a| a <= max_age).unwrap_or(false);
    let live_sources: Vec<serde_json::Value> = d
        .live
        .iter()
        .map(|s| serde_json::json!({"id": s.id, "status": s.status, "age_secs": s.age_secs, "stale": s.stale, "error": s.error.as_ref().map(|_| "source unavailable"), "last_ok_at": s.last_ok_at}))
        .collect();
    let live_ok = !live_enabled || d.live.iter().all(|s| s.status == "ok" && !s.stale);
    let status = if !data_ok {
        "error"
    } else if reload_error.is_some() || !live_ok {
        "degraded"
    } else {
        "ok"
    };
    let body = serde_json::json!({
        "status": status,
        "checked_at": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "data": {
            "snapshot": d.snapshot,
            "published_at": d.snapshot_published_at,
            "generated_at": generated,
            "age_secs": age,
            "max_age_secs": max_age,
            "fresh": data_ok,
            "pools": d.pools.len(),
            "coins": d.coins.len(),
            "reload_error": reload_error.map(|_| "data reload failed"),
        },
        "live": { "enabled": live_enabled, "ok": live_ok, "sources": live_sources },
    });
    (if data_ok { 200 } else { 503 }, body)
}

async fn healthz(s: web::Data<AppState>) -> HttpResponse {
    let max_age = std::env::var("HEALTH_MAX_DATA_AGE_SECS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(HEALTH_MAX_DATA_AGE_SECS);
    let err = s.reload_error.read().unwrap().clone();
    let (code, body) = health(
        &s.get(),
        chrono::Utc::now(),
        max_age,
        err.as_deref(),
        s.live_enabled,
    );
    HttpResponse::build(actix_web::http::StatusCode::from_u16(code).unwrap())
        .insert_header((header::CACHE_CONTROL, "no-store"))
        .insert_header(("X-Robots-Tag", "noindex"))
        .json(body)
}

/// Public, read-only copies of the data files (transparency). The generated files are served
/// from the same snapshot the pages were rendered from.
async fn data_file(
    s: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest,
) -> HttpResponse {
    const ALLOWED: &[&str] = &[
        "pools.json",
        "network.json",
        "archive.json",
        "miners.json",
        "vendors.json",
        "vendor-directory.json",
        "listings.json",
        "hashpower.json",
        "meta.json",
        "research.json",
        "cypherpunk-zcash.json",
        "grayscale-zcash.json",
        "current.json",
    ];
    if !ALLOWED.contains(&path.as_str()) {
        return HttpResponse::NotFound().finish();
    }
    let d = s.get();
    let file = if data::GENERATED.contains(&path.as_str()) {
        d.snapshot_dir.join(path.as_str())
    } else {
        s.data_dir.join(path.as_str())
    };
    match NamedFile::open(file) {
        Ok(f) => {
            let mut response = f.into_response(&req);
            response.headers_mut().insert(
                actix_web::http::header::HeaderName::from_static("x-robots-tag"),
                actix_web::http::header::HeaderValue::from_static("noindex"),
            );
            response
        }
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

async fn favicon(req: HttpRequest) -> HttpResponse {
    match NamedFile::open(static_dir().join("favicon.ico")) {
        Ok(f) => f.into_response(&req),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

async fn robots() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/plain; charset=utf-8")
        .body(format!(
            "User-agent: OAI-SearchBot\nAllow: /\n\nUser-agent: ChatGPT-User\nAllow: /\n\nUser-agent: *\nAllow: /\n\nSitemap: {}/sitemap.xml\n",
            views::layout::SITE
        ))
}

async fn llms(s: web::Data<AppState>) -> HttpResponse {
    let d = s.get();
    let updated = d.last_updated.as_deref().unwrap_or("unknown");
    let site = views::layout::SITE;
    let body = format!(
        r#"# equihash.com

> Source-backed Equihash mining directory for coins, pools, ASIC hardware, seller evidence, calculators and technical guides.

Updated: {updated}

## Start here

- [Zcash mining guide]({site}/zcash-mining): Equihash 200,9 hardware, pool selection, setup, costs, privacy and merged mining.
- [Zcash mining record]({site}/coin/zcash): current network, compatible miners, listed pools and sources.
- [Zcash pool comparison]({site}/pools): reported hashrate, fees, payout methods, minimum payouts, regions and source ages.
- [Equihash ASIC index]({site}/asics): miners ranked by hashrate with power, efficiency, current economics and seller offers.
- [Antminer Z15 Pro]({site}/asics/antminer-z15-pro): manufacturer specifications, compatible coins, profitability and current offers.
- [Equihash coins]({site}/coins): networks grouped by exact n,k parameters.
- [Merged mining guide]({site}/merged-mining): Zcash parent-chain and Wcash auxiliary-chain flow.
- [Equihash industry]({site}/industry): source-backed briefings on companies, mining fleets, investment products and infrastructure around Equihash.
- [Cypherpunk Zcash mining fleet]({site}/industry/cypherpunk-zcash-mining): SEC-sourced account of the reported 4.2 GSol/s fleet.
- [Grayscale ZCSH]({site}/industry/grayscale-zcash-etf): filed timeline and structure of the NYSE Arca-traded ZEC product.
- [Winklevoss WINK filing]({site}/industry/winklevoss-zcash-etf): what the preliminary Zcash ETF filing says and does not establish.
- [Sources and method]({site}/sources): provenance, refresh method and known limits.

## Machine-readable data

- [Pools JSON]({site}/data/pools.json)
- [Networks JSON]({site}/data/network.json)
- [ASIC JSON]({site}/data/miners.json)
- [Vendors JSON]({site}/data/vendors.json)
- [Global vendor directory JSON]({site}/data/vendor-directory.json)
- [Listings JSON]({site}/data/listings.json)
- [Hashpower JSON]({site}/data/hashpower.json)
- [Cypherpunk Zcash research data]({site}/data/cypherpunk-zcash.json)
- [Grayscale Zcash product data]({site}/data/grayscale-zcash.json)

## Editorial notes

- Pool and market figures are time-stamped snapshots from public project, pool and API sources.
- Vendor records are alphabetical by default. Optional sorts disclose their single factor; paid listings and affiliate relationships cannot change the default order.
- Availability is seller-declared unless expressly stated. Third-party review profiles are linked, while copied scores and counts are not republished. Missing vendor facts remain blank rather than inferred.
- equihash.com and Wcash share a maintainer. Wcash receives work only from participating merged-mining pools.
- Mining estimates are not forecasts or financial advice.
"#
    );
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "public, max-age=300"))
        .content_type("text/plain; charset=utf-8")
        .body(body)
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn sitemap_lastmod(value: Option<&str>) -> Option<String> {
    value.and_then(|v| {
        chrono::DateTime::parse_from_rfc3339(v)
            .ok()
            .map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
            .or_else(|| {
                (v.len() == 10 && chrono::NaiveDate::parse_from_str(v, "%Y-%m-%d").is_ok())
                    .then(|| v.to_string())
            })
    })
}

async fn sitemap(s: web::Data<AppState>) -> HttpResponse {
    let d = s.get();
    let data_updated = sitemap_lastmod(d.last_updated.as_deref());
    let hardware_updated = sitemap_lastmod(d.miners_verified_at.as_deref());
    let vendor_updated = sitemap_lastmod(
        d.listings_verified_at
            .as_deref()
            .or(d.vendors_verified_at.as_deref()),
    );
    let hashpower_updated = sitemap_lastmod(d.hashpower.observed_at.as_deref());
    let mut urls: Vec<(String, Option<String>)> = vec![
        ("/".into(), data_updated.clone()),
        ("/coins".into(), data_updated.clone()),
        ("/pools".into(), data_updated.clone()),
        ("/hashpower".into(), hashpower_updated),
        ("/asics".into(), hardware_updated.clone()),
        ("/vendors".into(), vendor_updated.clone()),
        ("/guides".into(), Some("2026-10-07".into())),
        ("/zcash-mining".into(), Some("2026-10-07".into())),
        ("/calculator".into(), data_updated.clone()),
        ("/merged-mining".into(), Some("2026-10-07".into())),
        ("/industry".into(), Some("2026-10-07".into())),
        (
            "/industry/cypherpunk-zcash-mining".into(),
            Some("2026-10-07".into()),
        ),
        (
            "/industry/grayscale-zcash-etf".into(),
            Some("2026-10-07".into()),
        ),
        (
            "/industry/winklevoss-zcash-etf".into(),
            Some("2026-10-07".into()),
        ),
        ("/archive".into(), data_updated.clone()),
        ("/about".into(), Some("2026-10-07".into())),
        ("/sources".into(), Some("2026-10-07".into())),
    ];
    urls.extend(
        d.coins
            .iter()
            .map(|c| (format!("/coin/{}", c.id), data_updated.clone())),
    );
    for c in d.coins.iter().filter(|c| c.active() && c.id != "zcash") {
        if d.live_pools()
            .any(|p| p.coin_id == c.id && !p.is_hashpower_marketplace())
        {
            urls.push((format!("/pools?coin={}", c.id), data_updated.clone()));
        }
    }
    urls.extend(
        d.live_pools()
            .filter(|p| !p.is_hashpower_marketplace())
            .map(|p| {
                (
                    format!("/pool/{}", p.slug),
                    sitemap_lastmod(
                        p.hashrate_observed_at
                            .as_deref()
                            .or(p.fetched_at.as_deref())
                            .or(d.last_updated.as_deref()),
                    ),
                )
            }),
    );
    urls.extend(
        d.miners
            .iter()
            .map(|m| (format!("/asics/{}", m.id), hardware_updated.clone())),
    );
    urls.extend(d.vendors.iter().map(|v| {
        (
            format!("/vendors/{}", v.slug),
            sitemap_lastmod(v.observed_at.as_deref()).or_else(|| vendor_updated.clone()),
        )
    }));
    let body: String = urls
        .iter()
        .map(|(path, lastmod)| {
            let loc = xml_escape(&format!("{}{}", views::layout::SITE, path));
            let modified = lastmod
                .as_deref()
                .map(|v| format!("<lastmod>{}</lastmod>", xml_escape(v)))
                .unwrap_or_default();
            format!("<url><loc>{loc}</loc>{modified}</url>")
        })
        .collect();
    HttpResponse::Ok()
        .content_type("application/xml; charset=utf-8")
        .body(format!(r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{body}</urlset>"#))
}

async fn not_found(s: web::Data<AppState>) -> HttpResponse {
    HttpResponse::NotFound()
        .content_type("text/html; charset=utf-8")
        .body(views::pages::not_found(&s.get()).into_string())
}

fn static_dir() -> PathBuf {
    std::env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("static"))
}

/// Security headers on every response. HSTS is left to the TLS proxy unless HSTS=1 (the app
/// itself usually speaks plain HTTP on localhost); see deploy/.
fn security_headers() -> middleware::DefaultHeaders {
    let mut h = middleware::DefaultHeaders::new()
        .add((header::CONTENT_SECURITY_POLICY, views::layout::csp()))
        .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .add((header::REFERRER_POLICY, "strict-origin-when-cross-origin"))
        .add((header::X_FRAME_OPTIONS, "DENY"))
        .add((
            "Permissions-Policy",
            "camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()",
        ))
        .add(("Cross-Origin-Opener-Policy", "same-origin"));
    if std::env::var("HSTS").map(|v| v == "1").unwrap_or(false) {
        h = h.add((
            header::STRICT_TRANSPORT_SECURITY,
            "max-age=31536000; includeSubDomains",
        ));
    }
    h
}

fn routes(cfg: &mut web::ServiceConfig, sdir: PathBuf) {
    // GET and HEAD on every route. Registered by hand rather than with #[route(method = "GET",
    // method = "HEAD")]: that macro keeps its methods in a HashSet, so the guard order (and the
    // binary) changed from build to build.
    let get_head = || web::route().guard(guard::Any(guard::Get()).or(guard::Head()));
    cfg.service(web::resource("/").route(get_head().to(home)))
        .service(web::resource("/pools").route(get_head().to(pools)))
        .service(web::resource("/hashpower").route(get_head().to(hashpower)))
        .service(web::resource("/coins").route(get_head().to(coins)))
        .service(web::resource("/coin/{id}").route(get_head().to(coin)))
        .service(web::resource("/pool/{slug}").route(get_head().to(pool)))
        .service(web::resource("/archive").route(get_head().to(archive)))
        .service(web::resource("/miners").route(get_head().to(miners_legacy)))
        .service(web::resource("/asics").route(get_head().to(miners)))
        .service(web::resource("/asics/{id}").route(get_head().to(hardware_detail)))
        .service(web::resource("/hardware").route(get_head().to(hardware_legacy)))
        .service(web::resource("/hardware/{id}").route(get_head().to(hardware_detail_legacy)))
        .service(web::resource("/vendors").route(get_head().to(vendors)))
        .service(web::resource("/vendors/{slug}").route(get_head().to(vendor)))
        .service(web::resource("/buy").route(get_head().to(buy_legacy)))
        .service(web::resource("/buy/vendor/{slug}").route(get_head().to(buy_vendor_legacy)))
        .service(web::resource("/add-vendor").route(get_head().to(add_vendor)))
        .service(web::resource("/calculator").route(get_head().to(calculator)))
        .service(web::resource("/merged-mining").route(get_head().to(guide)))
        .service(web::resource("/zcash-mining").route(get_head().to(zcash_mining)))
        .service(web::resource("/guides").route(get_head().to(guides)))
        .service(web::resource("/industry").route(get_head().to(industry)))
        .service(
            web::resource("/industry/cypherpunk-zcash-mining")
                .route(get_head().to(cypherpunk_research)),
        )
        .service(
            web::resource("/industry/winklevoss-zcash-etf").route(get_head().to(wink_research)),
        )
        .service(
            web::resource("/industry/grayscale-zcash-etf").route(get_head().to(grayscale_research)),
        )
        .service(
            web::resource("/research/cypherpunk-zcash-mining")
                .route(get_head().to(research_legacy)),
        )
        .service(web::resource("/search").route(get_head().to(search)))
        .service(web::resource("/contribute").route(get_head().to(contribute)))
        .service(web::resource("/add-pool").route(get_head().to(add_pool_legacy)))
        .service(web::resource("/about").route(get_head().to(about)))
        .service(web::resource("/sources").route(get_head().to(sources)))
        .service(web::resource("/api/live").route(get_head().to(api_live)))
        .service(web::resource("/healthz").route(get_head().to(healthz)))
        .service(web::resource("/data/{file}").route(get_head().to(data_file)))
        .service(web::resource("/favicon.ico").route(get_head().to(favicon)))
        .service(web::resource("/robots.txt").route(get_head().to(robots)))
        .service(web::resource("/llms.txt").route(get_head().to(llms)))
        .service(web::resource("/sitemap.xml").route(get_head().to(sitemap)))
        // Logos first: long immutable cache and a CSP of their own (src/views/logo.rs).
        .service(views::logo::service(&sdir))
        .service(Files::new("/static", sdir).use_etag(true))
        .default_service(web::to(not_found));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let data_dir = std::env::var("DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("data"));
    let d = data::load(&data_dir).unwrap_or_else(|e| {
        eprintln!("error: could not load data: {e}\nRun `npm run refresh` or set DATA_DIR.");
        std::process::exit(1);
    });
    println!(
        "Loaded {} pools, {} coins, {} miners, {} archive entries from {}{}",
        d.pools.len(),
        d.coins.len(),
        d.miners.len(),
        d.archive.len(),
        data_dir.display(),
        d.snapshot
            .as_deref()
            .map(|s| format!(" (snapshot {s})"))
            .unwrap_or_default()
    );
    let live_enabled = std::env::var("LIVE").map(|v| v != "0").unwrap_or(true);
    let state = web::Data::new(AppState::new(d, data_dir.clone(), live_enabled));

    // Hot reload: poll data/current.json, data/*.json and data/curated/ every 2 s; a new snapshot
    // or any edited, added, removed or replaced JSON file triggers a reload. A broken file keeps
    // the last good data (reported on /healthz) and is retried on the next poll.
    {
        let state = state.clone();
        std::thread::spawn(move || {
            let mut watcher = data::Reloader::new(&state.data_dir);
            loop {
                std::thread::sleep(Duration::from_secs(2));
                let l = state.live.read().unwrap().clone();
                match watcher.poll_with(Some(&l)) {
                    None => {}
                    Some(Ok(d)) => {
                        println!(
                            "Reloaded data: {} pools{}",
                            d.pools.len(),
                            d.snapshot
                                .as_deref()
                                .map(|s| format!(" (snapshot {s})"))
                                .unwrap_or_default()
                        );
                        *state.data.write().unwrap() = Arc::new(d);
                        *state.reload_error.write().unwrap() = None;
                    }
                    Some(Err(e)) => {
                        let mut cur = state.reload_error.write().unwrap();
                        if cur.as_deref() != Some(e.as_str()) {
                            eprintln!("reload failed (keeping previous data): {e}");
                        }
                        *cur = Some(e);
                    }
                }
            }
        });
    }

    // Live sources: a small background task polls each endpoint in data/curated/live-sources.json
    // every 30–60 s (re-reading the config each round, so adding a source is a JSON edit). A bad
    // answer keeps the previous good reading. LIVE=0 turns polling off.
    if live_enabled {
        let state = state.clone();
        actix_web::rt::spawn(async move {
            let client = match reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .user_agent("equihash.com live poller")
                .build()
            {
                Ok(c) => c,
                Err(e) => return eprintln!("live poller disabled: {e}"),
            };
            loop {
                let cfg = live::read_config(&state.data_dir).unwrap_or_default();
                let mut changed = false;
                for src in &cfg.sources {
                    let r = live::fetch(&client, src).await;
                    if let Err(e) = &r {
                        log::warn!("live source {}: {e}", src.id);
                    }
                    changed |= state
                        .live
                        .write()
                        .unwrap()
                        .record(&src.id, r, chrono::Utc::now());
                }
                // Rebuild even when nothing changed, so status and ages in /api/live stay current.
                if let Err(e) = state.rebuild() {
                    if changed {
                        eprintln!("live rebuild failed (keeping previous data): {e}");
                    }
                }
                actix_web::rt::time::sleep(Duration::from_secs(cfg.interval())).await;
            }
        });
    }

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    println!("equihash.com serving on http://{host}:{port}");
    let sdir = static_dir();
    HttpServer::new(move || {
        let sdir = sdir.clone();
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::new("%r %s %Dms"))
            .wrap(security_headers())
            .configure(move |c| routes(c, sdir))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{http::Method, test};
    use std::path::Path;

    fn state() -> web::Data<AppState> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
        web::Data::new(AppState::new(data::load(&dir).unwrap(), dir, false))
    }

    macro_rules! app {
        ($s:expr) => {
            test::init_service(
                App::new()
                    .app_data($s.clone())
                    .wrap(security_headers())
                    .configure(|c| routes(c, Path::new(env!("CARGO_MANIFEST_DIR")).join("static"))),
            )
            .await
        };
    }

    #[actix_web::test]
    async fn every_get_route_also_answers_head() {
        let s = state();
        let app = app!(s);
        let slug = s.get().pools[0].slug.clone();
        let coin = s.get().coins[0].id.clone();
        let miner = s.get().miners[0].id.clone();
        let paths = [
            "/".to_string(),
            "/pools?coin=wcash".into(),
            "/hashpower".into(),
            "/coins".into(),
            format!("/coin/{coin}"),
            format!("/pool/{slug}"),
            "/archive".into(),
            "/asics".into(),
            format!("/asics/{miner}"),
            "/vendors".into(),
            "/vendors?machine=antminer-z15-pro&region=UK&state=in_stock".into(),
            format!("/vendors/{}", s.get().vendors[0].slug),
            "/add-vendor".into(),
            "/guides".into(),
            "/zcash-mining".into(),
            "/industry".into(),
            "/industry/cypherpunk-zcash-mining".into(),
            "/industry/grayscale-zcash-etf".into(),
            "/industry/winklevoss-zcash-etf".into(),
            "/search?q=Z15+Pro".into(),
            "/contribute".into(),
            "/calculator".into(),
            "/merged-mining".into(),
            "/about".into(),
            "/sources".into(),
            "/api/live".into(),
            "/data/pools.json".into(),
            "/data/miners.json".into(),
            "/data/vendors.json".into(),
            "/data/vendor-directory.json".into(),
            "/data/listings.json".into(),
            "/data/hashpower.json".into(),
            "/data/cypherpunk-zcash.json".into(),
            "/data/grayscale-zcash.json".into(),
            "/favicon.ico".into(),
            "/robots.txt".into(),
            "/llms.txt".into(),
            "/sitemap.xml".into(),
            "/static/app.js".into(),
        ];
        for p in &paths {
            let get = test::call_service(&app, test::TestRequest::get().uri(p).to_request()).await;
            let head = test::call_service(
                &app,
                test::TestRequest::default()
                    .method(Method::HEAD)
                    .uri(p)
                    .to_request(),
            )
            .await;
            assert_eq!(get.status(), 200, "GET {p}");
            assert_eq!(head.status(), get.status(), "HEAD {p}");
            assert_eq!(
                head.headers().get(header::CONTENT_TYPE),
                get.headers().get(header::CONTENT_TYPE),
                "HEAD {p} content type"
            );
        }
        for m in [Method::GET, Method::HEAD] {
            let r = test::call_service(
                &app,
                test::TestRequest::default()
                    .method(m)
                    .uri("/?coin=wcash")
                    .to_request(),
            )
            .await;
            assert_eq!(r.status(), 301);
            assert_eq!(
                r.headers().get(header::LOCATION).unwrap(),
                "/pools?coin=wcash"
            );
        }
        for (from, to) in [
            ("/miners", "/asics"),
            ("/hardware", "/asics"),
            ("/add-pool", "/contribute#pool"),
        ] {
            for m in [Method::GET, Method::HEAD] {
                let r = test::call_service(
                    &app,
                    test::TestRequest::default()
                        .method(m)
                        .uri(from)
                        .to_request(),
                )
                .await;
                assert_eq!(r.status(), 301, "{from}");
                assert_eq!(r.headers().get(header::LOCATION).unwrap(), to, "{from}");
            }
        }
        let campaign = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/?utm_source=test")
                .to_request(),
        )
        .await;
        assert_eq!(
            campaign.status(),
            200,
            "campaign parameters must keep the discovery homepage"
        );
        // /healthz answers both too (200 or 503 depending on the data's age).
        for m in [Method::GET, Method::HEAD] {
            let r = test::call_service(
                &app,
                test::TestRequest::default()
                    .method(m.clone())
                    .uri("/healthz")
                    .to_request(),
            )
            .await;
            assert!(
                r.status() == 200 || r.status() == 503,
                "{m} /healthz: {}",
                r.status()
            );
        }
        let r = test::call_service(
            &app,
            test::TestRequest::default()
                .method(Method::HEAD)
                .uri("/nope")
                .to_request(),
        )
        .await;
        assert_eq!(r.status(), 404);
    }

    #[actix_web::test]
    async fn security_headers_are_set_on_pages() {
        let s = state();
        let app = app!(s);
        let r = test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await;
        let h = |n: &str| {
            r.headers()
                .get(n)
                .map(|v| v.to_str().unwrap().to_string())
                .unwrap_or_default()
        };
        let csp = h("content-security-policy");
        assert!(
            csp.contains("default-src 'self'")
                && csp.contains("frame-ancestors 'none'")
                && csp.contains("object-src 'none'")
                && csp.contains("base-uri 'none'"),
            "{csp}"
        );
        assert!(
            !csp.contains("script-src 'self' 'unsafe-inline'"),
            "inline scripts are allowed by hash only"
        );
        assert_eq!(h("x-content-type-options"), "nosniff");
        assert_eq!(h("referrer-policy"), "strict-origin-when-cross-origin");
        assert_eq!(h("x-frame-options"), "DENY");
    }

    #[actix_web::test]
    async fn discovery_metadata_is_indexable_and_structured() {
        let s = state();
        let app = app!(s);

        let home =
            test::call_and_read_body(&app, test::TestRequest::get().uri("/").to_request()).await;
        let home = String::from_utf8(home.to_vec()).unwrap();
        assert!(home.contains(
            r#"<meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1">"#
        ));
        assert!(home.contains(r#"<link rel="canonical" href="https://equihash.com/">"#));
        assert!(home.contains(r#"<link rel="alternate" type="text/plain" href="/llms.txt""#));
        let json = home
            .split(r#"<script type="application/ld+json">"#)
            .nth(1)
            .and_then(|s| s.split("</script>").next())
            .expect("JSON-LD block");
        let graph: serde_json::Value = serde_json::from_str(json).expect("valid JSON-LD");
        let nodes = graph["@graph"].as_array().unwrap();
        assert_eq!(
            nodes.iter().filter(|n| n["@type"] == "WebSite").count(),
            1,
            "WebSite is declared once on the homepage"
        );

        let asics =
            test::call_and_read_body(&app, test::TestRequest::get().uri("/asics").to_request())
                .await;
        let asics = String::from_utf8(asics.to_vec()).unwrap();
        let json = asics
            .split(r#"<script type="application/ld+json">"#)
            .nth(1)
            .and_then(|s| s.split("</script>").next())
            .expect("ASIC index JSON-LD block");
        let graph: serde_json::Value =
            serde_json::from_str(json).expect("valid ASIC index JSON-LD");
        let list = graph["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["@type"] == "ItemList")
            .expect("ASIC index exposes an ItemList");
        assert_eq!(list["numberOfItems"], s.get().miners.len());
        assert_eq!(
            list["itemListElement"][0]["url"],
            "https://equihash.com/asics/antminer-z15-pro"
        );

        let product = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/asics/antminer-z15-pro")
                .to_request(),
        )
        .await;
        let product = String::from_utf8(product.to_vec()).unwrap();
        let json = product
            .split(r#"<script type="application/ld+json">"#)
            .nth(1)
            .and_then(|s| s.split("</script>").next())
            .expect("product JSON-LD block");
        let graph: serde_json::Value = serde_json::from_str(json).expect("valid product JSON-LD");
        let nodes = graph["@graph"].as_array().unwrap();
        assert!(nodes.iter().any(|n| n["@type"] == "Product"));
        assert!(nodes.iter().any(|n| n["@type"] == "BreadcrumbList"));
        assert!(!nodes.iter().any(|n| n["@type"] == "WebSite"));

        let z15k = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/asics/antminer-z15k-565")
                .to_request(),
        )
        .await;
        let z15k = String::from_utf8(z15k.to_vec()).unwrap();
        assert!(z15k.contains("/static/shop/machines/antminer-z15k-market.9c0237c6c8.png"));
        assert!(z15k.contains("Market catalogue render ↗"));
        let json = z15k
            .split(r#"<script type="application/ld+json">"#)
            .nth(1)
            .and_then(|s| s.split("</script>").next())
            .expect("Z15K JSON-LD block");
        let graph: serde_json::Value = serde_json::from_str(json).expect("valid Z15K JSON-LD");
        let product = graph["@graph"]
            .as_array()
            .unwrap()
            .iter()
            .find(|node| node["@type"] == "Product")
            .expect("Z15K Product schema");
        assert_eq!(
            product["image"],
            "https://equihash.com/static/shop/machines/antminer-z15k-market.9c0237c6c8.png"
        );

        for path in ["/search?q=zcash", "/contribute", "/add-vendor"] {
            let body =
                test::call_and_read_body(&app, test::TestRequest::get().uri(path).to_request())
                    .await;
            let body = String::from_utf8(body.to_vec()).unwrap();
            assert!(
                body.contains(r#"<meta name="robots" content="noindex, follow">"#),
                "{path} must stay out of search results"
            );
        }
    }

    #[actix_web::test]
    async fn molepool_unpaid_promotion_is_disclosed_without_changing_other_hashrate_order() {
        let s = state();
        let app = app!(s);

        let home =
            test::call_and_read_body(&app, test::TestRequest::get().uri("/").to_request()).await;
        let home = String::from_utf8(home.to_vec()).unwrap();
        let workspace = home
            .split(r#"class="ed-workspace-pools""#)
            .nth(1)
            .expect("homepage pool shortlist");
        let via = workspace.find("ViaBTC").expect("ViaBTC in shortlist");
        let foundry = workspace.find("Foundry").expect("Foundry in shortlist");
        let f2pool = workspace.find("F2Pool").expect("F2Pool in shortlist");
        let molepool = workspace
            .find("molepool.com")
            .expect("Molepool in shortlist");
        assert!(via < foundry && foundry < f2pool && f2pool < molepool);
        assert!(workspace[..molepool + "molepool.com".len()].contains("4"));
        assert!(workspace.contains("Unpaid promotion"));
        assert!(workspace.contains("ed-workspace-promotion"));
        assert!(!workspace.contains("pool-promotion-label"));

        let pools = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/pools?coin=zcash")
                .to_request(),
        )
        .await;
        let pools = String::from_utf8(pools.to_vec()).unwrap();
        let table = pools
            .split(r#"id="pool-table""#)
            .nth(1)
            .expect("pool table");
        let molepool = table
            .find(r#"data-pool-id="zcash:molepool.com:70""#)
            .expect("promoted Molepool row");
        let via = table.find(r#"data-name="viabtc""#).expect("ViaBTC row");
        let foundry = table.find(r#"data-name="foundry""#).expect("Foundry row");
        let f2pool = table.find(r#"data-name="f2pool""#).expect("F2Pool row");
        assert!(molepool < via && via < foundry && foundry < f2pool);
        assert!(table.contains(r#"data-promotion="unpaid""#));
        assert!(
            table.contains(r#"class="pool-promoted-reveal" aria-hidden="true">Unpaid promotion"#)
        );
        assert!(!table.contains("pool-promotion-label"));
        assert!(pools.contains("Every other row remains ordered by reported hashrate."));
    }

    #[actix_web::test]
    async fn sitemap_contains_only_canonical_working_pages() {
        let s = state();
        let app = app!(s.clone());
        let response = test::call_service(
            &app,
            test::TestRequest::get().uri("/sitemap.xml").to_request(),
        )
        .await;
        assert_eq!(response.status(), 200);
        let body = test::read_body(response).await;
        let body = String::from_utf8(body.to_vec()).unwrap();
        assert!(body.contains("<lastmod>"));
        assert!(body.contains("https://equihash.com/zcash-mining"));
        assert!(!body.contains("https://equihash.com/miners"));
        assert!(!body.contains("https://equihash.com/add-pool"));
        assert!(!body.contains("https://equihash.com/contribute"));
        for old in s.get().slug_redirects.keys() {
            assert!(
                !body.contains(&format!("https://equihash.com/pool/{old}<")),
                "redirecting pool slug in sitemap: {old}"
            );
        }

        let mut seen = std::collections::HashSet::new();
        for loc in body
            .split("<loc>")
            .skip(1)
            .filter_map(|x| x.split("</loc>").next())
        {
            let path = loc
                .strip_prefix(views::layout::SITE)
                .expect("same-origin sitemap URL")
                .replace("&amp;", "&");
            assert!(seen.insert(path.clone()), "duplicate sitemap URL: {path}");
            let response =
                test::call_service(&app, test::TestRequest::get().uri(&path).to_request()).await;
            assert_eq!(response.status(), 200, "sitemap URL must resolve: {path}");
        }
        assert!(
            seen.len() > 25,
            "sitemap unexpectedly small: {}",
            seen.len()
        );
    }

    #[actix_web::test]
    async fn vendor_directory_uses_neutral_records_and_disclosed_sorts() {
        let s = state();
        assert_eq!(s.get().vendor_research.len(), 87);
        assert_eq!(
            s.get()
                .vendor_research
                .iter()
                .filter(|record| record.record_type == "coverage_gap")
                .count(),
            2
        );
        let app = app!(s);
        let directory =
            test::call_and_read_body(&app, test::TestRequest::get().uri("/vendors").to_request())
                .await;
        let directory = String::from_utf8(directory.to_vec()).unwrap();
        assert!(directory.contains("<h1>ASIC vendors</h1>"));
        assert!(directory.contains("85</dd>"));
        assert!(directory.contains("32</dd>"));
        assert!(directory.contains("Not published</dd>"));
        assert_eq!(directory.matches("vendor-directory-public-row").count(), 85);
        assert!(directory.contains("Sorted by: Alphabetical"));
        assert!(directory.contains("No Equihash.com recommendation or quality judgment is implied"));
        assert!(directory.contains("Domain-age sorting is disabled"));
        assert!(directory.contains("seller-declared unless expressly stated otherwise"));
        assert!(directory.contains(
            "Paid listings and affiliate relationships cannot influence the default order"
        ));
        assert!(directory.find("21energy").unwrap() < directory.find("21Mining").unwrap());
        assert!(!directory.contains("Tier A"));
        assert!(!directory.contains("Strongest evidence"));
        assert!(!directory.contains("global rank"));
        assert!(!directory.contains("delivery probability"));
        assert!(!directory.contains("No weak substitute was invented"));
        assert!(!directory.contains("No vetted local Z15 source identified"));
        assert!(!directory.contains("Trustpilot rating"));
        assert!(!directory.contains("Trustpilot review count"));
        assert!(!directory.contains("/5 ·"));

        let disabled_review_sort = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/vendors?sort=review_count")
                .to_request(),
        )
        .await;
        let disabled_review_sort = String::from_utf8(disabled_review_sort.to_vec()).unwrap();
        assert!(disabled_review_sort.contains("Sorted by: Alphabetical"));
        assert!(
            disabled_review_sort.find("21energy").unwrap()
                < disabled_review_sort.find("21Mining").unwrap()
        );

        let manufacturer = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/vendors?sort=manufacturer_direct")
                .to_request(),
        )
        .await;
        let manufacturer = String::from_utf8(manufacturer.to_vec()).unwrap();
        assert!(manufacturer.contains("Sorted by: Manufacturer direct"));
        assert!(
            manufacturer.find("BITMAIN official shop").unwrap()
                < manufacturer.find("21energy").unwrap()
        );

        let vendor = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/vendors/antminer-distribution-europe")
                .to_request(),
        )
        .await;
        let vendor = String::from_utf8(vendor.to_vec()).unwrap();
        assert!(vendor.contains(r#"data-brand-shape="rail""#));
        assert!(vendor.contains("--vendor-accent:#356ea5"));
        assert!(vendor.contains("1 miner · 3 listings"));
        assert!(!vendor.contains("data-machine-carousel"));
        assert_eq!(vendor.matches("data-machine-slide").count(), 1);
        assert!(vendor.contains("dispatch in seven days"));
        assert!(vendor.contains("Ships out in December"));
        assert_eq!(vendor.matches("seller-declared page claims").count(), 2);
        assert!(vendor.contains("Inclusion is not an endorsement or a fulfillment guarantee"));

        let broad_catalog = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/vendors/crypto-miner-bros")
                .to_request(),
        )
        .await;
        let broad_catalog = String::from_utf8(broad_catalog.to_vec()).unwrap();
        assert!(broad_catalog.contains(r#"data-brand-shape="wedge""#));
        assert!(broad_catalog.contains("--vendor-accent:#30343a"));
        assert!(broad_catalog.contains("6 miners · 9 listings"));
        assert!(broad_catalog.contains("data-machine-carousel"));
        assert_eq!(broad_catalog.matches("data-machine-slide").count(), 6);
        assert!(broad_catalog.contains("Antminer Z9 Mini"));

        let uk_vendor = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/vendors/the-bitcoin-miner-uk")
                .to_request(),
        )
        .await;
        let uk_vendor = String::from_utf8(uk_vendor.to_vec()).unwrap();
        assert!(uk_vendor.contains(r#"data-brand-shape="arc""#));
        assert!(uk_vendor.contains("--vendor-accent:#dd7b16"));
        assert!(!uk_vendor.contains("Warning record"));
        assert!(!uk_vendor.contains("Product links are withheld"));
        assert!(uk_vendor.contains("vendor-offer-table"));

        let search = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/search?q=Mineshop.eu")
                .to_request(),
        )
        .await;
        let search = String::from_utf8(search.to_vec()).unwrap();
        assert!(search.contains("Vendor directory"));
        assert!(search.contains("Mineshop.eu"));
    }

    #[actix_web::test]
    async fn crawler_files_are_clear_and_data_is_noindex() {
        let s = state();
        let app = app!(s);
        let robots = test::call_and_read_body(
            &app,
            test::TestRequest::get().uri("/robots.txt").to_request(),
        )
        .await;
        let robots = String::from_utf8(robots.to_vec()).unwrap();
        assert!(robots.contains("User-agent: OAI-SearchBot\nAllow: /"));
        assert!(robots.contains("Sitemap: https://equihash.com/sitemap.xml"));

        let llms =
            test::call_and_read_body(&app, test::TestRequest::get().uri("/llms.txt").to_request())
                .await;
        let llms = String::from_utf8(llms.to_vec()).unwrap();
        assert!(llms.contains("https://equihash.com/zcash-mining"));
        assert!(llms.contains("https://equihash.com/industry/cypherpunk-zcash-mining"));
        assert!(llms.contains("https://equihash.com/industry/grayscale-zcash-etf"));
        assert!(llms.contains("https://equihash.com/industry/winklevoss-zcash-etf"));
        assert!(llms.contains("https://equihash.com/data/pools.json"));
        assert!(llms.contains("https://equihash.com/data/vendor-directory.json"));
        assert!(llms.contains("Wcash receives work only from participating merged-mining pools"));

        let research = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/industry/cypherpunk-zcash-mining")
                .to_request(),
        )
        .await;
        let research = String::from_utf8(research.to_vec()).unwrap();
        assert!(research.contains("4,902 Z15 Pro miners"));
        assert!(research.contains("Preliminary WINK filing"));
        assert!(research.contains("not fleet telemetry"));

        let industry =
            test::call_and_read_body(&app, test::TestRequest::get().uri("/industry").to_request())
                .await;
        let industry = String::from_utf8(industry.to_vec()).unwrap();
        assert!(industry.contains("aria-current=\"page\">Industry"));
        assert!(industry.contains("/industry/grayscale-zcash-etf"));
        assert!(industry.contains("/industry/winklevoss-zcash-etf"));

        let grayscale = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/industry/grayscale-zcash-etf")
                .to_request(),
        )
        .await;
        let grayscale = String::from_utf8(grayscale.to_vec()).unwrap();
        assert!(grayscale.contains("Trading on NYSE Arca"));
        assert!(grayscale.contains("Not a miner"));

        let wink = test::call_and_read_body(
            &app,
            test::TestRequest::get()
                .uri("/industry/winklevoss-zcash-etf")
                .to_request(),
        )
        .await;
        let wink = String::from_utf8(wink.to_vec()).unwrap();
        assert!(wink.contains("Not effective, launched or trading"));
        assert!(wink.contains("does not mean WINK owns Cypherpunk’s miners"));

        let old = test::call_service(
            &app,
            test::TestRequest::get()
                .uri("/research/cypherpunk-zcash-mining")
                .to_request(),
        )
        .await;
        assert_eq!(old.status(), 301);
        assert_eq!(
            old.headers().get(header::LOCATION).unwrap(),
            "/industry/cypherpunk-zcash-mining"
        );

        for path in ["/api/live", "/healthz", "/data/pools.json"] {
            let response =
                test::call_service(&app, test::TestRequest::get().uri(path).to_request()).await;
            assert_eq!(
                response.headers().get("x-robots-tag").unwrap(),
                "noindex",
                "{path}"
            );
        }
    }

    #[actix_web::test]
    async fn health_is_503_when_data_is_stale_and_names_the_snapshot() {
        let s = state();
        let d = s.get();
        let gen = chrono::DateTime::parse_from_rfc3339(
            d.meta
                .generated_at
                .as_deref()
                .or(d.last_updated.as_deref())
                .unwrap(),
        )
        .unwrap()
        .with_timezone(&chrono::Utc);
        let (code, body) = health(
            &d,
            gen + chrono::Duration::minutes(30),
            HEALTH_MAX_DATA_AGE_SECS,
            None,
            false,
        );
        assert_eq!(code, 200);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["data"]["age_secs"], 1800);
        assert_eq!(body["data"]["snapshot"], serde_json::json!(d.snapshot));
        let (code, body) = health(
            &d,
            gen + chrono::Duration::hours(5),
            HEALTH_MAX_DATA_AGE_SECS,
            None,
            false,
        );
        assert_eq!(code, 503);
        assert_eq!(body["status"], "error");
        let (code, body) = health(
            &d,
            gen + chrono::Duration::minutes(30),
            HEALTH_MAX_DATA_AGE_SECS,
            Some("pools.json: bad"),
            false,
        );
        assert_eq!(
            (code, body["status"].as_str()),
            (200, Some("degraded")),
            "a failing reload is visible but the old data still serves"
        );
        assert_eq!(body["data"]["reload_error"], "data reload failed");
        assert!(
            !body.to_string().contains("pools.json: bad"),
            "internal reload details are not public"
        );
        // Live sources that have not answered yet make it degraded, never down.
        let (code, body) = health(
            &d,
            gen + chrono::Duration::minutes(30),
            HEALTH_MAX_DATA_AGE_SECS,
            None,
            true,
        );
        assert_eq!(code, 200);
        assert_eq!(
            body["status"],
            if d.live.is_empty() { "ok" } else { "degraded" }
        );
    }

    #[actix_web::test]
    async fn old_pool_urls_redirect_to_the_permalink() {
        let s = state();
        let d = s.get();
        let Some((old, to)) = d
            .slug_redirects
            .iter()
            .next()
            .map(|(a, b)| (a.clone(), b.clone()))
        else {
            // No renamed pool in the current data: make one.
            let mut d2 = (*d).clone();
            d2.slug_redirects
                .insert("old-name-for-a-pool".into(), d2.pools[0].slug.clone());
            let to = d2.pools[0].slug.clone();
            *s.data.write().unwrap() = Arc::new(d2);
            let app = app!(s);
            let r = test::call_service(
                &app,
                test::TestRequest::get()
                    .uri("/pool/old-name-for-a-pool")
                    .to_request(),
            )
            .await;
            assert_eq!(r.status(), 301);
            assert_eq!(
                r.headers().get(header::LOCATION).unwrap(),
                &format!("/pool/{to}")
            );
            return;
        };
        let app = app!(s);
        let r = test::call_service(
            &app,
            test::TestRequest::get()
                .uri(&format!("/pool/{old}"))
                .to_request(),
        )
        .await;
        assert_eq!(r.status(), 301);
        assert_eq!(
            r.headers().get(header::LOCATION).unwrap(),
            &format!("/pool/{to}")
        );
    }
}
