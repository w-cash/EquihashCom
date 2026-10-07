use crate::data::{Coin, Data};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Raw query values. Kept as strings so a bad value ("abc", "1e999", "-5") gets a clear message
/// on the form instead of a bare 400 from the query extractor.
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct CalcQuery {
    pub coin: Option<String>,
    pub hashrate: Option<String>, // kSol/s
    pub watts: Option<String>,
    pub power: Option<String>, // $/kWh
    pub fee: Option<String>,   // %
    pub price: Option<String>,
    pub reward: Option<String>,
    pub nethash: Option<String>, // Sol/s
    pub blocktime: Option<String>,
}

/// Bounds for one calculator input. The same table drives server-side checks and the
/// `min`/`max`/`data-*` attributes that static/app.js uses for the live check.
pub struct Field {
    pub name: &'static str,
    pub label: &'static str,
    pub unit: &'static str,
    pub min: f64,
    /// true: the value must be strictly greater than `min` (zero is not allowed).
    pub min_exclusive: bool,
    pub max: f64,
}

pub const FIELDS: &[Field] = &[
    Field { name: "hashrate", label: "Hashrate", unit: "kSol/s", min: 0.0, min_exclusive: true, max: 1e9 },
    Field { name: "watts", label: "Power draw", unit: "W", min: 0.0, min_exclusive: false, max: 1e7 },
    Field { name: "power", label: "Electricity", unit: "$/kWh", min: 0.0, min_exclusive: false, max: 10.0 },
    Field { name: "fee", label: "Pool fee", unit: "%", min: 0.0, min_exclusive: false, max: 100.0 },
    Field { name: "price", label: "Coin price", unit: "USD", min: 0.0, min_exclusive: false, max: 1e9 },
    Field { name: "reward", label: "Block reward to miners", unit: "coins", min: 0.0, min_exclusive: true, max: 1e7 },
    Field { name: "nethash", label: "Network hashrate", unit: "Sol/s", min: 0.0, min_exclusive: true, max: 1e18 },
    Field { name: "blocktime", label: "Block time", unit: "s", min: 1.0, min_exclusive: false, max: 86400.0 },
];

pub fn field(name: &str) -> &'static Field {
    FIELDS.iter().find(|f| f.name == name).expect("known calculator field")
}

fn bound(v: f64) -> String {
    if v >= 1e6 {
        format!("{v:e}").replace("e", "×10^")
    } else {
        fmt::num_short(v)
    }
}

/// Parse and range-check one input. Empty or missing means "not given" (Ok(None)).
pub fn check(name: &str, raw: Option<&str>) -> Result<Option<f64>, String> {
    let f = field(name);
    let Some(s) = raw.map(str::trim).filter(|s| !s.is_empty()) else { return Ok(None) };
    let v: f64 = s.parse().map_err(|_| "Enter a number.".to_string())?;
    if !v.is_finite() {
        return Err("Enter a finite number.".into());
    }
    if name == "fee" && !(0.0..=100.0).contains(&v) {
        return Err("Pool fee must be between 0% and 100%.".into());
    }
    if v < 0.0 && f.min >= 0.0 {
        return Err("Can't be negative.".into());
    }
    if f.min_exclusive && v <= f.min {
        return Err(format!("Must be more than {}.", bound(f.min)));
    }
    if v < f.min {
        return Err(format!("Must be at least {} {}.", bound(f.min), f.unit));
    }
    if v > f.max {
        return Err(format!("Must be {} {} or less.", bound(f.max), f.unit));
    }
    Ok(Some(v))
}

pub struct Checked {
    pub values: BTreeMap<&'static str, Option<f64>>,
    pub errors: BTreeMap<&'static str, String>,
}

impl Checked {
    pub fn get(&self, k: &str) -> Option<f64> {
        self.values.get(k).copied().flatten()
    }
}

pub fn validate(q: &CalcQuery) -> Checked {
    let raw: [(&'static str, &Option<String>); 8] = [
        ("hashrate", &q.hashrate),
        ("watts", &q.watts),
        ("power", &q.power),
        ("fee", &q.fee),
        ("price", &q.price),
        ("reward", &q.reward),
        ("nethash", &q.nethash),
        ("blocktime", &q.blocktime),
    ];
    let mut values = BTreeMap::new();
    let mut errors = BTreeMap::new();
    for (k, v) in raw {
        match check(k, v.as_deref()) {
            Ok(x) => {
                values.insert(k, x);
            }
            Err(e) => {
                values.insert(k, None);
                errors.insert(k, e);
            }
        }
    }
    Checked { values, errors }
}

pub struct Estimate {
    pub coins_day: f64,
    pub revenue_day: Option<f64>,
    pub power_day: f64,
    pub profit_day: Option<f64>,
}

fn in_range(name: &str, v: f64) -> bool {
    check(name, Some(&v.to_string())).map(|x| x.is_some()).unwrap_or(false)
}

/// Daily estimate. Returns None for any out-of-range input (same bounds as the form).
#[allow(clippy::too_many_arguments)]
pub fn estimate(hr_ksol: f64, watts: f64, power: f64, fee: f64, price: Option<f64>, reward: f64, net: f64, bt: f64) -> Option<Estimate> {
    let ok = in_range("hashrate", hr_ksol)
        && in_range("watts", watts)
        && in_range("power", power)
        && in_range("fee", fee)
        && price.map(|p| in_range("price", p)).unwrap_or(true)
        && in_range("reward", reward)
        && in_range("nethash", net)
        && in_range("blocktime", bt);
    if !ok {
        return None;
    }
    let share = ((hr_ksol * 1000.0) / net).min(1.0);
    let coins_day = share * (86400.0 / bt) * reward * (1.0 - fee / 100.0);
    let power_day = watts / 1000.0 * 24.0 * power;
    let revenue_day = price.map(|p| coins_day * p);
    Some(Estimate { coins_day, revenue_day, power_day, profit_day: revenue_day.map(|r| r - power_day) })
}

/// A per-machine estimate (machine ÷ network) only holds while the machine is a small part of the
/// network. At this share or more, adding it would itself move the estimate, so no figure is given.
/// The hardware page, the coin pages and the calculator (server and static/shared.js) all use it.
pub const OUTWEIGH_SHARE: f64 = 0.1;

pub fn outweighs_network(c: &Coin, hr_ksol: f64) -> bool {
    c.network.hashrate.map(|n| outweighs(hr_ksol, n)).unwrap_or(false)
}

fn outweighs(hr_ksol: f64, net: f64) -> bool {
    net > 0.0 && hr_ksol > 0.0 && hr_ksol * 1000.0 >= OUTWEIGH_SHARE * net
}

/// The calculator's version of the same rule, on whatever hashrate and network figure were
/// entered: the reason coins per day is n/a, or None when an estimate is meaningful.
/// Mirrored word for word by `shareGuard` in static/shared.js.
pub fn share_guard(hr_ksol: f64, net: f64) -> Option<String> {
    if !outweighs(hr_ksol, net) {
        return None;
    }
    let r = hr_ksol * 1000.0 / net;
    let how = if r >= 1.0 { format!("{r:.2}× the network estimate") } else { format!("{:.1}% of the network estimate", r * 100.0) };
    Some(format!(
        "Your hashrate is {how}. At 10% or more of the network a share-based estimate isn't meaningful: adding it would itself move the network hashrate and difficulty. Coins per day, revenue and operating margin are shown as n/a."
    ))
}

/// Why a per-machine figure is n/a.
pub fn per_day_na_reason(c: &Coin, hr_ksol: Option<f64>) -> &'static str {
    match (c.network.hashrate, hr_ksol) {
        (None, _) => "No network estimate is published, so there is no per-machine figure",
        (Some(_), Some(h)) if outweighs_network(c, h) => "One machine would be 10% or more of the network estimate, so a per-machine figure isn't meaningful",
        _ => "Block reward not sourced for this coin",
    }
}

/// Coins per day for a machine on a coin, using only sourced network data. None if anything is missing.
pub fn coins_per_day(c: &Coin, hr_ksol: f64, fee: f64) -> Option<f64> {
    if outweighs_network(c, hr_ksol) {
        return None;
    }
    let r = c.block_reward_miner.as_ref()?.value;
    estimate(hr_ksol, 0.0, 0.0, fee, None, r, c.network.hashrate?, c.network.block_time_target_s?).map(|e| e.coins_day)
}

fn money(v: Option<f64>) -> String {
    match v {
        Some(x) if x <= -0.005 => format!("−${:.2}", -x),
        Some(x) => format!("${:.2}", x.max(0.0)),
        None => fmt::NA.into(),
    }
}

fn coins(v: Option<f64>) -> String {
    match v {
        Some(x) if x >= 100.0 => format!("{x:.2}"),
        Some(x) if x >= 1.0 => format!("{x:.4}"),
        Some(x) => format!("{x:.6}"),
        None => fmt::NA.into(),
    }
}

pub fn render(d: &Data, q: &CalcQuery) -> Markup {
    let coins_list: Vec<_> = d.coins.iter().filter(|c| c.active()).collect();
    let coin_id = q.coin.clone().unwrap_or("zcash".into());
    let coin = coins_list.iter().find(|c| c.id == coin_id).or(coins_list.first()).cloned();
    let Some(coin) = coin else { return layout(d, Page { title: "Calculator", description: "", path: "/calculator", nav: "calculator" }, html! {}) };
    let ck = validate(q);
    let z15 = coin.z15_compatible == Some(true);
    // Defaults: an Antminer Z15 Pro (from data/miners.json) on 200,9 coins, blank otherwise.
    let pro = d.miners.iter().find(|m| m.model.ends_with("Z15 Pro"));
    let pick = |k: &str, dflt: Option<f64>| -> Option<f64> { if ck.errors.contains_key(k) { None } else { ck.get(k).or(dflt) } };
    let hr = pick("hashrate", if z15 { pro.and_then(|m| m.hashrate_ksol) } else { None });
    let watts = pick("watts", if z15 { pro.and_then(|m| m.watts) } else { None });
    let power = pick("power", Some(0.08));
    let fee = pick("fee", Some(1.0));
    let price = pick("price", coin.price_usd);
    let reward = pick("reward", coin.block_reward_miner.as_ref().map(|r| r.value));
    let net = pick("nethash", coin.network.hashrate);
    let bt = pick("blocktime", coin.network.block_time_target_s);
    let guard = match (hr, net) {
        (Some(h), Some(n)) if ck.errors.is_empty() => share_guard(h, n),
        _ => None,
    };
    let est = if ck.errors.is_empty() && guard.is_none() {
        match (hr, reward, net, bt) {
            (Some(h), Some(r), Some(n), Some(b)) => estimate(h, watts.unwrap_or(0.0), power.unwrap_or(0.0), fee.unwrap_or(0.0), price, r, n, b),
            _ => None,
        }
    } else {
        None
    };
    // Show what the user typed back to them when it was rejected.
    let raw_of = |k: &str| -> Option<String> {
        match k {
            "hashrate" => q.hashrate.clone(),
            "watts" => q.watts.clone(),
            "power" => q.power.clone(),
            "fee" => q.fee.clone(),
            "price" => q.price.clone(),
            "reward" => q.reward.clone(),
            "nethash" => q.nethash.clone(),
            _ => q.blocktime.clone(),
        }
    };
    let calc_json = serde_json::to_string(&coins_list.iter().map(|c| serde_json::json!({
        "id": c.id, "symbol": c.symbol, "label": c.label, "price": c.price_usd, "price_source": c.price_source,
        "reward": c.block_reward_miner.as_ref().map(|r| r.value), "reward_source": c.block_reward_miner.as_ref().and_then(|r| r.source_url.clone()),
        "reward_note": c.block_reward_miner.as_ref().and_then(|r| r.note.clone()),
        "nethash": c.network.hashrate, "nethash_source": c.network.hashrate_source, "nethash_blocks": c.network.hashrate_sample_blocks, "blocktime": c.network.block_time_target_s, "z15": c.z15_compatible, "source": c.source_url, "params": c.params(),
    })).collect::<Vec<_>>()).unwrap_or("[]".into());
    let num_in = |name: &'static str, v: Option<f64>, unit: &str, hint: Markup| {
        let f = field(name);
        let err = ck.errors.get(name);
        let shown = if err.is_some() { raw_of(name) } else { v.map(fmt::num_short) };
        html! {
            div class={"row" @if err.is_some() { " invalid" }} {
                label for={"c-" (name)} { (f.label) }
                div class="input-unit" {
                    input type="number" inputmode="decimal" name=(name) id={"c-" (name)} step="any"
                        min=(f.min) max=(f.max) data-min-exclusive=[f.min_exclusive.then_some("1")] data-unit=(f.unit)
                        value=[shown] aria-invalid=[err.map(|_| "true")] aria-describedby={"e-" (name) " h-" (name)};
                    span class="unit" { (unit) }
                }
                p class="err" id={"e-" (name)} role="alert" hidden[err.is_none()] { (err.cloned().unwrap_or_default()) }
                p class="hint" id={"h-" (name)} { (hint) }
            }
        }
    };
    let e30 = |v: Option<f64>| v.map(|x| x * 30.0);
    // Electricity doesn't depend on the share, so it still shows when the guard applies.
    let power_day = est.as_ref().map(|e| e.power_day).or_else(|| guard.as_ref().map(|_| watts.unwrap_or(0.0) / 1000.0 * 24.0 * power.unwrap_or(0.0)));
    layout(d, Page { title: "Zcash and Equihash mining calculator", description: "Estimate Zcash and Equihash mining revenue, electricity cost and operating margin for a Z15 Pro or custom hashrate using sourced network data.", path: "/calculator", nav: "calculator" }, html! {
        div class="wrap page" {
            header class="page-head" {
                h1 { "Zcash and Equihash mining calculator" }
                p class="lede" { "Estimate revenue, electricity cost and operating margin for one machine or a farm. It assumes the selected network hashrate, price and average luck hold, and excludes hardware cost, tax, downtime, stale shares, payout variance, hosting and import charges." }
            }
            div class="calc" {
                form class="calc-form" method="get" action="/calculator" id="calc" novalidate {
                    div class="row" {
                        label for="c-coin" { "Coin" }
                        select name="coin" id="c-coin" {
                            @for (label, cs) in d.param_groups(true) {
                                optgroup label=(label) data-rank-list {
                                    @for c in &cs { option value=(c.id) selected[c.id == coin.id] data-rank-id=(c.id) { (c.name) " (" (c.symbol) ")" } }
                                }
                            }
                        }
                    }
                    div class="presets" {
                        span { "Fill in a machine:" }
                        @for m in &d.miners { button type="button" class="preset" data-hr=[m.hashrate_ksol] data-w=[m.watts] title={(m.maker) " " (m.model) ": " (fmt::opt_num(m.hashrate_ksol)) " kSol/s, " (fmt::int(m.watts)) " W"} { (m.model.replace("Antminer ", "").replace(" ZMaster", "")) } }
                    }
                    fieldset {
                        legend { "Your side" }
                        (num_in("hashrate", hr, "kSol/s", html! {}))
                        (num_in("watts", watts, "W", html! {}))
                        (num_in("power", power, "$/kWh", html! {}))
                        (num_in("fee", fee, "%", html! { "0 to 100. Check your pool's page; the " a href="/" { "pool table" } " lists them." }))
                    }
                    fieldset {
                        legend { "Network side " span { "(filled from sources, edit freely)" } }
                        (num_in("price", price, "USD", html! { @if q.price.is_none() && coin.price_usd.is_some() { "miningpoolstats price feed" } @else if price.is_none() { "No sourced price for this coin. Enter one." } }))
                        (num_in("reward", reward, &coin.symbol, html! { @if let Some(r) = &coin.block_reward_miner { @if let Some(u) = &r.source_url { "Miner share of the block reward, from " (ext(u, &fmt::host(Some(u)).split('/').next().unwrap_or("").to_string())) "." } @if let Some(n) = &r.note { " " span class="reward-note" id="o-reward-note" { (n) } } } @else { "Not sourced for this coin. Enter it from the coin's explorer." } }))
                        (num_in("nethash", net, "Sol/s", nethash_hint(coin)))
                        (num_in("blocktime", bt, "s", html! { "Target block time" }))
                    }
                    button class="btn nojs-only" type="submit" { "Calculate" }
                }
                section class="calc-out" id="calc-out" aria-live="polite" {
                    h2 { "Estimate for " span id="o-name" { (coin.label) } }
                    p class="calc-bad" id="o-bad" hidden[ck.errors.is_empty()] { "Some inputs are out of range. Fix the marked fields and the figures will come back." }
                    p class="calc-bad" id="o-guard" hidden[guard.is_none()] { (guard.clone().unwrap_or_default()) }
                    table class="ledger" {
                        thead { tr { th {} th class="num" { "per day" } th class="num" { "per 30 days" } } }
                        tbody {
                            tr { th { "Mined" } td class="num" { span id="o-coins" { (coins(est.as_ref().map(|e| e.coins_day))) } " " span class="sym" { (coin.symbol) } } td class="num" { span id="o-coins30" { (coins(e30(est.as_ref().map(|e| e.coins_day)))) } " " span class="sym" { (coin.symbol) } } }
                            tr { th { "Revenue" } td class="num" id="o-rev" { (money(est.as_ref().and_then(|e| e.revenue_day))) } td class="num" id="o-rev30" { (money(e30(est.as_ref().and_then(|e| e.revenue_day)))) } }
                            tr { th { "Electricity" } td class="num" id="o-pow" { (money(power_day.map(|p| -p))) } td class="num" id="o-pow30" { (money(e30(power_day.map(|p| -p)))) } }
                            tr class="total" { th { "Estimated operating margin" } td class="num" id="o-profit" { (money(est.as_ref().and_then(|e| e.profit_day))) } td class="num" id="o-profit30" { (money(e30(est.as_ref().and_then(|e| e.profit_day)))) } }
                        }
                    }
                    dl class="facts" {
                        div { dt { "Your share of the network" } dd id="o-share" { (match (hr, net) { (Some(h), Some(n)) if est.is_some() => fmt::pct(Some(h * 1000.0 / n * 100.0)), _ => fmt::NA.into() }) } }
                        div { dt { "Break-even electricity price" } dd id="o-be" { (est.as_ref().and_then(|e| e.revenue_day).filter(|_| watts.unwrap_or(0.0) > 0.0).map(|r| format!("${:.3}/kWh", r / (watts.unwrap_or(1.0) / 1000.0 * 24.0))).unwrap_or(fmt::NA.into())) } }
                    }
                    p class="small" { "Mined per day = (your hashrate ÷ network hashrate) × (86,400 ÷ block time) × block reward × (1 − fee). Merged-mined aux coins are not included." }
                    p class="small" id="o-note" hidden[z15] { (coin.label) " uses Equihash " (coin.params()) ". The Antminer presets are 200,9 machines and won't mine it." }
                }
            }
        }
        script type="application/json" id="calc-data" { (PreEscaped(super::escape_script_json(&calc_json))) }
    })
}

/// Names where the prefilled network hashrate was read, rather than assuming miningpoolstats.
fn nethash_hint(c: &crate::data::Coin) -> Markup {
    let n = &c.network;
    match (n.hashrate, n.hashrate_source.as_deref()) {
        (Some(_), Some(u)) if u.starts_with("http") => html! {
            "Network estimate from " (ext(u, &fmt::host(Some(u)).split('/').next().unwrap_or("").to_string()))
            @if let Some(b) = n.hashrate_sample_blocks { ", over the previous " (b) " blocks" }
        },
        (Some(_), _) => html! { "Network estimate" },
        _ => html! { "Not published. Enter it." },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(pairs: &[(&str, &str)]) -> CalcQuery {
        let mut c = CalcQuery::default();
        for (k, v) in pairs {
            let v = Some(v.to_string());
            match *k {
                "coin" => c.coin = v,
                "hashrate" => c.hashrate = v,
                "watts" => c.watts = v,
                "power" => c.power = v,
                "fee" => c.fee = v,
                "price" => c.price = v,
                "reward" => c.reward = v,
                "nethash" => c.nethash = v,
                "blocktime" => c.blocktime = v,
                _ => {}
            }
        }
        c
    }

    #[test]
    fn fee_must_be_between_0_and_100() {
        assert!(check("fee", Some("101")).is_err());
        assert!(check("fee", Some("100.0001")).is_err());
        assert!(check("fee", Some("-0.5")).is_err());
        assert_eq!(check("fee", Some("0")).unwrap(), Some(0.0));
        assert_eq!(check("fee", Some("100")).unwrap(), Some(100.0));
        assert_eq!(check("fee", Some("2.5")).unwrap(), Some(2.5));
    }

    #[test]
    fn no_negatives_anywhere() {
        for f in FIELDS {
            assert!(check(f.name, Some("-1")).is_err(), "{} accepted -1", f.name);
        }
    }

    #[test]
    fn rejects_garbage_nan_and_infinity() {
        for bad in ["abc", "NaN", "inf", "-inf", "1e999", "1,5"] {
            assert!(check("hashrate", Some(bad)).is_err(), "accepted {bad}");
        }
    }

    #[test]
    fn zero_rules() {
        assert!(check("hashrate", Some("0")).is_err());
        assert!(check("nethash", Some("0")).is_err());
        assert!(check("reward", Some("0")).is_err());
        assert!(check("blocktime", Some("0")).is_err());
        assert_eq!(check("watts", Some("0")).unwrap(), Some(0.0));
        assert_eq!(check("power", Some("0")).unwrap(), Some(0.0));
        assert_eq!(check("price", Some("0")).unwrap(), Some(0.0));
    }

    #[test]
    fn upper_bounds() {
        assert!(check("power", Some("10.01")).is_err());
        assert!(check("hashrate", Some("2e9")).is_err());
        assert!(check("blocktime", Some("90000")).is_err());
        assert!(check("watts", Some("2e7")).is_err());
    }

    #[test]
    fn empty_means_not_given() {
        assert_eq!(check("price", None).unwrap(), None);
        assert_eq!(check("price", Some("  ")).unwrap(), None);
    }

    #[test]
    fn validate_collects_every_error() {
        let c = validate(&q(&[("fee", "150"), ("watts", "-3"), ("hashrate", "840")]));
        assert!(c.errors.contains_key("fee"));
        assert!(c.errors.contains_key("watts"));
        assert!(!c.errors.contains_key("hashrate"));
        assert_eq!(c.get("hashrate"), Some(840.0));
        assert_eq!(c.get("fee"), None);
    }

    #[test]
    fn estimate_refuses_out_of_range_inputs() {
        assert!(estimate(840.0, 2780.0, 0.08, 150.0, Some(1.0), 1.25, 3e10, 75.0).is_none());
        assert!(estimate(840.0, 2780.0, 0.08, -1.0, Some(1.0), 1.25, 3e10, 75.0).is_none());
        assert!(estimate(840.0, -1.0, 0.08, 1.0, Some(1.0), 1.25, 3e10, 75.0).is_none());
        assert!(estimate(840.0, 2780.0, 0.08, 1.0, Some(-1.0), 1.25, 3e10, 75.0).is_none());
        let e = estimate(840.0, 2780.0, 0.08, 100.0, Some(1.0), 1.25, 3e10, 75.0).unwrap();
        assert_eq!(e.coins_day, 0.0);
    }

    #[test]
    fn estimate_math() {
        // 840 kSol/s on a 30 GSol/s network, 75 s blocks, 1.25 reward, 1% fee
        let e = estimate(840.0, 2780.0, 0.08, 1.0, Some(100.0), 1.25, 3e10, 75.0).unwrap();
        let expect = 840e3 / 3e10 * (86400.0 / 75.0) * 1.25 * 0.99;
        assert!((e.coins_day - expect).abs() < 1e-12);
        assert!((e.power_day - 2.78 * 24.0 * 0.08).abs() < 1e-9);
    }

    #[test]
    fn no_per_machine_figure_when_one_machine_outweighs_the_network() {
        let mut c = Coin { block_reward_miner: Some(crate::data::Reward { value: 8.4, ..Default::default() }), ..Default::default() };
        c.network.hashrate = Some(506_255.0);
        c.network.block_time_target_s = Some(75.0);
        assert_eq!(coins_per_day(&c, 840.0, 1.0), None, "a Z15 Pro is bigger than the Wcash network");
        assert!(per_day_na_reason(&c, Some(840.0)).contains("10%"));
        assert!(coins_per_day(&c, 10.0, 1.0).is_some(), "a small share still gets a figure");
        c.network.hashrate = None;
        assert!(per_day_na_reason(&c, Some(840.0)).starts_with("No network estimate"));
    }

    #[test]
    fn share_is_capped_at_the_whole_network() {
        let e = estimate(1e8, 0.0, 0.0, 0.0, None, 1.0, 1e3, 60.0).unwrap();
        assert!((e.coins_day - 1440.0).abs() < 1e-9);
    }

    const GUARD_840: &str = "Your hashrate is 1.61× the network estimate. At 10% or more of the network a share-based estimate isn't meaningful: adding it would itself move the network hashrate and difficulty. Coins per day, revenue and operating margin are shown as n/a.";

    #[test]
    fn share_guard_matches_the_hardware_rule() {
        // Same literal as scripts/client.test.mjs, so server and client say the same thing.
        assert_eq!(share_guard(840.0, 522_000.0).as_deref(), Some(GUARD_840));
        assert!(share_guard(52.2, 522_000.0).unwrap().contains("10.0% of the network estimate"), "10% is inclusive");
        assert!(share_guard(52.1, 522_000.0).is_none());
        assert!(share_guard(840.0, 0.0).is_none() && share_guard(0.0, 1.0).is_none());
    }

    #[test]
    fn calculator_shows_na_not_a_capped_share_for_a_z15_pro_on_wcash() {
        let d = crate::data::load(&std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data")).unwrap();
        let page = render(&d, &q(&[("coin", "wcash"), ("hashrate", "840"), ("watts", "2780"), ("power", "0.08"), ("fee", "1"), ("nethash", "522000"), ("blocktime", "75")])).into_string();
        assert!(page.contains("id=\"o-guard\">Your hashrate is 1.61× the network estimate. At 10% or more of the network a share-based estimate isn"), "guard shown");
        assert!(page.contains("id=\"o-coins\">n/a<") && page.contains("id=\"o-share\">n/a<") && page.contains("id=\"o-rev\">n/a<"));
        assert!(!page.contains("10,148") && !page.contains("10148") && !page.contains("100.0%"));
        assert!(page.contains("id=\"o-pow\">−$5.34<"), "electricity still shows");
        // The WEC reward note: the subsidy ramps until height 40,000.
        assert!(page.contains("ramps up every block until height 40,000"), "reward note");
        // A small farm still gets a figure.
        let page = render(&d, &q(&[("coin", "wcash"), ("hashrate", "10"), ("nethash", "522000"), ("blocktime", "75")])).into_string();
        assert!(page.contains("id=\"o-guard\" hidden") && !page.contains("id=\"o-coins\">n/a<"));
    }
}
