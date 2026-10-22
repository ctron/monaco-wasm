# monaco-yew quickstart

A small JSON playground using the `MonacoEditor` Yew component: themes, language, editor options,
JSON diagnostics, Format, Load sample, and Download. Everything comes from Cargo dependencies;
there is no npm install, CDN, or worker configuration.

## Run it

```sh
cargo install trunk                 # once
rustup target add wasm32-unknown-unknown
cd examples/quickstart-yew
trunk serve --open
```

`trunk build --release --public-url /my-path/` produces a static site in `dist/`. Deploy the whole
directory, including `snippets/`, which contains the bundled Monaco runtime. The repository's
`tools/build-pages.sh` builds it, together with the Leptos version and the landing page, for
GitHub Pages.

## Start your own project from it

1. Copy this directory.
2. In `Cargo.toml`, replace the `monaco-yew` path dependency with `monaco-yew = "0.1"`.
3. Keep `index.html` (the Trunk entry point) and edit `src/main.rs`.

The parts worth copying:

- `<MonacoEditor ... />` takes `language`, `initial_value`, `options`, `class`, `style`, and the
  `on_ready`, `on_change`, and `on_error` callbacks.
- Keep the `EditorHandle` from `on_ready` and read or replace text only when needed
  (`value()`, `replace_document()`). `on_change` carries a version, not the text.
- `set_theme` is global to all editors on the page.
- The component defaults to `height: 20rem`; this example passes `style="height: 100%"` inside a
  flex layout to fill the window.

Monaco needs `worker-src blob:`, `style-src 'unsafe-inline'`, and `font-src data:` if you deploy
with a Content Security Policy; see the repository README.
