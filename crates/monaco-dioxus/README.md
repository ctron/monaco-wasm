# monaco-dioxus

A [Dioxus](https://dioxuslabs.com) component for the
[Monaco editor](https://microsoft.github.io/monaco-editor/), built on
[`monaco-wasm`](https://crates.io/crates/monaco-wasm). The editor runtime and its workers are
bundled: add a Cargo dependency, with no npm install, CDN, or worker configuration.

[Live demo](https://ctron.github.io/monaco-wasm/dioxus/) ·
[Quickstart project](https://github.com/ctron/monaco-wasm/tree/main/examples/quickstart-dioxus)

Enable the `web` feature for web builds (in fullstack apps, only for the client build):

```toml
[dependencies]
dioxus = { version = "0.7", features = ["web"] }
monaco-dioxus = { version = "0.1", features = ["web"] }
```

```rust
use dioxus::prelude::*;
use monaco_dioxus::MonacoEditor;

#[component]
fn App() -> Element {
    rsx! { MonacoEditor { language: "json", initial_value: "{{}}" } }
}
```

The [documentation](https://docs.rs/monaco-dioxus) covers the properties, reading and replacing
the text, changing options, fullstack apps, and deployment.

## License

Apache-2.0. The bundled Monaco runtime is MIT licensed.
