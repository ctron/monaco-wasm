// Shared patches for standalone Monaco worker bundles.
import { readFileSync } from "node:fs";
const WORKER_URL_PATTERN = /new URL\((['"])[^'"]+\1,\s*import\.meta\.url\)/g;
const SOURCE_PATCHES = {
  "editor/standalone/browser/services/standaloneWebWorkerService.js": [
    // Only an error message string mentions import.meta.url here.
    ["import.meta.url)", "import_meta_url)"],
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
      "\"throw new Error('monaco-wasm: URL-based workers are not supported');\"",
    ],
  ],
};

export const sourcePatchPlugin = {
  name: "monaco-source-patches",
  setup(build) {
    build.onLoad({ filter: /\.js$/ }, (args) => {
      const path = args.path.replaceAll("\\", "/");
      const patchKey = Object.keys(SOURCE_PATCHES).find((key) =>
        path.endsWith(`/esm/vs/${key}`),
      );
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
          throw new Error(
            `source patch for ${patchKey} no longer applies: ${from}`,
          );
        }
        contents = contents.replaceAll(from, to);
      }
      if (contents.includes("import.meta")) {
        throw new Error(`unhandled import.meta in ${args.path}`);
      }
      return { contents, loader: "js" };
    });
  },
};
