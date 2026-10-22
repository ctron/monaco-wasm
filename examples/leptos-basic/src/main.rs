//! Leptos CSR example: only Cargo dependencies, built with Trunk.

use leptos::mount::mount_to_body;
use leptos::prelude::*;
use monaco_leptos::{ContentChange, EditorError, EditorHandle, EditorOptions, MonacoEditor};
use web_sys::console;

const INITIAL: &str = "{\n  \"enabled\": true,\n  \"items\": [1, 2, 3]\n}\n";
const REPLACEMENT: &str = "{\n  \"replaced\": true\n}\n";

#[component]
fn App() -> impl IntoView {
    // The handle is read on demand; typing never copies text into signals.
    let handle = StoredValue::new_local(None::<EditorHandle>);
    let mounted = RwSignal::new(true);
    let read_only = RwSignal::new(false);
    let output = RwSignal::new(String::new());
    let options = Signal::derive(move || {
        let mut options = EditorOptions::default();
        options.read_only = read_only.get();
        options
    });

    let run = move |action: fn(&EditorHandle) -> Result<String, EditorError>| {
        let result = handle.with_value(|editor| match editor {
            Some(editor) => action(editor).unwrap_or_else(|error| format!("Error: {error}")),
            None => "Error: editor is not mounted".to_owned(),
        });
        output.set(result);
    };
    let toggle_read_only = move |_| {
        let next = !read_only.get();
        read_only.set(next);
        output.set(format!("read-only: {next}"));
    };
    let toggle_mounted = move |_| {
        let next = !mounted.get();
        if !next {
            handle.set_value(None);
        }
        mounted.set(next);
    };

    view! {
        <div id="controls">
            <button id="save" on:click=move |_| run(|editor| editor.value())>"Save"</button>
            <button
                id="replace"
                on:click=move |_| run(|editor| editor.replace_document(REPLACEMENT).map(|()| String::new()))
            >
                "Replace document"
            </button>
            <button id="read-only" on:click=toggle_read_only>"Toggle read-only"</button>
            <button id="mount" on:click=toggle_mounted>
                {move || if mounted.get() { "Unmount" } else { "Mount" }}
            </button>
        </div>
        <div id="editor">
            <Show when=move || mounted.get()>
                <MonacoEditor
                    language="json"
                    initial_value=INITIAL
                    options=options
                    on_ready=move |ready| handle.set_value(Some(ready))
                    on_change=|change: ContentChange| {
                        console::log_1(&format!("change: {change:?}").into());
                    }
                />
            </Show>
        </div>
        <pre id="output">{move || output.get()}</pre>
    }
}

fn main() {
    mount_to_body(App);
}
