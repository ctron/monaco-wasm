// Smoke test for a built demo page (examples/wasm-basic layout).
//
//   node smoke.mjs <directory containing index.html and pkg/> [--browser /path/to/chromium] [--csp "<policy>"]
//
// Serves the directory under /demo/ on a local port, so relative asset paths
// are exercised, then checks editing, JSON diagnostics, change events,
// disposal, and that no request leaves the origin. With --csp, every response
// carries that Content-Security-Policy and any violation fails the run.
// Exits non-zero on failure.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright-core";

const args = process.argv.slice(2);
const browserFlag = args.indexOf("--browser");
const executablePath = browserFlag >= 0 ? args.splice(browserFlag, 2)[1] : process.env.CHROMIUM;
const cspFlag = args.indexOf("--csp");
const csp = cspFlag >= 0 ? args.splice(cspFlag, 2)[1] : undefined;
const root = resolve(args[0] ?? "");
if (!args[0] || !existsSync(join(root, "index.html"))) {
  console.error("usage: node smoke.mjs <demo directory> [--browser <chromium>]");
  process.exit(2);
}

const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".wasm": "application/wasm",
  ".json": "application/json",
  ".css": "text/css",
};
const BASE = "/demo/";

const server = createServer((request, response) => {
  const url = new URL(request.url, "http://localhost");
  if (!url.pathname.startsWith(BASE)) {
    response.writeHead(404).end();
    return;
  }
  let path = normalize(join(root, url.pathname.slice(BASE.length)));
  if (!path.startsWith(root)) {
    response.writeHead(403).end();
    return;
  }
  if (existsSync(path) && statSync(path).isDirectory()) path = join(path, "index.html");
  if (!existsSync(path)) {
    response.writeHead(404).end();
    return;
  }
  const headers = { "content-type": TYPES[extname(path)] ?? "application/octet-stream" };
  if (csp) headers["content-security-policy"] = csp;
  response.writeHead(200, headers);
  createReadStream(path).pipe(response);
});
await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
const origin = `http://127.0.0.1:${server.address().port}`;

// Trunk adds SRI hashes to its <link rel="preload"> tags; Chrome warns that
// preloads ignore them (the actual script and wasm loads still check them).
const BENIGN_WARNINGS = [/The `integrity` attribute is currently ignored for preload destinations/];

const failures = [];
function expect(condition, message) {
  console.log(`${condition ? "ok  " : "FAIL"} ${message}`);
  if (!condition) failures.push(message);
}

const browser = await chromium.launch(executablePath ? { executablePath } : {});
try {
  const page = await browser.newPage();
  const problems = [];
  const changes = [];
  const requests = [];
  page.on("console", (message) => {
    if (message.text().startsWith("change:")) changes.push(message.text());
    if (BENIGN_WARNINGS.some((pattern) => pattern.test(message.text()))) return;
    if (message.type() === "error" || message.type() === "warning") {
      problems.push(`${message.type()}: ${message.text()}`);
    }
  });
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  page.on("request", (request) => requests.push(request.url()));
  page.on("response", (response) => {
    if (response.status() >= 400) problems.push(`HTTP ${response.status()} ${response.url()}`);
  });
  page.on("worker", (worker) =>
    worker.on("console", (message) => {
      if (message.type() === "error") problems.push(`worker: ${message.text()}`);
    }),
  );
  await page.addInitScript(() => {
    window.__cspViolations = [];
    document.addEventListener("securitypolicyviolation", (event) =>
      window.__cspViolations.push(`${event.effectiveDirective} blocked ${event.blockedURI || "inline"}`),
    );
  });
  const output = async (expected) => {
    await page
      .waitForFunction((text) => document.querySelector("#output").textContent.includes(text), expected, {
        timeout: 2000,
      })
      .catch(() => {});
    return page.textContent("#output");
  };
  const settle = () => page.evaluate(() => new Promise((done) => setTimeout(done, 50)));

  await page.goto(`${origin}${BASE}`);
  await page.waitForSelector(".monaco-editor .view-lines");
  const box = await page.locator("#editor").boundingBox();
  expect(box.width > 0 && box.height > 0, `editor is visible (${box.width}x${box.height})`);

  // The page loads the runtime as a wasm-bindgen snippet; importing the same
  // URL yields the same module instance, which exposes leak counters.
  const runtimeUrl = await page.evaluate(() =>
    performance
      .getEntriesByType("resource")
      .map((entry) => entry.name)
      .find((name) => name.endsWith("/js/runtime.js")),
  );
  expect(runtimeUrl !== undefined, "runtime loaded from generated snippets");
  const counts = () =>
    page.evaluate(async (url) => {
      const runtime = await import(url);
      return [runtime.getEditorApi().getModels().length, runtime.activeRelayCount()];
    }, runtimeUrl);

  await page.click(".monaco-editor .view-lines");
  await page.keyboard.press("Control+End");
  await page.keyboard.type("oops");
  const diagnostics = await page
    .waitForSelector(".squiggly-error", { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  expect(diagnostics, "invalid JSON produces diagnostics from the JSON worker");

  await page.click("#save");
  expect((await output("oops")).endsWith("oops"), "Save returns typed text");

  await settle();
  expect(changes.length > 0, `change events delivered (${changes.length})`);

  await page.click("#replace");
  await page.click("#save");
  expect((await output('"replaced": true')).includes('"replaced": true'), "Replace document");
  await settle();
  expect(changes.at(-1)?.includes("is_flush: true"), "replacement reports is_flush");

  await page.click("#read-only");
  expect((await output("read-only: true")) === "read-only: true", "read-only toggle");
  await page.click(".monaco-editor .view-lines");
  await page.keyboard.type("Z");
  await page.click("#save");
  expect(!(await output("replaced")).includes("Z"), "read-only editor rejects typing");
  await page.click("#read-only");
  await output("read-only: false");

  // Mounting goes through the page's Mount/Unmount button, so framework
  // demos rerender as they would for a user.
  const editorShown = (shown) =>
    page.waitForFunction(
      (shown) => (document.querySelector("#editor .monaco-editor") !== null) === shown,
      shown,
    );
  const toggleMount = async (shown) => {
    await page.click("#mount");
    await editorShown(shown);
  };

  // Only the plain demo exposes its owner to JS, which allows unmounting in
  // the same task as a change; the framework adapters test this directly.
  const hasDemoApp = await page.evaluate(() => window.demoApp !== undefined);
  if (hasDemoApp) {
    const beforeDrop = changes.length;
    await page.evaluate(() => {
      window.demoApp.replaceDocument();
      window.demoApp.unmount();
    });
    await settle();
    expect(changes.length === beforeDrop, "event queued before disposal is dropped");
    await page.evaluate(() => window.demoApp.mount());
    await editorShown(true);
  }

  await toggleMount(false);
  expect(
    await page.evaluate(() => document.querySelector("#editor .monaco-editor") === null),
    "unmount removes the editor",
  );
  expect(JSON.stringify(await counts()) === "[0,0]", "unmount releases model and relay");

  await toggleMount(true);
  const baseline = await counts();
  for (let i = 0; i < 100; i++) {
    await toggleMount(false);
    await toggleMount(true);
  }
  await settle();
  const after = await counts();
  expect(
    JSON.stringify(after) === JSON.stringify(baseline),
    `100 mount/dispose cycles keep models/relays at baseline ${JSON.stringify(baseline)}`,
  );

  const external = requests.filter(
    (url) => !url.startsWith(origin) && !url.startsWith("blob:") && !url.startsWith("data:"),
  );
  expect(external.length === 0, `no external requests (${requests.length} total)`);
  if (csp) {
    const violations = await page.evaluate(() => window.__cspViolations);
    expect(violations.length === 0, `no CSP violations${violations.length ? `: ${[...new Set(violations)].join("; ")}` : ""}`);
  }
  expect(problems.length === 0, `no console errors or warnings${problems.length ? `: ${problems.join("; ")}` : ""}`);
} finally {
  await browser.close();
  server.close();
}

if (failures.length > 0) {
  console.error(`${failures.length} check(s) failed`);
  process.exit(1);
}
