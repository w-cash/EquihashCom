//! Typed views over data/*.json. Everything optional is `Option`, rendered as "n/a".
//! `None` (not published / not known) and `Some(0.0)` (published as zero) are never conflated.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Scheme {
    pub scheme: String,
    pub fee_pct: Option<f64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct MergedMining {
    pub supported: Option<bool>,
    pub coins: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Pool {
    pub id: String,
    pub name: String,
    pub coin: String,
    pub coin_id: String,
    pub url: Option<String>,
    pub hashrate: Option<f64>,
    pub hashrate_unit: Option<String>,
    pub hashrate_is_reported: Option<bool>,
    pub network_share_pct: Option<f64>,
    pub miners: Option<f64>,
    pub workers: Option<f64>,
    pub fee_pct: Option<f64>,
    pub schemes: Vec<Scheme>,
    pub payout_schemes: Vec<String>,
    pub min_payout: Option<f64>,
    pub min_payout_unit: Option<String>,
    pub blocks_last_1000: Option<f64>,
    pub last_block_height: Option<f64>,
    pub last_block_time: Option<String>,
    pub region: Option<String>,
    pub regions: Vec<String>,
    pub country_code: Option<String>,
    pub merged_mining: MergedMining,
    pub solo: bool,
    pub active: bool,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub data_url: Option<String>,
    pub from_miningpoolstats: bool,
    pub fetched_at: Option<String>,
    pub notes: Option<String>,
    // Per-field provenance. `<field>_observed_at` only moves when that field was fetched.
    pub hashrate_source: Option<String>,
    pub hashrate_observed_at: Option<String>,
    /// Basis of the hashrate figure: "listed_pools" (the pool's own public stats, directly or via
    /// miningpoolstats), "operator_reported" (the operator told us) or "estimated".
    pub basis: Option<String>,
    pub fee_source: Option<String>,
    pub fee_observed_at: Option<String>,
    pub miners_source: Option<String>,
    pub miners_observed_at: Option<String>,
    pub blocks_source: Option<String>,
    pub blocks_observed_at: Option<String>,
    pub min_payout_source: Option<String>,
    pub min_payout_observed_at: Option<String>,
    /// Averaging window of a pool-API hashrate, in seconds (ZecWec: accepted work over 1200 s).
    pub hashrate_window_s: Option<u64>,
    /// Id of the data/curated/live-sources.json entry that feeds this row's hashrate, if any.
    pub live_source: Option<String>,
    // derived at load time
    /// Why the share cell doesn't show a plain percentage (e.g. the only listed pool reads above a
    /// network estimate taken over a different window).
    #[serde(skip_deserializing)]
    pub share_note: Option<String>,
    /// True when pool ÷ network came out above 100% and was capped.
    #[serde(skip_deserializing)]
    pub share_capped: bool,
    /// Verified social/community links for this pool (data/curated/links.json).
    #[serde(skip_deserializing)]
    pub links: Vec<SocialLink>,
    /// Logo or monogram (data/curated/logos.json, see src/views/logo.rs).
    #[serde(skip_deserializing)]
    pub logo: crate::views::logo::Logo,
    #[serde(skip_deserializing)]
    pub slug: String,
    #[serde(skip_deserializing)]
    pub region_tags: Vec<String>,
    #[serde(skip_deserializing)]
    pub coin_label: String,
    /// "network" | "pools" | "none" (copied from the coin; see `Coin::share_basis`).
    #[serde(skip_deserializing)]
    pub share_basis: String,
    /// Above 30% and meaningful (not just the only pool on a coin without a network figure).
    #[serde(skip_deserializing)]
    pub share_flag: bool,
    /// What the share figure is measured against; see `share_status`.
    #[serde(skip_deserializing)]
    pub share_status: String,
    /// The slug computed from coin, name and domain (the pre-permalink URL). Redirects to `slug`.
    #[serde(skip)]
    pub legacy_slug: String,
}

impl Pool {
    /// Lowest and highest fee across schemes / headline fee.
    pub fn fee_range(&self) -> Option<(f64, f64)> {
        let mut v: Vec<f64> = self.schemes.iter().filter_map(|s| s.fee_pct).collect();
        if let Some(f) = self.fee_pct {
            v.push(f);
        }
        if v.is_empty() {
            return None;
        }
        let lo = v.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = v.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        Some((lo, hi))
    }
    pub fn min_fee(&self) -> Option<f64> {
        self.fee_range().map(|r| r.0)
    }
    pub fn merged(&self) -> bool {
        self.merged_mining.supported == Some(true)
    }
    pub fn operator_reported(&self) -> bool {
        self.basis.as_deref() == Some("operator_reported") || self.hashrate_is_reported == Some(true)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Equihash {
    pub n: Option<u32>,
    pub k: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct NetworkStats {
    pub hashrate: Option<f64>,
    pub unit: Option<String>,
    pub hashrate_source: Option<String>,
    /// Where the source we read says its estimate comes from (miningpoolstats' `hashrate_src`).
    /// `hashrate_source` is always the file the number was read from.
    pub hashrate_upstream: Option<String>,
    pub difficulty: Option<f64>,
    pub height: Option<f64>,
    pub block_time_target_s: Option<f64>,
    pub block_time_avg_s: Option<f64>,
    pub pools_hashrate: Option<f64>,
    pub hashrate_observed_at: Option<String>,
    /// Basis of `hashrate`: "network" (a node, explorer or miningpoolstats network estimate) or
    /// "estimated". Never the listed pools' total: that is `Coin::reported`, kept separate.
    pub basis: Option<String>,
    pub difficulty_source: Option<String>,
    pub difficulty_observed_at: Option<String>,
    pub height_source: Option<String>,
    pub height_observed_at: Option<String>,
    /// Where the average block time was read (each figure carries its own source).
    pub block_time_source: Option<String>,
    pub block_time_observed_at: Option<String>,
    /// Blocks the network estimate is averaged over, when the source says (ZecWec: 120).
    pub hashrate_sample_blocks: Option<u64>,
    /// Id of the live source that feeds the network hashrate, if any.
    pub live_source: Option<String>,
}

/// What the listed pools report for one coin, kept apart from the network estimate.
#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Reported {
    /// Sum of positive pool hashrates. `None` when no row has a number; `Some(0.0)` when rows
    /// publish only zero.
    pub hashrate: Option<f64>,
    /// Rows with a positive hashrate.
    pub positive_pools: u32,
    /// Of those, rows whose hashrate is operator-reported (marked †).
    pub operator_pools: u32,
    /// Oldest hashrate_observed_at among the operator-reported rows.
    pub operator_observed_at: Option<String>,
    /// Newest hashrate_observed_at among rows with a number.
    pub observed_at: Option<String>,
}

/// One social or community link from data/curated/links.json.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[serde(default)]
pub struct SocialLink {
    pub kind: String,
    pub url: String,
    pub label: String,
    pub source_url: Option<String>,
    pub verified_at: Option<String>,
    pub status: String,
    /// Optional curator's note (e.g. why a manual override was chosen). Accepted, never rendered
    /// or sent to the browser.
    #[serde(skip_serializing)]
    pub note: Option<String>,
}

/// Link kinds in display order, with the visible label.
pub const LINK_KINDS: &[(&str, &str)] = &[
    ("website", "Website"),
    ("explorer", "Explorer"),
    ("docs", "Docs"),
    ("github", "GitHub"),
    ("forum", "Forum"),
    ("x", "X"),
    ("discord", "Discord"),
    ("telegram", "Telegram"),
    ("reddit", "Reddit"),
    ("status", "Status"),
    ("support", "Support"),
];

pub fn link_kind_label(kind: &str) -> Option<&'static str> {
    LINK_KINDS.iter().find(|(k, _)| *k == kind).map(|(_, l)| *l)
}

/// Links that may be shown: status "verified", a known kind and an http(s) URL; duplicates
/// dropped; ordered by `LINK_KINDS`. Unverified, dead and malformed entries never render.
pub fn verified_links(v: &[SocialLink]) -> Vec<SocialLink> {
    let mut out: Vec<SocialLink> = Vec::new();
    for l in v {
        let kind = l.kind.trim().to_ascii_lowercase();
        let url = l.url.trim();
        if l.status.trim() != "verified" || link_kind_label(&kind).is_none() {
            continue;
        }
        if safe_url(url).is_none() {
            continue;
        }
        if out.iter().any(|o| o.url == url) {
            continue;
        }
        out.push(SocialLink { kind, url: url.to_string(), ..l.clone() });
    }
    let pos = |k: &str| LINK_KINDS.iter().position(|(x, _)| *x == k).unwrap_or(usize::MAX);
    out.sort_by_key(|l| pos(&l.kind));
    out
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct LinksFile {
    pub generated_at: Option<String>,
    pub coins: BTreeMap<String, Vec<SocialLink>>,
    pub pools: BTreeMap<String, Vec<SocialLink>>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Link {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Reward {
    pub value: f64,
    pub unit: String,
    pub note: Option<String>,
    pub source_url: Option<String>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct CrossCheck {
    pub label: String,
    pub url: String,
    pub network_hashrate: Option<f64>,
    pub difficulty: Option<f64>,
    pub height: Option<f64>,
    pub avg_block_time_s: Option<f64>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Coin {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub algo_label: Option<String>,
    pub equihash: Option<Equihash>,
    pub z15_compatible: Option<bool>,
    pub hardware: Option<String>,
    pub mps_group: Option<String>,
    pub in_mps_index: bool,
    pub network: NetworkStats,
    pub price_usd: Option<f64>,
    pub price_source: Option<String>,
    pub block_reward_miner: Option<Reward>,
    pub mps_pool_count: u32,
    pub source_url: Option<String>,
    pub data_url: Option<String>,
    pub fetched_at: Option<String>,
    pub status: String,
    pub status_note: Option<String>,
    pub status_sources: Vec<Link>,
    pub cross_checks: Vec<CrossCheck>,
    pub pool_count: u32,
    pub merged_mining_parent: Option<String>,
    #[serde(skip_deserializing)]
    pub label: String,
    /// What `network_share_pct` is measured against: "network" (published network hashrate)
    /// or "pools" (sum of listed pools, used when the network estimate is below that sum).
    #[serde(skip_deserializing)]
    pub share_basis: String,
    #[serde(skip_deserializing)]
    pub share_denominator: Option<f64>,
    /// What the listed pools report (separate from `network.hashrate`).
    #[serde(skip_deserializing)]
    pub reported: Reported,
    #[serde(skip_deserializing)]
    pub links: Vec<SocialLink>,
    /// Logo or monogram (data/curated/logos.json, see src/views/logo.rs).
    #[serde(skip_deserializing)]
    pub logo: crate::views::logo::Logo,
}

/// A network estimate and a listed-pool total this far apart, in either direction, get a visible
/// data-quality warning: normal window and coverage differences don't explain a gap this size.
pub const DISCREPANCY_RATIO: f64 = 100.0;

impl Coin {
    /// How many times larger the bigger of (network estimate, listed-pool total) is than the
    /// smaller, when that factor reaches `DISCREPANCY_RATIO`. Needs both figures, positive.
    pub fn discrepancy(&self) -> Option<f64> {
        let n = self.network.hashrate.filter(|h| h.is_finite() && *h > 0.0)?;
        let r = self.reported.hashrate.filter(|h| h.is_finite() && *h > 0.0)?;
        let f = if n > r { n / r } else { r / n };
        (f >= DISCREPANCY_RATIO).then_some(f)
    }
    pub fn params(&self) -> String {
        match &self.equihash {
            Some(Equihash { n: Some(n), k: Some(k) }) => format!("{n},{k}"),
            _ => "n/a".into(),
        }
    }
    pub fn active(&self) -> bool {
        self.status == "active"
    }
    /// Exact Equihash parameter set, or None when the source doesn't publish it.
    pub fn nk(&self) -> Option<(u32, u32)> {
        match &self.equihash {
            Some(Equihash { n: Some(n), k: Some(k) }) => Some((*n, *k)),
            _ => None,
        }
    }
    /// Group order: 200,9 first (the Z15 family), then the other exact sets by n,k, unknown last.
    pub fn param_key(&self) -> (u8, u32, u32) {
        match self.nk() {
            Some((200, 9)) => (0, 200, 9),
            Some((n, k)) => (1, n, k),
            None => (2, 0, 0),
        }
    }
    pub fn group_label(&self) -> String {
        match self.nk() {
            Some((n, k)) => format!("Equihash {n},{k}"),
            None => "Parameters not published".into(),
        }
    }
}

/// Rank coins: grouped by exact (n,k) parameter set; within a group by the sum of positive
/// pool-reported hashrate, descending; zero and unavailable last; coin name, then id, as the
/// tie-break. Hashrates are never compared across parameter sets.
pub fn rank_cmp(a: &Coin, b: &Coin) -> std::cmp::Ordering {
    let pos = |c: &Coin| c.reported.hashrate.filter(|h| *h > 0.0);
    a.param_key()
        .cmp(&b.param_key())
        .then_with(|| match (pos(a), pos(b)) {
            (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        })
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        .then_with(|| a.id.cmp(&b.id))
}

/// Sum what the listed pools report for one coin.
pub fn reported_for(coin_id: &str, pools: &[Pool]) -> Reported {
    let own: Vec<&Pool> = pools.iter().filter(|p| p.coin_id == coin_id && p.hashrate.map(f64::is_finite).unwrap_or(false)).collect();
    if own.is_empty() {
        return Reported::default();
    }
    let positive: Vec<&&Pool> = own.iter().filter(|p| p.hashrate.unwrap_or(0.0) > 0.0).collect();
    let operator: Vec<&&&Pool> = positive.iter().filter(|p| p.operator_reported()).collect();
    Reported {
        hashrate: Some(positive.iter().filter_map(|p| p.hashrate).sum()),
        positive_pools: positive.len() as u32,
        operator_pools: operator.len() as u32,
        operator_observed_at: operator.iter().filter_map(|p| p.hashrate_observed_at.clone()).min(),
        observed_at: own.iter().filter_map(|p| p.hashrate_observed_at.clone()).max(),
    }
}

/// Fill per-field provenance on rows written before it existed: each field falls back to the row's
/// own source and fetched_at. Rows that already carry provenance are left alone.
pub fn fill_provenance(p: &mut Pool) {
    let src = p.data_url.clone().or(p.source_url.clone());
    let at = p.fetched_at.clone();
    for (s, o) in [
        (&mut p.hashrate_source, &mut p.hashrate_observed_at),
        (&mut p.fee_source, &mut p.fee_observed_at),
        (&mut p.miners_source, &mut p.miners_observed_at),
        (&mut p.blocks_source, &mut p.blocks_observed_at),
        (&mut p.min_payout_source, &mut p.min_payout_observed_at),
    ] {
        if s.is_none() && o.is_none() {
            *s = src.clone();
            *o = at.clone();
        }
    }
    if p.basis.is_none() && p.hashrate.is_some() {
        p.basis = Some(if p.hashrate_is_reported == Some(true) { "operator_reported" } else { "listed_pools" }.into());
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ArchivePool {
    pub id: String,
    pub name: String,
    pub operator: Option<String>,
    pub coin: String,
    pub url: Option<String>,
    pub retired: Option<String>,
    pub retired_label: Option<String>,
    pub reason: Option<String>,
    pub notes: Option<String>,
    pub sources: Vec<Link>,
    pub fetched_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Miner {
    pub id: String,
    pub maker: String,
    pub model: String,
    pub equihash: String,
    pub hashrate_ksol: Option<f64>,
    pub watts: Option<f64>,
    pub stated_efficiency_j_per_ksol: Option<f64>,
    pub notes: Option<String>,
    pub source_name: Option<String>,
    pub source_url: Option<String>,
    pub fetched_at: Option<String>,
}

impl Miner {
    pub fn efficiency(&self) -> Option<f64> {
        match (self.watts, self.hashrate_ksol) {
            (Some(w), Some(h)) if h > 0.0 => Some(w / h),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct NotListed {
    pub model: String,
    pub reason: String,
}

/// A shop that sells Equihash ASICs (data/curated/vendors.json). equihash.com never sells.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Vendor {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub url: Option<String>,
    pub regions: Vec<String>,
    /// Preferred region for UK-first ordering (e.g. "UK").
    pub region_focus: Option<String>,
    pub region_note: Option<String>,
    pub notes: Option<String>,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
    /// Resolved on load from logos.json vendors map.
    #[serde(skip)]
    pub logo: crate::views::logo::Logo,
    /// Filled on load: how many active listings point at this vendor.
    #[serde(skip)]
    pub listing_count: usize,
}

/// One retail listing (data/curated/listings.json). Price/stock/shipping are observed, never invented.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Listing {
    pub id: String,
    pub vendor_id: String,
    /// Links to data/miners.json id (manufacturer specs stay on /miners).
    pub miner_id: String,
    pub title: String,
    pub product_url: String,
    /// What the shop states (may differ from manufacturer typical).
    pub shop_hashrate_ksol: Option<f64>,
    pub hashrate_note: Option<String>,
    pub price_amount: Option<f64>,
    pub price_currency: Option<String>,
    /// false = ex VAT (as published); missing = not stated.
    pub price_includes_vat: Option<bool>,
    /// "in_stock" | "low_stock" | "preorder" | "out_of_stock" | "unknown"
    pub availability: Option<String>,
    pub availability_label: Option<String>,
    pub shipping_regions: Vec<String>,
    pub shipping_note: Option<String>,
    pub condition: Option<String>,
    /// Path relative to static/, e.g. "shop/machines/….webp".
    pub image: Option<String>,
    pub image_source_url: Option<String>,
    pub image_license_note: Option<String>,
    pub sku: Option<String>,
    pub source_url: Option<String>,
    pub observed_at: Option<String>,
    /// Resolved on load.
    #[serde(skip)]
    pub image_src: Option<String>,
    #[serde(skip)]
    pub vendor_name: String,
    #[serde(skip)]
    pub vendor_slug: String,
    #[serde(skip)]
    pub miner_label: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ResearchItem {
    pub pool: String,
    pub outcome: String,
    pub detail: String,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct ResearchCoin {
    pub coin: String,
    pub detail: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct SourceRef {
    pub id: String,
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Meta {
    pub generated_at: Option<String>,
    pub pool_count: u32,
    pub coin_count: u32,
    pub mps_pool_count: u32,
    pub non_mps_pool_count: u32,
    pub sources: Vec<SourceRef>,
    pub errors: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct PoolsFile {
    generated_at: Option<String>,
    pools: Vec<Pool>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct NetworkFile {
    coins: Vec<Coin>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct ArchiveFile {
    pools: Vec<ArchivePool>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct MinersFile {
    verified_at: Option<String>,
    miners: Vec<Miner>,
    not_listed: Vec<NotListed>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct VendorsFile {
    verified_at: Option<String>,
    vendors: Vec<Vendor>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct ListingsFile {
    verified_at: Option<String>,
    listings: Vec<Listing>,
}
#[derive(Deserialize, Default)]
#[serde(default)]
struct ResearchFile {
    items: Vec<ResearchItem>,
    coins: Vec<ResearchCoin>,
}

#[derive(Debug, Clone, Default)]
pub struct Data {
    pub pools: Vec<Pool>,
    pub coins: Vec<Coin>,
    pub archive: Vec<ArchivePool>,
    pub miners: Vec<Miner>,
    pub miners_verified_at: Option<String>,
    pub miners_not_listed: Vec<NotListed>,
    pub vendors: Vec<Vendor>,
    pub vendors_verified_at: Option<String>,
    pub listings: Vec<Listing>,
    pub listings_verified_at: Option<String>,
    pub research: Vec<ResearchItem>,
    pub research_coins: Vec<ResearchCoin>,
    pub meta: Meta,
    /// Newest fetched_at across pools (what "last updated" shows).
    pub last_updated: Option<String>,
    pub links_generated_at: Option<String>,
    /// Checked logos (data/curated/logos.json); pools and coins also carry their own `logo`.
    pub logos: crate::views::logo::Logos,
    /// Live sources config and the state of each poller (for /api/live and the page).
    pub live: Vec<crate::live::LiveStatus>,
    /// `stale_after_seconds` from live-sources.json: past this a live figure is marked stale.
    pub live_stale_after_secs: i64,
    /// Id of the published snapshot the generated files were read from (None: legacy flat layout).
    pub snapshot: Option<String>,
    /// Directory the generated files (pools, network, meta) were read from.
    pub snapshot_dir: std::path::PathBuf,
    /// When the refresh published that snapshot (from data/current.json).
    pub snapshot_published_at: Option<String>,
    /// Old pool URLs (computed slugs, curated aliases) -> the pool's permanent slug.
    pub slug_redirects: BTreeMap<String, String>,
}

/// Data older than this gets a visible stale notice.
pub const STALE_AFTER_SECS: i64 = 2 * 3600;

/// Age of an RFC 3339 timestamp at `now`, in seconds. None when the timestamp is missing or bad.
pub fn age_secs(ts: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> Option<i64> {
    let t = chrono::DateTime::parse_from_rfc3339(ts?).ok()?;
    Some((now - t.with_timezone(&chrono::Utc)).num_seconds())
}

/// Stale when older than the threshold, or when the age can't be known at all.
pub fn is_stale(ts: Option<&str>, now: chrono::DateTime<chrono::Utc>) -> bool {
    age_secs(ts, now).map(|a| a > STALE_AFTER_SECS).unwrap_or(true)
}

/// Headline counts for the home page, computed from the data.
#[derive(Debug, Clone, PartialEq)]
pub struct Headline {
    pub rows: usize,
    pub positive: usize,
    pub zero: usize,
    pub unavailable: usize,
    pub coins_with_rows: usize,
    pub from_mps: usize,
}

fn read<T: for<'de> Deserialize<'de> + Default>(dir: &Path, file: &str) -> Result<T, String> {
    let p = dir.join(file);
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))
}

// ---------- published snapshots ----------
//
// `npm run refresh` builds pools.json, network.json and meta.json in a staging directory,
// validates them, renames the directory to data/snapshots/<id>/ and then swaps data/current.json
// (write to a temp file, fsync, rename) to point at it. The server reads the three files through
// that manifest, and checks each file's size and SHA-256 against it, so it always serves one
// complete, consistent snapshot. Without data/current.json the files are read from data/ itself
// (the layout before snapshots existed).

/// Files the refresh generates; they are always published, and loaded, together.
pub const GENERATED: &[&str] = &["pools.json", "network.json", "meta.json"];
/// The manifest naming the current snapshot.
pub const MANIFEST: &str = "current.json";

#[derive(Debug, Clone, Deserialize)]
pub struct ManifestEntry {
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub snapshot: String,
    #[serde(default)]
    pub published_at: Option<String>,
    pub files: BTreeMap<String, ManifestEntry>,
}

/// Where the generated files of the current snapshot live.
#[derive(Debug, Clone)]
pub struct Snapshot {
    pub id: Option<String>,
    pub dir: std::path::PathBuf,
    manifest: Option<Manifest>,
}

/// A snapshot id is one path segment: letters, digits, '.', '_' and '-', not starting with '.'.
pub fn valid_snapshot_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && !id.starts_with('.') && id.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    ring::digest::digest(&ring::digest::SHA256, bytes).as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

impl Snapshot {
    /// Resolve data/current.json. A manifest that is unreadable, names a bad id or a missing
    /// directory, or lists none of the generated files is an error (the server keeps serving the
    /// previous data).
    pub fn current(dir: &Path) -> Result<Snapshot, String> {
        let mp = dir.join(MANIFEST);
        let raw = match std::fs::read_to_string(&mp) {
            Ok(r) => r,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Snapshot { id: None, dir: dir.to_path_buf(), manifest: None }),
            Err(e) => return Err(format!("{}: {e}", mp.display())),
        };
        let m: Manifest = serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", mp.display()))?;
        if !valid_snapshot_id(&m.snapshot) {
            return Err(format!("{}: bad snapshot id {:?}", mp.display(), m.snapshot));
        }
        let sdir = dir.join("snapshots").join(&m.snapshot);
        if !sdir.is_dir() {
            return Err(format!("{}: snapshot {} not found", mp.display(), sdir.display()));
        }
        for f in GENERATED {
            if !m.files.contains_key(*f) {
                return Err(format!("{}: snapshot {} does not list {f}", mp.display(), m.snapshot));
            }
        }
        Ok(Snapshot { id: Some(m.snapshot.clone()), dir: sdir, manifest: Some(m) })
    }
    pub fn published_at(&self) -> Option<&str> {
        self.manifest.as_ref().and_then(|m| m.published_at.as_deref())
    }
    /// Read one generated file. With a manifest, its size and SHA-256 must match the entry, so a
    /// half-written or swapped file is never parsed.
    pub fn read<T: for<'de> Deserialize<'de>>(&self, file: &str) -> Result<T, String> {
        let p = self.dir.join(file);
        let bytes = std::fs::read(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        if let Some(m) = &self.manifest {
            let e = m.files.get(file).ok_or_else(|| format!("{}: not in the manifest", p.display()))?;
            if e.bytes != bytes.len() as u64 || !e.sha256.eq_ignore_ascii_case(&sha256_hex(&bytes)) {
                return Err(format!("{}: does not match {MANIFEST} (size or SHA-256 differs)", p.display()));
            }
        }
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", p.display()))
    }
}

/// Path of a generated file in the current snapshot (or data/ itself in the flat layout).
#[cfg_attr(not(test), allow(dead_code))]
pub fn generated_path(dir: &Path, file: &str) -> std::path::PathBuf {
    Snapshot::current(dir).map(|s| s.dir.join(file)).unwrap_or_else(|_| dir.join(file))
}

// ---------- upstream URLs ----------

/// An upstream URL that may be rendered as a link: http or https with a host, and no whitespace,
/// control characters, angle brackets, backslashes or backticks (quotes are escaped when
/// rendered). Anything else (javascript:, data:, vbscript:, protocol-relative, relative or
/// malformed) gives None.
pub fn safe_url(u: &str) -> Option<String> {
    let t = u.trim();
    if t.is_empty() || t.len() > 2048 || t.chars().any(|c| c.is_whitespace() || c.is_control() || matches!(c, '<' | '>' | '\\' | '`')) {
        return None;
    }
    let lower = t.to_ascii_lowercase();
    let rest = lower.strip_prefix("https://").or_else(|| lower.strip_prefix("http://"))?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    // Userinfo can conceal credentials in a link and is never needed by this site.
    if authority.contains('@') {
        return None;
    }
    let host = authority;
    if host.is_empty() || host.starts_with('.') || host.starts_with(':') {
        return None;
    }
    Some(t.to_string())
}

/// Drop a URL field that isn't `safe_url`, noting what was dropped.
fn clean_url(v: &mut Option<String>, what: &str, dropped: &mut Vec<String>) {
    if let Some(u) = v.as_deref() {
        match safe_url(u) {
            Some(ok) => *v = Some(ok),
            None => {
                dropped.push(format!("{what}: {:?}", u.chars().take(80).collect::<String>()));
                *v = None;
            }
        }
    }
}

/// Text that may also be a URL (miningpoolstats' `hashrate_src`: "explorer" or a link). Plain
/// words are kept; anything with a scheme must be http(s).
fn clean_text_or_url(v: &mut Option<String>, what: &str, dropped: &mut Vec<String>) {
    let looks_like_url = v.as_deref().map(|s| {
        let s = s.trim();
        s.contains("//") || s.split_once(':').map(|(a, _)| !a.is_empty() && a.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))).unwrap_or(false)
    });
    if looks_like_url == Some(true) {
        clean_url(v, what, dropped);
    } else if let Some(s) = v.as_deref() {
        if s.chars().any(|c| c.is_control()) {
            dropped.push(format!("{what}: control characters"));
            *v = None;
        }
    }
}

fn clean_links(v: &mut Vec<Link>, what: &str, dropped: &mut Vec<String>) {
    v.retain(|l| {
        let ok = safe_url(&l.url).is_some();
        if !ok {
            dropped.push(format!("{what}: {:?}", l.url.chars().take(80).collect::<String>()));
        }
        ok
    });
}

pub fn clean_pool_urls(p: &mut Pool, dropped: &mut Vec<String>) {
    let id = p.id.clone();
    for (f, name) in [
        (&mut p.url, "url"),
        (&mut p.source_url, "source_url"),
        (&mut p.data_url, "data_url"),
        (&mut p.hashrate_source, "hashrate_source"),
        (&mut p.fee_source, "fee_source"),
        (&mut p.miners_source, "miners_source"),
        (&mut p.blocks_source, "blocks_source"),
        (&mut p.min_payout_source, "min_payout_source"),
    ] {
        clean_url(f, &format!("pool {id} {name}"), dropped);
    }
}

pub fn clean_coin_urls(c: &mut Coin, dropped: &mut Vec<String>) {
    let id = c.id.clone();
    let n = &mut c.network;
    for (f, name) in [
        (&mut c.source_url, "source_url"),
        (&mut c.data_url, "data_url"),
        (&mut c.price_source, "price_source"),
        (&mut n.hashrate_source, "network.hashrate_source"),
        (&mut n.difficulty_source, "network.difficulty_source"),
        (&mut n.height_source, "network.height_source"),
        (&mut n.block_time_source, "network.block_time_source"),
    ] {
        clean_url(f, &format!("coin {id} {name}"), dropped);
    }
    clean_text_or_url(&mut n.hashrate_upstream, &format!("coin {id} network.hashrate_upstream"), dropped);
    if let Some(r) = c.block_reward_miner.as_mut() {
        clean_url(&mut r.source_url, &format!("coin {id} block_reward_miner.source_url"), dropped);
    }
    clean_links(&mut c.status_sources, &format!("coin {id} status_sources"), dropped);
    c.cross_checks.retain(|x| {
        let ok = safe_url(&x.url).is_some();
        if !ok {
            dropped.push(format!("coin {id} cross_checks: {:?}", x.url.chars().take(80).collect::<String>()));
        }
        ok
    });
}

// ---------- pool permalinks (data/curated/permalinks.json) ----------

/// `{"pools": {"<pool id>": "<permalink>"}, "aliases": {"<old slug>": "<permalink>"}}`. The refresh
/// assigns a permalink once, to new pool ids only, and never rewrites one; a pool keeps its URL
/// when its name or domain changes.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct PermalinksFile {
    pub pools: BTreeMap<String, String>,
    pub aliases: BTreeMap<String, String>,
}

pub const PERMALINKS: &str = "permalinks.json";

/// A permalink is lowercase letters, digits and single dashes, like the slugs it started from.
pub fn valid_permalink(s: &str) -> bool {
    !s.is_empty() && s.len() <= 160 && slugify(s) == s
}

/// The slug a pool's URL was built from before permalinks: coin id, name and domain.
pub fn computed_slug(p: &Pool) -> String {
    let host = p.url.as_deref().unwrap_or("").trim_start_matches("https://").trim_start_matches("http://").trim_start_matches("www.").to_string();
    slugify(&format!("{}-{}-{}", p.coin_id, p.name, host))
}

/// Give every pool its permanent slug, and collect redirects from the old computed URLs.
/// Pools without an entry (a hand-added row before the next refresh) fall back to the computed
/// slug, made unique against every assigned permalink.
pub fn assign_slugs(pools: &mut [Pool], file: &PermalinksFile) -> BTreeMap<String, String> {
    let mut taken: HashSet<String> = HashSet::new();
    let mut owner: BTreeMap<&str, &str> = BTreeMap::new();
    for (id, link) in &file.pools {
        if !valid_permalink(link) {
            log::warn!("{PERMALINKS}: {id}: invalid permalink {link:?} ignored");
            continue;
        }
        if owner.contains_key(link.as_str()) {
            log::warn!("{PERMALINKS}: {link:?} is assigned twice; {id} falls back to its computed slug");
            continue;
        }
        owner.insert(link, id);
        taken.insert(link.clone());
    }
    // Old URLs, computed exactly as before (duplicates numbered in file order).
    let mut seen_legacy = HashSet::new();
    for p in pools.iter_mut() {
        let base = computed_slug(p);
        let mut s = base.clone();
        let mut i = 2;
        while !seen_legacy.insert(s.clone()) {
            s = format!("{base}-{i}");
            i += 1;
        }
        p.legacy_slug = s;
    }
    for p in pools.iter_mut() {
        p.slug = match file.pools.get(&p.id).filter(|l| owner.get(l.as_str()) == Some(&p.id.as_str())) {
            Some(l) => l.clone(),
            None => {
                let base = if p.legacy_slug.is_empty() { slugify(&p.id) } else { p.legacy_slug.clone() };
                let mut s = base.clone();
                let mut i = 2;
                while taken.contains(&s) {
                    s = format!("{base}-{i}");
                    i += 1;
                }
                taken.insert(s.clone());
                s
            }
        };
    }
    let mut redirects = BTreeMap::new();
    for p in pools.iter() {
        if p.legacy_slug != p.slug && !taken.contains(&p.legacy_slug) {
            redirects.insert(p.legacy_slug.clone(), p.slug.clone());
        }
    }
    for (old, to) in &file.aliases {
        if !taken.contains(old) && pools.iter().any(|p| &p.slug == to) {
            redirects.insert(old.clone(), to.clone());
        }
    }
    redirects
}

pub fn slugify(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Region buckets used by the filter, in display order.
pub const REGION_ORDER: &[&str] = &["Global", "North America", "South America", "Europe", "Russia", "Asia", "Middle East", "Oceania", "Africa"];

/// Map one region token (a country, a code or a continent, as pools and miningpoolstats write them)
/// to a filter bucket. Matching is on the whole token, never on substrings, so "Russia" can't
/// match "us" and "Canada" can't match "ca"-anything by accident.
pub fn region_bucket(token: &str) -> Option<&'static str> {
    // "Global (multi-region)" -> "global"; "US-East" -> "us east"
    let t: String = token.split('(').next().unwrap_or("").trim().to_lowercase().replace(['-', '_', '.'], " ");
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let b = match t.as_str() {
        "global" | "worldwide" | "world" | "ww" | "wl" | "multi region" | "anycast" => "Global",
        "us" | "usa" | "u s" | "united states" | "united states of america" | "us east" | "us west" | "us central" | "na" | "north america"
        | "ca" | "canada" | "mx" | "mexico" => "North America",
        "sa" | "south america" | "latam" | "latin america" | "br" | "brazil" | "ar" | "argentina" | "cl" | "chile" | "co" | "colombia" => "South America",
        "eu" | "europe" | "eu west" | "eu east" | "eu central" | "fr" | "france" | "de" | "germany" | "nl" | "netherlands" | "fi" | "finland"
        | "cz" | "czechia" | "czech republic" | "hu" | "hungary" | "pl" | "poland" | "uk" | "gb" | "united kingdom" | "great britain" | "es" | "spain"
        | "it" | "italy" | "se" | "sweden" | "no" | "norway" | "ch" | "switzerland" | "at" | "austria" | "ro" | "romania" | "bg" | "bulgaria"
        | "ua" | "ukraine" | "lt" | "lithuania" | "lv" | "latvia" | "ee" | "estonia" | "ie" | "ireland" | "pt" | "portugal" | "be" | "belgium"
        | "dk" | "denmark" | "sk" | "slovakia" | "is" | "iceland" => "Europe",
        "ru" | "russia" | "russian federation" => "Russia",
        "asia" | "ap" | "apac" | "asia pacific" | "hk" | "hong kong" | "sg" | "singapore" | "cn" | "china" | "in" | "india" | "jp" | "japan"
        | "kr" | "korea" | "south korea" | "tw" | "taiwan" | "kz" | "kazakhstan" | "vn" | "vietnam" | "th" | "thailand" | "my" | "malaysia"
        | "id" | "indonesia" | "ph" | "philippines" => "Asia",
        "me" | "middle east" | "ae" | "uae" | "united arab emirates" | "tr" | "turkey" | "il" | "israel" | "sa east" | "bh" | "bahrain" | "qa" | "qatar" => "Middle East",
        "oceania" | "au" | "australia" | "nz" | "new zealand" => "Oceania",
        "africa" | "za" | "south africa" | "ng" | "nigeria" | "eg" | "egypt" => "Africa",
        _ => return None,
    };
    Some(b)
}

/// Split a free-form region string ("US, EU, ASIA", "Global (multi-region)", "EU/US") into tokens.
pub fn region_tokens(s: &str) -> Vec<String> {
    s.replace(" and ", ",").split([',', '/', ';', '&', '|']).map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect()
}

/// Normalise a pool's region fields into filter buckets (ordered as `REGION_ORDER`).
/// Uses `region`, then `regions`, then `country_code` as a fallback when no text is given.
/// Returns the buckets and any tokens that could not be mapped (logged at load time).
pub fn region_tags_checked(p: &Pool) -> (Vec<String>, Vec<String>) {
    let mut tokens: Vec<String> = Vec::new();
    if let Some(r) = &p.region {
        tokens.extend(region_tokens(r));
    }
    for r in &p.regions {
        tokens.extend(region_tokens(r));
    }
    if tokens.is_empty() {
        if let Some(cc) = p.country_code.as_deref().filter(|c| !c.trim().is_empty()) {
            tokens.push(cc.to_string());
        }
    }
    let mut found: Vec<&str> = Vec::new();
    let mut unknown = Vec::new();
    for t in &tokens {
        match region_bucket(t) {
            Some(b) => {
                if !found.contains(&b) {
                    found.push(b)
                }
            }
            None => unknown.push(t.clone()),
        }
    }
    let tags = REGION_ORDER.iter().filter(|b| found.contains(b)).map(|b| b.to_string()).collect();
    (tags, unknown)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn region_tags(p: &Pool) -> Vec<String> {
    region_tags_checked(p).0
}

/// Canonical spelling of a payout scheme ("pplns " -> "PPLNS", "pps plus" stays as written but upper-cased).
pub fn canonical_scheme(s: &str) -> String {
    s.trim().to_uppercase()
}

/// Fingerprint of every file in `dir` and `dir/curated`: (path, mtime, size, content hash).
/// Any change (edit, replace with an older mtime, add, delete) changes the fingerprint.
/// The hash catches same-size edits inside one filesystem timestamp tick; the files are small
/// (well under a megabyte), so reading them every poll is cheap.
pub type Fingerprint = Vec<(std::path::PathBuf, Option<std::time::SystemTime>, u64, u64)>;

pub fn fingerprint(dir: &Path) -> Fingerprint {
    let mut out = Vec::new();
    for sub in [dir.to_path_buf(), dir.join("curated")] {
        if let Ok(rd) = std::fs::read_dir(&sub) {
            for e in rd.flatten() {
                let path = e.path();
                if path.extension().and_then(|x| x.to_str()) != Some("json") {
                    continue;
                }
                if let Ok(m) = e.metadata() {
                    use std::hash::{Hash, Hasher};
                    let mut h = std::collections::hash_map::DefaultHasher::new();
                    std::fs::read(&path).unwrap_or_default().hash(&mut h);
                    out.push((path, m.modified().ok(), m.len(), h.finish()));
                }
            }
        }
    }
    out.sort();
    out
}

/// Watches `data/` and `data/curated/` and reloads when anything changes.
pub struct Reloader {
    dir: std::path::PathBuf,
    last: Fingerprint,
}

impl Reloader {
    pub fn new(dir: &Path) -> Self {
        Reloader { dir: dir.to_path_buf(), last: fingerprint(dir) }
    }
    /// `None` when nothing changed. `Some(Ok)` with fresh data after a change.
    /// `Some(Err)` when the changed files don't load; the next poll retries until they do.
    #[cfg(test)]
    pub fn poll(&mut self) -> Option<Result<Data, String>> {
        self.poll_with(None)
    }
    /// `poll`, laying the pollers' last good readings over the reloaded files.
    pub fn poll_with(&mut self, live: Option<&crate::live::LiveState>) -> Option<Result<Data, String>> {
        let now = fingerprint(&self.dir);
        if now == self.last {
            return None;
        }
        let r = load_with_live(&self.dir, live);
        if r.is_ok() {
            // Re-read: a writer may have finished between the fingerprint and the load.
            self.last = now;
        }
        Some(r)
    }
}

/// Overlay a hand-curated row on the row the refresh script wrote for the same id.
/// The curated file wins for every field it sets, so a manual edit shows up straight away.
/// For rows marked `"live"` the refresh fetches the fee from the pool's API, so the fee, its
/// provenance and `data_url` keep the refreshed values (scheme fees follow the refreshed fee).
/// `"live_fields": ["fee", "hashrate"]` widens that to the hashrate (value, basis, source, time
/// and window), for pools with a public hashrate endpoint.
/// Nothing else is taken from the refresh: in particular the hashrate keeps its own
/// `hashrate_observed_at`, and the row's `fetched_at` stays the hand-verification time. A fee
/// refresh must never make an operator-reported hashrate look newer than it is.
pub fn merge_manual(base: Option<&serde_json::Value>, manual: &serde_json::Value) -> serde_json::Value {
    let mut out = base.cloned().filter(|b| b.is_object()).unwrap_or_else(|| serde_json::json!({}));
    let live = manual.get("live").and_then(|x| x.as_str()).is_some();
    // Fields a live endpoint refreshes. The refreshed values (and their provenance) win over the
    // hand-curated ones, and their times are never reset to verified_at. `live` alone means fee.
    let live_fields: Vec<String> = match manual.get("live_fields").and_then(|x| x.as_array()) {
        Some(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
        None if live => vec!["fee".into()],
        None => vec![],
    };
    let refreshed = |k: &str| -> bool {
        live_fields.iter().any(|f| match f.as_str() {
            "fee" => matches!(k, "fee_pct" | "fee_source" | "fee_observed_at" | "data_url"),
            "hashrate" => matches!(k, "hashrate" | "hashrate_source" | "hashrate_observed_at" | "hashrate_is_reported" | "basis" | "hashrate_window_s" | "live_source"),
            other => k == other || k == format!("{other}_source") || k == format!("{other}_observed_at"),
        })
    };
    let obj = out.as_object_mut().unwrap();
    if let Some(m) = manual.as_object() {
        for (k, v) in m {
            if k == "verified_at" || k == "live" || k == "live_fields" || k.starts_with('_') {
                continue;
            }
            if base.is_some() && refreshed(k) {
                continue;
            }
            obj.insert(k.clone(), v.clone());
        }
    }
    if base.is_some() && live_fields.iter().any(|f| f == "fee") {
        if let Some(fee) = obj.get("fee_pct").cloned().filter(|f| f.is_number()) {
            if let Some(serde_json::Value::Array(schemes)) = obj.get_mut("schemes") {
                for s in schemes.iter_mut() {
                    if let Some(so) = s.as_object_mut() {
                        so.insert("fee_pct".into(), fee.clone());
                    }
                }
            }
        }
    }
    if let Some(v) = manual.get("verified_at").filter(|v| v.is_string()) {
        obj.insert("fetched_at".into(), v.clone());
        // Hand-verified fields default to the verification time when the row doesn't say otherwise.
        for f in ["hashrate", "miners", "blocks", "min_payout", "fee"] {
            let key = format!("{f}_observed_at");
            if manual.get(&key).is_none() && !(base.is_some() && live_fields.iter().any(|x| x == f)) {
                obj.insert(key, v.clone());
            }
        }
    }
    out
}


/// Product photos for the Buy page. Path must be under static/shop/, webp/png only, ≤ 2 MiB.
pub fn check_shop_image(static_root: &Path, rel: &str) -> Result<(), String> {
    let rel = rel.trim_start_matches('/');
    if rel.contains("..") || rel.starts_with('.') {
        return Err("bad path".into());
    }
    let mut parts = rel.split('/');
    if parts.next() != Some("shop") {
        return Err("must be under shop/".into());
    }
    let name = parts.next_back().unwrap_or("");
    let ok_name = !name.is_empty()
        && !name.starts_with('.')
        && name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '.' | '-' | '_'));
    if !ok_name {
        return Err("bad file name".into());
    }
    let ext = name.rsplit('.').next().unwrap_or("");
    if !matches!(ext, "webp" | "png") {
        return Err("only webp/png".into());
    }
    let path = static_root.join(rel);
    let meta = std::fs::metadata(&path).map_err(|e| format!("missing ({e})"))?;
    const MAX: u64 = 2 * 1024 * 1024;
    if !meta.is_file() || meta.len() == 0 || meta.len() > MAX {
        return Err(format!("size {} not in 1..={MAX}", meta.len()));
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    match (ext, crate::views::logo::sniff(&bytes)) {
        ("webp", Some("webp")) | ("png", Some("png")) => Ok(()),
        (e, t) => Err(format!("extension .{e} but content is {}", t.unwrap_or("unknown"))),
    }
}

pub fn load(dir: &Path) -> Result<Data, String> {
    load_with_live(dir, None)
}

/// Pool ÷ denominator as a percentage, never above 100. Returns (share, capped).
pub fn share_pct(h: Option<f64>, denom: Option<f64>) -> (Option<f64>, bool) {
    match (h, denom) {
        (Some(h), Some(d)) if d > 0.0 && h.is_finite() => {
            let raw = h / d * 100.0;
            if raw > 100.0 { (Some(100.0), true) } else { (Some(raw), false) }
        }
        _ => (None, false),
    }
}

/// `load`, with the latest good readings from the live pollers laid over the files.
pub fn load_with_live(dir: &Path, live: Option<&crate::live::LiveState>) -> Result<Data, String> {
    // The generated files come from one published snapshot (data/current.json), read together.
    let snap = Snapshot::current(dir)?;
    let pf: PoolsFile = snap.read("pools.json")?;
    let nf: NetworkFile = snap.read("network.json")?;
    let mut meta: Meta = if snap.id.is_some() { snap.read("meta.json")? } else { read(dir, "meta.json").unwrap_or_default() };
    let mut af: ArchiveFile = read(dir, "archive.json")?;
    let mut mf: MinersFile = read(dir, "miners.json")?;
    let mut rf: ResearchFile = read(dir, "research.json").unwrap_or_default();
    // Upstream URLs are rendered as links: only http(s) ones are kept (see safe_url).
    let mut dropped: Vec<String> = Vec::new();
    // Social/community links (written by a separate research step). Missing file = no links;
    // a broken file is an error, so the last good data keeps serving.
    let links: LinksFile = match std::fs::read_to_string(dir.join("curated").join("links.json")) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", dir.join("curated").join("links.json").display()))?,
        Err(_) => LinksFile::default(),
    };
    let logos = crate::views::logo::load(dir)?;

    let mut coins = nf.coins;
    for c in coins.iter_mut() {
        clean_coin_urls(c, &mut dropped);
    }
    // Hand-added coins (data/curated/coins.json) appear even before the next refresh.
    if let Ok(raw) = std::fs::read_to_string(dir.join("curated").join("coins.json")) {
        match serde_json::from_str::<NetworkFile>(&raw) {
            Ok(extra) => {
                for mut c in extra.coins {
                    if c.id.is_empty() || coins.iter().any(|x| x.id == c.id) {
                        continue;
                    }
                    clean_coin_urls(&mut c, &mut dropped);
                    if c.status.is_empty() {
                        c.status = "active".into();
                    }
                    coins.push(c);
                }
            }
            Err(e) => log::warn!("curated/coins.json ignored: {e}"),
        }
    }
    // Coin status overrides (data/curated/coin-status.json) also apply without a refresh.
    if let Ok(raw) = std::fs::read_to_string(dir.join("curated").join("coin-status.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            for st in v.get("coins").and_then(|c| c.as_array()).cloned().unwrap_or_default() {
                let id = st.get("id").and_then(|x| x.as_str()).unwrap_or_default();
                if let Some(c) = coins.iter_mut().find(|c| c.id == id) {
                    if let Some(s) = st.get("status").and_then(|x| x.as_str()) {
                        c.status = s.into();
                    }
                    if let Some(n) = st.get("note").and_then(|x| x.as_str()) {
                        c.status_note = Some(n.into());
                    }
                    if let Some(mut src) = st.get("sources").cloned().and_then(|x| serde_json::from_value::<Vec<Link>>(x).ok()) {
                        clean_links(&mut src, &format!("coin-status {id} sources"), &mut dropped);
                        c.status_sources = src;
                    }
                }
            }
        }
    }
    // Disambiguate coins that share a name (e.g. Kerrigan on two parameter sets).
    let mut name_count: BTreeMap<String, u32> = BTreeMap::new();
    for c in &coins {
        *name_count.entry(c.name.clone()).or_default() += 1;
    }
    for c in coins.iter_mut() {
        c.label = if name_count[&c.name] > 1 { format!("{} ({})", c.name, c.params()) } else { c.name.clone() };
    }

    let mut pools = pf.pools;
    // Hand-curated rows (data/curated/manual-pools.json). New ids are added; ids the refresh
    // already wrote into pools.json are overlaid, so edits show without waiting for a refresh.
    match std::fs::read_to_string(dir.join("curated").join("manual-pools.json")) {
        Ok(raw) => match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => {
                for row in v.get("pools").and_then(|p| p.as_array()).cloned().unwrap_or_default() {
                    let id = row.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                    if id.is_empty() {
                        continue;
                    }
                    let idx = pools.iter().position(|p| p.id == id);
                    let base = idx.and_then(|i| serde_json::to_value(&pools[i]).ok());
                    let merged = merge_manual(base.as_ref(), &row);
                    match serde_json::from_value::<Pool>(merged) {
                        Ok(mut p) => {
                            if p.coin_id.is_empty() {
                                p.coin_id = coins.iter().find(|c| c.symbol == p.coin).map(|c| c.id.clone()).unwrap_or_default();
                            }
                            match idx {
                                Some(i) => pools[i] = p,
                                None => pools.push(p),
                            }
                        }
                        Err(e) => log::warn!("curated/manual-pools.json: row {id} ignored: {e}"),
                    }
                }
            }
            Err(e) => return Err(format!("{}: {e}", dir.join("curated").join("manual-pools.json").display())),
        },
        Err(_) => {}
    }
    // Permanent pool URLs (data/curated/permalinks.json); a broken file is an error, so the last
    // good data keeps serving. Missing file = every pool uses its computed slug.
    let permalinks: PermalinksFile = match std::fs::read_to_string(dir.join("curated").join(PERMALINKS)) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", dir.join("curated").join(PERMALINKS).display()))?,
        Err(_) => PermalinksFile::default(),
    };
    for p in pools.iter_mut() {
        clean_pool_urls(p, &mut dropped);
    }
    let slug_redirects = assign_slugs(&mut pools, &permalinks);
    for p in pools.iter_mut() {
        let (tags, unknown) = region_tags_checked(p);
        if !unknown.is_empty() {
            log::warn!("pool {}: unrecognised region value(s) {:?}; add them to data::region_bucket", p.id, unknown);
        }
        p.region_tags = tags;
        let mut schemes: Vec<String> = Vec::new();
        for s in p.payout_schemes.iter().map(|s| canonical_scheme(s)).chain(p.schemes.iter().map(|s| canonical_scheme(&s.scheme))) {
            if !s.is_empty() && !schemes.contains(&s) {
                schemes.push(s);
            }
        }
        p.payout_schemes = schemes;
        for s in p.schemes.iter_mut() {
            s.scheme = canonical_scheme(&s.scheme);
        }
        p.coin_label = coins.iter().find(|c| c.id == p.coin_id).map(|c| c.label.clone()).unwrap_or_else(|| p.coin.clone());
        fill_provenance(p);
        p.links = links.pools.get(&p.id).map(|v| verified_links(v)).unwrap_or_default();
        p.logo = logos.pool(&p.id, &p.name);
    }
    // Live endpoints (data/curated/live-sources.json): the last good reading of each, when newer
    // than what the files hold, replaces that one figure and its provenance.
    let live_cfg = crate::live::read_config(dir)?;
    crate::live::unify(&live_cfg, &mut pools);
    if let Some(state) = live {
        crate::live::apply(&live_cfg, state, &mut pools, &mut coins);
    }
    for c in coins.iter_mut() {
        c.pool_count = pools.iter().filter(|p| p.coin_id == c.id).count() as u32;
        c.reported = reported_for(&c.id, &pools);
        c.links = links.coins.get(&c.id).map(|v| verified_links(v)).unwrap_or_default();
        c.logo = logos.coin(&c.id, &c.name);
    }
    coins.sort_by(rank_cmp);
    // Network-share normalisation. Small coins' network-hashrate estimates are noisy and are
    // sometimes below the sum of what pools report, which would give shares above 100%.
    // In that case shares are measured against the listed pools' total instead (labelled in the UI).
    for c in coins.iter_mut() {
        let sum: f64 = pools.iter().filter(|p| p.coin_id == c.id).filter_map(|p| p.hashrate).filter(|h| *h > 0.0).sum();
        let net = c.network.hashrate.filter(|h| *h > 0.0);
        let (basis, denom) = match net {
            Some(n) if sum <= n * 1.02 => ("network", Some(n)),
            _ if sum > 0.0 => ("pools", Some(sum)),
            _ => ("none", None),
        };
        c.share_basis = basis.into();
        c.share_denominator = denom;
        let reporting = pools.iter().filter(|p| p.coin_id == c.id && p.hashrate.unwrap_or(0.0) > 0.0).count();
        for p in pools.iter_mut().filter(|p| p.coin_id == c.id) {
            let (share, capped) = share_pct(p.hashrate, denom);
            p.network_share_pct = share;
            p.share_capped = capped;
            p.share_basis = basis.into();
            p.share_note = None;
            // The only reporting pool reads above the network estimate: the two figures cover
            // different windows, so show both instead of a share over 100% (or a trivial 100%).
            if basis == "pools" && reporting == 1 && p.hashrate.unwrap_or(0.0) > 0.0 {
                if let (Some(n), Some(h)) = (net, p.hashrate) {
                    let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
                    p.share_note = Some(format!(
                        "{} is the only listed pool. Its {} reads above the network estimate{} ({}). The two cover different windows, so no share above 100% is shown.",
                        p.name,
                        match p.hashrate_window_s { Some(w) => format!("{}-minute hashrate ({})", w / 60, crate::fmt::hashrate(Some(h), &unit)), None => format!("hashrate ({})", crate::fmt::hashrate(Some(h), &unit)) },
                        match c.network.hashrate_sample_blocks { Some(b) => format!(" from the last {b} blocks"), None => String::new() },
                        crate::fmt::hashrate(Some(n), &unit)
                    ));
                }
            } else if basis == "pools" && reporting == 1 && net.is_none() && p.hashrate.unwrap_or(0.0) > 0.0 {
                p.share_note = Some(format!("{} is the only listed pool reporting hashrate, and no network estimate is published, so no share is shown.", p.name));
            } else if capped {
                p.share_note = Some("Above 100% of the network estimate (different measurement windows); capped at 100%.".into());
            }
            p.share_flag = p.network_share_pct.unwrap_or(0.0) > 30.0 && (basis == "network" || reporting > 1);
            p.share_status = share_status(p.hashrate, basis, capped, reporting, net.is_some()).into();
        }
    }
    for a in af.pools.iter_mut() {
        clean_url(&mut a.url, &format!("archive {} url", a.id), &mut dropped);
        clean_links(&mut a.sources, &format!("archive {} sources", a.id), &mut dropped);
    }
    for m in mf.miners.iter_mut() {
        clean_url(&mut m.source_url, &format!("miner {} source_url", m.id), &mut dropped);
    }
    for r in rf.items.iter_mut() {
        clean_url(&mut r.url, &format!("research {} url", r.pool), &mut dropped);
    }
    meta.sources.retain(|s| {
        let ok = safe_url(&s.url).is_some();
        if !ok {
            dropped.push(format!("meta source {}: {:?}", s.id, s.url.chars().take(80).collect::<String>()));
        }
        ok
    });
    for x in &dropped {
        log::warn!("unsafe or malformed URL dropped (only http/https links are rendered): {x}");
    }
    let last_updated = pools
        .iter()
        .filter(|p| p.from_miningpoolstats)
        .filter_map(|p| p.fetched_at.clone())
        .max()
        .or(pf.generated_at.clone());

    // Shop directory (data/curated/vendors.json + listings.json). Missing files = empty directory.
    let static_root = std::env::var("STATIC_DIR").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from("static"));
    let mut vf: VendorsFile = match std::fs::read_to_string(dir.join("curated").join("vendors.json")) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", dir.join("curated").join("vendors.json").display()))?,
        Err(_) => VendorsFile::default(),
    };
    let mut lf: ListingsFile = match std::fs::read_to_string(dir.join("curated").join("listings.json")) {
        Ok(raw) => serde_json::from_str(&raw).map_err(|e| format!("{}: {e}", dir.join("curated").join("listings.json").display()))?,
        Err(_) => ListingsFile::default(),
    };
    // Validate vendor URLs; attach logos.
    for v in vf.vendors.iter_mut() {
        clean_url(&mut v.url, &format!("vendor {} url", v.id), &mut dropped);
        clean_url(&mut v.source_url, &format!("vendor {} source_url", v.id), &mut dropped);
        if v.slug.is_empty() {
            v.slug = v.id.clone();
        }
        v.logo = logos.vendor(&v.id, &v.name);
    }
    // Validate listings: outbound product URL required; image must live under static/shop/.
    let miner_label_by_id: std::collections::HashMap<String, String> = mf
        .miners
        .iter()
        .map(|m| (m.id.clone(), format!("{} {}", m.maker, m.model)))
        .collect();
    let vendor_by_id: std::collections::HashMap<String, (String, String)> =
        vf.vendors.iter().map(|v| (v.id.clone(), (v.name.clone(), v.slug.clone()))).collect();
    let mut kept_listings = Vec::new();
    for mut listing in lf.listings.drain(..) {
        let mut product = Some(listing.product_url.clone());
        clean_url(&mut product, &format!("listing {} product_url", listing.id), &mut dropped);
        let Some(product_url) = product else { continue };
        listing.product_url = product_url;
        clean_url(&mut listing.source_url, &format!("listing {} source_url", listing.id), &mut dropped);
        clean_url(&mut listing.image_source_url, &format!("listing {} image_source_url", listing.id), &mut dropped);
        // n/a ≠ 0: a missing or non-positive price becomes None.
        if listing.price_amount.map(|p| !(p.is_finite() && p > 0.0)).unwrap_or(false) {
            listing.price_amount = None;
        }
        if listing.shop_hashrate_ksol.map(|h| !(h.is_finite() && h > 0.0)).unwrap_or(false) {
            listing.shop_hashrate_ksol = None;
        }
        listing.image_src = match listing.image.as_deref() {
            Some(rel) => match check_shop_image(&static_root, rel) {
                Ok(()) => Some(format!("/static/{rel}")),
                Err(why) => {
                    dropped.push(format!("listing {} image {rel}: {why}", listing.id));
                    None
                }
            },
            None => None,
        };
        if let Some((name, slug)) = vendor_by_id.get(&listing.vendor_id) {
            listing.vendor_name = name.clone();
            listing.vendor_slug = slug.clone();
        } else {
            dropped.push(format!("listing {}: unknown vendor_id {}", listing.id, listing.vendor_id));
            continue;
        }
        if let Some(label) = miner_label_by_id.get(&listing.miner_id) {
            listing.miner_label = label.clone();
        } else {
            listing.miner_label = listing.title.clone();
            dropped.push(format!("listing {}: miner_id {} not in miners.json (kept; label falls back to title)", listing.id, listing.miner_id));
        }
        kept_listings.push(listing);
    }
    lf.listings = kept_listings;
    // Listing counts on vendors.
    for v in vf.vendors.iter_mut() {
        v.listing_count = lf.listings.iter().filter(|l| l.vendor_id == v.id).count();
    }
    for x in &dropped {
        if x.starts_with("listing ") || x.starts_with("vendor ") {
            log::warn!("shop directory: {x}");
        }
    }
    drop(miner_label_by_id);
    drop(vendor_by_id);

    Ok(Data {
        pools,
        coins,
        archive: af.pools,
        miners: mf.miners,
        miners_verified_at: mf.verified_at,
        miners_not_listed: mf.not_listed,
        vendors: vf.vendors,
        vendors_verified_at: vf.verified_at,
        listings: lf.listings,
        listings_verified_at: lf.verified_at,
        research: rf.items,
        research_coins: rf.coins,
        meta,
        last_updated,
        links_generated_at: links.generated_at,
        logos,
        live: crate::live::statuses(&live_cfg, live, chrono::Utc::now()),
        live_stale_after_secs: live_cfg.stale_after_seconds,
        snapshot: snap.id.clone(),
        snapshot_dir: snap.dir.clone(),
        snapshot_published_at: snap.published_at().map(String::from),
        slug_redirects,
    })
}

/// What a pool's share figure is measured against:
/// - "network": the network estimate (the only case where it is a share of the network);
/// - "only_listed_pool": the one pool reporting hashrate, reading above the network estimate or
///   on a coin without one (no share is shown);
/// - "pools_exceed_network": the listed pools together read above the network estimate (or this
///   pool alone does), so shares are of the listed pools' total;
/// - "no_network_estimate": several pools, no network estimate; shares are of their total;
/// - "unavailable": the pool publishes no hashrate, or there is nothing to divide by.
pub fn share_status(h: Option<f64>, basis: &str, capped: bool, reporting: usize, has_net: bool) -> &'static str {
    let Some(h) = h.filter(|h| h.is_finite()) else { return "unavailable" };
    match basis {
        "network" if capped => "pools_exceed_network",
        "network" => "network",
        "pools" if reporting == 1 && h > 0.0 => "only_listed_pool",
        "pools" if has_net => "pools_exceed_network",
        "pools" => "no_network_estimate",
        _ => "unavailable",
    }
}

impl Data {

    pub fn vendor(&self, id: &str) -> Option<&Vendor> {
        self.vendors.iter().find(|v| v.id == id || v.slug == id)
    }

    pub fn listings_for_miner(&self, miner_id: &str) -> Vec<&Listing> {
        self.listings.iter().filter(|l| l.miner_id == miner_id).collect()
    }

    /// Machines that have at least one listing, Z15 Pro first, then miners.json order.
    pub fn buy_machine_groups(&self) -> Vec<(String, String, Vec<&Listing>)> {
        let mut out: Vec<(String, String, Vec<&Listing>)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        // Prefer miners.json order, but hoist antminer-z15-pro.
        let mut order: Vec<&Miner> = Vec::new();
        if let Some(pro) = self.miners.iter().find(|m| m.id == "antminer-z15-pro") {
            order.push(pro);
        }
        for m in &self.miners {
            if m.id != "antminer-z15-pro" {
                order.push(m);
            }
        }
        for m in order {
            let ls: Vec<&Listing> = self.listings.iter().filter(|l| l.miner_id == m.id).collect();
            if ls.is_empty() {
                continue;
            }
            seen.insert(m.id.as_str());
            out.push((m.id.clone(), format!("{} {}", m.maker, m.model), ls));
        }
        // Listings whose miner_id is not in miners.json still appear, grouped by miner_id.
        for l in &self.listings {
            if seen.contains(l.miner_id.as_str()) {
                continue;
            }
            let ls: Vec<&Listing> = self.listings.iter().filter(|x| x.miner_id == l.miner_id).collect();
            seen.insert(l.miner_id.as_str());
            out.push((l.miner_id.clone(), l.miner_label.clone(), ls));
        }
        out
    }

    pub fn coin(&self, id: &str) -> Option<&Coin> {
        self.coins.iter().find(|c| c.id == id)
    }
    /// Active coins grouped by exact parameter set, each group already in rank order.
    pub fn param_groups(&self, active_only: bool) -> Vec<(String, Vec<&Coin>)> {
        let mut out: Vec<(String, Vec<&Coin>)> = Vec::new();
        for c in self.coins.iter().filter(|c| !active_only || c.active()) {
            let label = c.group_label();
            match out.iter_mut().find(|(l, _)| *l == label) {
                Some((_, v)) => v.push(c),
                None => out.push((label, vec![c])),
            }
        }
        out
    }
    pub fn headline(&self) -> Headline {
        let live: Vec<&Pool> = self.live_pools().collect();
        let coins: HashSet<&str> = live.iter().map(|p| p.coin_id.as_str()).collect();
        Headline {
            rows: live.len(),
            positive: live.iter().filter(|p| p.hashrate.map(|h| h > 0.0).unwrap_or(false)).count(),
            zero: live.iter().filter(|p| p.hashrate.map(|h| h <= 0.0).unwrap_or(false)).count(),
            unavailable: live.iter().filter(|p| p.hashrate.is_none()).count(),
            coins_with_rows: coins.len(),
            from_mps: live.iter().filter(|p| p.from_miningpoolstats).count(),
        }
    }
    /// Newest pool-hashrate observation for a coin from automatic sources (what its age shows).
    /// Live-fed rows are left out: they carry their own age in the live line, and a fresh live
    /// reading must not hide that the coin's other figures are stale.
    pub fn coin_observed_at(&self, c: &Coin) -> Option<String> {
        self.pools
            .iter()
            .filter(|p| p.coin_id == c.id && !p.operator_reported() && p.live_source.is_none())
            .filter_map(|p| p.hashrate_observed_at.clone())
            .max()
            .or_else(|| c.network.hashrate_observed_at.clone())
            .or_else(|| c.network.height_observed_at.clone())
            .or_else(|| c.fetched_at.clone())
    }
    /// Whether the coin has figures that are not fed by a live source (and so need their own age
    /// line): any non-live pool row, or a network figure that isn't live.
    pub fn has_static_figures(&self, c: &Coin) -> bool {
        self.pools.iter().any(|p| p.coin_id == c.id && p.live_source.is_none())
            || (c.network.live_source.is_none() && !self.pools.iter().any(|p| p.coin_id == c.id))
    }
    pub fn active_coin_ids(&self) -> Vec<String> {
        self.coins.iter().filter(|c| c.active()).map(|c| c.id.clone()).collect()
    }
    /// Pools on coins whose PoW is still running.
    pub fn live_pools(&self) -> impl Iterator<Item = &Pool> {
        let active: HashSet<String> = self.active_coin_ids().into_iter().collect();
        self.pools.iter().filter(move |p| active.contains(&p.coin_id))
    }
    /// Every payout scheme that appears on a live pool, most common first. The payout filter is
    /// built from this, so a scheme added to the data (PPLNT, PPLNSBF, ...) is never missing.
    pub fn payout_schemes(&self) -> Vec<(String, usize)> {
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();
        for p in self.live_pools() {
            for s in &p.payout_schemes {
                *counts.entry(s.clone()).or_default() += 1;
            }
        }
        let mut v: Vec<(String, usize)> = counts.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    }
    /// Region buckets that appear on live pools, in `REGION_ORDER`.
    pub fn regions(&self) -> Vec<String> {
        REGION_ORDER.iter().filter(|r| self.live_pools().any(|p| p.region_tags.iter().any(|t| t == *r))).map(|r| r.to_string()).collect()
    }
    /// Pools that MPS still lists for coins whose PoW ended.
    pub fn ended_coin_pools(&self) -> Vec<&Pool> {
        let active: HashSet<String> = self.active_coin_ids().into_iter().collect();
        self.pools.iter().filter(|p| !active.contains(&p.coin_id)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn real_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("data")
    }

    /// A throwaway copy of data/ in the system temp dir.
    struct TempData(PathBuf);
    impl TempData {
        fn new(tag: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("equihash-test-{tag}-{}-{:?}", std::process::id(), std::thread::current().id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("curated")).unwrap();
            for sub in ["", "curated"] {
                for e in std::fs::read_dir(real_dir().join(sub)).unwrap().flatten() {
                    if e.path().is_file() && e.file_name() != MANIFEST {
                        std::fs::copy(e.path(), dir.join(sub).join(e.file_name())).unwrap();
                    }
                }
            }
            // The flat layout: the current snapshot's generated files sit in the temp dir itself.
            for f in GENERATED {
                std::fs::copy(generated_path(&real_dir(), f), dir.join(f)).unwrap();
            }
            TempData(dir)
        }
        fn manual(&self) -> PathBuf {
            self.0.join("curated").join("manual-pools.json")
        }
    }
    impl Drop for TempData {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn edit_manual(t: &TempData, f: impl FnOnce(&mut serde_json::Value)) {
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(t.manual()).unwrap()).unwrap();
        f(&mut v);
        std::fs::write(t.manual(), serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }

    fn pool(region: Option<&str>, regions: &[&str], cc: Option<&str>) -> Pool {
        Pool { region: region.map(String::from), regions: regions.iter().map(|s| s.to_string()).collect(), country_code: cc.map(String::from), ..Default::default() }
    }

    // ---- bug 1: manual edits hot-reload ----

    #[test]
    fn manual_edit_overrides_row_already_in_pools_json() {
        let t = TempData::new("override");
        // Pick a non-live manual row that the refresh already copied into pools.json.
        let pools_json: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(t.0.join("pools.json")).unwrap()).unwrap();
        let in_pools: Vec<String> = pools_json["pools"].as_array().unwrap().iter().filter_map(|p| p["id"].as_str().map(String::from)).collect();
        let mut target = String::new();
        edit_manual(&t, |v| {
            for row in v["pools"].as_array_mut().unwrap() {
                let id = row["id"].as_str().unwrap().to_string();
                if row.get("live").is_none() && in_pools.contains(&id) && target.is_empty() {
                    row["name"] = serde_json::json!("Renamed By Hand");
                    row["min_payout"] = serde_json::json!(0.123);
                    target = id;
                }
            }
        });
        assert!(!target.is_empty(), "test needs a non-live manual row that is also in pools.json");
        let d = load(&t.0).unwrap();
        let p = d.pools.iter().find(|p| p.id == target).unwrap();
        assert_eq!(p.name, "Renamed By Hand");
        assert_eq!(p.min_payout, Some(0.123));
        assert_eq!(d.pools.iter().filter(|p| p.id == target).count(), 1, "no duplicate row");
    }

    #[test]
    fn reloader_picks_up_manual_edit() {
        let t = TempData::new("reload");
        let mut r = Reloader::new(&t.0);
        assert!(r.poll().is_none(), "nothing changed yet");
        edit_manual(&t, |v| {
            v["pools"][0]["name"] = serde_json::json!("Hot Reloaded");
        });
        let d = r.poll().expect("change detected").expect("loads");
        assert!(d.pools.iter().any(|p| p.name == "Hot Reloaded"));
        assert!(r.poll().is_none(), "settles after reload");
    }

    #[test]
    fn reloader_sees_replacement_with_older_mtime_and_new_files() {
        let t = TempData::new("mtime");
        let before = fingerprint(&t.0);
        // Same size, older mtime (e.g. `cp -p` or a git checkout of an older file).
        let f = std::fs::File::options().write(true).open(t.0.join("archive.json")).unwrap();
        f.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000)).unwrap();
        drop(f);
        assert_ne!(before, fingerprint(&t.0));
        let before = fingerprint(&t.0);
        std::fs::write(t.0.join("curated").join("extra.json"), "{}").unwrap();
        assert_ne!(before, fingerprint(&t.0));
    }

    #[test]
    fn broken_file_keeps_last_good_data_and_retries() {
        let t = TempData::new("broken");
        let mut r = Reloader::new(&t.0);
        std::fs::write(t.manual(), "{ not json").unwrap();
        assert!(matches!(r.poll(), Some(Err(_))));
        assert!(matches!(r.poll(), Some(Err(_))), "retries while still broken");
        std::fs::copy(real_dir().join("curated").join("manual-pools.json"), t.manual()).unwrap();
        edit_manual(&t, |v| v["pools"][0]["name"] = serde_json::json!("Fixed"));
        let d = r.poll().expect("change detected").expect("loads again");
        assert!(d.pools.iter().any(|p| p.name == "Fixed"));
        assert!(r.poll().is_none(), "settles once good");
    }

    #[test]
    fn same_size_edit_in_same_tick_is_seen() {
        let t = TempData::new("tick");
        let mut r = Reloader::new(&t.0);
        let raw = std::fs::read_to_string(t.manual()).unwrap();
        let mtime = std::fs::metadata(t.manual()).unwrap().modified().unwrap();
        // Swap two characters inside a name: same size, and pin the old mtime.
        let i = raw.find("\"name\": \"").unwrap() + 9;
        let mut b = raw.into_bytes();
        b.swap(i, i + 1);
        std::fs::write(t.manual(), &b).unwrap();
        std::fs::File::options().write(true).open(t.manual()).unwrap().set_modified(mtime).unwrap();
        assert!(r.poll().is_some(), "content change detected despite equal size and mtime");
    }

    #[test]
    fn live_rows_keep_refreshed_fee() {
        let base = serde_json::json!({"id": "x", "name": "Old", "fee_pct": 0.5, "fetched_at": "2026-10-02T00:00:00Z", "schemes": [{"scheme": "PPLNS", "fee_pct": 0.5}]});
        let manual = serde_json::json!({"id": "x", "name": "New", "fee_pct": 0, "live": "zecwec", "verified_at": "2026-01-01T00:00:00Z", "schemes": [{"scheme": "PPLNS", "fee_pct": 0}, {"scheme": "SOLO", "fee_pct": 0}]});
        let m = merge_manual(Some(&base), &manual);
        assert_eq!(m["name"], "New");
        assert_eq!(m["fee_pct"], 0.5);
        // The row's time is the hand verification, not the fee refresh (round-3 timestamp bug).
        assert_eq!(m["fetched_at"], "2026-01-01T00:00:00Z");
        assert_eq!(m["schemes"][1]["fee_pct"], 0.5);
        assert!(m.get("live").is_none() && m.get("verified_at").is_none());
        // Not live: the hand-verified time becomes fetched_at.
        let m = merge_manual(Some(&base), &serde_json::json!({"id": "x", "verified_at": "2026-01-01T00:00:00Z"}));
        assert_eq!(m["fetched_at"], "2026-01-01T00:00:00Z");
    }

    // ---- bug 2: region normalisation ----

    #[test]
    fn every_region_value_in_the_data_maps_to_a_bucket() {
        let d = load(&real_dir()).unwrap();
        for p in &d.pools {
            let (_, unknown) = region_tags_checked(p);
            assert!(unknown.is_empty(), "pool {} has unmapped region values {:?}", p.id, unknown);
            if p.region.as_deref().map(|r| !r.trim().is_empty()).unwrap_or(false) {
                assert!(!p.region_tags.is_empty(), "pool {} has region {:?} but no tags", p.id, p.region);
            }
        }
        // Raw files too, including the curated rows.
        for path in [generated_path(&real_dir(), "pools.json"), real_dir().join("curated/manual-pools.json")] {
            let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
            let f = path.display();
            for row in v["pools"].as_array().unwrap() {
                let mut toks = vec![];
                if let Some(r) = row["region"].as_str() {
                    toks.extend(region_tokens(r));
                }
                for r in row["regions"].as_array().into_iter().flatten().filter_map(|x| x.as_str()) {
                    toks.extend(region_tokens(r));
                }
                if let Some(cc) = row["country_code"].as_str() {
                    toks.push(cc.to_string());
                }
                for t in toks {
                    assert!(region_bucket(&t).is_some(), "{f}: region token {t:?} not mapped");
                }
            }
        }
    }

    #[test]
    fn countries_and_codes_land_in_the_right_bucket() {
        let cases: &[(&str, &str)] = &[
            ("RU", "Russia"), ("Russia", "Russia"), ("Canada", "North America"), ("CA", "North America"), ("United States", "North America"),
            ("US", "North America"), ("France", "Europe"), ("Germany", "Europe"), ("Czech Republic", "Europe"), ("Hungary", "Europe"),
            ("Poland", "Europe"), ("EU", "Europe"), ("Europe", "Europe"), ("Hong Kong", "Asia"), ("Singapore", "Asia"), ("IN", "Asia"),
            ("ASIA", "Asia"), ("asia", "Asia"), ("South America", "South America"), ("Middle East", "Middle East"), ("Global", "Global"),
            ("Global (multi-region)", "Global"), ("WL", "Global"), ("North America", "North America"),
        ];
        for (input, want) in cases {
            assert_eq!(region_bucket(input), Some(*want), "{input}");
        }
    }

    #[test]
    fn no_substring_false_positives() {
        // v1 matched " us" / "eu" / " in" as substrings.
        assert_eq!(region_tags(&pool(Some("Russia"), &[], None)), vec!["Russia"]);
        assert_eq!(region_tags(&pool(Some("Hungary"), &[], None)), vec!["Europe"]);
        assert_eq!(region_tags(&pool(Some("Singapore"), &[], None)), vec!["Asia"]);
        assert_eq!(region_tags(&pool(Some("EU, IN, CA"), &[], None)), vec!["North America", "Europe", "Asia"]);
        assert_eq!(region_tags(&pool(Some("RU, EU, US, Asia"), &[], None)), vec!["North America", "Europe", "Russia", "Asia"]);
    }

    #[test]
    fn country_code_is_a_fallback_only() {
        assert_eq!(region_tags(&pool(None, &[], Some("DE"))), vec!["Europe"]);
        assert_eq!(region_tags(&pool(Some("Canada"), &[], Some("WL"))), vec!["North America"]);
        assert!(region_tags(&pool(None, &[], None)).is_empty());
    }

    // ---- bug 3 (data side): schemes are canonical and all reachable ----

    #[test]
    fn payout_schemes_include_everything_in_the_data() {
        let d = load(&real_dir()).unwrap();
        let all: Vec<String> = d.payout_schemes().into_iter().map(|(s, _)| s).collect();
        for s in ["PPLNT", "PPLNSBF", "PPLNS", "PPS", "PPS+", "FPPS", "PROP", "SOLO", "D-PPS"] {
            assert!(all.iter().any(|x| x == s), "{s} missing: {all:?}");
        }
        assert_eq!(canonical_scheme(" pplns "), "PPLNS");
    }

    // ---- round 3: data presentation ----

    /// A frozen copy of data/ from the 2 Oct 2026 refresh, so snapshot assertions survive refreshes.
    fn snap_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata").join("snapshot-2026-10-02")
    }
    fn ts(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&chrono::Utc)
    }
    fn coin(name: &str, nk: Option<(u32, u32)>, rep: Option<f64>) -> Coin {
        Coin {
            id: name.to_lowercase().replace(' ', "-"),
            name: name.into(),
            status: "active".into(),
            equihash: nk.map(|(n, k)| Equihash { n: Some(n), k: Some(k) }),
            reported: Reported { hashrate: rep, ..Default::default() },
            ..Default::default()
        }
    }
    fn prow(coin_id: &str, h: Option<f64>) -> Pool {
        Pool { coin_id: coin_id.into(), hashrate: h, ..Default::default() }
    }

    #[test]
    fn ranking_200_9_order_matches_snapshot() {
        let d = load(&snap_dir()).unwrap();
        let groups = d.param_groups(false);
        assert_eq!(groups[0].0, "Equihash 200,9");
        let names: Vec<&str> = groups[0].1.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(
            names,
            ["Zcash", "Pirate Chain", "Wcash", "Kerrigan", "Komodo", "Buck", "BitMark", "Hush", "Marmara", "Spacecoin", "Squishy Coin", "Tokel", "ZEN"],
            "200,9 order"
        );
        let hr = |n: &str| groups[0].1.iter().find(|c| c.name == n).unwrap().reported.hashrate;
        let close = |a: Option<f64>, b: f64| (a.unwrap() / b - 1.0).abs() < 0.005;
        assert!(close(hr("Zcash"), 30.36e9));
        assert!(close(hr("Pirate Chain"), 4.22e6));
        assert_eq!(hr("Wcash"), Some(440_000.0));
        assert!(close(hr("Kerrigan"), 358e3));
        assert!(close(hr("Komodo"), 148e3));
        assert!(close(hr("Buck"), 3.72e3));
        assert!(close(hr("BitMark"), 1.89e3));
        for c in &groups[0].1[7..] {
            assert!(c.reported.hashrate.map(|h| h <= 0.0).unwrap_or(true), "{} should be zero or n/a", c.name);
        }
    }

    #[test]
    fn ranking_never_interleaves_parameter_sets() {
        for dir in [snap_dir(), real_dir()] {
            let d = load(&dir).unwrap();
            // Groups are contiguous and in key order.
            for w in d.coins.windows(2) {
                assert!(w[0].param_key() <= w[1].param_key(), "{} before {}", w[0].name, w[1].name);
                if w[0].param_key() == w[1].param_key() {
                    assert_ne!(rank_cmp(&w[0], &w[1]), std::cmp::Ordering::Greater);
                }
            }
            let labels: Vec<String> = d.param_groups(false).into_iter().map(|(l, _)| l).collect();
            let mut dedup = labels.clone();
            dedup.dedup();
            assert_eq!(labels, dedup, "a parameter set appears twice");
        }
    }

    #[test]
    fn zero_and_unavailable_rank_last_by_name_within_their_group() {
        let mut v = vec![
            coin("Zulu", Some((200, 9)), None),
            coin("Huge Other Set", Some((144, 5)), Some(1e15)),
            coin("Beta", Some((200, 9)), Some(0.0)),
            coin("Alpha", Some((200, 9)), None),
            coin("Small", Some((200, 9)), Some(5.0)),
            coin("Big", Some((200, 9)), Some(10.0)),
            coin("Mystery", None, Some(1e18)),
        ];
        v.sort_by(rank_cmp);
        let n: Vec<&str> = v.iter().map(|c| c.name.as_str()).collect();
        // A huge hashrate on another parameter set never jumps ahead of 200,9.
        assert_eq!(n, ["Big", "Small", "Alpha", "Beta", "Zulu", "Huge Other Set", "Mystery"]);
    }

    #[test]
    fn fee_refresh_never_bumps_the_hashrate_time() {
        // pools.json after a refresh that re-read only ZecWec's fee.
        let base = serde_json::json!({
            "id": "zcash:zecwec.com", "hashrate": 440000, "fee_pct": 0.25,
            "fee_source": "https://pool.zecwec.com/api/v1/overview", "fee_observed_at": "2026-10-03T09:00:00Z",
            "fetched_at": "2026-10-03T09:00:00Z", "hashrate_observed_at": "2026-10-03T09:00:00Z",
            "schemes": [{"scheme": "PPLNS", "fee_pct": 0.25}]
        });
        let manual = serde_json::json!({
            "id": "zcash:zecwec.com", "live": "zecwec", "verified_at": "2026-10-02T23:29:00Z",
            "hashrate": 440000, "hashrate_source": "https://pool.zecwec.com/", "fee_pct": 0,
            "schemes": [{"scheme": "PPLNS", "fee_pct": 0}]
        });
        let m = merge_manual(Some(&base), &manual);
        assert_eq!(m["hashrate_observed_at"], "2026-10-02T23:29:00Z", "hashrate time must stay at verification");
        assert_eq!(m["fetched_at"], "2026-10-02T23:29:00Z", "row time must not follow the fee");
        assert_eq!(m["fee_observed_at"], "2026-10-03T09:00:00Z", "fee time follows the fee");
        assert_eq!(m["fee_pct"], 0.25);
        assert_eq!(m["fee_source"], "https://pool.zecwec.com/api/v1/overview");
        for f in ["miners", "blocks", "min_payout"] {
            assert_eq!(m[format!("{f}_observed_at")], "2026-10-02T23:29:00Z");
        }
    }

    #[test]
    fn loaded_live_rows_keep_the_verified_hashrate_time() {
        for dir in [snap_dir(), real_dir()] {
            let d = load(&dir).unwrap();
            let manual: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(dir.join("curated/manual-pools.json")).unwrap()).unwrap();
            for row in manual["pools"].as_array().unwrap() {
                let (Some(id), Some(v)) = (row["id"].as_str(), row["verified_at"].as_str()) else { continue };
                let p = d.pools.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("{id} missing"));
                let live_hashrate = row["live_fields"].as_array().map(|a| a.iter().any(|x| x == "hashrate")).unwrap_or(false);
                if row.get("hashrate_observed_at").is_none() && !live_hashrate {
                    assert_eq!(ts(p.hashrate_observed_at.as_deref().unwrap()), ts(v), "{id} hashrate time");
                }
                assert_eq!(ts(p.fetched_at.as_deref().unwrap()), ts(v), "{id} row time");
            }
        }
        // And the ZecWec fee was observed after its hashrate in the snapshot: two separate clocks.
        let d = load(&snap_dir()).unwrap();
        let z = d.pools.iter().find(|p| p.id == "zcash:zecwec.com").unwrap();
        assert!(ts(z.fee_observed_at.as_deref().unwrap()) > ts(z.hashrate_observed_at.as_deref().unwrap()));
        assert_eq!(z.basis.as_deref(), Some("operator_reported"));
    }

    #[test]
    fn every_row_carries_per_field_provenance_and_a_valid_basis() {
        let d = load(&real_dir()).unwrap();
        for p in &d.pools {
            assert!(matches!(p.basis.as_deref(), Some("listed_pools" | "pool_api" | "operator_reported" | "estimated" | "network") | None), "{} basis {:?}", p.id, p.basis);
            if p.hashrate.is_some() {
                assert!(p.hashrate_source.is_some() && p.hashrate_observed_at.is_some(), "{} hashrate provenance", p.id);
                assert!(p.basis.is_some(), "{} has a hashrate but no basis", p.id);
            }
            if p.fee_pct.is_some() || p.schemes.iter().any(|s| s.fee_pct.is_some()) {
                assert!(p.fee_source.is_some() && p.fee_observed_at.is_some(), "{} fee provenance", p.id);
            }
        }
        for c in &d.coins {
            if c.network.hashrate.is_some() {
                assert_eq!(c.network.basis.as_deref(), Some("network"), "{}", c.id);
                assert!(c.network.hashrate_observed_at.is_some(), "{}", c.id);
            }
        }
        // Wcash: the network figure, when present, comes from a network source, never from the pools' total.
        let w = d.coin("wcash").unwrap();
        if w.network.hashrate.is_some() {
            assert_eq!(w.network.basis.as_deref(), Some("network"));
            assert!(w.network.hashrate_source.as_deref().unwrap().ends_with("/hashrate/network"));
        }
    }

    #[test]
    fn unavailable_is_not_zero() {
        let pools = vec![prow("a", None), prow("b", Some(0.0)), prow("b", None), prow("c", Some(0.0)), prow("c", Some(7.0))];
        assert_eq!(reported_for("none", &pools).hashrate, None, "no rows: n/a");
        assert_eq!(reported_for("a", &pools).hashrate, None, "only n/a rows: n/a");
        let b = reported_for("b", &pools);
        assert_eq!(b.hashrate, Some(0.0), "a published zero stays zero");
        assert_eq!(b.positive_pools, 0);
        assert_eq!(reported_for("c", &pools).hashrate, Some(7.0));
        // Raw nulls load as None, raw zeros as Some(0).
        let raw: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(snap_dir().join("pools.json")).unwrap()).unwrap();
        let d = load(&snap_dir()).unwrap();
        for row in raw["pools"].as_array().unwrap() {
            let id = row["id"].as_str().unwrap();
            if let Some(p) = d.pools.iter().find(|p| p.id == id && p.from_miningpoolstats) {
                match row["hashrate"].as_f64() {
                    None => assert_eq!(p.hashrate, None, "{id}"),
                    Some(h) => assert_eq!(p.hashrate, Some(h), "{id}"),
                }
            }
        }
        assert!(d.pools.iter().any(|p| p.hashrate.is_none()) && d.pools.iter().any(|p| p.hashrate == Some(0.0)));
    }

    #[test]
    fn headline_counts_rows_not_pools() {
        let mut ended = coin("Gone", Some((200, 9)), None);
        ended.status = "ended".into();
        let d = Data {
            coins: vec![coin("A", Some((200, 9)), None), ended],
            pools: vec![prow("a", Some(10.0)), prow("a", Some(0.0)), prow("a", None), prow("gone", Some(5.0))],
            ..Default::default()
        };
        let h = d.headline();
        assert_eq!((h.rows, h.positive, h.zero, h.unavailable, h.coins_with_rows), (3, 1, 1, 1, 1));
        let h = load(&snap_dir()).unwrap().headline();
        assert_eq!((h.rows, h.positive), (117, 63));
        assert_eq!(h.rows, h.positive + h.zero + h.unavailable);
    }

    #[test]
    fn staleness() {
        let now = ts("2026-10-03T12:00:00Z");
        assert_eq!(age_secs(Some("2026-10-03T11:00:00Z"), now), Some(3600));
        assert!(!is_stale(Some("2026-10-03T11:00:00Z"), now));
        assert!(!is_stale(Some("2026-10-03T10:00:00Z"), now), "exactly 2h is not yet stale");
        assert!(is_stale(Some("2026-10-03T09:59:00Z"), now));
        assert!(is_stale(None, now), "unknown age counts as stale");
        assert!(is_stale(Some("garbage"), now));
    }

    fn write_links(t: &TempData, v: serde_json::Value) {
        std::fs::write(t.0.join("curated").join("links.json"), serde_json::to_string_pretty(&v).unwrap()).unwrap();
    }
    fn l(kind: &str, url: &str, status: &str) -> serde_json::Value {
        serde_json::json!({"kind": kind, "url": url, "label": format!("{kind} label"), "source_url": url, "verified_at": "2026-10-03T10:00:00Z", "status": status})
    }

    #[test]
    fn links_load_verified_only_and_stay_separate() {
        let t = TempData::new("links");
        write_links(&t, serde_json::json!({
            "generated_at": "2026-10-03T10:00:00Z",
            "coins": {"zcash": [l("website", "https://z.cash/", "verified"), l("x", "https://x.com/zcash", "unverified"), l("discord", "https://discord.gg/dead", "dead"), l("myspace", "https://myspace.com/z", "verified"), l("github", "javascript:alert(1)", "verified")]},
            "pools": {"zcash:zecwec.com": [l("telegram", "https://t.me/zecwec", "verified")]}
        }));
        let d = load(&t.0).unwrap();
        let z = d.coin("zcash").unwrap();
        assert_eq!(z.links.iter().map(|x| x.url.as_str()).collect::<Vec<_>>(), ["https://z.cash/"]);
        let p = d.pools.iter().find(|p| p.id == "zcash:zecwec.com").unwrap();
        assert_eq!(p.links.iter().map(|x| x.kind.as_str()).collect::<Vec<_>>(), ["telegram"]);
        // Pool links never leak onto the coin, or onto the same pool's row on another coin.
        assert!(!z.links.iter().any(|x| x.url.contains("t.me")));
        assert!(d.pools.iter().find(|p| p.id == "wcash:zecwec.com").unwrap().links.is_empty());
        assert_eq!(d.links_generated_at.as_deref(), Some("2026-10-03T10:00:00Z"));
        // A broken links.json is an error, so the reloader keeps serving the last good data.
        std::fs::write(t.0.join("curated").join("links.json"), "{ not json").unwrap();
        assert!(load(&t.0).is_err());
        // A missing file means no links, not an error.
        std::fs::remove_file(t.0.join("curated").join("links.json")).unwrap();
        assert!(load(&t.0).unwrap().coins.iter().all(|c| c.links.is_empty()));
    }

    #[test]
    fn links_tolerate_a_note_field_and_the_wcash_x_override_loads() {
        let t = TempData::new("links-note");
        let mut x = l("x", "https://x.com/WcashProject", "verified");
        x["note"] = serde_json::json!("manual override");
        write_links(&t, serde_json::json!({"generated_at": null, "coins": {"wcash": [x, l("x", "https://x.com/other", "unverified")]}, "pools": {}}));
        let d = load(&t.0).unwrap();
        let w = d.coin("wcash").unwrap();
        assert_eq!(w.links.iter().map(|x| x.url.as_str()).collect::<Vec<_>>(), ["https://x.com/WcashProject"]);
        assert!(!serde_json::to_string(&w.links).unwrap().contains("manual override"), "notes stay out of the page");
        // The real file, when present, keeps the hand-set Wcash X account.
        let real = load(&real_dir()).unwrap();
        if let Some(x) = real.coin("wcash").unwrap().links.iter().find(|l| l.kind == "x") {
            assert_eq!(x.url, "https://x.com/WcashProject");
        }
    }

    #[test]
    fn zprominers_loads_as_a_real_pool_with_its_x_link_and_official_logo() {
        let d = load(&real_dir()).unwrap();
        let p = d.pools.iter().find(|p| p.id == "zcash:zprominers.com").expect("ZProMiners row");
        assert_eq!(p.coin_id, "zcash");
        assert_eq!(p.slug, "zcash-zprominers-zprominers-com");
        assert_eq!(p.fee_range(), Some((1.0, 2.0)));
        assert!(p.links.iter().any(|l| l.kind == "x" && l.url == "https://x.com/ZProMiners"));
        assert_eq!(p.logo.kind, "official");
        assert!(p.logo.src.as_deref().unwrap_or_default().contains("/static/logos/pools/zprominers.com."));
    }

    #[test]
    fn reloader_picks_up_links_json() {
        let t = TempData::new("links-reload");
        let mut r = Reloader::new(&t.0);
        assert!(r.poll().is_none());
        write_links(&t, serde_json::json!({"generated_at": null, "coins": {"zcash": [l("website", "https://z.cash/", "verified")]}, "pools": {}}));
        let d = r.poll().expect("links.json change detected").expect("loads");
        assert_eq!(d.coin("zcash").unwrap().links.len(), 1);
    }

    // ---- live sources (ZecWec hashrate endpoints) ----

    fn live_state(pool: f64, net: f64) -> crate::live::LiveState {
        use crate::live::{Parsed, Reading};
        let now = chrono::Utc::now();
        // Stamped "now", so the reading is never older than what a refresh just wrote to the files.
        let at = now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
        let mut st = crate::live::LiveState::default();
        st.record("zecwec-pool", Ok(Parsed::Ok(Reading { hashrate: pool, observed_at: at.clone(), window_seconds: Some(1200), sample_blocks: None, height: None })), now);
        st.record("zecwec-wcash-network", Ok(Parsed::Ok(Reading { hashrate: net, observed_at: at, window_seconds: None, sample_blocks: Some(120), height: Some(16143) })), now);
        st
    }

    #[test]
    fn share_is_capped_at_100() {
        assert_eq!(share_pct(Some(50.0), Some(100.0)), (Some(50.0), false));
        assert_eq!(share_pct(Some(101.5), Some(100.0)), (Some(100.0), true));
        assert_eq!(share_pct(None, Some(100.0)), (None, false));
        assert_eq!(share_pct(Some(1.0), Some(0.0)), (None, false));
        // Every loaded row, live or not, stays at or below 100%.
        let t = TempData::new("cap");
        let d = load_with_live(&t.0, Some(&live_state(553_587.0, 507_977.0))).unwrap();
        assert!(d.pools.iter().all(|p| p.network_share_pct.map(|s| s <= 100.0).unwrap_or(true)));
    }

    #[test]
    fn only_listed_pool_above_the_network_shows_both_numbers() {
        let t = TempData::new("solo");
        let d = load_with_live(&t.0, Some(&live_state(553_587.0, 507_977.0))).unwrap();
        let w = d.coin("wcash").unwrap();
        assert_eq!(w.network.hashrate, Some(507_977.0));
        assert_eq!(w.network.basis.as_deref(), Some("network"));
        assert_eq!(w.network.hashrate_source.as_deref(), Some("https://pool.zecwec.com/api/v1/hashrate/network"));
        assert_eq!(w.network.hashrate_sample_blocks, Some(120));
        let p = d.pools.iter().find(|p| p.id == "wcash:zecwec.com").unwrap();
        assert_eq!(p.hashrate, Some(553_587.0));
        assert_eq!(p.basis.as_deref(), Some("pool_api"));
        assert!(!p.operator_reported(), "the live figure replaces the operator-reported one");
        assert_eq!(w.reported.operator_pools, 0, "no † on Wcash any more");
        let note = p.share_note.as_deref().unwrap();
        assert!(note.contains("only listed pool") && note.contains("20-minute") && note.contains("554 kSol/s") && note.contains("508 kSol/s") && note.contains("120 blocks"), "{note}");
        assert!(!p.share_capped);
        // The fee keeps its own (refreshed) time; the row time stays the hand verification.
        // Read both from the copied files, so the test doesn't break on every refresh.
        let pj: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(t.0.join("pools.json")).unwrap()).unwrap();
        let row = pj["pools"].as_array().unwrap().iter().find(|r| r["id"] == "wcash:zecwec.com").expect("refresh wrote the WEC row");
        assert_eq!(p.fee_observed_at.as_deref(), row["fee_observed_at"].as_str());
        let mj: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(t.manual()).unwrap()).unwrap();
        let verified = mj["pools"].as_array().unwrap().iter().find(|r| r["id"] == "wcash:zecwec.com").unwrap()["verified_at"].as_str().map(String::from);
        assert_eq!(p.fetched_at, verified);
        // Ranking still uses the pools' sum inside 200,9: Wcash stays between Pirate Chain and Kerrigan.
        let g = &d.param_groups(true)[0].1;
        let pos = |n: &str| g.iter().position(|c| c.name == n).unwrap();
        assert!(pos("Pirate Chain") < pos("Wcash") && pos("Wcash") < pos("Kerrigan"));
        // Below the network estimate it's a normal share.
        let d = load_with_live(&t.0, Some(&live_state(400_000.0, 507_977.0))).unwrap();
        let p = d.pools.iter().find(|p| p.id == "wcash:zecwec.com").unwrap();
        assert!(p.share_note.is_none() && (p.network_share_pct.unwrap() - 78.74).abs() < 0.1);
    }

    /// The (hashrate, basis, source, observed_at, window) both ZecWec rows show.
    fn zecwec_pair(d: &Data) -> [(Option<f64>, Option<String>, Option<String>, Option<String>, Option<u64>); 2] {
        ["zcash:zecwec.com", "wcash:zecwec.com"].map(|id| {
            let p = d.pools.iter().find(|p| p.id == id).unwrap_or_else(|| panic!("{id} missing"));
            (p.hashrate, p.basis.clone(), p.hashrate_source.clone(), p.hashrate_observed_at.clone(), p.hashrate_window_s)
        })
    }

    fn edit_pools(t: &TempData, f: impl Fn(&mut serde_json::Value)) {
        let path = t.0.join("pools.json");
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for row in v["pools"].as_array_mut().unwrap() {
            f(row);
        }
        std::fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();
    }

    #[test]
    fn zecwec_zec_and_wec_rows_always_show_the_same_pool_hashrate() {
        let t = TempData::new("zecwec-same");
        // 1. Files only (no poller yet): the same value, from the pool API, no †.
        let d = load(&t.0).unwrap();
        let [z, w] = zecwec_pair(&d);
        assert_eq!(z, w);
        if z.0.is_some() {
            assert_eq!(z.1.as_deref(), Some("pool_api"));
            assert_eq!(z.2.as_deref(), Some("https://pool.zecwec.com/api/v1/hashrate/pool"));
        }
        assert_eq!(d.coin("zcash").unwrap().reported.operator_pools, 0, "no operator-reported figure on Zcash any more");
        assert!(d.pools.iter().filter(|p| p.name == "ZecWec").all(|p| !p.operator_reported()));
        assert!(d.pools.iter().filter(|p| p.name == "ZecWec").all(|p| p.notes.as_deref().unwrap_or("").starts_with("Merge-mines ZEC and WEC with the same hashrate.")));
        // 2. Files that disagree (an old operator figure on one row): both take the API reading.
        edit_pools(&t, |row| match row["id"].as_str() {
            Some("zcash:zecwec.com") => {
                row["hashrate"] = serde_json::json!(440000);
                row["basis"] = serde_json::json!("operator_reported");
                row["hashrate_is_reported"] = serde_json::json!(true);
                row["hashrate_observed_at"] = serde_json::json!("2026-10-02T23:29:00.000Z");
            }
            Some("wcash:zecwec.com") => {
                row["hashrate"] = serde_json::json!(561866);
                row["basis"] = serde_json::json!("pool_api");
                row["hashrate_source"] = serde_json::json!("https://pool.zecwec.com/api/v1/hashrate/pool");
                row["hashrate_observed_at"] = serde_json::json!("2026-10-03T11:36:38Z");
                row["hashrate_window_s"] = serde_json::json!(1200);
            }
            _ => {}
        });
        let d = load(&t.0).unwrap();
        let [z, w] = zecwec_pair(&d);
        assert_eq!(z, w);
        assert_eq!(z.0, Some(561866.0));
        assert_eq!(z.3.as_deref(), Some("2026-10-03T11:36:38Z"));
        // 3. A newer poller reading moves both rows together; an older one moves neither.
        let mut st = crate::live::LiveState::default();
        let rd = |h: f64, at: &str| Ok(crate::live::Parsed::Ok(crate::live::Reading { hashrate: h, observed_at: at.into(), window_seconds: Some(1200), sample_blocks: None, height: None }));
        st.record("zecwec-pool", rd(600_000.0, "2026-10-03T12:00:00Z"), chrono::Utc::now());
        let d = load_with_live(&t.0, Some(&st)).unwrap();
        let [z, w] = zecwec_pair(&d);
        assert_eq!(z, w);
        assert_eq!(z.0, Some(600_000.0));
        st.record("zecwec-pool", rd(1.0, "2026-10-03T10:00:00Z"), chrono::Utc::now());
        let d = load_with_live(&t.0, Some(&st)).unwrap();
        let [z, w] = zecwec_pair(&d);
        assert_eq!(z, w);
        assert_eq!(z.0, Some(561866.0), "an older poller reading doesn't win over the files");
        // 4. No reading anywhere: both n/a, never the old operator figure on one of them.
        edit_pools(&t, |row| {
            if row["id"] == "wcash:zecwec.com" {
                row["hashrate"] = serde_json::Value::Null;
                row["basis"] = serde_json::Value::Null;
            }
        });
        let d = load(&t.0).unwrap();
        let [z, w] = zecwec_pair(&d);
        assert_eq!(z, w);
        assert_eq!(z.0, None);
    }

    #[test]
    fn api_live_ranking_follows_live_readings() {
        let t = TempData::new("rank-live");
        let kmd = load(&t.0).unwrap().coin("komodo").unwrap().reported.hashrate.expect("Komodo pools report hashrate");
        let now = chrono::Utc::now();
        let ids = |j: &serde_json::Value| j["ranking"].as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
        let pos = |v: &[String], id: &str| v.iter().position(|x| x == id).unwrap();
        for (pool, wcash_first) in [(kmd * 3.0, true), (kmd / 3.0, false)] {
            let d = load_with_live(&t.0, Some(&live_state(pool, 500_000.0))).unwrap();
            let j = crate::views::live_json(&d, now);
            let v = ids(&j);
            // Exactly the order the server renders every list in (d.coins, sorted by rank_cmp).
            assert_eq!(v, d.coins.iter().map(|c| c.id.clone()).collect::<Vec<_>>());
            let flat: Vec<String> = d.param_groups(false).into_iter().flat_map(|(_, cs)| cs.into_iter().map(|c| c.id.clone())).collect();
            assert_eq!(v, flat, "groups are contiguous");
            for w in d.coins.windows(2) {
                assert_ne!(rank_cmp(&w[0], &w[1]), std::cmp::Ordering::Greater);
            }
            assert_eq!(pos(&v, "wcash") < pos(&v, "komodo"), wcash_first, "live WEC pool reading {pool}");
            let w = &j["ranking"][pos(&v, "wcash")];
            assert_eq!(w["group"], "Equihash 200,9");
            assert_eq!(w["reported_text"], crate::fmt::reported(d.coin("wcash").unwrap()).0);
            assert!(w["option_text"].as_str().unwrap().starts_with("Wcash WEC · "), "{w}");
            assert!(w["title"].as_str().unwrap().starts_with("Wcash (WEC): "), "{w}");
        }
    }

    #[test]
    fn extreme_network_vs_pool_gap_is_flagged_either_way() {
        let mut c = Coin { name: "X".into(), ..Default::default() };
        c.network.hashrate = Some(15.4e9);
        c.reported.hashrate = Some(1080.0);
        assert!(c.discrepancy().unwrap() > 1e7);
        let n = crate::fmt::discrepancy_note(&c).unwrap();
        assert!(n.contains("15.4 GSol/s") && n.contains("1.08 kSol/s") && n.contains("about 14 million") && n.contains("network estimate the larger"), "{n}");
        // The other direction (listed pools far above the estimate) is flagged too.
        c.network.hashrate = Some(3.0);
        c.reported.hashrate = Some(17_100.0);
        assert!(crate::fmt::discrepancy_note(&c).unwrap().contains("listed pools' total the larger"));
        // Ordinary coverage gaps are not: 51× is a poorly covered coin, not a data error.
        c.network.hashrate = Some(580_000.0);
        c.reported.hashrate = Some(11_300.0);
        assert!(c.discrepancy().is_none());
        c.network.hashrate = Some(DISCREPANCY_RATIO);
        c.reported.hashrate = Some(1.0);
        assert!(c.discrepancy().is_some(), "the threshold is inclusive");
        // Both figures are needed; zero and n/a are not a gap.
        c.reported.hashrate = Some(0.0);
        assert!(c.discrepancy().is_none());
        c.reported.hashrate = None;
        assert!(c.discrepancy().is_none());
        c.network.hashrate = None;
        c.reported.hashrate = Some(5.0);
        assert!(c.discrepancy().is_none());
        assert_eq!(crate::fmt::factor(5_730.0), "5,700");
        assert_eq!(crate::fmt::factor(140.4), "140");
        assert_eq!(crate::fmt::factor(2.6e9), "2.6 billion");
    }

    #[test]
    fn manual_live_fields_keep_the_refreshed_hashrate() {
        let base = serde_json::json!({"id": "w", "hashrate": 553587, "basis": "pool_api", "hashrate_source": "https://pool.zecwec.com/api/v1/hashrate/pool", "hashrate_observed_at": "2026-10-03T11:09:23Z", "hashrate_window_s": 1200, "live_source": "zecwec-pool", "fee_pct": 0, "fee_observed_at": "2026-10-03T11:00:00Z"});
        let manual = serde_json::json!({"id": "w", "live": "zecwec", "live_fields": ["fee", "hashrate"], "hashrate": null, "basis": null, "hashrate_is_reported": false, "verified_at": "2026-10-02T23:29:00Z"});
        let m = merge_manual(Some(&base), &manual);
        assert_eq!(m["hashrate"], 553587);
        assert_eq!(m["basis"], "pool_api");
        assert_eq!(m["hashrate_observed_at"], "2026-10-03T11:09:23Z");
        assert_eq!(m["fee_observed_at"], "2026-10-03T11:00:00Z");
        assert_eq!(m["fetched_at"], "2026-10-02T23:29:00Z");
        assert!(m.get("live_fields").is_none());
        // With no refreshed row yet, the curated (null) hashrate is n/a, not zero.
        let m = merge_manual(None, &manual);
        assert!(m["hashrate"].is_null());
    }

    #[test]
    fn live_sources_config_parses_and_points_at_real_rows() {
        let cfg = crate::live::read_config(&real_dir()).unwrap();
        assert!((30..=60).contains(&cfg.interval()));
        let d = load(&real_dir()).unwrap();
        for s in &cfg.sources {
            assert!(s.url.starts_with("https://"), "{}", s.id);
            match s.target.as_str() {
                "pool" => {
                    assert!(!s.pool_ids().is_empty(), "{} feeds no pool", s.id);
                    for id in s.pool_ids() {
                        assert!(d.pools.iter().any(|p| p.id == id), "{} pool_id {id}", s.id);
                    }
                }
                "network" => assert!(d.coin(s.coin_id.as_deref().unwrap_or("")).is_some(), "{} coin_id", s.id),
                t => panic!("{}: unknown target {t}", s.id),
            }
        }
        // ZecWec stays the only listed WEC pool.
        assert_eq!(d.pools.iter().filter(|p| p.coin_id == "wcash").count(), 1);
    }

    // ---- snapshots, permalinks ----

    /// Publish the temp dir's flat generated files as snapshot `id`, the way scripts/snapshot.mjs
    /// does: complete directory first, then the manifest swapped in by rename.
    fn publish(t: &TempData, id: &str) {
        let sdir = t.0.join("snapshots").join(id);
        std::fs::create_dir_all(&sdir).unwrap();
        let mut files = serde_json::Map::new();
        for f in GENERATED {
            let bytes = std::fs::read(t.0.join(f)).unwrap();
            std::fs::write(sdir.join(f), &bytes).unwrap();
            files.insert(f.to_string(), serde_json::json!({"bytes": bytes.len(), "sha256": sha256_hex(&bytes)}));
        }
        let m = serde_json::json!({"snapshot": id, "published_at": "2026-10-03T15:00:00Z", "files": files});
        let tmp = t.0.join(".current.json.tmp-1");
        std::fs::write(&tmp, serde_json::to_vec(&m).unwrap()).unwrap();
        std::fs::rename(tmp, t.0.join(MANIFEST)).unwrap();
    }

    #[test]
    fn loads_the_snapshot_the_manifest_names_and_refuses_a_mismatched_file() {
        let t = TempData::new("snap");
        publish(&t, "20261003T150000Z");
        // The flat files are no longer read: break one to prove it.
        std::fs::write(t.0.join("pools.json"), "{broken").unwrap();
        let d = load(&t.0).unwrap();
        assert_eq!(d.snapshot.as_deref(), Some("20261003T150000Z"));
        assert_eq!(d.snapshot_published_at.as_deref(), Some("2026-10-03T15:00:00Z"));
        assert!(d.snapshot_dir.ends_with("snapshots/20261003T150000Z"));
        assert!(!d.pools.is_empty());
        // A file that doesn't match its manifest entry (size or SHA-256) is never parsed.
        let p = t.0.join("snapshots/20261003T150000Z/pools.json");
        let mut s = std::fs::read_to_string(&p).unwrap();
        s.push(' ');
        std::fs::write(&p, s).unwrap();
        assert!(load(&t.0).unwrap_err().contains("does not match current.json"));
        // Same size, different bytes: still refused.
        let raw = std::fs::read_to_string(&p).unwrap();
        std::fs::write(&p, format!("{}\n", raw.trim_end())).unwrap();
        assert!(load(&t.0).is_err());
        // Bad ids, a missing snapshot or an incomplete manifest are errors too.
        let e = serde_json::json!({"bytes": 1, "sha256": "00"});
        for (id, files) in [("../etc", true), (".hidden", true), ("missing", true), ("20261003T150000Z", false)] {
            let mut m = serde_json::json!({"snapshot": id, "files": {"pools.json": e, "network.json": e}});
            if files {
                m["files"]["meta.json"] = e.clone();
            }
            std::fs::write(t.0.join(MANIFEST), m.to_string()).unwrap();
            assert!(load(&t.0).is_err(), "{id}");
        }
    }

    #[test]
    fn reloader_switches_snapshots_as_one_set_and_keeps_the_last_good_one() {
        let t = TempData::new("snap-reload");
        publish(&t, "a1");
        let mut r = Reloader::new(&t.0);
        assert!(r.poll().is_none());
        let id = load(&t.0).unwrap().pools.iter().find(|p| p.from_miningpoolstats).unwrap().id.clone();
        // The next refresh renames one pool and is published as a new snapshot.
        edit_pools(&t, |row| {
            if row["id"] == id.as_str() {
                row["name"] = "Renamed Pool".into();
            }
        });
        publish(&t, "a2");
        let d = r.poll().expect("the manifest swap is noticed").unwrap();
        assert_eq!(d.snapshot.as_deref(), Some("a2"));
        assert_eq!(d.pools.iter().find(|p| p.id == id).unwrap().name, "Renamed Pool");
        // A manifest pointing at a snapshot that doesn't match: the error is reported and retried.
        std::fs::write(t.0.join("snapshots/a2/network.json"), "{}").unwrap();
        std::fs::write(t.0.join(MANIFEST), std::fs::read_to_string(t.0.join(MANIFEST)).unwrap().replace("a2", "a2")).unwrap();
        let mut r2 = Reloader::new(&t.0);
        std::fs::write(t.0.join(MANIFEST), std::fs::read_to_string(t.0.join(MANIFEST)).unwrap() + " ").unwrap();
        assert!(r2.poll().expect("change noticed").is_err(), "a2 is now corrupt: the server keeps serving what it has");
        assert!(r2.poll().expect("retried until it loads").is_err());
        // Rolling back to a1 (a manifest swap) loads again.
        publish(&t, "a3");
        let d = r2.poll().expect("change noticed").unwrap();
        assert_eq!(d.snapshot.as_deref(), Some("a3"));
    }

    #[test]
    fn a_renamed_pool_keeps_its_permalink_and_the_old_url_redirects() {
        let t = TempData::new("perma");
        let before = load(&t.0).unwrap();
        let p0 = before.pools.iter().find(|p| p.from_miningpoolstats && p.coin_id == "zcash").unwrap().clone();
        assert_eq!(p0.slug, p0.legacy_slug, "the permalinks started from the computed slugs, so no URL changed");
        edit_pools(&t, |row| {
            if row["id"] == p0.id.as_str() {
                row["name"] = "Brand New Name".into();
                row["url"] = "https://new-domain.example/".into();
            }
        });
        let after = load(&t.0).unwrap();
        let p1 = after.pools.iter().find(|p| p.id == p0.id).unwrap();
        assert_eq!(p1.slug, p0.slug, "permalink unchanged by a rename and a new domain");
        assert_eq!(p1.legacy_slug, "zcash-brand-new-name-new-domain-example");
        assert_eq!(after.slug_redirects.get(&p1.legacy_slug), Some(&p0.slug), "the name-based URL redirects");
        // Curated aliases redirect too; ones that point nowhere, or shadow a live slug, don't.
        let pf = t.0.join("curated").join(PERMALINKS);
        let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&pf).unwrap()).unwrap();
        let other = after.pools.iter().find(|p| p.id != p0.id).unwrap().slug.clone();
        v["aliases"] = serde_json::json!({"an-old-url": p0.slug, "dead-end": "no-such-pool", other.clone(): p0.slug});
        std::fs::write(&pf, v.to_string()).unwrap();
        let d = load(&t.0).unwrap();
        assert_eq!(d.slug_redirects.get("an-old-url"), Some(&p0.slug));
        assert!(!d.slug_redirects.contains_key("dead-end") && !d.slug_redirects.contains_key(&other));
        // A pool without an entry falls back to its computed slug, unique against assigned ones.
        v["pools"].as_object_mut().unwrap().remove(&p0.id);
        std::fs::write(&pf, v.to_string()).unwrap();
        let d = load(&t.0).unwrap();
        assert_eq!(d.pools.iter().find(|p| p.id == p0.id).unwrap().slug, "zcash-brand-new-name-new-domain-example");
        let mut slugs: Vec<&str> = d.pools.iter().map(|p| p.slug.as_str()).collect();
        let n = slugs.len();
        slugs.sort();
        slugs.dedup();
        assert_eq!(slugs.len(), n, "slugs stay unique");
        // A duplicate or malformed permalink is ignored (with a warning), never served twice.
        let mut f = PermalinksFile::default();
        f.pools.insert(d.pools[0].id.clone(), "Not A Slug!".into());
        f.pools.insert(d.pools[1].id.clone(), "same".into());
        f.pools.insert(d.pools[2].id.clone(), "same".into());
        let mut ps = d.pools.clone();
        assign_slugs(&mut ps, &f);
        assert_ne!(ps[0].slug, "Not A Slug!");
        assert_eq!(ps.iter().filter(|p| p.slug == "same").count(), 1);
    }

    #[test]
    fn safe_url_accepts_only_http_and_https() {
        for ok in ["https://z.cash/", "http://pool.example:8080/a?b=1&c=\"2\"", "HTTPS://Example.com"] {
            assert!(safe_url(ok).is_some(), "{ok}");
        }
        for bad in ["javascript:alert(1)", " JavaScript:alert(1)", "data:text/html,<b>", "vbscript:x", "//evil.example", "/relative", "ftp://x.example", "https://", "https://:80", "https://a b", "https://x/<script>", "https://x/\u{2028}y", "http://x\\y", "https://user:pass@example.com/x", "https://token@example.com/", ""] {
            assert!(safe_url(bad).is_none(), "{bad:?}");
        }
        let mut p = Pool { id: "p".into(), url: Some("javascript:alert(1)".into()), source_url: Some("https://ok.example/".into()), data_url: Some("data:x".into()), ..Default::default() };
        let mut dropped = vec![];
        clean_pool_urls(&mut p, &mut dropped);
        assert_eq!((p.url, p.source_url.as_deref(), p.data_url), (None, Some("https://ok.example/"), None));
        assert_eq!(dropped.len(), 2);
        let mut up = Some("explorer 1".to_string());
        clean_text_or_url(&mut up, "x", &mut dropped);
        assert_eq!(up.as_deref(), Some("explorer 1"), "plain words stay");
        let mut up = Some("javascript:alert(1)".to_string());
        clean_text_or_url(&mut up, "x", &mut dropped);
        assert_eq!(up, None);
    }

    #[test]
    fn shop_directory_loads_the_mining_shop_listing_without_inventing_prices() {
        let d = load(&real_dir()).unwrap();
        assert!(!d.vendors.is_empty(), "vendors.json should load");
        assert!(!d.listings.is_empty(), "listings.json should load");
        let v = d.vendors.iter().find(|v| v.id == "the-mining-shop-uk").expect("TMS vendor");
        assert_eq!(v.slug, "the-mining-shop-uk");
        assert!(v.regions.iter().any(|r| r == "UK"));
        assert!(!v.logo.is_fallback(), "vendor logo should resolve locally");
        assert!(v.listing_count >= 1);
        let l = d.listings.iter().find(|l| l.id == "tms-antminer-z15-pro-860").expect("Z15 Pro listing");
        assert_eq!(l.miner_id, "antminer-z15-pro");
        assert_eq!(l.vendor_id, "the-mining-shop-uk");
        assert_eq!(l.price_amount, Some(12800.0));
        assert_eq!(l.price_currency.as_deref(), Some("GBP"));
        assert_eq!(l.price_includes_vat, Some(false));
        assert_eq!(l.shop_hashrate_ksol, Some(860.0));
        assert_eq!(l.availability.as_deref(), Some("in_stock"));
        assert!(l.product_url.starts_with("https://www.theminingshop.co.uk/"));
        assert!(l.image_src.as_deref().unwrap_or("").starts_with("/static/shop/machines/"));
        // n/a ≠ 0: missing price stays None
        assert!(l.price_amount.unwrap() > 0.0);
        let groups = d.buy_machine_groups();
        assert_eq!(groups[0].0, "antminer-z15-pro", "Z15 Pro is the flagship group");
        assert!(!d.listings_for_miner("antminer-z15-pro").is_empty());
    }

    #[test]
    fn check_shop_image_rejects_path_escape_and_wrong_type() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
        assert!(check_shop_image(&root, "shop/machines/antminer-z15-pro-860.811ddcd13a.webp").is_ok());
        assert!(check_shop_image(&root, "../Cargo.toml").is_err());
        assert!(check_shop_image(&root, "logos/coins/zcash.47590b6def.svg").is_err());
        assert!(check_shop_image(&root, "shop/machines/nope.webp").is_err());
    }
}
