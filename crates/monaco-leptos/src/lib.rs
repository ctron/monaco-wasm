//! Leptos component for the Monaco editor.
//!
//! ```no_run
//! use leptos::prelude::*;
//! use monaco_leptos::MonacoEditor;
//!
//! #[component]
//! fn App() -> impl IntoView {
//!     view! { <MonacoEditor language="json" initial_value="{}" /> }
//! }
//! ```
//!
//! The editor owns its text: `initial_value` is read once on mount and the
//! component never writes a value back. Read or replace text through the
//! [`EditorHandle`] passed to `on_ready`. To switch documents, call
//! [`EditorHandle::replace_document`] or remount the component.
//!
//! Enable exactly one of the `csr`, `hydrate`, or `ssr` features, matching
//! your Leptos build. On the server the component renders an empty host and
//! never touches browser APIs; in the browser it creates the editor once the
//! host is mounted (after hydration claimed it, when hydrating).

use leptos::html::Div;
use leptos::prelude::*;
pub use monaco_wasm::{
    self, ContentChange, EditorError, EditorHandle, EditorOptions, LineNumbers, WordWrap, set_theme,
};
use monaco_wasm::{Editor, EditorConfig, validate_language};
use web_sys::console;

/// Default container style; a `style` prop is appended and can override it.
const CONTAINER_STYLE: &str = "height: 20rem; width: 100%; min-width: 0;";
const HOST_STYLE: &str = "height: 100%; width: 100%;";

fn report(on_error: Option<UnsyncCallback<EditorError>>, error: EditorError) {
    match on_error {
        Some(on_error) => on_error.run(error),
        None => console::error_2(&"monaco-leptos:".into(), &error.to_string().into()),
    }
}

/// The Monaco editor.
///
/// `language`, `options`, `class`, and `style` accept plain values or
/// signals; changes update the existing editor without recreating it.
#[component]
pub fn MonacoEditor(
    /// Read once, on mount. Changing it later has no effect.
    #[prop(optional, into)]
    initial_value: String,
    /// Monaco language ID. Default: `"plaintext"`.
    #[prop(into, default = Signal::stored("plaintext".to_owned()))]
    language: Signal<String>,
    /// Editor options. Default: [`EditorOptions::default`].
    #[prop(into, default = Signal::stored(EditorOptions::default()))]
    options: Signal<EditorOptions>,
    #[prop(optional, into)] class: Signal<String>,
    /// Appended to the default container style.
    #[prop(optional, into)]
    style: Signal<String>,
    /// Called once, after the editor and its change subscription exist.
    #[prop(optional, into)]
    on_ready: Option<UnsyncCallback<EditorHandle>>,
    /// Called for each content change, after Monaco applied it.
    #[prop(optional, into)]
    on_change: Option<UnsyncCallback<ContentChange>>,
    /// Setup failures and failures of updates the component applies. Logged
    /// to the console when not set.
    #[prop(optional, into)]
    on_error: Option<UnsyncCallback<EditorError>>,
) -> impl IntoView {
    let host = NodeRef::<Div>::new();
    // Browser-only state. The editor is not Send, so it lives in local
    // storage owned by this component.
    let editor = StoredValue::new_local(None::<Editor>);
    let error_handler = StoredValue::new_local(on_error);
    // Last language and options Monaco accepted.
    let applied = StoredValue::new_local(None::<(String, EditorOptions)>);
    let failed = RwSignal::new(false);
    let mounted = StoredValue::new(false);

    // Creates the editor once the host element exists. Language and options
    // are read untracked, so later changes go to the update effect instead
    // of recreating the editor; reading them now picks up any change made
    // before mount.
    Effect::new(move |_| {
        let Some(element) = host.get() else {
            return;
        };
        if mounted.get_value() {
            return;
        }
        mounted.set_value(true);

        let mut config = EditorConfig::default();
        config.initial_value = initial_value.clone();
        config.language = language.get_untracked();
        config.options = options.get_untracked();
        let accepted = (config.language.clone(), config.options.clone());

        match Editor::new(&element, config) {
            Ok(mut created) => {
                if let Some(on_change) = on_change {
                    created.set_on_change(move |change| on_change.run(change));
                }
                let handle = created.handle();
                editor.set_value(Some(created));
                applied.set_value(Some(accepted));
                if let Some(on_ready) = on_ready {
                    on_ready.run(handle);
                }
            }
            Err(error) => {
                failed.set(true);
                report(on_error, error);
            }
        }
    });

    // Applies language/options changes to the existing editor. Both are
    // validated before either is applied; a failed JS call leaves the field
    // unapplied until that value changes again.
    Effect::new(move |previous: Option<(String, EditorOptions)>| {
        let current = (language.get(), options.get());
        let Some(previous) = previous else {
            return current;
        };
        let Some(handle) = editor.with_value(|editor| editor.as_ref().map(Editor::handle)) else {
            return current;
        };
        let Some((applied_language, applied_options)) = applied.get_value() else {
            return current;
        };
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
            return current;
        }
        if let Some(language) = language_update {
            match handle.set_language(language) {
                Ok(()) => applied.update_value(|applied| {
                    if let Some(applied) = applied {
                        applied.0 = language.clone();
                    }
                }),
                Err(error) => report(on_error, error),
            }
        }
        if let Some(options) = options_update {
            match handle.update_options(options) {
                Ok(()) => applied.update_value(|applied| {
                    if let Some(applied) = applied {
                        applied.1 = options.clone();
                    }
                }),
                Err(error) => report(on_error, error),
            }
        }
        current
    });

    // The stored values may already be disposed while the owner is being
    // cleaned up, so they are accessed with try_*.
    on_cleanup(move || {
        let Some(Some(mut owned)) = editor.try_update_value(Option::take) else {
            return;
        };
        let result = owned.dispose();
        drop(owned);
        if let Err(error) = result {
            let handler = error_handler.try_get_value().flatten();
            report(handler, error);
        }
    });

    let container_style = move || {
        let style = style.get();
        if style.is_empty() {
            CONTAINER_STYLE.to_owned()
        } else {
            format!("{CONTAINER_STYLE} {style}")
        }
    };

    view! {
        <div class=move || class.get() style=container_style>
            <div node_ref=host style=HOST_STYLE></div>
            <Show when=move || failed.get()>
                <div role="alert">"Editor could not be initialized"</div>
            </Show>
        </div>
    }
}
