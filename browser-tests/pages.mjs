// Checks the assembled GitHub Pages site (tools/build-pages.sh output).
//
//   node pages.mjs <site directory> [--base /monaco-wasm/] [--screenshots <dir>]
//
// Serves the site under --base, checks the landing page in light, dark, and
// phone-sized layouts, and follows the links to both quickstarts. The
// quickstarts themselves are covered in depth by quickstart.mjs. Set
// CHROMIUM to use a system browser. Exits non-zero on failure.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright-core";

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 ? args.splice(index, 2)[1] : fallback;
};
const basePath = option("--base", "/monaco-wasm/");
const screenshots = option("--screenshots", undefined);
const root = resolve(args[0] ?? "");
if (!args[0] || !existsSync(join(root, "index.html"))) {
  console.error("usage: node pages.mjs <site directory> [--base /path/] [--screenshots <dir>]");
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
  if (existsSync(path) && statSync(path).isDirectory()) path = join(path, "index.html");
  if (!existsSync(path)) {
    response.writeHead(404).end();
    return;
  }
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
// See smoke.mjs: Trunk's preload SRI hashes trigger a benign Chrome warning.
const BENIGN = /The `integrity` attribute is currently ignored for preload destinations/;

const browser = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
const problems = [];
const external = [];
async function open(colorScheme, viewport) {
  const context = await browser.newContext({ colorScheme, viewport });
  const page = await context.newPage();
  page.on("console", (m) => {
    if (["error", "warning"].includes(m.type()) && !BENIGN.test(m.text())) problems.push(m.text());
  });
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("response", (response) => {
    if (response.status() >= 400) problems.push(`HTTP ${response.status()} ${response.url()}`);
  });
  page.on("request", (request) => {
    const url = request.url();
    if (!url.startsWith(origin) && !/^(blob|data):/.test(url)) external.push(url);
  });
  await page.goto(base);
  return page;
}

try {
  const page = await open("light", { width: 1280, height: 900 });
  check((await page.title()).includes("monaco-wasm"), "landing page loads");
  const links = await page.locator(".card a.button").evaluateAll((anchors) => anchors.map((a) => a.getAttribute("href")));
  check(JSON.stringify(links) === JSON.stringify(["yew/", "leptos/"]), `quickstart links ${JSON.stringify(links)}`);
  const repository = await page.locator(".links a").getAttribute("href");
  check(!repository.includes("__REPOSITORY_URL__"), `repository link filled in (${repository})`);
  const clipped = await page.evaluate(() => [...document.querySelectorAll("pre")].filter((pre) => pre.scrollWidth > pre.clientWidth).length);
  check(clipped === 0, `code snippets fit without scrolling on a desktop (${clipped} clipped)`);
  if (screenshots) await page.screenshot({ path: join(screenshots, "landing-light.png"), fullPage: true });

  for (const [label, heading] of [
    ["Open the Yew quickstart", "monaco-yew"],
    ["Open the Leptos quickstart", "monaco-leptos"],
  ]) {
    await page.goto(base);
    await page.click(`text=${label}`);
    const loaded = await page
      .waitForSelector(".monaco-editor .view-lines", { timeout: 20000 })
      .then(() => true)
      .catch(() => false);
    check(loaded && (await page.textContent("h1")) === heading, `${label} opens a working editor`);
  }

  const dark = await open("dark", { width: 1280, height: 900 });
  const background = await dark.evaluate(() => getComputedStyle(document.body).backgroundColor);
  check(background === "rgb(13, 17, 23)", `landing page follows the dark color scheme (${background})`);
  if (screenshots) await dark.screenshot({ path: join(screenshots, "landing-dark.png"), fullPage: true });

  const phone = await open("light", { width: 390, height: 844 });
  const overflow = await phone.evaluate(() => document.documentElement.scrollWidth > window.innerWidth);
  check(!overflow, "no horizontal page scroll on a phone");
  if (screenshots) await phone.screenshot({ path: join(screenshots, "landing-phone.png"), fullPage: true });

  check(external.length === 0, `no external requests${external.length ? `: ${external.join(" ")}` : ""}`);
  check(problems.length === 0, `no console errors or failed requests${problems.length ? `: ${problems.join(" | ")}` : ""}`);
} finally {
  await browser.close();
  server.close();
}

if (failures.length > 0) {
  console.error(`${failures.length} check(s) failed`);
  process.exit(1);
}
