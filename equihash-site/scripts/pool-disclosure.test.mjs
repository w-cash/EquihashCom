import { test } from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

// Exercise the actual pool-table controller with a small DOM. No copied visibility logic.
const app = fs.readFileSync(new URL("../static/app.js", import.meta.url), "utf8");
const controller = app.split("  // ---------- pool table: filter + sort ----------")[1].split("  // ---------- small-screen filter toggle ----------")[0];
const dataKey = (name) => name.slice(5).replace(/-([a-z])/g, (_, c) => c.toUpperCase());
function element(tag, attrs = {}, children = []) {
  const node = { tag, attrs: { ...attrs }, children: [], dataset: {}, hidden: false, listeners: {}, textContent: "", innerHTML: "" };
  for (const [key, value] of Object.entries(attrs)) if (key.startsWith("data-")) node.dataset[dataKey(key)] = value;
  node.getAttribute = (key) => key.startsWith("data-") ? node.dataset[dataKey(key)] ?? null : node.attrs[key] ?? null;
  node.setAttribute = (key, value) => { if (key.startsWith("data-")) node.dataset[dataKey(key)] = String(value); else node.attrs[key] = String(value); };
  node.classList = {
    contains: (name) => (node.attrs.class || "").split(/\s+/).includes(name),
    toggle: (name, on) => { const list = new Set((node.attrs.class || "").split(/\s+/).filter(Boolean)); if (on) list.add(name); else list.delete(name); node.attrs.class = [...list].join(" "); },
  };
  node.appendChild = (child) => { if (child.parent) child.parent.children = child.parent.children.filter((item) => item !== child); child.parent = node; node.children.push(child); return child; };
  node.addEventListener = (event, callback) => { (node.listeners[event] ||= []).push(callback); };
  node.click = () => (node.listeners.click || []).forEach((callback) => callback({ preventDefault() {}, target: node }));
  node.matches = (selector) => {
    const negative = selector.match(/:not\(([^)]+)\)/);
    if (negative && node.matches(negative[1])) return false;
    selector = selector.replace(/:not\([^)]+\)/g, "");
    const name = selector.match(/^[a-z]+/i)?.[0];
    if (name && node.tag !== name) return false;
    const id = selector.match(/#([\w-]+)/)?.[1];
    if (id && node.attrs.id !== id) return false;
    for (const [, name] of selector.matchAll(/\.([\w-]+)/g)) if (!node.classList.contains(name)) return false;
    for (const [, key, quoted, plain] of selector.matchAll(/\[([\w-]+)(?:=(?:"([^"]*)"|([^\]]*)))?\]/g)) {
      const got = node.getAttribute(key), value = quoted ?? plain;
      if (got == null || (value !== undefined && got !== value)) return false;
    }
    return true;
  };
  node.closest = (selector) => node.matches(selector) ? node : node.parent?.closest(selector) ?? null;
  node.querySelectorAll = (selector) => {
    const all = [], walk = (parent) => parent.children.forEach((child) => { all.push(child); walk(child); });
    walk(node);
    return all.filter((child) => selector.split(",").some((part) => {
      const chain = part.trim().split(/\s+/);
      if (!child.matches(chain.pop())) return false;
      let parent = child.parent;
      while (chain.length) { const wanted = chain.pop(); while (parent && !parent.matches(wanted)) parent = parent.parent; if (!parent) return false; parent = parent.parent; }
      return true;
    }));
  };
  node.querySelector = (selector) => node.querySelectorAll(selector)[0] || null;
  children.forEach(node.appendChild);
  return node;
}
function pool(id, hashrate, { promotion = false, fee = 1 } = {}) {
  return element("tr", {
    class: "pool-row", "data-pool-id": id, "data-name": id, "data-search": id,
    "data-schemes": "SOLO", "data-regions": "Europe", "data-fee": String(fee), "data-merged": "0",
    ...(hashrate == null ? {} : { "data-hashrate": String(hashrate) }),
    ...(promotion ? { "data-promotion": "unpaid" } : {}),
  }, [element("td", { class: "rank" }), element("td", { class: "hash" }, [element("span", { "data-f": "hashrate" })])]);
}
function setup(rows) {
  const more = element("tr", { class: "pool-more-control" }, [element("td", {}, [element("button", { "data-pool-more": "", "aria-expanded": "false" })])]);
  const group = element("tbody", { id: "pool-group-0", class: "pool-group" }, [...rows, more]);
  const header = element("thead", {}, [element("tr", {}, [
    element("th", { "data-sort": "hashrate", "aria-sort": "descending" }, [element("a")]),
    element("th", { "data-sort": "fee", "aria-sort": "none" }, [element("a")]),
  ])]);
  const table = element("table", { id: "pool-table" }, [header, group]);
  const fields = ["q", "scheme", "region", "fee", "hr", "merged", "hide_empty"].map((name) => Object.assign(element("input", { "data-filter": name }), { name, type: name === "hide_empty" ? "checkbox" : "search", checked: false, value: "" }));
  const form = element("form", { id: "filters" }, fields);
  const root = element("main", {}, [form, table, element("span", { id: "pool-count" }), element("span", { id: "pool-shown-count" }), element("p", { id: "empty" }), element("p", { id: "pool-promotion-note" })]);
  const context = vm.createContext({
    document: root, window: {}, location: { pathname: "/pools", hash: "" }, history: { replaceState() {} }, URLSearchParams,
    $: (selector, parent = root) => parent.querySelector(selector), $$: (selector, parent = root) => parent.querySelectorAll(selector),
    json: () => [], setTimeout: () => 1, clearTimeout() {},
  });
  vm.runInContext(controller, context);
  return { rows, group, more, button: more.querySelector("button"), root, fields: Object.fromEntries(fields.map((field) => [field.name, field])), run: (code) => vm.runInContext(code, context), context };
}

test("default pool disclosure keeps all records and reveals zero / unavailable rows on click", () => {
  const x = setup([pool("active", 1234.5), pool("zero", 0), pool("unknown", null)]);
  assert.deepEqual(x.rows.map((row) => row.hidden), [false, true, true]);
  assert.equal(x.more.hidden, false);
  assert.match(x.button.innerHTML, /^Show 2 more pools/);
  assert.equal(x.root.querySelector("#pool-count").textContent, 3);
  assert.equal(x.root.querySelector("#pool-shown-count").textContent, 1);
  x.button.click();
  assert.deepEqual(x.rows.map((row) => row.hidden), [false, false, false]);
  assert.equal(x.button.getAttribute("aria-expanded"), "true");
  assert.match(x.button.innerHTML, /^Hide 2 pools/);
  x.button.click();
  assert.deepEqual(x.rows.map((row) => row.hidden), [false, true, true]);
});

test("search and filters reveal matching zero rows; clearing restores the compact view", () => {
  const x = setup([pool("active", 100), pool("zero", 0), pool("unknown", null)]);
  x.fields.q.value = "zero";
  x.run("applyFilters(false)");
  assert.deepEqual(x.rows.map((row) => row.hidden), [true, false, true]);
  assert.equal(x.more.hidden, true);
  assert.equal(x.root.querySelector("#empty").hidden, true);
  x.fields.q.value = "";
  x.fields.region.value = "Europe";
  x.run("applyFilters(false)");
  assert.ok(x.rows.every((row) => !row.hidden));
  x.fields.region.value = "";
  x.run("applyFilters(false)");
  assert.deepEqual(x.rows.map((row) => row.hidden), [false, true, true]);
});

test("nondefault sorting reveals all rows in the requested order and keeps the control last", () => {
  const x = setup([pool("active", 100, { fee: 2 }), pool("zero", 0, { fee: 0 }), pool("unknown", null)]);
  x.run('sortBy("fee", "asc"); applyFilters(false)');
  assert.deepEqual(x.group.children.filter((row) => row.classList.contains("pool-row")).map((row) => row.dataset.poolId), ["zero", "unknown", "active"]);
  assert.ok(x.rows.every((row) => !row.hidden));
  assert.equal(x.group.children.at(-1), x.more);
  assert.equal(x.more.hidden, true);
});

test("a group with only zero/unavailable rows still exposes its disclosure and is not an empty result", () => {
  const x = setup([pool("zero", 0), pool("unknown", null)]);
  assert.equal(x.group.hidden, false);
  assert.equal(x.more.hidden, false);
  assert.equal(x.root.querySelector("#empty").hidden, true);
  x.button.click();
  assert.ok(x.rows.every((row) => !row.hidden));
});

test("live transitions update both promotional copies, rerank and disclose newly positive pools", () => {
  const x = setup([pool("zecwec", 0, { promotion: true }), pool("active", 100), pool("zecwec", 0), pool("unknown", null)]);
  assert.equal(x.rows[0].hidden, false, "placement is independent of the ordinary zero row");
  assert.equal(x.rows[2].hidden, true);
  x.context.update = { pools: { zecwec: { hashrate: 200, hashrate_text: "200 Sol/s" }, active: { hashrate: 0, hashrate_text: "0 Sol/s" } } };
  x.run("applyPoolReadings(update)");
  assert.equal(x.rows[2].hidden, false);
  assert.equal(x.rows[1].hidden, true);
  assert.deepEqual(x.group.children.filter((row) => row.classList.contains("pool-row")).map((row) => row.dataset.poolId), ["zecwec", "zecwec", "active", "unknown"]);
  assert.equal(x.rows[0].querySelector('[data-f="hashrate"]').textContent, "200 Sol/s");
  assert.equal(x.root.querySelector("#pool-count").textContent, 3, "promotion does not duplicate directory count");
  assert.equal(x.root.querySelector("#pool-shown-count").textContent, 1);
  x.context.update = { pools: { zecwec: { hashrate: null, hashrate_text: "n/a" } } };
  x.run("applyPoolReadings(update)");
  assert.equal(x.rows[2].dataset.hashrate, undefined);
  assert.equal(x.rows[2].hidden, true);
  assert.equal(x.rows[0].hidden, false);
});
