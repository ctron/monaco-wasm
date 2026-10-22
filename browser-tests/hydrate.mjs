// Checks the cargo-leptos SSR + hydration fixture (examples/leptos-hydrate).
//
//   node hydrate.mjs [http://127.0.0.1:3400/]
//
// Expects the fixture's server to be running (cargo leptos serve --release).
// Checks the server HTML (empty editor host, no editor or initial text), then
// hydrates in Chrome: exactly one editor, the component's DOM reused as
// rendered by the server, working event handlers, and no hydration or console
// warnings. Set CHROMIUM to use a system browser.

import { chromium } from "playwright-core";

const url = process.argv[2] ?? "http://127.0.0.1:3400/";
const INITIAL_TEXT = '"hydrated": true';
// The component's inner host, with no children. Leptos SSR may append a ";".
const EMPTY_HOST = /<div style="height: 100%; width: 100%;;?"><\/div>/;

// Serializes #editor without what Monaco adds to the host: its child nodes,
// data attributes, and CSS custom properties on the host's style. Styles are
// reserialized on both sides, so formatting differences (such as ";;") do
// not count.
function normalizedEditor() {
  const copy = document.getElementById("editor").cloneNode(true);
  const host = copy.querySelector(":scope > div > div");
  host.replaceChildren();
  for (const name of host.getAttributeNames()) {
    if (name !== "style") host.removeAttribute(name);
  }
  for (const property of [...host.style]) {
    if (property.startsWith("--")) host.style.removeProperty(property);
  }
  for (const element of copy.querySelectorAll("[style]")) {
    element.setAttribute("style", element.style.cssText);
  }
  return copy.innerHTML;
}

const failures = [];
const check = (ok, message) => {
  console.log(`${ok ? "ok  " : "FAIL"} ${message}`);
  if (!ok) failures.push(message);
};

const html = await (await fetch(url)).text();
check(EMPTY_HOST.test(html), "server renders an empty editor host");
check(!html.includes("monaco-editor"), "server renders no editor markup");
check(!html.includes(INITIAL_TEXT) && !html.includes("&quot;hydrated&quot;"), "initial text is not rendered as HTML");
check(!html.includes('role="alert"'), "server renders no error message");

const browser = await chromium.launch(process.env.CHROMIUM ? { executablePath: process.env.CHROMIUM } : {});
try {
  const page = await browser.newPage();
  const problems = [];
  page.on("console", (message) => {
    if (["error", "warning"].includes(message.type())) problems.push(`${message.type()}: ${message.text()}`);
  });
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  page.on("response", (response) => {
    if (response.status() >= 400) problems.push(`HTTP ${response.status()} ${response.url()}`);
  });

  // The server DOM as the browser parses it, before any client code runs.
  const serverPage = await browser.newPage({ javaScriptEnabled: false });
  await serverPage.goto(url);
  const serverEditor = await serverPage.evaluate(normalizedEditor);
  await serverPage.close();

  await page.goto(url);
  await page.waitForSelector(".monaco-editor .view-lines");
  await page.waitForTimeout(300);

  check((await page.locator(".monaco-editor").count()) === 1, "hydration creates exactly one editor");
  const text = await page.evaluate(() =>
    [...document.querySelectorAll(".view-line")].map((line) => line.textContent.replace(/ /g, " ")).join("\n"),
  );
  check(text.includes(INITIAL_TEXT), "editor shows the initial value from the client");
  check(
    await page.evaluate(() => document.querySelector("#editor > div > div").contains(document.querySelector(".monaco-editor"))),
    "editor is created inside the server-rendered host",
  );

  // Hydration must reuse the server DOM unchanged; only Monaco adds nodes,
  // and only inside the host.
  const hydratedEditor = await page.evaluate(normalizedEditor);
  check(
    hydratedEditor === serverEditor,
    `hydrated component matches the server DOM${hydratedEditor === serverEditor ? "" : `\n  server:   ${serverEditor}\n  hydrated: ${hydratedEditor}`}`,
  );

  await page.click("#increment");
  await page.click("#increment");
  check((await page.textContent("#increment")).trim() === "Clicked 2", "hydrated event handlers work");

  await page.click(".monaco-editor .view-lines");
  await page.keyboard.press("Control+End");
  await page.keyboard.type("x");
  const diagnostics = await page
    .waitForSelector(".squiggly-error", { timeout: 10000 })
    .then(() => true)
    .catch(() => false);
  check(diagnostics, "JSON worker runs after hydration");

  const hydrationWarnings = problems.filter((problem) => /hydrat/i.test(problem));
  check(hydrationWarnings.length === 0, `no hydration warnings${hydrationWarnings.length ? `: ${hydrationWarnings.join("; ")}` : ""}`);
  check(problems.length === 0, `no console errors or warnings${problems.length ? `: ${problems.join("; ")}` : ""}`);
} finally {
  await browser.close();
}

if (failures.length > 0) {
  console.error(`${failures.length} check(s) failed`);
  process.exit(1);
}
