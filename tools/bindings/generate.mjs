// Generates crates/monaco-wasm/src/sys/generated.rs from fixtures/facade.d.ts.
//
//   node generate.mjs            type-check the facade, regenerate, update manifest.json
//   node generate.mjs --check    regenerate in memory and fail on differences
//   node generate.mjs --update   pin ts-gen to the current upstream main, then regenerate
//
// ts-gen is installed on demand into .ts-gen/ at the rev pinned in
// package.json (`config.tsGenRev`). TS_GEN=/path/to/ts-gen overrides this.

import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "../..");
const check = process.argv.includes("--check");
const update = process.argv.includes("--update");

const PACKAGE = join(here, "package.json");
const FACADE = "fixtures/facade.d.ts";
const FACADE_CHECK = "fixtures/facade-check.ts";
const UPSTREAM = "node_modules/monaco-editor/monaco.d.ts";
const OUTPUT = join(repo, "crates/monaco-wasm/src/sys/generated.rs");
const MANIFEST = join(here, "manifest.json");
const EXTERNALS = ["HTMLElement=::web_sys::HtmlElement"];
const INSTALL_ROOT = join(here, ".ts-gen");

function sha256(path) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

function version(path) {
  return JSON.parse(readFileSync(path, "utf8")).version;
}

function run(command, args, options = {}) {
  return execFileSync(command, args, { encoding: "utf8", ...options });
}

const pkg = JSON.parse(readFileSync(PACKAGE, "utf8"));
const { tsGenRepository } = pkg.config;

if (update) {
  const head = run("git", ["ls-remote", tsGenRepository, "HEAD"]).split(/\s/)[0];
  if (!/^[0-9a-f]{40}$/.test(head)) {
    throw new Error(`could not resolve ts-gen HEAD: ${head}`);
  }
  if (head !== pkg.config.tsGenRev) {
    console.log(`ts-gen ${pkg.config.tsGenRev.slice(0, 7)} -> ${head.slice(0, 7)}`);
    pkg.config.tsGenRev = head;
    writeFileSync(PACKAGE, `${JSON.stringify(pkg, null, 2)}\n`);
  } else {
    console.log(`ts-gen is already at upstream HEAD ${head.slice(0, 7)}`);
  }
}
const { tsGenRev } = pkg.config;

// cargo records the installed source, including the git rev, in .crates.toml.
function installedRev() {
  const record = join(INSTALL_ROOT, ".crates.toml");
  if (!existsSync(record)) return null;
  return readFileSync(record, "utf8").match(/ts-gen [^ ]+ \(git\+[^#]+#([0-9a-f]{40})\)/)?.[1] ?? null;
}

function tsGenBinary() {
  if (process.env.TS_GEN) return process.env.TS_GEN;
  if (installedRev() !== tsGenRev) {
    console.log(`installing ts-gen ${tsGenRev.slice(0, 7)} into ${relative(process.cwd(), INSTALL_ROOT)}`);
    run(
      "cargo",
      ["install", "ts-gen", "--git", tsGenRepository, "--rev", tsGenRev, "--locked", "--force", "--root", INSTALL_ROOT],
      { stdio: "inherit" },
    );
  }
  return join(INSTALL_ROOT, "bin", process.platform === "win32" ? "ts-gen.exe" : "ts-gen");
}

// The bindings must describe the same Monaco release as the vendored runtime.
const monacoVersion = version(join(here, "node_modules/monaco-editor/package.json"));
const runtimeVersion = JSON.parse(
  readFileSync(join(repo, "crates/monaco-wasm/js/runtime-manifest.json"), "utf8"),
).monacoEditor;
if (monacoVersion !== runtimeVersion) {
  throw new Error(`monaco-editor ${monacoVersion} differs from the vendored runtime ${runtimeVersion}`);
}

execFileSync(process.execPath, [join(here, "node_modules/typescript/bin/tsc"), "--project", "tsconfig.json"], {
  cwd: here,
  stdio: "inherit",
});

const tsGen = tsGenBinary();
const tsGenVersion = run(tsGen, ["--version"]).trim();
const temp = mkdtempSync(join(tmpdir(), "monaco-wasm-bindings-"));
let generated;
try {
  const out = join(temp, "generated.rs");
  execFileSync(tsGen, ["--input", FACADE, "--output", out, ...EXTERNALS.flatMap((e) => ["--external", e])], {
    cwd: here,
    stdio: ["ignore", "ignore", "inherit"],
  });
  generated = readFileSync(out, "utf8");
} finally {
  rmSync(temp, { recursive: true, force: true });
}

const manifest = {
  tsGen: { repository: tsGenRepository, rev: tsGenRev, version: tsGenVersion },
  command: `ts-gen --input ${FACADE} --output ${relative(here, OUTPUT)} ${EXTERNALS.map((e) => `--external ${e}`).join(" ")}`,
  monacoEditor: monacoVersion,
  typescript: version(join(here, "node_modules/typescript/package.json")),
  inputs: {
    [FACADE]: sha256(join(here, FACADE)),
    [FACADE_CHECK]: sha256(join(here, FACADE_CHECK)),
    "tsconfig.json": sha256(join(here, "tsconfig.json")),
    [UPSTREAM]: sha256(join(here, UPSTREAM)),
  },
  output: createHash("sha256").update(generated).digest("hex"),
};
const manifestText = `${JSON.stringify(manifest, null, 2)}\n`;

if (check) {
  const stale = [];
  if (readFileSync(OUTPUT, "utf8") !== generated) stale.push(relative(repo, OUTPUT));
  if (readFileSync(MANIFEST, "utf8") !== manifestText) stale.push(relative(repo, MANIFEST));
  if (stale.length > 0) {
    console.error(`generated bindings are out of date: ${stale.join(", ")}; run npm run bindings:generate`);
    process.exit(1);
  }
  console.log("generated bindings are up to date");
} else {
  writeFileSync(OUTPUT, generated);
  writeFileSync(MANIFEST, manifestText);
  console.log(`wrote ${relative(process.cwd(), OUTPUT)}`);
}
