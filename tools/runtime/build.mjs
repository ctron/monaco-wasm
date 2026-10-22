// Builds the self-contained Monaco runtime vendored by the monaco-wasm crate.
//
//   node build.mjs           write crates/monaco-wasm/js/*
//   node build.mjs --check   rebuild in memory and fail if the vendored files differ
//
// The output is one standalone ES module with no imports, dynamic imports,
// import.meta URLs, or sibling assets. CSS and workers are embedded as strings.

import { createHash } from "node:crypto";
import { existsSync, readFileSync, writeFileSync, mkdirSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import * as esbuild from "esbuild";

const here = dirname(fileURLToPath(import.meta.url));
const outDir = join(here, "../../crates/monaco-wasm/js");
const monacoDir = join(here, "node_modules/monaco-editor");
const check = process.argv.includes("--check");

const MAIN_ENTRY = "src/editor.js";
const BRIDGE = "src/bridge.js";
const WORKER_ENTRIES = {
  editor: "node_modules/monaco-editor/esm/vs/editor/editor.worker.js",
  json: "node_modules/monaco-editor/esm/vs/languages/features/json/json.worker.js",
};
const RUNTIME_GLOBAL = "__monacoWasmRuntime";

// Monaco contains a few dynamic loading paths that are dead in this runtime,
// because the bridge always supplies MonacoEnvironment.getWorker and the
// diff editor is not part of the supported API. They are rewritten so that
// downstream bundlers never see a sibling URL or a runtime import().
//
// Every source patch must apply; a mismatch after a Monaco upgrade fails the
// build instead of silently shipping an unresolved import.
const WORKER_URL_PATTERN = /new URL\((['"])[^'"]+\1,\s*import\.meta\.url\)/g;
const SOURCE_PATCHES = {
  "editor/standalone/browser/services/standaloneWebWorkerService.js": [
    // Only an error message string mentions import.meta.url here.
    ['import.meta.url)', 'import_meta_url)'],
  ],
  "editor/common/diff/externalLinesDiffComputer.js": [
    [
      "import(/* webpackIgnore: true */ /* @vite-ignore */ `${url}`)",
      'Promise.reject(new Error("monaco-wasm: the external diff computer is not bundled"))',
    ],
  ],
  "platform/webWorker/browser/webWorkerServiceImpl.js": [
    [
      "`await import(ttPolicy?.createScriptURL(${JSON.stringify(workerScriptUrl)}) ?? ${JSON.stringify(workerScriptUrl)});`",
      '"throw new Error(\'monaco-wasm: URL-based workers are not supported\');"',
    ],
  ],
};

const sourcePatchPlugin = {
  name: "monaco-source-patches",
  setup(build) {
    build.onLoad({ filter: /\.js$/ }, (args) => {
      const path = args.path.replaceAll("\\", "/");
      const patchKey = Object.keys(SOURCE_PATCHES).find((key) => path.endsWith(`/esm/vs/${key}`));
      const source = readFileSync(args.path, "utf8");
      if (patchKey === undefined && !source.includes("import.meta")) {
        return undefined;
      }
      let contents = source.replace(
        WORKER_URL_PATTERN,
        '(() => { throw new Error("monaco-wasm: worker URLs are provided by MonacoEnvironment.getWorker"); })()',
      );
      for (const [from, to] of SOURCE_PATCHES[patchKey] ?? []) {
        if (!contents.includes(from)) {
          throw new Error(`source patch for ${patchKey} no longer applies: ${from}`);
        }
        contents = contents.replaceAll(from, to);
      }
      if (contents.includes("import.meta")) {
        throw new Error(`unhandled import.meta in ${relative(here, args.path)}`);
      }
      return { contents, loader: "js" };
    });
  },
};

const common = {
  absWorkingDir: here,
  bundle: true,
  write: false,
  minify: true,
  splitting: false,
  target: "es2022",
  platform: "browser",
  legalComments: "none",
  charset: "utf8",
  metafile: true,
  logLevel: "warning",
  plugins: [sourcePatchPlugin],
  // Only fonts are expected; any other asset type fails the build.
  loader: { ".ttf": "dataurl" },
};

async function bundle(entry, options = {}) {
  const result = await esbuild.build({
    ...common,
    entryPoints: [entry],
    format: "iife",
    outdir: "out",
    ...options,
  });
  const files = {};
  for (const file of result.outputFiles) {
    const ext = file.path.slice(file.path.lastIndexOf("."));
    if (files[ext] !== undefined || (ext !== ".js" && ext !== ".css")) {
      throw new Error(`unexpected build output ${file.path}`);
    }
    files[ext] = file.text;
  }
  return { files, inputs: Object.keys(result.metafile.inputs) };
}

function assertStandalone(name, code) {
  const forbidden = [
    [/import\.meta/, "import.meta"],
    [/\bimport\s*\(/, "dynamic import"],
    [/^\s*import[\s{*'"]/m, "static import"],
    [/\brequire\s*\(\s*['"]/, "require call"],
    [/sourceMappingURL/, "source map reference"],
  ];
  for (const [pattern, what] of forbidden) {
    if (pattern.test(code)) {
      throw new Error(`${name}: output contains ${what}`);
    }
  }
}

function assertCssAssetsInlined(css) {
  for (const match of css.matchAll(/url\(\s*(['"]?)([^'")]*)\1\s*\)/g)) {
    if (!match[2].startsWith("data:")) {
      throw new Error(`stylesheet references unhandled asset ${match[2]}`);
    }
  }
}

function sha256(text) {
  return createHash("sha256").update(text).digest("hex");
}

function bundledPackages(inputs) {
  const packages = new Set();
  for (const input of inputs) {
    const match = input.match(/node_modules\/((?:@[^/]+\/)?[^/]+)\//);
    if (match) {
      packages.add(match[1]);
    }
  }
  return [...packages].sort();
}

function packageNotices(name) {
  const dir = join(here, "node_modules", name);
  const pkg = JSON.parse(readFileSync(join(dir, "package.json"), "utf8"));
  const sections = [];
  for (const file of ["LICENSE", "LICENSE.md", "LICENSE.txt", "ThirdPartyNotices.txt"]) {
    const path = join(dir, file);
    if (existsSync(path)) {
      sections.push(readFileSync(path, "utf8").trim());
    }
  }
  if (sections.length === 0) {
    throw new Error(`no license text found for bundled package ${name}`);
  }
  const header = `${pkg.name} ${pkg.version} (${pkg.license})`;
  return `${header}\n${"=".repeat(header.length)}\n\n${sections.join("\n\n")}\n`;
}

async function build() {
  const main = await bundle(MAIN_ENTRY, { globalName: RUNTIME_GLOBAL });
  if (main.files[".css"] === undefined) {
    throw new Error("editor bundle produced no stylesheet");
  }
  assertStandalone("editor bundle", main.files[".js"]);
  assertCssAssetsInlined(main.files[".css"]);

  const workers = {};
  const inputs = new Set(main.inputs);
  for (const [kind, entry] of Object.entries(WORKER_ENTRIES)) {
    const worker = await bundle(entry);
    if (worker.files[".css"] !== undefined) {
      throw new Error(`${kind} worker produced a stylesheet`);
    }
    assertStandalone(`${kind} worker`, worker.files[".js"]);
    workers[kind] = worker.files[".js"];
    worker.inputs.forEach((input) => inputs.add(input));
  }

  const monacoVersion = JSON.parse(readFileSync(join(monacoDir, "package.json"), "utf8")).version;
  const bridge = readFileSync(join(here, BRIDGE), "utf8");
  const runtime = [
    `// Generated by tools/runtime/build.mjs from monaco-editor ${monacoVersion}. Do not edit.`,
    "// Third-party notices: THIRD_PARTY_NOTICES in this directory.",
    `const MONACO_CSS = ${JSON.stringify(main.files[".css"])};`,
    `const WORKER_SOURCES = { editor: ${JSON.stringify(workers.editor)}, json: ${JSON.stringify(workers.json)} };`,
    `function loadMonaco() {\n${main.files[".js"].trim()}\nreturn ${RUNTIME_GLOBAL};\n}`,
    bridge,
  ].join("\n");
  assertStandalone("runtime.js (excluding exports)", runtime.replace(/^export /gm, ""));

  const packages = bundledPackages([...inputs]);
  const notices =
    "Third-party software bundled in runtime.js by the monaco-wasm crate.\n\n" +
    packages.map(packageNotices).join("\n\n");

  const inputHashes = {};
  for (const file of ["build.mjs", "package-lock.json", MAIN_ENTRY, BRIDGE]) {
    inputHashes[file] = sha256(readFileSync(join(here, file)));
  }
  const manifest = {
    monacoEditor: monacoVersion,
    esbuild: esbuild.version,
    bundledPackages: Object.fromEntries(
      packages.map((name) => [
        name,
        JSON.parse(readFileSync(join(here, "node_modules", name, "package.json"), "utf8")).version,
      ]),
    ),
    entries: { editor: MAIN_ENTRY, workers: WORKER_ENTRIES },
    inputs: inputHashes,
    outputs: {
      "runtime.js": { sha256: sha256(runtime), bytes: Buffer.byteLength(runtime) },
      THIRD_PARTY_NOTICES: { sha256: sha256(notices), bytes: Buffer.byteLength(notices) },
    },
    sizes: {
      editorJs: Buffer.byteLength(main.files[".js"]),
      css: Buffer.byteLength(main.files[".css"]),
      editorWorker: Buffer.byteLength(workers.editor),
      jsonWorker: Buffer.byteLength(workers.json),
    },
  };

  return {
    "runtime.js": runtime,
    THIRD_PARTY_NOTICES: notices,
    "runtime-manifest.json": `${JSON.stringify(manifest, null, 2)}\n`,
  };
}

const outputs = await build();
if (check) {
  const stale = Object.entries(outputs)
    .filter(([name, text]) => {
      const path = join(outDir, name);
      return !existsSync(path) || readFileSync(path, "utf8") !== text;
    })
    .map(([name]) => name);
  if (stale.length > 0) {
    console.error(`vendored runtime is out of date: ${stale.join(", ")}; run npm run runtime:build`);
    process.exit(1);
  }
  console.log("vendored runtime is up to date");
} else {
  mkdirSync(outDir, { recursive: true });
  for (const [name, text] of Object.entries(outputs)) {
    writeFileSync(join(outDir, name), text);
  }
  const size = outputs["runtime.js"].length;
  console.log(`wrote ${relative(process.cwd(), outDir)} (runtime.js ${(size / 1024 / 1024).toFixed(2)} MiB)`);
}
