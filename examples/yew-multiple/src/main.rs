//! Two editors on one page: a draft and a read-only published copy.
//!
//! Each `MonacoEditor` is independent: its own model, undo history, handle,
//! and change events. Copying text between them goes through the handles. The
//! theme is the exception: Monaco has one theme per page, so `set_theme`
//! changes every editor at once.

use monaco_yew::{ContentChange, EditorHandle, EditorOptions, MonacoEditor, set_theme};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::HtmlSelectElement;
use yew::Renderer;
use yew::prelude::*;

const DRAFT: &str =
    "{\n  \"title\": \"Release notes\",\n  \"items\": [\"Two editors on one page\"]\n}\n";
const PUBLISHED: &str = "{\n  \"title\": \"Release notes\",\n  \"items\": []\n}\n";

type Handle = Rc<RefCell<Option<EditorHandle>>>;

/// Stores the handle from `on_ready`.
fn keep(handle: &Handle) -> Callback<EditorHandle> {
    let handle = handle.clone();
    Callback::from(move |ready| *handle.borrow_mut() = Some(ready))
}

/// Counts the change events of one editor.
fn count(edits: &UseStateHandle<u32>) -> Callback<ContentChange> {
    let edits = edits.clone();
    Callback::from(move |_| edits.set(*edits + 1))
}

#[function_component(App)]
fn app() -> Html {
    let draft = use_mut_ref(|| None::<EditorHandle>);
    let published = use_mut_ref(|| None::<EditorHandle>);
    let draft_edits = use_state(|| 0u32);
    let published_edits = use_state(|| 0u32);
    let message = use_state(String::new);
    let theme = use_state(|| String::from("vs"));

    // Replaces the published document with the text of the draft.
    let publish = {
        let (draft, published, message) = (draft.clone(), published.clone(), message.clone());
        Callback::from(move |_: MouseEvent| {
            let (draft, published) = (draft.borrow(), published.borrow());
            let result = match (draft.as_ref(), published.as_ref()) {
                (Some(draft), Some(published)) => draft
                    .value()
                    .and_then(|text| published.replace_document(&text)),
                _ => return,
            };
            if let Err(error) = result {
                message.set(format!("Error: {error}"));
            }
        })
    };
    let on_theme = {
        let (theme, message) = (theme.clone(), message.clone());
        Callback::from(move |event: Event| {
            let select: HtmlSelectElement = event.target_unchecked_into();
            // Applies to both editors: the theme is global in Monaco.
            match set_theme(&select.value()) {
                Ok(()) => theme.set(select.value()),
                Err(error) => message.set(format!("Error: {error}")),
            }
        })
    };

    let mut read_only = EditorOptions::default();
    read_only.read_only = true;

    html! {
        <main>
            <header>
                <h1>{"Two editors"}</h1>
                <p>{"Each editor has its own document, undo history, and change events."}</p>
            </header>
            <nav>
                <button id="publish" onclick={publish}>{"Publish draft"}</button>
                <label>
                    {"Theme (both editors) "}
                    <select id="theme" onchange={on_theme}>
                        <option value="vs" selected={*theme == "vs"}>{"Light"}</option>
                        <option value="vs-dark" selected={*theme == "vs-dark"}>{"Dark"}</option>
                    </select>
                </label>
                if !message.is_empty() {
                    <span role="status">{(*message).clone()}</span>
                }
            </nav>
            <div class="editors">
                <section id="draft">
                    <h2>{"Draft"}</h2>
                    <MonacoEditor
                        language="json"
                        initial_value={DRAFT}
                        on_ready={keep(&draft)}
                        on_change={count(&draft_edits)}
                    />
                    <p class="edits">{format!("Changes: {}", *draft_edits)}</p>
                </section>
                <section id="published">
                    <h2>{"Published (read-only)"}</h2>
                    <MonacoEditor
                        language="json"
                        initial_value={PUBLISHED}
                        options={read_only}
                        on_ready={keep(&published)}
                        on_change={count(&published_edits)}
                    />
                    <p class="edits">{format!("Changes: {}", *published_edits)}</p>
                </section>
            </div>
        </main>
    }
}

fn main() {
    Renderer::<App>::new().render();
}
