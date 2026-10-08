# equihash.com

A source-backed directory for the Equihash mining ecosystem. It connects exact parameter sets to coins, pools, compatible hardware, sellers and the Equihash hashpower market, then provides calculators and guides for people who run the machines. For every pool it lists hashrate and share, fee and payout scheme, minimum payout, recent blocks, region and the source of each number.

Any pool over 30% of a network is flagged. The homepage is a discovery hub; dedicated coin, pool, hardware and vendor pages provide the detail; and the site also has search, an earnings calculator, an archive, contribution templates and a miner-first merged-mining guide.

The production architecture is kept simple on purpose:

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
  views/           maud templates: hub/discovery, pool directory, entity pages, guides, calculator
static/            app.css, app.js, fonts/, icons, og.png (served as /static/*)
                   logos/coins/, logos/pools/ (local logo copies, see "Logos")
data/              current.json (manifest of the live snapshot), snapshots/<id>/ with
                   pools.json, network.json, meta.json (generated, one directory per refresh)
                   archive.json, miners.json, vendors.json, listings.json, hashpower.json,
                   research.json (hand-maintained or separately refreshed)
data/curated/      manual-pools.json, coins.json, coin-status.json (hand-maintained),
                   permalinks.json (pool URLs; new ids added by the refresh, never changed),
                   logos.json (logo files and sources, written by fetch-logos.mjs)
scripts/           refresh-data.mjs (pool/network refresh), refresh-hashpower.mjs (NiceHash),
                   refresh-vendor-listings.mjs (approved seller pages, daily),
                   snapshot.mjs (atomic publish, rollback),
                   fetch-logos.mjs (logos, run by hand), screenshots.mjs (dev only)
deploy/            systemd units, Caddy and nginx examples, check-health.sh
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
| `HEALTH_MAX_DATA_AGE_SECS` | `10800` | `/healthz` answers 503 when the data is older |
| `HSTS`       | unset       | `1` sends `Strict-Transport-Security` (only behind TLS; usually the proxy does it) |

Routes:

- Pages:
  - `/` is the discovery homepage and `/search?q=...` searches coins, pools and hardware.
  - `/coins` and `/coin/{id}` provide coin records grouped by exact Equihash parameters.
  - `/pools?coin=<id>` is the full pool comparison. Old `/?coin=<id>` links redirect there.
  - `/hashpower` separates NiceHash selling/buying from direct pool mining and includes an Equihash order-cost estimator.
  - `/pool/{slug}`, `/hardware`, `/hardware/{id}`, `/guides` and `/merged-mining` provide the main research paths.
  - `/calculator`, `/contribute`, `/archive`, `/about` and `/sources` provide tools, submissions and methodology. `/miners` and `/add-pool` remain available for older links.
- Raw data: `/data/{pools,network,miners,vendors,listings,hashpower,archive,meta,research,current}.json`. The generated pool/network files come from the same snapshot the pages were rendered from.
- Live figures: `/api/live` (JSON; see "API" below). The page polls it every 60 s.
- Health: `/healthz` (JSON; see "Monitoring" under Deploy).
- Also served: `/sitemap.xml`, `/robots.txt` and `/static/*`.
- Every route answers `HEAD` as well as `GET` (same status and headers, no body).
- `/pool/{slug}` uses the pool's permanent slug from `data/curated/permalinks.json`. Old URLs (the slug once computed from coin, name and domain, or an entry in its `aliases`) answer `301` to the permanent one.

### Hot reload

The server polls `data/` and `data/curated/` every 2 s. It takes a fingerprint of every `*.json` file there: path, mtime, size and a hash of the contents. A refresh changes `data/current.json`, so a new snapshot is picked up the same way.

The generated files are read only through `data/current.json`: the snapshot it names, with each file's size and SHA-256 checked against the manifest before it is parsed. The server therefore always serves one complete refresh, never a mix of two. Without `data/current.json` (the layout before snapshots) it reads `data/pools.json` and friends directly.

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
npm run refresh:hashpower # aggregate NiceHash EQUIHASH order book → data/hashpower.json
npm run refresh:listings  # validate approved product variants → data/listings.json
```

The script needs Node 18 or newer and has no dependencies. It takes about 3 minutes because it waits between requests to be polite to the sources. The running server picks up the result on its own.

**Atomic publish** (`scripts/snapshot.mjs`):
1. The new `pools.json`, `network.json` and `meta.json` are written into `data/snapshots/.staging-<id>/` and fsynced.
2. What is on disk is validated:
   - schema: ids present and unique, every pool on a known coin, numbers are numbers, counts match `meta.json`;
   - every upstream URL is http(s);
   - no critical source failed (a miningpoolstats coin page; prices, 2Miners, zergpool and live sources are not critical);
   - nothing dropped sharply against the current snapshot: pool rows below 80%, rows with positive hashrate below 60%, coins below 90%, or a coin losing all of its pool rows.
3. If anything fails, nothing is published: the staging directory is removed, the previous snapshot stays live, the problems are printed and the script exits with code 2. `--force` publishes anyway and records the problems in the manifest.
4. Otherwise the directory is renamed to `data/snapshots/<id>/` (the id is the UTC time, e.g. `20261003T141150Z`), and `data/current.json` is replaced by writing a temp file, fsyncing it and renaming it over the old one.
5. The newest 10 snapshots are kept, plus the current one and the one before it.

The first run on the old flat layout imports the existing `data/pools.json`, `network.json` and `meta.json` as snapshot `<time>-imported` (so you can roll back to it) and removes the flat copies. One refresh runs at a time (`data/.refresh.lock`).

```bash
node scripts/refresh-data.mjs --list            # snapshots, newest last, with the current one marked
node scripts/refresh-data.mjs --rollback        # point current.json at the snapshot before the current one
node scripts/refresh-data.mjs --rollback <id>   # or at a given one
DATA_DIR=/srv/equihash/data node scripts/refresh-data.mjs   # data somewhere else
```

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

`node scripts/refresh-data.mjs --live-only` re-reads only the live sources into a copy of the current snapshot and publishes it the same way. It takes about a second and leaves `generated_at` alone.

The refresh also gives every pool id it hasn't seen before a permanent slug in `data/curated/permalinks.json` (from coin, name and domain, made unique). Existing entries are never changed or removed, so a pool keeps its URL when its name or domain changes, and a retired pool's slug is never reused.

### Schedule it

Run it as often as you like. The site never promises a refresh interval. It shows the real age of the data instead: per coin, in the masthead, and as a stale notice once the newest pool figures are more than 2 hours old.

With systemd, use `deploy/equihash-refresh.service` and `deploy/equihash-refresh.timer` (see "Deploy"). Or an hourly cron, next to the deployed binary:

```cron
7 * * * * cd /srv/equihash && /usr/bin/node scripts/refresh-data.mjs >> /var/log/equihash-refresh.log 2>&1
```

Live sources don't need cron; the server polls them itself.

Vendor prices and batch wording have a separate daily job. `data/curated/vendor-listing-sources.json`
is the whitelist: every row names an exact product variant, currency, parser and plausible price
range. `scripts/refresh-vendor-listings.mjs` updates only price, availability wording and observation
time after those checks pass. A blocked page, changed markup, ambiguous variant or out-of-range price
keeps the previous good record. Structured `InStock` markup never overrides a configured future batch.

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
- If the only listed pool reads above the network estimate (ZecWec's 20-minute pool figure vs. the 120-block network estimate), or there is no network estimate at all, the share cell says **only listed pool**. Both numbers and the reason go in the coin header, the tooltip and the drawer.
- Each pool carries a `share_status` (`network`, `only_listed_pool`, `pools_exceed_network`, `no_network_estimate` or `unavailable`); `/api/live` exposes it (see "API").
- On an open page, when a live reading moves a coin across its network estimate, the share cells, the small-screen share, the split bar and its explanation, the Share column tooltip, the concentration alert, the pool page's share row and the drawer all switch mode without a reload. The server sends the markup its own templates render (`share_cell_html`, `split_html`, …), and `static/shared.js` `applyShareModes` swaps it in.
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
- `static/app.js` swaps those values into the coin header, the pool rows, the share column, the split bar and the age line every 60 s, and reorders the ranked lists. Without JS, the server-rendered page already has the latest reading.

### API

`GET /api/live` (JSON, `Cache-Control: max-age=15`). Read-only, computed from the server's own last good readings; it never calls an upstream on request.

```jsonc
{
  "now": "2026-10-03T14:15:38Z",
  "sources": [ { "id": "zecwec-pool", "target": "pool", "status": "ok|unavailable|error|pending",
                 "reading": { "hashrate": 553587, "observed_at": "…", "window_seconds": 1200, … },
                 "age_secs": 40, "stale": false, "poll_seconds": 45, … } ],
  "pools": {                       // every pool on a coin with a live figure
    "wcash:zecwec.com": {
      "coin_id": "wcash", "live": true,
      "hashrate": 553587, "hashrate_text": "554 kSol/s", "observed_at": "…", "age_text": "…",
      "share_pct": null,           // share OF THE NETWORK, or null when it isn't one
      "share_status": "only_listed_pool", // network | only_listed_pool | pools_exceed_network | no_network_estimate | unavailable
      "share_basis": "listed_pools",      // network | listed_pools | null
      "listed_share_pct": null,    // share of the listed pools' total, when that is what the page shows
      "share_denominator": { "hashrate": 553587, "unit": "Sol/s", "basis": "listed_pools",
                             "source": "sum of the hashrates the listed pools report", "pools": 1, "observed_at": "…" },
      "share_text": "only listed pool", "share_note": "…", "share_flag": false, "share_sort": 100,
      "share_cell_html": "…", "share_kv_label": "…", "share_kv_html": "…"   // server-rendered markup
    }
  },
  "coins": { "wcash": { "network_hashrate": 511000, "network_text": "511 kSol/s", "network_observed_at": "…",
                        "sample_blocks": 120, "reported_text": "…", "share_basis": "listed_pools",
                        "share_denominator": { … }, "share_th_title": "…", "split_html": "…", "concentration_html": "…" } },
  "concentration_all_html": "…",
  "ranking": [ { "id": "zcash", "name": "Zcash", "group": "Equihash 200,9", "reported_text": "…", … } ]
}
```

Rules:
- `share_pct` is a share of the network estimate or `null`, never a 100% with an unstated denominator. With `share_status: "network"`, `share_denominator` is the network estimate with its source URL and time.
- `only_listed_pool`: one pool reports hashrate and it reads above the network estimate (or there is none). `share_pct` and `listed_share_pct` are null; the denominator is the listed total.
- `pools_exceed_network`: the listed pools together read above the network estimate. `listed_share_pct` is each pool's share of their total. (When the pools read at most 2% above the estimate, shares stay against the network and are capped at 100%; a capped pool gets this status with the network as its basis, and `share_pct` is still null.)
- `no_network_estimate`: several pools, no estimate; `listed_share_pct` as above.
- `unavailable`: the pool publishes no hashrate.
- `*_html` fields are rendered by the same templates as the page and are escaped. The rest of the body is data: render it as text.

`GET /healthz` is described under "Monitoring".

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

**Manual override, keep it.** The Wcash X link (`coins.wcash`, kind `x`) is set by hand to `https://x.com/WcashProject`, with a `note` explaining it was chosen by the maintainer. If the external links-regeneration script is ever re-run, it must keep this entry and not revert it to a different account. Re-apply it after any regeneration. A unit test (`links_tolerate_a_note_field_and_the_wcash_x_override_loads`) fails if the loaded Wcash X link is anything else.

## Logos

Every coin and pool shows a small logo next to its name: 20px in the pool, networks and archive tables, 18px in the coin selector, the Z15 list and the Hardware page, 16px in the sidebar, 40px in the coin header, the pool page and the drawer (32px on phones). Each mark sits in a 1px-bordered paper chip (radius 2px), so brand colours that don't match the ledger still look tidy. The chip stays light in dark mode, so black-on-transparent marks stay visible. White-on-transparent marks get a dark chip, and opaque square icons fill the chip edge to edge.

- **Files:** `static/logos/coins/` and `static/logos/pools/`, local copies only (no hotlinking). The name carries a content hash (`zcash.<hash>.svg`), so `/static/logos/*` is served with `Cache-Control: public, max-age=31536000, immutable`, plus its own `Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; sandbox` in case an SVG is opened directly. Only `coins/` or `pools/` files ending in `.svg`, `.webp` or `.png` are served.
- **Data:** `data/curated/logos.json`, written by the fetch script and hot-reloaded like the other curated files:

  ```json
  { "generated_at": "ISO",
    "coins": { "<coin id>": { "file": "coins/zcash.1a2b3c4d5e.svg", "source_url": "https://…", "fetched_at": "ISO",
                              "kind": "official|repo|third_party|fallback", "license_note": "…",
                              "bg": "tile (optional)", "ink": "light (optional)" } },
    "pools": { "<pool id>": { …same, plus "domain": "himpool.com" } } }
  ```

  `kind` is `official` (the coin's or pool's own website, or the coin's official GitHub organisation), `repo` (cryptocurrency-icons, CC0-1.0), `third_party` (the CoinGecko image for that exact coin id) or `fallback`. A fallback has `"file": null`, a `monogram` (one or two letters) and `tried`, the URLs that were tried and why each failed. Extra fields (`source_page`, `format`, `bytes`, `source_px`) are informational.
- **Fallbacks:** a pool or coin with no usable logo, no entry at all (for example a pool added after the last fetch) or a file that fails the server's checks shows a monogram of its initials in the ledger's ink and paper tones. A mark is never drawn or invented.
- **Checks on load** (`src/views/logo.rs`): the path must be a plain file name in `coins/` or `pools/`, the bytes must match the extension (SVG, PNG or WebP only; size 1 B to 64 KB), and an SVG must pass `svg_is_safe`: no `script`, `foreignObject`, `image`, `iframe`/`embed`/`object`, `style`, animation, DOCTYPE or entities, no `on*=` handlers, no `javascript:` and no `href`/`src`/`url()` except same-document `#id` references. A failing file is logged and replaced by the monogram.
- **Accessibility and speed:** every logo has `width` and `height` (no layout shift). Small logos sit next to the name, so their `alt` is empty and the name isn't read twice. Header logos say "<name> logo". Rows below the first 12, the networks table, the Z15 list and the archive load lazily.

### Fetch logos for new coins and pools

```bash
npm i --no-save sharp svgo        # optional, or NODE_PATH=<dir with them>/node_modules
node scripts/fetch-logos.mjs      # only coins/pools without an entry (or with a missing file)
node scripts/fetch-logos.mjs --retry-fallbacks   # try the monograms again
node scripts/fetch-logos.mjs --force --id zcash --id himpool.com   # refetch specific ones
node scripts/fetch-logos.mjs --dry-run           # report only
```

The hourly refresh does not run this script. It reads the coins and pools from the current snapshot plus `data/curated/coins.json`, `manual-pools.json` and `archive.json`. It refuses to run if it can't read them, so it never prunes logos because of a missing file.

- **Coins:** each verified `website` in `links.json`, then the coin's official GitHub org avatar (from `links.json`, or failing that an org named after the coin that its own site links to), then cryptocurrency-icons (matched by name and symbol), then CoinGecko (`COINGECKO` map, and the answer's symbol must match). `PINNED` holds hand-picked official assets: Wcash uses the mark w.cash serves, and Zcash uses z.cash's coin mark rather than the Zcash Foundation's.
- **Pools:** one fetch per operator domain (`zec.2miners.com` and `solo-btg.2miners.com` share `2miners.com`). It reads the pool URL, the origin and the root domain: `<link rel=icon|apple-touch-icon|mask-icon|manifest>`, square `og:image`, `<img>` with "logo" in it, and `/favicon.svg`, `/apple-touch-icon.png` and `/favicon.ico`.
  - An image named after a coin (`coin.png`, `bitcoin-btc-logo.svg`) is skipped, because that's a coin logo and not the pool's.
  - If two unrelated operators turn out to share the same mark (a pool-software default theme, such as s-nomp's), both get monograms. So does a pool whose only icon is a coin's logo. `node scripts/fetch-logos.mjs --recheck` re-runs these checks on the files already there, with no network.
- **Choice:** square marks only (aspect ratio up to 1.6:1, so no wordmarks). An SVG beats a raster of 128px or more, which beats 96px, then 64px, and so on. Rasters under 32px are refused.
- **Output:**
  - SVGs are sanitised with an allowlist (`sanitizeSvg`), minified with svgo, then sanitised again.
  - An SVG that still needs a `<style>` block or uses `<text>` (fonts vary inside `<img>`) is rasterised instead.
  - Rasters (PNG, ICO including BMP payloads, JPEG, GIF) have transparent margins trimmed, are fitted into 96px and saved as WebP. The script aims for 10 KB or less per file.
  - Files no entry uses any more are deleted from `static/logos/`.

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
- **Pool URLs** live in `data/curated/permalinks.json` (`"pools": {"<pool id>": "<slug>"}`). Change a slug only on purpose, and add the old one to `"aliases": {"<old slug>": "<new slug>"}` so links keep working (301). An invalid or duplicate slug is ignored with a warning and the pool gets its computed slug.

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

## Security

- Every page is rendered by maud, which escapes all text and attributes. Data embedded for scripts (`<script type="application/json">`) goes through `views::script_json`, which also escapes `<`, `>`, `&`, U+2028 and U+2029, so no upstream string can close the script element.
- Upstream URLs are checked twice: by the refresh script before it publishes (`sanitizeUrls` in `scripts/snapshot.mjs`, and validation refuses a snapshot that still has one) and when the server loads data (`data::safe_url`). Only `http:` and `https:` URLs with a host and no whitespace, control characters, `<`, `>`, `\` or backtick are kept. Anything else (`javascript:`, `data:`, `vbscript:`, relative or protocol-relative URLs) is dropped with a warning and the field shows as n/a or plain text, never as a link. The drawer and calculator re-check URLs in the browser.
- The server sends `Content-Security-Policy` (`default-src 'self'`, scripts only from the site plus the hash of the one inline theme script, `object-src 'none'`, `base-uri 'none'`, `frame-ancestors 'none'`), `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`, `X-Frame-Options: DENY`, `Permissions-Policy` and `Cross-Origin-Opener-Policy: same-origin` on every response, and HSTS when `HSTS=1`.
- The tests feed malicious fixtures (`javascript:` and `data:` URLs, `</script>`, U+2028, quotes and markup in names) through loading and every page.

## Tests

```bash
cargo test
```

```bash
node --test scripts/   # the refresh script's pure functions
```

`cargo test` currently runs 88 tests and `node --test scripts/` runs 38 (the refresh script, snapshot publishing, and the page's live-update and copy-link code against a small fake DOM). Tests that need a fixed snapshot read `testdata/snapshot-2026-10-02/`, a frozen copy of `data/` from the 2 Oct refresh, so they don't change when the data does.

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
- **Headline:** synthetic counts, and 116 pool rows with 63 positive on the snapshot (NiceHash is classified separately as a marketplace).
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
- **Share modes:** `/api/live` gives `share_pct: null` with a status, basis and denominator whenever the share isn't of the network; in the browser, a coin crossing its network estimate either way switches the share cells, split bar, header explanation, drawer and pool page.
- **Security:** script JSON escaping (`</script>`, `<!--`, U+2028/9), `safe_url`, malicious fixtures through every page, and `sanitizeUrls` in the refresh.
- **Snapshots:** publish, refusal on a failed critical source or a sharp drop (previous snapshot kept), schema errors, rollback and pruning, importing the flat layout, the lock; the server loads only the snapshot the manifest names, rejects a file whose hash doesn't match, and switches snapshots as one set.
- **Network estimate:** the coin header and the share denominator show the upstream network figure with its own source, never the listed pools' total or a capped value, whether it reads below or just above the pools' total.
- **HTTP:** every GET route answers HEAD, security headers are set, `/healthz` turns 503 on stale data, and old pool URLs redirect.
- **Permalinks:** a renamed pool keeps its URL and the old one redirects; the refresh adds ids only for new pools.

## Data sources

| What | Source |
|---|---|
| Coin list, pools, network stats | https://miningpoolstats.stream (JSON at https://data.miningpoolstats.stream/data/{coin}.js and `coins_data.js`) |
| Equihash hashpower market | NiceHash public EQUIHASH order book and algorithm metadata at https://api2.nicehash.com |
| Prices (coin header, network table, Hardware page, calculator) | https://data.miningpoolstats.stream/data/price/{coin}.js; when that fails, the `price` field of the coin file https://data.miningpoolstats.stream/data/{coin}.js (the price source URL says which) |
| ZEC / BTG cross-checks and block reward | https://zec.2miners.com/api/stats, `/api/blocks`; https://btg.2miners.com/api/stats, `/api/blocks` |
| zpool | https://zpool.ca/api/currencies, https://zpool.ca/api/status |
| ZergPool (optional) | https://zergpool.com/api/currencies |
| ZecWec (ZEC + WEC) | https://pool.zecwec.com/api/v1/overview (fees). Pool hashrate, on both the ZEC and WEC rows (the pool merge-mines both with the same work): https://pool.zecwec.com/api/v1/hashrate/pool (accepted work, previous 1200 s). WEC network estimate: https://pool.zecwec.com/api/v1/hashrate/network (previous 120 blocks). |
| unMineable (ZEC) | https://unmineable.com/support/article/how-to-setup-antminer-for-asic-mining |
| Wcash network | https://wcashexplorer.com/api/v1/status, `/api/v1/blocks?limit=1` |
| Merged-mining guide | https://w.cash/whitepaper; READMEs of `wcash-zcash-aux` and `wcash-merge-miner` pinned at commit 3e6b8044; ZIP-244 |
| ASIC specs | Bitmain support spec pages (support.bitmain.com), innosilicon.shop |
| ASIC seller offers | Each approved seller product page and legal/company record, linked per listing in `data/vendors.json`, `data/listings.json` and `data/curated/vendor-listing-sources.json` |
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

## Deploy

No containers: one binary, the `static/` and `scripts/` folders, and a data directory. Example files are in `deploy/`.

**Layout** (each release in its own directory, so the previous one is kept for rollback; data is shared):

```text
/srv/equihash/releases/<version>/   equihash-site, static/, scripts/, package.json, deploy/
/srv/equihash/current -> releases/<version>
/srv/equihash/data/                 current.json, snapshots/, curated/, archive.json, miners.json, research.json
```

**Install** (as root, once):

```bash
useradd --system --home /srv/equihash --shell /usr/sbin/nologin equihash
install -d -o equihash -g equihash /srv/equihash/releases /srv/equihash/data
cp deploy/equihash-site.service deploy/equihash-refresh.service deploy/equihash-refresh.timer deploy/equihash-listings-refresh.service deploy/equihash-listings-refresh.timer /etc/systemd/system/
systemctl daemon-reload
systemctl enable --now equihash-site.service equihash-refresh.timer equihash-listings-refresh.timer
```

**Each release** (build on a machine with Rust, then copy):

```bash
cargo build --release
V=$(date -u +%Y%m%dT%H%M%SZ)
rsync -a target/release/equihash-site static scripts package.json deploy server:/srv/equihash/releases/$V/
# first deploy, or when hand-kept data changed (never overwrite the snapshots):
rsync -a --exclude snapshots/ --exclude current.json --exclude .refresh.lock data/ server:/srv/equihash/data/
ssh server "ln -sfn releases/$V /srv/equihash/current.new && mv -T /srv/equihash/current.new /srv/equihash/current && systemctl restart equihash-site"
```

- **Service:** `deploy/equihash-site.service` runs as the unprivileged `equihash` user on `127.0.0.1:8080` with a read-only filesystem (`ProtectSystem=strict`, `NoNewPrivileges`, no capabilities) and `Restart=always`. Data edits and new snapshots are picked up without a restart.
- **Hourly refresh:** `deploy/equihash-refresh.timer` runs `deploy/equihash-refresh.service` (a oneshot `node scripts/refresh-data.mjs`, Node 18+) at 7 minutes past each hour; only `/srv/equihash/data` is writable. A refused snapshot (exit 2) marks the unit as failed and leaves the previous data live. Logs: `journalctl -u equihash-refresh`.
- **Daily vendor refresh:** `deploy/equihash-listings-refresh.timer` runs the approved-source listing parser once a day with a randomized offset. A source that cannot be matched safely keeps its previous price and timestamp. Logs: `journalctl -u equihash-listings-refresh`.
- **Reverse proxy:** `deploy/Caddyfile` (automatic TLS) or `deploy/nginx.conf` (certbot certificates), proxying to `127.0.0.1:8080` and redirecting `www` and plain HTTP. They add HSTS. CSP (with `frame-ancestors 'none'`), `X-Content-Type-Options`, `Referrer-Policy` and `X-Frame-Options` come from the app, so the CSP keeps matching the page's inline script; Caddy only fills them in if missing. If you change `INLINE_SCRIPT` in `src/views/layout.rs`, the app's CSP hash follows on its own.
- **Monitoring:** `GET /healthz` returns JSON: `status` (`ok`, `degraded` when a live source is down or stale or the last reload failed, `error` when the data is older than `HEALTH_MAX_DATA_AGE_SECS`), the snapshot id, `published_at`, `generated_at`, data age, counts and each live source's status. It answers 200, or 503 when the data is too old (so a failing refresh shows up within 3 hours). Point an uptime checker at `https://equihash.com/healthz`, or run `deploy/check-health.sh` from cron/a timer. Also watch `systemctl --failed`.
- **Rollback:**
  - data: `sudo -u equihash env DATA_DIR=/srv/equihash/data node /srv/equihash/current/scripts/refresh-data.mjs --rollback` (or `--rollback <id>`; `--list` shows them). The server switches within 2 s. The last 10 snapshots are kept.
  - binary: point `current` at the previous release and restart: `ln -sfn releases/<previous> /srv/equihash/current.new && mv -T /srv/equihash/current.new /srv/equihash/current && systemctl restart equihash-site`. Keep at least the last two release directories.
- **Server needs:** no database. Node is only needed for the refresh.

## Dev: screenshots

```bash
npm i --no-save playwright-core   # or point NODE_PATH at an existing install
BASE=http://127.0.0.1:8090 node scripts/screenshots.mjs   # uses system Chrome/Chromium; OUT=dir/ to write elsewhere
```

This writes `screenshots/`:
- **Desktop, 1440×900, light:** `home`, `home-full`, `home-all-coins`, `home-komodo`, `home-wcash`, `home-dark` (dark scheme), `pool-drawer`, `pool-page`, `hardware`, `calculator`, `calculator-wcash` (a Z15 Pro on WEC: n/a with the reason), `calculator-invalid` (shows the validation), `merged-mining-guide`, `archive`, `about`, `add-pool`, `sources`.
- **Mobile, 390×844 @2x:** `mobile-home`, `mobile-home-full`, `mobile-pools`, `mobile-drawer`, `mobile-hardware`, `mobile-calculator`, `mobile-guide`, `mobile-archive`, `mobile-about`.

`screenshots/before-after/` holds the v1 vs v2 comparisons.
