// Dev tool: headless screenshots of the running site into screenshots/.
// Usage: BASE=http://127.0.0.1:8090 node scripts/screenshots.mjs
// Needs playwright-core (npm i --no-save playwright-core, or NODE_PATH to an install) and Chrome/Chromium.
import { chromium } from "playwright-core";
import fs from "node:fs";

const BASE = process.env.BASE || "http://127.0.0.1:8080";
const OUT = (process.env.OUT || new URL("../screenshots/", import.meta.url).pathname).replace(/\/?$/, "/");
const exe = process.env.CHROME || ["/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"].find((p) => fs.existsSync(p));
fs.mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({ executablePath: exe, args: ["--no-sandbox"] });
const errors = [];
async function shot(ctx, file, path, { full = false, click = null, wait = 500 } = {}) {
  const p = await ctx.newPage();
  p.on("pageerror", (e) => errors.push(`${path}: ${e.message}`));
  await p.goto(BASE + path, { waitUntil: "networkidle" });
  if (click) { await p.click(click); }
  await p.waitForTimeout(wait);
  await p.screenshot({ path: OUT + file, fullPage: full });
  console.log("saved", OUT + file);
  await p.close();
}
const desk = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: "light" });
const firstRow = "#pool-table tbody tr.pool-row:not([hidden]) .pool-link";
await shot(desk, "home.png", "/");
await shot(desk, "home-full.png", "/", { full: true });
await shot(desk, "home-all-coins.png", "/?coin=all");
await shot(desk, "home-komodo.png", "/?coin=komodo");
await shot(desk, "home-wcash.png", "/?coin=wcash");
await shot(desk, "pool-drawer.png", "/?coin=zcash#pools", { click: firstRow });
await shot(desk, "pool-page.png", "/pool/" + (process.env.POOL || "zcash-viabtc-viabtc-com"));
await shot(desk, "hardware.png", "/miners", { full: true });
await shot(desk, "calculator.png", "/calculator");
await shot(desk, "calculator-wcash.png", "/calculator?coin=wcash");
await shot(desk, "calculator-invalid.png", "/calculator?coin=zcash&hashrate=840&watts=2780&power=0.08&fee=150");
await shot(desk, "merged-mining-guide.png", "/merged-mining", { full: true });
await shot(desk, "archive.png", "/archive", { full: true });
await shot(desk, "add-pool.png", "/add-pool", { full: true });
await shot(desk, "about.png", "/about", { full: true });
await shot(desk, "sources.png", "/sources", { full: true });
const dark = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: "dark" });
await shot(dark, "home-dark.png", "/");
const mob = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "light" });
await shot(mob, "mobile-home.png", "/");
await shot(mob, "mobile-pools.png", "/?coin=zcash#pools");
await shot(mob, "mobile-home-full.png", "/", { full: true });
await shot(mob, "mobile-drawer.png", "/?coin=zcash#pools", { click: firstRow });
await shot(mob, "mobile-hardware.png", "/miners");
await shot(mob, "mobile-calculator.png", "/calculator", { full: true });
await shot(mob, "mobile-guide.png", "/merged-mining");
await shot(mob, "mobile-archive.png", "/archive");
await shot(mob, "mobile-about.png", "/about");
await browser.close();
if (errors.length) { console.error("page errors:\n" + errors.join("\n")); process.exit(1); }
