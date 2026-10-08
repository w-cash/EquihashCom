# Architecture and deployment audit

Audit date: 2026-10-08. Scope: local repository and candidate rendering. Production parity remains unconfirmed until the deployed binary, data directory, service units, proxy configuration and live route set are compared on the production host.

## Actual stack

- One Rust/Actix Web binary; Maud templates render HTML on the server.
- Typed Serde data in `src/data.rs`; generated pool/network data is read through a hashed `data/current.json` snapshot manifest.
- Hand-maintained or separately refreshed JSON supplies ASICs, vendors, offers, research and hashpower-market data.
- Vanilla CSS and JavaScript; no client framework or frontend build pipeline.
- Local assets are served below `/static`; data downloads below `/data`.
- Deployment examples use an unprivileged systemd service behind Caddy or nginx. Pool/network refresh and vendor-offer refresh run as separate restricted services.

This stack already supports the intended product. No framework migration is approved.

## Source and generated boundaries

`data/snapshots/*/{pools,network,meta}.json` and `data/current.json` are generated and atomically published by `scripts/refresh-data.mjs` plus `scripts/snapshot.mjs`. `data/listings.json` is a generated/candidate seller-claim dataset refreshed only through the approved-source configuration. `data/hashpower.json` is an aggregate marketplace snapshot. Curated files, ASIC records, vendor research and editorial research require their own reviewed changes.

The server keeps last-good state when a candidate reload is invalid. Optional dataset failures must be observable and must not silently replace a valid section with an empty one.

## Build and verification

Required local checks are Rust formatting/tests, every offline Node test, deterministic vendor import, critical-record validation, HTML route/HEAD tests, keyboard/mobile visual review and the release-gate report. A passing code suite is evidence for code gates only.

## Production confirmation still required

Record the production commit, binary checksum, Rust/Node versions, `DATA_DIR`, `STATIC_DIR`, active systemd unit text, timers, proxy headers, TLS/HSTS, log retention, deployed `current.json` snapshot, health response and rollback target. Compare the live sitemap and every new canonical route with the local candidate. Until this is done, production parity remains pending.
