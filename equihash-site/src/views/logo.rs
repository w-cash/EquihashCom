//! Coin and pool logos: data/curated/logos.json (written by scripts/fetch-logos.mjs) and the
//! local copies in static/logos/. The site only ever serves those local copies (no hotlinking).
//!
//! Every file is checked on load: the path must stay inside static/logos/{coins,pools,vendors}/, the bytes
//! must be the type the extension says (SVG, PNG or WebP only) and an SVG must pass `svg_is_safe`
//! (no scripts, event handlers, foreignObject, images or external references). Anything that
//! fails, and any coin or pool without an entry, gets a generated monogram instead, so a new coin
//! or pool never shows a broken image.
use maud::{html, Markup};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// URL prefix the files are served under (see `service`).
pub const URL_PREFIX: &str = "/static/logos/";
/// Largest logo file the server will serve (the fetch script aims for 10 KB).
pub const MAX_BYTES: u64 = 64 * 1024;

/// One entry in logos.json. Unknown fields (bytes, tried, domain...) are ignored.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(default)]
pub struct LogoEntry {
    /// Relative to static/logos/, e.g. "coins/zcash.1a2b3c4d5e.svg". None for a fallback.
    pub file: Option<String>,
    pub source_url: Option<String>,
    pub fetched_at: Option<String>,
    /// "official" | "repo" | "third_party" | "fallback"
    pub kind: String,
    pub license_note: Option<String>,
    /// Letters for a fallback monogram (the script writes them; derived from the name otherwise).
    pub monogram: Option<String>,
    /// "tile" when the image is an opaque square of its own (it then fills the chip edge to edge).
    pub bg: Option<String>,
    /// "light" when the mark is white or near-white on transparent (it sits on a dark chip).
    pub ink: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LogosFile {
    pub generated_at: Option<String>,
    pub coins: BTreeMap<String, LogoEntry>,
    pub pools: BTreeMap<String, LogoEntry>,
    pub vendors: BTreeMap<String, LogoEntry>,
}

/// What a page needs to draw one logo. Serialised with each pool for the drawer (static/app.js).
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Logo {
    /// Local URL, or None for a monogram.
    pub src: Option<String>,
    /// "official" | "repo" | "third_party" | "fallback"
    pub kind: String,
    /// Monogram letters (always set, so a failed image can still fall back client-side).
    pub mono: String,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub tile: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub light: bool,
}

impl Logo {
    pub fn is_fallback(&self) -> bool {
        self.src.is_none()
    }
}

/// Validated logos, keyed like logos.json.
#[derive(Debug, Clone, Default)]
pub struct Logos {
    pub generated_at: Option<String>,
    pub coins: BTreeMap<String, LogoEntry>,
    pub pools: BTreeMap<String, LogoEntry>,
    pub vendors: BTreeMap<String, LogoEntry>,
    /// Entries whose file was rejected, with the reason (logged once per load).
    pub rejected: Vec<String>,
}

impl Logos {
    pub fn coin(&self, id: &str, name: &str) -> Logo {
        resolve(self.coins.get(id), name)
    }
    pub fn pool(&self, id: &str, name: &str) -> Logo {
        resolve(self.pools.get(id), name)
    }
    pub fn vendor(&self, id: &str, name: &str) -> Logo {
        resolve(self.vendors.get(id), name)
    }
}

fn static_dir() -> PathBuf {
    std::env::var("STATIC_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("static"))
}

/// Load data/curated/logos.json and check every file in `STATIC_DIR`/logos. A missing file means
/// no logos (monograms everywhere); a broken one is an error, so the last good data keeps serving.
pub fn load(data_dir: &Path) -> Result<Logos, String> {
    load_from(data_dir, &static_dir())
}

pub fn load_from(data_dir: &Path, static_dir: &Path) -> Result<Logos, String> {
    let p = data_dir.join("curated").join("logos.json");
    let f: LogosFile = match std::fs::read_to_string(&p) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", p.display()))?,
        Err(_) => LogosFile::default(),
    };
    let root = static_dir.join("logos");
    let mut rejected = Vec::new();
    let mut check = |group: &str, m: BTreeMap<String, LogoEntry>| -> BTreeMap<String, LogoEntry> {
        m.into_iter()
            .map(|(id, mut e)| {
                if let Some(file) = e.file.clone() {
                    if let Err(why) = check_file(&root, group, &file) {
                        rejected.push(format!("{group} {id}: {file}: {why}"));
                        e.file = None;
                        e.kind = "fallback".into();
                    }
                } else {
                    e.kind = "fallback".into();
                }
                (id, e)
            })
            .collect()
    };
    let coins = check("coins", f.coins);
    let pools = check("pools", f.pools);
    let vendors = check("vendors", f.vendors);
    for r in &rejected {
        log::warn!("logo rejected, using a monogram: {r}");
    }
    Ok(Logos {
        generated_at: f.generated_at,
        coins,
        pools,
        vendors,
        rejected,
    })
}

/// The path is a plain file name inside static/logos/<group>/, the bytes are the type the
/// extension says, and an SVG passes `svg_is_safe`.
pub fn check_file(root: &Path, group: &str, file: &str) -> Result<(), String> {
    let (g, name) = file
        .split_once('/')
        .ok_or("not in coins/, pools/ or vendors/")?;
    if g != group {
        return Err(format!("expected {group}/"));
    }
    let ok_name = !name.is_empty()
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'));
    if !ok_name || name.contains("..") {
        return Err("bad file name".into());
    }
    let ext = name.rsplit('.').next().unwrap_or("");
    let path = root.join(g).join(name);
    let meta = std::fs::metadata(&path).map_err(|e| format!("missing ({e})"))?;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX_BYTES {
        return Err(format!("size {} not in 1..={MAX_BYTES}", meta.len()));
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    match (ext, sniff(&bytes)) {
        ("svg", Some("svg")) => {
            let s = std::str::from_utf8(&bytes).map_err(|_| "SVG is not UTF-8")?;
            if svg_is_safe(s) {
                Ok(())
            } else {
                Err("SVG failed the safety check".into())
            }
        }
        ("png", Some("png")) | ("webp", Some("webp")) => Ok(()),
        (e, t) => Err(format!(
            "extension .{e} but content is {}",
            t.unwrap_or("unknown")
        )),
    }
}

/// File type from the bytes (never the name). Only the three types the site serves.
pub fn sniff(b: &[u8]) -> Option<&'static str> {
    if b.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        return Some("png");
    }
    if b.len() > 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        return Some("webp");
    }
    let head = std::str::from_utf8(&b[..b.len().min(512)])
        .ok()
        .or_else(|| std::str::from_utf8(b).ok())?;
    let t = head.trim_start_matches('\u{feff}').trim_start();
    if t.starts_with("<svg ") || t.starts_with("<svg>") {
        return Some("svg");
    }
    None
}

/// Conservative check run on every SVG before it is served. The fetch script's sanitiser
/// (scripts/fetch-logos.mjs, `sanitizeSvg`) produces files that pass; anything else is refused.
/// Rejects: script, foreignObject, image/feImage, iframe/embed/object, animate/set, DOCTYPE and
/// entities, `on*=` event handlers, javascript:, @import, and any href/src or url() that isn't a
/// same-document `#id` reference.
pub fn svg_is_safe(s: &str) -> bool {
    let l = s.to_ascii_lowercase();
    if !(l.trim_start().starts_with("<svg ") || l.trim_start().starts_with("<svg>")) {
        return false;
    }
    const BAD: &[&str] = &[
        "<script",
        "foreignobject",
        "<image",
        "<feimage",
        "<iframe",
        "<embed",
        "<object",
        "<animate",
        "<set ",
        "<set>",
        "<!entity",
        "<!doctype",
        "<?",
        "javascript:",
        "vbscript:",
        "@import",
        "<a ",
        "<a>",
        "<style",
    ];
    // <style> is refused too: the sanitiser inlines nothing it can't check, and svgo turns
    // stylesheet rules into attributes. A logo that still needs a stylesheet falls back.
    if BAD.iter().any(|b| l.contains(b)) {
        return false;
    }
    let bytes = l.as_bytes();
    // on<letters>= preceded by whitespace (an event handler attribute).
    for (i, w) in l.match_indices("on") {
        let before = if i == 0 { b' ' } else { bytes[i - 1] };
        if !(before.is_ascii_whitespace() || before == b'/' || before == b'"' || before == b'\'') {
            continue;
        }
        let rest = &l[i + w.len()..];
        let name_len = rest.bytes().take_while(|c| c.is_ascii_alphabetic()).count();
        if name_len > 0 && rest[name_len..].trim_start().starts_with('=') {
            return false;
        }
    }
    // Every href/src value and every url() must be a #fragment.
    for key in ["href", "src"] {
        for (i, _) in l.match_indices(key) {
            let rest = l[i + key.len()..].trim_start();
            if let Some(v) = rest.strip_prefix('=') {
                let v = v.trim_start().trim_start_matches(['"', '\'']).trim_start();
                if !v.starts_with('#') {
                    return false;
                }
            }
        }
    }
    for (i, _) in l.match_indices("url(") {
        let v = l[i + 4..]
            .trim_start()
            .trim_start_matches(['"', '\''])
            .trim_start();
        if !v.starts_with('#') {
            return false;
        }
    }
    true
}

/// Monogram letters from a name: "himpool.com (solo)" → "HP", "ViaBTC" → "VB", "zpool" → "ZP".
/// Mirrors `monogram` in scripts/fetch-logos.mjs.
pub fn monogram(name: &str) -> String {
    let mut s: String = {
        // drop "(solo)" and similar
        let mut out = String::new();
        let mut depth = 0;
        for ch in name.chars() {
            match ch {
                '(' => depth += 1,
                ')' => depth = (depth - 1).max(0),
                _ if depth == 0 => out.push(ch),
                _ => {}
            }
        }
        out.trim().to_string()
    };
    for p in ["https://", "http://", "www.", "pool."] {
        if s.to_ascii_lowercase().starts_with(p) {
            s = s[p.len()..].to_string();
        }
    }
    if s.split('.').count() == 4
        && s.split('.').all(|x| {
            !x.is_empty()
                && x.chars()
                    .take_while(|c| *c != ':')
                    .all(|c| c.is_ascii_digit())
        })
    {
        return "IP".into();
    }
    // Strip a trailing TLD: "pooly.ca" → "pooly", "mining-dutch.nl" → "mining-dutch".
    if let Some((head, _)) = s.split_once('.') {
        s = head.to_string();
    }
    // Split camelCase and known compound suffixes: rockpool → rock pool, ViaBTC → Via BTC.
    let mut spaced = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        if i > 0 && c.is_ascii_uppercase() && chars[i - 1].is_ascii_lowercase() {
            spaced.push(' ');
        }
        spaced.push(c);
    }
    let lower = spaced.to_ascii_lowercase();
    let mut out = String::new();
    let mut i = 0;
    let lb = lower.as_bytes();
    while i < spaced.len() {
        let mut hit = None;
        for suf in [
            "pool", "miners", "miner", "mines", "mine", "mining", "hub", "solo",
        ] {
            if i > 0 && lower[i..].starts_with(suf) && lb[i - 1].is_ascii_alphanumeric() {
                let end = i + suf.len();
                if end == lower.len() || !lb[end].is_ascii_alphanumeric() {
                    hit = Some(suf.len());
                    break;
                }
            }
        }
        if let Some(n) = hit {
            out.push(' ');
            out.push_str(&spaced[i..i + n]);
            i += n;
        } else {
            out.push(spaced.as_bytes()[i] as char);
            i += 1;
        }
    }
    let words: Vec<&str> = out
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    match words.as_slice() {
        [] => "?".into(),
        [w] => {
            let first: String = if w.starts_with(|c: char| c.is_ascii_digit()) {
                w.chars().take(2).collect()
            } else {
                w.chars().take(1).collect()
            };
            first.to_ascii_uppercase()
        }
        [a, b, ..] => format!("{}{}", a.chars().next().unwrap(), b.chars().next().unwrap())
            .to_ascii_uppercase(),
    }
}

/// Entry → what the page draws. No entry or no usable file → a monogram from the name.
pub fn resolve(e: Option<&LogoEntry>, name: &str) -> Logo {
    let mono = e
        .and_then(|e| e.monogram.clone())
        .filter(|m| {
            !m.is_empty() && m.chars().count() <= 3 && m.chars().all(|c| c.is_alphanumeric())
        })
        .unwrap_or_else(|| monogram(name));
    match e.and_then(|e| e.file.as_ref().map(|f| (e, f))) {
        Some((e, f)) => Logo {
            src: Some(format!("{URL_PREFIX}{f}")),
            kind: e.kind.clone(),
            mono,
            tile: e.bg.as_deref() == Some("tile"),
            light: e.ink.as_deref() == Some("light"),
        },
        None => Logo {
            src: None,
            kind: "fallback".into(),
            mono,
            tile: false,
            light: false,
        },
    }
}

/// Where a logo sits: decides the size and whether it has its own alt text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum At {
    /// Pool table, networks table (20 px). Next to the name, so the image is decorative.
    Row,
    /// Coin selector, Z15 list, ASIC index (18 px). Decorative.
    List,
    /// Sidebar (16 px, the column is narrow). Decorative.
    Nav,
    /// Coin header, pool page (40 px). Carries "<name> logo".
    Head,
}

impl At {
    pub fn px(self) -> u32 {
        match self {
            At::Row => 20,
            At::List => 18,
            At::Nav => 16,
            At::Head => 40,
        }
    }
}

/// A logo in its bordered chip, with fixed width and height (no layout shift). `lazy` for
/// anything below the fold. Small logos sit right next to the visible name, so their alt is empty
/// (screen readers would otherwise read the name twice); header logos say "<name> logo".
pub fn chip(logo: &Logo, name: &str, at: At, lazy: bool) -> Markup {
    let px = at.px();
    let class = match at {
        At::Row => "logo lg-row",
        At::List => "logo lg-list",
        At::Nav => "logo lg-nav",
        At::Head => "logo lg-head",
    };
    let alt = if at == At::Head {
        format!("{name} logo")
    } else {
        String::new()
    };
    // Inner image size: the chip is px square with a 1 px border and a little padding, unless the
    // logo is an opaque tile of its own, which fills the chip.
    let inner = if logo.tile || at == At::Nav {
        px - 2
    } else if at == At::Head {
        px - 10
    } else {
        px - 4
    };
    html! {
        @match &logo.src {
            Some(src) => {
                span class={(class) @if logo.tile { " tile" } @if logo.light { " light" }} data-kind=(logo.kind) {
                    img src=(src) alt=(alt) width=(inner) height=(inner) decoding="async" loading=[lazy.then_some("lazy")];
                }
            }
            None => {
                span class={(class) " mono"} data-kind="fallback" role=[(at == At::Head).then_some("img")] aria-label=[(at == At::Head).then(|| format!("{name} (no logo published)"))] aria-hidden=[(at != At::Head).then_some("true")] title="No logo found; monogram" {
                    (logo.mono)
                }
            }
        }
    }
}

/// One line for the Sources page: where the logos come from and how many are monograms.
pub fn sources_note(d: &crate::data::Data) -> Markup {
    let coins = d.coins.iter().filter(|c| !c.logo.is_fallback()).count();
    let pools: std::collections::BTreeSet<&str> = d.pools.iter().map(|p| p.name.as_str()).collect();
    let with: std::collections::BTreeSet<&str> = d
        .pools
        .iter()
        .filter(|p| !p.logo.is_fallback())
        .map(|p| p.name.as_str())
        .collect();
    let vendors = d.vendors.iter().filter(|v| !v.logo.is_fallback()).count();
    html! {
        "Coin, pool and vendor logos are local copies, fetched from each entity's own website or official GitHub organisation (coin fallbacks may use the CC0 cryptocurrency-icons set or CoinGecko) and listed with their source in "
        code { "data/curated/logos.json" } ". "
        (coins) " of " (d.coins.len()) " coins, " (with.len()) " of " (pools.len()) " pool names and " (vendors) " of " (d.vendors.len()) " vendors have one; the rest show a plain monogram, never a drawn mark."
        @if let Some(g) = &d.logos.generated_at { " Logos last fetched " (crate::fmt::utc(Some(g))) "." }
        @if !d.logos.rejected.is_empty() { " " (d.logos.rejected.len()) " logo files failed the safety check and show a monogram." }
        " Logos are trademarks of their owners, shown only to identify them."
    }
}

/// /static/logos/*: the same files as /static, with a year-long immutable cache (file names carry
/// a content hash, so a new logo is a new URL) and a CSP that keeps an SVG opened directly from
/// running anything. Register before the general /static service.
pub fn service(static_dir: &Path) -> impl actix_web::dev::HttpServiceFactory + 'static {
    use actix_web::{http::header, middleware::DefaultHeaders, web};
    web::scope("/static/logos")
        .wrap(
            DefaultHeaders::new()
                .add((header::CACHE_CONTROL, "public, max-age=31536000, immutable"))
                .add((header::CONTENT_SECURITY_POLICY, "default-src 'none'; style-src 'unsafe-inline'; sandbox"))
                .add((header::X_CONTENT_TYPE_OPTIONS, "nosniff")),
        )
        .service(actix_files::Files::new("", static_dir.join("logos")).use_etag(true).use_last_modified(true).path_filter(|p, _| {
            // Only <group>/<file>.(svg|png|webp); no listing, no dotfiles, nothing else.
            let s = p.to_string_lossy();
            let mut parts = s.split('/');
            matches!((parts.next(), parts.next(), parts.next()), (Some("coins" | "pools" | "vendors"), Some(f), None)
                if !f.starts_with('.') && (f.ends_with(".svg") || f.ends_with(".webp") || f.ends_with(".png")))
        }))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let p = std::env::temp_dir().join(format!(
                "eqlogo-{tag}-{}-{:?}",
                std::process::id(),
                std::thread::current().id()
            ));
            let _ = std::fs::remove_dir_all(&p);
            for d in ["data/curated", "static/logos/coins", "static/logos/pools"] {
                std::fs::create_dir_all(p.join(d)).unwrap();
            }
            Tmp(p)
        }
        fn data(&self) -> PathBuf {
            self.0.join("data")
        }
        fn stat(&self) -> PathBuf {
            self.0.join("static")
        }
        fn file(&self, rel: &str, bytes: &[u8]) {
            std::fs::write(self.0.join("static/logos").join(rel), bytes).unwrap();
        }
        fn json(&self, v: serde_json::Value) {
            std::fs::write(self.data().join("curated/logos.json"), v.to_string()).unwrap();
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    const SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#f4b728" d="M12 0a12 12 0 1 0 0 24 12 12 0 0 0 0-24z"/></svg>"##;
    fn png() -> Vec<u8> {
        let mut v = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        v.extend_from_slice(&[0; 32]);
        v
    }
    fn webp() -> Vec<u8> {
        let mut v = b"RIFF\x20\0\0\0WEBPVP8L".to_vec();
        v.extend_from_slice(&[0; 24]);
        v
    }

    #[test]
    fn logos_json_loads_and_checks_every_file() {
        let t = Tmp::new("load");
        t.file("coins/zcash.abc.svg", SVG);
        t.file("pools/himpool.com.def.webp", &webp());
        t.file("pools/fake.png", b"<html>not a png</html>");
        t.file(
            "coins/evil.svg",
            br#"<svg xmlns="http://www.w3.org/2000/svg" onload="alert(1)"><path d="M0 0"/></svg>"#,
        );
        t.file("coins/pixel.png", &png());
        t.json(serde_json::json!({
            "generated_at": "2026-10-03T15:00:00Z",
            "coins": {
                "zcash": {"file": "coins/zcash.abc.svg", "kind": "official", "source_url": "https://z.cash/", "fetched_at": "2026-10-03T15:00:00Z", "license_note": "x", "extra": 1},
                "evil": {"file": "coins/evil.svg", "kind": "official"},
                "escape": {"file": "coins/../../../etc/passwd", "kind": "official"},
                "wrongdir": {"file": "pools/himpool.com.def.webp", "kind": "official"},
                "missing": {"file": "coins/nope.svg", "kind": "repo"},
                "renamed": {"file": "coins/pixel.svg", "kind": "repo"},
                "fb": {"file": null, "kind": "fallback", "monogram": "SQ"}
            },
            "pools": {
                "zcash:himpool.com:99": {"file": "pools/himpool.com.def.webp", "kind": "official", "bg": "tile"},
                "zcash:fake.com:1": {"file": "pools/fake.png", "kind": "official"}
            }
        }));
        let l = load_from(&t.data(), &t.stat()).unwrap();
        assert_eq!(l.generated_at.as_deref(), Some("2026-10-03T15:00:00Z"));
        let z = l.coin("zcash", "Zcash");
        assert_eq!(z.src.as_deref(), Some("/static/logos/coins/zcash.abc.svg"));
        assert_eq!(z.kind, "official");
        let h = l.pool("zcash:himpool.com:99", "himpool.com");
        assert!(h.tile && h.src.as_deref() == Some("/static/logos/pools/himpool.com.def.webp"));
        // Unsafe, escaping, misplaced, missing and mislabelled files all become monograms.
        for id in ["evil", "escape", "wrongdir", "missing", "renamed"] {
            let lg = l.coin(id, "Some Coin");
            assert!(lg.is_fallback(), "{id} should fall back");
            assert_eq!(lg.kind, "fallback");
            assert_eq!(lg.mono, "SC");
        }
        assert!(
            l.pool("zcash:fake.com:1", "Fake").is_fallback(),
            "HTML saved as .png is refused"
        );
        assert_eq!(l.rejected.len(), 6);
        // A fallback entry keeps its own monogram; an unknown id gets one from its name.
        assert_eq!(l.coin("fb", "Squishy Coin").mono, "SQ");
        let unknown = l.pool("zero:newpool.io:1", "newpool.io");
        assert!(unknown.is_fallback() && unknown.mono == "NP");
    }

    #[test]
    fn missing_logos_json_means_monograms_and_broken_is_an_error() {
        let t = Tmp::new("missing");
        let l = load_from(&t.data(), &t.stat()).unwrap();
        assert!(l.coins.is_empty() && l.coin("zcash", "Zcash").is_fallback());
        t.json(serde_json::json!({}));
        std::fs::write(t.data().join("curated/logos.json"), "{ nope").unwrap();
        assert!(
            load_from(&t.data(), &t.stat()).is_err(),
            "a broken file keeps the last good data"
        );
    }

    #[test]
    fn svg_safety_check() {
        assert!(svg_is_safe(std::str::from_utf8(SVG).unwrap()));
        assert!(svg_is_safe(
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><defs><linearGradient id="a"/></defs><path fill="url(#a)" d="M0 0"/><use href="#a"/></svg>"##
        ));
        for bad in [
            r#"<svg><script>alert(1)</script></svg>"#,
            r#"<svg onload="x()"><path d="M0 0"/></svg>"#,
            r#"<svg><path d="M0 0" ONCLICK = "x()"/></svg>"#,
            r#"<svg><g/onmouseover="x()"/></svg>"#,
            r#"<svg><foreignObject><div/></foreignObject></svg>"#,
            r#"<svg><image href="https://evil.example/x.png"/></svg>"#,
            r#"<svg><use href="https://evil.example/s.svg#a"/></svg>"#,
            r#"<svg><use xlink:href="data:image/svg+xml,..."/></svg>"#,
            r#"<svg><path fill="url(https://evil.example/f)" d="M0 0"/></svg>"#,
            r#"<svg><path style="fill:url( 'http://x/y' )" d="M0 0"/></svg>"#,
            r#"<svg><style>@import url(x.css);</style></svg>"#,
            r#"<svg><a href="javascript:alert(1)"><path d="M0 0"/></a></svg>"#,
            r#"<svg><animate attributeName="href" to="javascript:x"/></svg>"#,
            r#"<?xml version="1.0"?><!DOCTYPE svg [<!ENTITY x "y">]><svg/>"#,
            r#"<html><svg/></html>"#,
        ] {
            assert!(!svg_is_safe(bad), "accepted: {bad}");
        }
    }

    #[test]
    fn sniff_uses_bytes_not_names() {
        assert_eq!(sniff(SVG), Some("svg"));
        assert_eq!(sniff(&png()), Some("png"));
        assert_eq!(sniff(&webp()), Some("webp"));
        assert_eq!(sniff(b"GIF89a......"), None);
        assert_eq!(sniff(b"<!doctype html><svg>"), None);
    }

    #[test]
    fn monograms() {
        for (name, m) in [
            ("himpool.com (solo)", "HP"),
            ("ViaBTC", "VB"),
            ("zpool", "ZP"),
            ("Mining-Dutch", "MD"),
            ("2Miners", "2M"),
            ("pool.excc.co", "E"),
            ("195.3.222.105", "IP"),
            ("rockpool.cloud", "RP"),
            ("Binance Pool", "BP"),
            ("F2Pool", "FP"),
            ("coolmine.top", "CM"),
            ("Pirate Chain", "PC"),
            ("Zcash", "Z"),
        ] {
            assert_eq!(monogram(name), m, "{name}");
        }
    }

    #[test]
    fn chips_have_size_alt_and_lazy_loading() {
        let l = Logo {
            src: Some("/static/logos/coins/zcash.abc.svg".into()),
            kind: "official".into(),
            mono: "Z".into(),
            tile: false,
            light: false,
        };
        let row = chip(&l, "Zcash", At::Row, true).into_string();
        assert!(
            row.contains(r#"width="16" height="16""#)
                && row.contains(r#"alt="""#)
                && row.contains(r#"loading="lazy""#),
            "{row}"
        );
        let head = chip(&l, "Zcash", At::Head, false).into_string();
        assert!(
            head.contains(r#"alt="Zcash logo""#)
                && head.contains(r#"width="30""#)
                && !head.contains("loading="),
            "{head}"
        );
        let fb = chip(&resolve(None, "himpool.com"), "himpool.com", At::Row, false).into_string();
        assert!(
            fb.contains("mono")
                && fb.contains(">HP<")
                && fb.contains(r#"data-kind="fallback""#)
                && !fb.contains("<img"),
            "{fb}"
        );
        // Names are escaped in alt text.
        assert!(chip(&l, "<b>", At::Head, false)
            .into_string()
            .contains("&lt;b&gt; logo"));
    }

    #[test]
    fn real_logos_json_files_all_pass() {
        // Every file the fetch script wrote is safe to serve, and every listed coin has an entry.
        let l = load_from(Path::new("data"), Path::new("static")).unwrap();
        if l.coins.is_empty() {
            return; // no logos.json in this checkout
        }
        assert!(l.rejected.is_empty(), "rejected: {:?}", l.rejected);
        let d = crate::data::load(Path::new("data")).unwrap();
        for c in &d.coins {
            assert!(
                l.coins.contains_key(&c.id),
                "no logos.json entry for coin {}",
                c.id
            );
        }
        for p in &d.pools {
            assert!(
                l.pools.contains_key(&p.id),
                "no logos.json entry for pool {}",
                p.id
            );
        }
    }
}
