//! equihash.com: a small Actix Web server that renders the pool directory from data/*.json.
//!
//! Run:  cargo run --release            (serves http://127.0.0.1:8080, PORT/HOST/DATA_DIR/STATIC_DIR override)
//! Data: npm run refresh                (publishes a new snapshot; the server reloads it automatically)

mod data;
mod fmt;
mod live;
mod views;

use actix_files::{Files, NamedFile};
use actix_web::{guard, http::header, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
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
        AppState { data: RwLock::new(Arc::new(d)), data_dir, live: RwLock::new(live::LiveState::default()), live_enabled, reload_error: RwLock::new(None) }
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

async fn home(s: web::Data<AppState>, q: web::Query<views::home::Filters>) -> impl Responder {
    html(views::home::render(&s.get(), &q))
}

/// Pool pages live at their permanent slug (data/curated/permalinks.json). An old URL (the slug
/// once computed from coin, name and domain, or a curated alias) redirects there with a 301.
async fn pool(s: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let d = s.get();
    if let Some(p) = d.pools.iter().find(|p| p.slug == *path) {
        return html(views::pages::pool_page(&d, p));
    }
    if let Some(to) = d.slug_redirects.get(path.as_str()) {
        return HttpResponse::MovedPermanently().insert_header((header::LOCATION, format!("/pool/{to}"))).finish();
    }
    HttpResponse::NotFound().content_type("text/html; charset=utf-8").body(views::pages::not_found(&d).into_string())
}

async fn archive(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::archive(&s.get()))
}
async fn miners(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::miners(&s.get()))
}
async fn calculator(s: web::Data<AppState>, q: web::Query<views::calc::CalcQuery>) -> impl Responder {
    html(views::calc::render(&s.get(), &q))
}
async fn guide(s: web::Data<AppState>) -> impl Responder {
    html(views::guide::render(&s.get()))
}
async fn add_pool(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::add_pool(&s.get()))
}
async fn about(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::about(&s.get()))
}
async fn sources(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::sources(&s.get()))
}

/// Live figures for the page to poll (every 60 s). Read from the server's own last good readings,
/// so browsers never call the pool's endpoints.
async fn api_live(s: web::Data<AppState>) -> HttpResponse {
    let d = s.get();
    HttpResponse::Ok()
        .insert_header((header::CACHE_CONTROL, "public, max-age=15"))
        .json(views::live_json(&d, chrono::Utc::now()))
}

/// Default for `HEALTH_MAX_DATA_AGE_SECS`: three missed hourly refreshes.
const HEALTH_MAX_DATA_AGE_SECS: i64 = 3 * 3600;

/// Body and status of /healthz. 503 when the data is older than `max_age` (or its age is unknown);
/// "degraded" (still 200) when a reload is failing or a live source is down or stale.
fn health(d: &data::Data, now: chrono::DateTime<chrono::Utc>, max_age: i64, reload_error: Option<&str>, live_enabled: bool) -> (u16, serde_json::Value) {
    let generated = d.meta.generated_at.clone().or(d.last_updated.clone());
    let age = data::age_secs(generated.as_deref(), now);
    let data_ok = age.map(|a| a <= max_age).unwrap_or(false);
    let live_sources: Vec<serde_json::Value> = d
        .live
        .iter()
        .map(|s| serde_json::json!({"id": s.id, "status": s.status, "age_secs": s.age_secs, "stale": s.stale, "error": s.error.as_ref().map(|_| "source unavailable"), "last_ok_at": s.last_ok_at}))
        .collect();
    let live_ok = !live_enabled || d.live.iter().all(|s| s.status == "ok" && !s.stale);
    let status = if !data_ok { "error" } else if reload_error.is_some() || !live_ok { "degraded" } else { "ok" };
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
    let max_age = std::env::var("HEALTH_MAX_DATA_AGE_SECS").ok().and_then(|v| v.parse().ok()).unwrap_or(HEALTH_MAX_DATA_AGE_SECS);
    let err = s.reload_error.read().unwrap().clone();
    let (code, body) = health(&s.get(), chrono::Utc::now(), max_age, err.as_deref(), s.live_enabled);
    HttpResponse::build(actix_web::http::StatusCode::from_u16(code).unwrap()).insert_header((header::CACHE_CONTROL, "no-store")).json(body)
}

/// Public, read-only copies of the data files (transparency). The generated files are served
/// from the same snapshot the pages were rendered from.
async fn data_file(s: web::Data<AppState>, path: web::Path<String>, req: HttpRequest) -> HttpResponse {
    const ALLOWED: &[&str] = &["pools.json", "network.json", "archive.json", "miners.json", "meta.json", "research.json", "current.json"];
    if !ALLOWED.contains(&path.as_str()) {
        return HttpResponse::NotFound().finish();
    }
    let d = s.get();
    let file = if data::GENERATED.contains(&path.as_str()) { d.snapshot_dir.join(path.as_str()) } else { s.data_dir.join(path.as_str()) };
    match NamedFile::open(file) {
        Ok(f) => f.into_response(&req),
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
    HttpResponse::Ok().content_type("text/plain").body(format!("User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n", views::layout::SITE))
}

async fn sitemap(s: web::Data<AppState>) -> HttpResponse {
    let d = s.get();
    let mut urls: Vec<String> = ["/", "/miners", "/calculator", "/merged-mining", "/archive", "/add-pool", "/about", "/sources"].iter().map(|p| p.to_string()).collect();
    urls.extend(d.pools.iter().map(|p| format!("/pool/{}", p.slug)));
    let body: String = urls.iter().map(|u| format!("<url><loc>{}{}</loc></url>", views::layout::SITE, u)).collect();
    HttpResponse::Ok().content_type("application/xml").body(format!(r#"<?xml version="1.0" encoding="UTF-8"?><urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">{body}</urlset>"#))
}

async fn not_found(s: web::Data<AppState>) -> HttpResponse {
    HttpResponse::NotFound().content_type("text/html; charset=utf-8").body(views::pages::not_found(&s.get()).into_string())
}

fn static_dir() -> PathBuf {
    std::env::var("STATIC_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("static"))
}

/// Security headers on every response. HSTS is left to the TLS proxy unless HSTS=1 (the app
/// itself usually speaks plain HTTP on localhost); see deploy/.
fn security_headers() -> middleware::DefaultHeaders {
    let mut h = middleware::DefaultHeaders::new()
        .add((header::CONTENT_SECURITY_POLICY, views::layout::csp()))
        .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff"))
        .add((header::REFERRER_POLICY, "strict-origin-when-cross-origin"))
        .add((header::X_FRAME_OPTIONS, "DENY"))
        .add(("Permissions-Policy", "camera=(), microphone=(), geolocation=(), payment=(), usb=(), interest-cohort=()"))
        .add(("Cross-Origin-Opener-Policy", "same-origin"));
    if std::env::var("HSTS").map(|v| v == "1").unwrap_or(false) {
        h = h.add((header::STRICT_TRANSPORT_SECURITY, "max-age=31536000; includeSubDomains"));
    }
    h
}

fn routes(cfg: &mut web::ServiceConfig, sdir: PathBuf) {
    // GET and HEAD on every route. Registered by hand rather than with #[route(method = "GET",
    // method = "HEAD")]: that macro keeps its methods in a HashSet, so the guard order (and the
    // binary) changed from build to build.
    let get_head = || web::route().guard(guard::Any(guard::Get()).or(guard::Head()));
    cfg.service(web::resource("/").route(get_head().to(home)))
        .service(web::resource("/pool/{slug}").route(get_head().to(pool)))
        .service(web::resource("/archive").route(get_head().to(archive)))
        .service(web::resource("/miners").route(get_head().to(miners)))
        .service(web::resource("/calculator").route(get_head().to(calculator)))
        .service(web::resource("/merged-mining").route(get_head().to(guide)))
        .service(web::resource("/add-pool").route(get_head().to(add_pool)))
        .service(web::resource("/about").route(get_head().to(about)))
        .service(web::resource("/sources").route(get_head().to(sources)))
        .service(web::resource("/api/live").route(get_head().to(api_live)))
        .service(web::resource("/healthz").route(get_head().to(healthz)))
        .service(web::resource("/data/{file}").route(get_head().to(data_file)))
        .service(web::resource("/favicon.ico").route(get_head().to(favicon)))
        .service(web::resource("/robots.txt").route(get_head().to(robots)))
        .service(web::resource("/sitemap.xml").route(get_head().to(sitemap)))
        // Logos first: long immutable cache and a CSP of their own (src/views/logo.rs).
        .service(views::logo::service(&sdir))
        .service(Files::new("/static", sdir).use_etag(true))
        .default_service(web::to(not_found));
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let data_dir = std::env::var("DATA_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("data"));
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
        d.snapshot.as_deref().map(|s| format!(" (snapshot {s})")).unwrap_or_default()
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
                        println!("Reloaded data: {} pools{}", d.pools.len(), d.snapshot.as_deref().map(|s| format!(" (snapshot {s})")).unwrap_or_default());
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
            let client = match reqwest::Client::builder().timeout(Duration::from_secs(10)).user_agent("equihash.com live poller").build() {
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
                    changed |= state.live.write().unwrap().record(&src.id, r, chrono::Utc::now());
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
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
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
            test::init_service(App::new().app_data($s.clone()).wrap(security_headers()).configure(|c| routes(c, Path::new(env!("CARGO_MANIFEST_DIR")).join("static")))).await
        };
    }

    #[actix_web::test]
    async fn every_get_route_also_answers_head() {
        let s = state();
        let app = app!(s);
        let slug = s.get().pools[0].slug.clone();
        let paths = [
            "/".to_string(),
            "/?coin=wcash".into(),
            format!("/pool/{slug}"),
            "/archive".into(),
            "/miners".into(),
            "/calculator".into(),
            "/merged-mining".into(),
            "/add-pool".into(),
            "/about".into(),
            "/sources".into(),
            "/api/live".into(),
            "/data/pools.json".into(),
            "/data/miners.json".into(),
            "/favicon.ico".into(),
            "/robots.txt".into(),
            "/sitemap.xml".into(),
            "/static/app.js".into(),
        ];
        for p in &paths {
            let get = test::call_service(&app, test::TestRequest::get().uri(p).to_request()).await;
            let head = test::call_service(&app, test::TestRequest::default().method(Method::HEAD).uri(p).to_request()).await;
            assert_eq!(get.status(), 200, "GET {p}");
            assert_eq!(head.status(), get.status(), "HEAD {p}");
            assert_eq!(head.headers().get(header::CONTENT_TYPE), get.headers().get(header::CONTENT_TYPE), "HEAD {p} content type");
        }
        // /healthz answers both too (200 or 503 depending on the data's age).
        for m in [Method::GET, Method::HEAD] {
            let r = test::call_service(&app, test::TestRequest::default().method(m.clone()).uri("/healthz").to_request()).await;
            assert!(r.status() == 200 || r.status() == 503, "{m} /healthz: {}", r.status());
        }
        let r = test::call_service(&app, test::TestRequest::default().method(Method::HEAD).uri("/nope").to_request()).await;
        assert_eq!(r.status(), 404);
    }

    #[actix_web::test]
    async fn security_headers_are_set_on_pages() {
        let s = state();
        let app = app!(s);
        let r = test::call_service(&app, test::TestRequest::get().uri("/").to_request()).await;
        let h = |n: &str| r.headers().get(n).map(|v| v.to_str().unwrap().to_string()).unwrap_or_default();
        let csp = h("content-security-policy");
        assert!(csp.contains("default-src 'self'") && csp.contains("frame-ancestors 'none'") && csp.contains("object-src 'none'") && csp.contains("base-uri 'none'"), "{csp}");
        assert!(!csp.contains("script-src 'self' 'unsafe-inline'"), "inline scripts are allowed by hash only");
        assert_eq!(h("x-content-type-options"), "nosniff");
        assert_eq!(h("referrer-policy"), "strict-origin-when-cross-origin");
        assert_eq!(h("x-frame-options"), "DENY");
    }

    #[actix_web::test]
    async fn health_is_503_when_data_is_stale_and_names_the_snapshot() {
        let s = state();
        let d = s.get();
        let gen = chrono::DateTime::parse_from_rfc3339(d.meta.generated_at.as_deref().or(d.last_updated.as_deref()).unwrap()).unwrap().with_timezone(&chrono::Utc);
        let (code, body) = health(&d, gen + chrono::Duration::minutes(30), HEALTH_MAX_DATA_AGE_SECS, None, false);
        assert_eq!(code, 200);
        assert_eq!(body["status"], "ok");
        assert_eq!(body["data"]["age_secs"], 1800);
        assert_eq!(body["data"]["snapshot"], serde_json::json!(d.snapshot));
        let (code, body) = health(&d, gen + chrono::Duration::hours(5), HEALTH_MAX_DATA_AGE_SECS, None, false);
        assert_eq!(code, 503);
        assert_eq!(body["status"], "error");
        let (code, body) = health(&d, gen + chrono::Duration::minutes(30), HEALTH_MAX_DATA_AGE_SECS, Some("pools.json: bad"), false);
        assert_eq!((code, body["status"].as_str()), (200, Some("degraded")), "a failing reload is visible but the old data still serves");
        assert_eq!(body["data"]["reload_error"], "data reload failed");
        assert!(!body.to_string().contains("pools.json: bad"), "internal reload details are not public");
        // Live sources that have not answered yet make it degraded, never down.
        let (code, body) = health(&d, gen + chrono::Duration::minutes(30), HEALTH_MAX_DATA_AGE_SECS, None, true);
        assert_eq!(code, 200);
        assert_eq!(body["status"], if d.live.is_empty() { "ok" } else { "degraded" });
    }

    #[actix_web::test]
    async fn old_pool_urls_redirect_to_the_permalink() {
        let s = state();
        let d = s.get();
        let Some((old, to)) = d.slug_redirects.iter().next().map(|(a, b)| (a.clone(), b.clone())) else {
            // No renamed pool in the current data: make one.
            let mut d2 = (*d).clone();
            d2.slug_redirects.insert("old-name-for-a-pool".into(), d2.pools[0].slug.clone());
            let to = d2.pools[0].slug.clone();
            *s.data.write().unwrap() = Arc::new(d2);
            let app = app!(s);
            let r = test::call_service(&app, test::TestRequest::get().uri("/pool/old-name-for-a-pool").to_request()).await;
            assert_eq!(r.status(), 301);
            assert_eq!(r.headers().get(header::LOCATION).unwrap(), &format!("/pool/{to}"));
            return;
        };
        let app = app!(s);
        let r = test::call_service(&app, test::TestRequest::get().uri(&format!("/pool/{old}")).to_request()).await;
        assert_eq!(r.status(), 301);
        assert_eq!(r.headers().get(header::LOCATION).unwrap(), &format!("/pool/{to}"));
    }
}
