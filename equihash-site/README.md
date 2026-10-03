# equihash.com

A neutral directory of Equihash mining pools, built for miners. It covers every pool on every Equihash coin and lists hashrate share, fees, payout schemes, regions, sources and a 30% concentration warning. It also has an ASIC page, a profitability estimate, an archive of retired pools and ended coins, and a step-by-step merged-mining guide for pool operators.

This is a proof of concept. It is kept simple on purpose:

- **Backend:** one Rust binary built on [actix-web 4](https://actix.rs). HTML is rendered on the server with [maud](https://maud.lambda.xyz), whose templates are checked at compile time.
- **Data:** plain JSON files in `data/`. The server loads them into typed serde structs and reloads them when they change.
- **Frontend:** hand-written CSS (`static/app.css`) and one vanilla JS file (`static/app.js`). There is no build step, bundler or framework. Every page works without JS, and JS adds live filtering, a slide-in pool drawer (a bottom sheet on mobile), chart tabs and the live calculator.

```
src/
  main.rs          server, routes, env config, hot reload of data/
  data.rs          serde types + load-time derivations (slugs, regions, shares)
  fmt.rs           number/hashrate/date formatting (unknown → "n/a")
  views/           maud templates: layout, home, pages, guide, calc
static/            app.css, app.js, icons, og.png (served as /static/*)
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

Routes:

- Pages: `/` (pools, chart and networks), `/pool/{slug}`, `/archive`, `/miners`, `/calculator`, `/merged-mining`, `/add-pool`, `/about`, `/sources`.
- Raw data: `/data/{pools,network,miners,archive,meta,research}.json`.
- Also served: `/sitemap.xml`, `/robots.txt` and `/static/*`.

The server checks `data/` and `data/curated/` every 5 s and hot-reloads them. If a file is broken, it logs a warning and keeps serving the last good data.

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
6. **`data/curated/coin-status.json`.** It marks coins whose proof of work has ended.
7. **Shares.** It computes each pool's share as pool hashrate divided by the network hashrate. On some small coins the published network estimate is lower than the sum of what the pools report. In that case the share is measured against the listed pools' total instead, and the UI says so.

Every row stores `source_url`, `data_url` and `fetched_at`. Values the source does not publish stay `null` and show as **n/a**.

### Schedule it

cron (every 30 min), next to the deployed binary:

```cron
*/30 * * * * cd /srv/equihash && /usr/bin/node scripts/refresh-data.mjs >> /var/log/equihash-refresh.log 2>&1
```

Or use a GitHub Action that commits the refreshed JSON. Your deploy then pulls or rsyncs `data/`:

```yaml
# .github/workflows/refresh.yml
name: refresh-data
on:
  schedule: [{ cron: "*/30 * * * *" }]
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

## Editing data by hand (no code changes)

All changes in this section only need a JSON edit. The server shows them within 5 seconds, and the next refresh keeps them.

- **Add a pool that isn't on miningpoolstats.** Append an object to `data/curated/manual-pools.json`. It uses the same shape as a row in `pools.json`.
  - Required: `id`, `name`, `coin` (symbol), `url`, `source_url`, `verified_at`.
  - Set anything you can't source to `null`.
  - Set `"hashrate_is_reported": true` when the figure comes from the operator rather than a public API. The UI then marks it with †.
- **Add a coin.** Append an object to `data/curated/coins.json` with the same shape as a coin in `network.json`: `id`, `name`, `symbol`, `equihash: {n, k}`, `hardware`, `network: {hashrate, unit, difficulty, height, block_time_target_s, …}`, `source_url` and `fetched_at`. Coins that miningpoolstats already lists appear automatically on the next refresh. Pools for the new coin go in `manual-pools.json` with `"coin": "<SYMBOL>"`.
- **A coin's proof of work ended.** Add `{id, status: "ended", note, sources: [{label, url}]}` to `data/curated/coin-status.json`.
- **A pool shut down.** Add an entry to `data/archive.json` with `reason`, `retired` and at least one `sources` link.
- **A new ASIC.** Add an entry to `data/miners.json` with the manufacturer's spec URL. Efficiency is calculated as watts ÷ kSol/s.
- **The research log** behind `/sources` lives in `data/research.json`.

For templates or styling, edit `src/views/*.rs` and `cargo run` again (maud will catch template mistakes at compile time). You can also edit `static/app.css` or `static/app.js` and just reload the browser.

## Data sources

| What | Source |
|---|---|
| Coin list, pools, network stats | https://miningpoolstats.stream (JSON at https://data.miningpoolstats.stream/data/{coin}.js and `coins_data.js`) |
| Prices (calculator, coin cards) | https://data.miningpoolstats.stream/data/price/{coin}.js |
| ZEC / BTG cross-checks and block reward | https://zec.2miners.com/api/stats, `/api/blocks`; https://btg.2miners.com/api/stats, `/api/blocks` |
| zpool | https://zpool.ca/api/currencies, https://zpool.ca/api/status |
| ZergPool (optional) | https://zergpool.com/api/currencies |
| ZecWec (ZEC + WEC) | https://pool.zecwec.com/api/v1/overview (fees); hashrate ≈0.44 MSol/s as reported by the operator |
| unMineable (ZEC) | https://unmineable.com/support/article/how-to-setup-antminer-for-asic-mining |
| Wcash network | https://wcashexplorer.com/api/v1/status, `/api/v1/blocks?limit=1` |
| Merged-mining guide | https://w.cash/whitepaper; READMEs of `wcash-zcash-aux` and `wcash-merge-miner` pinned at commit 3e6b8044; ZIP-244 |
| ASIC specs | Bitmain support spec pages (support.bitmain.com), innosilicon.shop |
| Archive and ended coins | citations in `data/archive.json` and `data/curated/coin-status.json` |

### Known gaps

These show as n/a and are listed on `/sources`:

- **ZecWec hashrate** is the operator-reported figure, because the public API does not expose it.
- **unMineable** publishes no ZEC fee, scheme or hashrate data.
- **Not verified:** Suprnova (API returned 503), Nanopool (ZEC API returned 404) and ZergPool (522 errors during research).
- **No price feed** from miningpoolstats for MARKS, CDY, BUCK, HUSH, MCL, SQCN, TKL, VOLLAR or WEC.
- **WEC network hashrate** is not published.
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
BASE=http://127.0.0.1:8080 node scripts/screenshots.mjs   # uses system Chrome/Chromium
```
