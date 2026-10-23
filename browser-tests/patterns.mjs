// Checks the pattern examples (examples/yew-multiple, examples/yew-persist).
//
//   node patterns.mjs <dist directory> multiple|persist [--base /demo/]
//
// Serves the Trunk output under --base and drives the page: for multiple, two
// independent editors, copying between them, read-only, and the shared theme;
// for persist, saving to local storage after typing pauses, restoring on
// reload, the form submit, and resetting. Set CHROMIUM to use a system
// browser. Exits non-zero on failure.

import { createReadStream, existsSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, join, normalize, resolve } from "node:path";
import { chromium } from "playwright-core";

const args = process.argv.slice(2);
const option = (name, fallback) => {
  const index = args.indexOf(name);
  return index >= 0 ? args.splice(index, 2)[1] : fallback;
};
const basePath = option("--base", "/demo/");
const [dist, pattern] = args;
const root = resolve(dist ?? "");
if (!dist || !existsSync(join(root, "index.html")) || !["multiple", "persist"].includes(pattern)) {
  console.error("usage: node patterns.mjs <dist directory> multiple|persist [--base /path/]");
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
const url = `${origin}${basePath}`;

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
const page = await browser.newPage();
page.on("console", (m) => {
  if (["error", "warning"].includes(m.type()) && !BENIGN.test(m.text())) problems.push(m.text());
});
page.on("pageerror", (error) => problems.push(error.message));
page.on("response", (response) => {
  if (response.status() >= 400) problems.push(`HTTP ${response.status()} ${response.url()}`);
});
page.on("request", (request) => {
  const requested = request.url();
  if (!requested.startsWith(origin) && !/^(blob|data):/.test(requested)) external.push(requested);
});

// The text of every Monaco model on the page, through the runtime snippet
// (importing the page's URL yields the page's module instance). A local
// target/ can hold stale snippet copies that Trunk preloads too; the one the
// app imported is the one with models.
const texts = () =>
  page.evaluate(async () => {
    const runtimeUrls = performance
      .getEntriesByType("resource")
      .map((entry) => entry.name)
      .filter((name) => name.endsWith("/js/runtime.js"));
    for (const runtimeUrl of runtimeUrls) {
      const models = (await import(runtimeUrl)).getEditorApi().getModels();
      if (models.length > 0) return models.map((model) => model.getValue());
    }
    return [];
  });
const typeAtEnd = async (editor, text) => {
  await page.click(`${editor} .monaco-editor .view-lines`);
  await page.keyboard.press("Control+End");
  await page.keyboard.type(text);
};
const waitFor = (predicate, arg) =>
  page
    .waitForFunction(predicate, arg, { timeout: 5000 })
    .then(() => true)
    .catch(() => false);
const textIs = (selector, expected) =>
  waitFor(([selector, expected]) => document.querySelector(selector)?.textContent === expected, [selector, expected]);
const changes = async (editor) => Number((await page.textContent(`${editor} .edits`)).replace("Changes: ", ""));

async function multiple() {
  await page.goto(url);
  await page.waitForSelector("#published .monaco-editor .view-lines");
  check((await page.locator(".monaco-editor").count()) === 2, "two editors on the page");

  await typeAtEnd("#draft", "zzz");
  check(
    await waitFor(() => document.querySelector("#draft .edits").textContent !== "Changes: 0"),
    "the draft counts its own changes",
  );
  check((await changes("#published")) === 0, "the published editor sees none of them");
  let [draft, published] = await texts();
  check(draft.endsWith("zzz") && !published.includes("zzz"), "documents are independent");

  await page.click("#publish");
  check(await textIs("#published .edits", "Changes: 1"), "publishing replaces the published document");
  [draft, published] = await texts();
  check(published === draft, "published text equals the draft");

  await typeAtEnd("#published", "nope");
  [, published] = await texts();
  check(!published.includes("nope"), "the published editor is read-only");

  await typeAtEnd("#draft", "yyy");
  await page.click("#revert");
  [draft, published] = await texts();
  check(draft === published && !draft.includes("yyy"), "revert copies the published text back");

  check((await page.inputValue("#theme")) === "vs", "the theme menu starts on the editors' light theme");
  await page.selectOption("#theme", "vs-dark");
  check(
    await waitFor(() => [...document.querySelectorAll(".monaco-editor")].every((e) => e.classList.contains("vs-dark"))),
    "the theme applies to both editors",
  );
}

async function persist() {
  await page.goto(url);
  await page.waitForSelector(".monaco-editor .view-lines");
  check(await textIs("#status", "Nothing saved yet"), "starts with nothing saved");

  await typeAtEnd("form", "zzz");
  check(await textIs("#status", "Unsaved changes"), "typing marks the text unsaved");
  check(await textIs("#status", "Saved to local storage"), "saved once typing pauses");
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem("monaco-yew-persist")));
  check(saved?.endsWith("zzz"), "local storage holds the document");

  await page.reload();
  await page.waitForSelector(".monaco-editor .view-lines");
  check(await textIs("#status", "Restored from local storage"), "reload restores the document");
  check((await texts())[0]?.endsWith("zzz"), "the restored document has the edit");

  await page.fill("#title", "Report");
  await page.click("#submit");
  const submitted = await page.textContent("#submitted").catch(() => "");
  check(submitted.startsWith("title: Report\ndocument:\n") && submitted.endsWith("zzz"), "submit reads the title and the document");
  check(new URL(page.url()).pathname === basePath, "submit stays on the page");

  await page.click("#reset");
  check(await textIs("#status", "Saved to local storage"), "the reset document is saved");
  await page.reload();
  await page.waitForSelector(".monaco-editor .view-lines");
  check(!(await texts())[0]?.includes("zzz"), "reload after reset shows the default document");
}

try {
  await (pattern === "multiple" ? multiple() : persist());
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
