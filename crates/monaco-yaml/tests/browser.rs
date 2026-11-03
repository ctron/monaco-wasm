#![cfg(target_arch = "wasm32")]
use monaco_wasm::{Editor, EditorConfig, EditorError, Module, ModuleContext, Modules, sys};
use monaco_yaml::Yaml;
use std::sync::atomic::{AtomicUsize, Ordering};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen(
    inline_js = "export function pause() { return new Promise(r => setTimeout(r, 100)); } export function jsonMarkers(api) { return api.getModelMarkers({owner:'json'}).length; } export function markers(api) { return api.getModelMarkers({owner:'yaml'}).length; } export function languages(monaco) { return monaco.languages.getLanguages().some(l => l.id === 'yaml'); } export function tokens(api) { return api.tokenize('name: monaco # comment', 'yaml').flat().map(t => t.type); } export function factory() { return () => { throw new Error('unused'); }; }"
)]
extern "C" {
    fn pause() -> js_sys::Promise;
    fn markers(api: &JsValue) -> usize;
    #[wasm_bindgen(js_name = jsonMarkers)]
    fn json_markers(api: &JsValue) -> usize;
    fn languages(monaco: &JsValue) -> bool;
    fn tokens(api: &JsValue) -> Vec<String>;
    fn factory() -> js_sys::Function;
}

fn make_host() -> web_sys::HtmlElement {
    let document = web_sys::window().unwrap().document().unwrap();
    let host = document.create_element("div").unwrap();
    host.set_attribute("style", "width:600px;height:300px")
        .unwrap();
    document.body().unwrap().append_child(&host).unwrap();
    host.unchecked_into()
}

#[derive(Debug)]
struct Count;
static CALLS: AtomicUsize = AtomicUsize::new(0);
impl Module for Count {
    fn id(&self) -> &'static str {
        "test-count"
    }
    fn initialize(&self, context: &ModuleContext) -> Result<(), EditorError> {
        assert!(
            !languages(&context.monaco()?),
            "count runs before YAML registration"
        );
        assert_eq!(
            sys::editor_api().unwrap().try_get_models().unwrap().len(),
            0,
            "modules run before model creation"
        );
        CALLS.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[wasm_bindgen_test(async)]
async fn yaml_modules_initialize_once_and_validate() {
    let host = make_host();
    let api: JsValue = sys::editor_api().unwrap().into();
    let mut config = EditorConfig::default();
    config.modules = Modules::new(vec![std::sync::Arc::new(Count), std::sync::Arc::new(Yaml)]);
    config.language = "yaml".into();
    config.initial_value = "broken: [".into();
    let editor = Editor::new(&host, config.clone()).unwrap();
    let handle = editor.handle();
    let types = tokens(&api);
    assert!(
        types.iter().any(|t| t == "type.yaml"),
        "key tokens: {types:?}"
    );
    assert!(
        types.iter().any(|t| t == "comment.yaml"),
        "comment tokens: {types:?}"
    );
    for _ in 0..100 {
        if markers(&api) > 0 {
            break;
        }
        wasm_bindgen_futures::JsFuture::from(pause()).await.unwrap();
    }
    assert!(markers(&api) > 0, "YAML worker reports syntax errors");
    handle.replace_document("valid: true\n").unwrap();
    for _ in 0..100 {
        wasm_bindgen_futures::JsFuture::from(pause()).await.unwrap();
        if markers(&api) == 0 {
            break;
        }
    }
    assert_eq!(markers(&api), 0);
    let json_host = make_host();
    let mut json_config = EditorConfig::default();
    json_config.language = "json".into();
    json_config.initial_value = "{broken".into();
    let json = Editor::new(&json_host, json_config).unwrap();
    for _ in 0..100 {
        if json_markers(&api) > 0 {
            break;
        }
        wasm_bindgen_futures::JsFuture::from(pause()).await.unwrap();
    }
    assert!(
        json_markers(&api) > 0,
        "JSON worker still validates alongside YAML"
    );
    assert_eq!(handle.value().unwrap(), "valid: true\n");
    drop(json);
    json_host.remove();
    drop(editor);
    let again = Editor::new(&host, config).unwrap();
    assert_eq!(CALLS.load(Ordering::SeqCst), 1);
    drop(again);
    assert_eq!(
        sys::editor_api().unwrap().try_get_models().unwrap().len(),
        0
    );
    assert_eq!(sys::active_relay_count(), 0);
    host.remove();
}

#[derive(Debug)]
struct Failure;
static ATTEMPTS: AtomicUsize = AtomicUsize::new(0);
impl Module for Failure {
    fn id(&self) -> &'static str {
        "test-failure"
    }
    fn initialize(&self, context: &ModuleContext) -> Result<(), EditorError> {
        ATTEMPTS.fetch_add(1, Ordering::SeqCst);
        context.register_worker("test-worker", &factory())?;
        Err(EditorError::InvalidOptions("expected failure".into()))
    }
}
#[wasm_bindgen_test]
fn failed_modules_retry_without_leaking_workers_or_models() {
    let host = make_host();
    let mut config = EditorConfig::default();
    config.modules = Modules::from([Failure]);
    for _ in 0..2 {
        assert!(matches!(
            Editor::new(&host, config.clone()),
            Err(EditorError::InvalidOptions(_))
        ));
    }
    assert_eq!(ATTEMPTS.load(Ordering::SeqCst), 2);
    assert_eq!(
        sys::editor_api().unwrap().try_get_models().unwrap().len(),
        0
    );
    host.remove();
}

#[derive(Debug)]
struct Conflict;
impl Module for Conflict {
    fn id(&self) -> &'static str {
        "test-conflict"
    }
    fn initialize(&self, context: &ModuleContext) -> Result<(), EditorError> {
        context.register_worker("json", &factory())
    }
}
#[wasm_bindgen_test]
fn built_in_worker_labels_cannot_be_replaced() {
    let host = make_host();
    let mut config = EditorConfig::default();
    config.modules = Modules::from([Conflict]);
    assert!(matches!(
        Editor::new(&host, config),
        Err(EditorError::JavaScript {
            operation: "register worker",
            ..
        })
    ));
    host.remove();
}
