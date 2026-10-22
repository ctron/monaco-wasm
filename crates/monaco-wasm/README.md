# monaco-wasm

The [Monaco editor](https://microsoft.github.io/monaco-editor/) for Rust/WASM applications, with
the editor runtime, its stylesheet, and the editor and JSON workers bundled in the crate. Add a
Cargo dependency: there is no npm install, CDN, or worker configuration.

This is the framework-free core. For components, see
[`monaco-yew`](https://crates.io/crates/monaco-yew) and
[`monaco-leptos`](https://crates.io/crates/monaco-leptos).
[Live demos](https://ctron.github.io/monaco-wasm/).

```toml
[dependencies]
monaco-wasm = "0.1"
```

```rust
use monaco_wasm::{Editor, EditorConfig, EditorError};
use web_sys::HtmlElement;

fn mount_editor(host: &HtmlElement) -> Result<Editor, EditorError> {
    let mut config = EditorConfig::default();
    config.initial_value = "{\n  \"enabled\": true\n}".into();
    config.language = "json".into();
    Editor::new(host, config)
}
// Keep the returned Editor alive; dropping it disposes the editor.
```

The [documentation](https://docs.rs/monaco-wasm) covers ownership and handles, change
notifications, editor options and themes, deployment, and Content Security Policy requirements.

## License

Apache-2.0. The bundled Monaco runtime is MIT licensed; see `js/THIRD_PARTY_NOTICES`.
