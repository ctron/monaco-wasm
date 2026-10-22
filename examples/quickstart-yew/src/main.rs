//! monaco-yew quickstart: a small JSON playground.
//!
//! The editor owns its text. The app keeps an `EditorHandle` and reads or
//! replaces the document only when the user asks (Format, Download, Load
//! sample). Change notifications carry a version, not the text, so typing
//! never copies the document into Yew state.

use js_sys::Array;
use monaco_yew::{
    ContentChange, EditorError, EditorHandle, EditorOptions, LineNumbers, MonacoEditor, WordWrap,
    set_theme,
};
use serde_json::{Value, from_str, to_string_pretty};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{
    Blob, BlobPropertyBag, HtmlAnchorElement, HtmlInputElement, HtmlSelectElement, Url, console,
    window,
};
use yew::Renderer;
use yew::prelude::*;

const SAMPLE: &str = r#"{
  "name": "monaco-yew",
  "description": "Monaco in Yew, from Cargo dependencies only",
  "features": ["json diagnostics", "themes", "no npm", "no CDN"],
  "editor": {
    "fontSize": 14,
    "wordWrap": false
  },
  "tip": "Delete a comma or a quote to see JSON diagnostics."
}
"#;

const USAGE: &str = r#"[dependencies]
yew = { version = "0.23", features = ["csr"] }
monaco-yew = "0.1"

use monaco_yew::MonacoEditor;
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! { <MonacoEditor language="json" initial_value="{}" /> }
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

#[function_component(App)]
fn app() -> Html {
    let handle = use_mut_ref(|| None::<EditorHandle>);
    let theme = use_state(Theme::preferred);
    let language = use_state(|| AttrValue::from("json"));
    let options = use_state(EditorOptions::default);
    let status = use_state(Status::default);
    let message = use_state(|| None::<String>);

    // The theme is global to Monaco; apply it whenever it changes.
    use_effect_with(*theme, |theme| {
        if let Err(error) = set_theme(theme.monaco_name()) {
            console::error_1(&error.to_string().into());
        }
    });

    let on_ready = {
        let handle = handle.clone();
        Callback::from(move |ready| *handle.borrow_mut() = Some(ready))
    };
    let on_change = {
        let status = status.clone();
        Callback::from(move |change: ContentChange| {
            status.set(Status {
                edits: status.edits + 1,
                version: Some(change.version_id),
                last_was_replacement: change.is_flush,
            });
        })
    };
    let on_error = {
        let message = message.clone();
        Callback::from(move |error: EditorError| message.set(Some(error.to_string())))
    };

    // Runs `action` against the mounted editor and shows its outcome.
    let with_editor = |action: fn(&EditorHandle) -> Result<Option<String>, String>| {
        let (handle, message) = (handle.clone(), message.clone());
        Callback::from(move |_: MouseEvent| {
            let Some(editor) = handle.borrow().clone() else {
                return;
            };
            match action(&editor) {
                Ok(note) => message.set(note),
                Err(error) => message.set(Some(error)),
            }
        })
    };
    let format = with_editor(|editor| {
        let text = editor.value().map_err(|error| error.to_string())?;
        let formatted = format_json(&text)?;
        editor
            .replace_document(&formatted)
            .map_err(|error| error.to_string())?;
        Ok(Some("Formatted.".into()))
    });
    let load_sample = with_editor(|editor| {
        editor
            .replace_document(SAMPLE)
            .map_err(|error| error.to_string())?;
        Ok(None)
    });
    let save = with_editor(|editor| {
        let text = editor.value().map_err(|error| error.to_string())?;
        download(&text, "document.json").map_err(|error| format!("Download failed: {error:?}"))?;
        Ok(Some(format!(
            "Downloaded {} characters.",
            text.chars().count()
        )))
    });

    let on_theme = {
        let theme = theme.clone();
        Callback::from(move |event: Event| {
            let index: usize = event
                .target_unchecked_into::<HtmlSelectElement>()
                .selected_index()
                .max(0) as usize;
            theme.set(Theme::ALL[index]);
        })
    };
    let on_language = {
        let language = language.clone();
        Callback::from(move |event: Event| {
            language.set(
                event
                    .target_unchecked_into::<HtmlSelectElement>()
                    .value()
                    .into(),
            );
        })
    };
    // Each toggle copies the options, changes one field, and sets them; the
    // component applies the difference to the existing editor.
    let toggle = |apply: fn(&mut EditorOptions, bool)| {
        let options = options.clone();
        Callback::from(move |event: Event| {
            let checked = event.target_unchecked_into::<HtmlInputElement>().checked();
            let mut next = (*options).clone();
            apply(&mut next, checked);
            options.set(next);
        })
    };
    let on_font_size = {
        let options = options.clone();
        Callback::from(move |event: Event| {
            if let Ok(size) = event
                .target_unchecked_into::<HtmlInputElement>()
                .value()
                .parse()
            {
                let mut next = (*options).clone();
                next.font_size = size;
                options.set(next);
            }
        })
    };

    let page_class = classes!("app", theme.is_dark().then_some("dark"));
    let last_change = match status.version {
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
    };

    html! {
        <div class={page_class}>
            <header>
                <h1>{"monaco-yew"}</h1>
                <p>{"The Monaco editor in a Yew app — built from Cargo dependencies only. No npm, no CDN."}</p>
            </header>

            <nav class="toolbar" aria-label="Editor settings">
                <label>
                    {"Theme "}
                    <select onchange={on_theme}>
                        { for Theme::ALL.iter().map(|option| html! {
                            <option selected={*option == *theme}>{option.label()}</option>
                        }) }
                    </select>
                </label>
                <label>
                    {"Language "}
                    <select onchange={on_language}>
                        <option value="json" selected={*language == "json"}>{"JSON"}</option>
                        <option value="plaintext" selected={*language == "plaintext"}>{"Plain text"}</option>
                    </select>
                </label>
                <label>
                    {"Font size "}
                    <input type="number" min="8" max="40" value={options.font_size.to_string()} onchange={on_font_size} />
                </label>
                <label>
                    <input type="checkbox" checked={options.word_wrap == WordWrap::On}
                        onchange={toggle(|o, on| o.word_wrap = if on { WordWrap::On } else { WordWrap::Off })} />
                    {" Word wrap"}
                </label>
                <label>
                    <input type="checkbox" checked={options.line_numbers == LineNumbers::On}
                        onchange={toggle(|o, on| o.line_numbers = if on { LineNumbers::On } else { LineNumbers::Off })} />
                    {" Line numbers"}
                </label>
                <label>
                    <input type="checkbox" checked={options.minimap_enabled}
                        onchange={toggle(|o, on| o.minimap_enabled = on)} />
                    {" Minimap"}
                </label>
                <label>
                    <input type="checkbox" checked={options.read_only}
                        onchange={toggle(|o, on| o.read_only = on)} />
                    {" Read-only"}
                </label>
                <span class="actions">
                    <button onclick={format}>{"Format"}</button>
                    <button onclick={load_sample}>{"Load sample"}</button>
                    <button onclick={save}>{"Download"}</button>
                </span>
            </nav>

            <main>
                // The component defaults to 20rem high; here it fills the page.
                <MonacoEditor
                    class="editor"
                    style="height: 100%"
                    language={(*language).clone()}
                    initial_value={SAMPLE}
                    options={(*options).clone()}
                    {on_ready}
                    {on_change}
                    {on_error}
                />
            </main>

            <footer>
                <span>{last_change}</span>
                if let Some(text) = &*message {
                    <span class="message" role="status">{text}</span>
                }
            </footer>

            <details class="usage">
                <summary>{"Use it in your app"}</summary>
                <pre><code>{USAGE}</code></pre>
            </details>
        </div>
    }
}

fn main() {
    Renderer::<App>::new().render();
}
