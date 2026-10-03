# equihash.com

A directory of Equihash mining pools for people who run the machines. For every pool on every Equihash coin it lists:
- hashrate and share of the network
- fee and payout scheme
- minimum payout, recent blocks and region
- where each number came from

Any pool over 30% of a network is flagged. The site also has a hardware page, an earnings calculator, an archive of retired pools and ended coins, and a merged-mining guide for pool operators.

This is a proof of concept. It is kept simple on purpose:

- **Backend:** one Rust binary built on [actix-web 4](https://actix.rs). HTML is rendered on the server with [maud](https://maud.lambda.xyz), whose templates are checked at compile time.
- **Data:** plain JSON files in `data/`. The server loads them into typed serde structs and reloads them when they change.
- **Frontend:** hand-written CSS (`static/app.css`) and one vanilla JS file (`static/app.js`). There is no build step, bundler or framework. Every page works without JS. JS adds:
  - live filtering
  - a pool drawer (a bottom sheet on mobile)
  - the live calculator with inline validation

## Design (v2)

v2 is a full visual redesign. The research behind it (reference screenshots plus `DESIGN-NOTES.md`) lives in `research/` in the working copy and is not shipped.

The short version is a printed ledger kept by a miner:
- **Colours:** paper `#f7f5ef` and ink `#1d1c19`, hairline rules, one link blue, and red only for a pool above 30%. A warm dark mode follows `prefers-color-scheme`.
- **Restraint:** radii of 2px at most, no gradients, shadows or animation.
- **The pool table is the page:** dense, mono numbers, sorted by hashrate.
- **Prose** is plain and written for miners, starting with "How to pick a pool".

Fonts are self-hosted in `static/fonts/` (woff2, latin and latin-ext subsets, from Fontsource). All are SIL Open Font License 1.1, and the licence texts sit next to the files:

| Face | Used for |
|---|---|
| Source Serif 4 (400, 400 italic, 600) | headings and prose |
| IBM Plex Sans (400, 400 italic, 500, 600) | interface text |
| IBM Plex Mono (400, 500) | every number, hosts, code |

The icons (`static/favicon.svg`, `favicon.ico`, `apple-touch-icon.png`, `icon-512.png`) are a Source Serif "e" drawn as an SVG path, with a short red rule for the 30% line. `static/og.png` is rendered from `assets/og.html`. Neither contains live numbers, so they never go stale. To re-render after editing, open the HTML in Chrome at 1200×630 and screenshot it, or use the Playwright snippet in `scripts/screenshots.mjs` as a template.

```
src/
  main.rs          server, routes, env config, hot reload of data/
  data.rs          serde types + load-time derivations (slugs, regions, shares)
  fmt.rs           number/hashrate/date formatting (unknown → "n/a")
  views/           maud templates: layout, home, pages, guide, calc
static/            app.css, app.js, fonts/, icons, og.png (served as /static/*)
data/              pools.json, network.json, meta.json (generated)
                   archive.json, miners.json, research.json (hand-maintained)
data/curated/      manual-pools.json, coins.json, coin-status.json (hand-maintained)
scripts/           refresh-data.mjs (data refresh), screenshots.mjs (dev only)
assets/og.html     source of static/og.png
```

## Run

You need Rust 1.85 or newer.

```bash
cargo run --release
# → equihash.com serving on http://127.0.0.1:8080
```

| Env var      | Default     | Meaning                                  |
|--------------|-------------|------------------------------------------|
| `HOST`       | `127.0.0.1` | bind address (`0.0.0.0` inside a VM/LXC) |
| `PORT`       | `8080`      | port                                     |
| `DATA_DIR`   | `data`      | where the JSON lives                     |
| `STATIC_DIR` | `static`    | CSS/JS/icons                             |
| `RUST_LOG`   | `info`      | log level (actix request log)            |
| `LIVE`       | `1`         | `0` turns off the live-source poller     |

Routes:

- Pages:
  - `/` shows the pools. `?coin=<id>` picks a coin and `?coin=all` shows every coin. The page also has filters, "How to pick a pool", "What can a Z15 mine?" and all networks.
  - `/pool/{slug}`, `/archive` and `/miners` (the Hardware page).
  - `/calculator`, `/merged-mining`, `/add-pool`, `/about` and `/sources`.
- Raw data: `/data/{pools,network,miners,archive,meta,research}.json`.
- Live figures: `/api/live` (JSON; see "Live sources" below). The page polls it every 60 s.
- Also served: `/sitemap.xml`, `/robots.txt` and `/static/*`.

### Hot reload

The server polls `data/` and `data/curated/` every 2 s. It takes a fingerprint of every `*.json` file there: path, mtime, size and a hash of the contents.

Any difference triggers a full reload. That covers:
- an edit
- a replacement that has an older mtime (`cp -p`, a git checkout)
- a same-size edit within one timestamp tick
- a new or deleted file

If the reload fails (broken JSON, wrong shape), the server logs a warning and keeps serving the last good data. It retries on every poll until the files load.

Rows in `data/curated/manual-pools.json` are merged over the matching row in `pools.json` by `id`. A hand edit to a pool that the refresh script already wrote therefore shows up straight away. (In v1 these rows were skipped, which is why manual edits never seemed to hot-reload.)

The exception is fields a live endpoint refreshes. A row marked `"live"` takes `fee_pct`, `fee_source`, `fee_observed_at` and `data_url` from the refresh, and the scheme fees follow the refreshed fee. A row with `"live_fields": ["fee", "hashrate"]` also takes the hashrate, its `basis`, source, time and window from the refresh or the live poller. Nothing else comes from the refresh. In particular the row's `fetched_at` stays its `verified_at`, and a fee or height refresh never moves `hashrate_observed_at`. That was the v2 bug, where refreshing ZecWec's fee made its operator-reported hashrate look new.

`data/curated/links.json` and `data/curated/live-sources.json` hot-reload the same way. A broken file keeps the last good data.

## Refresh the data

```bash
npm run refresh          # same as: node scripts/refresh-data.mjs
```

The script needs Node 18 or newer and has no dependencies. It takes about 3 minutes because it waits between requests to be polite to the sources. It rewrites `data/pools.json`, `data/network.json` and `data/meta.json`, and the running server picks up the change on its own. If nothing can be fetched, it refuses to write an empty data set.

What it does, in order:

1. **miningpoolstats.** It reads the coin index (`coins_data.js`) and keeps every algorithm containing "Equihash", plus `horizen` and `flux`, which are tracked as ended coins. For each coin it fetches the pool JSON and price JSON from `data.miningpoolstats.stream`. These files need the page's own `last_time` token and a browser-like Referer. Without them the server returns 403.
2. **2Miners public APIs.** It cross-checks network hashrate, difficulty and height for ZEC and BTG, and reads the block reward credited to miners from the matured block lists.
3. **zpool and ZergPool public APIs.** These are optional extra pool sources. Rows that miningpoolstats already lists are skipped as duplicates.
4. **`data/curated/coins.json` and `manual-pools.json`.** It adds coins and pools that miningpoolstats does not cover. Rows marked `"live": "zecwec"` get their fees from the ZecWec public overview API.
5. **Wcash explorer API.** It reads WEC height, difficulty, observed block spacing and the current block reward.
6. **Live sources** (`data/curated/live-sources.json`). The same endpoints the server polls are written into the snapshot: the ZecWec pool hashrate (on both its ZEC and WEC rows) and the WEC network estimate. If an endpoint is unavailable, the previous good reading from the old `pools.json` or `network.json` is kept with its original `observed_at`, and a warning is logged.
7. **`data/curated/coin-status.json`.** It marks coins whose proof of work has ended.
8. **Ranking and shares.** Coins are ranked as described under "Data schema". Each pool's share is pool hashrate ÷ network hashrate, never above 100%. On some small coins the published network estimate is lower than the sum of what the pools report. Then the share is measured against the listed pools' total instead, and the UI says so.

Every value the source does not publish stays `null` and shows as **n/a**.

`node scripts/refresh-data.mjs --live-only` re-reads only the live sources into the existing `pools.json` and `network.json`. It takes about a second and leaves `generated_at` alone.

### Schedule it

Run it as often as you like. The site never promises a refresh interval. It shows the real age of the data instead: per coin, in the masthead, and as a stale notice once the newest pool figures are more than 2 hours old.

Hourly cron, next to the deployed binary:

```cron
7 * * * * cd /srv/equihash && /usr/bin/node scripts/refresh-data.mjs >> /var/log/equihash-refresh.log 2>&1
```

Live sources don't need cron; the server polls them itself.

Or use a GitHub Action that commits the refreshed JSON. Your deploy then pulls or rsyncs `data/`:

```yaml
# .github/workflows/refresh.yml
name: refresh-data
on:
  schedule: [{ cron: "7 * * * *" }]   # hourly; the site shows the real age either way
  workflow_dispatch:
permissions: { contents: write }
jobs:
  refresh:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: actions/setup-node@v4
        with: { node-version: 20 }
      - run: node scripts/refresh-data.mjs
      - run: |
          git config user.name "data-bot"
          git config user.email "data-bot@users.noreply.github.com"
          git add data/
          git diff --cached --quiet || git commit -m "data: refresh $(date -u +%FT%TZ)"
          git push
```

## Data schema

### Provenance per field

Every pool row (`pools.json`, `curated/manual-pools.json`) carries a source and an observation time for each figure, not just one for the row:

| Field | Source | Observed |
|---|---|---|
| hashrate | `hashrate_source` | `hashrate_observed_at` |
| fee | `fee_source` | `fee_observed_at` |
| miners | `miners_source` | `miners_observed_at` |
| blocks in last 1,000 | `blocks_source` | `blocks_observed_at` |
| minimum payout | `min_payout_source` | `min_payout_observed_at` |

The other row fields:
- `basis` says what kind of hashrate figure the row has:
  - `listed_pools`: published by the pool, read from miningpoolstats, zpool and similar;
  - `pool_api`: the pool's own public hashrate endpoint;
  - `operator_reported`: the operator told us (marked †);
  - `estimated`;
  - `null`: no figure.
- `hashrate_window_s` gives the averaging window when the source says (ZecWec: 1200).
- `live_source` names the `live-sources.json` entry that feeds the row.
- `fetched_at` is when the row was read (miningpoolstats) or checked by hand (`verified_at`). The page labels it "Row fetched" or "Row checked by hand". A field's own time moves only when that field was fetched.

Network rows (`network.json`, `coins[].network`) carry the same pattern:
- `hashrate_source` (the file the number was read from) and `hashrate_observed_at`;
- `hashrate_upstream`, miningpoolstats' own `hashrate_src` attribution, kept beside it because its figure can differ from that upstream's current one;
- `basis`, which is `network` or `null` (never the pools' total);
- `hashrate_sample_blocks` (ZecWec: 120) and `live_source`;
- `difficulty_source`/`difficulty_observed_at` and `height_source`/`height_observed_at`.

### n/a is not zero

`null` means not published and shows as **n/a**. `0` means published as zero and shows as **0 Sol/s**. The two are kept apart everywhere:
- In data, n/a sorts last in either direction.
- In the UI, a coin's listed-pool total is n/a when no row has a number, and 0 when the rows publish zero.
- The headline counts positive, zero and n/a rows separately.

### Network estimate vs. reported by listed pools

Each coin shows two separate figures:
- **Network estimate:** from the chain or a network source, or "unavailable".
- **Reported by listed pools:** the sum of positive pool hashrates.

The pools' total is never used as the network estimate. When no estimate exists, the total is shown with ≥, because it is only a floor. When any part of the total is operator-reported, it gets a †, and a footnote gives the number of pools and the date verified.

### Ranking and parameter sets

- Coins are grouped by exact Equihash (n,k): 200,9 first, then the others by (n,k), then "Parameters not published".
- Within a group, coins are ranked by the sum of positive pool-reported hashrate, descending. Zero and n/a come last, with name as the tie-break.
- The same order is used in the sidebar, the coin selector, "What can a Z15 mine?", All networks and the Hardware page.
- Hashrates on different parameter sets are never compared, summed, ranked or charted together. The all-coins pool table has one group per parameter set, and its ranks restart in each group.

### Shares

- A pool's share is pool ÷ network, capped at 100%.
- If the listed pools add up to more than the network estimate, shares are of the pools' total instead.
- If the only listed pool reads above the network estimate (ZecWec's 20-minute pool figure vs. the 120-block network estimate), the share cell says **only listed pool**. Both numbers and the reason go in the coin header, the tooltip and the drawer.
- Per-machine figures (Z15 coins/day, solo odds) are n/a when one machine would be 10% or more of the network estimate.

### Freshness

- The masthead shows a stale notice on every page when the newest miningpoolstats figures are more than 2 hours old (`STALE_AFTER_SECS` in `src/data.rs`).
- Each coin shows its own age, with a "stale" tag past 2 hours.
- The All networks table has an "As of" column.
- Live figures show their own age and are marked stale after `stale_after_seconds` (600 s).

### Headline

The home page lede is computed from the data. For the 2 Oct snapshot it reads: "117 tracked pool rows on 22 Equihash coins, 63 reporting positive hashrate (47 report zero, 7 publish none)." It counts rows on coins whose proof of work is still running.

## Live sources

`data/curated/live-sources.json` lists small public JSON endpoints. The server polls each one every `poll_seconds` (clamped to 30–60 s) in a background tokio task with `reqwest`, so browsers never call them. The refresh script writes the same readings into the static snapshot.

```json
{ "poll_seconds": 45, "stale_after_seconds": 600, "sources": [
  { "id": "zecwec-pool", "target": "pool", "pool_ids": ["zcash:zecwec.com", "wcash:zecwec.com"],
    "url": "https://pool.zecwec.com/api/v1/hashrate/pool", "label": "…", "unit": "Sol/s" },
  { "id": "zecwec-wcash-network", "target": "network", "coin_id": "wcash",
    "url": "https://pool.zecwec.com/api/v1/hashrate/network", "label": "…", "unit": "Sol/s" } ] }
```

How it behaves:
- A reading is used only when the endpoint returns `available: true` with a finite, non-negative hashrate and an `updated_at` (unix seconds) that isn't in the future.
- On `available: false`, an HTTP error or a timeout, the last good reading stays in use. Its age stays visible, the coin header says "endpoint down, showing the last good reading", and `/api/live` reports the status.
- A pool reading sets `basis: "pool_api"`, the source URL, `hashrate_observed_at = updated_at` and the window.
- `pool_ids` lists every row one endpoint feeds (`pool_id` still works for one row). ZecWec merge-mines ZEC and WEC with the same work, so its one pool figure feeds both rows. On every load the server gives all rows of such a source the newest reading from that endpoint (or n/a on all of them when none has one), so the rows can never disagree.
- A network reading sets `basis: "network"`, the source, the time and `hashrate_sample_blocks`. It fills the height only when no chain source supplied one, so Wcash's height and difficulty stay credited to the explorer and its hashrate to ZecWec's 120-block estimate.
- `fields` maps other field names, for an API that uses different keys. Adding another pool with a similar API is a JSON edit.
- The poller re-reads the config every round.
- `/api/live` returns each source's status and last good reading, plus the figures they feed, already formatted (`hashrate_text`, `share_text`, `network_text`, ages).
- `static/app.js` swaps those values into the coin header, the WEC row and the age line every 60 s. Without JS, the server-rendered page already has the latest reading.

## Social and community links

`data/curated/links.json` is written by a separate research step. This repo only reads it. Schema:

```json
{ "generated_at": "ISO", "coins": { "<coin id from network.json>": [ { "kind": "website|x|discord|telegram|github|reddit|forum|explorer|status|support|docs", "url": "https://…", "label": "…", "source_url": "…", "verified_at": "ISO", "status": "verified|unverified|dead", "note": "optional, never shown" } ] }, "pools": { "<pool id from pools.json>": [ same shape ] } }
```

Rendering rules (`verified_links` in `src/data.rs`, `src/views/links.rs`, the drawer in `static/app.js`):
- Only `status: "verified"` links are shown, with a known `kind` and an `http(s)` URL. Exact duplicates are dropped.
- Coin links sit under the coin title. Pool links sit under the pool title in the drawer and on the pool page. The two are never mixed, so a pool's links don't appear on its other coin's row.
- One link per kind is shown up front, and further links of the same kind sit behind "+N more". Kinds with no link aren't shown at all, so there are no empty icons.
- Links open with `target="_blank" rel="noopener noreferrer"`.
- Icons are a monochrome SVG sprite, `static/icons.svg`, with a text label next to each icon. X, Discord, Telegram, GitHub and Reddit are from Simple Icons (CC0 1.0). The other glyphs are drawn for this site.
- Unknown extra fields such as `note` are accepted and ignored.

**Manual override, keep it.** The Wcash X link (`coins.wcash`, kind `x`) is set by hand to `https://x.com/WcashProject`, with a `note` explaining it was chosen by the maintainer. If the links regeneration script (`/workspace/links-work/build.py`, outside this repo) is ever re-run, it must keep this entry and not revert it to a different account. Re-apply it after any regeneration. A unit test (`links_tolerate_a_note_field_and_the_wcash_x_override_loads`) fails if the loaded Wcash X link is anything else.

## Editing data by hand (no code changes)

All changes in this section only need a JSON edit. The server shows them within about 2 seconds, and the next refresh keeps them.

- **Add a pool that isn't on miningpoolstats.** Append an object to `data/curated/manual-pools.json`. It uses the same shape as a row in `pools.json`.
  - Required: `id`, `name`, `coin` (symbol), `url`, `source_url`, `verified_at`.
  - Set anything you can't source to `null`.
  - Set `"hashrate_is_reported": true` and `"basis": "operator_reported"` when the figure comes from the operator rather than a public API. The UI then marks it with †, with a footnote giving the date.
  - If the pool has a public hashrate endpoint, add it to `data/curated/live-sources.json` and set `"live_fields": ["hashrate"]` on the row.
- **Add a coin.** Append an object to `data/curated/coins.json` with the same shape as a coin in `network.json`: `id`, `name`, `symbol`, `equihash: {n, k}`, `hardware`, `network: {hashrate, unit, difficulty, height, block_time_target_s, …}`, `source_url` and `fetched_at`. Coins that miningpoolstats already lists appear automatically on the next refresh. Pools for the new coin go in `manual-pools.json` with `"coin": "<SYMBOL>"`.
- **A coin's proof of work ended.** Add `{id, status: "ended", note, sources: [{label, url}]}` to `data/curated/coin-status.json`.
- **A pool shut down.** Add an entry to `data/archive.json` with `reason`, `retired` and at least one `sources` link.
- **A new ASIC.** Add an entry to `data/miners.json` with the manufacturer's spec URL. Efficiency is calculated as watts ÷ kSol/s.
- **The research log** behind `/sources` lives in `data/research.json`.

Filters and normalisation follow the data, so a new value needs no code change:
- **Payout filter:** built from every scheme that appears in the data, canonicalised to upper case. PPLNT, PPLNSBF, SOLO and the rest appear as soon as a pool uses them.
- **Region filter:** built from the regions present. `region_bucket` in `src/data.rs` maps each country, code or region word to one of nine buckets (Global, North America, South America, Europe, Russia, Asia, Middle East, Oceania, Africa). It matches whole tokens, so "RU" is Russia and "Russia" is never read as "us" (v1 matched substrings). A token it doesn't know is logged at load, and a unit test fails if any value in `data/` maps to nothing.

For templates or styling, edit `src/views/*.rs` and `cargo run` again (maud will catch template mistakes at compile time). You can also edit `static/app.css` or `static/app.js` and just reload the browser.

## Calculator inputs

The server (`src/views/calc.rs`, the `FIELDS` table) and the browser (`static/app.js`, reading the `min`/`max` attributes the server writes) apply the same bounds. An out-of-range value gets an inline error and no estimate.

| Input | Allowed |
|---|---|
| Pool fee | 0–100 % |
| Hashrate | above 0, at most 10⁹ kSol/s |
| Power draw | 0 to 10⁷ W |
| Electricity | 0–10 $/kWh |
| Coin price | 0 to 10⁹ USD |
| Block reward | above 0, at most 10⁷ coins |
| Network hashrate | above 0, at most 10¹⁸ Sol/s |
| Block time | 1–86,400 s |

Negative numbers, NaN, infinity and text are all rejected.

## Tests

```bash
cargo test
```

```bash
node --test scripts/   # the refresh script's pure functions
```

`cargo test` runs 66 unit tests and `node --test scripts/` runs 14. Tests that need a fixed snapshot read `testdata/snapshot-2026-10-02/`, a frozen copy of `data/` from the 2 Oct refresh, so they don't change when the data does.

They cover:
- **Ranking:**
  - the 200,9 order on the snapshot is Zcash, Pirate Chain, Wcash, Kerrigan, Komodo, Buck, BitMark, then zero or n/a by name;
  - groups are never interleaved, and a huge hashrate on another parameter set never jumps ahead;
  - the same order holds in the sidebar, selector, Z15 list, networks table and Hardware page.
- **Timestamps:**
  - a ZecWec fee refresh never moves `hashrate_observed_at` or `fetched_at`;
  - loaded curated rows keep their verified time;
  - every row has per-field provenance and a valid `basis`;
  - `live_fields` keeps the refreshed hashrate.
- **n/a vs zero** in `reported_for`, `fmt::reported`, the pool row and the raw-to-loaded data.
- **Headline:** synthetic counts, and 117 rows with 63 positive on the snapshot.
- **Freshness:** the 2-hour boundary, unknown age counting as stale, and the stale notice appearing only past 2 hours.
- **Links:**
  - only verified, known-kind, http(s) links render, escaped and with `rel="noopener noreferrer"`;
  - duplicates fold behind "+N more";
  - coin and pool links stay separate;
  - a broken `links.json` keeps the last good data, and a missing one means no links;
  - hot-reload works, the `note` field is tolerated, and the Wcash X override loads.
- **Live sources:**
  - `available: false` (or missing) is not a reading, and nor are negative or future values;
  - the last good value survives unavailable answers and errors, and its age becomes stale;
  - an overlay sets basis, source and time, and never goes backwards;
  - shares are capped at 100%, and "only listed pool" shows both numbers;
  - Wcash shows the 120-block note with no † and no ≥;
  - `/api/live` carries the same figures;
  - there is no per-machine figure when one Z15 outweighs the network;
  - the config points at real rows, and ZecWec is the only listed WEC pool;
  - ZecWec's ZEC and WEC rows always show the same pool figure (files only, files that disagree, newer and older poller readings, no reading at all), in the server and in the refresh script.
- **Hot reload:** a manual override wins; a reload picks up an edit; an older mtime, a same-size same-tick edit and a new file are all detected; a broken file keeps the last good data and is retried.
- **Regions:** every value in `data/` maps to a bucket; RU, CA, IN and similar codes are handled; no substring false positives.
- **Payout schemes:** PPLNT and PPLNSBF are in the filter, and every filter option matches at least one pool.
- **Calculator:** fee above 100 % or below zero, negatives, zero hashrate, block-time bounds, NaN and infinity, text, and the boundary values themselves.

## Data sources

| What | Source |
|---|---|
| Coin list, pools, network stats | https://miningpoolstats.stream (JSON at https://data.miningpoolstats.stream/data/{coin}.js and `coins_data.js`) |
| Prices (coin header, network table, Hardware page, calculator) | https://data.miningpoolstats.stream/data/price/{coin}.js; when that fails, the `price` field of the coin file https://data.miningpoolstats.stream/data/{coin}.js (the price source URL says which) |
| ZEC / BTG cross-checks and block reward | https://zec.2miners.com/api/stats, `/api/blocks`; https://btg.2miners.com/api/stats, `/api/blocks` |
| zpool | https://zpool.ca/api/currencies, https://zpool.ca/api/status |
| ZergPool (optional) | https://zergpool.com/api/currencies |
| ZecWec (ZEC + WEC) | https://pool.zecwec.com/api/v1/overview (fees). Pool hashrate, on both the ZEC and WEC rows (the pool merge-mines both with the same work): https://pool.zecwec.com/api/v1/hashrate/pool (accepted work, previous 1200 s). WEC network estimate: https://pool.zecwec.com/api/v1/hashrate/network (previous 120 blocks). |
| unMineable (ZEC) | https://unmineable.com/support/article/how-to-setup-antminer-for-asic-mining |
| Wcash network | https://wcashexplorer.com/api/v1/status, `/api/v1/blocks?limit=1` |
| Merged-mining guide | https://w.cash/whitepaper; READMEs of `wcash-zcash-aux` and `wcash-merge-miner` pinned at commit 3e6b8044; ZIP-244 |
| ASIC specs | Bitmain support spec pages (support.bitmain.com), innosilicon.shop |
| Archive and ended coins | citations in `data/archive.json` and `data/curated/coin-status.json` |

### Known gaps

These show as n/a and are listed on `/sources`:

- **unMineable** publishes no ZEC fee, scheme or hashrate data.
- **Not verified:** Suprnova (API returned 503), Nanopool (ZEC API returned 404) and ZergPool (522 errors during research).
- **Prices** (as of the 3 Oct 2026 refresh; the Sources page recomputes this from the data each time):
  - BitMark (MARKS) shows n/a: its dedicated miningpoolstats price endpoint returned HTTP 404 and its main coin file has no fallback price.
  - Kerrigan (KRGN, both the 200,9 and 192,7 coins) shows a price, but it is the fallback price field from its main miningpoolstats coin file. Only its dedicated price endpoint failed (HTTP 404).
  - CDY, BUCK, HUSH, MCL, SQCN, TKL, VOLLAR and WEC publish no price, so they show n/a.
- **WEC network hashrate** comes only from ZecWec's 120-block estimate. The explorer doesn't publish one.
- **Block rewards** are sourced only for ZEC, BTG and WEC, so the calculator asks for the others.
- **Innosilicon A9 / A9+** are omitted because their manufacturer pages return 404.

## Deploy (notes only)

The site is one self-contained binary plus two folders:

```bash
cargo build --release
rsync -a target/release/equihash-site data static scripts package.json server:/srv/equihash/
# on the server
cd /srv/equihash && HOST=127.0.0.1 PORT=8080 ./equihash-site
```

- **Process manager:** run it under systemd (or similar) with `WorkingDirectory=/srv/equihash` and `Restart=always`.
- **Reverse proxy:** put Caddy or nginx in front for TLS and `equihash.com`. The app already sends compressed responses and basic security headers.
- **Scheduled refresh:** the refresh cron needs Node on the same host. Or run the GitHub Action and sync `data/` to the server.
- **Server needs:** no database and no runtime other than the binary. Node is only needed for refresh.

## Dev: screenshots

```bash
npm i --no-save playwright-core   # or point NODE_PATH at an existing install
BASE=http://127.0.0.1:8090 node scripts/screenshots.mjs   # uses system Chrome/Chromium; OUT=dir/ to write elsewhere
```

This writes `screenshots/`:
- **Desktop, 1440×900, light:** `home`, `home-full`, `home-all-coins`, `home-komodo`, `home-wcash`, `home-dark` (dark scheme), `pool-drawer`, `pool-page`, `hardware`, `calculator`, `calculator-wcash` (a Z15 Pro on WEC: n/a with the reason), `calculator-invalid` (shows the validation), `merged-mining-guide`, `archive`, `about`, `add-pool`, `sources`.
- **Mobile, 390×844 @2x:** `mobile-home`, `mobile-home-full`, `mobile-pools`, `mobile-drawer`, `mobile-hardware`, `mobile-calculator`, `mobile-guide`, `mobile-archive`, `mobile-about`.

`screenshots/before-after/` holds the v1 vs v2 comparisons.
