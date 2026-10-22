//! monaco-leptos quickstart: a small JSON playground.
//!
//! The editor owns its text. The app keeps an `EditorHandle` and reads or
//! replaces the document only when the user asks (Format, Download, Load
//! sample). Change notifications carry a version, not the text, so typing
//! never copies the document into signals.

use js_sys::Array;
use leptos::ev::Event;
use leptos::mount::mount_to_body;
use leptos::prelude::*;
use monaco_leptos::{
    ContentChange, EditorError, EditorHandle, EditorOptions, LineNumbers, MonacoEditor, WordWrap,
    set_theme,
};
use serde_json::{Value, from_str, to_string_pretty};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url, console, window};

const SAMPLE: &str = r#"{
  "name": "monaco-leptos",
  "description": "Monaco in Leptos, from Cargo dependencies only",
  "features": ["json diagnostics", "themes", "no npm", "no CDN"],
  "editor": {
    "fontSize": 14,
    "wordWrap": false
  },
  "tip": "Delete a comma or a quote to see JSON diagnostics."
}
"#;

const USAGE: &str = r#"[dependencies]
leptos = { version = "0.8", features = ["csr"] }
monaco-leptos = { version = "0.1", features = ["csr"] }

use leptos::prelude::*;
use monaco_leptos::MonacoEditor;

#[component]
fn App() -> impl IntoView {
    view! { <MonacoEditor language="json" initial_value="{}" /> }
}"#;

#[derive(Clone, Copy, PartialEq)]
enum Theme {
    Light,
    Dark,
    HighContrastLight,
    HighContrastDark,
}

impl Theme {
    const ALL: [Theme; 4] = [
        Theme::Light,
        Theme::Dark,
        Theme::HighContrastLight,
        Theme::HighContrastDark,
    ];

    /// Monaco's built-in theme name.
    fn monaco_name(self) -> &'static str {
        match self {
            Theme::Light => "vs",
            Theme::Dark => "vs-dark",
            Theme::HighContrastLight => "hc-light",
            Theme::HighContrastDark => "hc-black",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Theme::Light => "Light",
            Theme::Dark => "Dark",
            Theme::HighContrastLight => "High contrast light",
            Theme::HighContrastDark => "High contrast dark",
        }
    }

    fn is_dark(self) -> bool {
        matches!(self, Theme::Dark | Theme::HighContrastDark)
    }

    fn preferred() -> Theme {
        let prefers_dark = window()
            .and_then(|window| {
                window
                    .match_media("(prefers-color-scheme: dark)")
                    .ok()
                    .flatten()
            })
            .is_some_and(|query| query.matches());
        if prefers_dark {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

/// What the status bar shows, updated from change notifications.
#[derive(Clone, Default, PartialEq)]
struct Status {
    edits: u32,
    version: Option<f64>,
    last_was_replacement: bool,
}

/// Pretty-prints JSON, keeping key order.
fn format_json(text: &str) -> Result<String, String> {
    let value: Value = from_str(text).map_err(|error| format!("Not valid JSON: {error}"))?;
    let mut formatted = to_string_pretty(&value).map_err(|error| error.to_string())?;
    formatted.push('\n');
    Ok(formatted)
}

/// Offers `text` as a file download.
fn download(text: &str, file_name: &str) -> Result<(), JsValue> {
    let document = window()
        .and_then(|window| window.document())
        .ok_or("no document")?;
    let options = BlobPropertyBag::new();
    options.set_type("application/json");
    let blob = Blob::new_with_str_sequence_and_options(&Array::of1(&text.into()), &options)?;
    let url = Url::create_object_url_with_blob(&blob)?;
    let link: HtmlAnchorElement = document.create_element("a")?.unchecked_into();
    link.set_href(&url);
    link.set_download(file_name);
    link.click();
    Url::revoke_object_url(&url)
}

#[component]
fn App() -> impl IntoView {
    // The handle is not Send, so it lives in local storage.
    let handle = StoredValue::new_local(None::<EditorHandle>);
    let theme = RwSignal::new(Theme::preferred());
    let language = RwSignal::new("json".to_owned());
    let options = RwSignal::new(EditorOptions::default());
    let status = RwSignal::new(Status::default());
    let message = RwSignal::new(None::<String>);

    // The theme is global to Monaco; apply it whenever it changes.
    Effect::new(move |_| {
        if let Err(error) = set_theme(theme.get().monaco_name()) {
            console::error_1(&error.to_string().into());
        }
    });

    // Runs `action` against the mounted editor and shows its outcome.
    let with_editor = move |action: fn(&EditorHandle) -> Result<Option<String>, String>| {
        let Some(outcome) = handle.with_value(|editor| editor.as_ref().map(action)) else {
            return;
        };
        match outcome {
            Ok(note) => message.set(note),
            Err(error) => message.set(Some(error)),
        }
    };
    let format = move |_| {
        with_editor(|editor| {
            let text = editor.value().map_err(|error| error.to_string())?;
            let formatted = format_json(&text)?;
            editor
                .replace_document(&formatted)
                .map_err(|error| error.to_string())?;
            Ok(Some("Formatted.".into()))
        })
    };
    let load_sample = move |_| {
        with_editor(|editor| {
            editor
                .replace_document(SAMPLE)
                .map_err(|error| error.to_string())?;
            Ok(None)
        })
    };
    let save = move |_| {
        with_editor(|editor| {
            let text = editor.value().map_err(|error| error.to_string())?;
            download(&text, "document.json")
                .map_err(|error| format!("Download failed: {error:?}"))?;
            Ok(Some(format!(
                "Downloaded {} characters.",
                text.chars().count()
            )))
        })
    };

    let on_theme = move |event: Event| {
        let name = event_target_value(&event);
        if let Some(selected) = Theme::ALL.into_iter().find(|t| t.monaco_name() == name) {
            theme.set(selected);
        }
    };
    // Each toggle changes one field of the options signal; the component
    // applies the difference to the existing editor.
    let toggle = move |apply: fn(&mut EditorOptions, bool)| {
        move |event: Event| {
            let checked = event_target_checked(&event);
            options.update(|options| apply(options, checked));
        }
    };
    let on_font_size = move |event: Event| {
        if let Ok(size) = event_target_value(&event).parse() {
            options.update(|options| options.font_size = size);
        }
    };

    let last_change = move || {
        let status = status.get();
        match status.version {
            None => "No changes yet".to_string(),
            Some(version) => format!(
                "{} edits · version {} · last change: {}",
                status.edits,
                version,
                if status.last_was_replacement {
                    "document replaced"
                } else {
                    "typing"
                }
            ),
        }
    };

    view! {
        <div class=move || if theme.get().is_dark() { "app dark" } else { "app" }>
            <header>
                <h1>"monaco-leptos"</h1>
                <p>
                    "The Monaco editor in a Leptos app — built from Cargo dependencies only. No npm, no CDN. "
                    <a href="https://docs.rs/monaco-leptos">"Documentation"</a>
                </p>
            </header>

            <nav class="toolbar" aria-label="Editor settings">
                <label>
                    "Theme "
                    <select on:change=on_theme>
                        {Theme::ALL
                            .into_iter()
                            .map(|option| {
                                view! {
                                    <option
                                        value=option.monaco_name()
                                        selected=move || theme.get() == option
                                    >
                                        {option.label()}
                                    </option>
                                }
                            })
                            .collect_view()}
                    </select>
                </label>
                <label>
                    "Language "
                    <select on:change=move |event| language.set(event_target_value(&event))>
                        <option value="json" selected=move || language.get() == "json">
                            "JSON"
                        </option>
                        <option value="yaml" selected=move || language.get() == "yaml">
                            "YAML"
                        </option>
                        <option value="plaintext" selected=move || language.get() == "plaintext">
                            "Plain text"
                        </option>
                    </select>
                </label>
                <label>
                    "Font size "
                    <input
                        type="number"
                        min="8"
                        max="40"
                        prop:value=move || options.with(|options| options.font_size.to_string())
                        on:change=on_font_size
                    />
                </label>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=move || options.with(|options| options.word_wrap == WordWrap::On)
                        on:change=toggle(|options, on| {
                            options.word_wrap = if on { WordWrap::On } else { WordWrap::Off };
                        })
                    />
                    " Word wrap"
                </label>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=move || options.with(|options| options.line_numbers == LineNumbers::On)
                        on:change=toggle(|options, on| {
                            options.line_numbers = if on { LineNumbers::On } else { LineNumbers::Off };
                        })
                    />
                    " Line numbers"
                </label>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=move || options.with(|options| options.minimap_enabled)
                        on:change=toggle(|options, on| options.minimap_enabled = on)
                    />
                    " Minimap"
                </label>
                <label>
                    <input
                        type="checkbox"
                        prop:checked=move || options.with(|options| options.read_only)
                        on:change=toggle(|options, on| options.read_only = on)
                    />
                    " Read-only"
                </label>
                <span class="actions">
                    <button on:click=format>"Format"</button>
                    <button on:click=load_sample>"Load sample"</button>
                    <button on:click=save>"Download"</button>
                </span>
            </nav>

            <main>
                // The component defaults to 20rem high; here it fills the page.
                <MonacoEditor
                    class="editor"
                    style="height: 100%"
                    language=language
                    initial_value=SAMPLE
                    options=options
                    on_ready=move |ready| handle.set_value(Some(ready))
                    on_change=move |change: ContentChange| {
                        status
                            .update(|status| {
                                status.edits += 1;
                                status.version = Some(change.version_id);
                                status.last_was_replacement = change.is_flush;
                            })
                    }
                    on_error=move |error: EditorError| message.set(Some(error.to_string()))
                />
            </main>

            <footer>
                <span>{last_change}</span>
                {move || {
                    message
                        .get()
                        .map(|text| view! { <span class="message" role="status">{text}</span> })
                }}
            </footer>

            <details class="usage">
                <summary>"Use it in your app"</summary>
                <pre>
                    <code>{USAGE}</code>
                </pre>
            </details>
        </div>
    }
}

fn main() {
    mount_to_body(App);
}
