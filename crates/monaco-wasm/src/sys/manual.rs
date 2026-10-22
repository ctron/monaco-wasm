//! Handwritten imports of the bundled bridge module, which is ours rather
//! than Monaco's and has no upstream declarations.

use js_sys::Function;
use wasm_bindgen::prelude::*;

use super::{Disposable, EditorApi, TextModel};

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
