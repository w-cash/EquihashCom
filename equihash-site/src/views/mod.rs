pub mod calc;
pub mod guide;
pub mod home;
pub mod layout;
pub mod links;
pub mod pages;

use crate::data::Data;
use crate::fmt;

/// Body of /api/live: each poller's status, plus the figures it feeds, already formatted the way
/// the server renders them, so the page can swap text in place without its own formatting rules.
pub fn live_json(d: &Data, now: chrono::DateTime<chrono::Utc>) -> serde_json::Value {
    let mut pools = serde_json::Map::new();
    for p in d.pools.iter().filter(|p| p.live_source.is_some()) {
        let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
        let ts = p.hashrate_observed_at.as_deref();
        pools.insert(
            p.id.clone(),
            serde_json::json!({
                "hashrate": p.hashrate,
                "hashrate_text": fmt::hashrate(p.hashrate, &unit),
                "basis": p.basis,
                "source": p.hashrate_source,
                "observed_at": ts,
                "age_text": fmt::age(crate::data::age_secs(ts, now)).map(|a| format!("{a} ago")),
                "window_seconds": p.hashrate_window_s,
                "share_pct": p.network_share_pct,
                "share_text": home::share_text(p),
                "share_note": p.share_note,
                // What the mobile row shows under the hashrate, and the cell's tooltip.
                "share_short": if p.share_note.is_some() { home::share_text(p) } else { p.network_share_pct.map(|s| fmt::pct(Some(s))).unwrap_or_default() },
                "title": home::hashrate_title(p),
            }),
        );
    }
    let mut coins = serde_json::Map::new();
    for c in d.coins.iter().filter(|c| c.network.live_source.is_some() || d.pools.iter().any(|p| p.coin_id == c.id && p.live_source.is_some())) {
        let unit = c.network.unit.clone().unwrap_or("Sol/s".into());
        let ts = c.network.hashrate_observed_at.as_deref();
        let (rep, dag) = fmt::reported(c);
        coins.insert(
            c.id.clone(),
            serde_json::json!({
                "network_hashrate": c.network.hashrate,
                "network_text": if c.network.hashrate.is_some() { fmt::hashrate(c.network.hashrate, &unit) } else { "unavailable".into() },
                "network_note": home::network_note(c),
                "network_observed_at": ts,
                "network_age_text": fmt::age(crate::data::age_secs(ts, now)).map(|a| format!("{a} ago")),
                "sample_blocks": c.network.hashrate_sample_blocks,
                "height": c.network.height,
                "reported_text": rep,
                "reported_dagger": dag,
            }),
        );
    }
    serde_json::json!({
        "now": now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "sources": d.live,
        "pools": pools,
        "coins": coins,
        "ranking": ranking_json(d),
    })
}

/// Every coin in the server's rank order (`data::rank_cmp`: by parameter set, then by what the
/// listed pools report), with the figures that move with it. The page reorders its sidebar,
/// selectors, Z15 tables and networks table to this order without a reload.
pub fn ranking_json(d: &Data) -> serde_json::Value {
    let pro = d.miners.iter().find(|m| m.model.ends_with("Z15 Pro")).and_then(|m| m.hashrate_ksol);
    serde_json::Value::Array(
        d.coins
            .iter()
            .map(|c| {
                let (rep, dag) = fmt::reported(c);
                let unit = c.network.unit.clone().unwrap_or("Sol/s".into());
                let per = pro.and_then(|h| calc::coins_per_day(c, h, 1.0));
                serde_json::json!({
                    "id": c.id,
                    "name": c.name,
                    "symbol": c.symbol,
                    "group": c.group_label(),
                    "active": c.active(),
                    "reported_hashrate": c.reported.hashrate,
                    "reported_text": rep,
                    "reported_dagger": dag,
                    "network_hashrate": c.network.hashrate,
                    "network_text": if c.network.hashrate.is_some() { fmt::hashrate(c.network.hashrate, &unit) } else { "unavailable".into() },
                    "option_text": home::coin_option_text(c),
                    "title": home::coin_title(c),
                    "z15_day": per.map(|v| format!("{v:.4}")),
                    "z15_usd": match (per, c.price_usd) { (Some(v), Some(p)) => format!("${:.2}", v * p), _ => "n/a".into() },
                    "z15_na_reason": calc::per_day_na_reason(c, pro),
                })
            })
            .collect(),
    )
}
