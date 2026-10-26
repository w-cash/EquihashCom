import { validateSourceUrl } from "./source-policy.mjs";

// Reviewed MiningPoolStats feed hosts. Page HTML must never extend this allowlist.
export const MPS_FEED_HOSTS = ["server.miningpoolstats.stream", "data.miningpoolstats.stream"];
const MPS = "https://miningpoolstats.stream";
const DEFAULT_FEED = "https://server.miningpoolstats.stream";

const decodeAttribute = (value) => value.replace(/&(?:amp|#38|#x26);/gi, "&");

/** Follow the exact JSON preload advertised by the official page, including its cache token.
 * Ignore unrelated resources and commented-out preloads; constrain every selected URL to the
 * reviewed HTTPS feed hosts. Older pages without a preload use the current reviewed host. */
export function resolveMpsFeed(html, page = "") {
  if (page && !/^[a-z0-9-]+$/.test(page)) throw new Error("invalid MiningPoolStats coin page");
  const clean = String(html).replace(/<!--[\s\S]*?-->/g, "");
  const resource = `/data/${page || "coins_data"}.js`;
  const timestamp = clean.match(/\bvar\s+last_time\s*=\s*["'](\d+)["']/)?.[1] ?? null;
  for (const tag of clean.match(/<link\b[^>]*>/gi) || []) {
    const attrs = Object.fromEntries([...tag.matchAll(/([\w:-]+)\s*=\s*(["'])(.*?)\2/g)]
      .map(([, key, , value]) => [key.toLowerCase(), decodeAttribute(value)]));
    if (!attrs.rel?.toLowerCase().split(/\s+/).includes("preload") || attrs.as?.toLowerCase() !== "fetch" || !attrs.href) continue;
    const candidate = new URL(attrs.href, MPS);
    if (candidate.pathname !== resource) continue;
    const url = validateSourceUrl(candidate.href, MPS_FEED_HOSTS);
    if (url.hash) throw new Error("MiningPoolStats feed URL must not contain a fragment");
    return { url: url.href, timestamp };
  }
  const url = new URL(resource, DEFAULT_FEED);
  if (timestamp) url.searchParams.set("t", timestamp);
  return { url: url.href, timestamp };
}

export function mpsPriceUrl(feedUrl, page) {
  const url = validateSourceUrl(feedUrl, MPS_FEED_HOSTS);
  if (!/^[a-z0-9-]+$/.test(page) || url.pathname !== `/data/${page}.js`) throw new Error("invalid MiningPoolStats price source");
  url.pathname = `/data/price/${page}.js`;
  return url.href;
}
