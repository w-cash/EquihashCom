use crate::data::{Data, Pool};
use crate::fmt;
use crate::views::home::schemes_text;
use crate::views::layout::{ext, layout, Page};
use crate::views::links;
use crate::views::logo::{self, At};
use maud::{html, Markup};

fn kv(label: &str, value: Markup) -> Markup {
    html! { tr { th scope="row" { (label) } td { (value) } } }
}

/// "data.miningpoolstats.stream · 2026-10-02 23:39 UTC" under a value: where it came from and when.
fn prov(src: Option<&str>, at: Option<&str>) -> Markup {
    if src.is_none() && at.is_none() {
        return html! {};
    }
    html! {
        br; span class="prov" {
            @if let Some(u) = src { (ext(u, fmt::host(Some(u)).split('/').next().unwrap_or(""))) }
            @if src.is_some() && at.is_some() { " · " }
            @if let Some(t) = at { (fmt::utc(Some(t))) }
        }
    }
}

fn basis_text(p: &Pool) -> &'static str {
    match p.basis.as_deref() {
        Some("operator_reported") => "reported by the operator, not independently measured",
        Some("estimated") => "estimated",
        Some("pool_api") => "from the pool's public hashrate API",
        Some(_) => "published by the pool",
        None => "not published",
    }
}

/// Label of the pool page's share row: what the share is measured against.
pub fn share_kv_label(p: &Pool) -> &'static str {
    if p.share_basis == "pools" {
        "Share of pool-reported hashrate"
    } else {
        "Share of network"
    }
}

/// Value of the pool page's share row (also sent in /api/live).
pub fn share_kv_value(p: &Pool) -> Markup {
    html! {
        @if let Some(n) = &p.share_note {
            (crate::views::home::share_text(p)) br; span class="na" { (n) }
        } @else {
            span class=[p.share_flag.then_some("red")] { (fmt::pct(p.network_share_pct)) }
            @if p.share_basis == "pools" { br; span class="na" { "The published network estimate is below the pools' total, so the share is of the listed pools." } }
        }
    }
}

pub fn pool_fields(d: &Data, p: &Pool) -> Markup {
    let unit = p.hashrate_unit.clone().unwrap_or("Sol/s".into());
    let coin = d.coin(&p.coin_id);
    html! {
        table class="kv" {
            tbody {
                (kv("Coin", html! { (p.coin_label) " " span class="sym" { (p.coin) } @if let Some(c) = coin { ", Equihash " (c.params()) } }))
                (kv("Hashrate", html! {
                    (fmt::hashrate(p.hashrate, &unit)) @if p.operator_reported() { span class="dag" { "†" } } " " span class="na" { "(" (basis_text(p)) @if let Some(w) = p.hashrate_window_s { ", accepted work over the previous " (w / 60) " minutes" } ")" }
                    (prov(p.hashrate_source.as_deref(), p.hashrate_observed_at.as_deref()))
                }))
                tr data-share-kv=(p.id) { th scope="row" { (share_kv_label(p)) } td { (share_kv_value(p)) } }
                (kv("Miners", html! { (fmt::int(p.miners)) (prov(p.miners_source.as_deref(), p.miners_observed_at.as_deref())) }))
                (kv("Workers", html! { (fmt::int(p.workers)) }))
                (kv("Fee", html! { (fmt::fee(p.fee_range())) (prov(p.fee_source.as_deref(), p.fee_observed_at.as_deref())) }))
                (kv("Payout", html! {
                    (schemes_text(p))
                    @let per: Vec<String> = p.schemes.iter().filter_map(|s| s.fee_pct.map(|f| format!("{} {}%", s.scheme, fmt::num_short(f)))).collect();
                    @if !per.is_empty() { br; span class="na" { (per.join(", ")) } }
                }))
                (kv("Minimum payout", html! { @if let Some(m) = p.min_payout { (fmt::num_short(m)) " " (p.min_payout_unit.clone().unwrap_or_default()) } @else { "n/a" } (prov(p.min_payout_source.as_deref(), p.min_payout_observed_at.as_deref())) }))
                (kv("Blocks in last 1,000", html! { (fmt::int(p.blocks_last_1000)) (prov(p.blocks_source.as_deref(), p.blocks_observed_at.as_deref())) }))
                (kv("Last block", html! { @if p.last_block_height.is_some() { (fmt::int(p.last_block_height)) ", " (fmt::utc(p.last_block_time.as_deref())) } @else { "n/a" } }))
                (kv("Region", html! { (p.region.clone().unwrap_or("n/a".into())) }))
                (kv("Merged mining", html! { @if p.merged() { (p.merged_mining.coins.join(", ")) @if let Some(n) = &p.merged_mining.note { br; span class="na" { (n) } } } @else { "None reported" } }))
                (kv("Website", html! { @if let Some(u) = &p.url { (ext(u, &fmt::host(Some(u)))) } @else { "n/a" } }))
                (kv("Source", html! { @if let Some(u) = &p.source_url { (ext(u, p.source_name.as_deref().unwrap_or(u))) } @else { "n/a" } }))
                (kv("Raw data", html! { @if let Some(u) = &p.data_url { (ext(u, &fmt::host(Some(u)))) } @else { "n/a" } }))
                (kv(if p.from_miningpoolstats { "Row fetched" } else { "Row checked by hand" }, html! { (fmt::utc(p.fetched_at.as_deref())) }))
            }
        }
        @if let Some(n) = &p.notes { p class="small" { (n) } }
    }
}

pub fn pool_page(d: &Data, p: &Pool) -> Markup {
    let title = format!("{} {} pool", p.name, p.coin);
    let desc = format!(
        "{} mining pool for {}: hashrate, network share, fee, payout scheme and sources.",
        p.name, p.coin_label
    );
    let path = format!("/pool/{}", p.slug);
    layout(
        d,
        Page {
            title: &title,
            description: &desc,
            path: &path,
            nav: "pools",
        },
        html! {
            div class="wrap page narrow" {
                p class="crumb" { a href={"/pools?coin=" (p.coin_id) "#pools"} { "All " (p.coin_label) " pools" } }
                header class="page-head" {
                    div class="title-row" {
                        h1 { (logo::chip(&p.logo, &p.name, At::Head, false)) (p.name) " " span class="sym" { (p.coin) } }
                        (copy_link(&path, "Copy link to this pool"))
                    }
                    (links::render(&p.links, &format!("{} links", p.name), "pool-links"))
                    @if p.share_flag { p class="alert" { strong { "Concentration. " } "This pool has more than 30% of the hashrate on " (p.coin_label) "." } }
                }
                (pool_fields(d, p))
                div class="pool-next" {
                    h2 { "Next steps" }
                    div class="card-actions" {
                        a href={"/coin/" (p.coin_id)} { "Open " (p.coin_label) " overview" }
                        a href={"/calculator?coin=" (p.coin_id)} { "Calculate mining output" }
                        a href={"/pools?coin=" (p.coin_id) "#pools"} { "Compare with other pools" }
                        a href="/contribute#pool" { "Correct this listing" }
                    }
                }
            }
        },
    )
}

/// "Copy link" for a permanent URL (needs JavaScript, so hidden without it). The button copies the
/// absolute URL and confirms with "Copied" in a polite live region.
pub fn copy_link(path: &str, aria: &str) -> Markup {
    html! {
        span class="copy-link-wrap js-only" {
            button type="button" class="copy-link" data-copy-url=(path) aria-label=(aria) { "Copy link" }
            span class="copy-status" role="status" aria-live="polite" {}
        }
    }
}

pub fn archive(d: &Data) -> Markup {
    let ended = d.ended_coin_pools();
    let ended_coins: Vec<_> = d.coins.iter().filter(|c| !c.active()).collect();
    layout(d, Page { title: "Archive: retired Equihash pools and ended coins", description: "Retired Equihash mining pools and coins whose proof of work ended, each with a citation.", path: "/archive", nav: "archive" }, html! {
        div class="wrap page" {
            header class="page-head" {
                h1 { "Archive" }
                p class="lede" { "Pools that have shut down, and Equihash coins that stopped using proof of work. Something only goes in here once a source confirms it, and the source is linked." }
            }
            h2 { "Retired pools" }
            div class="table-scroll" {
                table class="data archive stack" {
                    thead { tr { th scope="col" { "Pool" } th scope="col" { "Coin" } th scope="col" { "Retired" } th scope="col" { "What happened" } th scope="col" { "Sources" } } }
                    tbody {
                        @for a in &d.archive {
                            tr {
                                td { (logo::chip(&d.logos.pool(&a.id, &a.name), &a.name, At::Row, true)) strong { (a.name) } @if let Some(o) = &a.operator { br; span class="na" { (o) } } }
                                td data-label="Coin" { span class="sym" { (a.coin) } }
                                td class="nowrap" data-label="Retired" { (a.retired_label.clone().unwrap_or("n/a".into())) }
                                td { @if let Some(r) = &a.reason { (r) } @if let Some(n) = &a.notes { br; span class="na" { (n) } } }
                                td class="srcs" { @for (i, s) in a.sources.iter().enumerate() { @if i > 0 { br; } (ext(&s.url, &s.label)) } }
                            }
                        }
                    }
                }
            }
            h2 { "Coins whose Equihash proof of work ended" }
            div class="table-scroll" {
                table class="data archive stack" {
                    thead { tr { th scope="col" { "Coin" } th scope="col" { "n,k" } th scope="col" { "What happened" } th scope="col" { "Sources" } } }
                    tbody {
                        @for c in &ended_coins {
                            @let pools: Vec<_> = ended.iter().filter(|p| p.coin_id == c.id).collect();
                            tr {
                                td { (logo::chip(&c.logo, &c.name, At::Row, true)) strong { (c.name) } " " span class="sym" { (c.symbol) } }
                                td class="mono" data-label="Equihash" { (c.params()) }
                                td {
                                    @if let Some(n) = &c.status_note { (n) }
                                    @if !pools.is_empty() {
                                        br; span class="na" { "Still listed on miningpoolstats with no hashrate: "
                                            @for (i, p) in pools.iter().enumerate() { @if i > 0 { ", " } (p.name) @if let Some(t) = &p.last_block_time { " (last block " (fmt::utc(Some(t))) ")" } }
                                        }
                                    }
                                }
                                td class="srcs" { @for (i, s) in c.status_sources.iter().enumerate() { @if i > 0 { br; } (ext(&s.url, &s.label)) } }
                            }
                        }
                    }
                }
            }
        }
    })
}

pub fn about(d: &Data) -> Markup {
    layout(d, Page { title: "About", description: "About equihash.com, the open directory for Equihash coins, mining pools, hardware and guides.", path: "/about", nav: "about" }, html! {
        div class="wrap page narrow prose" {
            header class="page-head" { h1 { "About equihash.com" } p class="lede" { "equihash.com lists Equihash coins, mining pools, ASICs, sellers and hashpower markets, and shows where each number came from." } }
            p class="about-line" { "Maintained by " a href="https://x.com/MykytaSamardak" rel="me noopener" target="_blank" { "Mykyta Samardak (@MykytaSamardak)" } "." }
            p { "The same person maintains this site and Wcash. Only Zcash pools that support Wcash send work to it, so this site never counts all Zcash hashrate as Wcash hashrate." }
            h2 { "How it's run" }
            p { "Pool order comes from the figures in the table. Nobody can pay for a row, a higher position or a badge." }
            p { "The site has no referral links and does not collect payment for hardware, pool or coin listings. Corrections can be submitted from the " a href="/contribute" { "contribution page" } " and are checked against a public source." }
            p { "Every number links to its source and has a time checked. If a pool does not publish a value, the table says n/a instead of guessing." }
            p { "Concentration matters to everyone who mines a coin, so any pool above 30% of a network is marked in red." }
            h2 { "Where the data comes from" }
            p { "A small script reads the public miningpoolstats data for every Equihash coin, adds the pools that miningpoolstats doesn't list after checking them by hand, and writes plain JSON files that this site serves as they are. You can download them from the links at the bottom of every page. The details, and what couldn't be verified, are on the " a href="/sources" { "sources page" } "." }
        }
    })
}

pub fn sources(d: &Data) -> Markup {
    let m = &d.meta;
    layout(
        d,
        Page {
            title: "Sources and method",
            description:
                "Where every number on equihash.com comes from, and what could not be verified.",
            path: "/sources",
            nav: "",
        },
        html! {
            div class="wrap page narrow prose" {
                header class="page-head" {
                    h1 { "Sources and method" }
                    p class="lede" { "Last generated " (fmt::utc(m.generated_at.as_deref())) ": " (m.pool_count) " pool rows, " (m.mps_pool_count) " from miningpoolstats and " (m.non_mps_pool_count) " checked directly, across " (m.coin_count) " coins." }
                }
                h2 { "Machine-readable data" }
                p { "These stable JSON downloads contain the same records used to render the site. Fields keep their source and observation time where the upstream source publishes them. Units are stored with the value; an unavailable value is null, never zero." }
                ul {
                    li { a href="/data/pools.json" { "Pool records" } " — pool identity, coin, reported hashrate, fees, payout methods, regions and field-level sources." }
                    li { a href="/data/network.json" { "Network records" } " — Equihash parameters, network estimates, rewards, price inputs and observation times." }
                    li { a href="/data/miners.json" { "ASIC records" } " — manufacturer hashrate, power, parameter compatibility and specification sources." }
                    li { a href="/data/vendor-directory.json" { "Global ASIC vendor research" } ", " a href="/data/vendors.json" { "Equihash vendor records" } " and " a href="/data/listings.json" { "offer records" } " — evidence tiers, seller identity records and time-stamped public product claims." }
                    li { a href="/data/hashpower.json" { "Hashpower market snapshot" } " — aggregate NiceHash EQUIHASH order-book observations." }
                    li { a href="/data/current.json" { "Current snapshot manifest" } " — the exact generated files, sizes and SHA-256 values loaded by the server." }
                }
                p class="small" { "These downloads aggregate facts from the cited third-party sources. No separate licence is asserted for upstream data; consult each source for its terms." }
                h2 { "Read by the refresh script" }
                ul { @for s in &m.sources { li { (ext(&s.url, &s.label)) } } }
                @if !d.live.is_empty() {
                    h2 { "Polled live by this server" }
                    p { "These public endpoints are read server-side every " (d.live[0].poll_seconds) " s (listed in " code { "data/curated/live-sources.json" } "). A reading is used only when the endpoint says it is available; otherwise the last good one stays, with its age." }
                    ul {
                        @for l in &d.live {
                            li {
                                (ext(&l.url, l.label.as_deref().unwrap_or(&l.id)))
                                ": " (l.status)
                                @if let Some(r) = &l.reading { ", last good reading " (fmt::utc(Some(&r.observed_at))) }
                            }
                        }
                    }
                }
                h2 { "Hashpower market" }
                p { "The " a href="/hashpower" { "Hashpower" } " page reads an aggregate snapshot of the public NiceHash EQUIHASH BTC order book. Connected speed, visible order counts and top bids are observed values, not forecasts or guaranteed miner revenue. Individual order identifiers are discarded. The snapshot was observed " (fmt::utc(d.hashpower.observed_at.as_deref())) "." }
                ul {
                    @if let Some(u) = d.hashpower.source_url.as_deref() { li { (ext(u, "NiceHash public EQUIHASH order book")) } }
                    @if let Some(u) = d.hashpower.connection_guide_url.as_deref() { li { (ext(u, "NiceHash Equihash XNSUB notice")) } }
                    li { "Refresh script: " code { "scripts/refresh-hashpower.mjs" } "; public aggregate: " a href="/data/hashpower.json" { "data/hashpower.json" } "." }
                }
                h2 { "Curated by hand" }
                ul {
                    li { (ext("https://w.cash/whitepaper", "Wcash protocol specification")) " (merged-mining guide, WEC parameters)" }
                    li { (ext("https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-zcash-aux/README.md", "wcash-zcash-aux README at 3e6b8044")) }
                    li { (ext("https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-merge-miner/README.md", "wcash-merge-miner README at 3e6b8044")) }
                    li { "ASIC specs: Bitmain support spec pages and the Innosilicon product page, linked per row in the " a href="/asics" { "ASIC index" } "." }
                    li { "ASIC vendor identities and listing snapshots: seller pages, terms and public company records where available, linked per record in the " a href="/vendors" { "Vendor directory" } ". Approved product variants are checked daily; a blocked page, ambiguous match or implausible price keeps its previous good record and timestamp. Inventory, fulfillment and warranty performance are not independently checked." }
                    li { "Archive: pool and operator announcements and community threads, linked per entry on the " a href="/archive" { "Archive" } "." }
                }
                h2 { "Method" }
                ul {
                    li { "miningpoolstats: the coin page is loaded to read its current data timestamp, then " code { "data.miningpoolstats.stream/data/<coin>.js?t=<ts>" } " is fetched with a browser User-Agent and Referer." }
                    li { "miningpoolstats marks unknown or hidden values as −1 or −2. Those become n/a." }
                    li { "Share = pool hashrate ÷ the coin's network hashrate from the same file. On small coins the network estimate is sometimes below the sum of what pools report; then shares are of the listed pools' total, and the page says so." }
                    li { "Equihash parameters come from the miningpoolstats label. Plain \"Equihash\" is 200,9." }
                    li { "Regions are normalised from what each pool writes (\"US, EU, ASIA\", \"Canada\", \"RU\") into the buckets used by the region filter." }
                    (price_notes(d))
                    li { "Every hashrate, fee, miner count, block count and minimum payout carries its own source and observation time, shown on each pool's page. A field's time only moves when that field was fetched: refreshing a fee or a block height never makes a hashrate look newer." }
                    li { "Vendor refreshes use an allowlist of exact product variants, expected currency and price range. Future-batch wording is kept as editorial data; storefront structured data cannot silently turn it into an in-stock claim." }
                    li { "The network estimate and what the listed pools report are separate figures and are never substituted for each other. When no network estimate is published, the pools' total is shown with ≥ (a floor). † marks a figure the operator gave us rather than one we could read." }
                    li { "Coins are ranked within their exact Equihash parameter set by the sum of positive pool-reported hashrate; zero and n/a come last, by name. Hashrates on different parameter sets are never compared, summed or charted together." }
                    li { "n/a means not published; 0 means published as zero. The two are kept apart in the data and on the page." }
                    li { "Figures older than two hours get a visible stale notice." }
                    li { "Social and community links come from " code { "data/curated/links.json" } ". Only entries marked verified are shown, coin links next to the coin and pool links next to the pool."
                        @if let Some(g) = &d.links_generated_at { " That file was last generated " (fmt::utc(Some(g))) "." } }
                    li { (logo::sources_note(d)) }
                }
                h2 { "Research log" }
                div class="table-scroll" {
                    table class="data" {
                        thead { tr { th scope="col" { "Pool" } th scope="col" { "Outcome" } th scope="col" { "Detail" } } }
                        tbody { @for r in &d.research { tr {
                            td class="nowrap" { (r.pool) }
                            td class="nowrap" { span class={"outcome " (r.outcome)} { (r.outcome) } }
                            td { (r.detail) @if let Some(u) = &r.url { " " (ext(u, "link")) } }
                        } } }
                    }
                }
                h2 { "Coin notes" }
                ul { @for c in &d.research_coins { li { strong { (c.coin) } ": " (c.detail) } } }
                @if !m.errors.is_empty() {
                    h2 { "Warnings from the last refresh" }
                    ul class="mono small" { @for e in &m.errors { li { (e) } } }
                }
            }
        },
    )
}

pub fn not_found(d: &Data) -> Markup {
    layout(
        d,
        Page {
            title: "Not found",
            description: "Page not found.",
            path: "/404",
            nav: "",
        },
        html! {
            div class="wrap page narrow prose" {
                header class="page-head" {
                    h1 { "Nothing here" }
                    p class="lede" { "That page doesn't exist, or a listing was renamed. " a href="/" { "Back to the Equihash directory" } "." }
                }
            }
        },
    )
}

/// The Sources page's price line, computed from the data so it can't drift from what is shown:
/// which coins use the dedicated miningpoolstats price endpoint, which fall back to the price
/// field in their main coin file (and why), and which have no price at all.
pub fn price_notes(d: &Data) -> Markup {
    let active: Vec<&crate::data::Coin> = d.coins.iter().filter(|c| c.active()).collect();
    let dedicated = |c: &crate::data::Coin| {
        c.price_source
            .as_deref()
            .map(|s| s.contains("/price/"))
            .unwrap_or(false)
    };
    // "price <coin id>: <url>: HTTP 404" in meta.json's errors from the last refresh.
    let failed = |c: &crate::data::Coin| -> Option<String> {
        let prefix = format!("price {}:", c.id);
        d.meta
            .errors
            .iter()
            .find(|e| e.starts_with(&prefix))
            .map(|e| e.rsplit(": ").next().unwrap_or("error").to_string())
    };
    let n_dedicated = active
        .iter()
        .filter(|c| c.price_usd.is_some() && dedicated(c))
        .count();
    let fallback: Vec<&&crate::data::Coin> = active
        .iter()
        .filter(|c| c.price_usd.is_some() && !dedicated(c))
        .collect();
    let none_failed: Vec<&&crate::data::Coin> = active
        .iter()
        .filter(|c| c.price_usd.is_none() && failed(c).is_some())
        .collect();
    let none_quiet: Vec<&&crate::data::Coin> = active
        .iter()
        .filter(|c| c.price_usd.is_none() && failed(c).is_none())
        .collect();
    let names = |v: &[&&crate::data::Coin]| {
        v.iter()
            .map(|c| format!("{} ({})", c.label, c.symbol))
            .collect::<Vec<_>>()
            .join(", ")
    };
    html! {
        li {
            "Prices: " (n_dedicated) @if n_dedicated == 1 { " active coin uses" } @else { " active coins use" } " miningpoolstats' dedicated price endpoint (" code { "data/price/<coin>.js" } "). "
            @for c in &fallback {
                (c.label) " (" (c.symbol) ") shows " (fmt::price(c.price_usd)) " from the fallback price field in its main miningpoolstats coin file"
                @if let Some(e) = failed(c) { ", because its dedicated price endpoint failed at the last refresh (" (e) ")" } ". "
            }
            @if !none_failed.is_empty() {
                "No price (n/a) for " (names(&none_failed)) ": the dedicated price endpoint failed at the last refresh ("
                (none_failed.iter().filter_map(|c| failed(c)).collect::<Vec<_>>().join(", ")) ") and the coin file has no fallback price. "
            }
            @if !none_quiet.is_empty() { "No price is published for " (names(&none_quiet)) ", so those show n/a. " }
            "Prices appear in the coin header, the networks table, the ASIC index's USD/day column and the calculator, where they can be overridden."
        }
    }
}
