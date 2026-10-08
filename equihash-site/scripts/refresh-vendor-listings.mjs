#!/usr/bin/env node

// Refresh only explicitly approved vendor variants. A parser must identify the exact
// configured variant and a plausible price before the last good listing is replaced.

import { open, readFile, rename, unlink, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { fetchBounded, validateSourceUrl } from "./source-policy.mjs";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DATA_DIR = path.resolve(process.env.DATA_DIR || path.join(ROOT, "data"));
const CONFIG = path.join(DATA_DIR, "curated", "vendor-listing-sources.json");
const LISTINGS = path.join(DATA_DIR, "listings.json");
const LOCK = path.join(DATA_DIR, ".vendor-listings-refresh.lock");
const DRY_RUN = process.argv.includes("--dry-run");
const MAX_BYTES = 3 * 1024 * 1024;
export const CURRENT_AVAILABILITY_MAX_AGE_MS = 36 * 60 * 60 * 1000;

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
  return validateSourceUrl(url.href, [url.hostname]);
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

function jsonLd(html) {
  const values = [];
  for (const match of html.matchAll(/<script[^>]+type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi)) {
    try { values.push(JSON.parse(decodeEntities(match[1]))); } catch { /* malformed unrelated data is ignored */ }
  }
  return values;
}

function flattenJsonLd(value, out = []) {
  if (Array.isArray(value)) for (const item of value) flattenJsonLd(item, out);
  else if (value && typeof value === "object") {
    out.push(value);
    if (value["@graph"]) flattenJsonLd(value["@graph"], out);
  }
  return out;
}

function currenciesIn(text = "") {
  const found = new Set();
  const patterns = [
    /(?:priceCurrency|currencyCode|currency|product:price:currency)["'\s:=]+(?:content=["'])?([A-Z]{3})\b/gi,
    /<meta[^>]+content=["']([A-Z]{3})["'][^>]+(?:product:price:currency|price:currency)/gi,
  ];
  for (const pattern of patterns) for (const match of text.matchAll(pattern)) found.add(match[1].toUpperCase());
  if (/£\s*\d/.test(text)) found.add("GBP");
  if (/€\s*\d|\d\s*€/.test(text)) found.add("EUR");
  if (/\$\s*\d/.test(text)) found.add("USD");
  return found;
}

function availabilityIn(text = "") {
  const normalized = plainText(text).toLowerCase();
  // A negative statement wins over positive boilerplate on the same offer.
  if (/\b(sold\s*out|out\s*of\s*stock|outofstock|unavailable|not\s*available)\b/.test(normalized)
      || /"(?:available|is_in_stock)"\s*:\s*false/.test(text.toLowerCase())) return "sold_out";
  if (/\bwait\s*list|\bwaitlist\b/.test(normalized)) return "waitlist";
  if (/\bback[ -]?order/.test(normalized)) return "backorder";
  if (/\bpre[ -]?order|\bbatch\b|\b(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|may|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\s+20\d{2}\b|\b(?:jan(?:uary)?|feb(?:ruary)?|mar(?:ch)?|apr(?:il)?|jun(?:e)?|jul(?:y)?|aug(?:ust)?|sep(?:tember)?|oct(?:ober)?|nov(?:ember)?|dec(?:ember)?)\b/.test(normalized)) return "preorder";
  if (/\bdispatch(?:es|ed)?\b|\bships?\s+(?:out|within|in)\b/.test(normalized)) return "dispatch_claim";
  if (/\b(request (?:an? )?(?:offer|quote)|available deals?|contact (?:us|sales) for (?:a )?(?:price|quote))\b/.test(normalized)) return "quote";
  if (/\bin\s*stock\b|\binstock\b|\bready\s*to\s*ship\b/.test(normalized)
      || /"(?:available|is_in_stock)"\s*:\s*true/.test(text.toLowerCase())) return "in_stock";
  return null;
}

function bindEvidence(html, source, offer) {
  const evidence = String(offer.evidenceText || "");
  const localCurrencies = currenciesIn(evidence);
  const currencies = localCurrencies.size ? localCurrencies : currenciesIn(html);
  if (!currencies.size) throw new Error("currency evidence not found for the matched offer");
  if (currencies.size !== 1 || !currencies.has(source.currency)) {
    throw new Error(`currency evidence contradicts ${source.currency}: ${[...currencies].sort().join(", ")}`);
  }
  const availability = availabilityIn(evidence);
  if (!availability) throw new Error("availability evidence not found for the matched offer");
  // A seller's backorder flag is a stricter, non-current form of a configured future batch.
  // Preserve the exact observed state instead of upgrading it to the curated `preorder` label.
  const conservativeFutureState = source.availability === "preorder" && availability === "backorder";
  if (availability !== source.availability && !conservativeFutureState) {
    throw new Error(`availability evidence contradicts ${source.availability}: ${availability}`);
  }
  return { currency: source.currency, availability };
}

function wooOffer(html, source) {
  const needle = source.variant_match.toLowerCase().replace(/\s+/g, " ");
  const candidates = wooVariations(html).filter((v) => JSON.stringify(v.attributes || v).toLowerCase().replace(/\s+/g, " ").includes(needle));
  if (candidates.length !== 1) throw new Error(`expected one WooCommerce variant for “${source.variant_match}”, found ${candidates.length}`);
  const v = candidates[0];
  const availabilityText = plainText(v.availability_html || "").toLowerCase();
  return { price: money(v.display_price ?? v.display_regular_price), evidenceText: `${JSON.stringify(v)} ${availabilityText}` };
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
  return { price, evidenceText: JSON.stringify(v) };
}

function adeOffer(html, source) {
  const marker = `id="${source.block_id}"`;
  const start = html.indexOf(marker);
  if (start < 0) throw new Error(`ADE block ${source.block_id} not found`);
  const next = html.indexOf('id="product', start + marker.length);
  const block = html.slice(start, next > start ? next : start + 50000);
  const price = money(block.match(/data-excl-price=["']([^"']+)/i)?.[1] ?? block.match(/€\s*([\d.,]+)/)?.[1]);
  return { price, evidenceText: block };
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
    return { price: money(selected.attrs.price), evidenceText: `${JSON.stringify(selected.attrs)} ${selected.body}` };
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
  return { price: prices[0], evidenceText: block };
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
  const products = jsonLd(html).flatMap((value) => flattenJsonLd(value))
    .filter((value) => String(value["@type"] || "").toLowerCase() === "product")
    .filter((value) => String(value.name || "").toLowerCase().includes(source.name_match.toLowerCase()));
  if (products.length > 1) throw new Error(`expected one structured product for “${source.name_match}”, found ${products.length}`);
  if (products.length === 1) {
    const offers = Array.isArray(products[0].offers) ? products[0].offers : [products[0].offers].filter(Boolean);
    if (offers.length !== 1) throw new Error(`expected one offer for “${source.name_match}”, found ${offers.length}`);
    let faq;
    if (source.faq_question_match) {
      const needle = source.faq_question_match.toLowerCase();
      const questions = jsonLd(html).flatMap((value) => flattenJsonLd(value))
        .filter((value) => String(value["@type"] || "").toLowerCase() === "faqpage")
        .flatMap((value) => Array.isArray(value.mainEntity) ? value.mainEntity : [value.mainEntity].filter(Boolean))
        .filter((value) => String(value?.name || "").toLowerCase().includes(needle));
      if (questions.length !== 1) throw new Error(`expected one FAQ answer for “${source.faq_question_match}”, found ${questions.length}`);
      faq = questions[0];
    }
    return { price: money(offers[0].price ?? offers[0].lowPrice), evidenceText: JSON.stringify({ name: products[0].name, offer: offers[0], faq }) };
  }
  const headings = [...html.matchAll(/<h1\b[^>]*>([\s\S]*?)<\/h1>/gi)]
    .filter((match) => plainText(match[1]).toLowerCase().includes(source.name_match.toLowerCase()));
  if (headings.length !== 1) throw new Error(`unstructured page does not identify one “${source.name_match}” product`);
  return { price: metaPrice(html), evidenceText: html };
}

function mineshopOffer(html, source) {
  const text = plainText(html);
  if (!text.toLowerCase().includes(source.batch_match.toLowerCase())) throw new Error(`batch “${source.batch_match}” not found`);
  return { price: metaPrice(html), evidenceText: html };
}

export function parseOffer(html, source) {
  for (const token of source.required_tokens || []) if (!plainText(html).toLowerCase().includes(token.toLowerCase())) throw new Error(`required product token missing: ${token}`);
  const parsers = { woo_variation: wooOffer, shopify_variant: shopifyOffer, ade_product: adeOffer, hashlabs_deal: hashlabsOffer, generic_product: genericOffer, mineshop_batch: mineshopOffer };
  const parser = parsers[source.parser];
  if (!parser) throw new Error(`unknown parser: ${source.parser}`);
  const offer = parser(html, source);
  if (!offer.price || offer.price < source.min_price || offer.price > source.max_price) throw new Error(`price failed range check: ${offer.price}`);
  const evidence = bindEvidence(html, source, offer);
  return {
    price_amount: offer.price,
    price_currency: evidence.currency,
    availability: evidence.availability,
    availability_label: source.availability_label,
  };
}

async function fetchPage(raw, allowedHosts) {
  const result = await fetchBounded(raw, {
    allowedHosts,
    accept: "text/html,application/xhtml+xml",
    contentTypes: ["text/html", "application/xhtml+xml"],
    maxBytes: MAX_BYTES,
    timeoutMs: 25_000,
    userAgent: "equihash.com vendor research refresh/1.0 (+https://equihash.com/sources)",
  });
  return new TextDecoder().decode(result.body);
}

async function pool(items, limit, job) {
  const results = new Array(items.length); let cursor = 0;
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, async () => {
    while (cursor < items.length) { const i = cursor++; try { results[i] = { ok: true, value: await job(items[i]) }; } catch (error) { results[i] = { ok: false, error }; } }
  }));
  return results;
}

export function applyOffer(listing, offer, observedAt) {
  const out = { ...listing };
  if (offer.price_amount !== undefined) {
    out.price_amount = offer.price_amount;
    out.price_currency = offer.price_currency;
    out.price_observed_at = observedAt;
  }
  if (offer.availability !== undefined) {
    out.availability = offer.availability;
    out.availability_label = offer.availability_label;
    out.availability_observed_at = observedAt;
    // Compatibility for clients that have not yet adopted the per-field clocks. It follows
    // availability, never a price-only refresh, so price cannot make stock look freshly checked.
    out.observed_at = observedAt;
    delete out.last_known_availability;
    delete out.last_known_availability_label;
    delete out.availability_refresh_error_at;
  }
  return out;
}

export function expireCurrentAvailability(listing, attemptedAt, reason, maxAgeMs = CURRENT_AVAILABILITY_MAX_AGE_MS) {
  if (!["in_stock", "dispatch_claim"].includes(listing.availability)) return listing;
  const checkedAt = listing.availability_observed_at || listing.observed_at;
  const age = checkedAt ? Date.parse(attemptedAt) - Date.parse(checkedAt) : Number.POSITIVE_INFINITY;
  if (!reason && Number.isFinite(age) && age <= maxAgeMs) return listing;
  const previous = listing.availability_label || listing.availability;
  return {
    ...listing,
    last_known_availability: listing.availability,
    last_known_availability_label: previous,
    availability: "unknown",
    availability_label: `Current availability unknown; last seller claim: ${previous}`,
    availability_observed_at: checkedAt || null,
    ...(reason ? { availability_refresh_error_at: attemptedAt } : {}),
  };
}

async function main() {
  let lock;
  try {
    if (!DRY_RUN) lock = await open(LOCK, "wx");
    const [config, document] = await Promise.all([readFile(CONFIG, "utf8").then(JSON.parse), readFile(LISTINGS, "utf8").then(JSON.parse)]);
    const listings = new Map(document.listings.map((row) => [row.id, row]));
    const urls = [...new Set(config.sources.map((source) => assertPublicHttps(source.url).href))];
    const allowedHosts = [...new Set(urls.map((url) => new URL(url).hostname))];
    const fetched = await pool(urls, 4, async (url) => fetchPage(url, allowedHosts));
    const pages = new Map(urls.map((url, i) => [url, fetched[i]]));
    const observedAt = new Date().toISOString();
    const failures = [], changes = [], expired = [];
    for (const source of config.sources) {
      const listing = listings.get(source.listing_id);
      if (!listing) { failures.push({ listing_id: source.listing_id, error: "listing is missing" }); continue; }
      const fetchedPage = pages.get(assertPublicHttps(source.url).href);
      if (!fetchedPage?.ok) {
        const error = fetchedPage?.error?.message || "fetch failed";
        failures.push({ listing_id: source.listing_id, error });
        const next = expireCurrentAvailability(listing, observedAt, error);
        if (next !== listing) { listings.set(source.listing_id, next); expired.push(source.listing_id); }
        continue;
      }
      try {
        const offer = parseOffer(fetchedPage.value, source);
        listings.set(source.listing_id, applyOffer(listing, offer, observedAt));
        changes.push(`${source.listing_id}: ${offer.price_currency} ${offer.price_amount} · ${offer.availability_label}`);
      } catch (error) {
        failures.push({ listing_id: source.listing_id, error: error.message });
        const next = expireCurrentAvailability(listing, observedAt, error.message);
        if (next !== listing) { listings.set(source.listing_id, next); expired.push(source.listing_id); }
      }
    }
    for (const [id, listing] of listings) {
      const next = expireCurrentAvailability(listing, observedAt, null);
      if (next !== listing) { listings.set(id, next); expired.push(id); }
    }
    const output = {
      ...document,
      ...(changes.length ? { verified_at: observedAt } : {}),
      refresh: {
        attempted_at: observedAt,
        succeeded: changes.length,
        failed: failures.length,
        expired_current_availability: [...new Set(expired)].length,
        errors: failures.map((failure) => ({ listing_id: failure.listing_id, error: String(failure.error).slice(0, 240) })),
      },
      listings: document.listings.map((row) => listings.get(row.id)),
    };
    if (!DRY_RUN) {
      const tmp = `${LISTINGS}.tmp-${process.pid}`;
      await writeFile(tmp, `${JSON.stringify(output, null, 2)}\n`, { mode: 0o644 });
      await rename(tmp, LISTINGS);
    }
    console.log(`${DRY_RUN ? "Checked" : "Updated"} ${changes.length}/${config.sources.length} approved variants; ${failures.length} failures; ${[...new Set(expired)].length} current availability claims expired.`);
    for (const line of changes) console.log(`ok  ${line}`);
    for (const failure of failures) console.warn(`keep ${failure.listing_id}: ${failure.error}`);
    if (!changes.length) {
      throw new Error("no approved listing passed validation; offer values were preserved and the failed refresh was recorded");
    }
  } finally {
    await lock?.close().catch(() => {});
    if (lock) await unlink(LOCK).catch(() => {});
  }
}

if (pathToFileURL(process.argv[1]).href === import.meta.url) main().catch((error) => { console.error(error.message); process.exitCode = 2; });
