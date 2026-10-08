use crate::data::Data;
use crate::views::layout::{ext, layout, Page};
use maud::{html, Markup, PreEscaped};

const WP: &str = "https://w.cash/whitepaper";
const AUX: &str = "https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-zcash-aux/README.md";
const MM: &str = "https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/wcash-merge-miner/README.md";
const ZIP244: &str = "https://zips.z.cash/zip-0244";
/// Coinbase-script length rule as enforced by the pinned Zebra code in the Wcash source tree.
const CBLEN_SRC: &str = "https://github.com/w-cash/wolf/blob/3e6b8044eac789e6e6289772a80e996cb94eb43d/zebra-chain/src/transparent/serialize.rs#L66-L106";
/// The consensus rule itself, in the Zcash protocol specification.
const ZPROTO_TXN: &str = "https://zips.z.cash/protocol/protocol.pdf#txnconsensus";

const FLOW: &str = r#"  Wcash node                 pool                            ASIC
  ----------                 ----                            ----
  createauxblock  ------->   put child commitment in   --->  one Equihash (200,9)
  (child candidate)          the Zcash coinbase              search, no changes
                                                                  |
                       +------------------------------------------+
                       |                                          |
              meets the Zcash target?                    meets the Wcash target?
                       |                                          |
              submitblock  -> ZEC block                 submitauxblock -> WEC block"#;

const ARCH: &str = r#"  +---------------------------+
  | Wcash node                |  createauxblock, submitauxblock,
  | loopback only             |  getauxblockstatus, retireauxblock
  +-------------+-------------+
                |
  +-------------+-------------+      +--------------------+      +----------------+
  | Pool coordinator          | ---> | Stratum edge       | ---> | ASICs          |
  | builds jobs, checks       |      | TLS, auth, vardiff |      | Equihash 200,9 |
  | payouts and commitment,   |      | rate limits        |      | no changes     |
  | routes winners            |      +--------------------+      +----------------+
  +------+-------------+------+
         |             |
  +------+------+  +---+-------------------------+
  | Zcash       |  | independent Zcash           |
  | template    |  | validator(s): proposal-mode |
  | node        |  | check of the exact block    |
  | loopback    |  | separate failure domain     |
  +-------------+  +-----------------------------+"#;

const BYTES: &str = r#"  +--------+----------+----------+--------------------+-----------+-----------+
  | height | pool tag | fabe6d6d | reverse(aux root)  | tree size |   nonce   |
  | canon. | optional |   4 B    |       32 B         | 4 B u32le | 4 B u32le |
  +--------+----------+----------+--------------------+-----------+-----------+
                      |<------------- 44-byte commitment, last ------------->|"#;

fn cite(n: u8) -> Markup {
    html! { sup class="cite" { a href={"#ref-" (n)} { "[" (n) "]" } } }
}

pub fn render(d: &Data) -> Markup {
    let toc = [
        ("miners", "Start here: miners"),
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
        title: "Zcash + Wcash merged mining guide",
        description: "Technical Equihash merged-mining guide for pool operators using Zcash as the parent chain and Wcash as the AuxPoW example, with RPC flow and coinbase layout.",
        path: "/merged-mining",
        nav: "merged-mining",
    }, html! {
        div class="wrap page guide" {
            header class="page-head guide-head" {
                p class="kicker" { "For miners and pool operators" }
                h1 { "Merged mining on Equihash" }
                p class="lede" {
                    "A participating pool can reuse the work from an Equihash 200,9 miner for both Zcash and Wcash. "
                    "Miners keep the same ASIC and do not divide their Zcash hashrate; the pool adds the auxiliary-chain work and separate WEC accounting."
                }
                p class="disclosure" {
                    "The worked example is " (ext("https://w.cash", "Wcash")) " (WEC), an independent Zcash-derived chain merged-mined with Zcash as the "
                    em { "parent" } " through AuxPoW v2. Every Wcash detail below comes from its "
                    (ext(WP, "protocol specification")) (cite(1)) " and the pinned source READMEs it references" (cite(2)) (cite(3)) "; all of it was checked against those documents on 3 October 2026, and the coinbase-script limit against the pinned source code and the Zcash protocol specification" (cite(5)) (cite(6)) ". "
                    "The maintainer of this site also builds Wcash (see " a href="/about" { "About" } "). The flow is the general AuxPoW pattern; other aux chains define their own commitment details."
                }
            }
            section id="miners" class="merge-start" {
                div {
                    p class="eyebrow" { "MINER QUICK START" }
                    h2 { "Mine Zcash and Wcash with the same work" }
                    p { "Use a Zcash-compatible Equihash 200,9 machine, such as the Antminer Z15 Pro, and connect it to a pool that explicitly supports ZEC + WEC merged mining. The pool handles the extra chain connection and tells you how WEC payouts are configured." }
                    div class="merge-actions" {
                        a class="btn" href="/pools?coin=zcash&merged=1#pools" { "Find a supporting pool" }
                        a class="text-link" href="#architecture" { "Pool operator setup →" }
                    }
                }
                dl {
                    div { dt { "Hardware" } dd { "Equihash 200,9 ASIC" } }
                    div { dt { "Miner changes" } dd { "None when the pool supports it" } }
                    div { dt { "Security scope" } dd { "Only participating pool work secures Wcash" } }
                }
            }
            div class="guide-layout" {
                nav class="toc" aria-label="On this page" {
                    p { "Contents" }
                    ol { @for (id, t) in toc { li { a href={"#" (id)} { (t) } } } }
                }
                article class="prose" {
                    h2 id="overview" { "1. How merged mining works" }
                    p { "In merged mining a pool hashes one header but checks the result against two chains. The " strong { "parent chain" } " (here Zcash) is the chain whose block header the ASIC actually solves. The " strong { "auxiliary (child) chain" } " (here Wcash) accepts that same parent proof of work, as long as the parent block's coinbase commits to the child block the pool prepared." }
                    p { "Wcash accepts exactly one parent work profile: Zcash Equihash (200,9)" (cite(1)) ". The miner searches one Equihash space, and each chain evaluates the result against " em { "its own" } " target:" }
                    figure class="diagram" {
                        pre aria-label="Flow: the Wcash node hands the pool a child candidate; the pool commits it in the Zcash coinbase; the ASIC runs one Equihash search; a result that meets the Zcash target becomes a ZEC block, one that meets the Wcash target becomes a WEC block." { (PreEscaped(FLOW)) }
                        figcaption { "One search, two independent targets." }
                    }
                    p { "A single Equihash result may satisfy the Zcash target, the Wcash target, both, or neither" (cite(1)) ". The targets are independent: the parent header's " code { "nBits" } " is diagnostic only and cannot weaken the Wcash target, which Wcash derives from its own authenticated chain state" (cite(1)) (cite(2)) "." }

                    h2 id="costs" { "2. What changes (and what doesn't)" }
                    div class="cols" {
                        div {
                            h3 { "Stays the same" }
                            ul {
                                li { "ASIC hardware and firmware" }
                                li { "Stratum URL, worker names and difficulty handling, as miners see them" }
                                li { "The Zcash block: an ordinary, consensus-valid Zcash block with ordinary Zcash payouts" }
                                li { "Zcash hashrate and block-finding odds, because the same work is reused" }
                            }
                        }
                        div {
                            h3 { "The pool adds" }
                            ul {
                                li { "A Wcash node with private, authenticated RPC" }
                                li { "A Zcash template node that can place the 44-byte commitment in miner data" }
                                li { "At least one independent Zcash node to proposal-validate templates" }
                                li { "Dual submission: Zcash winners to Zcash, Wcash winners through " code { "submitauxblock" } }
                                li { "Separate accounting and payouts for the aux coin" }
                            }
                        }
                    }
                    p class="callout" { strong { "Important. " } "merged mining does not give the aux chain all of Zcash's hashrate. Only miners and pools that include the Wcash commitment contribute work to Wcash" (cite(1)) ". A valid AuxPoW also doesn't prove the parent block made it into Zcash's best chain" (cite(1)) "." }

                    h2 id="architecture" { "3. Pool architecture" }
                    p { "The reference coordinator uses three nodes" (cite(3)) ":" }
                    figure class="diagram" {
                        pre aria-label="Architecture: Wcash node, Zcash template node and independent Zcash validators talk to the pool coordinator over private RPC; the coordinator serves jobs through the stratum edge to unchanged Equihash 200,9 ASICs." { (PreEscaped(ARCH)) }
                        figcaption { "Reference coordinator layout" (cite(3)) "." }
                    }
                    p { "The two template sources choose reward recipients, so the reference implementation requires them to be literal loopback endpoints; a proposal validator may be remote over HTTPS but should be an independent process and failure domain" (cite(3)) ". Keep all node RPCs private and authenticated; the specification calls the mining RPCs operator interfaces, not public web APIs" (cite(1)) "." }

                    h2 id="rpc" { "4. RPC flow, step by step" }
                    ol class="steps" {
                        li {
                            h4 { "Request a child candidate" }
                            pre { "createauxblock(\"<Wcash payout address>\")" }
                            p { "Returns the proof-free block bytes, canonical child ID (" code { "hash" } "), predecessor, " code { "target" } ", compact " code { "bits" } ", " code { "height" } ", " code { "coinbasevalue" } ", " code { "chainid" } " and a per-candidate retirement capability (" code { "retiretoken" } ")" (cite(1)) (cite(3)) ". Decode the returned block and independently verify these fields before building a parent job" (cite(1)) "." }
                            p class="small" { "A transparent Wcash address selects the normal pool path; a Unified Address with the private receiver selects a private Ironwood coinbase. A coinbase must use exactly one of the two modes" (cite(1)) ". Transparent coinbase outputs mature after 100 blocks and, under the inherited spend policy, must be shielded before ordinary transparent settlement, so plan payouts around that" (cite(1)) ". The block subsidy changes every block (it ramps up until height 40,000), so build the payout from " code { "coinbasevalue" } " rather than a fixed figure" (cite(1)) ". The node caches candidates in a bounded 16-entry cache" (cite(3)) "." }
                        }
                        li {
                            h4 { "Compute the aux root and build the commitment" }
                            p { "The aux leaf is not the bare block ID. It is SHA-256d over " code { "\"Wcash/ZcashAuxPoW/leaf/v2\\0\" || 53414357 || raw-wcash-block-id" } ", where " code { "53414357" } " is chain ID " code { "0x57434153" } " little-endian" (cite(2)) ". The familiar AuxPoW marker, power-of-two tree size, nonce and deterministic Namecoin slot formula are kept" (cite(2)) ". Then form the 44-byte suffix (see " a href="#coinbase" { "layout" } ")." }
                        }
                        li {
                            h4 { "Get a parent template that carries the commitment" }
                            p { "Append the 44 bytes as the final bytes of the Zcash coinbase input's miner data. In the reference stack this happens inside the Zcash template node through a private " code { "getblocktemplate" } " extension (" code { "wcashaux" } "), which reserves the carrier's cost during transaction selection and recomputes the affected commitments" (cite(3)) ". Changing the coinbase authorizing data changes " code { "hashAuthDataRoot" } " and " code { "hashBlockCommitments" } ", so the header must be rebuilt from the modified coinbase" (cite(2)) (cite(3)) "." }
                            p class="small" { "That template node is the pool-controlled Zcash node built from the pinned Wcash source branch, not a stock Zcash release: " code { "wcashaux" } " comes from that build" (cite(3)) ". Both parent nodes (template and proposal validator) must also expose the read-only " code { "getblockstatus(hash)" } " RPC from the matching release; a parent that lacks it fails closed before any work is issued" (cite(3)) "." }
                        }
                        li {
                            h4 { "Validate before releasing work" }
                            p { "Verify the payout outputs and the commitment locally, then send the exact serialized parent block to at least one distinct Zcash node in proposal mode; release the job only when the validators agree on the tip and accept the proposal" (cite(3)) ". Template substitution is a named threat in the spec's threat model" (cite(1)) "." }
                        }
                        li {
                            h4 { "Distribute jobs and validate shares" }
                            p { "Miners receive ordinary Equihash 200,9 jobs. For full winner coverage, set " code { "WCASH_SHARE_TARGET=network" } " in the reference stack: each new job then advertises the easier of the two network targets, so every possible winner on either chain is submitted" (cite(3)) "." }
                        }
                        li {
                            h4 { "Submit winners independently, per chain" }
                            pre { "# meets Zcash target\nsubmitblock(<parent block hex>)               # Zcash node\n\n# meets Wcash target\nsubmitauxblock(\"<candidate hash>\", \"<AuxPoW v2 hex>\")   # Wcash node" }
                            p { code { "submitauxblock" } " attaches the witness, verifies the proof-independent child ID is unchanged, and submits through normal consensus and gossip" (cite(1)) ". A failure on one chain must never suppress the other" (cite(3)) "." }
                        }
                        li {
                            h4 { "Track status, retire stale candidates" }
                            pre { "getauxblockstatus(\"<candidate hash>\", \"<AuxPoW v2 hex>\")\nretireauxblock(\"<candidate hash>\", \"<retire token>\")" }
                            p { "Status distinguishes best-chain, side-chain, conflicting-witness, pending and unknown outcomes" (cite(1)) ". Retire unsolved or parent-only candidates once all share handlers for that job have finished, so tip churn can't fill the cache; never retire a candidate after a Wcash winner was found for it" (cite(3)) "." }
                        }
                    }

                    h2 id="coinbase" { "5. Coinbase commitment layout" }
                    p { "AuxPoW v2 appends one 44-byte suffix to the Zcash coinbase input's miner data (the bytes after its canonical height)" (cite(1)) (cite(2)) ":" }
                    figure class="diagram" {
                        pre aria-label="Zcash coinbase input script: height, optional pool tag, then the 44-byte commitment: 4-byte marker fabe6d6d, 32-byte reversed aux root, 4-byte tree size, 4-byte nonce" { (PreEscaped(BYTES)) }
                        figcaption { "Coinbase miner data, start to end. The commitment must be the last 44 bytes." }
                    }
                    pre { "fabe6d6d || reverse(aux-root) || tree-size:u32-le || nonce:u32-le\n4 bytes     32 bytes          4 bytes            4 bytes     = 44 bytes" }
                    ul {
                        li { "The " code { "fabe6d6d" } " marker must occur exactly once in the whole miner-data field" (cite(1)) (cite(2)) "." }
                        li { "The 44-byte suffix must end the miner-data field; pool identification bytes may precede it" (cite(1)) (cite(2)) "." }
                        li { "No " code { "OP_RETURN" } " output and no script-push wrapper; AuxPoW v1 (and its OP_RETURN carrier) is rejected" (cite(1)) (cite(2)) "." }
                        li { "Only canonical Zcash V5 and V6 coinbases are accepted" (cite(1)) "." }
                        li { "Budget: a Zcash coinbase transaction script must be 2 to 100 bytes long" (cite(6)) ", and the pinned Zebra code rejects a longer one while parsing the coinbase" (cite(5)) ". The height push, any pool tag and the 44-byte commitment must fit in those 100 bytes together, so shorten long pool tags." }
                        li { "Byte order: 32-byte IDs, digests and Merkle nodes use raw serialized order (the reverse of RPC/explorer display hex); only the aux root is reversed when placed in the carrier" (cite(2)) "." }
                    }
                    details class="vector" {
                        summary { "Test vector (from the pinned wcash-zcash-aux README)" (cite(2)) }
                        dl class="vec" {
                            div class="kv" { dt { "raw Wcash block ID" } dd { "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f" } }
                            div class="kv" { dt { "v2 leaf" } dd { "b38059dd1ca081e4a23cd86d4743aa48ce241675c5e89a1692976f4f509a8c82" } }
                            div class="kv" { dt { "nonce / index" } dd { "0x0d0c0b0a / 3" } }
                            div class="kv" { dt { "raw aux root" } dd { "8d5950951d66307c42153ca11df77463bb828206cc18a1711a24b0e346781b92" } }
                            div class="kv" { dt { "miner-data suffix" } dd { "fabe6d6d921b7846e3b0241a71a118cc068282bb6374f71da13c15427c30661d9550598d040000000a0b0c0d" } }
                        }
                        p class="small" { "The full 1,806-byte proof is published in the repository's test-vectors directory." }
                    }

                    h2 id="proof" { "6. The AuxPoW v2 proof" }
                    p { "Under " (ext(ZIP244, "ZIP-244")) ", the mined transaction ID does not bind the coinbase's authorizing data, so checking the transaction Merkle root alone would leave the miner-data commitment unauthenticated. Wcash validates two paths" (cite(1)) (cite(2)) ":" }
                    ol {
                        li { strong { "Transaction path: " } "coinbase transaction ID → SHA-256d branch at index 0 → parent transaction Merkle root." }
                        li { strong { "Authorization path: " } "coinbase authorizing-data digest → " code { "ZcashAuthDatHash" } " branch at index 0 → " code { "hashAuthDataRoot" } ", which with the supplied chain-history root reproduces the header's " code { "hashBlockCommitments" } "." }
                    }
                    p { "Both paths must have the same depth. The proof is encoded in this exact order" (cite(2)) ":" }
                    div class="table-scroll" {
                        table class="data" {
                            thead { tr { th { "Field" } th { "Size" } } }
                            tbody {
                                @for (f, s) in [("\"WCAZ\" magic", "4 bytes"), ("version (= 2)", "u8"), ("coinbase length", "CompactSize"), ("canonical Zcash coinbase", "coinbase length"), ("transaction Merkle branch length", "CompactSize"), ("transaction Merkle branch", "32 bytes per node"), ("coinbase transaction index (= 0)", "u32-le"), ("auth-data Merkle branch length", "CompactSize"), ("auth-data Merkle branch", "32 bytes per node"), ("coinbase auth-data index (= 0)", "u32-le"), ("parent chain-history root", "32 bytes"), ("auxiliary Merkle branch length", "CompactSize"), ("auxiliary Merkle branch", "32 bytes per node"), ("auxiliary index", "u32-le"), ("canonical Zcash Equihash (200,9) header", "1,487 bytes")] {
                                    tr { td { (f) } td class="mono" { (s) } }
                                }
                            }
                        }
                    }
                    p class="small" { "Decoding rejects unknown versions, non-minimal CompactSize, trailing bytes, oversized fields, non-zero coinbase indexes and mismatched branch depths. Proofs are capped at 256 KiB, coinbases at 128 KiB, parent paths at 32 levels and aux trees at 16 levels" (cite(2)) ". The normal Zcash coinbase-script consensus limit is considerably tighter, and Zebra enforces it when parsing the supplied transaction" (cite(2)) ": 2 to 100 bytes" (cite(5)) (cite(6)) "." }

                    h2 id="checklist" { "7. Operator checklist" }
                    p class="ck-progress" aria-live="polite" {
                        span class="ck-count" { "0 of 15 done" }
                        " "
                        button type="button" class="linkish ck-reset" { "untick all" }
                    }
                    ul class="checklist" {
                        @for item in [
                            "Run a Wcash node; keep its RPC (default port 48232) private and authenticated.",
                            "Pin genesis hashes for every node and refuse work if any node reports a different network.",
                            "Use an operator-owned transparent Wcash payout address for the normal pool path (or a Unified Address for a private Ironwood coinbase).",
                            "Reserve 44 bytes of coinbase miner data; keep the pool tag short enough that height, tag and commitment fit Zcash's 100-byte coinbase-script limit.",
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
                    p class="small" { "Ticks are saved in your browser only. Sources: " a href="#ref-1" { "[1]" } " " a href="#ref-2" { "[2]" } " " a href="#ref-3" { "[3]" } "." }

                    h2 id="pitfalls" { "8. Pitfalls & safety" }
                    dl class="pitfalls" {
                        div { dt { "You can't bolt this onto someone else's pool" } dd { "A downstream proxy can't add the commitment to an upstream pool's job: changing the coinbase authorizing data changes the auth-data root, block commitments and therefore the Equihash header, so the shares no longer match the upstream job" (cite(3)) ". The pool that builds the template must add it." } }
                        div { dt { "Two witnesses, one block ID" } dd { "Wcash block IDs are computed over the proof-independent header, so two valid AuxPoW witnesses give the same ID" (cite(1)) ". Confirm via witness-exact status, and treat a conflicting witness as a separate state" (cite(3)) "." } }
                        div { dt { "Target confusion" } dd { "Never derive the child target from the parent's nBits; use the target from " code { "createauxblock" } (cite(1)) (cite(2)) "." } }
                        div { dt { "Stale candidates & reorgs" } dd { "Candidates are bounded and time-limited; unknown, expired or invalid candidates fail closed on submission" (cite(1)) "." } }
                        div { dt { "No audit implied" } dd { "The specification states it does not claim an external security audit or automatic pool compatibility; wallet, pool and release practices need independent review" (cite(1)) "." } }
                    }

                    h2 id="params" { "9. Wcash reference parameters" }
                    div class="table-scroll" {
                        table class="data" {
                            tbody {
                                @for (k, v) in [
                                    ("Parent chain", "Zcash (Wcash is the auxiliary chain)"),
                                    ("Work profile", "Equihash (200,9), authenticated through AuxPoW v2"),
                                    ("Target block spacing", "75 s"),
                                    ("Mainnet genesis", "2026-09-19 12:00:00 UTC, zero-reward (no premine)"),
                                    ("Mainnet genesis ID", "5bae12c8662a577b04ce1591af1a137c128f0cb51018a5f1622d861d1bb6fc48"),
                                    ("Address prefixes", "W1… / W3… transparent (P2PKH / P2SH), wu1… unified"),
                                    ("Protocol allocation", "0%: no founders reward, funding stream, treasury or dev tax"),
                                    ("Coinbase maturity", "100 blocks (transparent coinbase outputs, which must be shielded before ordinary transparent settlement)"),
                                    ("Block subsidy", "Rises linearly every block until height 40,000, then declines block by block; use coinbasevalue from createauxblock"),
                                    ("Difficulty adjustment", "Zcash-derived damped 17-block retarget"),
                                    ("Default P2P / RPC ports", "48233 / 48232"),
                                    ("AuxPoW chain ID", "0x57434153"),
                                ] { tr { th { (k) } td { @if v.len() == 64 && v.chars().all(|c| c.is_ascii_hexdigit()) { code class="hex" { (v) } } @else { (v) } } } }
                            }
                        }
                    }
                    p class="small" { "All values from the Wcash protocol specification" (cite(1)) " and the AuxPoW README" (cite(2)) ". ZecWec is the only listed WEC pool (and the only known public one); see its row in the " a href="/pools?coin=wcash#pools" { "pool table" } "." }

                    h2 id="refs" { "References" }
                    ol class="refs" {
                        li id="ref-1" { (ext(WP, "Wcash Protocol Specification (SPEC/0.1, source 3e6b8044)")) ": w.cash/whitepaper" }
                        li id="ref-2" { (ext(AUX, "wcash-zcash-aux/README.md @ 3e6b8044")) ": AuxPoW v2 binary layout, test vector" }
                        li id="ref-3" { (ext(MM, "wcash-merge-miner/README.md @ 3e6b8044")) ": reference coordinator, trust boundaries, RPC usage" }
                        li id="ref-4" { (ext(ZIP244, "ZIP-244: Transaction Identifier Non-Malleability")) }
                        li id="ref-5" { (ext(CBLEN_SRC, "zebra-chain/src/transparent/serialize.rs @ 3e6b8044, lines 66–106")) ": " code { "parse_coinbase_height" } " quotes the rule and rejects coinbase scripts shorter than " code { "MIN_COINBASE_SCRIPT_LEN" } " or longer than " code { "MAX_COINBASE_SCRIPT_LEN" } }
                        li id="ref-6" { (ext(ZPROTO_TXN, "Zcash Protocol Specification (v2026.7.0), §7.1.2 Transaction Consensus Rules")) ": “A coinbase transaction script MUST have length in {2 .. 100} bytes.”" }
                    }
                }
            }
        }
    })
}
