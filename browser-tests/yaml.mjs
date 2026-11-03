import { createReadStream, existsSync, statSync, readFileSync } from "node:fs";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright-core";

const args = process.argv.slice(2);
const browserFlag = args.indexOf("--browser");
const executablePath =
  browserFlag >= 0 ? args.splice(browserFlag, 2)[1] : process.env.CHROMIUM;
const cspFlag = args.indexOf("--csp");
let csp = cspFlag >= 0 ? args.splice(cspFlag, 2)[1] : undefined;
const root = resolve(args[0] ?? "");
if (!args[0] || !existsSync(join(root, "index.html"))) {
  console.error(
    "usage: node smoke.mjs <demo directory> [--browser <chromium>]",
  );
  process.exit(2);
}

// Authorize only Trunk's generated bootstrap; workers still run under the
// supplied policy, without allowing arbitrary inline scripts.
if (csp) {
  const html = readFileSync(join(root, "index.html"), "utf8");
  const hashes = [
    ...html.matchAll(/<script type="module">([\s\S]*?)<\/script>/g),
  ].map(
    ([, source]) =>
      `'sha256-${createHash("sha256").update(source).digest("base64")}'`,
  );
  csp = csp.replace(
    /script-src[^;]*/,
    (directive) => `${directive} ${hashes.join(" ")}`,
  );
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
  if (existsSync(path) && statSync(path).isDirectory())
    path = join(path, "index.html");
  if (!existsSync(path)) {
    response.writeHead(404).end();
    return;
  }
  const headers = {
    "content-type": TYPES[extname(path)] ?? "application/octet-stream",
  };
  if (csp) headers["content-security-policy"] = csp;
  response.writeHead(200, headers);
  createReadStream(path).pipe(response);
});
await new Promise((ready) => server.listen(0, "127.0.0.1", ready));
const origin = `http://127.0.0.1:${server.address().port}`;

// Trunk adds SRI hashes to its <link rel="preload"> tags; Chrome warns that
// preloads ignore them (the actual script and wasm loads still check them).
const BENIGN_WARNINGS = [
  /The `integrity` attribute is currently ignored for preload destinations/,
];

const failures = [];
function expect(condition, message) {
  console.log(`${condition ? "ok  " : "FAIL"} ${message}`);
  if (!condition) failures.push(message);
}

const browser = await chromium.launch(executablePath ? { executablePath } : {});
try {
  const page = await browser.newPage();
  const problems = [];
  const external = [];
  page.on("pageerror", (error) => problems.push(error.message));
  page.on("console", (message) => {
    if (
      message.type() === "error" ||
      (message.type() === "warning" &&
        !BENIGN_WARNINGS.some((p) => p.test(message.text())))
    )
      problems.push(message.text());
  });
  page.on("request", (request) => {
    if (
      !request.url().startsWith(origin) &&
      !/^(blob:|data:)/.test(request.url())
    )
      external.push(request.url());
  });
  await page.goto(`${origin}${BASE}`);
  await page.waitForSelector(".monaco-editor .view-lines").catch((error) => {
    throw new Error(`${error.message}\nBrowser errors: ${problems.join("; ")}`);
  });
  await page.click(".monaco-editor .view-lines");
  await page.keyboard.press("Control+End");
  await page.keyboard.type("\nbroken: ]");
  await page
    .waitForSelector(".squiggly-error", { timeout: 15000 })
    .catch(async (error) => {
      throw new Error(
        `${error.message}\nBrowser errors: ${problems.join("; ")}\nText: ${await page.locator(".view-lines").innerText()}`,
      );
    });
  expect(true, "YAML worker reports invalid syntax");
  await page.keyboard.press("Control+A");
  await page.keyboard.type("valid: true");
  await page.waitForFunction(() => !document.querySelector(".squiggly-error"));
  expect(true, "correcting YAML clears diagnostics");
  expect(problems.length === 0, `no browser errors: ${problems.join("; ")}`);
  expect(external.length === 0, `no external requests: ${external.join("; ")}`);
} finally {
  await browser.close();
  server.close();
}
if (failures.length) process.exitCode = 1;
