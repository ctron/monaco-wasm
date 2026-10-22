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

The [documentation](https://docs.rs/monaco-yew) covers the properties, reading and replacing
the text, changing options, and deployment.

## License

Apache-2.0. The bundled Monaco runtime is MIT licensed.
