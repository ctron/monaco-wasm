// Maintainer-only build. Consumers receive a standalone wasm-bindgen snippet.
import { sourcePatchPlugin } from "../runtime/source-patches.mjs";
import * as esbuild from "esbuild";
import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createHash } from "node:crypto";
const here = dirname(fileURLToPath(import.meta.url));
const out = join(here, "../../crates/monaco-yaml/js");
const options = {
  absWorkingDir: here,
  bundle: true,
  write: false,
  minify: true,
  target: "es2022",
  platform: "browser",
  format: "iife",
  metafile: true,
  plugins: [
    {
      name: "monaco-paths",
      setup(build) {
        build.onResolve({ filter: /editor\.api\.js$/ }, () => ({
          path: "monaco",
          namespace: "shared-monaco",
        }));
        build.onLoad({ filter: /.*/, namespace: "shared-monaco" }, () => ({
          contents: "export const languages = __monaco.languages;",
        }));
        build.onResolve({ filter: /^monaco-editor\/esm\/vs\// }, (args) => ({
          path: join(here, "node_modules", args.path),
        }));
      },
    },
  ],
  legalComments: "none",
};
const main = await esbuild.build({
  ...options,
  entryPoints: ["src/main.js"],
  globalName: "yamlModule",
});
// The worker has its own Monaco worker implementation, not a second editor runtime.
const worker = await esbuild.build({
  ...options,
  entryPoints: ["node_modules/monaco-yaml/yaml.worker.js"],
  plugins: [...options.plugins, sourcePatchPlugin],
});
for (const result of [main, worker]) {
  if (result.warnings.length) throw new Error("unexpected bundle warnings");
  // Prettier's parser contains this spelling as data, not executable code.
  const executable = result.outputFiles[0].text.replaceAll(
    '"import.meta.resolve"',
    '"parser-spelling"',
  );
  if (/import\.meta|\bimport\s*\(/.test(executable))
    throw new Error("unresolved runtime import");
}
if (
  Object.keys(main.metafile.inputs).some((p) =>
    p.includes("monaco-editor/esm/vs/editor/"),
  )
)
  throw new Error("YAML bundles a second editor");
const source = `function loadModule(__monaco) {\n${main.outputFiles[0].text}\nreturn yamlModule;\n}\nconst workerSource = ${JSON.stringify(worker.outputFiles[0].text)};\nlet workerUrl;\nexport function createWorker() {\n  return () => {\n    workerUrl ??= URL.createObjectURL(new Blob([workerSource], { type: 'text/javascript' }));\n    return new Worker(workerUrl, { name: 'yaml' });\n  };\n}\nexport function install(monaco) { loadModule(monaco).install(monaco); }\n`;
const names = new Set();
for (const result of [main, worker])
  for (const path of Object.keys(result.metafile.inputs)) {
    const match = path.match(/node_modules\/((?:@[^/]+\/)?[^/]+)/);
    if (match) names.add(match[1]);
  }
let notices = "Third-party software bundled in monaco-yaml.\n\n";
const versions = {};
for (const name of [...names].sort()) {
  const dir = join(here, "node_modules", name);
  const pkg = JSON.parse(readFileSync(join(dir, "package.json")));
  versions[name] = pkg.version;
  const license = [
    "LICENSE",
    "LICENSE.txt",
    "LICENSE.md",
    "license",
    "License.txt",
  ].find((f) => existsSync(join(dir, f)));
  if (!license && name !== "@vscode/l10n")
    throw new Error(`missing license for ${name}`);
  notices += `${name} ${pkg.version}\n${readFileSync(license ? join(dir, license) : join(here, "l10n-LICENSE"), "utf8")}\n\n`;
  const thirdParty = join(dir, "ThirdPartyNotices.txt");
  if (existsSync(thirdParty))
    notices += readFileSync(thirdParty, "utf8") + "\n\n";
}
const hash = (text) => createHash("sha256").update(text).digest("hex");
const manifest = {
  packages: versions,
  inputs: Object.fromEntries(
    [
      "build.mjs",
      "package-lock.json",
      "src/main.js",
      "l10n-LICENSE",
      "../runtime/source-patches.mjs",
    ].map((f) => [f, hash(readFileSync(join(here, f)))]),
  ),
  bytes: {
    main: main.outputFiles[0].contents.length,
    worker: worker.outputFiles[0].contents.length,
  },
  sha256: hash(source),
};
const files = {
  "yaml.js": source,
  THIRD_PARTY_NOTICES: notices,
  "runtime-manifest.json": JSON.stringify(manifest, null, 2) + "\n",
};
mkdirSync(out, { recursive: true });
for (const [name, text] of Object.entries(files)) {
  const path = join(out, name);
  if (process.argv.includes("--check")) {
    if (!existsSync(path) || readFileSync(path, "utf8") !== text)
      throw new Error(`stale ${name}`);
  } else writeFileSync(path, text);
}
console.log(
  `YAML main ${manifest.bytes.main} bytes, worker ${manifest.bytes.worker} bytes`,
);
