// Tests for the pure parts of the logo fetcher. Run: node --test scripts/
import test from "node:test";
import assert from "node:assert/strict";
import { sanitizeSvg, svgLooksSafe, sniff, monogram, operatorDomain, iconCandidates, namesACoin, score, normaliseSvgRoot, icoLargest, similarity } from "./fetch-logos.mjs";

test("sanitizeSvg strips scripts, handlers, foreignObject, images and external references", () => {
  const dirty = `<?xml version="1.0"?><!-- c --><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink" xmlns:inkscape="x" inkscape:version="1" viewBox="0 0 24 24" onload="alert(1)" width="24">
    <script>alert(1)</script><script/>
    <foreignObject><div xmlns="http://www.w3.org/1999/xhtml">x</div></foreignObject>
    <image href="https://evil.example/t.png"/><feImage xlink:href="https://evil.example/x"/>
    <a href="javascript:alert(1)"><path d="M0 0h24v24z" fill="#f4b728" onclick="x()" ONMOUSEOVER='y()'/></a>
    <use href="https://evil.example/s.svg#a"/><use xlink:href="#ok"/>
    <rect fill="url(https://evil.example/f)" style="fill:url('http://x/y');stroke:url(#g)" width="1" height="1"/>
    <path d="M1 1" href="&#106;avascript:alert(1)" data-src="data:text/html,x"/>
    <animate attributeName="href" to="javascript:alert(1)"/><set attributeName="onload" to="x"/>
    <style>@import url(https://evil.example/a.css); .a{fill:url(https://e/x)} .b{fill:red}</style>
    <metadata><rdf:RDF/></metadata><sodipodi:namedview/>
  </svg>`;
  const s = sanitizeSvg(dirty);
  assert.ok(s, "a usable SVG remains");
  for (const bad of ["<script", "onload", "onclick", "onmouseover", "foreignObject", "<image", "feImage", "evil.example", "javascript", "<animate", "<set", "@import", "inkscape", "sodipodi", "metadata", "<a ", "<?xml", "<!--", "data:"]) {
    assert.ok(!s.toLowerCase().includes(bad.toLowerCase()), `${bad} survived: ${s}`);
  }
  assert.match(s, /^<svg xmlns="http:\/\/www\.w3\.org\/2000\/svg"/);
  assert.ok(s.includes('xlink:href="#ok"') && s.includes("url(#g)") && s.includes('fill="#f4b728"'), s);
  assert.ok(s.includes("<path"), "the <a> wrapper is dropped, its drawing kept");
});

test("sanitizeSvg refuses entities, non-SVG roots and empty drawings", () => {
  assert.equal(sanitizeSvg(`<!DOCTYPE svg [<!ENTITY x "y">]><svg><path d="M0 0"/></svg>`), null);
  assert.equal(sanitizeSvg(`<html><svg><path d="M0 0"/></svg></html>`), null);
  assert.equal(sanitizeSvg(`<svg xmlns="http://www.w3.org/2000/svg"><script>x</script></svg>`), null);
  assert.equal(sanitizeSvg(`<svg><path d="M0 0"/> < </svg>`), null, "malformed markup");
});

test("sanitised output passes the same check the server runs", () => {
  const s = normaliseSvgRoot(sanitizeSvg(`<svg width="64" height="32"><rect width="64" height="32" fill="#0a0a0a"/><path d="M10 17l7 31" stroke="#00ff00"/></svg>`));
  assert.ok(svgLooksSafe(s), s);
  assert.match(s, /viewBox="0 0 64 32"/);
  assert.ok(!/\swidth="64"/.test(s.match(/<svg[^>]*>/)[0]), "fixed size removed from the root");
  for (const bad of [`<svg onload="x"/>`, `<svg><use href="http://x/y#a"/></svg>`, `<svg><style>.a{}</style></svg>`, `<svg><path fill="url(http://x)"/></svg>`, `<html/>`]) assert.equal(svgLooksSafe(bad), false, bad);
});

test("file type comes from the bytes", () => {
  assert.equal(sniff(Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])), "png");
  assert.equal(sniff(Buffer.from("RIFF\0\0\0\0WEBPVP8 ")), "webp");
  assert.equal(sniff(Buffer.from([0, 0, 1, 0, 1, 0])), "ico");
  assert.equal(sniff(Buffer.from(`\uFEFF<?xml version="1.0"?>\n<!-- x --><svg viewBox="0 0 1 1"/>`)), "svg");
  assert.equal(sniff(Buffer.from("<!doctype html><html><svg/></html>")), null, "an HTML page served as favicon.svg is not an image");
  assert.equal(sniff(Buffer.from("GIF89a")), "gif");
});

test("monograms match the server's", () => {
  // Same cases as src/views/logo.rs `monograms`.
  for (const [name, m] of [["himpool.com (solo)", "HP"], ["ViaBTC", "VB"], ["zpool", "ZP"], ["Mining-Dutch", "MD"], ["2Miners", "2M"], ["pool.excc.co", "E"], ["195.3.222.105", "IP"], ["rockpool.cloud", "RP"], ["Binance Pool", "BP"], ["F2Pool", "FP"], ["coolmine.top", "CM"], ["Pirate Chain", "PC"], ["Zcash", "Z"]]) {
    assert.equal(monogram(name), m, name);
  }
});

test("operator domains group a pool's subdomains", () => {
  assert.equal(operatorDomain("https://zec.2miners.com"), "2miners.com");
  assert.equal(operatorDomain("https://solo-btg.2miners.com/x"), "2miners.com");
  assert.equal(operatorDomain("http://buckpool.myvnc.com"), "buckpool.myvnc.com", "dynamic DNS is not one operator");
  assert.equal(operatorDomain("http://195.3.222.105:3380"), "195.3.222.105");
  assert.equal(operatorDomain("not a url"), null);
});

test("icon discovery reads quoted data: URIs and manifest/apple-touch links", () => {
  const html = `<link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em'>x</text></svg>"><link rel="apple-touch-icon" sizes="180x180" href="/a.png"><link rel=manifest href=/m.json><meta property="og:image" content="https://x.example/og.png"><img class="site-logo" src="/logo.svg">`;
  const c = iconCandidates(html, "https://pool.example/");
  assert.equal(c[0].via, "icon");
  assert.ok(c[0].url.startsWith("data:image/svg+xml,<svg"), c[0].url);
  assert.deepEqual(c.slice(1).map((x) => [x.via, x.url]), [["apple-touch-icon", "https://pool.example/a.png"], ["manifest", "https://pool.example/m.json"], ["og:image", "https://x.example/og.png"], ["img.logo", "https://pool.example/logo.svg"]]);
});

test("a pool's coin images are not its logo; wordmarks and tiny icons lose", () => {
  const words = new Set(["coin", "bitcoin", "btc", "zcash", "kerrigan"]);
  assert.ok(namesACoin("https://placepool.pro/_next/static/media/bitcoin-btc-logo.c97719d7.svg", words));
  assert.ok(namesACoin("https://zec.molepool.com/coin.png", words));
  assert.ok(!namesACoin("https://himpool.com/assets/brand/himpool-gold-gradient-v1.png", words));
  assert.equal(score({ type: "svg", w: 300, h: 60, via: "img.logo", kind: "official" }), -1, "wordmark");
  assert.ok(score({ type: "png", w: 16, h: 16, via: "icon", kind: "official" }) < 0, "16px favicon");
  assert.ok(score({ type: "png", w: 180, h: 180, via: "apple-touch-icon", kind: "official" }) > score({ type: "svg", w: 1, h: 1, via: "repo", kind: "repo" }), "official beats repo");
  assert.ok(score({ type: "svg", w: 1, h: 1, via: "repo", kind: "repo" }) > score({ type: "png", w: 256, h: 256, via: "coingecko", kind: "third_party" }), "repo beats CoinGecko");
});

test("ICO: the largest PNG entry is used", () => {
  const png = Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), Buffer.alloc(16)]);
  const hdr = Buffer.alloc(6 + 32);
  hdr.writeUInt16LE(1, 2); hdr.writeUInt16LE(2, 4);
  // entry 1: 16x16 at offset 38, entry 2: 64x64 at offset 38 too (same payload is fine for the test)
  hdr[6] = 16; hdr[7] = 16; hdr.writeUInt32LE(png.length, 6 + 8); hdr.writeUInt32LE(38, 6 + 12);
  hdr[22] = 64; hdr[23] = 64; hdr.writeUInt32LE(png.length, 22 + 8); hdr.writeUInt32LE(38, 22 + 12);
  const r = icoLargest(Buffer.concat([hdr, png]));
  assert.equal(r.w, 64);
  assert.ok(r.png.equals(png));
});

test("shared-mark similarity: the same mark scores 1, a different one far less", () => {
  const a = [10, -10, 20, -20, 0, 0], b = a.map((x) => x * 0.5), c = [-10, 10, 0, 0, 20, -20];
  assert.equal(similarity(a, a), 1);
  assert.ok(similarity(a, b) > 0.999, "a lighter copy of the same mark");
  assert.ok(similarity(a, c) < 0.5);
  assert.equal(similarity(a, [1, 2]), 0, "different sizes never match");
  assert.equal(similarity([0, 0], [0, 0]), 0, "blank images never match");
});
