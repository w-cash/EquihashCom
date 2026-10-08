import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { applyOffer, assertPublicHttps, expireCurrentAvailability, parseOffer, wooVariations } from "./refresh-vendor-listings.mjs";

const base = {
  required_tokens: ["Z15", "Pro"], currency: "USD", availability: "preorder",
  availability_label: "Configured batch", min_price: 1000, max_price: 30000,
};

test("ADE parser selects the configured product block", () => {
  const html = '<meta property="product:price:currency" content="EUR"><h1>Antminer Z15 Pro</h1><div id="product1" data-excl-price="12999">860K Ships out in 7 days</div><div id="product2" data-excl-price="9599">840K December 2026 batch</div>';
  const result = parseOffer(html, { ...base, parser: "ade_product", block_id: "product2", currency: "EUR" });
  assert.equal(result.price_amount, 9599);
  assert.equal(result.availability_label, "Configured batch");
});

test("WooCommerce parser decodes and uniquely matches a variation", () => {
  const variants = JSON.stringify([
    { attributes: { attribute_delivery: "In Stock -840K-New" }, display_price: 13400, availability_html: "<p>In stock</p>" },
    { attributes: { attribute_delivery: "Mar Batch-840K Mix" }, display_price: 8200 },
  ]).replaceAll("&", "&amp;").replaceAll('"', "&quot;");
  const html = `<script>var store={"currency":"USD"}</script><h1>Bitmain Antminer Z15 Pro</h1><form data-product_variations="${variants}"></form>`;
  assert.equal(wooVariations(html).length, 2);
  const result = parseOffer(html, { ...base, parser: "woo_variation", variant_match: "Mar Batch-840K Mix" });
  assert.equal(result.price_amount, 8200);
});

test("Shopify parser selects an exact named batch", () => {
  const html = '<h1>Antminer Z15 Pro</h1><script>{"variants":[{"public_title":"In Stock","price":14399,"available":true,"currencyCode":"USD"},{"public_title":"April 2027","price":5299,"available":true,"currencyCode":"USD"}]}</script>';
  const result = parseOffer(html, { ...base, parser: "shopify_variant", variant_match: "April 2027" });
  assert.equal(result.price_amount, 5299);
});

test("Hashlabs parser binds price to the configured facility vicinity", () => {
  const html = '<h1>Antminer Z15 Pro</h1><div class="deal-option">hosting USA request an offer <script>{"price":10498}</script> $0.069/kWh</div><div>hosting Brazil {"price":10593}</div>';
  const result = parseOffer(html, { ...base, parser: "hashlabs_deal", mode: "hosting", facility_match: "USA", availability: "quote" });
  assert.equal(result.price_amount, 10498);
});

test("MineShop batch parser keeps curated availability despite misleading schema stock", () => {
  const html = '<h1>Bitmain Antminer Z15 Pro</h1><script type="application/ld+json">{"offers":{"price":"7199.87","priceCurrency":"EUR","availability":"https://schema.org/InStock"}}</script><p>March 2027 batch shipping</p>';
  const result = parseOffer(html, { ...base, parser: "mineshop_batch", batch_match: "March 2027", currency: "EUR", availability_label: "March 2027 batch — not immediate stock" });
  assert.equal(result.price_amount, 7199.87);
  assert.equal(result.availability, "preorder");
  assert.match(result.availability_label, /not immediate stock/);
});

test("source URL guard blocks local and non-HTTPS targets", () => {
  assert.throws(() => assertPublicHttps("http://vendor.example/product"));
  assert.throws(() => assertPublicHttps("https://127.0.0.1/product"));
  assert.throws(() => assertPublicHttps("https://192.168.1.2/product"));
  assert.equal(assertPublicHttps("https://vendor.example/product").hostname, "vendor.example");
});

test("applying a fully evidenced refresh uses independent field clocks and preserves editorial notes", () => {
  const before = { id: "offer", price_amount: 1, shipping_note: "Confirm serials", condition: "new" };
  const after = applyOffer(before, { price_amount: 2, price_currency: "USD", availability: "quote", availability_label: "Request an offer" }, "2026-10-08T10:00:00.000Z");
  assert.equal(after.shipping_note, "Confirm serials");
  assert.equal(after.condition, "new");
  assert.equal(after.price_amount, 2);
  assert.equal(after.price_observed_at, "2026-10-08T10:00:00.000Z");
  assert.equal(after.availability_observed_at, "2026-10-08T10:00:00.000Z");
  assert.equal(after.observed_at, "2026-10-08T10:00:00.000Z");
});

test("a price-only refresh cannot make manually checked stock look newer", () => {
  const before = {
    id: "offer", price_amount: 1, price_currency: "USD", availability: "in_stock",
    availability_label: "Seller said in stock", availability_observed_at: "2026-10-01T10:00:00Z",
    observed_at: "2026-10-01T10:00:00Z",
  };
  const after = applyOffer(before, { price_amount: 2, price_currency: "USD" }, "2026-10-08T10:00:00Z");
  assert.equal(after.price_observed_at, "2026-10-08T10:00:00Z");
  assert.equal(after.availability_observed_at, "2026-10-01T10:00:00Z");
  assert.equal(after.observed_at, "2026-10-01T10:00:00Z");
  assert.equal(after.availability, "in_stock");
});

test("contradictory USD and sold-out evidence cannot publish configured GBP in-stock fields", () => {
  const html = `<h1>Antminer Z15 Pro</h1><script type="application/ld+json">${JSON.stringify({
    "@type": "Product", name: "Antminer Z15 Pro", offers: {
      price: "5000", priceCurrency: "USD", availability: "https://schema.org/OutOfStock",
    },
  })}</script>`;
  assert.throws(
    () => parseOffer(html, { ...base, parser: "generic_product", name_match: "Z15 Pro", currency: "GBP", availability: "in_stock" }),
    /currency evidence contradicts GBP/,
  );
});

test("a structured generic parser binds the price to the named product instead of the first price", () => {
  const html = `
    <h1>Antminer Z15 Pro</h1>
    <script type="application/ld+json">${JSON.stringify({ "@graph": [
      { "@type": "Product", name: "Other ASIC", offers: { price: "1999", priceCurrency: "USD", availability: "https://schema.org/InStock" } },
      { "@type": "Product", name: "Antminer Z15 Pro", offers: { price: "12999", priceCurrency: "USD", availability: "https://schema.org/InStock" } },
    ] })}</script>`;
  const result = parseOffer(html, { ...base, parser: "generic_product", name_match: "Z15 Pro", availability: "in_stock", availability_label: "In stock" });
  assert.equal(result.price_amount, 12999);
});

test("an exact variant with sold-out evidence rejects a configured in-stock state", () => {
  const variants = JSON.stringify([{
    attributes: { attribute_delivery: "840K" }, display_price: 8200,
    availability_html: "<p>Sold out</p>", is_in_stock: false,
  }]).replaceAll("&", "&amp;").replaceAll('"', "&quot;");
  const html = `<script>var store={"currency":"USD"}</script><h1>Bitmain Antminer Z15 Pro</h1><form data-product_variations="${variants}"></form>`;
  assert.throws(
    () => parseOffer(html, { ...base, parser: "woo_variation", variant_match: "840K", availability: "in_stock" }),
    /availability evidence contradicts in_stock: sold_out/,
  );
});

test("a seller backorder remains backorder instead of being upgraded to configured preorder", () => {
  const variants = JSON.stringify([{
    attributes: { attribute_delivery: "March 2027 batch" }, display_price: 8200,
    availability_html: "<p>Available on backorder</p>", is_in_stock: true,
  }]).replaceAll("&", "&amp;").replaceAll('"', "&quot;");
  const html = `<script>var store={"currency":"USD"}</script><h1>Bitmain Antminer Z15 Pro</h1><form data-product_variations="${variants}"></form>`;
  const result = parseOffer(html, { ...base, parser: "woo_variation", variant_match: "March 2027 batch" });
  assert.equal(result.availability, "backorder");
});

test("failed or stale current availability becomes historical unknown without changing its evidence date", () => {
  const current = {
    availability: "in_stock", availability_label: "Seller said in stock",
    availability_observed_at: "2026-10-08T08:00:00Z", observed_at: "2026-10-08T08:00:00Z",
  };
  const failed = expireCurrentAvailability(current, "2026-10-08T09:00:00Z", "HTTP 503");
  assert.equal(failed.availability, "unknown");
  assert.equal(failed.last_known_availability, "in_stock");
  assert.equal(failed.availability_observed_at, "2026-10-08T08:00:00Z");
  assert.equal(failed.availability_refresh_error_at, "2026-10-08T09:00:00Z");

  const stale = expireCurrentAvailability(current, "2026-10-10T00:00:00Z", null);
  assert.equal(stale.availability, "unknown");
  const stillCurrent = expireCurrentAvailability(current, "2026-10-08T09:00:00Z", null);
  assert.equal(stillCurrent, current);
});

test("future batches survive current-stock expiry because their dated state is not spot availability", () => {
  const batch = { availability: "preorder", availability_label: "March 2027 batch", availability_observed_at: "2026-01-01T00:00:00Z" };
  assert.equal(expireCurrentAvailability(batch, "2026-10-08T10:00:00Z", "HTTP 503"), batch);
});

test("the approved-source allowlist names unique real listing records", async () => {
  const [config, document] = await Promise.all([
    readFile(new URL("../data/curated/vendor-listing-sources.json", import.meta.url), "utf8").then(JSON.parse),
    readFile(new URL("../data/listings.json", import.meta.url), "utf8").then(JSON.parse),
  ]);
  const listingIds = new Set(document.listings.map((row) => row.id));
  const sourceIds = config.sources.map((source) => source.listing_id);
  assert.equal(new Set(sourceIds).size, sourceIds.length);
  assert.ok(config.sources.length >= 20);
  for (const source of config.sources) {
    assert.ok(listingIds.has(source.listing_id), source.listing_id);
    assert.equal(assertPublicHttps(source.url).protocol, "https:");
    assert.ok(source.min_price < source.max_price);
  }
});
