//! Yew example: only Cargo dependencies, built with Trunk.

use monaco_yew::{ContentChange, EditorError, EditorHandle, EditorOptions, MonacoEditor};
use web_sys::console;
use yew::Renderer;
use yew::prelude::*;

const INITIAL: &str = "{\n  \"enabled\": true,\n  \"items\": [1, 2, 3]\n}\n";
const REPLACEMENT: &str = "{\n  \"replaced\": true\n}\n";

#[function_component(App)]
fn app() -> Html {
    // The handle is read on demand; typing never copies text into state.
    let handle = use_mut_ref(|| None::<EditorHandle>);
    let mounted = use_state(|| true);
    let read_only = use_state(|| false);
    let output = use_state(String::new);

    let on_ready = {
        let handle = handle.clone();
        Callback::from(move |ready| *handle.borrow_mut() = Some(ready))
    };
    let on_change = Callback::from(|change: ContentChange| {
        console::log_1(&format!("change: {change:?}").into());
    });
    let run = |action: fn(&EditorHandle) -> Result<String, EditorError>| {
        let (handle, output) = (handle.clone(), output.clone());
        Callback::from(move |_| match handle.borrow().as_ref() {
            Some(editor) => match action(editor) {
                Ok(text) => output.set(text),
                Err(error) => output.set(format!("Error: {error}")),
            },
            None => output.set("Error: editor is not mounted".into()),
        })
    };
    let save = run(|editor| editor.value());
    let replace = run(|editor| editor.replace_document(REPLACEMENT).map(|()| String::new()));
    let toggle_read_only = {
        let (read_only, output) = (read_only.clone(), output.clone());
        Callback::from(move |_| {
            let next = !*read_only;
            read_only.set(next);
            output.set(format!("read-only: {next}"));
        })
    };
    let toggle_mounted = {
        let (mounted, handle) = (mounted.clone(), handle.clone());
        Callback::from(move |_| {
            let next = !*mounted;
            if !next {
                *handle.borrow_mut() = None;
            }
            mounted.set(next);
        })
    };

    let mut options = EditorOptions::default();
    options.read_only = *read_only;

    html! {
        <>
            <div id="controls">
                <button id="save" onclick={save}>{"Save"}</button>
                <button id="replace" onclick={replace}>{"Replace document"}</button>
                <button id="read-only" onclick={toggle_read_only}>{"Toggle read-only"}</button>
                <button id="mount" onclick={toggle_mounted}>
                    {if *mounted { "Unmount" } else { "Mount" }}
                </button>
            </div>
            <div id="editor">
                if *mounted {
                    <MonacoEditor language="json" initial_value={INITIAL} {options} {on_ready} {on_change} />
                }
            </div>
            <pre id="output">{(*output).clone()}</pre>
        </>
    }
}

fn main() {
    Renderer::<App>::new().render();
}
