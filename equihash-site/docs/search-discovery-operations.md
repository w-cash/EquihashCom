# Search-discovery operations

This file separates repository work from account-level work. Do not claim a search result, ranking change or human-visitor count until the named measurement source confirms it.

## Completed in the application

- The application redirects every request whose host is `www.equihash.com` to the exact non-www path and query with HTTP 308. The reverse proxy configurations keep their matching redirects.
- Coin and pool titles include exact Equihash parameters where names can collide.
- Warning-only vendor records and pool records without enough sourced fields remain accessible with `noindex, follow` and are excluded from the sitemap.
- Origin request logs include method/path, status, host, query-stripped referrer and user agent. The application logger deliberately omits client addresses and request query strings. Run `npm run audit:http-status -- /path/to/access.log` before adding a redirect for a 404 or removing a legacy redirect, restrict log access, and keep the shortest operational retention period that supports the review.
- Anonymous HTML advertises a five-minute shared-cache TTL, while `/api/live`, `/healthz` and data routes retain their route-specific policies.
- Public JSON responses carry `schema_version: "1.0"`.
- The daily data refresh appends Zcash network, Z15 economics and seller observations to `market-history.json` without interpolation.

## Cloudflare owner actions

1. Create one Bulk Redirect or Redirect Rule: hostname equals `www.equihash.com`; preserve path and query; target scheme/host `https://equihash.com`; status 308. Test `/`, `/pools?coin=zcash` and a static asset. Keep the application redirect as the origin fallback.
2. Create a Cache Rule for GET/HEAD requests on `equihash.com` that are eligible HTML documents. Cache at the edge for five minutes. Bypass `/api/*`, `/healthz`, `/contribute`, `/add-vendor`, `/add-pool`, non-GET/HEAD methods and any future authenticated path. Verify `Age`/`CF-Cache-Status`, then confirm a new source snapshot appears within the intended window.
3. Export 404 and 301/308 traffic grouped by path, user agent, host and referrer. Add redirects only for legitimate external links or URLs with search impressions. Ignore or block exploit probes rather than creating routes for them.
4. Confirm every intended subdomain serves HTTPS before enabling `Strict-Transport-Security` with `includeSubDomains`. The checked-in nginx and Caddy examples include HSTS; production currently needs its active edge/proxy setting verified.
5. In AI Crawl Control, allow verified Google, Bing and OAI-SearchBot traffic. Do not whitelist a user-agent string alone when Cloudflare offers verified-bot or published-IP validation.

## Search measurement owner actions

1. Give the operating account access to the `equihash.com` Google Search Console domain property. Submit `https://equihash.com/sitemap.xml` and record a dated baseline export of queries, pages, countries, devices, impressions, clicks, CTR and position.
2. Verify Bing Webmaster Tools, submit the same sitemap and record its baseline.
3. Annotate the release date in the measurement log. Review specialist query families after 7, 14 and 30 days. Cloudflare request or visit counts alone are not ranking or human-audience evidence.

## Release checks

Run `cargo test`, `npm test`, `npm run verify:gates`, and a production-mode crawl. The crawl must find one canonical host, unique titles among indexable URLs, no internal 404s, no `noindex` URL in the sitemap and a valid JSON-LD object on every HTML page.
