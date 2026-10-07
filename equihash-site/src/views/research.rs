//! Source-backed briefings on companies, capital and infrastructure around Equihash.

use crate::data::{CypherpunkSource, Data};
use crate::fmt;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};

pub const INDUSTRY_PATH: &str = "/industry";
pub const CYPHERPUNK_PATH: &str = "/industry/cypherpunk-zcash-mining";
pub const WINK_PATH: &str = "/industry/winklevoss-zcash-etf";
pub const GRAYSCALE_PATH: &str = "/industry/grayscale-zcash-etf";

fn source<'a>(d: &'a Data, id: &str) -> Option<&'a CypherpunkSource> {
    d.cypherpunk.sources.iter().find(|s| s.id == id)
}

fn source_link(d: &Data, id: &str, label: &str) -> Markup {
    match source(d, id) {
        Some(s) => ext(&s.url, label),
        None => html! { span class="na" { (label) } },
    }
}

fn grayscale_source<'a>(d: &'a Data, id: &str) -> Option<&'a CypherpunkSource> {
    d.grayscale.sources.iter().find(|s| s.id == id)
}

fn grayscale_source_link(d: &Data, id: &str, label: &str) -> Markup {
    match grayscale_source(d, id) {
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

#[allow(dead_code)]
fn cypherpunk_legacy(d: &Data) -> Markup {
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
            title: "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet",
            description: "A sourced account of Cypherpunk’s reported 4.2 GSol/s Zcash fleet, 4,902 Z15 Pro miners, first production and transaction records.",
            path: CYPHERPUNK_PATH,
            nav: "industry",
        },
        html! {
            article class="research-note" {
                header class="research-hero" {
                    div class="wrap research-hero-grid" {
                        div {
                            p class="eyebrow" { "MINING FLEET · CYPHERPUNK · 7 OCTOBER 2026" }
                            h1 { "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet." }
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
                                div { strong { "Zcash ETF filing." } p { "Winklevoss Asset Services filed a preliminary registration statement for a ZEC-holding exchange-traded product. The filing says the trust entered an agreement with Cypherpunk to serve as its Zcash Ecosystem Partner." } }
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
                            p { "The filing describes the " strong { "Winklevoss Zcash ETF" } ", proposed ticker " strong { "WINK" } ", as a trust that would hold ZEC directly. It names Gemini Trust as custodian, states a 0.25% annual sponsor fee, and discloses an agreement for Cypherpunk to serve as “Zcash Ecosystem Partner.”" }
                            p { "The S-1 is preliminary. The product is not effective, approved or trading. This filing is separate from Cypherpunk’s mining operation." }
                            p class="source-line" { a href=(WINK_PATH) { "Read the full WINK filing briefing" } " · " (source_link(d, "wink-etf-s1", "Open the preliminary S-1")) }
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

#[allow(dead_code)]
fn index_legacy(d: &Data) -> Markup {
    let share = current_equivalent(d)
        .map(|(p, _, _)| format!("{p:.1}%"))
        .unwrap_or_else(|| "n/a".into());
    layout(
        d,
        Page {
            title: "Equihash industry: companies, capital and infrastructure",
            description: "Source-backed briefings on companies, mining operations, investment products and infrastructure around Equihash and Zcash.",
            path: INDUSTRY_PATH,
            nav: "industry",
        },
        html! {
            div class="industry-index" {
                header class="wrap industry-head" {
                    p class="eyebrow" { "COMPANIES, CAPITAL AND INFRASTRUCTURE" }
                    h1 { "Equihash industry" }
                    p class="lede" { "Companies, mining operations and capital around Equihash networks, traced to primary records." }
                }
                section class="wrap industry-lead" aria-labelledby="industry-lead-title" {
                    a class="industry-lead-link" href=(CYPHERPUNK_PATH) {
                        div {
                            p class="research-kicker" { "MINING FLEET · CYPHERPUNK" }
                            h2 id="industry-lead-title" { "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet" }
                            p { "The company says 4,902 Z15 Pro units are deployed across three U.S. sites. The reported fleet equals about " strong { (share) } " of the latest Zcash network estimate on this site." }
                            span { "Read the fleet briefing →" }
                        }
                        dl {
                            div { dt { "Reported fleet" } dd { "4.2 GSol/s" } }
                            div { dt { "Machines" } dd { "4,902" } }
                            div { dt { "Evidence" } dd { "SEC filings" } }
                        }
                    }
                }
                section class="wrap industry-list" aria-labelledby="industry-list-title" {
                    header { p class="ed-section-index" { "LATEST BRIEFINGS" } h2 id="industry-list-title" { "Follow the institutions around Zcash" } }
                    article {
                        p class="research-kicker" { "FILING BRIEF · WINKLEVOSS" }
                        h3 { a href=(WINK_PATH) { "WINK filed: what the preliminary Zcash ETF prospectus says" } }
                        p { "A proposed Nasdaq product, a 0.25% sponsor fee and Cypherpunk’s disclosed advisory agreement—without implying the fund is launched or contributes hashpower." }
                        div class="industry-meta" { time datetime="2026-10-06" { "6 October 2026" } span { "PRELIMINARY · NOT TRADING" } }
                    }
                    article {
                        p class="research-kicker" { "FUND BRIEF · GRAYSCALE" }
                        h3 { a href=(GRAYSCALE_PATH) { "From OTC trust to ZCSH: Grayscale’s Zcash vehicle changed shape" } }
                        p { "The product formed in 2017 now trades on NYSE Arca. The filings show its structure, fee, split and custody arrangements—and that it is not a miner." }
                        div class="industry-meta" { time datetime="2026-10-07" { "Updated 7 October 2026" } span { "TRADING · NYSE ARCA" } }
                    }
                }
            }
        },
    )
}

pub fn cypherpunk(d: &Data) -> Markup {
    let r = &d.cypherpunk;
    let fleet = r
        .reported_hashrate_gsol
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "n/a".into());
    let machines = r
        .machine_count
        .map(|v| fmt::group(v as i128))
        .unwrap_or_else(|| "n/a".into());
    let output = r
        .mined_zec
        .map(|v| format!("{} ZEC", fmt::num_short(v)))
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

    layout(
        d,
        Page {
            title: "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet",
            description: "A sourced account of Cypherpunk’s reported 4.2 GSol/s Zcash fleet, 4,902 Z15 Pro miners, first production and transaction records.",
            path: CYPHERPUNK_PATH,
            nav: "industry",
        },
        html! {
            article class="briefing" {
                header class="wrap briefing-head" {
                    a class="briefing-crumb" href=(INDUSTRY_PATH) { "Industry / Cypherpunk" }
                    p class="eyebrow" { "MINING FLEET · COMPANY DISCLOSURE" }
                    h1 { "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet" }
                    p class="briefing-deck" { "The company reports 4,902 Z15 Pro miners at three U.S. sites. Its filings set out the fleet acquisition, hosting agreements and first production figures." }
                    p class="briefing-byline" { "equihash.com · 7 October 2026 · " (source_link(d, "mining-follow-up", "Company update: 22 September 2026 ↗")) }
                }
                div class="wrap briefing-main" {
                    dl class="briefing-facts" {
                        div { dt { "Reported capacity" } dd { (&fleet) " GSol/s" } small { "Company figure" } }
                        div { dt { "Antminer Z15 Pro units" } dd { (&machines) } small { "Three U.S. sites" } }
                        div { dt { "18–31 August output" } dd { (&output) } small { "Company-reported production" } }
                    }
                    figure class="briefing-record" aria-labelledby="fleet-record-title" {
                        div class="briefing-machine" {
                            img src="/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp" alt="Bitmain Antminer Z15 Pro mining machine" width="1200" height="1200";
                            figcaption { strong { "Antminer Z15 Pro" } "Product image · " (source_link(d, "asset-agreement", "fleet equipment record ↗")) }
                        }
                        div class="briefing-timeline" {
                            p id="fleet-record-title" { "FLEET RECORD · AUG–SEP 2026" }
                            ol {
                                li { time datetime="2026-08-17" { "17 AUG" } div { strong { "4,902 machines" } span { "Miners and hosting agreements acquired." } (source_link(d, "asset-agreement", "Purchase agreement ↗")) } }
                                li { time datetime="2026-08-18" { "18 AUG" } div { strong { "Mining announced" } span { "Sites in Texas, Tennessee and West Virginia." } (source_link(d, "fleet-8k", "Company announcement ↗")) } }
                                li { time datetime="2026-09-22" { "22 SEP" } div { strong { "3,023.13 ZEC" } span { "Reported output for 18–31 August." } (source_link(d, "mining-follow-up", "Production update ↗")) } }
                            }
                            small { "Company-reported figures, published as SEC filing exhibits." }
                        }
                    }
                    div class="briefing-layout" {
                        nav class="briefing-contents" aria-label="In this briefing" {
                            p { "IN THIS BRIEFING" }
                            a href="#fleet" { "The fleet" }
                            a href="#production" { "First production" }
                            a href="#miners" { "For other miners" }
                            a href="#connections" { "Connected moves" }
                            a href="#primary-sources" { "Sources" }
                        }
                        div class="briefing-copy" {
                            section id="fleet" aria-labelledby="what-happened" {
                                h2 id="what-happened" { "4,902 Z15 Pro miners at three U.S. sites" }
                                p { "Cypherpunk Technologies announced the fleet on 18 August 2026 after its mining subsidiary acquired 4,902 Bitmain Antminer Z15 Pro units and their hosting agreements. The agreement names facilities in Barstow, Texas; Morristown, Tennessee; and Fairview, West Virginia." }
                                p { "The company reported approximately " strong { (&fleet) " GSol/s" } " of Equihash 200,9 capacity. It described this as approximately " strong { (r.launch_network_share_pct.map(|v| format!("{v:.0}%")).unwrap_or_else(|| "n/a".into())) } " of Zcash network hashpower on 18 August. That percentage is a dated company estimate." }
                                p class="source-line" { (source_link(d, "asset-agreement", "Asset Purchase Agreement")) " · " (source_link(d, "fleet-8k", "18 August announcement")) }
                            }
                            section id="production" aria-labelledby="production-title" {
                                h2 id="production-title" { "3,023.13 ZEC in the first reported period" }
                                p { "In its 22 September update, Cypherpunk said the operation produced 3,023.13 ZEC from 18 through 31 August and added it to the company treasury." }
                                p { "The figure comes from the company’s filing exhibit. The sources reviewed do not include independent fleet telemetry or a public pool endpoint." }
                                p class="source-line" { (source_link(d, "mining-follow-up", "22 September production update")) }
                            }
                            section id="miners" aria-labelledby="miners-watch" {
                                h2 id="miners-watch" { "What this means for other Zcash miners" }
                                p { "A fleet of this size can materially affect the network hashrate that other miners compete against. Its share depends on both its operating capacity and the rest of the network." }
                                @if let Some((_, _, observed)) = &share {
                                    aside class="briefing-comparison" {
                                        strong { "Share if the reported fleet remains online: " (&share_text) }
                                        span { "4.2 GSol/s compared with a " (&network_text) " network estimate observed " (observed) ". This combines separate snapshots; it is not fleet telemetry." }
                                        a href="/coin/zcash" { "Open the current Zcash record →" }
                                    }
                                }
                                p { "Fleet ownership does not identify the pool receiving its work. The documents reviewed establish Zcash mining; they do not establish Wcash merged mining." }
                            }
                            section id="connections" aria-labelledby="connections-title" {
                                h2 id="connections-title" { "Three connected moves, kept separate" }
                                ol class="briefing-events" {
                                    li { time datetime="2025-10" { "October 2025" } div { strong { "Treasury financing" } p { "Winklevoss Capital led a $58.88 million private placement that began Cypherpunk’s Zcash treasury strategy. The total deal size is not a disclosed personal contribution from the twins." } } }
                                    li { time datetime="2026-08-17" { "17 August 2026" } div { strong { "Fleet transaction" } p { "Cypherpunk Mining acquired the fleet from Moria Mining in an equity transaction involving Winklevoss Treasury Investments." } } }
                                    li { time datetime="2026-10-06" { "6 October 2026" } div { strong { "Preliminary WINK filing" } p { "Winklevoss Asset Services filed for a ZEC-holding exchange-traded product and named Cypherpunk as Zcash Ecosystem Partner." } } }
                                }
                                p class="source-line" { (source_link(d, "cyph-10k", "Cypherpunk 10-K")) " · " (source_link(d, "fleet-8k-cover", "Fleet transaction 8-K")) " · " a href=(WINK_PATH) { "WINK filing briefing" } }
                            }
                            section id="primary-sources" class="briefing-sources" aria-labelledby="sources-title" {
                                h2 id="sources-title" { "Sources" }
                                ol {
                                    @for s in &r.sources { li { (ext(&s.url, &s.label)) span { (&s.source_type) } } }
                                }
                            }
                        }
                    }
                }
            }
        },
    )
}

pub fn index(d: &Data) -> Markup {
    layout(
        d,
        Page {
            title: "Equihash industry: companies, capital and infrastructure",
            description: "Briefings on companies, mining operations, investment products and infrastructure around Equihash and Zcash, linked to primary records.",
            path: INDUSTRY_PATH,
            nav: "industry",
        },
        html! {
            div class="industry-index industry-desk" {
                header class="wrap industry-head" {
                    p class="eyebrow" { "COMPANIES, CAPITAL AND INFRASTRUCTURE" }
                    h1 { "Industry" }
                    p class="lede" { "Mining companies, funds and infrastructure connected to Zcash and other Equihash networks." }
                }
                section class="wrap industry-lead" aria-labelledby="industry-lead-title" {
                    div class="industry-lead-grid" {
                        article class="industry-feature" {
                            p class="research-kicker" { "MINING · CYPHERPUNK" }
                            h2 id="industry-lead-title" { a href=(CYPHERPUNK_PATH) { "Cypherpunk reports a 4.2 GSol/s Zcash mining fleet" } }
                            p { "The company reports 4,902 Z15 Pro miners at three U.S. sites. Its filings detail the acquisition, hosting agreements and first production figures." }
                            p class="industry-date" { "7 October 2026 · Company disclosures" }
                            a class="industry-read" href=(CYPHERPUNK_PATH) { "Read the fleet briefing →" }
                            figure class="industry-machine" {
                                img src="/static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp" alt="Bitmain Antminer Z15 Pro mining machine" width="1200" height="1200";
                                figcaption { strong { "Antminer Z15 Pro" } "Product image · " (source_link(d, "asset-agreement", "fleet equipment record ↗")) }
                            }
                        }
                        figure class="industry-evidence" {
                            figcaption { "FLEET RECORD · AUG–SEP 2026" }
                            ol {
                                li { time datetime="2026-08-17" { "17 AUG" } div { strong { "4,902 machines" } span { "Miners and hosting agreements acquired." } (source_link(d, "asset-agreement", "Purchase agreement ↗")) } }
                                li { time datetime="2026-08-18" { "18 AUG" } div { strong { "Mining announced" } span { "Sites in Texas, Tennessee and West Virginia." } (source_link(d, "fleet-8k", "Company announcement ↗")) } }
                                li { time datetime="2026-09-22" { "22 SEP" } div { strong { "3,023.13 ZEC" } span { "Reported output for 18–31 August." } (source_link(d, "mining-follow-up", "Production update ↗")) } }
                            }
                            small { "Company-reported figures, published as SEC filing exhibits." }
                        }
                    }
                }
                section class="wrap industry-list" aria-labelledby="industry-list-title" {
                    header { p class="ed-section-index" { "MORE BRIEFINGS" } h2 id="industry-list-title" { "Funds and filings" } }
                    article {
                        p class="research-kicker" { "FUND · GRAYSCALE" }
                        h3 { a href=(GRAYSCALE_PATH) { "Grayscale’s Zcash fund: listing, fees and holdings" } }
                        p { "ZCSH trades on NYSE Arca and holds ZEC. The filings record its listing, custody arrangements and annual sponsor fee." }
                        div class="industry-meta" { time datetime="2026-10-07" { "Updated 7 October 2026" } span { "TRADING · NYSE ARCA" } }
                    }
                    article {
                        p class="research-kicker" { "FILING · WINKLEVOSS" }
                        h3 { a href=(WINK_PATH) { "Winklevoss files for a Zcash ETF under proposed ticker WINK" } }
                        p { "The 6 October filing proposes direct ZEC holdings, a 0.25% annual sponsor fee and Gemini custody. The registration statement remains preliminary." }
                        div class="industry-meta" { time datetime="2026-10-06" { "6 October 2026" } span { "PRELIMINARY FILING" } }
                    }
                }
                section class="wrap industry-companies" aria-labelledby="industry-companies-title" {
                    h2 id="industry-companies-title" { "Companies in focus" }
                    ul {
                        li { a href=(CYPHERPUNK_PATH) { span { strong { "Cypherpunk Technologies" } small { "Zcash mining and treasury" } } b aria-hidden="true" { "→" } } }
                        li { a href=(GRAYSCALE_PATH) { span { strong { "Grayscale" } small { "Zcash investment product" } } b aria-hidden="true" { "→" } } }
                        li { a href=(WINK_PATH) { span { strong { "Winklevoss Asset Services" } small { "Proposed WINK sponsor" } } b aria-hidden="true" { "→" } } }
                    }
                }
            }
        },
    )
}

pub fn wink(d: &Data) -> Markup {
    let e = &d.cypherpunk.etf;
    layout(
        d,
        Page {
            title: "WINK filed: what the preliminary Zcash ETF prospectus says",
            description: "What the 6 October 2026 preliminary Winklevoss Zcash ETF filing says about WINK, Gemini custody, fees and Cypherpunk’s disclosed role.",
            path: WINK_PATH,
            nav: "industry",
        },
        html! {
            article class="research-note" {
                header class="research-hero" {
                    div class="wrap research-hero-grid" {
                        div {
                            p class="eyebrow" { "FILING BRIEF · WINKLEVOSS · 7 OCTOBER 2026" }
                            h1 { "WINK filed: what the preliminary Zcash ETF prospectus says." }
                            p class="lede" { "Winklevoss Asset Services filed a proposed spot-ZEC product on 6 October. WINK is a proposed ticker, and the filing is not yet an operating fund." }
                            p class="research-byline" { "Read from the preliminary registration statement filed with the U.S. Securities and Exchange Commission." }
                        }
                        aside class="research-status" {
                            span { "STATUS" }
                            strong { "Preliminary filing" }
                            small { "Not effective, launched or trading" }
                            small class="research-company-link" { (source_link(d, "wink-etf-s1", "Open the SEC filing ↗")) }
                        }
                    }
                }
                div class="wrap research-body" {
                    dl class="research-facts" {
                        div { dt { "Proposed ticker" } dd { (&e.proposed_ticker) } small { "Nasdaq, subject to issuance" } }
                        div { dt { "Proposed fee" } dd { (e.annual_fee_pct.map(|v| format!("{v:.2}%")).unwrap_or_else(|| "n/a".into())) } small { "Annual sponsor fee" } }
                        div { dt { "Sponsor" } dd class="research-fact-text" { "Winklevoss Asset Services" } small { "Filing party" } }
                        div { dt { "Custodian" } dd class="research-fact-text" { "Gemini Trust" } small { "Named in the filing" } }
                    }
                    section aria-labelledby="wink-proposes" {
                        p class="research-kicker" { "WHAT THE FILING PROPOSES" }
                        h2 id="wink-proposes" { "A trust intended to hold ZEC directly" }
                        p { "The preliminary S-1 describes the Winklevoss Zcash ETF as a trust that would hold ZEC and issue shares proposed for Nasdaq under the ticker WINK. Winklevoss Asset Services, LLC is named as sponsor, Gemini Trust Company, LLC as custodian, and the sponsor fee is stated as 0.25% annually." }
                        p { "Cameron Winklevoss signed the filing as chief executive of the sponsor. The document does not establish that the product has become effective or that shares are available to trade." }
                        p class="source-line" { (source_link(d, "wink-etf-s1", "Preliminary registration statement")) }
                    }
                    section aria-labelledby="cypherpunk-role" {
                        p class="research-kicker" { "CYPHERPUNK’S DISCLOSED ROLE" }
                        h2 id="cypherpunk-role" { "Protocol guidance is separate from mining" }
                        p { "The filing says the trust entered an agreement for Cypherpunk to serve as Zcash Ecosystem Partner on matters that may include network polling, voting and protocol guidance. The fund remains preliminary. The agreement does not mean WINK owns Cypherpunk’s miners, funds the fleet or adds Equihash hashpower." }
                        p { "Cypherpunk’s reported mining operation is covered separately so the evidence for the fleet and the proposed fund are not mixed together." }
                        p class="source-line" { a href=(CYPHERPUNK_PATH) { "Read the Cypherpunk mining fleet briefing" } }
                    }
                    section aria-labelledby="wink-watch" {
                        p class="research-kicker" { "WHAT TO WATCH" }
                        h2 id="wink-watch" { "A filing is the beginning, not the launch" }
                        ul {
                            li { "An effective registration statement and exchange notice would be needed before describing WINK as a trading product." }
                            li { "The proposed fee, custodian and other terms may change in later amendments." }
                            li { "A ZEC-holding product can affect market access and demand; it does not perform proof of work." }
                        }
                    }
                    section class="research-sources" aria-labelledby="wink-sources" {
                        p class="research-kicker" { "PRIMARY SOURCE" }
                        h2 id="wink-sources" { "Document behind this briefing" }
                        div class="table-scroll" { table class="data" { thead { tr { th { "Document" } th { "Type" } } } tbody {
                            @if let Some(s) = source(d, "wink-etf-s1") { tr { td { (ext(&s.url, &s.label)) } td { (&s.source_type) } } }
                        } } }
                    }
                }
            }
        },
    )
}

pub fn grayscale(d: &Data) -> Markup {
    let r = &d.grayscale;
    let aum = r
        .issuer_aum_usd
        .map(|v| format!("${:.0}m+", v / 1_000_000.0))
        .unwrap_or_else(|| "n/a".into());
    layout(
        d,
        Page {
            title: "From OTC trust to ZCSH: Grayscale’s Zcash vehicle changed shape",
            description: "The filed timeline, structure, fee, custody and limits of Grayscale’s NYSE Arca-traded Zcash product, ZCSH.",
            path: GRAYSCALE_PATH,
            nav: "industry",
        },
        html! {
            article class="research-note" {
                header class="research-hero" {
                    div class="wrap research-hero-grid" {
                        div {
                            p class="eyebrow" { "FUND BRIEF · GRAYSCALE · 7 OCTOBER 2026" }
                            h1 { "From OTC trust to ZCSH: Grayscale’s Zcash vehicle changed shape." }
                            p class="lede" { "The vehicle formed in 2017 became an NYSE Arca-traded ZEC product in August 2026. Here is the filed timeline, the $500 million AUM claim, and the limits of its connection to mining." }
                            p class="research-byline" { "Product status and terms checked against SEC filings and the issuer’s official product page." }
                        }
                        aside class="research-status" {
                            span { "STATUS" }
                            strong { "Trading on NYSE Arca" }
                            small { "Ticker ZCSH · ZEC-holding product" }
                            small class="research-company-link" { (grayscale_source_link(d, "zcsh-site", "Official product page ↗")) }
                        }
                    }
                }
                div class="wrap research-body" {
                    dl class="research-facts" {
                        div { dt { "Ticker" } dd { (&r.ticker) } small { (&r.primary_market) } }
                        div { dt { "Sponsor fee" } dd { (r.sponsor_fee_pct.map(|v| format!("{v:.1}%")).unwrap_or_else(|| "n/a".into())) } small { "Annual, accrued daily" } }
                        div { dt { "Issuer AUM claim" } dd { (&aum) } small { "Filed 8 September 2026" } }
                        div { dt { "Mining status" } dd class="research-fact-text" { "Not a miner" } small { "Adds no hashpower" } }
                    }
                    section class="research-timeline" aria-labelledby="zcsh-timeline" {
                        p class="research-kicker" { "FILED TIMELINE" }
                        h2 id="zcsh-timeline" { "A trust became an exchange-traded product" }
                        ol {
                            li { time datetime="2017-10-23" { "23 Oct 2017" } div { strong { "Trust formed." } p { "The vehicle began as Grayscale Zcash Trust (ZEC)." } } }
                            li { time datetime="2021-10-18" { "18 Oct 2021" } div { strong { "OTCQX trading history." } p { "The prospectus records the start of public quotations before the later exchange listing." } } }
                            li { time datetime="2026-08-24" { "24 Aug 2026" } div { strong { "Legal name changed." } p { "The vehicle changed its name to The Zcash ETF." } } }
                            li { time datetime="2026-08-25" { "25 Aug 2026" } div { strong { "NYSE Arca trading began." } p { "Shares began trading under ticker ZCSH." } } }
                            li { time datetime="2026-09-30" { "30 Sep 2026" } div { strong { "Forward split." } p { "A 3-for-1 share split became effective before the market opened." } } }
                        }
                        p class="source-line" { (grayscale_source_link(d, "zcsh-8a", "Form 8-A")) " · " (grayscale_source_link(d, "zcsh-prospectus", "Prospectus")) " · " (grayscale_source_link(d, "zcsh-split", "Split filing")) }
                    }
                    section aria-labelledby="zcsh-holds" {
                        p class="research-kicker" { "WHAT THE PRODUCT HOLDS" }
                        h2 id="zcsh-holds" { "ZEC exposure through shares" }
                        p { "ZCSH holds ZEC and issues shares intended to reflect the value of those holdings after fees and expenses. The sponsor fee is 2.5% annually, accrued daily and payable in ZEC. The prospectus names Coinbase Custody as primary custodian; a later supplement added Anchorage Digital Bank as an additional custodian." }
                        p { "On 8 September the issuer said assets under management exceeded $500 million. That is a dated issuer statement filed as an exhibit, not a live figure. The same filing reports that Digital Currency Group exchanged 85,705.32563297 ZEC for approximately $100 million of ZCSH shares." }
                        p class="source-line" { (grayscale_source_link(d, "zcsh-aum-exhibit", "8 September issuer exhibit")) " · " (grayscale_source_link(d, "zcsh-custodian", "Custodian supplement")) }
                    }
                    section class="research-comparison" aria-labelledby="zcsh-mining" {
                        div {
                            p class="research-kicker" { "CONNECTION TO MINING" }
                            h2 id="zcsh-mining" { "Market structure, not hashpower" }
                            p { "Grayscale is not identified as a Zcash miner in these records. ZCSH holds ZEC; it does not operate Equihash machines or contribute work to the network. Its relevance is institutional access, custody and potential market demand." }
                        }
                        aside {
                            p class="formula mono" { "ZCSH holds ZEC ≠ ZCSH mines ZEC" }
                            p { "Shares are not direct ownership of the trust’s ZEC, and the vehicle is not registered as an investment company under the Investment Company Act of 1940." }
                            small { "Read the prospectus risk and structural disclosures before drawing conclusions from the ticker alone." }
                        }
                    }
                    section class="research-sources" aria-labelledby="zcsh-sources" {
                        p class="research-kicker" { "PRIMARY SOURCES" }
                        h2 id="zcsh-sources" { "Documents behind this briefing" }
                        div class="table-scroll" { table class="data" { thead { tr { th { "Document" } th { "Type" } } } tbody {
                            @for s in &r.sources { tr { td { (ext(&s.url, &s.label)) } td { (&s.source_type) } } }
                        } } }
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
        section class="ed-research-note industry-home" aria-labelledby="industry-home-title" {
            div class="wrap" {
                header class="industry-home-head" {
                    div {
                        p class="ed-section-index" { "INDUSTRY / COMPANIES AND CAPITAL" }
                        h2 id="industry-home-title" { "Equihash industry" }
                        p { "Mining fleets, company filings and investment products connected to Zcash." }
                    }
                    a class="ed-light-button" href=(INDUSTRY_PATH) { "View all Industry briefings" }
                }
                div class="industry-home-grid" {
                    a href=(CYPHERPUNK_PATH) {
                        span { "MINING FLEET · CYPHERPUNK" }
                        strong { "4.2 GSol/s and 4,902 Z15 Pro miners" }
                        small { "Share if the reported fleet remains online: " (share) }
                    }
                    a href=(GRAYSCALE_PATH) {
                        span { "FUND BRIEF · GRAYSCALE" }
                        strong { "ZCSH is trading on NYSE Arca" }
                        small { "Listing, fee, holdings and custody" }
                    }
                    a href=(WINK_PATH) {
                        span { "FILING BRIEF · WINKLEVOSS" }
                        strong { "WINK is filed, not launched" }
                        small { "What the preliminary Zcash ETF prospectus says" }
                    }
                }
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
            a href=(CYPHERPUNK_PATH) { "Read the SEC-sourced briefing →" }
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
            p { a href=(CYPHERPUNK_PATH) { "Read the fleet briefing" } " · " a href=(WINK_PATH) { "Read the WINK filing briefing" } }
        }
    }
}
