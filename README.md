# monaco-wasm

Monaco editor for Rust/WASM, with a Yew component and a Leptos component planned. See
[DESIGN.md](DESIGN.md).

Status: milestones 1–3 complete (package proof, core API, Yew). The Leptos adapter is next.

Try it: [`examples/quickstart`](examples/quickstart) is a JSON playground built with the Yew
component. `.github/workflows/pages.yml` publishes it to GitHub Pages.

## Using it

```toml
[dependencies]
monaco-wasm = "0.1"
```

The Monaco 0.57.0 runtime, stylesheet, and editor/JSON workers are bundled in the crate. No npm,
CDN, or worker configuration is needed. Deploy the whole build output directory, including the
generated `snippets/`.

Requirements for the page:

- Content Security Policy: monaco-wasm needs `worker-src blob:` (bundled workers),
  `style-src 'unsafe-inline'` (Monaco injects its stylesheet and inline styles) and
  `font-src data:` (the inlined icon font). Measured with Chromium; this policy works for the plain
  example and is checked by `tools/consumer-check.sh`:

  ```text
  default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self';
  worker-src blob:; style-src 'self' 'unsafe-inline'; font-src data:; img-src data:
  ```

  `script-src 'wasm-unsafe-eval'` and `connect-src 'self'` are needed by any wasm-bindgen
  application; `img-src data:` only by the example's favicon. Policies without these mechanisms
  are not supported yet.
- No other `MonacoEnvironment` may be defined on the page; editor creation then fails with a setup
  error instead of overwriting it.

Yew:

```toml
[dependencies]
yew = { version = "0.23", features = ["csr"] }
monaco-yew = "0.1"
```

```rust
use monaco_yew::MonacoEditor;
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! { <MonacoEditor language="json" initial_value="{}" /> }
}
```

The component owns its text: `initial_value` is read once, and nothing is written back on
rerender. Use the `EditorHandle` from `on_ready` to read or replace the document, or remount with a
new `key`. The container defaults to `height: 20rem; width: 100%`; a `style` prop is appended.

## Development

```sh
cargo check -p monaco-wasm --target wasm32-unknown-unknown --locked
cargo test -p monaco-wasm
wasm-pack build examples/wasm-basic --target web --out-dir pkg --no-pack
# serve examples/wasm-basic/ (e.g. under /demo/) and open index.html
(cd examples/yew-basic && trunk build --release --public-url /demo/)   # Trunk resolves the crate from the current directory
```

Rebuilding the vendored runtime (maintainers only, needs Node):

```sh
cd tools/runtime
npm ci
npm run runtime:build   # writes crates/monaco-wasm/js/
npm run runtime:check   # fails if the vendored files are stale
```

Regenerating the low-level bindings (maintainers only; needs Node and cargo, installs the pinned
ts-gen on demand; see `tools/bindings/DECISION.md`):

```sh
cd tools/bindings
npm ci
npm run bindings:generate   # writes crates/monaco-wasm/src/sys/generated.rs
npm run bindings:check
npm run bindings:update     # move ts-gen to upstream main
```

Testing (needs Node, Chrome/Chromium, and for the core tests `wasm-bindgen-cli` matching
`Cargo.lock` plus a `chromedriver` matching the browser, via PATH or `CHROMEDRIVER`):

```sh
cargo test -p monaco-wasm -p monaco-yew --target wasm32-unknown-unknown   # core API and Yew component in headless Chrome
cd browser-tests && npm ci && cd ..                         # CHROMIUM=/usr/bin/chromium-browser for a system browser
node browser-tests/smoke.mjs examples/wasm-basic          # after the wasm-pack build above
node browser-tests/smoke.mjs examples/yew-basic/dist      # after the Trunk build above
node browser-tests/quickstart.mjs examples/quickstart/dist --base /monaco-yew/   # after trunk build --public-url /monaco-yew/
tools/consumer-check.sh                                   # packaged consumer, Node stubbed out, with and without CSP
```

Node is needed only by maintainers (vendored runtime, bindings, tests). `tools/consumer-check.sh`
verifies that building against the packaged crate never invokes it.

## Measured (bootstrap, 2026-10-21)

| Item | Size |
| --- | --- |
| `js/runtime.js` raw / gzip / brotli | 5.0 MiB / 1.36 MiB / 0.96 MiB |
| `monaco-wasm-0.1.0.crate` | 1.4 MiB |
| example WASM after wasm-opt | 58 KiB |

Known cost: wasm-bindgen copies the 5 MiB snippet into the WASM custom section through const
evaluation, so a clean build of `monaco-wasm` takes about 65 s (rustc 1.99.0, wasm-bindgen
0.2.129). Incremental builds are unaffected. This is accepted for now (2026-10-21); alternatives
(compressed workers, runtime evaluation, an upstream wasm-bindgen fix) are deferred.

Tested with rustc 1.99.0, wasm-bindgen 0.2.129, wasm-pack 0.15.0, Trunk 0.22.0, Yew 0.23.0,
esbuild 0.28.2, ts-gen 5341b86,
TypeScript 5.9.3, Chromium.
