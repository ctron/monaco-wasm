//! Saving the text: restore it from local storage, save it when typing
//! pauses, and submit it with a form.
//!
//! The editor owns the document; change events carry a version, not the text.
//! To keep a copy elsewhere, wait until typing pauses and read the text once
//! through the handle. A form reads it the same way on submit, so no hidden
//! field has to mirror the document.

use gloo_storage::{LocalStorage, Storage};
use gloo_timers::callback::Timeout;
use monaco_yew::{ContentChange, EditorHandle, MonacoEditor};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::{FormData, HtmlFormElement};
use yew::Renderer;
use yew::prelude::*;

const KEY: &str = "monaco-yew-persist";
const DEFAULT: &str = "{\n  \"note\": \"Edit me, then reload the page.\"\n}\n";
/// How long typing has to pause before the text is saved.
const SAVE_DELAY_MS: u32 = 500;

type Handle = Rc<RefCell<Option<EditorHandle>>>;

#[derive(Clone, PartialEq)]
enum Status {
    Restored,
    NothingSaved,
    Pending,
    Saved,
    Failed(String),
}

impl Status {
    fn text(&self) -> String {
        match self {
            Status::Restored => "Restored from local storage".into(),
            Status::NothingSaved => "Nothing saved yet".into(),
            Status::Pending => "Unsaved changes".into(),
            Status::Saved => "Saved to local storage".into(),
            Status::Failed(error) => format!("Saving failed: {error}"),
        }
    }
}

/// Reads the document once and writes it to local storage.
fn save(handle: &Handle) -> Result<(), String> {
    let Some(editor) = handle.borrow().clone() else {
        return Ok(());
    };
    let text = editor.value().map_err(|error| error.to_string())?;
    LocalStorage::set(KEY, text).map_err(|error| error.to_string())
}

#[function_component(App)]
fn app() -> Html {
    let handle = use_mut_ref(|| None::<EditorHandle>);
    // Read once: the editor uses initial_value only when it mounts.
    let restored = use_memo((), |_| LocalStorage::get::<String>(KEY).ok());
    let status = use_state(|| match *restored {
        Some(_) => Status::Restored,
        None => Status::NothingSaved,
    });
    // A pending save is dropped, and so cancelled, with the component.
    let timer = use_mut_ref(|| None::<Timeout>);
    let submitted = use_state(|| None::<String>);

    let on_ready = {
        let handle = handle.clone();
        Callback::from(move |ready| *handle.borrow_mut() = Some(ready))
    };
    let on_change = {
        let (handle, status) = (handle.clone(), status.clone());
        Callback::from(move |_: ContentChange| {
            status.set(Status::Pending);
            let (handle, status) = (handle.clone(), status.clone());
            // Replacing the timeout drops, and so cancels, the previous one.
            *timer.borrow_mut() = Some(Timeout::new(SAVE_DELAY_MS, move || {
                status.set(match save(&handle) {
                    Ok(()) => Status::Saved,
                    Err(error) => Status::Failed(error),
                });
            }));
        })
    };
    let on_submit = {
        let (handle, submitted) = (handle.clone(), submitted.clone());
        Callback::from(move |event: SubmitEvent| {
            event.prevent_default();
            let form: HtmlFormElement = event.target_unchecked_into();
            let title = FormData::new_with_form(&form)
                .ok()
                .and_then(|data| data.get("title").as_string())
                .unwrap_or_default();
            let Some(editor) = handle.borrow().clone() else {
                return;
            };
            // A real app would send this to a server.
            submitted.set(Some(match editor.value() {
                Ok(text) => format!("title: {title}\ndocument:\n{text}"),
                Err(error) => format!("Error: {error}"),
            }));
        })
    };
    let reset = {
        let handle = handle.clone();
        Callback::from(move |_: MouseEvent| {
            // The replacement is a change like any other, so it is saved too.
            if let Some(editor) = handle.borrow().as_ref() {
                let _ = editor.replace_document(DEFAULT);
            }
        })
    };

    let initial_value = AttrValue::from(restored.as_deref().unwrap_or(DEFAULT).to_owned());

    html! {
        <main>
            <header>
                <h1>{"Saving the text"}</h1>
                <p>{"Edits are saved to local storage when typing pauses, and restored on reload."}</p>
            </header>
            <form onsubmit={on_submit}>
                <label>
                    {"Title "}
                    <input id="title" name="title" value="My document" />
                </label>
                <MonacoEditor language="json" {initial_value} {on_ready} {on_change} />
                <div class="actions">
                    <button id="submit" type="submit">{"Submit"}</button>
                    <button id="reset" type="button" onclick={reset}>{"Reset document"}</button>
                    <span id="status" role="status">{status.text()}</span>
                </div>
            </form>
            if let Some(text) = &*submitted {
                <pre id="submitted">{text}</pre>
            }
        </main>
    }
}

fn main() {
    Renderer::<App>::new().render();
}
