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

The [documentation](https://docs.rs/monaco-leptos) covers the properties, reading and replacing
the text, changing options, server-side rendering and hydration, and deployment.

## License

Apache-2.0. The bundled Monaco runtime is MIT licensed.
