//! Browser tests for the Leptos component.
//!
//!     cargo test -p monaco-leptos --target wasm32-unknown-unknown

#![cfg(target_arch = "wasm32")]

use js_sys::{Object, Promise};
use leptos::mount::mount_to;
use leptos::prelude::*;
use monaco_leptos::monaco_wasm::sys;
use monaco_leptos::{
    ContentChange, EditorError, EditorHandle, EditorOptions, MonacoEditor, WordWrap,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{Element, HtmlElement, window};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen(inline_js = r#"
export function settle() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}
export function modelOf(editor) {
  return editor.getModel();
}
export function languageOf(editor) {
  return editor.getModel().getLanguageId();
}
export function rawOption(editor, name) {
  const options = editor.getRawOptions();
  return name === "minimap" ? options.minimap.enabled : options[name];
}
export function typeText(editor, text) {
  editor.trigger("test", "type", { text });
}
export function failNextCreate(api) {
  const original = api.create;
  api.create = function () {
    api.create = original;
    throw new Error("injected create failure");
  };
}
export function failNextSetLanguage(api) {
  const original = api.setModelLanguage;
  api.setModelLanguage = function () {
    api.setModelLanguage = original;
    throw new Error("injected language failure");
  };
}
"#)]
extern "C" {
    fn settle() -> Promise;
    #[wasm_bindgen(js_name = modelOf)]
    fn model_of(editor: &sys::StandaloneCodeEditor) -> JsValue;
    #[wasm_bindgen(js_name = languageOf)]
    fn language_of(editor: &sys::StandaloneCodeEditor) -> String;
    #[wasm_bindgen(js_name = rawOption)]
    fn raw_option(editor: &sys::StandaloneCodeEditor, name: &str) -> JsValue;
    #[wasm_bindgen(js_name = typeText)]
    fn type_text(editor: &sys::StandaloneCodeEditor, text: &str);
    #[wasm_bindgen(js_name = failNextCreate)]
    fn fail_next_create(api: &sys::EditorApi);
    #[wasm_bindgen(js_name = failNextSetLanguage)]
    fn fail_next_set_language(api: &sys::EditorApi);
}

/// Lets effects and queued change events run.
async fn tick() {
    for _ in 0..2 {
        JsFuture::from(settle()).await.unwrap();
    }
}

fn counts() -> (usize, u32) {
    let api = sys::editor_api().unwrap();
    (
        api.try_get_models().unwrap().len(),
        sys::active_relay_count(),
    )
}

/// Records every value a callback receives. A local StoredValue is a Copy,
/// Send handle, so view closures can capture it.
struct Recorder<T: 'static>(StoredValue<Vec<T>, LocalStorage>);

impl<T: 'static> Clone for Recorder<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> Copy for Recorder<T> {}

impl<T: 'static> Recorder<T> {
    fn new() -> Self {
        Self(StoredValue::new_local(Vec::new()))
    }
    fn callback(self) -> UnsyncCallback<T> {
        UnsyncCallback::new(move |value| self.0.update_value(|values| values.push(value)))
    }
    fn len(self) -> usize {
        self.0.with_value(Vec::len)
    }
}

impl<T: Clone + 'static> Recorder<T> {
    fn last(self) -> T {
        self.0
            .with_value(|values| values.last().cloned())
            .expect("callback was not called")
    }
}

/// Signals the tests drive; the editor is mounted while `show` is true and
/// remounted whenever `key` changes.
#[derive(Clone, Copy)]
struct Controls {
    show: RwSignal<bool>,
    key: RwSignal<u32>,
    language: RwSignal<String>,
    options: RwSignal<EditorOptions>,
    class: RwSignal<String>,
    style: RwSignal<String>,
}

struct Fixture {
    root: HtmlElement,
    unmount: Option<Box<dyn std::any::Any>>,
    controls: Controls,
    ready: Recorder<EditorHandle>,
    changes: Recorder<ContentChange>,
    errors: Recorder<EditorError>,
}

impl Fixture {
    fn mount(initial_value: &str, language: &str) -> Self {
        let document = window().unwrap().document().unwrap();
        let root: HtmlElement = document.create_element("div").unwrap().unchecked_into();
        document.body().unwrap().append_child(&root).unwrap();
        let controls = Controls {
            show: RwSignal::new(true),
            key: RwSignal::new(0),
            language: RwSignal::new(language.to_owned()),
            options: RwSignal::new(EditorOptions::default()),
            class: RwSignal::new(String::new()),
            style: RwSignal::new(String::new()),
        };
        let (ready, changes, errors) = (Recorder::new(), Recorder::new(), Recorder::new());
        let initial_value = initial_value.to_owned();
        let view = move || {
            let initial_value = initial_value.clone();
            view! {
                <Show when=move || controls.show.get()>
                    {
                        let initial_value = initial_value.clone();
                        move || {
                            controls.key.track();
                            view! {
                                <MonacoEditor
                                    initial_value=initial_value.clone()
                                    language=controls.language
                                    options=controls.options
                                    class=controls.class
                                    style=controls.style
                                    on_ready=ready.callback()
                                    on_change=changes.callback()
                                    on_error=errors.callback()
                                />
                            }
                        }
                    }
                </Show>
            }
        };
        let handle = mount_to(root.clone(), view);
        Self {
            root,
            unmount: Some(Box::new(handle)),
            controls,
            ready,
            changes,
            errors,
        }
    }

    fn handle(&self) -> EditorHandle {
        self.ready.last()
    }

    fn raw(&self) -> sys::StandaloneCodeEditor {
        self.handle().raw_editor().unwrap()
    }

    fn container(&self) -> HtmlElement {
        self.root.first_element_child().unwrap().unchecked_into()
    }

    fn alert(&self) -> Option<Element> {
        self.root.query_selector("[role=alert]").unwrap()
    }

    async fn destroy(mut self) {
        drop(self.unmount.take());
        tick().await;
        self.root.remove();
    }
}

#[wasm_bindgen_test]
async fn unconfigured_component_is_visible_and_style_is_appended() {
    let fixture = Fixture::mount("", "plaintext");
    tick().await;
    let container = fixture.container();
    assert_eq!(container.offset_height(), 320, "default height 20rem");
    assert!(container.offset_width() > 0);
    let host: HtmlElement = container.first_element_child().unwrap().unchecked_into();
    assert_eq!(host.offset_height(), 320, "host fills the container");
    assert!(host.query_selector(".monaco-editor").unwrap().is_some());

    fixture.controls.style.set("height: 100px".into());
    fixture.controls.class.set("custom".into());
    tick().await;
    let container = fixture.container();
    assert_eq!(
        container.offset_height(),
        100,
        "custom style overrides the default"
    );
    assert_eq!(container.class_name(), "custom");
    assert_eq!(fixture.ready.len(), 1);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn signal_updates_change_the_existing_editor() {
    let fixture = Fixture::mount("{}", "plaintext");
    tick().await;
    let raw = fixture.raw();
    let model = model_of(&raw);
    type_text(&raw, " ");
    let typed = fixture.handle().value().unwrap();

    let mut options = EditorOptions::default();
    options.read_only = true;
    options.font_size = 18.0;
    options.word_wrap = WordWrap::On;
    options.minimap_enabled = true;
    fixture.controls.options.set(options);
    fixture.controls.language.set("json".into());
    tick().await;
    assert_eq!(raw_option(&raw, "readOnly"), true);
    assert_eq!(raw_option(&raw, "fontSize"), 18.0);
    assert_eq!(raw_option(&raw, "wordWrap"), "on");
    assert_eq!(raw_option(&raw, "minimap"), true);
    assert_eq!(language_of(&raw), "json");

    fixture.controls.options.set(EditorOptions::default());
    tick().await;
    assert_eq!(raw_option(&raw, "readOnly"), false);
    assert_eq!(raw_option(&raw, "fontSize"), 14.0);

    assert!(Object::is(&raw, &fixture.raw()), "same editor");
    assert!(Object::is(&model, &model_of(&raw)), "same model");
    assert_eq!(fixture.handle().value().unwrap(), typed, "text kept");
    assert_eq!(fixture.ready.len(), 1, "not recreated");
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn changes_before_mount_are_not_lost() {
    let fixture = Fixture::mount("{}", "plaintext");
    // Effects have not run yet: the editor does not exist.
    let mut options = EditorOptions::default();
    options.read_only = true;
    fixture.controls.language.set("json".into());
    fixture.controls.options.set(options);
    tick().await;

    let raw = fixture.raw();
    assert_eq!(language_of(&raw), "json");
    assert_eq!(raw_option(&raw, "readOnly"), true);
    assert_eq!(fixture.ready.len(), 1);
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn changes_are_reported() {
    let fixture = Fixture::mount("", "plaintext");
    tick().await;
    type_text(&fixture.raw(), "a");
    tick().await;
    assert_eq!(fixture.changes.len(), 1);
    assert!(!fixture.changes.last().is_flush);

    fixture.handle().replace_document("b").unwrap();
    tick().await;
    assert_eq!(fixture.changes.len(), 2);
    assert!(fixture.changes.last().is_flush);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn hiding_disposes_and_retained_handles_report_disposed() {
    let baseline = counts();
    let fixture = Fixture::mount("x", "plaintext");
    tick().await;
    let handle = fixture.handle();
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));

    fixture.controls.show.set(false);
    tick().await;
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
    assert_eq!(counts(), baseline);
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn unmounting_the_app_disposes() {
    let baseline = counts();
    let fixture = Fixture::mount("x", "plaintext");
    tick().await;
    let handle = fixture.handle();
    fixture.destroy().await;
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
    assert_eq!(counts(), baseline);
}

#[wasm_bindgen_test]
async fn remount_creates_a_new_editor() {
    let fixture = Fixture::mount("first", "plaintext");
    tick().await;
    let first = fixture.handle();
    fixture.controls.key.update(|key| *key += 1);
    tick().await;
    let second = fixture.handle();
    assert_ne!(first, second);
    assert!(matches!(first.value(), Err(EditorError::Disposed)));
    assert_eq!(second.value().unwrap(), "first", "initial value used again");
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn invalid_update_is_reported_and_applies_neither_field() {
    let fixture = Fixture::mount("{}", "plaintext");
    tick().await;
    let raw = fixture.raw();
    let mut invalid = EditorOptions::default();
    invalid.font_size = -1.0;
    fixture.controls.options.set(invalid);
    fixture.controls.language.set("json".into());
    tick().await;
    assert!(matches!(
        fixture.errors.last(),
        EditorError::InvalidOptions(_)
    ));
    assert_eq!(
        language_of(&raw),
        "plaintext",
        "valid language not applied either"
    );

    let mut valid = EditorOptions::default();
    valid.read_only = true;
    fixture.controls.options.set(valid);
    tick().await;
    assert_eq!(raw_option(&raw, "readOnly"), true);
    assert_eq!(
        language_of(&raw),
        "plaintext",
        "language did not change again"
    );
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn failed_update_is_retried_only_by_a_relevant_change() {
    let fixture = Fixture::mount("{}", "plaintext");
    tick().await;
    let raw = fixture.raw();

    fail_next_set_language(&sys::editor_api().unwrap());
    fixture.controls.language.set("json".into());
    tick().await;
    match fixture.errors.last() {
        EditorError::JavaScript { operation, .. } => assert_eq!(operation, "set language"),
        other => panic!("unexpected error {other:?}"),
    }
    assert_eq!(language_of(&raw), "plaintext");

    let mut options = EditorOptions::default();
    options.read_only = true;
    fixture.controls.options.set(options);
    tick().await;
    assert_eq!(
        language_of(&raw),
        "plaintext",
        "no retry on unrelated changes"
    );
    assert_eq!(fixture.errors.len(), 1, "no retry loop");

    fixture.controls.language.set("plaintext".into());
    tick().await;
    fixture.controls.language.set("json".into());
    tick().await;
    assert_eq!(language_of(&raw), "json", "retried when the value changes");
    assert!(fixture.handle().value().is_ok(), "editor kept alive");
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn setup_failure_shows_an_alert_and_reports_the_error() {
    let baseline = counts();
    fail_next_create(&sys::editor_api().unwrap());
    let fixture = Fixture::mount("", "plaintext");
    tick().await;

    let alert = fixture.alert().expect("alert rendered");
    assert_eq!(
        alert.text_content().unwrap(),
        "Editor could not be initialized"
    );
    match fixture.errors.last() {
        EditorError::JavaScript { operation, .. } => assert_eq!(operation, "create editor"),
        other => panic!("unexpected error {other:?}"),
    }
    assert_eq!(fixture.ready.len(), 0);
    let host = fixture.container().first_element_child().unwrap();
    assert_eq!(host.child_element_count(), 0, "host left empty");
    assert_eq!(counts(), baseline);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn hundred_mount_cycles_return_to_baseline() {
    let baseline = counts();
    let fixture = Fixture::mount("", "json");
    tick().await;
    for _ in 0..100 {
        fixture.controls.show.set(false);
        tick().await;
        fixture.controls.show.set(true);
        tick().await;
    }
    assert_eq!(fixture.ready.len(), 101);
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));
    fixture.controls.show.set(false);
    tick().await;
    assert_eq!(counts(), baseline);
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}
