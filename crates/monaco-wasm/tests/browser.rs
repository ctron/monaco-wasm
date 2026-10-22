//! Browser tests for the core API.
//!
//!     cargo test -p monaco-wasm --target wasm32-unknown-unknown
//!
//! All tests share one page and therefore one Monaco runtime; leak checks
//! compare model and relay counts against a per-test baseline.

#![cfg(target_arch = "wasm32")]

use js_sys::{Error as JsError, Object, Promise, Reflect};
use monaco_wasm::{
    ContentChange, Editor, EditorConfig, EditorError, EditorHandle, EditorOptions, LineNumbers,
    WordWrap, set_theme, sys,
};
use std::cell::{Cell, RefCell};
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
export function rawOptions(editor) {
  const options = editor.getRawOptions();
  return {
    readOnly: options.readOnly,
    fontSize: options.fontSize,
    wordWrap: options.wordWrap,
    minimap: options.minimap.enabled,
    lineNumbers: options.lineNumbers,
    ariaLabel: options.ariaLabel,
  };
}
export function modelOf(editor) {
  return editor.getModel();
}
export function canUndo(editor) {
  return editor.getModel().canUndo();
}
export function languageOf(editor) {
  return editor.getModel().getLanguageId();
}
export function typeText(editor, text) {
  editor.trigger("test", "type", { text });
}
export function tokenTypes(api, text, language) {
  return api.tokenize(text, language).flat().map((token) => token.type);
}
export function hostHasTheme(host, theme) {
  return host.querySelector(".monaco-editor")?.classList.contains(theme) ?? false;
}

// Failure injection: each helper affects only the next matching call.
export function failNextCreate(api) {
  const original = api.create;
  api.create = function () {
    api.create = original;
    throw new Error("injected create failure");
  };
}
// Monaco's editor subscribes to its model while being created, so model
// patches are applied only after create() returns: they then affect exactly
// the subscription made by the change relay.
function afterNextCreate(api, patch) {
  const original = api.create;
  api.create = function (host, options) {
    api.create = original;
    const editor = original.call(this, host, options);
    patch(options.model);
    return editor;
  };
}
export function failNextSubscribe(api) {
  afterNextCreate(api, (model) => {
    model.onDidChangeContent = () => {
      throw new Error("injected subscribe failure");
    };
  });
}
// The returned disposable throws without unsubscribing, and the listener is
// kept so the test can invoke it after disposal.
export function failNextUnsubscribe(api, leaked) {
  afterNextCreate(api, (model) => {
    const subscribe = model.onDidChangeContent.bind(model);
    model.onDidChangeContent = (listener) => {
      leaked.listener = listener;
      subscribe(listener);
      return {
        dispose() {
          throw new Error("injected unsubscribe failure");
        },
      };
    };
  });
}
export function callLeaked(leaked) {
  leaked.listener({ versionId: 999, isFlush: false });
}
"#)]
extern "C" {
    fn settle() -> Promise;
    #[wasm_bindgen(js_name = rawOptions)]
    fn raw_options(editor: &sys::StandaloneCodeEditor) -> JsValue;
    #[wasm_bindgen(js_name = modelOf)]
    fn model_of(editor: &sys::StandaloneCodeEditor) -> JsValue;
    #[wasm_bindgen(js_name = canUndo)]
    fn can_undo(editor: &sys::StandaloneCodeEditor) -> bool;
    #[wasm_bindgen(js_name = languageOf)]
    fn language_of(editor: &sys::StandaloneCodeEditor) -> String;
    #[wasm_bindgen(js_name = typeText)]
    fn type_text(editor: &sys::StandaloneCodeEditor, text: &str);
    #[wasm_bindgen(js_name = tokenTypes)]
    fn token_types(api: &sys::EditorApi, text: &str, language: &str) -> Vec<String>;
    #[wasm_bindgen(js_name = hostHasTheme)]
    fn host_has_theme(host: &HtmlElement, theme: &str) -> bool;
    #[wasm_bindgen(js_name = failNextCreate)]
    fn fail_next_create(api: &sys::EditorApi);
    #[wasm_bindgen(js_name = failNextSubscribe)]
    fn fail_next_subscribe(api: &sys::EditorApi);
    #[wasm_bindgen(js_name = failNextUnsubscribe)]
    fn fail_next_unsubscribe(api: &sys::EditorApi, leaked: &Object);
    #[wasm_bindgen(js_name = callLeaked)]
    fn call_leaked(leaked: &Object);
}

/// Waits until queued microtasks, including change relays, have run.
async fn tick() {
    JsFuture::from(settle()).await.unwrap();
}

struct Host(HtmlElement);

impl Host {
    fn new() -> Self {
        let document = window().unwrap().document().unwrap();
        let host: HtmlElement = document.create_element("div").unwrap().unchecked_into();
        host.set_attribute("style", "width: 400px; height: 200px")
            .unwrap();
        document.body().unwrap().append_child(&host).unwrap();
        Self(host)
    }
}

impl Drop for Host {
    fn drop(&mut self) {
        self.0.remove();
    }
}

fn api() -> sys::EditorApi {
    sys::editor_api().unwrap()
}

/// (models, relays) currently alive in the page.
fn counts() -> (usize, u32) {
    (
        api().try_get_models().unwrap().len(),
        sys::active_relay_count(),
    )
}

fn json_config(text: &str) -> EditorConfig {
    let mut config = EditorConfig::default();
    config.initial_value = text.into();
    config.language = "json".into();
    config
}

fn recorder(editor: &mut Editor) -> Rc<RefCell<Vec<ContentChange>>> {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    editor.set_on_change(move |change| sink.borrow_mut().push(change));
    events
}

fn raw(handle: &EditorHandle) -> sys::StandaloneCodeEditor {
    handle.raw_editor().unwrap()
}

fn option(editor: &sys::StandaloneCodeEditor, name: &str) -> JsValue {
    Reflect::get(&raw_options(editor), &name.into()).unwrap()
}

#[wasm_bindgen_test]
fn creates_with_initial_text_language_and_defaults() {
    let host = Host::new();
    let editor = Editor::new(&host.0, json_config("{}")).unwrap();
    let handle = editor.handle();
    let raw = raw(&handle);

    assert_eq!(handle.value().unwrap(), "{}");
    assert_eq!(language_of(&raw), "json");
    assert_eq!(option(&raw, "readOnly"), false);
    assert_eq!(option(&raw, "fontSize"), 14.0);
    assert_eq!(option(&raw, "wordWrap"), "off");
    assert_eq!(option(&raw, "minimap"), false);
    assert_eq!(option(&raw, "lineNumbers"), "on");
    assert_eq!(option(&raw, "ariaLabel"), "Code editor");
}

#[wasm_bindgen_test]
fn every_option_changes_and_resets() {
    let host = Host::new();
    let editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let handle = editor.handle();
    let raw = raw(&handle);

    let mut changed = EditorOptions::default();
    changed.read_only = true;
    changed.font_size = 20.0;
    changed.word_wrap = WordWrap::On;
    changed.minimap_enabled = true;
    changed.line_numbers = LineNumbers::Off;
    changed.aria_label = "JSON input".into();
    handle.update_options(&changed).unwrap();
    assert_eq!(option(&raw, "readOnly"), true);
    assert_eq!(option(&raw, "fontSize"), 20.0);
    assert_eq!(option(&raw, "wordWrap"), "on");
    assert_eq!(option(&raw, "minimap"), true);
    assert_eq!(option(&raw, "lineNumbers"), "off");
    assert_eq!(option(&raw, "ariaLabel"), "JSON input");

    handle.update_options(&EditorOptions::default()).unwrap();
    assert_eq!(option(&raw, "readOnly"), false);
    assert_eq!(option(&raw, "fontSize"), 14.0);
    assert_eq!(option(&raw, "wordWrap"), "off");
    assert_eq!(option(&raw, "minimap"), false);
    assert_eq!(option(&raw, "lineNumbers"), "on");
    assert_eq!(option(&raw, "ariaLabel"), "Code editor");

    handle.set_language("json").unwrap();
    assert_eq!(language_of(&raw), "json");
    handle.set_language("plaintext").unwrap();
    assert_eq!(language_of(&raw), "plaintext");
}

#[wasm_bindgen_test]
fn invalid_values_are_rejected_before_mutation() {
    let host = Host::new();
    let baseline = counts();

    let mut config = EditorConfig::default();
    config.language = " ".into();
    assert!(matches!(
        Editor::new(&host.0, config),
        Err(EditorError::InvalidOptions(_))
    ));
    let mut config = EditorConfig::default();
    config.options.font_size = f64::INFINITY;
    assert!(matches!(
        Editor::new(&host.0, config),
        Err(EditorError::InvalidOptions(_))
    ));
    assert_eq!(counts(), baseline, "rejected configs create nothing");
    assert_eq!(host.0.child_element_count(), 0);

    let editor = Editor::new(&host.0, json_config("{}")).unwrap();
    let handle = editor.handle();
    let raw = raw(&handle);
    let mut invalid = EditorOptions::default();
    invalid.read_only = true;
    invalid.font_size = f64::NAN;
    assert!(matches!(
        handle.update_options(&invalid),
        Err(EditorError::InvalidOptions(_))
    ));
    assert_eq!(
        option(&raw, "readOnly"),
        false,
        "no field of a rejected update applies"
    );
    let mut invalid = EditorOptions::default();
    invalid.aria_label = "".into();
    assert!(handle.update_options(&invalid).is_err());
    assert!(matches!(
        handle.set_language(""),
        Err(EditorError::InvalidOptions(_))
    ));
    assert_eq!(language_of(&raw), "json");
}

#[wasm_bindgen_test]
async fn replacement_keeps_model_clears_undo_and_notifies_once() {
    let host = Host::new();
    let mut editor = Editor::new(&host.0, json_config("{}")).unwrap();
    let events = recorder(&mut editor);
    let handle = editor.handle();
    let raw = raw(&handle);
    let model = model_of(&raw);

    type_text(&raw, " ");
    assert!(can_undo(&raw));
    tick().await;
    events.borrow_mut().clear();

    handle.replace_document("[1]").unwrap();
    tick().await;
    assert_eq!(handle.value().unwrap(), "[1]");
    assert!(Object::is(&model, &model_of(&raw)), "same model instance");
    assert!(!can_undo(&raw), "undo history cleared");
    let events = events.borrow();
    assert_eq!(events.len(), 1, "exactly one notification: {events:?}");
    assert!(events[0].is_flush);
}

#[wasm_bindgen_test]
async fn changes_are_delivered_asynchronously_in_order() {
    let host = Host::new();
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let events = recorder(&mut editor);
    let raw = raw(&editor.handle());

    type_text(&raw, "a");
    type_text(&raw, "b");
    type_text(&raw, "c");
    assert!(
        events.borrow().is_empty(),
        "not delivered inside Monaco's mutation"
    );
    tick().await;

    let events = events.borrow();
    assert_eq!(events.len(), 3);
    assert!(events.iter().all(|event| !event.is_flush));
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].version_id < pair[1].version_id)
    );
}

#[wasm_bindgen_test]
async fn callback_can_read_and_replace_once() {
    let host = Host::new();
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let handle = editor.handle();
    let seen = Rc::new(RefCell::new(Vec::new()));
    let replaced = Rc::new(Cell::new(false));
    {
        let (handle, seen, replaced) = (handle.clone(), seen.clone(), replaced.clone());
        editor.set_on_change(move |change| {
            seen.borrow_mut()
                .push((handle.value().unwrap(), change.is_flush));
            if !replaced.replace(true) {
                handle.replace_document("replaced").unwrap();
            }
        });
    }

    type_text(&raw(&handle), "x");
    tick().await;
    tick().await;

    assert_eq!(handle.value().unwrap(), "replaced");
    // The first event reads the latest value, which already includes its own
    // replacement only after the callback ran; the replacement queues a
    // second, later event instead of recursing.
    assert_eq!(
        *seen.borrow(),
        vec![("x".to_string(), false), ("replaced".to_string(), true)]
    );
}

#[wasm_bindgen_test]
async fn callback_can_dispose_its_owner() {
    let host = Host::new();
    let baseline = counts();
    let owner: Rc<RefCell<Option<Editor>>> = Rc::new(RefCell::new(None));
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let handle = editor.handle();
    let calls = Rc::new(Cell::new(0));
    {
        let (owner, calls, handle) = (owner.clone(), calls.clone(), handle.clone());
        editor.set_on_change(move |_| {
            calls.set(calls.get() + 1);
            let editor = owner.borrow_mut().take();
            drop(editor);
            assert!(matches!(handle.value(), Err(EditorError::Disposed)));
        });
    }
    *owner.borrow_mut() = Some(editor);

    let raw = raw(&handle);
    type_text(&raw, "a");
    type_text(&raw, "b");
    tick().await;
    tick().await;

    assert_eq!(calls.get(), 1, "the second queued event is dropped");
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
    assert_eq!(counts(), baseline);
}

#[wasm_bindgen_test]
async fn replaced_callback_receives_already_queued_events() {
    let host = Host::new();
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let first = recorder(&mut editor);
    type_text(&raw(&editor.handle()), "a");
    let second = recorder(&mut editor);
    tick().await;
    assert!(first.borrow().is_empty());
    assert_eq!(second.borrow().len(), 1);
}

#[wasm_bindgen_test]
async fn events_queued_before_disposal_are_dropped() {
    let host = Host::new();
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let events = recorder(&mut editor);
    let handle = editor.handle();
    handle.replace_document("queued").unwrap();
    editor.dispose().unwrap();
    tick().await;
    assert!(events.borrow().is_empty());
}

#[wasm_bindgen_test]
async fn disposal_is_idempotent_and_handles_report_disposed() {
    let host = Host::new();
    let baseline = counts();
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let handle = editor.handle();
    let other = handle.clone();
    assert_eq!(handle, other);
    assert_eq!(counts(), (baseline.0 + 1, baseline.1 + 1));

    editor.dispose().unwrap();
    editor.dispose().unwrap();
    drop(editor);
    tick().await;

    assert_eq!(counts(), baseline);
    assert_eq!(host.0.child_element_count(), 0, "Monaco removed its DOM");
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
    assert!(matches!(
        handle.replace_document("x"),
        Err(EditorError::Disposed)
    ));
    assert!(matches!(
        handle.set_language("json"),
        Err(EditorError::Disposed)
    ));
    assert!(matches!(
        handle.update_options(&EditorOptions::default()),
        Err(EditorError::Disposed)
    ));
    assert!(matches!(handle.focus(), Err(EditorError::Disposed)));
    assert!(matches!(handle.layout(), Err(EditorError::Disposed)));
    assert!(matches!(handle.raw_editor(), Err(EditorError::Disposed)));
}

#[wasm_bindgen_test]
async fn dropping_the_owner_disposes() {
    let host = Host::new();
    let baseline = counts();
    let editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let handle = editor.handle();
    drop(editor);
    tick().await;
    assert_eq!(counts(), baseline);
    assert!(matches!(handle.value(), Err(EditorError::Disposed)));
}

#[wasm_bindgen_test]
async fn hundred_mount_dispose_cycles_return_to_baseline() {
    let host = Host::new();
    let baseline = counts();
    let mut handles = Vec::new();
    for i in 0..100 {
        let mut editor = Editor::new(&host.0, json_config(&format!("[{i}]"))).unwrap();
        let _events = recorder(&mut editor);
        editor.handle().replace_document("[]").unwrap();
        handles.push(editor.handle());
        if i % 2 == 0 {
            editor.dispose().unwrap();
        }
    }
    tick().await;
    assert_eq!(counts(), baseline);
    assert!(
        handles
            .iter()
            .all(|handle| matches!(handle.value(), Err(EditorError::Disposed)))
    );
}

#[wasm_bindgen_test]
async fn editors_are_isolated_and_theme_is_shared() {
    let (first_host, second_host) = (Host::new(), Host::new());
    let first = Editor::new(&first_host.0, json_config("1")).unwrap();
    let mut second = Editor::new(&second_host.0, json_config("2")).unwrap();
    let second_events = recorder(&mut second);
    let first_handle = first.handle();
    let second_handle = second.handle();
    assert_ne!(first_handle, second_handle);

    drop(first);
    type_text(&raw(&second_handle), "3");
    tick().await;
    assert_eq!(second_handle.value().unwrap(), "32");
    assert_eq!(second_events.borrow().len(), 1);
    assert!(matches!(first_handle.value(), Err(EditorError::Disposed)));

    let third_host = Host::new();
    let third = Editor::new(&third_host.0, EditorConfig::default()).unwrap();
    set_theme("vs-dark").unwrap();
    assert!(host_has_theme(&second_host.0, "vs-dark"));
    assert!(host_has_theme(&third_host.0, "vs-dark"));
    let fourth_host = Host::new();
    let _fourth = Editor::new(&fourth_host.0, EditorConfig::default()).unwrap();
    assert!(
        host_has_theme(&fourth_host.0, "vs-dark"),
        "creation keeps the chosen theme"
    );
    set_theme("vs").unwrap();
    drop(third);
}

fn assert_js_error(error: EditorError, expected_operation: &str, expected_message: &str) {
    match error {
        EditorError::JavaScript { operation, cause } => {
            assert_eq!(operation, expected_operation);
            let message = cause.unchecked_into::<JsError>().message();
            assert_eq!(
                String::from(message),
                expected_message,
                "original JS error preserved"
            );
        }
        other => panic!("expected a JavaScript error, got {other:?}"),
    }
}

#[wasm_bindgen_test]
async fn failed_editor_creation_cleans_up_the_model() {
    let host = Host::new();
    let baseline = counts();
    fail_next_create(&api());
    let error = Editor::new(&host.0, EditorConfig::default()).err().unwrap();
    assert_js_error(error, "create editor", "injected create failure");
    tick().await;
    assert_eq!(counts(), baseline);
}

#[wasm_bindgen_test]
async fn failed_subscription_cleans_up_editor_model_and_relay() {
    let host = Host::new();
    let baseline = counts();
    fail_next_subscribe(&api());
    let error = Editor::new(&host.0, EditorConfig::default()).err().unwrap();
    assert_js_error(error, "subscribe", "injected subscribe failure");
    tick().await;
    assert_eq!(counts(), baseline);
    assert_eq!(host.0.child_element_count(), 0, "editor DOM removed");
}

#[wasm_bindgen_test]
async fn failed_unsubscribe_still_disposes_and_detaches() {
    let host = Host::new();
    let baseline = counts();
    let leaked = Object::new();
    fail_next_unsubscribe(&api(), &leaked);
    let mut editor = Editor::new(&host.0, EditorConfig::default()).unwrap();
    let calls = Rc::new(Cell::new(0));
    {
        let calls = calls.clone();
        editor.set_on_change(move |_| calls.set(calls.get() + 1));
    }

    let error = editor.dispose().err().unwrap();
    assert_js_error(error, "unsubscribe", "injected unsubscribe failure");
    assert!(editor.dispose().is_ok(), "a second dispose is a no-op");
    tick().await;
    assert_eq!(
        counts(),
        baseline,
        "editor and model disposed despite the failure"
    );

    // Monaco never removed the listener; invoking it must not reach the
    // released Rust closure.
    call_leaked(&leaked);
    tick().await;
    assert_eq!(calls.get(), 0);
}

#[wasm_bindgen_test]
async fn yaml_is_highlighted() {
    let host = Host::new();
    let mut config = EditorConfig::default();
    config.initial_value = "name: monaco # comment\n".into();
    config.language = "yaml".into();
    let editor = Editor::new(&host.0, config).unwrap();
    // The language definition loads asynchronously on first use.
    tick().await;

    assert_eq!(language_of(&raw(&editor.handle())), "yaml");
    let types = token_types(&api(), "name: monaco # comment", "yaml");
    assert!(
        types.iter().any(|t| t == "type.yaml"),
        "key token: {types:?}"
    );
    assert!(
        types.iter().any(|t| t == "comment.yaml"),
        "comment token: {types:?}"
    );
}
