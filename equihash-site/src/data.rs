//! Typed views over data/*.json. Everything optional is `Option`, rendered as "n/a".

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
    // derived at load time
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
    pub difficulty: Option<f64>,
    pub height: Option<f64>,
    pub block_time_target_s: Option<f64>,
    pub block_time_avg_s: Option<f64>,
    pub pools_hashrate: Option<f64>,
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
}

impl Coin {
    pub fn params(&self) -> String {
        match &self.equihash {
            Some(Equihash { n: Some(n), k: Some(k) }) => format!("{n},{k}"),
            _ => "n/a".into(),
        }
    }
    pub fn active(&self) -> bool {
        self.status == "active"
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
    pub research: Vec<ResearchItem>,
    pub research_coins: Vec<ResearchCoin>,
    pub meta: Meta,
    /// Newest fetched_at across pools (what "last updated" shows).
    pub last_updated: Option<String>,
}

fn read<T: for<'de> Deserialize<'de> + Default>(dir: &Path, file: &str) -> Result<T, String> {
    let p = dir.join(file);
    let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))
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

/// Normalise free-form region strings into a few filterable buckets.
pub fn region_tags(p: &Pool) -> Vec<String> {
    let mut set: Vec<String> = Vec::new();
    let mut push = |s: &str| {
        if !set.iter().any(|x| x == s) {
            set.push(s.to_string())
        }
    };
    let text = format!("{} {}", p.region.clone().unwrap_or_default(), p.regions.join(" ")).to_lowercase();
    if text.contains("global") {
        push("Global");
    }
    if text.contains("united states") || text.contains(" us") || text.starts_with("us") || text.contains("north america") || text.contains("canada") {
        push("North America");
    }
    if text.contains("eu") || text.contains("europe") || text.contains("france") || text.contains("germany") {
        push("Europe");
    }
    if text.contains("asia") || text.contains("hong kong") || text.contains("singapore") || text.contains(" in") || text.contains("china") {
        push("Asia");
    }
    if text.contains("russia") {
        push("Russia");
    }
    if text.contains("south america") {
        push("South America");
    }
    if text.contains("middle east") {
        push("Middle East");
    }
    set
}

pub fn load(dir: &Path) -> Result<Data, String> {
    let pf: PoolsFile = read(dir, "pools.json")?;
    let nf: NetworkFile = read(dir, "network.json")?;
    let af: ArchiveFile = read(dir, "archive.json")?;
    let mf: MinersFile = read(dir, "miners.json")?;
    let rf: ResearchFile = read(dir, "research.json").unwrap_or_default();
    let meta: Meta = read(dir, "meta.json").unwrap_or_default();

    let mut coins = nf.coins;
    // Hand-added coins (data/curated/coins.json) appear even before the next refresh.
    if let Ok(raw) = std::fs::read_to_string(dir.join("curated").join("coins.json")) {
        match serde_json::from_str::<NetworkFile>(&raw) {
            Ok(extra) => {
                for mut c in extra.coins {
                    if c.id.is_empty() || coins.iter().any(|x| x.id == c.id) {
                        continue;
                    }
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
                    if let Some(src) = st.get("sources").cloned().and_then(|x| serde_json::from_value::<Vec<Link>>(x).ok()) {
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
    // Hand-curated rows (data/curated/manual-pools.json) appear even before the next refresh.
    if let Ok(raw) = std::fs::read_to_string(dir.join("curated").join("manual-pools.json")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            for row in v.get("pools").and_then(|p| p.as_array()).cloned().unwrap_or_default() {
                let id = row.get("id").and_then(|x| x.as_str()).unwrap_or_default().to_string();
                if id.is_empty() || pools.iter().any(|p| p.id == id) {
                    continue;
                }
                if let Ok(mut p) = serde_json::from_value::<Pool>(row.clone()) {
                    if p.coin_id.is_empty() {
                        p.coin_id = coins.iter().find(|c| c.symbol == p.coin).map(|c| c.id.clone()).unwrap_or_default();
                    }
                    if p.fetched_at.is_none() {
                        p.fetched_at = row.get("verified_at").and_then(|x| x.as_str()).map(String::from);
                    }
                    pools.push(p);
                }
            }
        }
    }
    let mut seen = HashSet::new();
    for p in pools.iter_mut() {
        let host = p
            .url
            .as_deref()
            .unwrap_or("")
            .trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_start_matches("www.")
            .to_string();
        let base = slugify(&format!("{}-{}-{}", p.coin_id, p.name, host));
        let mut slug = base.clone();
        let mut i = 2;
        while !seen.insert(slug.clone()) {
            slug = format!("{base}-{i}");
            i += 1;
        }
        p.slug = slug;
        p.region_tags = region_tags(p);
        p.coin_label = coins.iter().find(|c| c.id == p.coin_id).map(|c| c.label.clone()).unwrap_or_else(|| p.coin.clone());
    }
    for c in coins.iter_mut() {
        c.pool_count = pools.iter().filter(|p| p.coin_id == c.id).count() as u32;
    }
    // Neutral ordering: active coins first, then by number of tracked pools, then name.
    coins.sort_by(|a, b| {
        b.active()
            .cmp(&a.active())
            .then(b.pool_count.cmp(&a.pool_count))
            .then(a.name.cmp(&b.name))
    });
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
            p.network_share_pct = match (p.hashrate, denom) {
                (Some(h), Some(d)) => Some(h / d * 100.0),
                _ => None,
            };
            p.share_basis = basis.into();
            p.share_flag = p.network_share_pct.unwrap_or(0.0) > 30.0 && (basis == "network" || reporting > 1);
        }
    }
    let last_updated = pools
        .iter()
        .filter(|p| p.from_miningpoolstats)
        .filter_map(|p| p.fetched_at.clone())
        .max()
        .or(pf.generated_at.clone());

    Ok(Data {
        pools,
        coins,
        archive: af.pools,
        miners: mf.miners,
        miners_verified_at: mf.verified_at,
        miners_not_listed: mf.not_listed,
        research: rf.items,
        research_coins: rf.coins,
        meta,
        last_updated,
    })
}

impl Data {
    pub fn coin(&self, id: &str) -> Option<&Coin> {
        self.coins.iter().find(|c| c.id == id)
    }
    pub fn active_coin_ids(&self) -> Vec<String> {
        self.coins.iter().filter(|c| c.active()).map(|c| c.id.clone()).collect()
    }
    /// Pools on coins whose PoW is still running.
    pub fn live_pools(&self) -> impl Iterator<Item = &Pool> {
        let active: HashSet<String> = self.active_coin_ids().into_iter().collect();
        self.pools.iter().filter(move |p| active.contains(&p.coin_id))
    }
    /// Pools that MPS still lists for coins whose PoW ended.
    pub fn ended_coin_pools(&self) -> Vec<&Pool> {
        let active: HashSet<String> = self.active_coin_ids().into_iter().collect();
        self.pools.iter().filter(|p| !active.contains(&p.coin_id)).collect()
    }
}
