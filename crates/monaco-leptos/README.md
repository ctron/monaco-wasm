# monaco-leptos

A [Leptos](https://leptos.dev) component for the
[Monaco editor](https://microsoft.github.io/monaco-editor/), built on
[`monaco-wasm`](https://crates.io/crates/monaco-wasm). The editor runtime and its workers are
bundled: add a Cargo dependency, with no npm install, CDN, or worker configuration.

[Live demo](https://ctron.github.io/monaco-wasm/leptos/) ·
[Quickstart project](https://github.com/ctron/monaco-wasm/tree/main/examples/quickstart-leptos)

Enable the same rendering mode as your Leptos build: `csr`, `hydrate`, or `ssr`.

```toml
[dependencies]
leptos = { version = "0.8", features = ["csr"] }
monaco-leptos = { version = "0.1", features = ["csr"] }
```

```rust
use leptos::prelude::*;
use monaco_leptos::MonacoEditor;

#[component]
fn App() -> impl IntoView {
    view! { <MonacoEditor language="json" initial_value="{}" /> }
}
```

- The editor owns its text: `initial_value` is read once, and nothing is written back. Use the
  `EditorHandle` from `on_ready` to read or replace the document, or remount the component.
- `language`, `options`, `class`, and `style` accept values or signals; changes update the
  existing editor in place.
- Server-side rendering emits an empty host and never runs browser code; the editor is created
  in the browser after hydration, and the initial text is not rendered as HTML.

See the [`monaco-wasm`](https://crates.io/crates/monaco-wasm) README for deployment and Content
Security Policy requirements.

## License

Apache-2.0. The bundled Monaco runtime is MIT licensed.
