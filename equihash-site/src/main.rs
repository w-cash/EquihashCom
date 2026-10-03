//! equihash.com: a small Actix Web server that renders the pool directory from data/*.json.
//!
//! Run:  cargo run --release            (serves http://127.0.0.1:8080, PORT/HOST/DATA_DIR/STATIC_DIR override)
//! Data: npm run refresh                (rewrites data/*.json; the server reloads changed files automatically)

mod data;
mod fmt;
mod views;

use actix_files::{Files, NamedFile};
use actix_web::{get, http::header, middleware, web, App, HttpRequest, HttpResponse, HttpServer, Responder};
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime};

struct AppState {
    data: RwLock<Arc<data::Data>>,
    data_dir: PathBuf,
}

impl AppState {
    fn get(&self) -> Arc<data::Data> {
        self.data.read().unwrap().clone()
    }
}

fn html(m: maud::Markup) -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .insert_header((header::CACHE_CONTROL, "public, max-age=60"))
        .body(m.into_string())
}

#[get("/")]
async fn home(s: web::Data<AppState>, q: web::Query<views::home::Filters>) -> impl Responder {
    html(views::home::render(&s.get(), &q))
}

#[get("/pool/{slug}")]
async fn pool(s: web::Data<AppState>, path: web::Path<String>) -> impl Responder {
    let d = s.get();
    match d.pools.iter().find(|p| p.slug == *path) {
        Some(p) => html(views::pages::pool_page(&d, p)),
        None => HttpResponse::NotFound().content_type("text/html; charset=utf-8").body(views::pages::not_found(&d).into_string()),
    }
}

#[get("/archive")]
async fn archive(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::archive(&s.get()))
}
#[get("/miners")]
async fn miners(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::miners(&s.get()))
}
#[get("/calculator")]
async fn calculator(s: web::Data<AppState>, q: web::Query<views::calc::CalcQuery>) -> impl Responder {
    html(views::calc::render(&s.get(), &q))
}
#[get("/merged-mining")]
async fn guide(s: web::Data<AppState>) -> impl Responder {
    html(views::guide::render(&s.get()))
}
#[get("/add-pool")]
async fn add_pool(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::add_pool(&s.get()))
}
#[get("/about")]
async fn about(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::about(&s.get()))
}
#[get("/sources")]
async fn sources(s: web::Data<AppState>) -> impl Responder {
    html(views::pages::sources(&s.get()))
}

/// Public, read-only copies of the data files (transparency).
#[get("/data/{file}")]
async fn data_file(s: web::Data<AppState>, path: web::Path<String>, req: HttpRequest) -> HttpResponse {
    const ALLOWED: &[&str] = &["pools.json", "network.json", "archive.json", "miners.json", "meta.json", "research.json"];
    if !ALLOWED.contains(&path.as_str()) {
        return HttpResponse::NotFound().finish();
    }
    match NamedFile::open(s.data_dir.join(path.as_str())) {
        Ok(f) => f.into_response(&req),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/favicon.ico")]
async fn favicon(req: HttpRequest) -> HttpResponse {
    match NamedFile::open(static_dir().join("favicon.ico")) {
        Ok(f) => f.into_response(&req),
        Err(_) => HttpResponse::NotFound().finish(),
    }
}

#[get("/robots.txt")]
async fn robots() -> HttpResponse {
    HttpResponse::Ok().content_type("text/plain").body(format!("User-agent: *\nAllow: /\nSitemap: {}/sitemap.xml\n", views::layout::SITE))
}

#[get("/sitemap.xml")]
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

fn newest_mtime(dir: &PathBuf) -> Option<SystemTime> {
    let mut newest = None;
    for sub in [dir.clone(), dir.join("curated")] {
        if let Ok(rd) = std::fs::read_dir(&sub) {
            for e in rd.flatten() {
                if let Ok(m) = e.metadata().and_then(|m| m.modified()) {
                    if newest.map(|n| m > n).unwrap_or(true) {
                        newest = Some(m);
                    }
                }
            }
        }
    }
    newest
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    let data_dir = std::env::var("DATA_DIR").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("data"));
    let d = data::load(&data_dir).unwrap_or_else(|e| {
        eprintln!("error: could not load data: {e}\nRun `npm run refresh` or set DATA_DIR.");
        std::process::exit(1);
    });
    println!("Loaded {} pools, {} coins, {} miners, {} archive entries from {}", d.pools.len(), d.coins.len(), d.miners.len(), d.archive.len(), data_dir.display());
    let state = web::Data::new(AppState { data: RwLock::new(Arc::new(d)), data_dir: data_dir.clone() });

    // Hot reload: re-read data/*.json when any file changes (after `npm run refresh` or a manual edit).
    {
        let state = state.clone();
        std::thread::spawn(move || {
            let mut last = newest_mtime(&state.data_dir);
            loop {
                std::thread::sleep(Duration::from_secs(5));
                let now = newest_mtime(&state.data_dir);
                if now != last {
                    std::thread::sleep(Duration::from_millis(500)); // let writers finish
                    match data::load(&state.data_dir) {
                        Ok(d) => {
                            println!("Reloaded data: {} pools", d.pools.len());
                            *state.data.write().unwrap() = Arc::new(d);
                            last = now;
                        }
                        Err(e) => eprintln!("reload failed (keeping previous data): {e}"),
                    }
                }
            }
        });
    }

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = std::env::var("PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080);
    println!("equihash.com serving on http://{host}:{port}");
    let sdir = static_dir();
    HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .wrap(middleware::Compress::default())
            .wrap(middleware::Logger::new("%r %s %Dms"))
            .wrap(middleware::DefaultHeaders::new().add((header::X_CONTENT_TYPE_OPTIONS, "nosniff")).add((header::REFERRER_POLICY, "strict-origin-when-cross-origin")))
            .service(home)
            .service(pool)
            .service(archive)
            .service(miners)
            .service(calculator)
            .service(guide)
            .service(add_pool)
            .service(about)
            .service(sources)
            .service(data_file)
            .service(favicon)
            .service(robots)
            .service(sitemap)
            .service(Files::new("/static", sdir.clone()).use_etag(true))
            .default_service(web::to(not_found))
    })
    .bind((host.as_str(), port))?
    .run()
    .await
}
