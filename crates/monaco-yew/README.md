# monaco-yew

A [Yew](https://yew.rs) component for the [Monaco editor](https://microsoft.github.io/monaco-editor/),
built on [`monaco-wasm`](https://crates.io/crates/monaco-wasm). The editor runtime and its workers
are bundled: add a Cargo dependency, with no npm install, CDN, or worker configuration.

[Live demo](https://ctron.github.io/monaco-wasm/yew/) ·
[Quickstart project](https://github.com/ctron/monaco-wasm/tree/main/examples/quickstart-yew)

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

- The editor owns its text: `initial_value` is read once, and nothing is written back on rerender.
  Use the `EditorHandle` from `on_ready` to read or replace the document, or remount with a new
  `key`.
- `language` and `options` update the existing editor in place; `on_change` reports changes,
  `on_error` reports setup and update failures.
- The container defaults to `height: 20rem; width: 100%`; a `style` prop is appended and can
  override it.

Supports client-side rendering. See the [`monaco-wasm`](https://crates.io/crates/monaco-wasm)
README for deployment and Content Security Policy requirements.

## License

Apache-2.0. The bundled Monaco runtime is MIT licensed.
