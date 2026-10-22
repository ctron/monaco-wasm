# monaco-wasm

The [Monaco editor](https://microsoft.github.io/monaco-editor/) for Rust web applications, with
components for [Yew](https://yew.rs) and [Leptos](https://leptos.dev). The editor runtime, its
stylesheet, and the editor and JSON workers are bundled in the crate: add a Cargo dependency, with
no npm install, CDN, or worker configuration.

| Crate | Use it for |
| --- | --- |
| [`monaco-yew`](crates/monaco-yew) | A `MonacoEditor` component for Yew 0.23 |
| [`monaco-leptos`](crates/monaco-leptos) | A `MonacoEditor` component for Leptos 0.8 (CSR, SSR, hydration) |
| [`monaco-wasm`](crates/monaco-wasm) | The framework-free core: plain Rust/WASM, or building your own integration |

Both components follow the same model: **the editor owns its text**. `initial_value` is read once,
and the component never writes a value back on rerender. Keep the `EditorHandle` passed to
`on_ready` and read or replace the document through it when you need to. Change notifications
carry a version, not the text, so typing never copies the document into framework state.

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

### Properties

| Prop | Type | Default | Behavior |
| --- | --- | --- | --- |
| `initial_value` | `AttrValue` | empty | Read once, on mount. |
| `language` | `AttrValue` | `"plaintext"` | Updates the existing model. |
| `options` | [`EditorOptions`](#editor-options) | defaults | Updates the existing editor. |
| `class` | `Classes` | none | Classes of the container. |
| `style` | `AttrValue` | none | Appended to the default `height: 20rem; width: 100%; min-width: 0`. |
| `on_ready` | `Callback<EditorHandle>` | none | Called once, after the editor exists. |
| `on_change` | `Callback<ContentChange>` | none | Each content change: `version_id`, `is_flush`. |
| `on_error` | `Callback<EditorError>` | console | Setup failures and failed updates. |

### Reading the text and changing options

```rust
use monaco_yew::{EditorHandle, EditorOptions, MonacoEditor, WordWrap};
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    // Keep the handle from on_ready; reading it does not rerender.
    let handle = use_mut_ref(|| None::<EditorHandle>);
    let on_ready = {
        let handle = handle.clone();
        Callback::from(move |ready| *handle.borrow_mut() = Some(ready))
    };
    let save = {
        let handle = handle.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(editor) = handle.borrow().as_ref() {
                let text = editor.value().unwrap_or_default();
                // ... store `text` somewhere
            }
        })
    };

    let mut options = EditorOptions::default();
    options.word_wrap = WordWrap::On;

    html! {
        <>
            <MonacoEditor language="json" initial_value="{}" {options} {on_ready} />
            <button onclick={save}>{"Save"}</button>
        </>
    }
}
```

To switch to a different document, call `replace_document` on the handle, or remount the
component with a new `key`. Changing `initial_value` alone does nothing.

The component supports client-side rendering. A complete example is the
[Yew quickstart](examples/quickstart-yew) ([live](https://ctron.github.io/monaco-wasm/yew/)).

## Leptos

Enable the same rendering mode on `monaco-leptos` as on Leptos: `csr`, `hydrate`, or `ssr`.

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

### Properties

| Prop | Type | Default | Behavior |
| --- | --- | --- | --- |
| `initial_value` | `String` (`into`) | empty | Read once, on mount. |
| `language` | `Signal<String>` (`into`) | `"plaintext"` | A value or signal; updates the existing model. |
| `options` | `Signal<EditorOptions>` (`into`) | defaults | A value or signal; updates the existing editor. See [editor options](#editor-options). |
| `class` | `Signal<String>` (`into`) | none | Classes of the container. |
| `style` | `Signal<String>` (`into`) | none | Appended to the default `height: 20rem; width: 100%; min-width: 0`. |
| `on_ready` | `UnsyncCallback<EditorHandle>` | none | Called once, after the editor exists. |
| `on_change` | `UnsyncCallback<ContentChange>` | none | Each content change: `version_id`, `is_flush`. |
| `on_error` | `UnsyncCallback<EditorError>` | console | Setup failures and failed updates. |

Callbacks accept closures.

### Reading the text and changing options

```rust
use leptos::prelude::*;
use monaco_leptos::{EditorHandle, EditorOptions, MonacoEditor};

#[component]
fn App() -> impl IntoView {
    // The handle is not Send, so it lives in local storage.
    let handle = StoredValue::new_local(None::<EditorHandle>);
    let read_only = RwSignal::new(false);
    let options = Signal::derive(move || {
        let mut options = EditorOptions::default();
        options.read_only = read_only.get();
        options
    });
    let save = move |_| {
        handle.with_value(|editor| {
            if let Some(editor) = editor {
                let text = editor.value().unwrap_or_default();
                // ... store `text` somewhere
            }
        })
    };

    view! {
        <MonacoEditor
            language="json"
            initial_value="{}"
            options=options
            on_ready=move |ready| handle.set_value(Some(ready))
        />
        <button on:click=save>"Save"</button>
        <button on:click=move |_| read_only.update(|on| *on = !*on)>"Toggle read-only"</button>
    }
}
```

To switch to a different document, call `replace_document` on the handle, or remount the
component (for example inside a keyed view).

### Server-side rendering and hydration

Forward your application's features to `monaco-leptos`:

```toml
[features]
hydrate = ["leptos/hydrate", "monaco-leptos/hydrate"]
ssr = ["leptos/ssr", "monaco-leptos/ssr"]
```

On the server the component renders an empty editor host and never runs browser code. In the
browser, hydration claims that host and the editor is created afterwards; the initial text is
supplied by the client and never rendered as HTML.

A complete CSR example is the [Leptos quickstart](examples/quickstart-leptos)
([live](https://ctron.github.io/monaco-wasm/leptos/)); a cargo-leptos SSR setup is in
[`examples/leptos-hydrate`](examples/leptos-hydrate).

## Plain Rust/WASM

`monaco-wasm` works without a framework:

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
```

`Editor` owns the editor and its model; dropping it disposes both. `Editor::handle` returns a
cloneable, weak `EditorHandle`, and `set_on_change` sets the change callback. Unlike the
components, the raw editor does not size its host. `monaco_wasm::sys` exposes the low-level
bindings for anything the safe API does not cover. See
[`examples/wasm-basic`](examples/wasm-basic).

## Editor options

`EditorOptions` is shared by all three crates. Start from `EditorOptions::default()` and assign
fields; updating applies all fields, so setting one back to its default resets it.

| Field | Default | |
| --- | --- | --- |
| `read_only` | `false` | |
| `font_size` | `14.0` | Must be finite and positive. |
| `word_wrap` | `WordWrap::Off` | |
| `minimap_enabled` | `false` | |
| `line_numbers` | `LineNumbers::On` | |
| `aria_label` | `"Code editor"` | Must not be blank. |

`set_theme("vs-dark")` switches the theme of every editor on the page; the built-in themes are
`vs`, `vs-dark`, `hc-light`, and `hc-black`. The bundled runtime includes JSON (with
diagnostics) and plain text.

## Deployment

- Deploy the whole build output of Trunk, wasm-pack, or cargo-leptos, including the generated
  `snippets/` directory: it contains the bundled Monaco runtime (about 1.4 MiB gzipped). No other
  files are downloaded at runtime.
- With a Content Security Policy, the editor needs `worker-src blob:`,
  `style-src 'unsafe-inline'`, and `font-src data:`. A complete policy that works:

  ```text
  default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self';
  worker-src blob:; style-src 'self' 'unsafe-inline'; font-src data:; img-src data:
  ```

  `script-src 'wasm-unsafe-eval'` and `connect-src 'self'` are needed by any wasm-bindgen
  application.
- The page must not define its own `MonacoEnvironment`; editor creation then fails with a setup
  error instead of overwriting it.

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
