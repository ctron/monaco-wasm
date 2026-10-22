# Binding generation decision

Status: **accepted — ts-gen over a typed facade**. Decided 2026-10-21.

## Result

| Input | Outcome |
| --- | --- |
| `esm/vs/editor/editor.api.d.ts` | Rejected. The `editor`/`languages` namespaces are declared without `export` and re-exported through a trailing `export { … }` list; ts-gen emits only 172 lines of top-level interfaces and none of the needed API. |
| `monaco.d.ts` (global `declare namespace monaco`) | Rejected. Generates 30k lines including every needed member, but namespace functions bind as global lookups (`js_namespace = "editor"`), which contradicts the runtime contract: the bundle sets no global and the API object comes from `getEditorApi()`. Also unresolved DOM types without `--external`, and unsupported mapped/conditional/indexed types erased to `JsValue`. |
| `fixtures/facade.d.ts` | **Accepted.** Structural interfaces for exactly the inventory below. Output: 280 lines, compiles for wasm32 and native without warnings, typed dictionaries with builders, a `try_*` (`catch`) variant of every method. |

`fixtures/facade-check.ts` type-checks the facade against upstream `monaco.d.ts` in both
directions (upstream results satisfy facade types; facade arguments satisfy upstream parameters).
Deliberate mismatches, such as `versionId: string`, a non-upstream `wordWrap` value, or
`getValue(): number`, fail the check.

Browser verification with the generated bindings (Chromium, `/demo/`): create, read, replace with
`isFlush`, option update, dispose, change events delivered in order, an event queued before an
unmount dropped, and 100 mount/dispose cycles returning model and relay counts to baseline.

## Pins and updates

- ts-gen: tracked from upstream `main` (the crates.io `0.1.0` release lags behind: it always emits
  a `PromiseExt` helper that needs `wasm-bindgen-futures` and maps arrays to `Array<T>`). The
  current rev is `config.tsGenRev` in `package.json`; the script installs exactly that rev into
  `.ts-gen/` on demand, so no manual `cargo install` is needed.
- `npm run bindings:update` moves the pin to upstream `HEAD` and regenerates. Review a changed
  `generated.rs` and rerun the browser checks before committing.
- monaco-editor `0.57.0` (must equal the vendored runtime; the script enforces this), TypeScript
  `5.9.3`. Hashes of every input and the output: `manifest.json`.

```sh
cd tools/bindings && npm ci
npm run bindings:generate   # type-check facade, write crates/monaco-wasm/src/sys/generated.rs
npm run bindings:check      # regenerate in memory, fail on differences
npm run bindings:update     # pin ts-gen to upstream main, then regenerate
```

History: evaluated at `967387b` (2026-10-21); moved to `5341b86` (2026-10-22) with byte-identical
output.

## What stays handwritten (`src/sys/manual.rs`)

- The bundled bridge (`getEditorApi`, `ChangeRelay`, `activeRelayCount`): ours, not Monaco's.
- `queueMicrotask`, used to defer releasing the relay closure.
- Ownership, liveness, and exception policy in `editor.rs`. TypeScript does not describe these.
  The safe API only calls the generated `try_*` methods.

## Notes for upgrades and extensions

- Add members to `facade.d.ts`, extend `facade-check.ts` to cover them, regenerate.
- Use named type aliases for string unions. ts-gen names anonymous unions after the first property
  it meets and reuses them across identical unions, which is why the facade declares
  `type OnOff = "off" | "on"`.
- Don't declare facade names that collide with globals in the configured `lib`; `tsconfig.json`
  sets `types: []` so ambient `@types` (e.g. Node's global `Disposable`) cannot leak in.
