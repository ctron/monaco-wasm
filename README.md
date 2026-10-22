# monaco-wasm

The [Monaco editor](https://microsoft.github.io/monaco-editor/) for Rust web applications, with
components for [Yew](https://yew.rs) and [Leptos](https://leptos.dev). The editor runtime, its
stylesheet, and the editor and JSON workers are bundled in the crate: add a Cargo dependency, with
no npm install, CDN, or worker configuration.

| Crate | Use it for | Documentation |
| --- | --- | --- |
| [`monaco-yew`](crates/monaco-yew) | A `MonacoEditor` component for Yew 0.23 | [docs.rs](https://docs.rs/monaco-yew) |
| [`monaco-leptos`](crates/monaco-leptos) | A `MonacoEditor` component for Leptos 0.8 (CSR, SSR, hydration) | [docs.rs](https://docs.rs/monaco-leptos) |
| [`monaco-wasm`](crates/monaco-wasm) | The framework-free core: plain Rust/WASM, or your own integration | [docs.rs](https://docs.rs/monaco-wasm) |

The documentation covers the properties, reading and replacing the text, editor options,
server-side rendering, and deployment (including Content Security Policy requirements).

## Yew

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

A complete example: [`examples/quickstart-yew`](examples/quickstart-yew)
([live](https://ctron.github.io/monaco-wasm/yew/)).

## Leptos

Enable the same rendering mode (`csr`, `hydrate`, or `ssr`) as on Leptos:

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

A complete example: [`examples/quickstart-leptos`](examples/quickstart-leptos)
([live](https://ctron.github.io/monaco-wasm/leptos/)); for SSR with cargo-leptos, see
[`examples/leptos-hydrate`](examples/leptos-hydrate).

## Plain Rust/WASM

```toml
[dependencies]
monaco-wasm = "0.1"
```

```rust
use monaco_wasm::{Editor, EditorConfig, EditorError};
use web_sys::HtmlElement;

fn mount_editor(host: &HtmlElement) -> Result<Editor, EditorError> {
    let mut config = EditorConfig::default();
    config.language = "json".into();
    Editor::new(host, config)
}
```

See [`examples/wasm-basic`](examples/wasm-basic).

## Compatibility

| | |
| --- | --- |
| Rust | 1.88 or newer |
| Monaco | 0.57.0 (bundled) |
| Yew | 0.23 |
| Leptos | 0.8 |
| Browsers | Tested with Chromium |

## Development

Building, testing, and the maintainer tooling are described in [DEVELOPMENT.md](DEVELOPMENT.md);
the architecture and its decisions are in [DESIGN.md](DESIGN.md).

## License

Apache-2.0, see [LICENSE](LICENSE). The bundled Monaco runtime is MIT licensed; see
[`crates/monaco-wasm/js/THIRD_PARTY_NOTICES`](crates/monaco-wasm/js/THIRD_PARTY_NOTICES).
