pub mod calc;
pub mod guide;
pub mod home;
pub mod layout;
pub mod links;
pub mod logo;
pub mod pages;

use crate::data::{Coin, Data, Pool};
use crate::fmt;

/// Make serialized JSON safe inside `<script type="application/json">`: `<`, `>` and `&` become
/// \u003c, \u003e and \u0026, and U+2028 / U+2029 become \u2028 / \u2029. Those characters only
/// occur inside JSON strings, so the escapes are valid JSON and JSON.parse returns the original
/// values; no upstream string can close the element, open a comment or start markup.
pub fn escape_script_json(json: &str) -> String {
    let mut out = String::with_capacity(json.len() + 16);
    for ch in json.chars() {
        match ch {
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out
}

/// Serialize for a `<script type="application/json">` block (see `escape_script_json`).
pub fn script_json<T: serde::Serialize + ?Sized>(v: &T) -> String {
    escape_script_json(&serde_json::to_string(v).unwrap_or_else(|_| "null".into()))
}

/// The share fields /api/live carries for one pool. `share_pct` is a share *of the network* or
/// null: whenever the figure would be measured against anything else (the listed pools' total,
/// or a single pool against itself), it is null and `share_status` / `share_basis` say why.
pub fn share_json(c: Option<&Coin>, p: &Pool) -> serde_json::Value {
    let status = p.share_status.as_str();
    let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
    let net_denom = |c: &Coin| {
        serde_json::json!({
            "hashrate": c.network.hashrate,
            "unit": c.network.unit.clone().unwrap_or(unit.clone()),
            "basis": "network",
            "source": c.network.hashrate_source,
            "observed_at": c.network.hashrate_observed_at,
            "description": match c.network.hashrate_sample_blocks { Some(b) => format!("network estimate from the last {b} blocks"), None => "network estimate".to_string() },
        })
    };
    let listed_denom = |c: &Coin| {
        serde_json::json!({
            "hashrate": c.share_denominator,
            "unit": unit,
            "basis": "listed_pools",
            "source": "sum of the hashrates the listed pools report",
            "observed_at": c.reported.observed_at,
            "pools": c.reported.positive_pools,
            "description": "listed pools' reported total",
        })
    };
    let (basis, denom) = match (status, c) {
        ("unavailable", _) | (_, None) => (serde_json::Value::Null, serde_json::Value::Null),
        // One pool, alone above the network estimate (capped): still the network's figure.
        ("pools_exceed_network", Some(c)) if c.share_basis == "network" => ("network".into(), net_denom(c)),
        ("network", Some(c)) => ("network".into(), net_denom(c)),
        (_, Some(c)) => ("listed_pools".into(), listed_denom(c)),
    };
    serde_json::json!({
        "share_pct": if status == "network" { p.network_share_pct } else { None },
        "share_status": status,
        "share_basis": basis,
        // Share of the listed pools' total, when that is what the page shows (several pools).
        "listed_share_pct": if c.map(|c| c.share_basis == "pools").unwrap_or(false) && status != "only_listed_pool" { p.network_share_pct } else { None },
        "share_denominator": denom,
    })
}

/// Body of /api/live: each poller's status, plus the figures it feeds, already formatted the way
/// the server renders them, so the page can swap text in place without its own formatting rules.
pub fn live_json(d: &Data, now: chrono::DateTime<chrono::Utc>) -> serde_json::Value {
    let sources: Vec<serde_json::Value> = d
        .live
        .iter()
        .map(|s| {
            let mut value = serde_json::to_value(s).unwrap_or_else(|_| serde_json::json!({"id": s.id, "status": "unavailable"}));
            if s.error.is_some() {
                value["error"] = "source unavailable".into();
            }
            value
        })
        .collect();
    // Coins with a live figure. Every pool on them is listed: a new network reading moves all of
    // their shares, and can switch what the shares are measured against.
    let live_coins: Vec<&Coin> = d.coins.iter().filter(|c| c.network.live_source.is_some() || d.pools.iter().any(|p| p.coin_id == c.id && p.live_source.is_some())).collect();
    let mut pools = serde_json::Map::new();
    for p in d.pools.iter().filter(|p| p.live_source.is_some() || live_coins.iter().any(|c| c.id == p.coin_id)) {
        let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
        let ts = p.hashrate_observed_at.as_deref();
        let coin = d.coin(&p.coin_id);
        let mut v = serde_json::json!({
            "coin_id": p.coin_id,
            "live": p.live_source.is_some(),
            "hashrate": p.hashrate,
            "hashrate_text": fmt::hashrate(p.hashrate, &unit),
            "basis": p.basis,
            "source": p.hashrate_source,
            "observed_at": ts,
            "age_text": fmt::age(crate::data::age_secs(ts, now)).map(|a| format!("{a} ago")),
            "window_seconds": p.hashrate_window_s,
            "share_text": home::share_text(p),
            "share_note": p.share_note,
            "share_flag": p.share_flag,
            // The row's value for sorting by share (what the Share cell's bar is drawn from).
            "share_sort": p.network_share_pct,
            // What the mobile row shows under the hashrate, and the cell's tooltip.
            "share_short": home::m_share(p),
            "title": home::hashrate_title(p),
            // Server-rendered markup for the share cell and the pool page's share row, so a page
            // left open switches between a percentage and "only listed pool" without a reload.
            "share_cell_html": home::share_cell(p).into_string(),
            "share_kv_label": pages::share_kv_label(p),
            "share_kv_html": pages::share_kv_value(p).into_string(),
        });
        if let (Some(o), serde_json::Value::Object(s)) = (v.as_object_mut(), share_json(coin, p)) {
            o.extend(s);
        }
        pools.insert(p.id.clone(), v);
    }
    let mut coins = serde_json::Map::new();
    for c in live_coins.iter().copied() {
        let unit = c.network.unit.clone().unwrap_or("Sol/s".into());
        let ts = c.network.hashrate_observed_at.as_deref();
        let (rep, dag) = fmt::reported(c);
        let (basis, denom) = match c.share_basis.as_str() {
            "network" => ("network", serde_json::json!({"hashrate": c.share_denominator, "unit": unit, "basis": "network", "source": c.network.hashrate_source, "observed_at": ts})),
            "pools" => ("listed_pools", serde_json::json!({"hashrate": c.share_denominator, "unit": unit, "basis": "listed_pools", "source": "sum of the hashrates the listed pools report", "observed_at": c.reported.observed_at, "pools": c.reported.positive_pools})),
            _ => ("", serde_json::Value::Null),
        };
        coins.insert(
            c.id.clone(),
            serde_json::json!({
                "share_basis": if basis.is_empty() { serde_json::Value::Null } else { basis.into() },
                "share_denominator": denom,
                "share_th_title": home::share_th_title(Some(c)),
                "split_html": home::split_bar(d, c).into_string(),
                "concentration_html": home::concentration_view(d, Some(c)).into_string(),
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
        "sources": sources,
        "pools": pools,
        "coins": coins,
        "concentration_all_html": home::concentration_view(d, None).into_string(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// A flat copy of data/ (current snapshot's generated files plus the hand-kept files) that a
    /// test can edit before loading.
    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Tmp {
            let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
            let dir = std::env::temp_dir().join(format!("equihash-views-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("curated")).unwrap();
            for sub in ["", "curated"] {
                for e in std::fs::read_dir(real.join(sub)).unwrap().flatten() {
                    if e.path().is_file() && e.file_name() != crate::data::MANIFEST {
                        std::fs::copy(e.path(), dir.join(sub).join(e.file_name())).unwrap();
                    }
                }
            }
            for f in crate::data::GENERATED {
                std::fs::copy(crate::data::generated_path(&real, f), dir.join(f)).unwrap();
            }
            Tmp(dir)
        }
        fn edit(&self, file: &str, f: impl FnOnce(&mut serde_json::Value)) {
            let p = self.0.join(file);
            let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&p).unwrap()).unwrap();
            f(&mut v);
            std::fs::write(&p, serde_json::to_string(&v).unwrap()).unwrap();
        }
        fn set_network(&self, coin: &str, h: Option<f64>) {
            self.edit("network.json", |v| {
                for c in v["coins"].as_array_mut().unwrap() {
                    if c["id"] == coin {
                        c["network"]["hashrate"] = serde_json::json!(h);
                    }
                }
            });
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn script_block<'a>(html: &'a str, id: &str) -> &'a str {
        let open = format!("<script type=\"application/json\" id=\"{id}\">");
        let start = html.find(&open).unwrap_or_else(|| panic!("no #{id}")) + open.len();
        &html[start..start + html[start..].find("</script>").unwrap()]
    }

    #[test]
    fn script_json_escapes_everything_that_could_end_the_script_element() {
        let v = serde_json::json!({"name": "</script><script>alert(1)</script><!-- & \u{2028}\u{2029}"});
        let s = script_json(&v);
        for bad in ["<", ">", "&", "\u{2028}", "\u{2029}"] {
            assert!(!s.contains(bad), "{bad:?} in {s}");
        }
        assert_eq!(serde_json::from_str::<serde_json::Value>(&s).unwrap(), v, "JSON.parse gives back the same value");
    }

    /// The network estimate is always the upstream network figure with its own source, never the
    /// listed pools' total or a capped value, whichever side of the pools' total it falls on.
    #[test]
    fn network_estimate_is_the_upstream_figure_not_the_pool_total() {
        let t = Tmp::new("netest");
        const SRC: &str = "https://data.miningpoolstats.stream/data/zcash.js?t=1";
        let pools: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(t.0.join("pools.json")).unwrap()).unwrap();
        let sum: f64 = pools["pools"].as_array().unwrap().iter().filter(|r| r["coin_id"] == "zcash").filter_map(|r| r["hashrate"].as_f64()).filter(|h| *h > 0.0).sum();
        // Below the pools' total (shares fall back to the pools), and just above it (shares capped).
        for net in [sum * 0.897, sum * 1.013] {
            t.edit("network.json", |v| {
                let c = v["coins"].as_array_mut().unwrap().iter_mut().find(|c| c["id"] == "zcash").unwrap();
                c["network"]["hashrate"] = net.into();
                c["network"]["hashrate_source"] = SRC.into();
                c["network"]["hashrate_upstream"] = "https://zec.2miners.com".into();
            });
            let d = crate::data::load(&t.0).unwrap();
            let c = d.coin("zcash").unwrap();
            assert_eq!(c.network.hashrate, Some(net), "loaded as published");
            assert_eq!(c.network.hashrate_source.as_deref(), Some(SRC));
            assert!((c.reported.hashrate.unwrap() - sum).abs() < 1.0);
            let pair = home::hashrate_pair(c).into_string();
            let net_text = crate::fmt::hashrate(Some(net), "Sol/s");
            let sum_text = crate::fmt::hashrate(Some(sum), "Sol/s");
            assert_ne!(net_text, sum_text, "fixture keeps the two apart");
            assert!(pair.contains(&format!("Network estimate: <b data-f=\"network\">{net_text}</b>")), "{pair}");
            assert!(pair.contains(&format!("<b data-f=\"reported\">{sum_text}")), "{pair}");
            assert!(pair.contains("data.miningpoolstats.stream") && pair.contains("zec.2miners.com"), "the estimate keeps its own source: {pair}");
            let p = d.pools.iter().find(|p| p.coin_id == "zcash" && p.hashrate.unwrap_or(0.0) > 0.0).unwrap();
            let j = share_json(Some(c), p);
            if net < sum {
                assert_eq!(c.share_basis, "pools");
                assert_eq!(j["share_denominator"]["basis"], "listed_pools");
                assert!((j["share_denominator"]["hashrate"].as_f64().unwrap() - sum).abs() < 1.0);
            } else {
                assert_eq!(c.share_basis, "network");
                assert_eq!(j["share_denominator"]["hashrate"].as_f64(), Some(net), "the network figure, not a capped one");
                assert_eq!(j["share_denominator"]["source"], SRC);
            }
            let page = home::render_at(&d, &home::Filters { coin: Some("zcash".into()), ..Default::default() }, chrono::Utc::now()).into_string();
            assert!(page.contains(&net_text) && page.contains(&sum_text));
        }
    }

    const EVIL: &str = "</script><script>alert(1)</script><img src=x onerror=alert(2)><!--\u{2028}";

    #[test]
    fn malicious_upstream_fixtures_never_become_markup_scripts_or_links() {
        let t = Tmp::new("evil");
        let mut target = String::new();
        t.edit("pools.json", |v| {
            let row = v["pools"].as_array_mut().unwrap().iter_mut().find(|r| r["coin_id"] == "zcash" && r["from_miningpoolstats"] == true).unwrap();
            target = row["id"].as_str().unwrap().to_string();
            row["name"] = EVIL.into();
            row["notes"] = EVIL.into();
            row["region"] = EVIL.into();
            row["url"] = "javascript:alert(3)".into();
            row["source_url"] = "data:text/html,<script>alert(4)</script>".into();
            row["data_url"] = " JaVaScRiPt:alert(5)".into();
            row["hashrate_source"] = "vbscript:msgbox(6)".into();
            row["merged_mining"] = serde_json::json!({"supported": true, "coins": [EVIL], "note": EVIL});
        });
        t.edit("network.json", |v| {
            let c = v["coins"].as_array_mut().unwrap().iter_mut().find(|c| c["id"] == "zcash").unwrap();
            c["name"] = format!("Zcash{EVIL}").into();
            c["source_url"] = "javascript:alert(7)".into();
            c["network"]["hashrate_source"] = "data:,x".into();
            c["network"]["hashrate_upstream"] = "javascript:alert(8)".into();
            c["status_sources"] = serde_json::json!([{"label": EVIL, "url": "javascript:alert(9)"}]);
        });
        t.edit("archive.json", |v| {
            if let Some(a) = v["pools"].as_array_mut().and_then(|a| a.first_mut()) {
                a["url"] = "javascript:alert(10)".into();
                a["sources"] = serde_json::json!([{"label": "x", "url": "javascript:alert(11)"}]);
            }
        });
        let d = crate::data::load(&t.0).unwrap();
        let p = d.pools.iter().find(|p| p.id == target).unwrap();
        assert_eq!((p.url.as_deref(), p.source_url.as_deref(), p.data_url.as_deref(), p.hashrate_source.as_deref()), (None, None, None, None), "unsafe URLs dropped at load");
        let z = d.coin("zcash").unwrap();
        assert!(z.source_url.is_none() && z.network.hashrate_source.is_none() && z.network.hashrate_upstream.is_none() && z.status_sources.is_empty());
        let f = |coin: &str| home::Filters { coin: Some(coin.into()), ..Default::default() };
        let pages = [
            ("home zcash", home::render(&d, &f("zcash")).into_string()),
            ("home all", home::render(&d, &f("all")).into_string()),
            ("pool page", pages::pool_page(&d, p).into_string()),
            ("calculator", calc::render(&d, &calc::CalcQuery { coin: Some("zcash".into()), ..Default::default() }).into_string()),
            ("archive", pages::archive(&d).into_string()),
            ("sources", pages::sources(&d).into_string()),
        ];
        for (name, html) in &pages {
            let lower = html.to_lowercase();
            for bad in ["<script>alert", "<img src=x", "<!--\u{2028}", "href=\"javascript:", "href=\" javascript:", "href=\"data:", "href=\"vbscript:", "alert(3)", "alert(4)", "alert(7)", "alert(9)", "alert(10)", "alert(11)"] {
                assert!(!lower.contains(bad), "{name}: {bad:?} reached the page");
            }
            // Every <script> element the page has is one we wrote.
            assert_eq!(lower.matches("<script").count(), lower.matches("</script>").count(), "{name}: script tags balanced");
        }
        let (_, home_html) = &pages[0];
        assert!(home_html.contains("&lt;/script&gt;&lt;script&gt;alert(1)"), "the name is shown, escaped");
        let block = script_block(home_html, "pool-data");
        assert!(!block.contains('\u{2028}') && !block.contains('<'), "pool-data is escaped for the script context");
        let rows: serde_json::Value = serde_json::from_str(block).unwrap();
        assert_eq!(rows.as_array().unwrap().iter().find(|r| r["id"] == target.as_str()).unwrap()["name"], EVIL, "and parses back to the original");
        let calc_block = script_block(&pages[3].1, "calc-data");
        assert!(!calc_block.contains('<') && !calc_block.contains('\u{2028}'));
        // /api/live is served as JSON (nosniff), so raw names are fine there; but the markup
        // fragments the page inserts are server-rendered and escaped like the page itself.
        let j = live_json(&d, chrono::Utc::now());
        let mut frags = vec![j["concentration_all_html"].as_str().unwrap().to_string()];
        for group in ["pools", "coins"] {
            for v in j[group].as_object().unwrap().values() {
                for (k, x) in v.as_object().unwrap() {
                    if k.ends_with("_html") {
                        frags.push(x.as_str().unwrap_or_default().to_string());
                    }
                }
            }
        }
        assert!(frags.len() > 20);
        for h in &frags {
            assert!(!h.contains("<script") && !h.contains("<img") && !h.contains("javascript:"), "{h}");
        }
        assert!(!j.to_string().contains("javascript:alert"), "unsafe URLs never reach the API either");
    }

    #[test]
    fn api_live_share_is_null_unless_it_is_a_share_of_the_network() {
        // Wcash: one listed pool. Network estimate well above it: a share of the network.
        let t = Tmp::new("share");
        t.set_network("wcash", Some(1.0e9));
        let d = crate::data::load(&t.0).unwrap();
        let j = live_json(&d, chrono::Utc::now());
        let p = &j["pools"]["wcash:zecwec.com"];
        assert_eq!(p["share_status"], "network");
        assert_eq!(p["share_basis"], "network");
        assert!(p["share_pct"].as_f64().unwrap() > 0.0 && p["share_pct"].as_f64().unwrap() < 1.0);
        assert_eq!(p["share_denominator"]["hashrate"], 1.0e9);
        assert_eq!(p["share_denominator"]["basis"], "network");
        assert!(p["share_denominator"]["source"].as_str().unwrap().starts_with("https://"));
        assert_eq!(j["coins"]["wcash"]["share_basis"], "network");
        // The fragments are what the page renders.
        let page = home::render(&d, &home::Filters { coin: Some("wcash".into()), ..Default::default() }).into_string();
        assert!(page.contains(p["share_cell_html"].as_str().unwrap()));
        assert!(page.contains(j["coins"]["wcash"]["split_html"].as_str().unwrap()));
        assert!(!p["share_cell_html"].as_str().unwrap().contains("only listed pool"));

        // Network estimate below the pool: only listed pool, share_pct null (never a 100%).
        t.set_network("wcash", Some(1.0));
        let d = crate::data::load(&t.0).unwrap();
        let j = live_json(&d, chrono::Utc::now());
        let p = &j["pools"]["wcash:zecwec.com"];
        assert_eq!(p["share_status"], "only_listed_pool");
        assert_eq!(p["share_basis"], "listed_pools");
        assert!(p["share_pct"].is_null() && p["listed_share_pct"].is_null());
        assert_eq!(p["share_denominator"]["basis"], "listed_pools");
        assert_eq!(p["share_denominator"]["pools"], 1);
        assert!(p["share_cell_html"].as_str().unwrap().contains("only listed pool"));
        assert!(j["coins"]["wcash"]["split_html"].as_str().unwrap().contains("split-note solo"));
        assert_eq!(p["share_kv_label"], "Share of pool-reported hashrate");
        let page = home::render(&d, &home::Filters { coin: Some("wcash".into()), ..Default::default() }).into_string();
        assert!(page.contains(p["share_cell_html"].as_str().unwrap()) && page.contains(j["coins"]["wcash"]["split_html"].as_str().unwrap()));

        // Zcash (fed by a live pool row, so every Zcash pool is listed): pools above the network.
        t.set_network("zcash", Some(1.0));
        let d = crate::data::load(&t.0).unwrap();
        let j = live_json(&d, chrono::Utc::now());
        let zpools: Vec<&serde_json::Value> = j["pools"].as_object().unwrap().values().filter(|p| p["coin_id"] == "zcash").collect();
        assert!(zpools.len() > 10, "all Zcash pools are listed, not just the live-fed one");
        let reporting: Vec<_> = zpools.iter().filter(|p| p["hashrate"].as_f64().unwrap_or(0.0) > 0.0).collect();
        assert!(reporting.iter().all(|p| p["share_status"] == "pools_exceed_network" && p["share_pct"].is_null() && p["share_basis"] == "listed_pools"));
        let total: f64 = reporting.iter().map(|p| p["listed_share_pct"].as_f64().unwrap()).sum();
        assert!((total - 100.0).abs() < 1e-6, "listed shares add up to 100% of the listed total: {total}");
        assert!(zpools.iter().filter(|p| p["hashrate"].is_null()).all(|p| p["share_status"] == "unavailable" && p["share_denominator"].is_null()));
        // No network estimate at all.
        t.set_network("zcash", None);
        let d = crate::data::load(&t.0).unwrap();
        let j = live_json(&d, chrono::Utc::now());
        assert!(j["pools"].as_object().unwrap().values().filter(|p| p["coin_id"] == "zcash" && p["hashrate"].as_f64().unwrap_or(0.0) > 0.0).all(|p| p["share_status"] == "no_network_estimate" && p["share_pct"].is_null()));
        // Nowhere in /api/live is a 100% share without a stated basis.
        for p in j["pools"].as_object().unwrap().values() {
            if p["share_pct"].as_f64() == Some(100.0) {
                assert_eq!(p["share_basis"], "network");
            }
        }
    }

    #[test]
    fn api_live_redacts_poll_error_details() {
        let t = Tmp::new("live-errors");
        let mut d = crate::data::load(&t.0).unwrap();
        assert!(!d.live.is_empty(), "fixture has live sources");
        d.live[0].error = Some("PRIVATE_DETAIL_SENTINEL".into());
        let j = live_json(&d, chrono::Utc::now());
        assert_eq!(j["sources"][0]["error"], "source unavailable");
        let text = j.to_string();
        assert!(!text.contains("PRIVATE_DETAIL_SENTINEL"));
    }
}
