// Small bridge imported by the monaco-wasm crate through wasm-bindgen.
//
// build.mjs prepends three generated declarations to this file:
//   MONACO_CSS      editor stylesheet text, with fonts inlined as data URLs
//   WORKER_SOURCES  { editor, json }: self-contained classic worker scripts
//   loadMonaco()    evaluates the bundled Monaco module and returns its API
//
// Importing this module performs no browser work. Everything happens on the
// first getEditorApi() call.

const ENVIRONMENT_MARKER = Symbol.for("monaco-wasm.environment");

let editorApi = null;
const workerUrls = new Map();
let activeRelays = 0;

function workerKind(label) {
  switch (label) {
    case "editorWorkerService":
      return "editor";
    case "json":
      return "json";
    default:
      return null;
  }
}

function workerUrl(kind) {
  let url = workerUrls.get(kind);
  if (url === undefined) {
    const blob = new Blob([WORKER_SOURCES[kind]], { type: "text/javascript" });
    url = URL.createObjectURL(blob);
    workerUrls.set(kind, url);
  }
  return url;
}

function installEnvironment() {
  const existing = globalThis.MonacoEnvironment;
  if (existing !== undefined) {
    if (existing && existing[ENVIRONMENT_MARKER]) {
      return;
    }
    throw new Error(
      "monaco-wasm: a foreign MonacoEnvironment is already defined. " +
        "The bundled runtime cannot share a page with a separately configured Monaco.",
    );
  }
  globalThis.MonacoEnvironment = {
    [ENVIRONMENT_MARKER]: true,
    getWorker(_moduleId, label) {
      const kind = workerKind(label);
      if (kind === null) {
        throw new Error(`monaco-wasm: no bundled worker for label "${label}"`);
      }
      return new Worker(workerUrl(kind), { name: label });
    },
  };
}

function installStyles() {
  const style = document.createElement("style");
  style.setAttribute("data-monaco-wasm", "");
  style.textContent = MONACO_CSS;
  document.head.appendChild(style);
}

/**
 * Initializes the runtime on first use and returns the `monaco.editor` API.
 * Throws if the page is not a browser document or the environment conflicts.
 */
export function getEditorApi() {
  if (editorApi !== null) {
    return editorApi;
  }
  if (typeof document === "undefined" || typeof Worker === "undefined") {
    throw new Error("monaco-wasm: a browser document with Web Worker support is required");
  }
  installEnvironment();
  installStyles();
  editorApi = loadMonaco().editor;
  return editorApi;
}

/**
 * Forwards model content changes to `callback(versionId, isFlush)` from a
 * microtask, in FIFO order. After `detach()`, queued and future events are
 * dropped and the callback reference is released.
 */
export class ChangeRelay {
  constructor(callback) {
    this.callback = callback;
    this.active = true;
    activeRelays += 1;
  }

  subscribe(model) {
    return model.onDidChangeContent((event) => {
      if (!this.active) {
        return;
      }
      const versionId = event.versionId;
      const isFlush = event.isFlush;
      queueMicrotask(() => {
        const callback = this.callback;
        if (this.active && callback !== null) {
          callback(versionId, isFlush);
        }
      });
    });
  }

  detach() {
    if (this.active) {
      this.active = false;
      this.callback = null;
      activeRelays -= 1;
    }
  }
}

/** Number of relays not yet detached; used by leak tests. */
export function activeRelayCount() {
  return activeRelays;
}
