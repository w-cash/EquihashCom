//! Live sources: small public JSON endpoints polled server-side (data/curated/live-sources.json).
//!
//! Each source feeds one figure: a pool row's hashrate (`target: "pool"`) or a coin's network
//! hashrate (`target: "network"`). A reading is used only when the endpoint says `available: true`;
//! anything else keeps the previous good reading, whose age stays visible. Adding a pool with a
//! similar API is a JSON edit: point `url` at it and map its field names in `fields`.

use crate::data::{Coin, Pool};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const CONFIG: &str = "live-sources.json";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Fields {
    pub available: String,
    pub hashrate: String,
    pub updated_at: String,
    pub window_seconds: String,
    pub sample_blocks: String,
    pub height: String,
}
impl Default for Fields {
    fn default() -> Self {
        Fields {
            available: "available".into(),
            hashrate: "hashrate_sol_s".into(),
            updated_at: "updated_at".into(),
            window_seconds: "window_seconds".into(),
            sample_blocks: "sample_blocks".into(),
            height: "height".into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Source {
    pub id: String,
    /// "pool" or "network".
    pub target: String,
    pub coin_id: Option<String>,
    pub pool_id: Option<String>,
    /// Several pool rows fed by one endpoint (a merge-mining pool whose one hashrate covers every
    /// chain it mines, e.g. ZecWec's ZEC and WEC rows). One reading, so the rows always agree.
    pub pool_ids: Vec<String>,
    pub url: String,
    pub label: Option<String>,
    /// Hashrate unit of the endpoint's figure.
    pub unit: Option<String>,
    pub fields: Fields,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    /// Poll interval, clamped to 30–60 s.
    pub poll_seconds: u64,
    /// A reading older than this is marked stale on the page.
    pub stale_after_seconds: i64,
    pub sources: Vec<Source>,
}
impl Default for Config {
    fn default() -> Self {
        Config { poll_seconds: 45, stale_after_seconds: 600, sources: vec![] }
    }
}
impl Source {
    /// Every pool row this source feeds: `pool_ids`, plus `pool_id` for older configs.
    pub fn pool_ids(&self) -> Vec<String> {
        let mut v = self.pool_ids.clone();
        if let Some(p) = &self.pool_id {
            if !v.contains(p) {
                v.push(p.clone());
            }
        }
        v
    }
    pub fn feeds_pool(&self, id: &str) -> bool {
        self.target == "pool" && self.pool_ids().iter().any(|p| p == id)
    }
}

impl Config {
    pub fn interval(&self) -> u64 {
        self.poll_seconds.clamp(30, 60)
    }
}

/// Missing file = no live sources; a broken file is an error (the last good data keeps serving).
pub fn read_config(dir: &Path) -> Result<Config, String> {
    let p = dir.join("curated").join(CONFIG);
    match std::fs::read_to_string(&p) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", p.display())),
        Err(_) => Ok(Config::default()),
    }
}

/// One good reading.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Reading {
    pub hashrate: f64,
    /// When the endpoint computed the figure (its `updated_at`), ISO 8601 UTC.
    pub observed_at: String,
    pub window_seconds: Option<u64>,
    pub sample_blocks: Option<u64>,
    pub height: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Parsed {
    Ok(Reading),
    /// The endpoint answered but said `available: false` (or didn't say true).
    Unavailable,
}

fn uint(v: &serde_json::Value, k: &str) -> Option<u64> {
    v.get(k).and_then(|x| x.as_u64().or_else(|| x.as_f64().filter(|f| f.is_finite() && *f >= 0.0).map(|f| f as u64)))
}

/// Parse one response. Only `available: true` with a finite, non-negative hashrate and a sane
/// `updated_at` (unix seconds, not in the future) counts as a reading.
pub fn parse(src: &Source, body: &serde_json::Value, now: chrono::DateTime<chrono::Utc>) -> Result<Parsed, String> {
    let f = &src.fields;
    if body.get(&f.available).and_then(|x| x.as_bool()) != Some(true) {
        return Ok(Parsed::Unavailable);
    }
    let h = body.get(&f.hashrate).and_then(|x| x.as_f64()).filter(|h| h.is_finite() && *h >= 0.0).ok_or_else(|| format!("{}: no valid {}", src.id, f.hashrate))?;
    let ts = body.get(&f.updated_at).and_then(|x| x.as_i64()).ok_or_else(|| format!("{}: no valid {}", src.id, f.updated_at))?;
    let at = chrono::DateTime::from_timestamp(ts, 0).ok_or_else(|| format!("{}: bad {}", src.id, f.updated_at))?;
    if at > now + chrono::Duration::minutes(5) {
        return Err(format!("{}: {} is in the future", src.id, f.updated_at));
    }
    Ok(Parsed::Ok(Reading {
        hashrate: h,
        observed_at: at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        window_seconds: uint(body, &f.window_seconds),
        sample_blocks: uint(body, &f.sample_blocks),
        height: uint(body, &f.height),
    }))
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct SourceState {
    /// Last good reading; kept through unavailable answers and errors.
    pub last_good: Option<Reading>,
    /// "ok" | "unavailable" | "error" | "pending".
    pub status: String,
    pub error: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_ok_at: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LiveState {
    pub sources: BTreeMap<String, SourceState>,
}

impl LiveState {
    /// Record one poll. Returns true when the last good reading changed (the page needs a rebuild).
    pub fn record(&mut self, id: &str, result: Result<Parsed, String>, now: chrono::DateTime<chrono::Utc>) -> bool {
        let st = self.sources.entry(id.to_string()).or_default();
        let t = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        st.last_attempt_at = Some(t.clone());
        match result {
            Ok(Parsed::Ok(r)) => {
                let changed = st.last_good.as_ref() != Some(&r);
                st.last_good = Some(r);
                st.status = "ok".into();
                st.error = None;
                st.last_ok_at = Some(t);
                changed
            }
            Ok(Parsed::Unavailable) => {
                st.status = "unavailable".into();
                st.error = Some("endpoint reports available: false".into());
                false
            }
            Err(e) => {
                st.status = "error".into();
                st.error = Some(e);
                false
            }
        }
    }
}

fn newer(reading: &str, current: Option<&str>) -> bool {
    match current.and_then(|c| chrono::DateTime::parse_from_rfc3339(c).ok()) {
        None => true,
        Some(c) => chrono::DateTime::parse_from_rfc3339(reading).map(|r| r >= c).unwrap_or(false),
    }
}

/// Lay the last good readings over the loaded rows. A reading only replaces a figure when it is at
/// least as new as what the files already hold (the refresh script writes the same endpoints).
pub fn apply(cfg: &Config, state: &LiveState, pools: &mut [Pool], coins: &mut [Coin]) {
    for src in &cfg.sources {
        let Some(r) = state.sources.get(&src.id).and_then(|s| s.last_good.as_ref()) else { continue };
        match src.target.as_str() {
            "pool" => {
                // One reading for every row the source feeds: the poller's, unless the files
                // already hold a newer one from the same endpoint.
                let best = file_reading(src, pools).filter(|f| !newer(&r.observed_at, Some(&f.observed_at))).unwrap_or_else(|| r.clone());
                for p in pools.iter_mut().filter(|p| src.feeds_pool(&p.id)) {
                    set_pool(p, src, &best);
                }
            }
            "network" => {
                for c in coins.iter_mut().filter(|c| Some(&c.id) == src.coin_id.as_ref()) {
                    if c.network.live_source.is_some() && !newer(&r.observed_at, c.network.hashrate_observed_at.as_deref()) {
                        continue;
                    }
                    set_network(c, src, r);
                }
            }
            _ => {}
        }
    }
}

/// The newest reading from this endpoint that the files hold on any of the source's rows.
fn file_reading(src: &Source, pools: &[Pool]) -> Option<Reading> {
    pools
        .iter()
        .filter(|p| src.feeds_pool(&p.id) && p.basis.as_deref() == Some("pool_api"))
        .filter(|p| p.live_source.as_deref().map(|l| l == src.id).unwrap_or(true) || p.hashrate_source.as_deref() == Some(src.url.as_str()))
        .filter_map(|p| Some(Reading { hashrate: p.hashrate?, observed_at: p.hashrate_observed_at.clone()?, window_seconds: p.hashrate_window_s, sample_blocks: None, height: None }))
        .filter(|r| chrono::DateTime::parse_from_rfc3339(&r.observed_at).is_ok())
        .max_by_key(|r| chrono::DateTime::parse_from_rfc3339(&r.observed_at).unwrap())
}

/// Rows fed by one pool source always show one figure. Runs on every load, with or without a
/// poller reading: every fed row takes the newest reading from that endpoint found in the files.
/// When none of them has one yet, the rows show n/a rather than an older figure of another kind
/// (e.g. a hand-entered operator figure), so the two can never disagree.
pub fn unify(cfg: &Config, pools: &mut [Pool]) {
    for src in cfg.sources.iter().filter(|s| s.target == "pool" && s.pool_ids().len() > 1) {
        match file_reading(src, pools) {
            Some(r) => {
                for p in pools.iter_mut().filter(|p| src.feeds_pool(&p.id)) {
                    set_pool(p, src, &r);
                }
            }
            None => {
                for p in pools.iter_mut().filter(|p| src.feeds_pool(&p.id)) {
                    p.hashrate = None;
                    p.hashrate_is_reported = Some(false);
                    p.basis = None;
                    p.hashrate_source = Some(src.url.clone());
                    p.hashrate_observed_at = None;
                    p.hashrate_window_s = None;
                    p.live_source = Some(src.id.clone());
                }
            }
        }
    }
}

pub fn set_pool(p: &mut Pool, src: &Source, r: &Reading) {
    p.hashrate = Some(r.hashrate);
    p.hashrate_unit = Some(src.unit.clone().unwrap_or("Sol/s".into()));
    p.hashrate_is_reported = Some(false);
    p.basis = Some("pool_api".into());
    p.hashrate_source = Some(src.url.clone());
    p.hashrate_observed_at = Some(r.observed_at.clone());
    p.hashrate_window_s = r.window_seconds;
    p.live_source = Some(src.id.clone());
}

pub fn set_network(c: &mut Coin, src: &Source, r: &Reading) {
    let n = &mut c.network;
    n.hashrate = Some(r.hashrate);
    n.unit = Some(src.unit.clone().unwrap_or("Sol/s".into()));
    n.basis = Some("network".into());
    n.hashrate_source = Some(src.url.clone());
    n.hashrate_upstream = None;
    n.hashrate_observed_at = Some(r.observed_at.clone());
    n.hashrate_sample_blocks = r.sample_blocks;
    n.live_source = Some(src.id.clone());
    // Height stays credited to the chain source (the explorer) when it has one: the network
    // hashrate endpoint only fills a height nobody else supplied, so each figure keeps one source.
    if let Some(h) = r.height {
        if n.height.is_none() && newer(&r.observed_at, n.height_observed_at.as_deref()) {
            n.height = Some(h as f64);
            n.height_source = Some(src.url.clone());
            n.height_observed_at = Some(r.observed_at.clone());
        }
    }
}

/// What /api/live and the page show about each poller.
#[derive(Debug, Clone, Serialize, Default)]
pub struct LiveStatus {
    pub id: String,
    pub target: String,
    pub coin_id: Option<String>,
    pub pool_id: Option<String>,
    pub pool_ids: Vec<String>,
    pub url: String,
    pub label: Option<String>,
    pub status: String,
    pub error: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_ok_at: Option<String>,
    pub reading: Option<Reading>,
    pub age_secs: Option<i64>,
    pub stale: bool,
    pub poll_seconds: u64,
}

pub fn statuses(cfg: &Config, state: Option<&LiveState>, now: chrono::DateTime<chrono::Utc>) -> Vec<LiveStatus> {
    cfg.sources
        .iter()
        .map(|s| {
            let st = state.and_then(|x| x.sources.get(&s.id)).cloned().unwrap_or_default();
            let age = st.last_good.as_ref().and_then(|r| crate::data::age_secs(Some(&r.observed_at), now));
            LiveStatus {
                id: s.id.clone(),
                target: s.target.clone(),
                coin_id: s.coin_id.clone(),
                pool_id: s.pool_ids().first().cloned(),
                pool_ids: if s.target == "pool" { s.pool_ids() } else { vec![] },
                url: s.url.clone(),
                label: s.label.clone(),
                status: if st.status.is_empty() { "pending".into() } else { st.status },
                error: st.error,
                last_attempt_at: st.last_attempt_at,
                last_ok_at: st.last_ok_at,
                stale: age.map(|a| a > cfg.stale_after_seconds).unwrap_or(true),
                reading: st.last_good,
                age_secs: age,
                poll_seconds: cfg.interval(),
            }
        })
        .collect()
}

/// One poll of one source (server-side; browsers never call the endpoint).
pub async fn fetch(client: &reqwest::Client, src: &Source) -> Result<Parsed, String> {
    let resp = client.get(&src.url).send().await.map_err(|e| format!("{}: {e}", src.id))?;
    if !resp.status().is_success() {
        return Err(format!("{}: HTTP {}", src.id, resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| format!("{}: {e}", src.id))?;
    parse(src, &body, chrono::Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn src() -> Source {
        Source { id: "zecwec-pool".into(), target: "pool".into(), pool_id: Some("wcash:zecwec.com".into()), url: "https://pool.zecwec.com/api/v1/hashrate/pool".into(), ..Default::default() }
    }
    fn now() -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::from_timestamp(1_791_025_800, 0).unwrap()
    }

    #[test]
    fn parses_an_available_reading() {
        let p = parse(&src(), &json!({"available": true, "hashrate_sol_s": 553587, "window_seconds": 1200, "updated_at": 1_791_025_763}), now()).unwrap();
        let Parsed::Ok(r) = p else { panic!("{p:?}") };
        assert_eq!(r.hashrate, 553587.0);
        assert_eq!(r.window_seconds, Some(1200));
        assert_eq!(r.observed_at, "2026-10-03T11:09:23Z");
    }

    #[test]
    fn available_false_or_missing_is_not_a_reading() {
        for body in [
            json!({"available": false, "hashrate_sol_s": 999, "updated_at": 1_791_025_763}),
            json!({"hashrate_sol_s": 999, "updated_at": 1_791_025_763}),
            json!({"available": "true", "hashrate_sol_s": 999, "updated_at": 1_791_025_763}),
        ] {
            assert_eq!(parse(&src(), &body, now()).unwrap(), Parsed::Unavailable, "{body}");
        }
        assert!(parse(&src(), &json!({"available": true, "hashrate_sol_s": -1, "updated_at": 1}), now()).is_err());
        assert!(parse(&src(), &json!({"available": true, "hashrate_sol_s": 5}), now()).is_err());
        assert!(parse(&src(), &json!({"available": true, "hashrate_sol_s": 5, "updated_at": 1_891_025_763}), now()).is_err(), "future timestamp");
    }

    #[test]
    fn keeps_the_last_good_value() {
        let mut st = LiveState::default();
        let good = parse(&src(), &json!({"available": true, "hashrate_sol_s": 553587, "window_seconds": 1200, "updated_at": 1_791_025_763}), now()).unwrap();
        assert!(st.record("a", Ok(good.clone()), now()));
        assert!(!st.record("a", Ok(good), now()), "same reading: nothing to rebuild");
        assert!(!st.record("a", Ok(Parsed::Unavailable), now()));
        assert_eq!(st.sources["a"].status, "unavailable");
        assert_eq!(st.sources["a"].last_good.as_ref().unwrap().hashrate, 553587.0, "kept through available:false");
        assert!(!st.record("a", Err("timeout".into()), now()));
        assert_eq!(st.sources["a"].status, "error");
        assert_eq!(st.sources["a"].last_good.as_ref().unwrap().hashrate, 553587.0, "kept through errors");
        // Its age keeps growing and becomes visible as stale.
        let cfg = Config { sources: vec![Source { id: "a".into(), ..src() }], ..Default::default() };
        let later = now() + chrono::Duration::minutes(30);
        let s = &statuses(&cfg, Some(&st), later)[0];
        assert!(s.stale && s.age_secs.unwrap() > 1800 && s.reading.is_some());
    }

    #[test]
    fn apply_sets_basis_source_and_time_and_never_goes_backwards() {
        let cfg = Config { sources: vec![src()], ..Default::default() };
        let mut st = LiveState::default();
        st.record("zecwec-pool", Ok(Parsed::Ok(Reading { hashrate: 553587.0, observed_at: "2026-10-03T11:09:23Z".into(), window_seconds: Some(1200), sample_blocks: None, height: None })), now());
        let mut pools = vec![Pool { id: "wcash:zecwec.com".into(), hashrate: Some(440000.0), basis: Some("operator_reported".into()), hashrate_is_reported: Some(true), ..Default::default() }];
        apply(&cfg, &st, &mut pools, &mut []);
        let p = &pools[0];
        assert_eq!(p.hashrate, Some(553587.0));
        assert_eq!(p.basis.as_deref(), Some("pool_api"));
        assert!(!p.operator_reported());
        assert_eq!(p.hashrate_source.as_deref(), Some("https://pool.zecwec.com/api/v1/hashrate/pool"));
        assert_eq!(p.hashrate_observed_at.as_deref(), Some("2026-10-03T11:09:23Z"));
        assert_eq!(p.hashrate_window_s, Some(1200));
        // A file value that is newer than the poller's last reading is kept.
        let mut pools = vec![Pool { id: "wcash:zecwec.com".into(), hashrate: Some(1.0), basis: Some("pool_api".into()), hashrate_observed_at: Some("2026-10-03T12:00:00Z".into()), ..Default::default() }];
        apply(&cfg, &st, &mut pools, &mut []);
        assert_eq!(pools[0].hashrate, Some(1.0));
    }
}
