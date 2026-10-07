#!/usr/bin/env node
/**
 * equihash.com logo fetcher (dev tool, NOT part of the hourly refresh).
 *
 * Finds a logo for every coin (data/network.json + data/curated/coins.json) and every pool
 * (data/pools.json + data/curated/manual-pools.json + data/archive.json), keeps a sanitised,
 * optimised local copy in static/logos/{coins,pools}/ and records where it came from in
 * data/curated/logos.json. The site only ever serves these local copies (no hotlinking).
 *
 * Source order, best first. Nothing is ever drawn or invented: if no real logo can be fetched,
 * the entry is a generated monogram and is marked `kind: "fallback"`.
 *   coins: the coin's own website (data/curated/links.json, kind "website": <link rel=icon>,
 *          apple-touch-icon, web manifest icons, square og:image or header logo) and its
 *          official GitHub org avatar           -> kind "official"
 *          cryptocurrency-icons (CC0-1.0), matched by coin name and symbol -> kind "repo"
 *          CoinGecko image for an exact coin id (COINGECKO below)          -> kind "third_party"
 *   pools: the pool's own site (pool URL, then the operator's root domain) -> kind "official"
 *
 * Usage:
 *   node scripts/fetch-logos.mjs                 # only coins/pools with no entry yet (or a missing file)
 *   node scripts/fetch-logos.mjs --retry-fallbacks
 *   node scripts/fetch-logos.mjs --force         # refetch everything
 *   node scripts/fetch-logos.mjs --id zcash --id himpool.com --force
 *   node scripts/fetch-logos.mjs --dry-run       # report, write nothing
 *
 * Needs Node 18+. `sharp` (PNG/ICO/JPEG -> WebP, resizing, trimming) and `svgo` (SVG
 * optimisation) are optional: `npm i --no-save sharp svgo`, or NODE_PATH=<dir>/node_modules.
 * Without sharp, raster sources are only kept when they are already a PNG/WebP of the right size.
 * SVG sanitising is done here with no dependency and always runs, last, on every SVG written.
 */
import fs from "node:fs/promises";
import path from "node:path";
import crypto from "node:crypto";
import { createRequire } from "node:module";
import { fileURLToPath, pathToFileURL } from "node:url";

const ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const DATA = path.join(ROOT, "data");
const LOGOS_DIR = path.join(ROOT, "static", "logos");
const LOGOS_JSON = path.join(DATA, "curated", "logos.json");
const UA = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0 Safari/537.36 equihash.com-logo-fetch";
const MAX_BYTES = 2_000_000;
const TARGET_PX = 96; // longest side of a raster logo (shown at 16-40 CSS px, so 2x-retina sharp)
const MAX_FILE = 10 * 1024; // aim; files above this are re-encoded smaller

// Exact CoinGecko coin ids (last resort, kind "third_party"). The API answer must also carry the
// coin's symbol, so a wrong id can't attach another coin's logo.
export const COINGECKO = {
  zcash: "zcash", piratechain: "pirate-chain", komodo: "komodo", bitcoingold: "bitcoin-gold",
  horizen: "zencash", flux: "zelcash", bitcoinz: "bitcoinz", zclassic: "zclassic", ycash: "ycash",
  hush: "hush", aion: "aion", litecoinz: "litecoinz", gemlink: "gemlink", zero: "zero",
  anonymous: "anon", tokel: "tokel", marmara: "marmara-chain", "bitmark-equihash": "bitmark",
  exchangecoin: "exchangecoin", bitcoincandy: "bitcoin-candy", vdimension: "vollar",
};
// Hand-picked official assets that beat what auto-discovery finds (the brand's own file).
// Keys: coin id or pool operator domain. Values: list of URLs, tried in order.
export const PINNED = {
  // Zcash: the coin mark from z.cash (its second website, zfnd.org, would give the Foundation's logo).
  zcash: ["https://z.cash/wp-content/uploads/2023/03/zcash-logo.svg"],
};
// Owner-supplied marks are kept during a broad --force refresh. To replace one deliberately,
// target it by id as well: --id wcash --force. The full-resolution source is in assets/brand/.
const OWNER_SUPPLIED = new Set(["wcash"]);
// Candidates auto-discovery must skip (e.g. a generic avatar, a sponsor's logo, a wordmark).
export const SKIP = {
  // Serves the same /static/logo.svg as rockpool.cloud (pool software default, not this operator's
  // own mark), so it gets a monogram rather than another pool's logo.
  "195.3.222.105": ["/static/logo.svg"],
};
// Shared-hosting and dynamic-DNS domains: the operator is the full host, not the domain.
const SHARED_HOST_DOMAINS = new Set(["myvnc.com", "duckdns.org", "ddns.net", "no-ip.org", "github.io", "herokuapp.com", "netlify.app", "vercel.app", "pages.dev"]);

// ---------------------------------------------------------------- pure helpers (tested)

/** Operator key for a pool URL: the registrable domain (zec.2miners.com -> 2miners.com). */
export function operatorDomain(url) {
  let h;
  try { h = new URL(url).hostname.toLowerCase(); } catch { return null; }
  if (/^\d+\.\d+\.\d+\.\d+$/.test(h) || h.includes(":")) return h;
  const parts = h.split(".");
  if (parts.length <= 2) return h;
  const two = parts.slice(-2).join(".");
  if (SHARED_HOST_DOMAINS.has(two)) return parts.slice(-3).join(".");
  if (/^(co|com|org|net|ac)\.[a-z]{2}$/.test(two)) return parts.slice(-3).join(".");
  return two;
}

/** Safe file stem from an id or domain. */
export function stem(s) {
  return String(s).toLowerCase().replace(/[^a-z0-9.-]+/g, "-").replace(/^[-.]+|[-.]+$/g, "").slice(0, 60) || "x";
}

/** One or two letters for a generated monogram: "himpool.com (solo)" -> "HP", "ViaBTC" -> "VB". */
export function monogram(name) {
  let s = String(name || "").replace(/\(.*?\)/g, " ").trim();
  s = s.replace(/^(https?:\/\/)?(www\.|pool\.)/i, "");
  s = s.replace(/\.(com|net|org|io|xyz|cc|pro|top|site|space|online|cloud|fr|es|nl|ca|it|tech|co|me|info|biz|app|dev)\b.*$/i, "");
  if (/^\d+(\.\d+){3}/.test(s)) return "IP";
  // CamelCase and known suffixes split compound names: rockpool -> rock pool, ViaBTC -> Via BTC.
  s = s.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/(pool|mines?|miners?|mining|hub|solo)\b/gi, " $1");
  const words = s.split(/[^A-Za-z0-9]+/).filter(Boolean);
  if (!words.length) return "?";
  if (words.length === 1) {
    const w = words[0];
    return (/^\d/.test(w) ? w.slice(0, 2) : w[0]).toUpperCase();
  }
  return (words[0][0] + words[1][0]).toUpperCase();
}

/** File type from the bytes, never from the URL or Content-Type. */
export function sniff(buf) {
  if (!buf || buf.length < 4) return null;
  const b = buf;
  if (b[0] === 0x89 && b[1] === 0x50 && b[2] === 0x4e && b[3] === 0x47) return "png";
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return "jpeg";
  if (b.slice(0, 4).toString("latin1") === "GIF8") return "gif";
  if (b.slice(0, 4).toString("latin1") === "RIFF" && b.length > 12 && b.slice(8, 12).toString("latin1") === "WEBP") return "webp";
  if (b[0] === 0 && b[1] === 0 && b[2] === 1 && b[3] === 0) return "ico";
  if (b.length > 12 && b.slice(4, 12).toString("latin1").startsWith("ftypavi")) return "avif";
  const head = b.slice(0, 4096).toString("utf8").replace(/^\uFEFF/, "");
  const t = head.replace(/<\?xml[\s\S]*?\?>/g, "").replace(/<!--[\s\S]*?-->/g, "").replace(/<!DOCTYPE[^>[]*(\[[\s\S]*?\])?>/gi, "").trimStart();
  if (/^<svg[\s>]/i.test(t)) return "svg";
  return null;
}

const ENT = { amp: "&", lt: "<", gt: ">", quot: '"', apos: "'", nbsp: "\u00a0" };
export function decodeEntities(s) {
  return String(s).replace(/&(#x[0-9a-f]+|#\d+|[a-z]+);?/gi, (m, e) => {
    if (e[0] === "#") {
      const n = e[1] === "x" || e[1] === "X" ? parseInt(e.slice(2), 16) : parseInt(e.slice(1), 10);
      return Number.isFinite(n) && n > 0 && n < 0x110000 ? String.fromCodePoint(n) : "";
    }
    return ENT[e.toLowerCase()] ?? m;
  });
}
const escAttr = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

// Elements kept. Everything else is dropped with its whole subtree (script, foreignObject, image,
// feImage, animate/set, iframe, metadata, editor namespaces...). <a> is unwrapped (children kept).
const SVG_ELEMENTS = new Set([
  "svg", "g", "path", "rect", "circle", "ellipse", "line", "polyline", "polygon", "defs", "symbol", "use",
  "lineargradient", "radialgradient", "stop", "clippath", "mask", "pattern", "text", "tspan", "textpath", "title", "desc", "style",
  "filter", "feblend", "fecolormatrix", "fecomponenttransfer", "fecomposite", "feflood", "fegaussianblur", "femerge", "femergenode",
  "femorphology", "feoffset", "fefunca", "fefuncr", "fefuncg", "fefuncb", "fedropshadow", "feturbulence", "fedisplacementmap", "fetile",
  "fespecularlighting", "fediffuselighting", "fedistantlight", "fepointlight", "fespotlight", "feconvolvematrix", "marker",
]);
const UNWRAP = new Set(["a", "switch"]);
const URL_ATTRS = new Set(["href", "xlink:href", "src", "xml:base", "action", "formaction"]);

/** Only same-document references survive: url(#id). */
function safeCss(css) {
  let s = decodeEntities(String(css)).replace(/\\/g, "");
  s = s.replace(/@import[^;]*;?/gi, "").replace(/@font-face\s*{[^}]*}/gi, "").replace(/@namespace[^;]*;?/gi, "");
  if (/expression\s*\(|behavior\s*:|-moz-binding|javascript:|<\/?\w/i.test(s)) return null;
  s = s.replace(/url\(\s*(['"]?)(.*?)\1\s*\)/gi, (m, q, u) => (u.trim().startsWith("#") ? `url(${u.trim()})` : "none"));
  return s;
}

/**
 * Sanitise an SVG with an allowlist. Drops scripts, event handlers, foreignObject, images,
 * animation, external references (href/xlink:href not starting with "#", url() to anything but
 * #id, @import), DOCTYPE/entities, comments and processing instructions. Returns the cleaned SVG
 * string, or null when the input isn't a usable SVG (no root <svg>, entities, nothing to draw).
 */
export function sanitizeSvg(input) {
  let src = String(input).replace(/^\uFEFF/, "");
  if (/<!ENTITY/i.test(src)) return null; // entity expansion tricks: refuse outright
  src = src.replace(/<\?[\s\S]*?\?>/g, "").replace(/<!--[\s\S]*?-->/g, "").replace(/<!DOCTYPE[^>[]*(\[[\s\S]*?\])?>/gi, "");
  const out = [];
  const stack = []; // {name, keep}
  let skipDepth = 0; // >0 while inside a dropped subtree
  let sawRoot = false, drawable = 0;
  const re = /<!\[CDATA\[([\s\S]*?)\]\]>|<\/\s*([A-Za-z][\w:.-]*)\s*>|<([A-Za-z][\w:.-]*)((?:\s+[^\s=>\/]+(?:\s*=\s*(?:"[^"]*"|'[^']*'|[^\s>]+))?)*)\s*(\/?)>|([^<]+)|(<)/g;
  let m;
  while ((m = re.exec(src))) {
    const [, cdata, close, open, attrs, selfClose, text, stray] = m;
    if (stray) return null; // malformed markup
    const inStyle = stack.length && stack[stack.length - 1].name === "style" && stack[stack.length - 1].keep;
    if (cdata !== undefined || text !== undefined) {
      if (skipDepth) continue;
      const t = cdata !== undefined ? cdata : text;
      if (inStyle) { const c = safeCss(t); if (c === null) return null; out.push(c.replace(/</g, "")); }
      else if (stack.length) out.push(cdata !== undefined ? escAttr(t).replace(/"/g, "&quot;") : t.replace(/&(?!(#\d+|#x[0-9a-f]+|amp|lt|gt|quot|apos);)/gi, "&amp;"));
      continue;
    }
    if (close) {
      const name = close.toLowerCase();
      // pop to the matching element
      let i = stack.length - 1;
      while (i >= 0 && stack[i].name !== name) i--;
      if (i < 0) continue;
      while (stack.length > i) {
        const e = stack.pop();
        if (e.keep === "drop") skipDepth--;
        else if (e.keep === true) out.push(`</${e.tag}>`);
      }
      continue;
    }
    // open tag
    const tag = open.replace(/^svg:/i, "");
    const name = tag.toLowerCase();
    const isSelf = selfClose === "/";
    if (skipDepth) { if (!isSelf) { stack.push({ name, keep: "drop" }); skipDepth++; } continue; }
    if (!sawRoot && name !== "svg") return null;
    let keep = SVG_ELEMENTS.has(name) && !name.includes(":") ? true : UNWRAP.has(name) ? "unwrap" : "drop";
    if (keep === "drop") { if (!isSelf) { stack.push({ name, keep: "drop" }); skipDepth++; } continue; }
    if (keep === "unwrap") { if (!isSelf) stack.push({ name, keep: "unwrap" }); continue; }
    const isRoot = !sawRoot;
    sawRoot = true;
    if (!["svg", "defs", "title", "desc", "style", "g", "symbol", "stop", "lineargradient", "radialgradient", "clippath", "mask", "filter"].includes(name) && !name.startsWith("fe")) drawable++;
    const kept = [];
    const ar = /([^\s=>\/]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?/g;
    let a;
    const seen = new Set();
    while ((a = ar.exec(attrs || ""))) {
      const an = a[1];
      const al = an.toLowerCase();
      const raw = a[2] ?? a[3] ?? a[4] ?? "";
      const v = decodeEntities(raw);
      if (seen.has(al)) continue;
      seen.add(al);
      if (al.startsWith("on")) continue; // event handlers
      if (!/^[a-z_][a-z0-9_.:-]*$/i.test(an)) continue;
      if (al.startsWith("xmlns")) { if (al === "xmlns" || al === "xmlns:xlink") kept.push(`${al}="${al === "xmlns" ? "http://www.w3.org/2000/svg" : "http://www.w3.org/1999/xlink"}"`); continue; }
      if (al.includes(":") && !["xlink:href", "xml:space"].includes(al)) continue; // inkscape:, sodipodi:, xml:base...
      if (URL_ATTRS.has(al)) { if (/^#[\w.:-]+$/.test(v.trim()) && (al === "href" || al === "xlink:href")) kept.push(`${an}="${escAttr(v.trim())}"`); continue; }
      if (/javascript:|vbscript:|data:/i.test(v.replace(/\s+/g, ""))) continue;
      let val = v;
      if (al === "style" || /url\(/i.test(v)) { const c = safeCss(v); if (c === null) continue; val = c; }
      kept.push(`${an}="${escAttr(val)}"`);
    }
    if (isRoot && !kept.some((k) => k.startsWith("xmlns="))) kept.unshift('xmlns="http://www.w3.org/2000/svg"');
    out.push(`<${tag}${kept.length ? " " + kept.join(" ") : ""}${isSelf ? "/" : ""}>`);
    if (!isSelf) stack.push({ name, tag, keep: true });
  }
  while (stack.length) { const e = stack.pop(); if (e.keep === true) out.push(`</${e.tag}>`); }
  if (!sawRoot || drawable === 0) return null;
  // Drop the now-unused xlink namespace declaration's dependency issues: keep it only if used.
  let s = out.join("").replace(/>\s+</g, "><").trim();
  if (!/xlink:/.test(s.replace(/xmlns:xlink="[^"]*"/, ""))) s = s.replace(/\s?xmlns:xlink="[^"]*"/, "");
  else if (!/xmlns:xlink=/.test(s)) s = s.replace(/^<svg/, '<svg xmlns:xlink="http://www.w3.org/1999/xlink"');
  return s;
}

/** Same checks the server runs before it serves an SVG logo (see src/views/logo.rs). */
export function svgLooksSafe(s) {
  const l = String(s).toLowerCase();
  if (!/^<svg[\s>]/.test(l.trimStart())) return false;
  // Mirrors svg_is_safe in src/views/logo.rs. <style> is refused there too (writeLogo rasterises
  // an SVG that still needs a stylesheet after svgo).
  if (/<script|foreignobject|<image|<feimage|<iframe|<embed|<object|<!entity|<!doctype|<\?|javascript:|vbscript:|@import|<set[\s>]|<animate|<a[\s>]|<style/.test(l)) return false;
  if (/\son[a-z]+\s*=/.test(l)) return false;
  for (const m of l.matchAll(/(?:href|src)\s*=\s*["']?\s*([^"'\s>]*)/g)) if (!m[1].startsWith("#")) return false;
  for (const m of l.matchAll(/url\(\s*['"]?\s*([^)'"]*)/g)) if (!m[1].startsWith("#")) return false;
  return true;
}

/** width/height of an SVG from its viewBox (or width/height attributes). */
export function svgSize(s) {
  const root = (String(s).match(/<svg\b[^>]*>/i) || [""])[0];
  const vb = root.match(/viewBox\s*=\s*["']\s*([-\d.eE]+)[\s,]+([-\d.eE]+)[\s,]+([\d.eE]+)[\s,]+([\d.eE]+)/i);
  if (vb) return { w: +vb[3], h: +vb[4], viewBox: true };
  const w = root.match(/\bwidth\s*=\s*["']?([\d.]+)(px)?["'\s>]/i), h = root.match(/\bheight\s*=\s*["']?([\d.]+)(px)?["'\s>]/i);
  if (w && h) return { w: +w[1], h: +h[1], viewBox: false };
  return null;
}

/** Make sure the root has a viewBox (needed to scale inside <img>), and drop fixed width/height. */
export function normaliseSvgRoot(s) {
  const size = svgSize(s);
  if (!size) return null;
  return s.replace(/<svg\b[^>]*>/i, (root) => {
    let r = root;
    if (!size.viewBox) r = r.replace(/<svg/i, `<svg viewBox="0 0 ${size.w} ${size.h}"`);
    return r.replace(/\s(width|height)\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)/gi, "");
  });
}

function attrsOf(tag) {
  const o = {};
  for (const a of tag.matchAll(/([^\s=<>\/]+)\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+))/g)) o[a[1].toLowerCase()] = decodeEntities(a[2] ?? a[3] ?? a[4] ?? "");
  return o;
}

/** Icon candidates named in a page's HTML: <link rel=icon/apple-touch-icon/mask-icon/manifest>, og:image, header logos. */
export function iconCandidates(html, base) {
  const out = [];
  const abs = (u) => { try { return new URL(u.trim(), base).href; } catch { return null; } };
  const head = String(html).slice(0, 400_000);
  const TAG = (name) => new RegExp(`<${name}\\b(?:[^>"']|"[^"]*"|'[^']*')*>`, "gi");
  for (const m of head.matchAll(TAG("link"))) {
    const a = attrsOf(m[0]);
    const rel = (a.rel || "").toLowerCase();
    if (!a.href) continue;
    const u = abs(a.href);
    if (!u) continue;
    const sizes = (a.sizes || "").toLowerCase();
    const px = sizes === "any" ? 512 : Math.max(0, ...sizes.split(/\s+/).map((s) => parseInt(s, 10) || 0));
    if (/apple-touch-icon/.test(rel)) out.push({ url: u, via: "apple-touch-icon", hint: px || 180 });
    else if (/mask-icon/.test(rel)) out.push({ url: u, via: "mask-icon", hint: 0 });
    else if (/\bicon\b/.test(rel)) out.push({ url: u, via: "icon", hint: px || (/svg/.test(a.type || "") || /\.svg(\?|$)|^data:image\/svg/i.test(u) ? 512 : 0) });
    else if (/manifest/.test(rel)) out.push({ url: u, via: "manifest", hint: 0 });
  }
  for (const m of head.matchAll(TAG("meta"))) {
    const a = attrsOf(m[0]);
    const k = (a.property || a.name || "").toLowerCase();
    if ((k === "og:image" || k === "twitter:image") && a.content) { const u = abs(a.content); if (u) out.push({ url: u, via: k, hint: 0 }); }
  }
  let n = 0;
  for (const m of head.matchAll(TAG("img"))) {
    const a = attrsOf(m[0]);
    const src = a.src || a["data-src"];
    if (!src || !/logo/i.test([a.class, a.id, a.alt, src].join(" "))) continue;
    const u = abs(src);
    if (u && n++ < 4) out.push({ url: u, via: "img.logo", hint: 0 });
  }
  return out;
}

/** Find the largest image inside an ICO. Returns { buf, w, h, png: bool, bmp?: {...} }. */
export function icoLargest(buf) {
  if (sniff(buf) !== "ico") return null;
  const count = buf.readUInt16LE(4);
  let best = null;
  for (let i = 0; i < count && 6 + 16 * (i + 1) <= buf.length; i++) {
    const o = 6 + 16 * i;
    const w = buf[o] || 256, h = buf[o + 1] || 256, size = buf.readUInt32LE(o + 8), off = buf.readUInt32LE(o + 12);
    const bpp = buf.readUInt16LE(o + 6);
    if (off + size > buf.length) continue;
    if (!best || w * h > best.w * best.h || (w * h === best.w * best.h && bpp > best.bpp)) best = { w, h, bpp, data: buf.slice(off, off + size) };
  }
  if (!best) return null;
  if (sniff(best.data) === "png") return { w: best.w, h: best.h, png: best.data };
  // BMP payload (DIB header, bottom-up, double height incl. AND mask). 32 and 24 bpp only.
  const d = best.data;
  const hdr = d.readUInt32LE(0), W = d.readInt32LE(4), H = Math.abs(d.readInt32LE(8)) / 2, bits = d.readUInt16LE(14);
  if (bits !== 32 && bits !== 24) return null;
  const stride = Math.ceil((W * bits) / 32) * 4, maskStride = Math.ceil(W / 32) * 4;
  const px = hdr, maskOff = px + stride * H;
  const rgba = Buffer.alloc(W * H * 4);
  let anyAlpha = false;
  for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const s = px + (H - 1 - y) * stride + x * (bits / 8);
    const t = (y * W + x) * 4;
    rgba[t] = d[s + 2]; rgba[t + 1] = d[s + 1]; rgba[t + 2] = d[s];
    rgba[t + 3] = bits === 32 ? d[s + 3] : 255;
    if (bits === 32 && d[s + 3]) anyAlpha = true;
  }
  if (bits === 24 || !anyAlpha) for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
    const mo = maskOff + (H - 1 - y) * maskStride + (x >> 3);
    const masked = mo < d.length && (d[mo] >> (7 - (x & 7))) & 1;
    rgba[(y * W + x) * 4 + 3] = masked ? 0 : 255;
  }
  return { w: W, h: H, raw: rgba };
}

/** A pool page's image named after a coin ("coin.png", "bitcoin-btc-logo.svg") is a coin logo, not the pool's. */
export function namesACoin(url, coinWords) {
  let p;
  try { p = url.startsWith("data:") ? "" : decodeURIComponent(new URL(url).pathname).toLowerCase(); } catch { return false; }
  const file = p.split("/").pop() || "";
  const tokens = file.replace(/\.[a-z0-9]+$/, "").split(/[^a-z0-9]+/).filter(Boolean);
  return tokens.some((t) => coinWords.has(t));
}
export const COMMON_COIN_WORDS = ["coin", "coins", "bitcoin", "btc", "ethereum", "eth", "litecoin", "ltc", "monero", "xmr", "doge", "dogecoin", "kaspa", "kas", "zcash", "zec", "flag", "usdt", "tether"];

/** How good a candidate is (higher is better). Square-ish only; a wordmark can't sit in a 20px chip. */
export function score(c) {
  if (!c || !c.w || !c.h) return -1;
  const aspect = Math.max(c.w, c.h) / Math.min(c.w, c.h);
  if (aspect > 1.6) return -1;
  const viaBonus = { pinned: 1000, "apple-touch-icon": 6, icon: 5, manifest: 5, "mask-icon": -15, "favicon-probe": 2, "img.logo": 1, "og:image": -10, "twitter:image": -12, github: 0, repo: 0, coingecko: 0 }[c.via] ?? 0;
  const tierBonus = { official: 200, repo: 100, third_party: 0 }[c.kind] ?? 0;
  let q;
  if (c.type === "svg") q = 100;
  else { const s = Math.min(c.w, c.h); q = s >= 128 ? 90 : s >= 96 ? 85 : s >= 64 ? 70 : s >= 48 ? 45 : s >= 32 ? 30 : -1000; }
  if (c.via === "github") q = 78; // avatars are fine squares, but a site's own SVG/large icon beats them
  return tierBonus + q + viaBonus - (aspect > 1.15 ? 8 : 0) - 3 * (c.order || 0);
}

// ---------------------------------------------------------------- IO

let sharp = null, svgo = null;
function loadOptional() {
  const req = createRequire(import.meta.url);
  try { sharp = req("sharp"); } catch { sharp = null; }
  try { svgo = req("svgo"); } catch { svgo = null; }
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function get(url, { accept = "*/*", timeout = 15000, tries = 2 } = {}) {
  if (url.startsWith("data:")) {
    const m = url.match(/^data:([^,;]*)((?:;[^,;]*)*),(.*)$/s);
    if (!m) return null;
    const b64 = /;base64/i.test(m[2]);
    let text = m[3];
    try { text = decodeURIComponent(m[3]); } catch { /* raw, unencoded SVG */ }
    const buf = b64 ? Buffer.from(m[3], "base64") : Buffer.from(text, "utf8");
    return { buf, url, status: 200, type: m[1] };
  }
  if (!/^https?:\/\//i.test(url)) return null;
  for (let i = 0; i < tries; i++) {
    const ac = new AbortController();
    const t = setTimeout(() => ac.abort(), timeout);
    try {
      const r = await fetch(url, { headers: { "user-agent": UA, accept, "accept-language": "en" }, redirect: "follow", signal: ac.signal });
      if (r.status === 429 && i + 1 < tries) { await sleep(8000); continue; }
      if (!r.ok) return { status: r.status, url: r.url };
      const len = +r.headers.get("content-length") || 0;
      if (len > MAX_BYTES) return { status: 413, url: r.url };
      const buf = Buffer.from(await r.arrayBuffer());
      if (buf.length > MAX_BYTES) return { status: 413, url: r.url };
      return { buf, url: r.url, status: r.status, type: r.headers.get("content-type") || "" };
    } catch (e) {
      if (i + 1 >= tries) return { status: 0, url, error: String(e.message || e) };
    } finally { clearTimeout(t); }
  }
  return null;
}

/** Download and measure a candidate. Returns a ready-to-write image or null. */
async function evaluate(cand) {
  const r = await get(cand.url, { accept: "image/svg+xml,image/png,image/webp,image/*;q=0.8,*/*;q=0.5" });
  if (!r || !r.buf) return { ...cand, error: r ? `HTTP ${r.status}${r.error ? " " + r.error : ""}` : "bad url" };
  const type = sniff(r.buf);
  const base = { ...cand, final_url: r.url.startsWith("data:") ? cand.url : r.url };
  if (!type) return { ...base, error: "not an image" };
  if (type === "svg") {
    const clean = sanitizeSvg(r.buf.toString("utf8"));
    if (!clean) return { ...base, error: "unusable SVG" };
    const norm = normaliseSvgRoot(clean);
    const size = norm && svgSize(norm);
    if (!size) return { ...base, error: "SVG without size" };
    return { ...base, type, svg: norm, w: size.w, h: size.h };
  }
  if (!sharp) {
    if (type === "png" || type === "webp") {
      const w = type === "png" ? r.buf.readUInt32BE(16) : null, h = type === "png" ? r.buf.readUInt32BE(20) : null;
      return { ...base, type, buf: r.buf, w, h, noSharp: true };
    }
    return { ...base, error: `${type} needs sharp` };
  }
  try {
    let img;
    if (type === "ico") {
      const ico = icoLargest(r.buf);
      if (!ico) return { ...base, error: "unreadable ICO" };
      img = ico.png ? sharp(ico.png) : sharp(ico.raw, { raw: { width: ico.w, height: ico.h, channels: 4 } });
    } else img = sharp(r.buf, { animated: false });
    const meta = await img.metadata();
    const raw = await img.ensureAlpha().png().toBuffer();
    return { ...base, type, raster: raw, w: meta.width, h: meta.height };
  } catch (e) {
    return { ...base, error: "decode failed: " + e.message };
  }
}

async function discover(pageUrl, githubOrgs = null) {
  const out = [];
  const r = await get(pageUrl, { accept: "text/html,application/xhtml+xml" });
  const base = r && r.buf ? r.url : pageUrl;
  if (r && r.buf && !sniff(r.buf)) {
    const html = r.buf.toString("utf8");
    if (githubOrgs) for (const m of html.matchAll(/https:\/\/github\.com\/([A-Za-z0-9-]+)(?=["'\/])/g)) if (!["sponsors", "features", "about", "login", "orgs"].includes(m[1])) githubOrgs.add(m[1]);
    for (const c of iconCandidates(html, base)) {
      if (c.via === "manifest") {
        const mr = await get(c.url, { accept: "application/manifest+json,application/json" });
        try {
          const man = JSON.parse(mr.buf.toString("utf8"));
          for (const ic of man.icons || []) {
            if (!ic.src) continue;
            const px = Math.max(0, ...String(ic.sizes || "").split(/\s+/).map((s) => parseInt(s, 10) || 0));
            out.push({ url: new URL(ic.src, mr.url).href, via: "manifest", hint: px });
          }
        } catch { /* no usable manifest */ }
      } else out.push(c);
    }
  }
  const origin = new URL(base).origin;
  for (const p of ["/favicon.svg", "/apple-touch-icon.png", "/favicon.ico"]) out.push({ url: origin + p, via: "favicon-probe", hint: 0 });
  // de-dupe, keep the first (most specific) occurrence; at most 10 per page
  const seen = new Set();
  return out.filter((c) => (seen.has(c.url) ? false : (seen.add(c.url), true))).slice(0, 12);
}

/** Write the chosen image; returns { file, bytes, w, h, format }. */
async function writeLogo(best, group, key) {
  let ext, bytes;
  if (best.svg) {
    let s = best.svg;
    if (svgo) {
      try {
        s = svgo.optimize(s, { multipass: true, plugins: [{ name: "preset-default", params: { overrides: { cleanupIds: { minify: true } } } }, "removeDimensions"] }).data;
      } catch { /* keep unoptimised */ }
    }
    s = sanitizeSvg(s); // sanitise last, always
    if (s && !/viewBox/i.test(s)) s = normaliseSvgRoot(s);
    if (!s) throw new Error("SVG unusable after optimisation");
    bytes = Buffer.from(s, "utf8");
    ext = "svg";
    // A <style> block svgo couldn't inline, or <text> (an <img> SVG can't load web fonts, so
    // emoji and lettering would render differently on every OS): rasterise once, here.
    const needsRaster = !svgLooksSafe(s) || /<text[\s>]/i.test(s);
    if (needsRaster && !sharp) throw new Error("SVG needs a stylesheet; install sharp to rasterise it");
    if (needsRaster) { bytes = await rasterToWebp(sharp(bytes, { density: 300 })); ext = "webp"; }
    // Big SVG (embedded detail): a 96px WebP is smaller and looks the same at 40px.
    else if (bytes.length > MAX_FILE && sharp) {
      const ras = await rasterToWebp(sharp(bytes, { density: 300 }));
      if (ras.length < bytes.length) { bytes = ras; ext = "webp"; }
    }
  } else if (best.raster) {
    bytes = await rasterToWebp(sharp(best.raster));
    ext = "webp";
  } else if (best.buf) {
    bytes = best.buf;
    ext = sniff(best.buf);
  }
  const hash = crypto.createHash("sha256").update(bytes).digest("hex").slice(0, 10);
  const rel = `${group}/${stem(key)}.${hash}.${ext}`;
  return { rel, bytes, ext, ...(await analyse(bytes, ext)) };
}

/**
 * How the chip should hold the logo: bg "tile" when the image is an opaque square of its own
 * (it then fills the chip), ink "light" when it is white or near-white on transparent (it then
 * sits on a dark chip, or it would vanish on the paper-coloured one).
 */
export async function analyse(bytes, ext) {
  if (!sharp) return {};
  try {
    const { data } = await sharp(bytes, ext === "svg" ? { density: 144 } : {}).resize(48, 48, { fit: "fill" }).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
    const px = (x, y) => data.slice((y * 48 + x) * 4, (y * 48 + x) * 4 + 4);
    const tile = [[4, 4], [43, 4], [4, 43], [43, 43]].every(([x, y]) => px(x, y)[3] > 240);
    let n = 0, lum = 0;
    for (let i = 0; i < data.length; i += 4) if (data[i + 3] > 128) { n++; lum += (0.2126 * data[i] + 0.7152 * data[i + 1] + 0.0722 * data[i + 2]) / 255; }
    const out = {};
    if (tile) out.bg = "tile";
    else if (n > 48 * 48 * 0.03 && lum / n > 0.86) out.ink = "light";
    return out;
  } catch { return {}; }
}

async function rasterToWebp(img) {
  // Trim transparent margins only (a logo on a solid tile keeps its tile), then fit 96px.
  const buf = await img.ensureAlpha().png().toBuffer();
  let s = sharp(buf);
  const { data } = await sharp(buf).raw().toBuffer({ resolveWithObject: true });
  if (data[3] === 0) { try { s = sharp(await sharp(buf).trim({ threshold: 1 }).png().toBuffer()); } catch { /* nothing to trim */ } }
  const m = await s.metadata();
  const fit = Math.min(TARGET_PX, Math.max(m.width, m.height));
  const resized = await s.resize({ width: fit, height: fit, fit: "inside", withoutEnlargement: true, kernel: "lanczos3" }).png().toBuffer();
  for (const q of [90, 82, 74, 64]) {
    const out = await sharp(resized).webp({ quality: q, alphaQuality: 90, effort: 6, smartSubsample: true }).toBuffer();
    if (out.length <= MAX_FILE || q === 64) return out;
  }
  return resized;
}

/**
 * Signature of a mark for spotting the same logo on two sites: margins trimmed, fitted into 24x24
 * on white, mean-centred RGB. Two signatures with a cosine similarity of SAME_MARK or more are the
 * same mark (re-encoded, resized or re-padded); different marks score well below (about 0.85 at
 * most across the current set).
 */
async function signature(bytes, ext) {
  let s = sharp(bytes, ext === "svg" ? { density: 144 } : {}).flatten({ background: "#ffffff" });
  try { s = sharp(await s.trim({ threshold: 20 }).png().toBuffer()); } catch { /* nothing to trim */ }
  const raw = await s.resize(24, 24, { fit: "contain", background: "#ffffff" }).removeAlpha().raw().toBuffer();
  const v = Array.from(raw);
  const m = v.reduce((a, b) => a + b, 0) / v.length;
  return v.map((x) => x - m);
}
export function similarity(a, b) {
  if (a.length !== b.length) return 0;
  let d = 0, x = 0, y = 0;
  for (let i = 0; i < a.length; i++) { d += a[i] * b[i]; x += a[i] * a[i]; y += b[i] * b[i]; }
  return x && y ? d / Math.sqrt(x * y) : 0;
}
const SAME_MARK = 0.92;

/** The name a pool's monogram is made from: its listed name, or the host on shared hosting. */
function poolLabel(domain, ps) {
  const two = String(domain).split(".").slice(-2).join(".");
  if (SHARED_HOST_DOMAINS.has(two)) return domain.split(".")[0];
  return ps.find((p) => !/solo/i.test(p.name))?.name || ps[0]?.name || domain;
}

// ---------------------------------------------------------------- main

async function generatedDir() {
  try {
    const m = JSON.parse(await fs.readFile(path.join(DATA, "current.json"), "utf8"));
    if (/^[A-Za-z0-9T_-]+$/.test(m.snapshot || "")) return path.join(DATA, "snapshots", m.snapshot);
  } catch { /* flat layout */ }
  return DATA;
}

async function readJson(p, fallback) { try { return JSON.parse(await fs.readFile(p, "utf8")); } catch { return fallback; } }

async function coinCandidates(coin, links, ciManifest) {
  const cands = [];
  for (const u of PINNED[coin.id] || []) for (const c of (await discoverOrDirect(u))) cands.push({ ...c, kind: "official", via: c.via === "favicon-probe" ? c.via : "pinned", page: u });
  const ls = (links.coins?.[coin.id] || []).filter((l) => l.status === "verified");
  const sites = ls.filter((l) => l.kind === "website");
  const siteGithub = new Set();
  for (const [i, l] of sites.entries()) {
    for (const c of await discover(l.url, siteGithub)) cands.push({ ...c, kind: "official", page: l.url, order: i });
  }
  const orgs = new Set();
  for (const l of ls.filter((l) => l.kind === "github")) {
    const m = l.url.match(/^https:\/\/github\.com\/([A-Za-z0-9-]+)/);
    if (m) orgs.add(m[1]);
  }
  // No GitHub link in links.json: use the org the coin's own website links to.
  // Only an org named after the coin: sites also link to libraries, themes and Google's org.
  const coinKey = (x) => String(x || "").toLowerCase().replace(/[^a-z0-9]/g, "");
  const keys = [coinKey(coin.name), coinKey(coin.id.split("-")[0]), coin.symbol?.length >= 3 ? coinKey(coin.symbol) : null].filter(Boolean);
  if (!orgs.size) for (const o of siteGithub) if (keys.some((k) => coinKey(o).includes(k))) orgs.add(o);
  for (const o of orgs) cands.push({ url: `https://github.com/${o}.png?size=128`, via: "github", kind: "official", page: `https://github.com/${o}` });
  const ci = (ciManifest || []).find((x) => x.symbol?.toLowerCase() === coin.symbol?.toLowerCase() && x.name?.toLowerCase().replace(/[^a-z]/g, "") === coin.name?.toLowerCase().replace(/[^a-z]/g, ""));
  if (ci) cands.push({ url: `https://raw.githubusercontent.com/spothq/cryptocurrency-icons/master/svg/color/${ci.symbol.toLowerCase()}.svg`, via: "repo", kind: "repo", page: "https://github.com/spothq/cryptocurrency-icons" });
  return cands;
}

async function discoverOrDirect(u) {
  if (/\.(svg|png|webp|ico|jpe?g)(\?|$)/i.test(u)) return [{ url: u, via: "pinned", hint: 0 }];
  return discover(u);
}

async function coingecko(coin) {
  const id = COINGECKO[coin.id];
  if (!id) return null;
  const r = await get(`https://api.coingecko.com/api/v3/coins/${id}?localization=false&tickers=false&market_data=false&community_data=false&developer_data=false&sparkline=false`, { accept: "application/json", tries: 3 });
  if (!r?.buf) return null;
  try {
    const j = JSON.parse(r.buf.toString("utf8"));
    if (String(j.symbol).toLowerCase() !== String(coin.symbol).toLowerCase()) return null;
    const img = j.image?.large || j.image?.small;
    return img ? { url: img, via: "coingecko", kind: "third_party", page: `https://www.coingecko.com/en/coins/${id}` } : null;
  } catch { return null; }
}

async function pick(cands, skip = []) {
  const evals = [];
  for (const c of cands) {
    if (skip.some((s) => c.url.includes(s))) continue;
    const e = await evaluate(c);
    e.score = e.error ? -1 : score(e);
    evals.push(e);
  }
  evals.sort((a, b) => b.score - a.score);
  return { best: evals.find((e) => e.score >= 0) || null, tried: evals };
}

function license(kind, label, best) {
  if (kind === "official") return `${label}'s own logo, from ${best.via === "github" ? "its official GitHub organisation" : "its own website"}. Trademark of its owner; shown only to identify it.`;
  if (kind === "repo") return "cryptocurrency-icons by spothq (CC0-1.0). Trademark of its owner; shown only to identify it.";
  if (kind === "third_party") return "CoinGecko coin image for this exact coin id. Trademark of its owner; shown only to identify it.";
  return "Generated monogram: no logo could be fetched from an official or reputable source.";
}

async function main() {
  const args = process.argv.slice(2);
  const force = args.includes("--force"), dry = args.includes("--dry-run"), retryFallbacks = args.includes("--retry-fallbacks");
  // --recheck: no network. Re-run the shared-mark checks on the files already in static/logos/.
  const recheck = args.includes("--recheck");
  const only = args.filter((a, i) => args[i - 1] === "--id");
  const which = recheck ? "none" : args.includes("--coins") ? "coins" : args.includes("--pools") ? "pools" : "both";
  loadOptional();
  if (!sharp) console.warn("note: sharp not found; ICO/JPEG sources are skipped and PNGs are kept as-is (npm i --no-save sharp svgo)");
  if (!svgo) console.warn("note: svgo not found; SVGs are sanitised but not minified");

  // Generated files live in the snapshot data/current.json names (scripts/snapshot.mjs); older
  // checkouts have them flat in data/.
  const gen = await generatedDir();
  const net = await readJson(path.join(gen, "network.json"), { coins: [] });
  const extraCoins = await readJson(path.join(DATA, "curated", "coins.json"), { coins: [] });
  const pools = await readJson(path.join(gen, "pools.json"), { pools: [] });
  const manual = await readJson(path.join(DATA, "curated", "manual-pools.json"), { pools: [] });
  const archive = await readJson(path.join(DATA, "archive.json"), { pools: [] });
  const links = await readJson(path.join(DATA, "curated", "links.json"), { coins: {}, pools: {} });
  const prev = await readJson(LOGOS_JSON, { coins: {}, pools: {} });
  // Never prune or write from missing inputs: an empty coin or pool list means the data files
  // couldn't be read, not that every logo should go.
  if (!(net.coins || []).length || !(pools.pools || []).length) {
    console.error(`refusing to run: no coins or pools read from ${path.relative(ROOT, gen) || "data"}/ (network.json, pools.json)`);
    process.exit(2);
  }
  const out = { _doc: "Written by scripts/fetch-logos.mjs (see README, 'Logos'). kind: official | repo | third_party | fallback. file is relative to static/logos/; fallback entries have file null and render as a monogram.", generated_at: new Date().toISOString(), coins: { ...(prev.coins || {}) }, pools: { ...(prev.pools || {}) } };
  const now = () => new Date().toISOString();
  const exists = async (rel) => { try { await fs.access(path.join(LOGOS_DIR, rel)); return true; } catch { return false; } };
  const need = async (e, keys) => {
    if (only.length && !keys.some((k) => only.includes(k))) return false;
    if (force || (only.length && force)) return true;
    if (!e) return true;
    if (e.kind === "fallback") return retryFallbacks;
    return !(e.file && (await exists(e.file)));
  };
  const ciManifest = which === "none" || which === "pools" ? null : (await get("https://raw.githubusercontent.com/spothq/cryptocurrency-icons/master/manifest.json"))?.buf;
  const ci = ciManifest ? JSON.parse(ciManifest.toString("utf8")) : [];
  const report = [];

  // ---- coins
  const seenCoin = new Set();
  const allCoins = [...(net.coins || []), ...(extraCoins.coins || [])].filter((c) => (seenCoin.has(c.id) ? false : (seenCoin.add(c.id), true)));
  if (which === "coins" || which === "both") for (const coin of allCoins) {
    const current = out.coins[coin.id];
    const deliberateOwnerRefresh = force && only.includes(coin.id);
    if (OWNER_SUPPLIED.has(coin.id) && current?.file && (await exists(current.file)) && !deliberateOwnerRefresh) continue;
    if (!(await need(out.coins[coin.id], [coin.id]))) continue;
    let cands = await coinCandidates(coin, links, ci);
    let { best, tried } = await pick(cands, SKIP[coin.id]);
    if (!best || best.kind !== "official") {
      const cg = await coingecko(coin);
      if (cg) { const more = await pick([cg]); tried = tried.concat(more.tried); if (!best || (more.best && more.best.score > best.score)) best = best || more.best; }
    }
    out.coins[coin.id] = await entry(best, "coins", coin.id, coin.name, tried, dry);
    report.push(["coin", coin.id, out.coins[coin.id].kind, (out.coins[coin.id].source_url || "-").slice(0, 90)]);
  }

  // ---- pools (one fetch per operator domain)
  const allPools = [...(pools.pools || []), ...(manual.pools || []), ...(archive.pools || []).map((p) => ({ ...p, archived: true }))];
  const coinWords = new Set(COMMON_COIN_WORDS);
  for (const c of allCoins) for (const w of [c.id, c.symbol, c.name.replace(/\s+/g, "")]) if (w && w.length >= 3) coinWords.add(w.toLowerCase());
  const byDomain = new Map();
  for (const p of allPools) {
    const d = operatorDomain(p.url || "") || stem(p.id);
    if (!byDomain.has(d)) byDomain.set(d, []);
    byDomain.get(d).push(p);
  }
  if (which === "pools" || which === "both") for (const [domain, ps] of byDomain) {
    const ids = ps.map((p) => p.id);
    const needs = [];
    for (const p of ps) if (await need(out.pools[p.id], [p.id, domain])) needs.push(p);
    if (!needs.length) continue;
    const pages = [...new Set([...(PINNED[domain] || []), ...ps.map((p) => p.url).filter(Boolean).map((u) => new URL(u).origin + "/"), /^\d/.test(domain) ? null : `https://${domain}/`, /^\d/.test(domain) ? null : `https://www.${domain}/`].filter(Boolean))];
    const cands = [];
    for (const u of [...new Set([ps[0].url, ...pages].filter(Boolean))].slice(0, 4)) for (const c of await discoverOrDirect(u)) cands.push({ ...c, kind: "official", via: (PINNED[domain] || []).includes(u) && c.via !== "favicon-probe" ? "pinned" : c.via, page: u });
    const uniq = [];
    const seen = new Set();
    for (const c of cands) if (!seen.has(c.url)) { seen.add(c.url); uniq.push(c); }
    const filtered = uniq.filter((c) => !(["img.logo", "og:image", "twitter:image", "icon", "apple-touch-icon", "manifest"].includes(c.via) && namesACoin(c.url, coinWords)) || c.via === "pinned");
    const { best, tried } = await pick(filtered, SKIP[domain]);
    for (const c of uniq) if (!filtered.includes(c)) tried.push({ url: c.url, error: "named after a coin, not the pool's own mark" });
    const name = poolLabel(domain, ps);
    const e = await entry(best, "pools", domain, name, tried, dry);
    for (const p of needs) out.pools[p.id] = { ...e, domain, ...(p.archived ? { archived: true } : {}) };
    report.push(["pool", domain + ` (${ids.length})`, e.kind, (e.source_url || "-").slice(0, 90)]);
  }

  // ---- A pool mark that is also another operator's (a pool-software default theme, e.g. s-nomp)
  // or a coin's logo is not the pool's own: those pools get monograms.
  if (sharp) {
    const sigs = new Map(); // file -> signature
    const sigOf = async (e) => {
      if (!sigs.has(e.file)) { try { sigs.set(e.file, await signature(await fs.readFile(path.join(LOGOS_DIR, e.file)), e.format)); } catch { sigs.set(e.file, null); } }
      return sigs.get(e.file);
    };
    const coinSigs = [];
    for (const [id, e] of Object.entries(out.coins)) if (e.file) coinSigs.push({ id, h: await sigOf(e) });
    const poolSigs = [];
    for (const [id, e] of Object.entries(out.pools)) if (e.file && e.kind !== "fallback") poolSigs.push({ id, e, h: await sigOf(e), domain: e.domain });
    const flagged = [];
    for (const a of poolSigs) {
      if (!a.h) continue;
      const twins = [...new Set(poolSigs.filter((b) => b.h && b.domain !== a.domain && similarity(a.h, b.h) >= SAME_MARK).map((b) => b.domain))];
      const coins = [...new Set(coinSigs.filter((c) => c.h && similarity(a.h, c.h) >= SAME_MARK).map((c) => c.id))];
      if (twins.length) flagged.push({ a, why: `the same mark is used by ${twins.join(", ")}, so it is a shared pool-software default rather than this operator's own` });
      else if (coins.length) flagged.push({ a, why: `it is the ${coins.join(" / ")} coin logo, not a mark of the pool's own` });
    }
    const domains = new Set(flagged.map((f) => f.a.domain));
    // Sanity guard: a check that suddenly flags many operators is broken, not right.
    if (domains.size > 6) console.warn(`shared-mark check flagged ${domains.size} operators; ignoring it (check the threshold)`);
    else for (const { a, why } of flagged) {
      console.warn(`monogram for ${a.id}: ${why}`);
      out.pools[a.id] = { file: null, source_url: null, fetched_at: a.e.fetched_at, kind: "fallback", license_note: `Generated monogram: the logo found at ${a.e.source_url} was not used because ${why}.`, monogram: monogram(poolLabel(a.domain, byDomain.get(a.domain) || [])), domain: a.domain, ...(a.e.archived ? { archived: true } : {}) };
    }
  }
  // Monograms on shared hosting use the host name: buckpool.myvnc.com -> "B", not "myvnc".
  for (const [id, e] of Object.entries(out.pools)) if (e.kind === "fallback" && e.domain && byDomain.has(e.domain)) e.monogram = monogram(poolLabel(e.domain, byDomain.get(e.domain)));

  // ---- write
  const coinIds = new Set(allCoins.map((c) => c.id)), poolIds = new Set(allPools.map((p) => p.id));
  for (const k of Object.keys(out.coins)) if (!coinIds.has(k)) delete out.coins[k];
  for (const k of Object.keys(out.pools)) if (!poolIds.has(k)) delete out.pools[k];
  console.table(report);
  const tally = (g) => Object.values(out[g]).reduce((a, e) => ((a[e.kind] = (a[e.kind] || 0) + 1), a), {});
  console.log("coins", tally("coins"), "pools", tally("pools"));
  if (dry) return;
  await fs.writeFile(LOGOS_JSON, JSON.stringify(out, null, 2) + "\n");
  // Remove files no entry points at any more (only inside static/logos/).
  const used = new Set([...Object.values(out.coins), ...Object.values(out.pools)].map((e) => e.file).filter(Boolean));
  for (const g of ["coins", "pools"]) for (const f of await fs.readdir(path.join(LOGOS_DIR, g)).catch(() => [])) if (!used.has(`${g}/${f}`)) await fs.unlink(path.join(LOGOS_DIR, g, f));
  console.log("wrote", path.relative(ROOT, LOGOS_JSON));

  async function entry(best, group, key, label, tried, dryRun) {
    const base = { fetched_at: now() };
    if (best) {
      try {
        const w = await writeLogo(best, group, key);
        if (!dryRun) { await fs.mkdir(path.join(LOGOS_DIR, group), { recursive: true }); await fs.writeFile(path.join(LOGOS_DIR, w.rel), w.bytes); }
        const inline = best.final_url.startsWith("data:");
        return { file: w.rel, source_url: inline ? best.page : best.final_url, ...(inline ? { source_detail: "inline data: URI in the page's <link rel=icon>" } : {}), source_page: best.page || null, fetched_at: base.fetched_at, kind: best.kind, license_note: license(best.kind, label, best), format: w.ext, bytes: w.bytes.length, ...(w.bg ? { bg: w.bg } : {}), ...(w.ink ? { ink: w.ink } : {}), source_px: best.type === "svg" ? "vector" : `${best.w}x${best.h}` };
      } catch (e) { tried.push({ url: best.final_url, error: "write failed: " + e.message }); }
    }
    return { file: null, source_url: null, fetched_at: base.fetched_at, kind: "fallback", license_note: license("fallback"), monogram: monogram(label), tried: tried.slice(0, 8).map((t) => `${t.url.startsWith("data:") ? "data: URI" : t.url} → ${t.error || `score ${t.score}`}`) };
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  main().catch((e) => { console.error(e); process.exit(1); });
}
