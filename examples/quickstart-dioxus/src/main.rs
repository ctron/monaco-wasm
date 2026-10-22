//! monaco-dioxus quickstart: a small JSON playground.
//!
//! The editor owns its text. The app keeps an `EditorHandle` and reads or
//! replaces the document only when the user asks (Format, Download, Load
//! sample). Change notifications carry a version, not the text, so typing
//! never copies the document into signals.

use dioxus::prelude::*;
use js_sys::Array;
use monaco_dioxus::{
    ContentChange, EditorError, EditorHandle, EditorOptions, LineNumbers, MonacoEditor, WordWrap,
    set_theme,
};
use serde_json::{Value, from_str, to_string_pretty};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url, console, window};

const SAMPLE: &str = r#"{
  "name": "monaco-dioxus",
  "description": "Monaco in Dioxus, from Cargo dependencies only",
  "features": ["json diagnostics", "themes", "no npm", "no CDN"],
  "editor": {
    "fontSize": 14,
    "wordWrap": false
  },
  "tip": "Delete a comma or a quote to see JSON diagnostics."
}
"#;

const USAGE: &str = r#"[dependencies]
dioxus = { version = "0.7", features = ["web"] }
monaco-dioxus = { version = "0.1", features = ["web"] }

use dioxus::prelude::*;
use monaco_dioxus::MonacoEditor;

#[component]
fn App() -> Element {
    rsx! { MonacoEditor { language: "json", initial_value: "{{}}" } }
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
fn App() -> Element {
    let mut handle = use_signal(|| None::<EditorHandle>);
    let mut theme = use_signal(Theme::preferred);
    let mut language = use_signal(|| "json".to_owned());
    let mut options = use_signal(EditorOptions::default);
    let mut status = use_signal(Status::default);
    let mut message = use_signal(|| None::<String>);

    // The theme is global to Monaco; apply it whenever it changes.
    use_effect(move || {
        if let Err(error) = set_theme(theme().monaco_name()) {
            console::error_1(&error.to_string().into());
        }
    });

    // Runs `action` against the mounted editor and shows its outcome.
    let mut with_editor = move |action: fn(&EditorHandle) -> Result<Option<String>, String>| {
        let Some(outcome) = handle.read().as_ref().map(action) else {
            return;
        };
        match outcome {
            Ok(note) => message.set(note),
            Err(error) => message.set(Some(error)),
        }
    };

    let last_change = match status().version {
        None => "No changes yet".to_string(),
        Some(version) => format!(
            "{} edits · version {} · last change: {}",
            status().edits,
            version,
            if status().last_was_replacement {
                "document replaced"
            } else {
                "typing"
            }
        ),
    };

    rsx! {
        div { class: if theme().is_dark() { "app dark" } else { "app" },
            header {
                h1 { "monaco-dioxus" }
                p {
                    "The Monaco editor in a Dioxus app — built from Cargo dependencies only. No npm, no CDN. "
                    a { href: "https://docs.rs/monaco-dioxus", "Documentation" }
                }
            }

            nav { class: "toolbar", aria_label: "Editor settings",
                label {
                    "Theme "
                    select {
                        onchange: move |event| {
                            let name = event.value();
                            if let Some(selected) = Theme::ALL.into_iter().find(|t| t.monaco_name() == name) {
                                theme.set(selected);
                            }
                        },
                        for option in Theme::ALL {
                            option {
                                value: option.monaco_name(),
                                selected: theme() == option,
                                "{option.label()}"
                            }
                        }
                    }
                }
                label {
                    "Language "
                    select {
                        onchange: move |event| language.set(event.value()),
                        option { value: "json", selected: language() == "json", "JSON" }
                        option { value: "yaml", selected: language() == "yaml", "YAML" }
                        option { value: "plaintext", selected: language() == "plaintext", "Plain text" }
                    }
                }
                label {
                    "Font size "
                    input {
                        r#type: "number",
                        min: "8",
                        max: "40",
                        value: "{options.read().font_size}",
                        onchange: move |event| {
                            if let Ok(size) = event.value().parse() {
                                options.write().font_size = size;
                            }
                        },
                    }
                }
                label {
                    input {
                        r#type: "checkbox",
                        checked: options.read().word_wrap == WordWrap::On,
                        onchange: move |event| {
                            options.write().word_wrap = if event.checked() { WordWrap::On } else { WordWrap::Off };
                        },
                    }
                    " Word wrap"
                }
                label {
                    input {
                        r#type: "checkbox",
                        checked: options.read().line_numbers == LineNumbers::On,
                        onchange: move |event| {
                            options.write().line_numbers = if event.checked() { LineNumbers::On } else { LineNumbers::Off };
                        },
                    }
                    " Line numbers"
                }
                label {
                    input {
                        r#type: "checkbox",
                        checked: options.read().minimap_enabled,
                        onchange: move |event| options.write().minimap_enabled = event.checked(),
                    }
                    " Minimap"
                }
                label {
                    input {
                        r#type: "checkbox",
                        checked: options.read().read_only,
                        onchange: move |event| options.write().read_only = event.checked(),
                    }
                    " Read-only"
                }
                span { class: "actions",
                    button {
                        onclick: move |_| {
                            with_editor(|editor| {
                                let text = editor.value().map_err(|error| error.to_string())?;
                                let formatted = format_json(&text)?;
                                editor.replace_document(&formatted).map_err(|error| error.to_string())?;
                                Ok(Some("Formatted.".into()))
                            })
                        },
                        "Format"
                    }
                    button {
                        onclick: move |_| {
                            with_editor(|editor| {
                                editor.replace_document(SAMPLE).map_err(|error| error.to_string())?;
                                Ok(None)
                            })
                        },
                        "Load sample"
                    }
                    button {
                        onclick: move |_| {
                            with_editor(|editor| {
                                let text = editor.value().map_err(|error| error.to_string())?;
                                download(&text, "document.json")
                                    .map_err(|error| format!("Download failed: {error:?}"))?;
                                Ok(Some(format!("Downloaded {} characters.", text.chars().count())))
                            })
                        },
                        "Download"
                    }
                }
            }

            main {
                // The component defaults to 20rem high; here it fills the page.
                MonacoEditor {
                    class: "editor",
                    style: "height: 100%",
                    language: language(),
                    initial_value: SAMPLE,
                    options: options(),
                    on_ready: move |ready| handle.set(Some(ready)),
                    on_change: move |change: ContentChange| {
                        let mut status = status.write();
                        status.edits += 1;
                        status.version = Some(change.version_id);
                        status.last_was_replacement = change.is_flush;
                    },
                    on_error: move |error: EditorError| message.set(Some(error.to_string())),
                }
            }

            footer {
                span { "{last_change}" }
                if let Some(text) = message() {
                    span { class: "message", role: "status", "{text}" }
                }
            }

            details { class: "usage",
                summary { "Use it in your app" }
                pre { code { "{USAGE}" } }
            }
        }
    }
}

fn main() {
    dioxus::launch(App);
}
