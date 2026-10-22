//! Browser tests for the Dioxus component.
//!
//!     cargo test -p monaco-dioxus --features web --target wasm32-unknown-unknown

#![cfg(all(target_arch = "wasm32", feature = "web"))]

use dioxus::prelude::*;
use dioxus::web::{Config, launch::launch_virtual_dom};
use js_sys::{Object, Promise};
use monaco_dioxus::monaco_wasm::sys;
use monaco_dioxus::{ContentChange, EditorError, EditorHandle, EditorOptions, MonacoEditor};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{HtmlElement, window};

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
  return editor.getRawOptions()[name];
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

/// Lets Dioxus render and queued change events run.
async fn tick() {
    for _ in 0..3 {
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

type Log<T> = Rc<RefCell<Vec<T>>>;

/// What the harness records from the component's callbacks.
#[derive(Clone, Default)]
struct Recorders {
    ready: Log<EditorHandle>,
    changes: Log<ContentChange>,
    errors: Log<EditorError>,
}

#[derive(Props, Clone)]
struct HarnessProps {
    initial_value: String,
    recorders: Recorders,
}

impl PartialEq for HarnessProps {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

/// A parent whose state the tests change by clicking its buttons.
#[allow(non_snake_case)]
fn Harness(props: HarnessProps) -> Element {
    let mut show = use_signal(|| true);
    let mut key = use_signal(|| 0u32);
    let mut rerender = use_signal(|| 0u32);
    let mut language = use_signal(|| "plaintext".to_owned());
    let mut options = use_signal(EditorOptions::default);
    let mut style = use_signal(String::new);
    let Recorders {
        ready,
        changes,
        errors,
    } = props.recorders.clone();

    rsx! {
        span { class: "rerender", "{rerender}" }
        button { class: "show", onclick: move |_| show.toggle() }
        button { class: "key", onclick: move |_| key += 1 }
        button { class: "rerender", onclick: move |_| rerender += 1 }
        button { class: "json", onclick: move |_| language.set("json".to_owned()) }
        button { class: "plaintext", onclick: move |_| language.set("plaintext".to_owned()) }
        button {
            class: "options",
            onclick: move |_| {
                let mut changed = EditorOptions::default();
                changed.read_only = true;
                changed.font_size = 18.0;
                options.set(changed);
            },
        }
        button { class: "defaults", onclick: move |_| options.set(EditorOptions::default()) }
        button {
            class: "invalid",
            onclick: move |_| {
                let mut invalid = EditorOptions::default();
                invalid.font_size = -1.0;
                options.set(invalid);
                language.set("json".to_owned());
            },
        }
        button { class: "style", onclick: move |_| style.set("height: 100px".to_owned()) }
        div { class: "slot",
            if show() {
                MonacoEditor {
                    key: "{key}",
                    initial_value: props.initial_value.clone(),
                    language: language(),
                    options: options(),
                    class: "custom",
                    style: style(),
                    on_ready: move |handle| ready.borrow_mut().push(handle),
                    on_change: move |change| changes.borrow_mut().push(change),
                    on_error: move |error| errors.borrow_mut().push(error),
                }
            }
        }
    }
}

struct Fixture {
    root: web_sys::Element,
    recorders: Recorders,
}

impl Fixture {
    async fn mount(initial_value: &str) -> Self {
        let document = window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let recorders = Recorders::default();
        let props = HarnessProps {
            initial_value: initial_value.to_owned(),
            recorders: recorders.clone(),
        };
        launch_virtual_dom(
            VirtualDom::new_with_props(Harness, props),
            Config::new().rootelement(root.clone()),
        );
        tick().await;
        Self { root, recorders }
    }

    async fn click(&self, button: &str) {
        let element: HtmlElement = self
            .root
            .query_selector(&format!("button.{button}"))
            .unwrap()
            .unwrap_or_else(|| panic!("no button {button}"))
            .unchecked_into();
        element.click();
        tick().await;
    }

    fn ready(&self) -> usize {
        self.recorders.ready.borrow().len()
    }

    fn errors(&self) -> Vec<EditorError> {
        self.recorders.errors.borrow().clone()
    }

    fn handle(&self) -> EditorHandle {
        self.recorders
            .ready
            .borrow()
            .last()
            .cloned()
            .expect("on_ready was not called")
    }

    fn raw(&self) -> sys::StandaloneCodeEditor {
        self.handle().raw_editor().unwrap()
    }

    fn container(&self) -> HtmlElement {
        self.root
            .query_selector(".slot > div")
            .unwrap()
            .expect("editor container")
            .unchecked_into()
    }

    /// The app keeps running; hiding the editor disposes it so the next
    /// test starts from the same baseline.
    async fn finish(self) {
        if self.root.query_selector(".slot > div").unwrap().is_some() {
            self.click("show").await;
        }
        self.root.remove();
    }
}

#[wasm_bindgen_test]
async fn unconfigured_component_is_visible_and_style_is_appended() {
    let fixture = Fixture::mount("").await;
    let container = fixture.container();
    assert_eq!(container.offset_height(), 320, "default height 20rem");
    assert!(container.offset_width() > 0);
    assert_eq!(container.class_name(), "custom");
    let host: HtmlElement = container.first_element_child().unwrap().unchecked_into();
    assert_eq!(host.offset_height(), 320, "host fills the container");
    assert!(host.query_selector(".monaco-editor").unwrap().is_some());

    fixture.click("style").await;
    assert_eq!(
        fixture.container().offset_height(),
        100,
        "custom style overrides the default"
    );
    assert_eq!(fixture.ready(), 1);
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn prop_changes_update_the_existing_editor() {
    let fixture = Fixture::mount("{}").await;
    let raw = fixture.raw();
    let model = model_of(&raw);
    type_text(&raw, " ");
    let typed = fixture.handle().value().unwrap();

    fixture.click("options").await;
    fixture.click("json").await;
    assert_eq!(raw_option(&raw, "readOnly"), true);
    assert_eq!(raw_option(&raw, "fontSize"), 18.0);
    assert_eq!(language_of(&raw), "json");

    fixture.click("defaults").await;
    assert_eq!(raw_option(&raw, "readOnly"), false);
    assert_eq!(raw_option(&raw, "fontSize"), 14.0);

    fixture.click("rerender").await;
    assert!(Object::is(&raw, &fixture.raw()), "same editor");
    assert!(Object::is(&model, &model_of(&raw)), "same model");
    assert_eq!(fixture.handle().value().unwrap(), typed, "text kept");
    assert_eq!(fixture.ready(), 1, "not recreated");
    assert!(fixture.errors().is_empty());
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn changes_are_reported() {
    let fixture = Fixture::mount("").await;
    type_text(&fixture.raw(), "a");
    tick().await;
    {
        let changes = fixture.recorders.changes.borrow();
        assert_eq!(changes.len(), 1);
        assert!(!changes[0].is_flush);
    }
    fixture.handle().replace_document("b").unwrap();
    tick().await;
    {
        let changes = fixture.recorders.changes.borrow();
        assert_eq!(changes.len(), 2);
        assert!(changes[1].is_flush);
    }
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn hiding_disposes_and_retained_handles_report_disposed() {
    let baseline = counts();
    let fixture = Fixture::mount("x").await;
    let handle = fixture.handle();
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));

    fixture.click("show").await;
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
    assert_eq!(counts(), baseline);
    assert!(fixture.errors().is_empty());
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn key_change_remounts() {
    let fixture = Fixture::mount("text").await;
    let first = fixture.handle();
    fixture.click("key").await;
    let second = fixture.handle();
    assert_ne!(first, second);
    assert!(matches!(first.value(), Err(EditorError::Disposed)));
    assert_eq!(second.value().unwrap(), "text", "initial value used again");
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn invalid_update_is_reported_and_applies_neither_field() {
    let fixture = Fixture::mount("{}").await;
    let raw = fixture.raw();
    fixture.click("invalid").await;
    assert!(matches!(
        fixture.errors().last(),
        Some(EditorError::InvalidOptions(_))
    ));
    assert_eq!(
        language_of(&raw),
        "plaintext",
        "valid language not applied either"
    );

    fixture.click("options").await;
    assert_eq!(raw_option(&raw, "readOnly"), true);
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn failed_update_is_retried_only_by_a_relevant_change() {
    let fixture = Fixture::mount("{}").await;
    let raw = fixture.raw();

    fail_next_set_language(&sys::editor_api().unwrap());
    fixture.click("json").await;
    match fixture.errors().last() {
        Some(EditorError::JavaScript { operation, .. }) => assert_eq!(*operation, "set language"),
        other => panic!("unexpected error {other:?}"),
    }
    assert_eq!(language_of(&raw), "plaintext");

    fixture.click("options").await;
    fixture.click("rerender").await;
    assert_eq!(
        language_of(&raw),
        "plaintext",
        "no retry on unrelated changes"
    );
    assert_eq!(fixture.errors().len(), 1, "no retry loop");

    fixture.click("plaintext").await;
    fixture.click("json").await;
    assert_eq!(language_of(&raw), "json", "retried when the prop changes");
    assert!(fixture.handle().value().is_ok(), "editor kept alive");
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn setup_failure_shows_an_alert_and_reports_the_error() {
    let baseline = counts();
    fail_next_create(&sys::editor_api().unwrap());
    let fixture = Fixture::mount("").await;

    let alert = fixture
        .root
        .query_selector("[role=alert]")
        .unwrap()
        .expect("alert rendered");
    assert_eq!(
        alert.text_content().unwrap(),
        "Editor could not be initialized"
    );
    match fixture.errors().last() {
        Some(EditorError::JavaScript { operation, .. }) => assert_eq!(*operation, "create editor"),
        other => panic!("unexpected error {other:?}"),
    }
    assert_eq!(fixture.ready(), 0);
    let host = fixture.container().first_element_child().unwrap();
    assert_eq!(host.child_element_count(), 0, "host left empty");
    assert_eq!(counts(), baseline);
    fixture.finish().await;
}

#[wasm_bindgen_test]
async fn hundred_mount_cycles_return_to_baseline() {
    let baseline = counts();
    let fixture = Fixture::mount("").await;
    for _ in 0..100 {
        fixture.click("show").await;
        fixture.click("show").await;
    }
    assert_eq!(fixture.ready(), 101);
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));
    fixture.click("show").await;
    assert_eq!(counts(), baseline);
    assert!(fixture.errors().is_empty());
    fixture.finish().await;
}
