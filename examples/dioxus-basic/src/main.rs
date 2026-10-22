//! Dioxus web example: only Cargo dependencies.

use dioxus::prelude::*;
use monaco_dioxus::{ContentChange, EditorError, EditorHandle, EditorOptions, MonacoEditor};
use web_sys::console;

const INITIAL: &str = "{\n  \"enabled\": true,\n  \"items\": [1, 2, 3]\n}\n";
const REPLACEMENT: &str = "{\n  \"replaced\": true\n}\n";

#[component]
fn App() -> Element {
    // The handle is read on demand; typing never copies text into signals.
    let mut handle = use_signal(|| None::<EditorHandle>);
    let mut mounted = use_signal(|| true);
    let mut read_only = use_signal(|| false);
    let mut output = use_signal(String::new);

    let mut run = move |action: fn(&EditorHandle) -> Result<String, EditorError>| {
        let result = match handle.read().as_ref() {
            Some(editor) => action(editor).unwrap_or_else(|error| format!("Error: {error}")),
            None => "Error: editor is not mounted".to_owned(),
        };
        output.set(result);
    };

    let mut options = EditorOptions::default();
    options.read_only = read_only();

    rsx! {
        div { id: "controls",
            button { id: "save", onclick: move |_| run(|editor| editor.value()), "Save" }
            button {
                id: "replace",
                onclick: move |_| run(|editor| editor.replace_document(REPLACEMENT).map(|()| String::new())),
                "Replace document"
            }
            button {
                id: "read-only",
                onclick: move |_| {
                    let next = !read_only();
                    read_only.set(next);
                    output.set(format!("read-only: {next}"));
                },
                "Toggle read-only"
            }
            button {
                id: "mount",
                onclick: move |_| {
                    let next = !mounted();
                    if !next {
                        handle.set(None);
                    }
                    mounted.set(next);
                },
                if mounted() { "Unmount" } else { "Mount" }
            }
        }
        div { id: "editor",
            if mounted() {
                MonacoEditor {
                    language: "json",
                    initial_value: INITIAL,
                    options,
                    on_ready: move |ready| handle.set(Some(ready)),
                    on_change: move |change: ContentChange| {
                        console::log_1(&format!("change: {change:?}").into());
                    },
                }
            }
        }
        pre { id: "output", "{output}" }
    }
}

fn main() {
    dioxus::launch(App);
}
