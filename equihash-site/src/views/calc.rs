use crate::data::Data;
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup, PreEscaped};
use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
#[serde(default)]
pub struct CalcQuery {
    pub coin: Option<String>,
    pub hashrate: Option<f64>, // kSol/s
    pub watts: Option<f64>,
    pub power: Option<f64>, // $/kWh
    pub fee: Option<f64>,   // %
    pub price: Option<f64>,
    pub reward: Option<f64>,
    pub nethash: Option<f64>, // Sol/s
    pub blocktime: Option<f64>,
}

pub struct Estimate {
    pub coins_day: f64,
    pub revenue_day: Option<f64>,
    pub power_day: f64,
    pub profit_day: Option<f64>,
}

pub fn estimate(hr_ksol: f64, watts: f64, power: f64, fee: f64, price: Option<f64>, reward: f64, net: f64, bt: f64) -> Option<Estimate> {
    if hr_ksol <= 0.0 || net <= 0.0 || bt <= 0.0 || reward <= 0.0 {
        return None;
    }
    let share = (hr_ksol * 1000.0) / net;
    let coins_day = share * (86400.0 / bt) * reward * (1.0 - fee / 100.0);
    let power_day = watts / 1000.0 * 24.0 * power;
    let revenue_day = price.map(|p| coins_day * p);
    Some(Estimate { coins_day, revenue_day, power_day, profit_day: revenue_day.map(|r| r - power_day) })
}

pub fn render(d: &Data, q: &CalcQuery) -> Markup {
    let coins: Vec<_> = d.coins.iter().filter(|c| c.active()).collect();
    let coin_id = q.coin.clone().unwrap_or("zcash".into());
    let coin = coins.iter().find(|c| c.id == coin_id).or(coins.first()).cloned();
    let Some(coin) = coin else { return layout(d, Page { title: "Calculator", description: "", path: "/calculator", nav: "calculator" }, html! {}) };
    let hr = q.hashrate.unwrap_or(if coin.z15_compatible == Some(true) { 840.0 } else { 0.0 });
    let watts = q.watts.unwrap_or(if coin.z15_compatible == Some(true) { 2780.0 } else { 0.0 });
    let power = q.power.unwrap_or(0.08);
    let fee = q.fee.unwrap_or(1.0);
    let price = q.price.or(coin.price_usd);
    let reward = q.reward.or(coin.block_reward_miner.as_ref().map(|r| r.value));
    let net = q.nethash.or(coin.network.hashrate);
    let bt = q.blocktime.or(coin.network.block_time_target_s);
    let est = match (reward, net, bt) {
        (Some(r), Some(n), Some(b)) => estimate(hr, watts, power, fee, price, r, n, b),
        _ => None,
    };
    let calc_json = serde_json::to_string(&coins.iter().map(|c| serde_json::json!({
        "id": c.id, "symbol": c.symbol, "label": c.label, "price": c.price_usd, "price_source": c.price_source,
        "reward": c.block_reward_miner.as_ref().map(|r| r.value), "reward_source": c.block_reward_miner.as_ref().and_then(|r| r.source_url.clone()),
        "reward_note": c.block_reward_miner.as_ref().and_then(|r| r.note.clone()),
        "nethash": c.network.hashrate, "blocktime": c.network.block_time_target_s, "z15": c.z15_compatible, "source": c.source_url,
    })).collect::<Vec<_>>()).unwrap_or("[]".into());
    let num_in = |name: &str, label: &str, v: Option<f64>, step: &str, unit: &str, hint: Markup| html! {
        label class="field" {
            span { (label) }
            div class="input-unit" { input type="number" inputmode="decimal" name=(name) id={"c-" (name)} step=(step) min="0" value=[v.map(fmt::num_short)] placeholder="enter value"; span class="unit" { (unit) } }
            small class="hint" id={"h-" (name)} { (hint) }
        }
    };
    layout(d, Page { title: "Equihash mining profitability estimate", description: "Estimate Equihash mining revenue and power cost from your hashrate, wattage and power price, using sourced network hashrate, block reward and price.", path: "/calculator", nav: "calculator" }, html! {
        section class="wrap section" {
            h1 class="page-title" { "Profitability " span class="grad" { "estimate" } }
            p class="lede" { "A rough estimate, not a promise. It assumes constant network hashrate, difficulty, price and luck, and ignores pool payout variance, stales and hardware downtime." }
            div class="calc-grid" {
                form class="card pad calc-form" method="get" action="/calculator" id="calc" {
                    label class="field" {
                        span { "Coin" }
                        select name="coin" id="c-coin" {
                            @for c in &coins { option value=(c.id) selected[c.id == coin.id] { (c.symbol) " · " (c.label) } }
                        }
                    }
                    div class="presets" {
                        span class="muted sm" { "Presets:" }
                        @for m in &d.miners { button type="button" class="chip-btn" data-hr=[m.hashrate_ksol] data-w=[m.watts] { (m.model.replace("Antminer ", "")) } }
                    }
                    (num_in("hashrate", "Your hashrate", Some(hr), "any", "kSol/s", html! {}))
                    (num_in("watts", "Power draw", Some(watts), "any", "W", html! {}))
                    (num_in("power", "Electricity price", Some(power), "0.001", "$/kWh", html! {}))
                    (num_in("fee", "Pool fee", Some(fee), "0.1", "%", html! {}))
                    (num_in("price", "Coin price", price, "any", "USD", html! { @if q.price.is_none() && coin.price_usd.is_some() { "miningpoolstats price feed · editable" } @else if price.is_none() { "No sourced price: enter one" } }))
                    (num_in("reward", "Block reward to miners", reward, "any", &coin.symbol, html! { @if let Some(r) = &coin.block_reward_miner { @if let Some(u) = &r.source_url { "From " (ext(u, &fmt::host(Some(u)))) } } @else { "Not sourced for this coin: enter it" } }))
                    (num_in("nethash", "Network hashrate", net, "any", "Sol/s", html! { @if coin.network.hashrate.is_some() { "miningpoolstats" } @else { "Not published: enter it" } }))
                    (num_in("blocktime", "Target block time", bt, "any", "s", html! {}))
                    button class="btn primary nojs-only" type="submit" { "Calculate" }
                }
                div class="card pad calc-out" id="calc-out" {
                    p class="eyebrow" { "Estimated, per day" }
                    div class="out-big" { span id="o-coins" { (est.as_ref().map(|e| format!("{:.6}", e.coins_day)).unwrap_or("n/a".into())) } " " span class="muted" id="o-sym" { (coin.symbol) } }
                    dl class="out-list" {
                        div { dt { "Revenue" } dd id="o-rev" { (est.as_ref().and_then(|e| e.revenue_day).map(|v| format!("${v:.2}")).unwrap_or("n/a".into())) } }
                        div { dt { "Power cost" } dd id="o-pow" { (est.as_ref().map(|e| format!("${:.2}", e.power_day)).unwrap_or("n/a".into())) } }
                        div class="profit" { dt { "Profit" } dd id="o-profit" { (est.as_ref().and_then(|e| e.profit_day).map(|v| format!("${v:.2}")).unwrap_or("n/a".into())) } }
                        div { dt { "Per 30 days" } dd id="o-month" { (est.as_ref().and_then(|e| e.profit_day).map(|v| format!("${:.2}", v * 30.0)).unwrap_or("n/a".into())) } }
                        div { dt { "Your share of network" } dd id="o-share" { (match net { Some(n) if n > 0.0 => fmt::pct(Some(hr * 1000.0 / n * 100.0)), _ => "n/a".into() }) } }
                        div { dt { "Break-even power price" } dd id="o-be" { (est.as_ref().and_then(|e| e.revenue_day).filter(|_| watts > 0.0).map(|r| format!("${:.3}/kWh", r / (watts / 1000.0 * 24.0))).unwrap_or("n/a".into())) } }
                    }
                    p class="fine" { "Formula: (your hashrate ÷ network hashrate) × (86,400 ÷ block time) × block reward × (1 − fee). Merged-mined aux rewards are not included." }
                    @if coin.z15_compatible != Some(true) { p class="note" { (coin.label) " uses Equihash " (coin.params()) "; the Z15 presets don't apply to it." } }
                }
            }
        }
        script type="application/json" id="calc-data" { (PreEscaped(calc_json.replace("</", "<\\/"))) }
    })
}
