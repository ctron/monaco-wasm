# Development

How to build, test, and release this repository. For using the crates, see the
[README](README.md).

## Repository layout

| Path | Contents |
| --- | --- |
| `crates/monaco-wasm` | Core crate: vendored runtime (`js/`), bindings (`src/sys/`), `Editor` API |
| `crates/monaco-yew`, `crates/monaco-leptos`, `crates/monaco-dioxus` | Framework components |
| `examples/wasm-basic`, `yew-basic`, `leptos-basic`, `dioxus-basic` | Minimal demos driven by the smoke test |
| `examples/leptos-hydrate`, `dioxus-hydrate` | SSR and hydration fixtures (cargo-leptos, dx) |
| `examples/quickstart-yew`, `quickstart-leptos`, `quickstart-dioxus` | Playgrounds, pinned on the GitHub Pages site |
| `examples/pages` | The landing page; `examples.json` lists every example, how it is built, and which are pinned as cards |
| `tools/runtime` | Builds the vendored Monaco runtime (Node, esbuild) |
| `tools/bindings` | Generates `src/sys/generated.rs` (ts-gen) |
| `browser-tests` | Playwright scripts for the demos and the site |

The workspace manifest sets version, edition, rust-version, license, repository, and homepage once
in `[workspace.package]`, and every dependency's version in `[workspace.dependencies]`. Members
inherit them with `.workspace = true` and only add features or `optional`. Yew and Leptos are
declared without default features there, so that each member selects its rendering mode.

## Building

Use a current stable Rust with the `wasm32-unknown-unknown` target; CI uses the latest stable.
The oldest supported Rust is `rust-version` in the workspace manifest (see below). Trunk resolves the crate from the current
directory, so run it inside the example.

```sh
cargo check -p monaco-wasm --target wasm32-unknown-unknown --locked
cargo test -p monaco-wasm -p monaco-yew -p monaco-leptos -p monaco-dioxus
wasm-pack build examples/wasm-basic --target web --out-dir pkg --no-pack
(cd examples/yew-basic && trunk build --release --public-url /demo/)
(cd examples/leptos-basic && trunk build --release --public-url /demo/)
(cd examples/dioxus-basic && trunk build --release --public-url /demo/)
```

A clean build of `monaco-wasm` takes about 65 s: wasm-bindgen copies the 5 MiB runtime snippet into
the WASM custom section through const evaluation. Incremental builds are unaffected. This cost is
accepted for now; alternatives (compressed workers, runtime evaluation, an upstream wasm-bindgen
fix) are deferred.

## Vendored runtime

Maintainers only; needs Node. Consumers never run this.

```sh
cd tools/runtime
npm ci
npm run runtime:build   # writes crates/monaco-wasm/js/
npm run runtime:check   # fails if the vendored files are stale
```

## Low-level bindings

Maintainers only; needs Node and cargo. The script installs the pinned ts-gen on demand. See
[`tools/bindings/DECISION.md`](tools/bindings/DECISION.md).

```sh
cd tools/bindings
npm ci
npm run bindings:generate   # writes crates/monaco-wasm/src/sys/generated.rs
npm run bindings:check
npm run bindings:update     # move ts-gen to upstream main
```

## Testing

Needs Node and Chrome or Chromium. The component tests also need `wasm-bindgen-cli` matching
`Cargo.lock` and a `chromedriver` matching the browser, via `PATH` or `CHROMEDRIVER`. Set
`CHROMIUM=/usr/bin/chromium-browser` to use a system browser for the Playwright scripts.

```sh
# Core API and the components, in headless Chrome
cargo test -p monaco-wasm -p monaco-yew -p monaco-leptos -p monaco-dioxus \
  --features monaco-dioxus/web --target wasm32-unknown-unknown

cd browser-tests && npm ci && cd ..
node browser-tests/smoke.mjs examples/wasm-basic          # after the wasm-pack build; --csp "<policy>" also
node browser-tests/smoke.mjs examples/yew-basic/dist      # after the Trunk builds
node browser-tests/smoke.mjs examples/leptos-basic/dist
node browser-tests/smoke.mjs examples/dioxus-basic/dist
tools/hydrate-check.sh leptos                             # SSR + hydration fixture (cargo-leptos), port 3400
tools/hydrate-check.sh dioxus                             # fullstack + hydration fixture (dx), port 3410

# The GitHub Pages site, as deployed (needs jq, Trunk, and wasm-pack)
tools/build-pages.sh site /monaco-wasm/
node browser-tests/pages.mjs site --base /monaco-wasm/
node browser-tests/quickstart.mjs site/yew --base /monaco-wasm/yew/
node browser-tests/quickstart.mjs site/leptos --base /monaco-wasm/leptos/
node browser-tests/quickstart.mjs site/dioxus --base /monaco-wasm/dioxus/
```

## CI and GitHub Pages

- `.github/workflows/ci.yml`: formatting, clippy (wasm, native, SSR) and tests on both the minimum
  Rust version (with a freshly resolved lockfile) and stable, semver checks against the latest
  crates.io release (cargo-semver-checks), the crates' browser tests, and the runtime and bindings
  checks. The `package` job packages the crates and uploads them; the `examples` matrix then
  replaces `crates/` with those packages in each job and builds and checks one example in Chrome,
  so every example runs against exactly what would be published. The final `ci` job succeeds only
  if all other jobs did; it is the check to require in branch protection. Add new jobs to its
  `needs`.
- `.github/workflows/pages.yml`: builds the site with `tools/build-pages.sh`, checks it, and
  deploys it on every push to `main`. Every example in `examples/pages/examples.json` with a
  `build` (`trunk` or `wasm-pack`) is published under its `id`; the landing page renders the
  pinned ones as cards and lists all of them. Examples without a `build` link to their source.

## Releasing

Releases use [cargo-release](https://github.com/crate-ci/cargo-release), configured in
`[workspace.metadata.release]` in `Cargo.toml`. From an up-to-date `main` with green CI:

```sh
cargo release minor            # dry run: shows what would change
cargo release minor --execute  # or patch / major / an explicit version
```

All crates share one version: cargo-release bumps `[workspace.package]` and the internal entries in
`[workspace.dependencies]`, commits, creates a single `v<version>` tag (for example `v0.2.0`),
and pushes. It does not publish.

The tag triggers `.github/workflows/release.yml`. It runs the full CI workflow on the tagged commit
(including the semver check of the new version against the previous release), checks that the tag
matches the version of every crate (setting the tag's version with `cargo-workspaces` must not
change any file), and only when both pass runs `cargo publish --workspace`: `monaco-wasm` first, then the components. It authenticates with
crates.io [trusted publishing](https://crates.io/docs/trusted-publishing): each of the four crates
has a trusted publisher on crates.io for the repository `ctron/monaco-wasm`, the workflow
`release.yml`, and the environment `release`, so no API token is stored.

The `release` environment only accepts `v*` tags.

## Minimum supported Rust

1.88, set as `rust-version` in the workspace manifest. `monaco-wasm` alone builds from 1.85; the
components need 1.88 (let chains, Leptos 0.8). With `rust-version` set, Cargo resolves dependency
versions a 1.88 user can build; CI checks this.

## Measured (2026-10-21)

| Item | Size |
| --- | --- |
| `js/runtime.js` raw / gzip / brotli | 5.0 MiB / 1.36 MiB / 0.96 MiB |
| `monaco-wasm-0.1.0.crate` | 1.4 MiB |
| example WASM after wasm-opt | 58 KiB |

Tested with rustc 1.99.0, wasm-bindgen 0.2.129, wasm-pack 0.15.0, Trunk 0.22.0, Yew 0.23.0,
Leptos 0.8.22, leptos_axum 0.8.10, cargo-leptos 0.3.7, Dioxus and dioxus-cli 0.7.10, esbuild 0.28.2, ts-gen 5341b86,
TypeScript 5.9.3, and Chromium.
