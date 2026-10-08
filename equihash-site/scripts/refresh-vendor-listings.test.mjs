import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { applyOffer, assertPublicHttps, parseOffer, wooVariations } from "./refresh-vendor-listings.mjs";

const base = {
  required_tokens: ["Z15", "Pro"], currency: "USD", availability: "preorder",
  availability_label: "Configured batch", min_price: 1000, max_price: 30000,
};

test("ADE parser selects the configured product block", () => {
  const html = '<h1>Antminer Z15 Pro</h1><div id="product1" data-excl-price="12999">860K Ships out in 7 days</div><div id="product2" data-excl-price="9599">840K December</div>';
  const result = parseOffer(html, { ...base, parser: "ade_product", block_id: "product2", currency: "EUR" });
  assert.equal(result.price_amount, 9599);
  assert.equal(result.availability_label, "Configured batch");
});

test("WooCommerce parser decodes and uniquely matches a variation", () => {
  const variants = JSON.stringify([
    { attributes: { attribute_delivery: "In Stock -840K-New" }, display_price: 13400, availability_html: "<p>In stock</p>" },
    { attributes: { attribute_delivery: "Mar Batch-840K Mix" }, display_price: 8200 },
  ]).replaceAll("&", "&amp;").replaceAll('"', "&quot;");
  const html = `<h1>Bitmain Antminer Z15 Pro</h1><form data-product_variations="${variants}"></form>`;
  assert.equal(wooVariations(html).length, 2);
  const result = parseOffer(html, { ...base, parser: "woo_variation", variant_match: "Mar Batch-840K Mix" });
  assert.equal(result.price_amount, 8200);
});

test("Shopify parser selects an exact named batch", () => {
  const html = '<h1>Antminer Z15 Pro</h1><script>{"variants":[{"public_title":"In Stock","price":14399},{"public_title":"April 2027","price":5299}]}</script>';
  const result = parseOffer(html, { ...base, parser: "shopify_variant", variant_match: "April 2027" });
  assert.equal(result.price_amount, 5299);
});

test("Hashlabs parser binds price to the configured facility vicinity", () => {
  const html = '<h1>Antminer Z15 Pro</h1><div class="deal-option">hosting USA <script>{"price":10498}</script> $0.069/kWh</div><div>hosting Brazil {"price":10593}</div>';
  const result = parseOffer(html, { ...base, parser: "hashlabs_deal", mode: "hosting", facility_match: "USA" });
  assert.equal(result.price_amount, 10498);
});

test("MineShop batch parser keeps curated availability despite misleading schema stock", () => {
  const html = '<h1>Bitmain Antminer Z15 Pro</h1><script type="application/ld+json">{"offers":{"price":"7199.87","availability":"https://schema.org/InStock"}}</script><p>March 2027 batch shipping</p>';
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

test("applying a refresh changes observed fields and preserves editorial notes", () => {
  const before = { id: "offer", price_amount: 1, shipping_note: "Confirm serials", condition: "new" };
  const after = applyOffer(before, { price_amount: 2, price_currency: "USD", availability: "quote", availability_label: "Request an offer" }, "2026-10-08T10:00:00.000Z");
  assert.equal(after.shipping_note, "Confirm serials");
  assert.equal(after.condition, "new");
  assert.equal(after.price_amount, 2);
  assert.equal(after.observed_at, "2026-10-08T10:00:00.000Z");
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
