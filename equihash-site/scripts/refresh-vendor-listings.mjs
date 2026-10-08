#!/usr/bin/env node

// Refresh only explicitly approved vendor variants. A parser must identify the exact
// configured variant and a plausible price before the last good listing is replaced.

import { open, readFile, rename, unlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = path.resolve(process.env.DATA_DIR || path.join(ROOT, "data"));
const CONFIG = path.join(DATA_DIR, "curated", "vendor-listing-sources.json");
const LISTINGS = path.join(DATA_DIR, "listings.json");
const LOCK = path.join(DATA_DIR, ".vendor-listings-refresh.lock");
const DRY_RUN = process.argv.includes("--dry-run");
const MAX_BYTES = 3 * 1024 * 1024;

export function decodeEntities(value = "") {
  return value
    .replace(/&#x([0-9a-f]+);/gi, (_, n) => String.fromCodePoint(parseInt(n, 16)))
    .replace(/&#(\d+);/g, (_, n) => String.fromCodePoint(Number(n)))
    .replace(/&(quot|apos|amp|lt|gt|nbsp|ndash|mdash);/gi, (_, n) => ({
      quot: '"', apos: "'", amp: "&", lt: "<", gt: ">", nbsp: " ", ndash: "–", mdash: "—",
    }[n.toLowerCase()]));
}

export function plainText(value = "") {
  return decodeEntities(value.replace(/<script[\s\S]*?<\/script>/gi, " ").replace(/<style[\s\S]*?<\/style>/gi, " ").replace(/<[^>]+>/g, " "))
    .replace(/\s+/g, " ").trim();
}

export function assertPublicHttps(raw) {
  const url = new URL(raw);
  if (url.protocol !== "https:" || url.username || url.password) throw new Error("source must be a credential-free HTTPS URL");
  const host = url.hostname.toLowerCase().replace(/^\[|\]$/g, "");
  if (host === "localhost" || host.endsWith(".local") || host === "::1" || /^127\./.test(host) || /^10\./.test(host) || /^192\.168\./.test(host) || /^169\.254\./.test(host)) throw new Error("local/private source blocked");
  const v4 = host.match(/^(\d+)\.(\d+)\.(\d+)\.(\d+)$/)?.slice(1).map(Number);
  if (v4 && (v4.some((n) => n > 255) || v4[0] === 0 || v4[0] === 127 || v4[0] === 10 || (v4[0] === 172 && v4[1] >= 16 && v4[1] <= 31) || (v4[0] === 192 && v4[1] === 168))) throw new Error("local/private source blocked");
  return url;
}

function balancedJson(text, start) {
  const opener = text[start];
  const closer = opener === "[" ? "]" : "}";
  let depth = 0, quoted = false, escaped = false;
  for (let i = start; i < text.length; i++) {
    const c = text[i];
    if (quoted) {
      if (escaped) escaped = false;
      else if (c === "\\") escaped = true;
      else if (c === '"') quoted = false;
    } else if (c === '"') quoted = true;
    else if (c === opener) depth++;
    else if (c === closer && --depth === 0) return text.slice(start, i + 1);
  }
  throw new Error("unterminated embedded JSON");
}

function findJsonAfter(text, marker, opener = "[") {
  const at = text.indexOf(marker);
  if (at < 0) throw new Error(`embedded data marker not found: ${marker}`);
  const start = text.indexOf(opener, at + marker.length);
  if (start < 0) throw new Error("embedded JSON start not found");
  return JSON.parse(balancedJson(text, start));
}

export function wooVariations(html) {
  const match = html.match(/data-product_variations\s*=\s*(?:"([\s\S]*?)"|'([\s\S]*?)')/i);
  if (!match) throw new Error("WooCommerce variants not found");
  const parsed = JSON.parse(decodeEntities(match[1] ?? match[2]));
  if (!Array.isArray(parsed)) throw new Error("WooCommerce variant payload is not an array");
  return parsed;
}

function money(value) {
  const n = Number(String(value ?? "").replace(/[^0-9.-]/g, ""));
  return Number.isFinite(n) && n > 0 ? n : null;
}

function wooOffer(html, source) {
  const needle = source.variant_match.toLowerCase().replace(/\s+/g, " ");
  const candidates = wooVariations(html).filter((v) => JSON.stringify(v.attributes || v).toLowerCase().replace(/\s+/g, " ").includes(needle));
  if (candidates.length !== 1) throw new Error(`expected one WooCommerce variant for “${source.variant_match}”, found ${candidates.length}`);
  const v = candidates[0];
  const availabilityText = plainText(v.availability_html || "").toLowerCase();
  return { price: money(v.display_price ?? v.display_regular_price), pageText: `${JSON.stringify(v.attributes || {})} ${availabilityText}` };
}

function shopifyOffer(html, source) {
  let variants;
  try { variants = findJsonAfter(html, '"variants":', "["); }
  catch { variants = findJsonAfter(html, 'variants":', "["); }
  const needle = source.variant_match.toLowerCase();
  const candidates = variants.filter((v) => String(v.public_title ?? v.title ?? v.name ?? "").toLowerCase() === needle);
  if (candidates.length !== 1) throw new Error(`expected one Shopify variant for “${source.variant_match}”, found ${candidates.length}`);
  const v = candidates[0];
  const raw = v.price?.amount ?? v.price;
  const price = typeof raw === "number" && raw > 100000 ? raw / 100 : money(raw);
  return { price, pageText: JSON.stringify(v) };
}

function adeOffer(html, source) {
  const marker = `id="${source.block_id}"`;
  const start = html.indexOf(marker);
  if (start < 0) throw new Error(`ADE block ${source.block_id} not found`);
  const next = html.indexOf('id="product', start + marker.length);
  const block = html.slice(start, next > start ? next : start + 50000);
  const price = money(block.match(/data-excl-price=["']([^"']+)/i)?.[1] ?? block.match(/€\s*([\d.,]+)/)?.[1]);
  return { price, pageText: plainText(block) };
}

function hashlabsOffer(html, source) {
  const compact = html.replace(/\\u0026/g, "&").replace(/\\"/g, '"');
  const options = [...compact.matchAll(/<label\s+class=["'][^"']*deal-option[^"']*["']([^>]*)>([\s\S]*?)<\/label>/gi)].map((match) => {
    const attrs = Object.fromEntries([...match[1].matchAll(/data-([\w-]+)=["']([^"']*)["']/gi)].map((item) => [item[1].toLowerCase(), decodeEntities(item[2])]));
    return { attrs, body: match[2] };
  });
  const candidates = options.filter(({ attrs }) => attrs.mode?.toLowerCase() === source.mode.toLowerCase()
    && (!source.facility_match || attrs.facility?.toLowerCase() === source.facility_match.toLowerCase()));
  if (candidates.length === 1) {
    const selected = candidates[0];
    return { price: money(selected.attrs.price), pageText: plainText(selected.body) };
  }
  if (candidates.length > 1) throw new Error(`Hashlabs option is ambiguous: ${candidates.length} matches`);
  const facility = source.facility_match || "";
  const needles = source.mode === "delivery" ? ["delivery", "buyer"] : ["hosting", facility.toLowerCase()];
  const lower = compact.toLowerCase();
  let pivot = 0;
  for (const needle of needles) { const found = lower.indexOf(needle, pivot); if (found < 0) throw new Error(`Hashlabs deal text not found: ${needle}`); pivot = found; }
  const block = compact.slice(Math.max(0, pivot - 1500), pivot + 3000);
  const prices = [...block.matchAll(/(?:data-price=["']|"price"\s*:\s*|\$)"?([0-9]{4,5}(?:\.[0-9]+)?)/gi)].map((m) => money(m[1])).filter(Boolean);
  if (!prices.length) throw new Error("Hashlabs deal price not found near configured option");
  return { price: prices[0], pageText: plainText(block) };
}

function metaPrice(html) {
  const patterns = [
    /<meta[^>]+property=["']product:price:amount["'][^>]+content=["']([^"']+)/i,
    /<meta[^>]+content=["']([^"']+)["'][^>]+property=["']product:price:amount["']/i,
    /"price"\s*:\s*"?([0-9]+(?:\.[0-9]+)?)/i,
  ];
  for (const p of patterns) { const n = money(html.match(p)?.[1]); if (n) return n; }
  return null;
}

function genericOffer(html, source) {
  const text = plainText(html);
  if (!text.toLowerCase().includes(source.name_match.toLowerCase())) throw new Error(`product name “${source.name_match}” not found`);
  return { price: metaPrice(html), pageText: text };
}

function mineshopOffer(html, source) {
  const text = plainText(html);
  if (!text.toLowerCase().includes(source.batch_match.toLowerCase())) throw new Error(`batch “${source.batch_match}” not found`);
  return { price: metaPrice(html), pageText: text };
}

export function parseOffer(html, source) {
  for (const token of source.required_tokens || []) if (!plainText(html).toLowerCase().includes(token.toLowerCase())) throw new Error(`required product token missing: ${token}`);
  const parsers = { woo_variation: wooOffer, shopify_variant: shopifyOffer, ade_product: adeOffer, hashlabs_deal: hashlabsOffer, generic_product: genericOffer, mineshop_batch: mineshopOffer };
  const parser = parsers[source.parser];
  if (!parser) throw new Error(`unknown parser: ${source.parser}`);
  const offer = parser(html, source);
  if (!offer.price || offer.price < source.min_price || offer.price > source.max_price) throw new Error(`price failed range check: ${offer.price}`);
  return { price_amount: offer.price, price_currency: source.currency, availability: source.availability, availability_label: source.availability_label };
}

async function fetchPage(raw) {
  const url = assertPublicHttps(raw);
  const response = await fetch(url, { redirect: "follow", signal: AbortSignal.timeout(25000), headers: { "user-agent": "equihash.com vendor research refresh/1.0 (+https://equihash.com/sources)", accept: "text/html,application/xhtml+xml" } });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  assertPublicHttps(response.url);
  const type = response.headers.get("content-type") || "";
  if (!/text\/html|application\/xhtml\+xml/i.test(type)) throw new Error(`unexpected content type: ${type || "missing"}`);
  const declared = Number(response.headers.get("content-length"));
  if (declared > MAX_BYTES) throw new Error("response exceeds size limit");
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (bytes.byteLength > MAX_BYTES) throw new Error("response exceeds size limit");
  return new TextDecoder().decode(bytes);
}

async function pool(items, limit, job) {
  const results = new Array(items.length); let cursor = 0;
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (cursor < items.length) { const i = cursor++; try { results[i] = { ok: true, value: await job(items[i]) }; } catch (error) { results[i] = { ok: false, error }; } }
  }));
  return results;
}

export function applyOffer(listing, offer, observedAt) {
  return { ...listing, ...offer, observed_at: observedAt };
}

async function main() {
  let lock;
  try {
    if (!DRY_RUN) lock = await open(LOCK, "wx");
    const [config, document] = await Promise.all([readFile(CONFIG, "utf8").then(JSON.parse), readFile(LISTINGS, "utf8").then(JSON.parse)]);
    const listings = new Map(document.listings.map((row) => [row.id, row]));
    const urls = [...new Set(config.sources.map((source) => assertPublicHttps(source.url).href))];
    const fetched = await pool(urls, 4, async (url) => fetchPage(url));
    const pages = new Map(urls.map((url, i) => [url, fetched[i]]));
    const observedAt = new Date().toISOString();
    const failures = [], changes = [];
    for (const source of config.sources) {
      const listing = listings.get(source.listing_id);
      if (!listing) { failures.push(`${source.listing_id}: listing is missing`); continue; }
      const fetchedPage = pages.get(assertPublicHttps(source.url).href);
      if (!fetchedPage?.ok) { failures.push(`${source.listing_id}: ${fetchedPage?.error?.message || "fetch failed"}`); continue; }
      try {
        const offer = parseOffer(fetchedPage.value, source);
        listings.set(source.listing_id, applyOffer(listing, offer, observedAt));
        changes.push(`${source.listing_id}: ${offer.price_currency} ${offer.price_amount} · ${offer.availability_label}`);
      } catch (error) { failures.push(`${source.listing_id}: ${error.message}`); }
    }
    if (!changes.length) throw new Error(`no approved listing passed validation; last good file preserved\n${failures.join("\n")}`);
    const output = { ...document, verified_at: observedAt, listings: document.listings.map((row) => listings.get(row.id)) };
    if (!DRY_RUN) {
      const tmp = `${LISTINGS}.tmp-${process.pid}`;
      await writeFile(tmp, `${JSON.stringify(output, null, 2)}\n`, { mode: 0o644 });
      await rename(tmp, LISTINGS);
    }
    console.log(`${DRY_RUN ? "Checked" : "Updated"} ${changes.length}/${config.sources.length} approved variants; ${failures.length} preserved from the last good file.`);
    for (const line of changes) console.log(`ok  ${line}`);
    for (const line of failures) console.warn(`keep ${line}`);
  } finally {
    await lock?.close().catch(() => {});
    if (lock) await unlink(LOCK).catch(() => {});
  }
}

if (pathToFileURL(process.argv[1]).href === import.meta.url) main().catch((error) => { console.error(error.message); process.exitCode = 2; });
