//! Handwritten imports of the bundled bridge module, which is ours rather
//! than Monaco's and has no upstream declarations.

use super::{Disposable, EditorApi, TextModel};
use js_sys::Function;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/js/runtime.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = getEditorApi)]
    pub(super) fn get_editor_api() -> Result<EditorApi, JsValue>;

    /// Queued, detachable forwarder of model content events.
    pub type ChangeRelay;

    /// `callback` receives `(versionId: number, isFlush: boolean)`.
    #[wasm_bindgen(constructor)]
    pub fn new(callback: &Function) -> ChangeRelay;

    #[wasm_bindgen(method, catch)]
    pub fn subscribe(this: &ChangeRelay, model: &TextModel) -> Result<Disposable, JsValue>;

    #[wasm_bindgen(method)]
    pub fn detach(this: &ChangeRelay);

    /// Relays created and not yet detached in this document.
    #[wasm_bindgen(js_name = activeRelayCount)]
    pub fn active_relay_count() -> u32;
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = queueMicrotask)]
    pub(crate) fn queue_microtask(callback: &Function);
}

#[wasm_bindgen(module = "/js/runtime.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = moduleRegistered)]
    pub fn module_registered(id: &str) -> Result<bool, JsValue>;
    #[wasm_bindgen(catch, js_name = markModuleRegistered)]
    pub fn mark_module_registered(id: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = getMonaco)]
    pub fn module_monaco() -> Result<JsValue, JsValue>;
    #[wasm_bindgen(catch, js_name = unregisterModuleWorker)]
    pub fn unregister_module_worker(label: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(catch, js_name = registerModuleWorker)]
    pub fn register_module_worker(label: &str, factory: &Function) -> Result<(), JsValue>;
}
