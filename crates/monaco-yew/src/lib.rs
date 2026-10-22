//! Yew component for the Monaco editor.
//!
//! ```no_run
//! use monaco_yew::MonacoEditor;
//! use yew::prelude::*;
//!
//! #[function_component(App)]
//! fn app() -> Html {
//!     html! {
//!         <MonacoEditor language="json" initial_value="{}" />
//!     }
//! }
//! ```
//!
//! The editor owns its text: `initial_value` is read once on mount and the
//! component never writes a value back on rerender. Read or replace text
//! through the [`EditorHandle`] passed to `on_ready`. To switch documents,
//! call [`EditorHandle::replace_document`] or remount with a new `key`.
//!
//! Supports client-side rendering. Under server-side rendering the component
//! renders its empty container and never touches browser APIs.

use monaco_wasm::{Editor, EditorConfig, validate_language};
use web_sys::{HtmlElement, console};
use yew::prelude::*;

pub use monaco_wasm::{
    self, ContentChange, EditorError, EditorHandle, EditorOptions, LineNumbers, WordWrap, set_theme,
};

/// Default container style; a `style` prop is appended and can override it.
const CONTAINER_STYLE: &str = "height: 20rem; width: 100%; min-width: 0;";
const HOST_STYLE: &str = "height: 100%; width: 100%;";

#[derive(Properties, PartialEq)]
pub struct MonacoEditorProps {
    /// Read once, on mount. Changing it later has no effect.
    #[prop_or_default]
    pub initial_value: AttrValue,
    /// Monaco language ID; updates the existing model.
    #[prop_or(AttrValue::from("plaintext"))]
    pub language: AttrValue,
    /// Applied on mount and whenever the value changes.
    #[prop_or_default]
    pub options: EditorOptions,
    #[prop_or_default]
    pub class: Classes,
    /// Appended to the default container style.
    #[prop_or_default]
    pub style: AttrValue,
    /// Called once, after the editor and its change subscription exist.
    #[prop_or_default]
    pub on_ready: Option<Callback<EditorHandle>>,
    /// Called for each content change, after Monaco applied it.
    #[prop_or_default]
    pub on_change: Option<Callback<ContentChange>>,
    /// Setup failures and failures of updates the component applies. Logged
    /// to the console when not set.
    #[prop_or_default]
    pub on_error: Option<Callback<EditorError>>,
}

pub enum Msg {
    Changed(ContentChange),
    SetupFailed(EditorError),
}

pub struct MonacoEditor {
    host: NodeRef,
    editor: Option<Editor>,
    failed: bool,
    /// Last language and options Monaco accepted.
    applied_language: AttrValue,
    applied_options: EditorOptions,
}

fn report(props: &MonacoEditorProps, error: EditorError) {
    match &props.on_error {
        Some(on_error) => on_error.emit(error),
        None => console::error_2(&"monaco-yew:".into(), &error.to_string().into()),
    }
}

impl MonacoEditor {
    fn mount(&mut self, ctx: &Context<Self>) {
        let Some(host) = self.host.cast::<HtmlElement>() else {
            return;
        };
        let props = ctx.props();
        let mut config = EditorConfig::default();
        config.initial_value = props.initial_value.to_string();
        config.language = props.language.to_string();
        config.options = props.options.clone();

        match Editor::new(&host, config) {
            Ok(mut editor) => {
                let link = ctx.link().clone();
                editor.set_on_change(move |change| link.send_message(Msg::Changed(change)));
                self.applied_language = props.language.clone();
                self.applied_options = props.options.clone();
                let handle = editor.handle();
                self.editor = Some(editor);
                if let Some(on_ready) = &props.on_ready {
                    on_ready.emit(handle);
                }
            }
            // Reported from update(), which also rerenders to show the alert.
            Err(error) => ctx.link().send_message(Msg::SetupFailed(error)),
        }
    }

    /// Applies changed language/options props. Both are validated before
    /// either is applied; a failed JS call leaves the field unapplied until
    /// that prop changes again.
    fn apply_updates(&mut self, ctx: &Context<Self>, old: &MonacoEditorProps) {
        let Some(editor) = &self.editor else {
            return;
        };
        let props = ctx.props();
        let language = (props.language != old.language && props.language != self.applied_language)
            .then_some(&props.language);
        let options = (props.options != old.options && props.options != self.applied_options)
            .then_some(&props.options);
        if language.is_none() && options.is_none() {
            return;
        }

        let validation = language
            .map_or(Ok(()), |language| validate_language(language))
            .and_then(|()| options.map_or(Ok(()), |options| options.validate()));
        if let Err(error) = validation {
            report(props, error);
            return;
        }

        let handle = editor.handle();
        if let Some(language) = language {
            match handle.set_language(language) {
                Ok(()) => self.applied_language = language.clone(),
                Err(error) => report(props, error),
            }
        }
        if let Some(options) = options {
            match handle.update_options(options) {
                Ok(()) => self.applied_options = options.clone(),
                Err(error) => report(props, error),
            }
        }
    }
}

impl Component for MonacoEditor {
    type Message = Msg;
    type Properties = MonacoEditorProps;

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            host: NodeRef::default(),
            editor: None,
            failed: false,
            applied_language: AttrValue::default(),
            applied_options: EditorOptions::default(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Changed(change) => {
                // The editor may have been disposed after the event was queued.
                if self.editor.is_some()
                    && let Some(on_change) = &ctx.props().on_change
                {
                    on_change.emit(change);
                }
                false
            }
            Msg::SetupFailed(error) => {
                self.failed = true;
                report(ctx.props(), error);
                true
            }
        }
    }

    fn changed(&mut self, ctx: &Context<Self>, old_props: &Self::Properties) -> bool {
        self.apply_updates(ctx, old_props);
        let props = ctx.props();
        props.class != old_props.class || props.style != old_props.style
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let props = ctx.props();
        let style = if props.style.is_empty() {
            AttrValue::from(CONTAINER_STYLE)
        } else {
            AttrValue::from(format!("{CONTAINER_STYLE} {}", props.style))
        };
        html! {
            <div class={props.class.clone()} {style}>
                <div ref={self.host.clone()} style={HOST_STYLE}></div>
                if self.failed {
                    <div role="alert">{"Editor could not be initialized"}</div>
                }
            </div>
        }
    }

    fn rendered(&mut self, ctx: &Context<Self>, first_render: bool) {
        if first_render {
            self.mount(ctx);
        }
    }

    fn destroy(&mut self, ctx: &Context<Self>) {
        if let Some(mut editor) = self.editor.take()
            && let Err(error) = editor.dispose()
        {
            report(ctx.props(), error);
        }
    }
}
