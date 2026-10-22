use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, console};

use crate::options::validate_language;
use crate::sys::{
    self, ChangeRelay, Disposable, EditorApi, EditorUpdateOptions, MinimapOptions,
    StandaloneCodeEditor, StandaloneEditorConstructionOptions, TextModel,
};
use crate::{EditorConfig, EditorError, EditorOptions};

/// A content change notification, delivered from a microtask after Monaco
/// applied the change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContentChange {
    /// Monaco's model version. Ordered, but not necessarily consecutive.
    pub version_id: f64,
    /// True when the whole document was replaced.
    pub is_flush: bool,
}

type ChangeCallback = Rc<dyn Fn(ContentChange)>;
type RelayClosure = Closure<dyn Fn(f64, bool)>;

struct State {
    disposed: Cell<bool>,
    api: EditorApi,
    editor: StandaloneCodeEditor,
    model: TextModel,
    relay: ChangeRelay,
    relay_closure: RefCell<Option<RelayClosure>>,
    subscription: RefCell<Option<Disposable>>,
    callback: RefCell<ChangeCallback>,
}

fn noop_callback() -> ChangeCallback {
    Rc::new(|_| {})
}

impl State {
    fn dispatch(&self, version_id: f64, is_flush: bool) {
        if self.disposed.get() {
            return;
        }
        // Clone under a short borrow, so the callback may replace itself,
        // replace the document, or dispose the editor.
        let callback = self.callback.borrow().clone();
        callback(ContentChange {
            version_id,
            is_flush,
        });
    }

    fn dispose(&self) -> Result<(), EditorError> {
        if self.disposed.replace(true) {
            return Ok(());
        }
        self.relay.detach();
        *self.callback.borrow_mut() = noop_callback();

        let mut first_error = None;
        let subscription = self.subscription.borrow_mut().take();
        if let Some(subscription) = subscription {
            record(
                &mut first_error,
                subscription
                    .try_dispose()
                    .map_err(EditorError::js("unsubscribe")),
            );
        }
        record(
            &mut first_error,
            self.editor
                .try_dispose()
                .map_err(EditorError::js("dispose editor")),
        );
        record(
            &mut first_error,
            self.model
                .try_dispose()
                .map_err(EditorError::js("dispose model")),
        );

        // This may run from inside the relay closure. wasm-bindgen 0.2.129
        // reference-counts owned closures across calls, but the dependency
        // range admits older releases, so release it a microtask later. The
        // relay is already detached, so nothing can call it meanwhile.
        let closure = self.relay_closure.borrow_mut().take();
        if let Some(closure) = closure {
            let release = Closure::once_into_js(move || drop(closure));
            sys::queue_microtask(release.unchecked_ref());
        }

        first_error.map_or(Ok(()), Err)
    }
}

fn record(first_error: &mut Option<EditorError>, result: Result<(), EditorError>) {
    if let Err(error) = result {
        first_error.get_or_insert(error);
    }
}

fn ensure_browser_target() -> Result<(), EditorError> {
    if cfg!(target_arch = "wasm32") {
        Ok(())
    } else {
        Err(EditorError::UnsupportedTarget)
    }
}

fn apply_options(target: &EditorUpdateOptions, options: &EditorOptions) {
    let minimap = MinimapOptions::new();
    minimap.set_enabled(options.minimap_enabled);
    target.set_read_only(options.read_only);
    target.set_font_size(options.font_size);
    target.set_word_wrap(options.word_wrap.into());
    target.set_minimap(&minimap);
    target.set_line_numbers(options.line_numbers.into());
    target.set_aria_label(&options.aria_label);
}

/// The sole owner of a Monaco editor and its model.
///
/// Dropping it disposes both. Use [`Editor::handle`] for cloneable access
/// that does not keep the editor alive.
pub struct Editor {
    state: Rc<State>,
}

impl Editor {
    /// Creates an editor inside `host`. Sizing `host` is up to the caller.
    pub fn new(host: &HtmlElement, config: EditorConfig) -> Result<Self, EditorError> {
        config.validate()?;
        ensure_browser_target()?;

        let api = sys::editor_api().map_err(EditorError::js("initialize runtime"))?;
        let model = api
            .try_create_model(&config.initial_value, &config.language)
            .map_err(EditorError::js("create model"))?;

        let construction = StandaloneEditorConstructionOptions::builder()
            .model(&model)
            .automatic_layout(true)
            .build();
        apply_options(&construction, &config.options);
        let editor = match api.try_create(host, &construction) {
            Ok(editor) => editor,
            Err(cause) => {
                let _ = model.try_dispose();
                return Err(EditorError::JavaScript {
                    operation: "create editor",
                    cause,
                });
            }
        };

        let state = Rc::new_cyclic(|weak: &Weak<State>| {
            let weak = weak.clone();
            let closure: RelayClosure = Closure::new(move |version_id, is_flush| {
                if let Some(state) = weak.upgrade() {
                    state.dispatch(version_id, is_flush);
                }
            });
            State {
                disposed: Cell::new(false),
                api,
                editor,
                model,
                relay: ChangeRelay::new(closure.as_ref().unchecked_ref()),
                relay_closure: RefCell::new(Some(closure)),
                subscription: RefCell::new(None),
                callback: RefCell::new(noop_callback()),
            }
        });

        match state.relay.subscribe(&state.model) {
            Ok(subscription) => *state.subscription.borrow_mut() = Some(subscription),
            Err(cause) => {
                let _ = state.dispose();
                return Err(EditorError::JavaScript {
                    operation: "subscribe",
                    cause,
                });
            }
        }

        Ok(Self { state })
    }

    pub fn handle(&self) -> EditorHandle {
        EditorHandle {
            state: Rc::downgrade(&self.state),
        }
    }

    /// Replaces the change callback. The Monaco subscription is unchanged.
    pub fn set_on_change(&mut self, callback: impl Fn(ContentChange) + 'static) {
        if !self.state.disposed.get() {
            *self.state.callback.borrow_mut() = Rc::new(callback);
        }
    }

    /// Disposes the editor and model. Idempotent; every cleanup step is
    /// attempted and the first failure is returned.
    pub fn dispose(&mut self) -> Result<(), EditorError> {
        self.state.dispose()
    }
}

impl Drop for Editor {
    fn drop(&mut self) {
        if let Err(error) = self.state.dispose() {
            console::error_2(
                &"monaco-wasm: editor cleanup failed:".into(),
                &error.to_string().into(),
            );
        }
    }
}

/// A weak, cloneable reference to an [`Editor`]. Operations return
/// [`EditorError::Disposed`] once the owner is gone.
#[derive(Clone)]
pub struct EditorHandle {
    state: Weak<State>,
}

impl PartialEq for EditorHandle {
    fn eq(&self, other: &Self) -> bool {
        self.state.ptr_eq(&other.state)
    }
}

impl Eq for EditorHandle {}

impl std::fmt::Debug for EditorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EditorHandle")
            .field("live", &self.live().is_ok())
            .finish()
    }
}

impl EditorHandle {
    fn live(&self) -> Result<Rc<State>, EditorError> {
        self.state
            .upgrade()
            .filter(|state| !state.disposed.get())
            .ok_or(EditorError::Disposed)
    }

    /// Reads the whole document. O(document length).
    pub fn value(&self) -> Result<String, EditorError> {
        self.live()?
            .model
            .try_get_value()
            .map_err(EditorError::js("get value"))
    }

    /// Replaces the document text in the same model. Resets undo history and
    /// emits a normal change notification with `is_flush` set.
    pub fn replace_document(&self, value: &str) -> Result<(), EditorError> {
        self.live()?
            .model
            .try_set_value(value)
            .map_err(EditorError::js("set value"))
    }

    pub fn set_language(&self, language: &str) -> Result<(), EditorError> {
        validate_language(language)?;
        let state = self.live()?;
        state
            .api
            .try_set_model_language(&state.model, language)
            .map_err(EditorError::js("set language"))
    }

    /// Applies every supported option, resetting fields left at default.
    pub fn update_options(&self, options: &EditorOptions) -> Result<(), EditorError> {
        options.validate()?;
        let state = self.live()?;
        state
            .editor
            .try_update_options(&{
                let update = EditorUpdateOptions::new();
                apply_options(&update, options);
                update
            })
            .map_err(EditorError::js("update options"))
    }

    pub fn focus(&self) -> Result<(), EditorError> {
        self.live()?
            .editor
            .try_focus()
            .map_err(EditorError::js("focus"))
    }

    pub fn layout(&self) -> Result<(), EditorError> {
        self.live()?
            .editor
            .try_layout()
            .map_err(EditorError::js("layout"))
    }

    /// The raw editor, bypassing liveness checks. See [`sys`](crate::sys).
    pub fn raw_editor(&self) -> Result<StandaloneCodeEditor, EditorError> {
        Ok(self.live()?.editor.clone())
    }
}

/// Sets the theme of every editor on the page, initializing the runtime if
/// necessary. Accepts built-in or already registered theme names.
pub fn set_theme(name: &str) -> Result<(), EditorError> {
    ensure_browser_target()?;
    let api = sys::editor_api().map_err(EditorError::js("initialize runtime"))?;
    api.try_set_theme(name)
        .map_err(EditorError::js("set theme"))
}
