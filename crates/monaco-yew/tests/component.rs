//! Browser tests for the Yew component.
//!
//!     cargo test -p monaco-yew --target wasm32-unknown-unknown

#![cfg(target_arch = "wasm32")]

use js_sys::{Object, Promise};
use monaco_yew::monaco_wasm::sys;
use monaco_yew::{ContentChange, EditorError, EditorHandle, EditorOptions, MonacoEditor, WordWrap};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{Element, HtmlElement, window};
use yew::prelude::*;
use yew::{AppHandle, Renderer};

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

/// Lets Yew render and queued change events run.
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

/// A parent whose rerenders, key, and mounting the tests control.
#[derive(Properties, PartialEq, Clone)]
struct HarnessProps {
    show: bool,
    key: u32,
    rerender: u32,
    initial_value: AttrValue,
    language: AttrValue,
    options: EditorOptions,
    class: Classes,
    style: AttrValue,
    on_ready: Option<Callback<EditorHandle>>,
    on_change: Option<Callback<ContentChange>>,
    on_error: Option<Callback<EditorError>>,
}

#[function_component(Harness)]
fn harness(props: &HarnessProps) -> Html {
    html! {
        <div>
            <span class="rerender">{props.rerender}</span>
            if props.show {
                <MonacoEditor
                    key={props.key.to_string()}
                    initial_value={props.initial_value.clone()}
                    language={props.language.clone()}
                    options={props.options.clone()}
                    class={props.class.clone()}
                    style={props.style.clone()}
                    on_ready={props.on_ready.clone()}
                    on_change={props.on_change.clone()}
                    on_error={props.on_error.clone()}
                />
            }
        </div>
    }
}

/// Records every value a callback receives.
struct Recorder<T>(Rc<RefCell<Vec<T>>>);

impl<T: 'static> Recorder<T> {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(Vec::new())))
    }
    fn callback(&self) -> Callback<T> {
        let sink = self.0.clone();
        Callback::from(move |value| sink.borrow_mut().push(value))
    }
    fn len(&self) -> usize {
        self.0.borrow().len()
    }
}

impl<T: Clone> Recorder<T> {
    fn last(&self) -> T {
        self.0
            .borrow()
            .last()
            .cloned()
            .expect("callback was not called")
    }
}

struct Fixture {
    root: Element,
    app: AppHandle<Harness>,
    props: HarnessProps,
    ready: Recorder<EditorHandle>,
    changes: Recorder<ContentChange>,
    errors: Recorder<EditorError>,
}

impl Fixture {
    async fn mount(initial_value: &str, language: &str) -> Self {
        let document = window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let (ready, changes, errors) = (Recorder::new(), Recorder::new(), Recorder::new());
        let props = HarnessProps {
            show: true,
            key: 0,
            rerender: 0,
            initial_value: AttrValue::from(initial_value.to_string()),
            language: AttrValue::from(language.to_string()),
            options: EditorOptions::default(),
            class: Classes::new(),
            style: AttrValue::default(),
            on_ready: Some(ready.callback()),
            on_change: Some(changes.callback()),
            on_error: Some(errors.callback()),
        };
        let app = Renderer::<Harness>::with_root_and_props(root.clone(), props.clone()).render();
        tick().await;
        Self {
            root,
            app,
            props,
            ready,
            changes,
            errors,
        }
    }

    async fn update(&mut self, change: impl FnOnce(&mut HarnessProps)) {
        change(&mut self.props);
        self.app.update(self.props.clone());
        tick().await;
    }

    fn handle(&self) -> EditorHandle {
        self.ready.last()
    }

    fn raw(&self) -> sys::StandaloneCodeEditor {
        self.handle().raw_editor().unwrap()
    }

    fn container(&self) -> HtmlElement {
        self.root
            .query_selector("span + div")
            .unwrap()
            .unwrap()
            .unchecked_into()
    }

    fn alert(&self) -> Option<Element> {
        self.root.query_selector("[role=alert]").unwrap()
    }

    /// Yew tears the app down asynchronously; wait so the next test's
    /// baseline does not include this editor.
    async fn destroy(self) {
        self.app.destroy();
        tick().await;
        self.root.remove();
    }
}

#[wasm_bindgen_test]
async fn unconfigured_component_is_visible_and_style_is_appended() {
    let mut fixture = Fixture::mount("", "plaintext").await;
    let container = fixture.container();
    assert_eq!(container.offset_height(), 320, "default height 20rem");
    assert!(container.offset_width() > 0);
    let host: HtmlElement = container.first_element_child().unwrap().unchecked_into();
    assert_eq!(host.offset_height(), 320, "host fills the container");
    assert!(host.query_selector(".monaco-editor").unwrap().is_some());

    fixture
        .update(|props| {
            props.style = "height: 100px".into();
            props.class = classes!("custom");
        })
        .await;
    let container = fixture.container();
    assert_eq!(
        container.offset_height(),
        100,
        "custom style overrides the default"
    );
    assert_eq!(container.class_name(), "custom");
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn text_and_model_survive_rerenders_and_initial_value_changes() {
    let mut fixture = Fixture::mount("{}", "json").await;
    let raw = fixture.raw();
    let model = model_of(&raw);
    type_text(&raw, " ");
    let typed = fixture.handle().value().unwrap();

    fixture.update(|props| props.rerender += 1).await;
    fixture
        .update(|props| props.initial_value = "[]".into())
        .await;

    assert_eq!(fixture.handle().value().unwrap(), typed);
    assert!(Object::is(&model, &model_of(&fixture.raw())), "same model");
    assert_eq!(fixture.ready.len(), 1, "editor not recreated");
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn options_and_language_update_the_existing_editor() {
    let mut fixture = Fixture::mount("{}", "plaintext").await;
    let raw = fixture.raw();

    let mut options = EditorOptions::default();
    options.read_only = true;
    options.font_size = 18.0;
    options.word_wrap = WordWrap::On;
    options.minimap_enabled = true;
    fixture
        .update(|props| {
            props.options = options.clone();
            props.language = "json".into();
        })
        .await;
    assert_eq!(raw_option(&raw, "readOnly"), true);
    assert_eq!(raw_option(&raw, "fontSize"), 18.0);
    assert_eq!(raw_option(&raw, "wordWrap"), "on");
    assert_eq!(raw_option(&raw, "minimap"), true);
    assert_eq!(language_of(&raw), "json");

    fixture
        .update(|props| props.options = EditorOptions::default())
        .await;
    assert_eq!(raw_option(&raw, "readOnly"), false);
    assert_eq!(raw_option(&raw, "fontSize"), 14.0);
    assert!(Object::is(&raw, &fixture.raw()), "same editor");
    assert_eq!(fixture.ready.len(), 1);
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn on_ready_fires_once_and_on_change_uses_the_newest_callback() {
    let mut fixture = Fixture::mount("", "plaintext").await;
    let replacement_ready = Recorder::new();
    let replacement_changes = Recorder::new();
    fixture
        .update(|props| {
            props.on_ready = Some(replacement_ready.callback());
            props.on_change = Some(replacement_changes.callback());
        })
        .await;
    assert_eq!(fixture.ready.len(), 1);
    assert_eq!(replacement_ready.len(), 0, "on_ready is not fired again");

    type_text(&fixture.raw(), "a");
    tick().await;
    assert_eq!(fixture.changes.len(), 0);
    assert_eq!(replacement_changes.len(), 1);
    assert!(!replacement_changes.last().is_flush);

    fixture.handle().replace_document("b").unwrap();
    tick().await;
    assert!(replacement_changes.last().is_flush);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn unmount_disposes_and_retained_handles_report_disposed() {
    let baseline = counts();
    let mut fixture = Fixture::mount("x", "plaintext").await;
    let handle = fixture.handle();
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));

    fixture.update(|props| props.show = false).await;
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
    assert_eq!(counts(), baseline);
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn key_change_remounts_with_the_new_initial_value() {
    let mut fixture = Fixture::mount("first", "plaintext").await;
    let first = fixture.handle();
    fixture
        .update(|props| {
            props.initial_value = "second".into();
            props.key += 1;
        })
        .await;
    let second = fixture.handle();
    assert_ne!(first, second);
    assert!(matches!(first.value(), Err(EditorError::Disposed)));
    assert_eq!(second.value().unwrap(), "second");
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn invalid_update_is_reported_and_applies_neither_field() {
    let mut fixture = Fixture::mount("{}", "plaintext").await;
    let raw = fixture.raw();
    let mut invalid = EditorOptions::default();
    invalid.font_size = -1.0;
    fixture
        .update(|props| {
            props.options = invalid;
            props.language = "json".into();
        })
        .await;
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
    fixture.update(|props| props.options = valid).await;
    assert_eq!(raw_option(&raw, "readOnly"), true);
    assert_eq!(fixture.errors.len(), 1);
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn failed_update_is_retried_only_by_a_relevant_prop_change() {
    let mut fixture = Fixture::mount("{}", "plaintext").await;
    let raw = fixture.raw();

    fail_next_set_language(&sys::editor_api().unwrap());
    fixture.update(|props| props.language = "json".into()).await;
    match fixture.errors.last() {
        EditorError::JavaScript { operation, .. } => assert_eq!(operation, "set language"),
        other => panic!("unexpected error {other:?}"),
    }
    assert_eq!(language_of(&raw), "plaintext");

    fixture.update(|props| props.rerender += 1).await;
    assert_eq!(
        language_of(&raw),
        "plaintext",
        "no retry on unrelated changes"
    );
    assert_eq!(fixture.errors.len(), 1, "no retry loop");

    fixture
        .update(|props| props.language = "plaintext".into())
        .await;
    fixture.update(|props| props.language = "json".into()).await;
    assert_eq!(language_of(&raw), "json", "retried when the prop changes");
    assert!(fixture.handle().value().is_ok(), "editor kept alive");
    fixture.destroy().await;
}

#[wasm_bindgen_test]
async fn setup_failure_shows_an_alert_and_reports_the_error() {
    let baseline = counts();
    fail_next_create(&sys::editor_api().unwrap());
    let fixture = Fixture::mount("", "plaintext").await;

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
    let mut fixture = Fixture::mount("", "json").await;
    for _ in 0..100 {
        fixture.update(|props| props.show = false).await;
        fixture.update(|props| props.show = true).await;
    }
    assert_eq!(fixture.ready.len(), 101);
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));
    fixture.update(|props| props.show = false).await;
    assert_eq!(counts(), baseline);
    assert_eq!(fixture.errors.len(), 0);
    fixture.destroy().await;
}
