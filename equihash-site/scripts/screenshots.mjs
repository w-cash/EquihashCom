// Dev tool: headless screenshots of the running site into screenshots/.
// Usage: BASE=http://127.0.0.1:8080 node scripts/screenshots.mjs
// Needs playwright-core (npm i -D playwright-core, or NODE_PATH to an install) and a Chrome/Chromium binary.
import { chromium } from "playwright-core";
import fs from "node:fs";

const BASE = process.env.BASE || "http://127.0.0.1:8080";
const OUT = new URL("../screenshots/", import.meta.url).pathname;
const exe = process.env.CHROME || ["/usr/bin/google-chrome", "/usr/bin/chromium", "/usr/bin/chromium-browser"].find((p) => fs.existsSync(p));
fs.mkdirSync(OUT, { recursive: true });

const browser = await chromium.launch({ executablePath: exe, args: ["--no-sandbox"] });
const desk = await browser.newContext({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: "dark" });
const shots = [
  ["home-pools.png", "/", { full: false }],
  ["home-full.png", "/?coin=zcash", { full: true }],
  ["archive.png", "/archive", { full: true }],
  ["miners.png", "/miners", { full: true }],
  ["merged-mining-guide.png", "/merged-mining", { full: true }],
  ["calculator.png", "/calculator", { full: false }],
  ["add-pool.png", "/add-pool", { full: true }],
  ["about.png", "/about", { full: true }],
  ["sources.png", "/sources", { full: false }],
];
for (const [file, path, o] of shots) {
  const p = await desk.newPage();
  await p.goto(BASE + path, { waitUntil: "networkidle" });
  await p.waitForTimeout(1100);
  await p.screenshot({ path: OUT + file, fullPage: o.full });
  console.log("saved", OUT + file);
  await p.close();
}
// Pool table + drawer open on the largest ZEC pool
{
  const p = await desk.newPage();
  await p.goto(BASE + "/?coin=zcash#pools", { waitUntil: "networkidle" });
  await p.waitForTimeout(400);
  await p.click("#pool-table tbody tr.pool-row:not([hidden]) .pool-link");
  await p.waitForTimeout(700);
  await p.screenshot({ path: OUT + "pool-drawer.png" });
  console.log("saved", OUT + "pool-drawer.png");
  await p.close();
}
// Mobile
const mob = await browser.newContext({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "dark" });
for (const [file, path, full] of [["mobile-home.png", "/", false], ["mobile-pools.png", "/?coin=zcash#pools", false], ["mobile-home-full.png", "/", true]]) {
  const p = await mob.newPage();
  await p.goto(BASE + path, { waitUntil: "networkidle" });
  await p.waitForTimeout(1100);
  await p.screenshot({ path: OUT + file, fullPage: full });
  console.log("saved", OUT + file);
  await p.close();
}
{
  const p = await mob.newPage();
  await p.goto(BASE + "/?coin=zcash#pools", { waitUntil: "networkidle" });
  await p.click("#pool-table tbody tr.pool-row:not([hidden]) .pool-link");
  await p.waitForTimeout(700);
  await p.screenshot({ path: OUT + "mobile-drawer.png" });
  console.log("saved", OUT + "mobile-drawer.png");
  await p.close();
}
await browser.close();
