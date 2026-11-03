# monaco-wasm

[![CI](https://img.shields.io/github/actions/workflow/status/ctron/monaco-wasm/ci.yml?branch=main&label=CI)](https://github.com/ctron/monaco-wasm/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/monaco-wasm.svg)](https://crates.io/crates/monaco-wasm)
[![docs.rs](https://img.shields.io/docsrs/monaco-wasm)](https://docs.rs/monaco-wasm)

The [Monaco editor](https://microsoft.github.io/monaco-editor/) for Rust web applications, with
components for [Yew](https://yew.rs), [Leptos](https://leptos.dev), and [Dioxus](https://dioxuslabs.com). The editor runtime, its
stylesheet, and the editor and JSON workers are bundled in the crate: add a Cargo dependency, with
no npm install, CDN, or worker configuration.

| Crate | Use it for | Documentation |
| --- | --- | --- |
| [`monaco-yew`](crates/monaco-yew) | A `MonacoEditor` component for Yew 0.23 | [docs.rs](https://docs.rs/monaco-yew) |
| [`monaco-leptos`](crates/monaco-leptos) | A `MonacoEditor` component for Leptos 0.8 (CSR, SSR, hydration) | [docs.rs](https://docs.rs/monaco-leptos) |
| [`monaco-dioxus`](crates/monaco-dioxus) | A `MonacoEditor` component for Dioxus 0.7 (web, fullstack) | [docs.rs](https://docs.rs/monaco-dioxus) |
| [`monaco-wasm`](crates/monaco-wasm) | The framework-free core: plain Rust/WASM, or your own integration | [docs.rs](https://docs.rs/monaco-wasm) |

The documentation covers the properties, reading and replacing the text, editor options,
server-side rendering, and deployment (including Content Security Policy requirements).

## Yew

```toml
[dependencies]
yew = { version = "0.23", features = ["csr"] }
monaco-yew = "0.2"
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
monaco-leptos = { version = "0.2", features = ["csr"] }
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

## Dioxus

Enable the `web` feature (in fullstack apps, only for the client build):

```toml
[dependencies]
dioxus = { version = "0.7", features = ["web"] }
monaco-dioxus = { version = "0.2", features = ["web"] }
```

```rust
use dioxus::prelude::*;
use monaco_dioxus::MonacoEditor;

#[component]
fn App() -> Element {
    rsx! { MonacoEditor { language: "json", initial_value: "{{}}" } }
}
```

A complete example: [`examples/quickstart-dioxus`](examples/quickstart-dioxus)
([live](https://ctron.github.io/monaco-wasm/dioxus/)); for fullstack with `dx`, see
[`examples/dioxus-hydrate`](examples/dioxus-hydrate).

## Plain Rust/WASM

```toml
[dependencies]
monaco-wasm = "0.2"
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

## Optional modules and migration to 0.2

YAML is no longer built into the core runtime. Add `monaco-yaml = "0.2"`
and pass its fieldless `Yaml` module to the editor:

```rust
use monaco_yew::{MonacoEditor, Modules};
use monaco_yaml::Yaml;
use yew::prelude::*;

# fn example() -> Html {
html! { <MonacoEditor language="yaml" modules={Modules::from([Yaml])} /> }
# }
```

Leptos accepts `modules=[Yaml]` and Dioxus accepts `modules: [Yaml]`.
Plain WASM applications set `config.modules = [Yaml].into()`. See [`examples/yaml-basic`](examples/yaml-basic).

Modules initialize in list order before the editor model is created. A successful
module ID initializes once per page, and registration is visible to all editors.
The module list is read once on mount. Disposing an editor releases its editor
and model, but module registration persists until page unload. Initialization
errors prevent mounting and follow the existing error path; failed initialization
can be retried. Applications without the YAML crate ship none of its assets.

The YAML module includes highlighting, validation, completion, hover, and
formatting. Remote schema downloads are disabled.

## Compatibility

| Component | Support |
| --- | --- |
| Rust | 1.88 or newer |
| Monaco | 0.57.0 (bundled) |
| Languages | JSON (with validation), plain text; YAML through `monaco-yaml` |
| Yew | 0.23 |
| Leptos | 0.8 |
| Dioxus | 0.7 (web, fullstack) |
| Browsers | Tested with Chromium |

> [!TIP]
> Use wasm-bindgen 0.2.130 or newer. Older versions take about a minute to compile `monaco-wasm`
> after every clean or update, because of the size of the bundled editor runtime. With 0.2.130 it
> takes under a second. To update an existing lock file, run
> `cargo update -p wasm-bindgen -p web-sys -p js-sys`, since `web-sys` and `js-sys` pin the
> wasm-bindgen version.

## Development

Building, testing, and the maintainer tooling are described in [DEVELOPMENT.md](DEVELOPMENT.md).

## License

Apache-2.0, see [LICENSE](LICENSE). The bundled Monaco runtime is MIT licensed; see
[`crates/monaco-wasm/js/THIRD_PARTY_NOTICES`](crates/monaco-wasm/js/THIRD_PARTY_NOTICES).
