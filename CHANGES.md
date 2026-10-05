# CHANGES — 5 Oct 2026 (Buy: Equihash ASIC marketplace)

Baseline: production `main` at `68461c4`. Prepared and verified as the next production release.

## What
New top-nav **Buy** tab (`/buy`) — a prestige shop directory for Equihash ASICs. Hardware (`/miners`) stays manufacturer specs only.

## First listing (hand-checked)
- **Vendor:** The Mining Shop UK — https://www.theminingshop.co.uk/
- **Product:** Bitmain Antminer Z15 Pro 860KSol — https://www.theminingshop.co.uk/bitmain-antminer-z15-pro-860ksol-equihash-zcash-miner/
- **Observed 2026-10-05 20:15 UTC:** price **£12,800 GBP ex VAT**; availability **In stock — ready to ship**; free UK delivery (schema shippingRate £0 to GB), DDP UK/EU warehouse; shop hashrate **860 kSol** (Bitmain typical 840 remains on Hardware); `miner_id` = `antminer-z15-pro`.

## Files
- `data/curated/vendors.json`, `data/curated/listings.json`
- `data/curated/logos.json` (+ vendors map) · `static/logos/vendors/the-mining-shop-uk.239a526176.png`
- `static/shop/machines/antminer-z15-pro-860.811ddcd13a.webp`
- `src/views/buy.rs` · logo/data/layout/pages/main/fmt/CSS/JS wiring
- `/buy/vendor/{slug}`, `/add-vendor`, Hardware “Where to buy”, footer honesty
- Tests: data load, shop image path check, money n/a≠0, route 200 + outbound CTA

## Design
Extends the ledger: Source Serif + IBM Plex, paper/ink, 2px radius, large product photography in calm cards, soft “View at {shop}” CTAs. No affiliate ribbons, no urgency, no checkout.
