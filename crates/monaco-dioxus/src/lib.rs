//! A [Dioxus](https://dioxuslabs.com) component for the
//! [Monaco editor](https://microsoft.github.io/monaco-editor/).
//!
//! Built on [`monaco_wasm`], which bundles the editor runtime and its workers: add a Cargo
//! dependency, with no npm install, CDN, or worker configuration.
//!
//! Enable the `web` feature for web builds:
//!
//! ```toml
//! [dependencies]
//! dioxus = { version = "0.7", features = ["web"] }
//! monaco-dioxus = { version = "0.1", features = ["web"] }
//! ```
//!
//! Built with `dx`, the app works as is. Built with another tool such as Trunk, the page needs the
//! element Dioxus mounts into: `<div id="main"></div>`.
//!
//! # Fullstack
//!
//! Enable `web` only for the client build:
//!
//! ```toml
//! [features]
//! web = ["dioxus/web", "monaco-dioxus/web"]
//! server = ["dioxus/server"]
//! ```
//!
//! On the server the component renders an empty editor host and never runs browser code. In the
//! browser, hydration claims that host and the editor is created afterwards; the initial text is
//! supplied by the client and never rendered as HTML.
//!
//! # Example
//!
//! ```rust
//! use dioxus::prelude::*;
//! use monaco_dioxus::MonacoEditor;
//!
//! #[component]
//! fn App() -> Element {
//!     rsx! { MonacoEditor { language: "json", initial_value: "{{}}" } }
//! }
//! ```
//!
//! # Properties
//!
//! | Prop | Type | Default | Behavior |
//! | --- | --- | --- | --- |
//! | `initial_value` | `String` | empty | Read once, on mount. |
//! | `language` | `String` | `"plaintext"` | Updates the existing model. |
//! | `options` | [`EditorOptions`] | defaults | Updates the existing editor. |
//! | `class` | `String` | empty | Classes of the container. |
//! | `style` | `String` | empty | Appended to the default `height: 20rem; width: 100%; min-width: 0`. |
//! | `on_ready` | `EventHandler<EditorHandle>` | none | Called once, after the editor exists. |
//! | `on_change` | `EventHandler<ContentChange>` | none | Each content change, after Monaco applied it. |
//! | `on_error` | `EventHandler<EditorError>` | console | Setup failures and failed updates. |
//!
//! # Reading the text and changing options
//!
//! The editor owns its text: `initial_value` is read once, and the component never writes a value
//! back on rerender. Keep the [`EditorHandle`] passed to `on_ready` and read or replace the
//! document through it when you need to. [`ContentChange`] carries a version, not the text, so
//! typing never copies the document into signals.
//!
//! ```rust
//! use dioxus::prelude::*;
//! use monaco_dioxus::{EditorHandle, EditorOptions, MonacoEditor};
//!
//! #[component]
//! fn App() -> Element {
//!     let mut handle = use_signal(|| None::<EditorHandle>);
//!     let mut read_only = use_signal(|| false);
//!     let mut options = EditorOptions::default();
//!     options.read_only = read_only();
//!
//!     rsx! {
//!         MonacoEditor {
//!             language: "json",
//!             initial_value: "{{}}",
//!             options,
//!             on_ready: move |ready| handle.set(Some(ready)),
//!         }
//!         button {
//!             onclick: move |_| {
//!                 if let Some(editor) = handle.read().as_ref() {
//!                     let text = editor.value().unwrap_or_default();
//!                     // ... store `text` somewhere
//!                     # let _ = text;
//!                 }
//!             },
//!             "Save"
//!         }
//!         button { onclick: move |_| read_only.toggle(), "Toggle read-only" }
//!     }
//! }
//! ```
//!
//! To switch to a different document, call [`EditorHandle::replace_document`], or remount the
//! component with a new `key`. Changing `initial_value` alone does nothing.
//!
//! Changes to `language` and `options` are validated together before either is applied. If
//! applying one fails, the editor keeps running, the error goes to `on_error`, and the change is
//! retried the next time that prop changes.
//!
//! For deployment and Content Security Policy requirements, see
//! [`monaco_wasm`'s deployment notes](monaco_wasm#deployment).

use dioxus::prelude::*;
pub use monaco_wasm::{
    self, ContentChange, EditorError, EditorHandle, EditorOptions, LineNumbers, WordWrap, set_theme,
};
use monaco_wasm::{Editor, validate_language};
use std::cell::RefCell;
use std::rc::Rc;
use web_sys::console;

/// Default container style; a `style` prop is appended and can override it.
const CONTAINER_STYLE: &str = "height: 20rem; width: 100%; min-width: 0;";
const HOST_STYLE: &str = "height: 100%; width: 100%;";

fn report(on_error: Option<EventHandler<EditorError>>, error: EditorError) {
    match on_error {
        Some(on_error) => on_error.call(error),
        None => console::error_2(&"monaco-dioxus:".into(), &error.to_string().into()),
    }
}

/// Browser-only state, shared between the event handlers of one component.
#[derive(Default)]
struct State {
    editor: Option<Editor>,
    /// Last language and options Monaco accepted.
    applied: Option<(String, EditorOptions)>,
    /// Language and options of the previous update, to tell which one changed.
    previous: Option<(String, EditorOptions)>,
}

/// The Monaco editor.
///
/// Changes to `language` and `options` update the existing editor without recreating it.
#[component]
pub fn MonacoEditor(
    /// Read once, on mount. Changing it later has no effect.
    #[props(default, into)]
    initial_value: String,
    /// Monaco language ID.
    #[props(default = "plaintext".to_owned(), into)]
    language: String,
    /// Editor options.
    #[props(default)]
    options: EditorOptions,
    /// Classes of the container.
    #[props(default, into)]
    class: String,
    /// Appended to the default container style.
    #[props(default, into)]
    style: String,
    /// Called once, after the editor and its change subscription exist.
    on_ready: Option<EventHandler<EditorHandle>>,
    /// Called for each content change, after Monaco applied it.
    on_change: Option<EventHandler<ContentChange>>,
    /// Setup failures and failures of updates the component applies. Logged to the console when
    /// not set.
    on_error: Option<EventHandler<EditorError>>,
) -> Element {
    let state = use_hook(|| Rc::new(RefCell::new(State::default())));
    let failed = use_signal(|| false);

    // Applies language/options changes to the existing editor. Both are validated before either
    // is applied; a failed JS call leaves the field unapplied until that prop changes again.
    let effect_state = state.clone();
    let (watched_language, watched_options) = (language.clone(), options.clone());
    use_effect(use_reactive!(|(watched_language, watched_options)| {
        let state = &effect_state;
        let current = (watched_language, watched_options);
        let mut guard = state.borrow_mut();
        let Some(previous) = guard.previous.replace(current.clone()) else {
            return;
        };
        let (Some(editor), Some((applied_language, applied_options))) =
            (guard.editor.as_ref(), guard.applied.clone())
        else {
            return;
        };
        let handle = editor.handle();
        drop(guard);

        let (new_language, new_options) = &current;
        let language_update = (*new_language != previous.0 && *new_language != applied_language)
            .then_some(new_language);
        let options_update =
            (*new_options != previous.1 && *new_options != applied_options).then_some(new_options);
        let validation = language_update
            .map_or(Ok(()), |language| validate_language(language))
            .and_then(|()| options_update.map_or(Ok(()), |options| options.validate()));
        if let Err(error) = validation {
            report(on_error, error);
            return;
        }
        if let Some(language) = language_update {
            match handle.set_language(language) {
                Ok(()) => {
                    if let Some(applied) = state.borrow_mut().applied.as_mut() {
                        applied.0 = language.clone();
                    }
                }
                Err(error) => report(on_error, error),
            }
        }
        if let Some(options) = options_update {
            match handle.update_options(options) {
                Ok(()) => {
                    if let Some(applied) = state.borrow_mut().applied.as_mut() {
                        applied.1 = options.clone();
                    }
                }
                Err(error) => report(on_error, error),
            }
        }
    }));

    use_drop({
        let state = state.clone();
        move || {
            let editor = state.borrow_mut().editor.take();
            if let Some(mut editor) = editor
                && let Err(error) = editor.dispose()
            {
                report(on_error, error);
            }
        }
    });

    // Creates the editor once the host element exists. Only web builds have a DOM; on the server
    // the host stays empty.
    let onmounted = {
        let state = state.clone();
        let language = language.clone();
        let options = options.clone();
        move |event: MountedEvent| {
            #[cfg(feature = "web")]
            {
                use dioxus::web::WebEventExt;
                use monaco_wasm::EditorConfig;
                use wasm_bindgen::JsCast;

                let mut failed = failed;
                if state.borrow().editor.is_some() || failed() {
                    return;
                }
                let element: web_sys::HtmlElement = event.as_web_event().unchecked_into();
                let mut config = EditorConfig::default();
                config.initial_value = initial_value.clone();
                config.language = language.clone();
                config.options = options.clone();
                let accepted = (config.language.clone(), config.options.clone());

                match Editor::new(&element, config) {
                    Ok(mut created) => {
                        if let Some(on_change) = on_change {
                            created.set_on_change(move |change| on_change.call(change));
                        }
                        let handle = created.handle();
                        {
                            let mut state = state.borrow_mut();
                            state.editor = Some(created);
                            state.applied = Some(accepted);
                        }
                        if let Some(on_ready) = on_ready {
                            on_ready.call(handle);
                        }
                    }
                    Err(error) => {
                        failed.set(true);
                        report(on_error, error);
                    }
                }
            }
            #[cfg(not(feature = "web"))]
            let _ = (&event, &state, &language, &options, &initial_value);
        }
    };

    let container_style = if style.is_empty() {
        CONTAINER_STYLE.to_owned()
    } else {
        format!("{CONTAINER_STYLE} {style}")
    };

    rsx! {
        div { class, style: container_style,
            div { style: HOST_STYLE, onmounted }
            if failed() {
                div { role: "alert", "Editor could not be initialized" }
            }
        }
    }
}
