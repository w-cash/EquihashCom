use crate::data::Data;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup};

const WP: &str = "https://w.cash/whitepaper";
const AUX: &str = "https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-zcash-aux/README.md";
const MM: &str = "https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-merge-miner/README.md";
const ZIP244: &str = "https://zips.z.cash/zip-0244";

fn cite(n: u8) -> Markup {
    html! { sup class="cite" { a href={"#ref-" (n)} { "[" (n) "]" } } }
}

pub fn render(d: &Data) -> Markup {
    let toc = [
        ("overview", "How merged mining works"),
        ("costs", "What changes (and what doesn't)"),
        ("architecture", "Pool architecture"),
        ("rpc", "RPC flow, step by step"),
        ("coinbase", "Coinbase commitment layout"),
        ("proof", "The AuxPoW v2 proof"),
        ("checklist", "Operator checklist"),
        ("pitfalls", "Pitfalls & safety"),
        ("params", "Wcash reference parameters"),
        ("refs", "References"),
    ];
    layout(d, Page {
        title: "Merged mining guide for Equihash pools (Zcash parent, Wcash example)",
        description: "Technical, step-by-step guide for Equihash pool operators: add an aux chain with createauxblock/submitauxblock and a 44-byte coinbase commitment. No miner firmware change. Worked example: Wcash on Zcash.",
        path: "/merged-mining",
        nav: "merged-mining",
    }, html! {
        section class="wrap section guide" {
            div class="guide-hero" {
                p class="eyebrow" { "For pool operators" }
                h1 class="page-title" { "Merged mining on Equihash, " span class="grad" { "step by step" } }
                p class="lede" {
                    "How an Equihash 200,9 pool can mine an auxiliary chain on top of Zcash: run the aux node, call "
                    code { "createauxblock" } " / " code { "submitauxblock" } ", and append a 44-byte commitment to the end of the Zcash coinbase miner data. "
                    "Miners keep their ASICs, firmware and stratum settings exactly as they are."
                }
                div class="callout" {
                    strong { "Worked example: " } (ext("https://w.cash", "Wcash")) " (WEC), an independent Zcash-derived chain merged-mined with Zcash as the "
                    em { "parent" } " via AuxPoW v2. Every Wcash-specific detail below comes from its "
                    (ext(WP, "protocol specification")) (cite(1)) " and the pinned source READMEs it references" (cite(2)) (cite(3)) ". "
                    "Disclosure: this site's maintainer also builds Wcash (see " a href="/about" { "About" } "). The flow is the general AuxPoW pattern; other aux chains define their own commitment details."
                }
            }
            div class="guide-layout" {
                nav class="toc card" aria-label="On this page" {
                    p class="toc-title" { "On this page" }
                    ol { @for (id, t) in toc { li { a href={"#" (id)} { (t) } } } }
                }
                article class="prose" {
                    h2 id="overview" { "1. How merged mining works" }
                    p { "In merged mining a pool hashes one header but checks the result against two chains. The " strong { "parent chain" } " (here Zcash) is the chain whose block header the ASIC actually solves. The " strong { "auxiliary (child) chain" } " (here Wcash) accepts that same parent proof of work, as long as the parent block's coinbase commits to the child block the pool prepared." }
                    p { "Wcash accepts exactly one parent work profile: Zcash Equihash (200,9)" (cite(1)) ". The miner searches one Equihash space, and each chain evaluates the result against " em { "its own" } " target:" }
                    div class="flow" {
                        div class="flow-node" { span class="flow-n" { "01" } strong { "Aux node" } span { "Exact child candidate" } }
                        div class="flow-arrow" {}
                        div class="flow-node" { span class="flow-n" { "02" } strong { "Pool" } span { "Commits candidate in parent coinbase" } }
                        div class="flow-arrow" {}
                        div class="flow-node" { span class="flow-n" { "03" } strong { "ASIC" } span { "One Equihash (200,9) search" } }
                        div class="flow-split" {
                            div class="flow-node out" { strong { "Meets Zcash target?" } span { "→ ZEC block" } }
                            div class="flow-node out alt" { strong { "Meets Wcash target?" } span { "→ WEC block" } }
                        }
                    }
                    p { "A single Equihash result may satisfy the Zcash target, the Wcash target, both, or neither" (cite(1)) ". The targets are independent: the parent header's " code { "nBits" } " is diagnostic only and cannot weaken the Wcash target, which Wcash derives from its own authenticated chain state" (cite(1)) (cite(2)) "." }

                    h2 id="costs" { "2. What changes (and what doesn't)" }
                    div class="two-col" {
                        div class="card pad good" {
                            h4 { "Unchanged" }
                            ul class="ticks" {
                                li { "ASIC hardware and firmware (no firmware change)" }
                                li { "Stratum URL, worker names, difficulty handling as seen by miners" }
                                li { "The Zcash block: an ordinary, consensus-valid Zcash block with ordinary Zcash payouts" }
                                li { "Zcash hashrate and block-finding odds: the same work is reused" }
                            }
                        }
                        div class="card pad warnish" {
                            h4 { "Pool-side additions" }
                            ul class="ticks" {
                                li { "A Wcash node (private, authenticated RPC)" }
                                li { "A Zcash template node that can place the 44-byte commitment into miner data" }
                                li { "At least one independent Zcash node to proposal-validate templates" }
                                li { "Dual submission: Zcash winners to Zcash, Wcash winners via " code { "submitauxblock" } }
                                li { "Separate accounting and payouts for the aux coin" }
                            }
                        }
                    }
                    p class="note" { strong { "Important: " } "merged mining does not give the aux chain all of Zcash's hashrate. Only miners and pools that include the Wcash commitment contribute work to Wcash" (cite(1)) ". A valid AuxPoW also doesn't prove the parent block made it into Zcash's best chain" (cite(1)) "." }

                    h2 id="architecture" { "3. Pool architecture" }
                    p { "The reference coordinator uses three nodes" (cite(3)) ":" }
                    div class="arch" {
                        div class="arch-col" {
                            div class="arch-box child" { strong { "Wcash node" } span { "createauxblock · submitauxblock · getauxblockstatus · retireauxblock" } span class="arch-tag" { "loopback only" } }
                            div class="arch-box parent" { strong { "Zcash template node" } span { "getblocktemplate with commitment, submitblock" } span class="arch-tag" { "loopback only" } }
                            div class="arch-box validator" { strong { "Independent Zcash validator(s)" } span { "proposal-mode check of the exact block" } span class="arch-tag" { "separate failure domain" } }
                        }
                        div class="arch-mid" { div class="arch-box core" { strong { "Pool coordinator" } span { "builds jobs, verifies payouts & commitment, validates shares, routes winners" } } }
                        div class="arch-col" {
                            div class="arch-box asic" { strong { "Stratum edge" } span { "TLS, auth, vardiff, rate limits" } }
                            div class="arch-box asic" { strong { "ASICs" } span { "Equihash 200,9 (e.g. Antminer Z15)" } span class="arch-tag" { "no changes" } }
                        }
                    }
                    p { "The two template sources choose reward recipients, so the reference implementation requires them to be literal loopback endpoints; a proposal validator may be remote over HTTPS but should be an independent process and failure domain" (cite(3)) ". Keep all node RPCs private and authenticated; the specification calls the mining RPCs operator interfaces, not public web APIs" (cite(1)) "." }

                    h2 id="rpc" { "4. RPC flow, step by step" }
                    ol class="rpc-steps" {
                        li {
                            h4 { "Request a child candidate" }
                            pre class="code" { "createauxblock(\"<Wcash payout address>\")" }
                            p { "Returns the proof-free block bytes, canonical child ID (" code { "hash" } "), predecessor, " code { "target" } ", compact " code { "bits" } ", " code { "height" } ", " code { "coinbasevalue" } ", " code { "chainid" } " and a per-candidate retirement capability (" code { "retiretoken" } ")" (cite(1)) (cite(3)) ". Decode the returned block and independently verify these fields before building a parent job" (cite(1)) "." }
                            p class="muted sm" { "A transparent Wcash address selects the normal pool path; a Unified Address with the private receiver selects a private Ironwood coinbase. A coinbase must use exactly one of the two modes" (cite(1)) ". The node caches candidates in a bounded 16-entry cache" (cite(3)) "." }
                        }
                        li {
                            h4 { "Compute the aux root and build the commitment" }
                            p { "The aux leaf is not the bare block ID. It is SHA-256d over " code { "\"Wcash/ZcashAuxPoW/leaf/v2\\0\" || 53414357 || raw-wcash-block-id" } ", where " code { "53414357" } " is chain ID " code { "0x57434153" } " little-endian" (cite(2)) ". The familiar AuxPoW marker, power-of-two tree size, nonce and deterministic Namecoin slot formula are kept" (cite(2)) ". Then form the 44-byte suffix (see " a href="#coinbase" { "layout" } ")." }
                        }
                        li {
                            h4 { "Get a parent template that carries the commitment" }
                            p { "Append the 44 bytes as the final bytes of the Zcash coinbase input's miner data. In the reference stack this happens inside the Zcash template node through a private " code { "getblocktemplate" } " extension (" code { "wcashaux" } "), which reserves the carrier's cost during transaction selection and recomputes the affected commitments" (cite(3)) ". Changing the coinbase authorizing data changes " code { "hashAuthDataRoot" } " and " code { "hashBlockCommitments" } ", so the header must be rebuilt from the modified coinbase" (cite(2)) (cite(3)) "." }
                        }
                        li {
                            h4 { "Validate before releasing work" }
                            p { "Verify the payout outputs and the commitment locally, then send the exact serialized parent block to at least one distinct Zcash node in proposal mode; release the job only when the validators agree on the tip and accept the proposal" (cite(3)) ". Template substitution is a named threat in the spec's threat model" (cite(1)) "." }
                        }
                        li {
                            h4 { "Distribute jobs and validate shares" }
                            p { "Miners receive ordinary Equihash 200,9 jobs. For full winner coverage the reference listener advertises the easier of the two network targets so every possible winner on either chain is submitted" (cite(3)) "." }
                        }
                        li {
                            h4 { "Submit winners independently, per chain" }
                            pre class="code" { "# meets Zcash target\nsubmitblock(<parent block hex>)               # Zcash node\n\n# meets Wcash target\nsubmitauxblock(\"<candidate hash>\", \"<AuxPoW v2 hex>\")   # Wcash node" }
                            p { code { "submitauxblock" } " attaches the witness, verifies the proof-independent child ID is unchanged, and submits through normal consensus and gossip" (cite(1)) ". A failure on one chain must never suppress the other" (cite(3)) "." }
                        }
                        li {
                            h4 { "Track status, retire stale candidates" }
                            pre class="code" { "getauxblockstatus(\"<candidate hash>\", \"<AuxPoW v2 hex>\")\nretireauxblock(\"<candidate hash>\", \"<retire token>\")" }
                            p { "Status distinguishes best-chain, side-chain, conflicting-witness, pending and unknown outcomes" (cite(1)) ". Retire unsolved or parent-only candidates once all share handlers for that job have finished, so tip churn can't fill the cache; never retire a candidate after a Wcash winner was found for it" (cite(3)) "." }
                        }
                    }

                    h2 id="coinbase" { "5. Coinbase commitment layout" }
                    p { "AuxPoW v2 appends one 44-byte suffix to the Zcash coinbase input's miner data (the bytes after its canonical height)" (cite(1)) (cite(2)) ":" }
                    div class="bytes" role="img" aria-label="Zcash coinbase input script: height, optional pool tag, then the 44-byte commitment: 4-byte marker fabe6d6d, 32-byte reversed aux root, 4-byte tree size, 4-byte nonce" {
                        div class="bytes-row" {
                            div class="byte-seg seg-h" style="flex:1.2" { strong { "height" } span { "canonical" } }
                            div class="byte-seg seg-tag" style="flex:1.6" { strong { "pool tag" } span { "optional" } }
                            div class="byte-seg seg-mm" style="flex:5.5" {
                                div class="mm-inner" {
                                    div class="mm-part" style="flex:1" { strong { "fabe6d6d" } span { "4 B marker" } }
                                    div class="mm-part root" style="flex:4" { strong { "reverse(aux-root)" } span { "32 B" } }
                                    div class="mm-part" style="flex:1" title="Merkle tree size" { strong { "size" } span { "4 B u32-le" } }
                                    div class="mm-part" style="flex:1" { strong { "nonce" } span { "4 B u32-le" } }
                                }
                                div class="mm-label" { "44-byte merged-mining commitment · must be the final miner data" }
                            }
                        }
                        div class="bytes-axis" { span { "start of coinbase miner data" } span { "end →" } }
                    }
                    pre class="code" { "fabe6d6d || reverse(aux-root) || tree-size:u32-le || nonce:u32-le\n4 bytes     32 bytes          4 bytes            4 bytes     = 44 bytes" }
                    ul class="ticks" {
                        li { "The " code { "fabe6d6d" } " marker must occur exactly once in the whole miner-data field" (cite(1)) (cite(2)) "." }
                        li { "The 44-byte suffix must end the miner-data field; pool identification bytes may precede it" (cite(1)) (cite(2)) "." }
                        li { "No " code { "OP_RETURN" } " output and no script-push wrapper; AuxPoW v1 (and its OP_RETURN carrier) is rejected" (cite(1)) (cite(2)) "." }
                        li { "Only canonical Zcash V5 and V6 coinbases are accepted" (cite(1)) "." }
                        li { "Budget: Zcash keeps Bitcoin's 100-byte coinbase-script limit, so height + pool tag + 44 bytes must fit. Shorten long pool tags." }
                        li { "Byte order: 32-byte IDs, digests and Merkle nodes use raw serialized order (the reverse of RPC/explorer display hex); only the aux root is reversed when placed in the carrier" (cite(2)) "." }
                    }
                    details class="card pad vector" {
                        summary { "Test vector (from the pinned wcash-zcash-aux README)" (cite(2)) }
                        dl class="kv-grid mono sm" {
                            div class="kv" { dt { "raw Wcash block ID" } dd { "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f" } }
                            div class="kv" { dt { "v2 leaf" } dd { "b38059dd1ca081e4a23cd86d4743aa48ce241675c5e89a1692976f4f509a8c82" } }
                            div class="kv" { dt { "nonce / index" } dd { "0x0d0c0b0a / 3" } }
                            div class="kv" { dt { "raw aux root" } dd { "8d5950951d66307c42153ca11df77463bb828206cc18a1711a24b0e346781b92" } }
                            div class="kv" { dt { "miner-data suffix" } dd { "fabe6d6d921b7846e3b0241a71a118cc068282bb6374f71da13c15427c30661d9550598d040000000a0b0c0d" } }
                        }
                        p class="sm muted" { "The full 1,806-byte proof is published in the repository's test-vectors directory." }
                    }

                    h2 id="proof" { "6. The AuxPoW v2 proof" }
                    p { "Under " (ext(ZIP244, "ZIP-244")) ", the mined transaction ID does not bind the coinbase's authorizing data, so checking the transaction Merkle root alone would leave the miner-data commitment unauthenticated. Wcash validates two paths" (cite(1)) (cite(2)) ":" }
                    ol {
                        li { strong { "Transaction path: " } "coinbase transaction ID → SHA-256d branch at index 0 → parent transaction Merkle root." }
                        li { strong { "Authorization path: " } "coinbase authorizing-data digest → " code { "ZcashAuthDatHash" } " branch at index 0 → " code { "hashAuthDataRoot" } ", which with the supplied chain-history root reproduces the header's " code { "hashBlockCommitments" } "." }
                    }
                    p { "Both paths must have the same depth. The proof is encoded in this exact order" (cite(2)) ":" }
                    div class="table-wrap" {
                        table class="net-table" {
                            thead { tr { th { "Field" } th { "Size" } } }
                            tbody {
                                @for (f, s) in [("\"WCAZ\" magic", "4 bytes"), ("version (= 2)", "u8"), ("coinbase length", "CompactSize"), ("canonical Zcash coinbase", "coinbase length"), ("transaction Merkle branch length", "CompactSize"), ("transaction Merkle branch", "32 bytes per node"), ("coinbase transaction index (= 0)", "u32-le"), ("auth-data Merkle branch length", "CompactSize"), ("auth-data Merkle branch", "32 bytes per node"), ("coinbase auth-data index (= 0)", "u32-le"), ("parent chain-history root", "32 bytes"), ("auxiliary Merkle branch length", "CompactSize"), ("auxiliary Merkle branch", "32 bytes per node"), ("auxiliary index", "u32-le"), ("canonical Zcash Equihash (200,9) header", "1,487 bytes")] {
                                    tr { td { (f) } td class="mono" { (s) } }
                                }
                            }
                        }
                    }
                    p class="muted sm" { "Decoding rejects unknown versions, non-minimal CompactSize, trailing bytes, oversized fields, non-zero coinbase indexes and mismatched branch depths. Proofs are capped at 256 KiB, coinbases at 128 KiB, parent paths at 32 levels and aux trees at 16 levels" (cite(2)) "." }

                    h2 id="checklist" { "7. Operator checklist" }
                    div class="ck-progress" aria-live="polite" {
                        span class="ck-track" { span class="ck-fill" {} }
                        span class="ck-count mono" { "0 done" }
                        button type="button" class="chip-btn ck-reset" { "Reset" }
                    }
                    ul class="checklist" {
                        @for item in [
                            "Run a Wcash node; keep its RPC (default port 48232) private and authenticated.",
                            "Pin genesis hashes for every node and refuse work if any node reports a different network.",
                            "Use an operator-owned transparent Wcash payout address for the normal pool path (or a Unified Address for a private Ironwood coinbase).",
                            "Reserve 44 bytes of coinbase miner data; keep the pool tag short enough to fit Zcash's coinbase-script limit.",
                            "Build the commitment: v2 leaf → aux root (power-of-two tree, Namecoin slot) → fabe6d6d || reverse(root) || size || nonce.",
                            "Rebuild the parent coinbase, auth-data root and hashBlockCommitments from the modified coinbase.",
                            "Verify Zcash and Wcash payout outputs and the exact commitment locally before releasing a job.",
                            "Proposal-validate the exact parent block on at least one independent Zcash node.",
                            "Advertise share targets so every winner on either chain is captured.",
                            "Persist winners durably before submitting; submit Zcash and Wcash winners independently and retry idempotently.",
                            "Track Wcash winners with getauxblockstatus (witness-exact), not by block ID alone.",
                            "Retire stale candidates with retireauxblock after all handlers finish; never after a Wcash winner.",
                            "Mature rewards: transparent Wcash coinbase outputs mature after 100 blocks.",
                            "Account and pay ZEC and WEC separately; publish both fee policies.",
                            "Verify against the published test vector before going live.",
                        ] {
                            li { label { input type="checkbox"; span { (item) } } }
                        }
                    }
                    p class="fine" { "Ticks are saved in your browser only. Sources: " a href="#ref-1" { "[1]" } " " a href="#ref-2" { "[2]" } " " a href="#ref-3" { "[3]" } "." }

                    h2 id="pitfalls" { "8. Pitfalls & safety" }
                    div class="pitfalls" {
                        div class="card pad" { h4 { "You can't bolt this onto someone else's pool" } p { "A downstream proxy can't add the commitment to an upstream pool's job: changing the coinbase authorizing data changes the auth-data root, block commitments and therefore the Equihash header, so the shares no longer match the upstream job" (cite(3)) ". The pool that builds the template must add it." } }
                        div class="card pad" { h4 { "Two witnesses, one block ID" } p { "Wcash block IDs are computed over the proof-independent header, so two valid AuxPoW witnesses give the same ID" (cite(1)) ". Confirm via witness-exact status, and treat a conflicting witness as a separate state" (cite(3)) "." } }
                        div class="card pad" { h4 { "Target confusion" } p { "Never derive the child target from the parent's nBits; use the target from " code { "createauxblock" } (cite(1)) (cite(2)) "." } }
                        div class="card pad" { h4 { "Stale candidates & reorgs" } p { "Candidates are bounded and time-limited; unknown, expired or invalid candidates fail closed on submission" (cite(1)) "." } }
                        div class="card pad" { h4 { "No audit implied" } p { "The specification states it does not claim an external security audit or automatic pool compatibility; wallet, pool and release practices need independent review" (cite(1)) "." } }
                    }

                    h2 id="params" { "9. Wcash reference parameters" }
                    div class="table-wrap" {
                        table class="net-table" {
                            tbody {
                                @for (k, v) in [
                                    ("Parent chain", "Zcash (Wcash is the auxiliary chain)"),
                                    ("Work profile", "Equihash (200,9), authenticated through AuxPoW v2"),
                                    ("Target block spacing", "75 s"),
                                    ("Mainnet genesis", "2026-09-19 12:00:00 UTC, zero-reward (no premine)"),
                                    ("Protocol allocation", "0%: no founders reward, funding stream, treasury or dev tax"),
                                    ("Coinbase maturity", "100 blocks (transparent coinbase outputs)"),
                                    ("Difficulty adjustment", "Zcash-derived damped 17-block retarget"),
                                    ("Default P2P / RPC ports", "48233 / 48232"),
                                    ("AuxPoW chain ID", "0x57434153"),
                                ] { tr { th { (k) } td { (v) } } }
                            }
                        }
                    }
                    p class="fine" { "All values from the Wcash protocol specification" (cite(1)) " and the AuxPoW README" (cite(2)) ". The only pool currently mining WEC is ZecWec; see its row in the " a href="/?coin=wcash#pools" { "pool table" } "." }

                    h2 id="refs" { "References" }
                    ol class="refs" {
                        li id="ref-1" { (ext(WP, "Wcash Protocol Specification (SPEC/0.1, source 3e6b8044)")) ": w.cash/whitepaper" }
                        li id="ref-2" { (ext(AUX, "wcash-zcash-aux/README.md @ 3e6b8044")) ": AuxPoW v2 binary layout, test vector" }
                        li id="ref-3" { (ext(MM, "wcash-merge-miner/README.md @ 3e6b8044")) ": reference coordinator, trust boundaries, RPC usage" }
                        li id="ref-4" { (ext(ZIP244, "ZIP-244: Transaction Identifier Non-Malleability")) }
                    }
                }
            }
        }
    })
}
