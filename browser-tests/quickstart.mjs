// Checks the quickstart playground (examples/quickstart) as deployed.
//
//   node quickstart.mjs <dist directory> [--base /monaco-yew/] [--screenshots <dir>]
//
// Serves the Trunk output under --base (the GitHub Pages path), then drives
// the page like a user: diagnostics, Format, Load sample, Download, themes,
// options, read-only, and a phone-sized layout. Set CHROMIUM to use a system
// browser. Exits non-zero on failure.

import { createReadStream, existsSync, readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright-core";

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 ? args.splice(index, 2)[1] : fallback;
};
const basePath = option("--base", "/monaco-yew/");
const screenshots = option("--screenshots", undefined);
const root = resolve(args[0] ?? "");
if (!args[0] || !existsSync(join(root, "index.html"))) {
  console.error("usage: node quickstart.mjs <dist directory> [--base /path/] [--screenshots <dir>]");
  process.exit(2);
}

const TYPES = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".css": "text/css" };
const server = createServer((request, response) => {
  const url = new URL(request.url, "http://localhost");
  if (!url.pathname.startsWith(basePath)) {
    response.writeHead(404).end();
    return;
  }
  let path = normalize(join(root, url.pathname.slice(basePath.length)));
  if (!path.startsWith(root)) {
    response.writeHead(403).end();
    return;
  }
  if (!existsSync(path) || statSync(path).isDirectory()) path = join(root, "index.html");
  response.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" });
  createReadStream(path).pipe(response);
});
await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
const origin = `http://127.0.0.1:${server.address().port}`;
const base = `${origin}${basePath}`;

const failures = [];
const check = (ok, message) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${message}`);
  if (!ok) failures.push(message);
};
const shot = async (page, name, options = {}) => {
  if (screenshots) await page.screenshot({ path: join(screenshots, `${name}.png`), ...options });
};
// See smoke.mjs: Trunk's preload SRI hashes trigger a benign Chrome warning.
const BENIGN = /The `integrity` attribute is currently ignored for preload destinations/;

const b = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
const problems = [], external = [];
async function open(scheme, viewport) {
  const ctx = await b.newContext({ colorScheme: scheme, viewport, acceptDownloads: true });
  const page = await ctx.newPage();
  page.on("console", (m) => { if (["error", "warning"].includes(m.type()) && !BENIGN.test(m.text())) problems.push(m.text()); });
  page.on("pageerror", (e) => problems.push(e.message));
  page.on("request", (r) => { const u = r.url(); if (!u.startsWith(origin) && !/^(blob|data):/.test(u)) external.push(u); });
  await page.goto(base);
  await page.waitForSelector(".monaco-editor .view-lines");
  await page.waitForTimeout(500);
  return page;
}
const value = (page) => page.evaluate(() => [...document.querySelectorAll(".view-line")].map((l) => l.textContent).join("\n"));

const page = await open("light", { width: 1280, height: 800 });
await shot(page, "light");
check(await page.evaluate(() => document.querySelector(".monaco-editor").classList.contains("vs")), "light system theme -> vs");
const box = await page.locator("main .monaco-editor").boundingBox();
check(box.height > 400, `editor fills the page (${Math.round(box.height)}px)`);

// Break the JSON: diagnostics appear.
await page.click(".monaco-editor .view-lines");
await page.keyboard.press("Control+End"); await page.keyboard.type(",");
check(await page.waitForSelector(".squiggly-error", { timeout: 10000 }).then(() => true, () => false), "JSON diagnostics");
await page.click("text=Format");
check((await page.textContent("footer")).includes("Not valid JSON"), "Format reports invalid JSON");
check((await page.textContent("footer")).includes("edits"), "status counts edits");
await page.click("text=Load sample");
await page.waitForTimeout(200);
check((await page.textContent("footer")).includes("document replaced"), "Load sample replaces the document");
// Collapse formatting, then Format restores indentation.
await page.click(".monaco-editor .view-lines"); await page.keyboard.press("Control+A"); await page.keyboard.press("Delete"); await page.keyboard.type('{"a":1,"b":[1,2]}');
await page.click("text=Format"); await page.waitForTimeout(200);
check((await page.textContent("footer")).includes("Formatted."), "Format pretty-prints");
const [download] = await Promise.all([page.waitForEvent("download"), page.click("text=Download")]);
const saved = readFileSync(await download.path(), "utf8");
check(saved.includes('"a": 1') && download.suggestedFilename() === "document.json", `Download saves the document (${download.suggestedFilename()})`);

await page.selectOption("nav select >> nth=0", { label: "Dark" });
await page.waitForTimeout(200);
check(await page.evaluate(() => document.querySelector(".monaco-editor").classList.contains("vs-dark") && document.querySelector(".app").classList.contains("dark")), "theme switch updates editor and page");
await page.check("text=Minimap"); await page.check("text=Word wrap"); await page.fill("nav input[type=number]", "20"); await page.press("nav input[type=number]", "Tab");
await page.waitForTimeout(300);
check(await page.evaluate(() => document.querySelector(".minimap") !== null && getComputedStyle(document.querySelector(".minimap")).width !== "0px"), "minimap toggle");
await shot(page, "dark");
await page.check("text=Read-only");
await page.click(".monaco-editor .view-lines"); const before = await value(page); await page.keyboard.type("ZZZ");
check((await value(page)) === before, "read-only rejects typing");
await page.selectOption("nav select >> nth=1", "plaintext"); await page.waitForTimeout(200);
check(await page.evaluate(() => document.querySelector(".squiggly-error") === null), "plain text has no JSON diagnostics");

const dark = await open("dark", { width: 390, height: 844 });
check(await dark.evaluate(() => document.querySelector(".monaco-editor").classList.contains("vs-dark")), "dark system theme -> vs-dark");
const phoneEditor = await dark.locator("main .monaco-editor").boundingBox();
check(phoneEditor.height > 200, `phone layout gives the editor height (${Math.round(phoneEditor.height)}px)`);
const overflow = await dark.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
check(!overflow, "no horizontal page scroll on a phone");
await shot(dark, "phone");

check(external.length === 0, `no external requests ${external.join(" ")}`);
check(problems.length === 0, `no console problems ${problems.join(" | ")}`);
await b.close();
server.close();
if (failures.length > 0) {
  console.error(`${failures.length} check(s) failed`);
  process.exit(1);
}
