//! Source-backed institutional mining research notes.

use crate::data::{CypherpunkSource, Data};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};

pub const PATH: &str = "/research/cypherpunk-zcash-mining";

fn source<'a>(d: &'a Data, id: &str) -> Option<&'a CypherpunkSource> {
    d.cypherpunk.sources.iter().find(|s| s.id == id)
}

fn source_link(d: &Data, id: &str, label: &str) -> Markup {
    match source(d, id) {
        Some(s) => ext(&s.url, label),
        None => html! { span class="na" { (label) } },
    }
}

pub fn current_equivalent(d: &Data) -> Option<(f64, f64, String)> {
    let fleet = d.cypherpunk.reported_hashrate_gsol.filter(|v| *v > 0.0)?;
    let zec = d.coin("zcash")?;
    let network = zec.network.hashrate.filter(|v| *v > 0.0)? / 1_000_000_000.0;
    Some((
        fleet / network * 100.0,
        network,
        fmt::utc(zec.network.hashrate_observed_at.as_deref()),
    ))
}

pub fn render(d: &Data) -> Markup {
    let r = &d.cypherpunk;
    let fleet = r
        .reported_hashrate_gsol
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "n/a".into());
    let share = current_equivalent(d);
    let share_text = share
        .as_ref()
        .map(|(p, _, _)| format!("{p:.1}%"))
        .unwrap_or_else(|| "n/a".into());
    let network_text = share
        .as_ref()
        .map(|(_, n, _)| format!("{n:.2} GSol/s"))
        .unwrap_or_else(|| "n/a".into());
    let transaction = r
        .transaction_usd
        .map(|v| format!("${:.2}m", v / 1_000_000.0))
        .unwrap_or_else(|| "n/a".into());

    layout(
        d,
        Page {
            title: "Cypherpunk’s 4.2 GSol/s Zcash mining fleet",
            description: "A sourced account of Cypherpunk’s 4.2 GSol/s Zcash fleet, 4,902 Z15 Pro miners, Winklevoss transactions and the preliminary WINK Zcash ETF filing.",
            path: PATH,
            nav: "",
        },
        html! {
            article class="research-note" {
                header class="research-hero" {
                    div class="wrap research-hero-grid" {
                        div {
                            p class="eyebrow" { "NETWORK NOTE · 7 OCTOBER 2026" }
                            h1 { "Cypherpunk put 4.2 GSol/s behind Zcash." }
                            p class="lede" { "The U.S. fleet was about 18% of network hashpower at launch. Its share now moves with the network." }
                            p class="research-byline" { "Company-reported fleet figures, checked against SEC filings and compared with the latest Zcash network estimate on equihash.com." }
                        }
                        aside class="research-status" {
                            span { "STATUS" }
                            strong { "Fleet reported deployed" }
                            small { "Latest company confirmation: 22 September 2026" }
                            small class="research-company-link" { (source_link(d, "cypherpunk-site", "cypherpunk.com ↗")) }
                        }
                    }
                }
                div class="wrap research-body" {
                    dl class="research-facts" {
                        div { dt { "Reported fleet" } dd { (fleet) " GSol/s" } small { "Company figure" } }
                        div { dt { "Machines" } dd { (r.machine_count.map(|v| fmt::group(v as i128)).unwrap_or_else(|| "n/a".into())) } small { (&r.machine_model) } }
                        div { dt { "Current equivalent" } dd { (&share_text) } small { "Against " (&network_text) " network estimate" } }
                        div { dt { "Transaction" } dd { (transaction) } small { "Equity consideration" } }
                    }

                    section class="research-lead" aria-labelledby="what-happened" {
                        p class="research-kicker" { "THE FLEET" }
                        h2 id="what-happened" { "4,902 Z15 Pro miners at three U.S. sites" }
                        p { "Cypherpunk Technologies announced the fleet on 18 August 2026 after its mining subsidiary acquired " strong { "4,902 Bitmain Antminer Z15 Pro units" } " and their hosting agreements. The agreement names facilities in Barstow, Texas; Morristown, Tennessee; and Fairview, West Virginia." }
                        p { "Cypherpunk reported approximately " strong { (&fleet) " GSol/s" } " of Equihash 200,9 capacity. On launch day the company described that as approximately " strong { (r.launch_network_share_pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "n/a".into())) } " of Zcash network hashpower. That percentage is a dated launch snapshot." }
                        p class="source-line" { (source_link(d, "asset-agreement", "Asset Purchase Agreement")) " · " (source_link(d, "fleet-8k", "18 August announcement")) }
                    }

                    section class="research-comparison" aria-labelledby="size-now" {
                        div {
                            p class="research-kicker" { "SIZE NOW" }
                            h2 id="size-now" { "About " (&share_text) " of the latest network estimate" }
                            p { "The latest Zcash estimate stored by equihash.com is " strong { (&network_text) } ". Dividing Cypherpunk’s reported 4.2 GSol/s by that separate network snapshot gives " strong { (&share_text) } "." }
                        }
                        aside {
                            p class="formula mono" { "4.2 GSol/s ÷ " (&network_text) " = " (&share_text) }
                            p { "This is a comparison of two snapshots, not live telemetry from Cypherpunk. It assumes the reported fleet remains deployed at 4.2 GSol/s." }
                            @if let Some((_, _, observed)) = &share {
                                small { "Network estimate observed " (observed) ". " a href="/coin/zcash" { "Open the current Zcash record" } "." }
                            }
                        }
                    }

                    section aria-labelledby="production" {
                        p class="research-kicker" { "FIRST REPORTED OUTPUT" }
                        h2 id="production" { "3,023.13 ZEC from 18–31 August" }
                        p { "In a 22 September filing exhibit, Cypherpunk said the mining operation produced 3,023.13 ZEC during its first reported period, from 18 through 31 August, and added it to the company treasury. This is company-reported production; no public pool endpoint or independent fleet telemetry was identified." }
                        p class="source-line" { (source_link(d, "mining-follow-up", "22 September SEC filing exhibit")) }
                    }

                    section class="research-timeline" aria-labelledby="winklevoss-connection" {
                        p class="research-kicker" { "THE WINKLEVOSS CONNECTION" }
                        h2 id="winklevoss-connection" { "One network, three separate moves" }
                        ol {
                            li {
                                time datetime="2025-10" { "October 2025" }
                                div { strong { "Treasury financing." } p { "Winklevoss Capital led a $58.88 million private placement that initiated Cypherpunk’s Zcash treasury strategy. The total deal size is not the twins’ disclosed personal contribution." } }
                            }
                            li {
                                time datetime="2026-08-17" { "August 2026" }
                                div { strong { "Mining fleet transaction." } p { "Cypherpunk Mining acquired the fleet from Moria Mining in an equity transaction involving Winklevoss Treasury Investments. The SEC filing describes the legal counterparties; it does not mean the twins personally operate the fleet." } }
                            }
                            li {
                                time datetime="2026-10-06" { "October 2026" }
                                div { strong { "Zcash ETF filing." } p { "Winklevoss Asset Services filed a preliminary registration statement for a ZEC-holding exchange-traded product and named Cypherpunk as its proposed Zcash Ecosystem Partner." } }
                            }
                        }
                        p class="source-line" {
                            (source_link(d, "cyph-10k", "Cypherpunk 10-K")) " · "
                            (source_link(d, "fleet-8k-cover", "Fleet transaction 8-K")) " · "
                            (source_link(d, "winklevoss-ownership", "Winklevoss ownership filing"))
                        }
                    }

                    section class="research-etf" aria-labelledby="etf-title" {
                        header { span { "FILED · NOT LAUNCHED" } p { "Preliminary SEC registration statement" } }
                        div {
                            p class="research-kicker" { "PROPOSED WINK ETF" }
                            h2 id="etf-title" { "A direct ZEC product is proposed for Nasdaq" }
                            p { "The filing describes the " strong { "Winklevoss Zcash ETF" } ", proposed ticker " strong { "WINK" } ", as a trust that would hold ZEC directly. Gemini Trust is named as proposed custodian, the sponsor fee is stated as 0.25% annually, and Cypherpunk is named as the proposed “Zcash Ecosystem Partner.”" }
                            p { "The S-1 is preliminary. The product is not effective, approved or trading." }
                            p class="source-line" { (source_link(d, "wink-etf-s1", "Read the preliminary S-1")) }
                        }
                    }

                    section aria-labelledby="miners-watch" {
                        p class="research-kicker" { "WHY MINERS SHOULD CARE" }
                        h2 id="miners-watch" { "The fleet is large enough to move the operating baseline" }
                        p { "A 4.2 GSol/s fleet changes the network denominator miners compete against. If the fleet stays online while the rest of the network is unchanged, difficulty and each independent miner’s expected share adjust around it. Pool concentration still depends on where the fleet points its work; fleet ownership alone does not identify a public pool." }
                        ul {
                            li { "Watch network hashrate and difficulty rather than repeating the launch-day 18%." }
                            li { "Keep company-reported fleet capacity separate from public pool hashrate." }
                            li { "No source reviewed here establishes that Cypherpunk merge-mines Wcash. The evidence supports Zcash mining only." }
                        }
                    }

                    section class="research-sources" aria-labelledby="primary-sources" {
                        p class="research-kicker" { "PRIMARY SOURCES" }
                        h2 id="primary-sources" { "Documents behind this note" }
                        div class="table-scroll" {
                            table class="data" {
                                thead { tr { th { "Document" } th { "Type" } } }
                                tbody {
                                    @for s in &r.sources { tr { td { (ext(&s.url, &s.label)) } td { (&s.source_type) } } }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

pub fn home_note(d: &Data) -> Markup {
    let share = current_equivalent(d)
        .map(|(p, _, _)| format!("{p:.1}%"))
        .unwrap_or_else(|| "n/a".into());
    html! {
        section class="ed-research-note" aria-labelledby="cypherpunk-home-title" {
            div class="wrap ed-research-grid" {
                div {
                    p class="ed-section-index" { "NETWORK NOTE / ZCASH" }
                    h2 id="cypherpunk-home-title" { "Cypherpunk reports a 4.2 GSol/s U.S. fleet." }
                    p { "The 4,902-machine Z15 Pro fleet equals about " strong { (share) } " of the latest Zcash network estimate shown here. At launch, the company called it approximately 18%." }
                }
                dl {
                    div { dt { "Fleet" } dd { "4.2 GSol/s" } }
                    div { dt { "Machines" } dd { "4,902" } }
                    div { dt { "ETF status" } dd { "Filed, not launched" } }
                }
                a class="ed-light-button" href=(PATH) { "Read the sourced note" }
            }
        }
    }
}

pub fn coin_note(d: &Data) -> Markup {
    let share = current_equivalent(d)
        .map(|(p, _, _)| format!("{p:.1}%"))
        .unwrap_or_else(|| "n/a".into());
    html! {
        section class="side-card research-side-card" {
            p class="eyebrow" { "INSTITUTIONAL MINING" }
            h2 { "4.2 GSol/s reported fleet" }
            p { "Cypherpunk says 4,902 Z15 Pro miners are deployed in the U.S. That equals about " strong { (share) } " of this page’s latest network estimate." }
            a href=(PATH) { "Read the SEC-sourced note →" }
        }
    }
}

pub fn guide_note(d: &Data) -> Markup {
    let share = current_equivalent(d)
        .map(|(p, _, _)| format!("{p:.1}%"))
        .unwrap_or_else(|| "n/a".into());
    html! {
        section class="institutional-mining" aria-labelledby="institutional-mining-title" {
            p class="research-kicker" { "INDUSTRIAL ZCASH MINING" }
            h2 id="institutional-mining-title" { "One disclosed fleet reports 4.2 GSol/s" }
            p { "Cypherpunk Technologies says it operates 4,902 Antminer Z15 Pro units across three U.S. hosting locations. The reported fleet is equivalent to about " strong { (share) } " of the latest Zcash network estimate on this site; that comparison is not live fleet telemetry." }
            p { a href=(PATH) { "Read the fleet, transaction and proposed WINK ETF note" } }
        }
    }
}
