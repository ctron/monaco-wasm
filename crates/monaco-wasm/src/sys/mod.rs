//! Low-level bindings to the bundled Monaco runtime.
//!
//! `generated` is produced by ts-gen from a facade checked against Monaco's
//! declarations (`tools/bindings`); `manual` binds the bundled bridge. Use the
//! `try_*` methods: the others do not catch JavaScript exceptions.
//!
//! Raw objects are mutable capabilities: callers must not dispose, replace
//! the model of, or keep using a raw editor obtained from an
//! [`EditorHandle`](crate::EditorHandle) after its owner is disposed. Objects
//! created directly through [`EditorApi`] are owned by the caller, who must
//! dispose them.

#[allow(clippy::all)]
mod generated;
mod manual;

pub use generated::*;
pub(crate) use manual::queue_microtask;
pub use manual::{ChangeRelay, active_relay_count};

use wasm_bindgen::JsValue;

/// Returns the `monaco.editor` API, initializing the bundled runtime on
/// first use. No ownership is attached to the returned object.
pub fn editor_api() -> Result<EditorApi, JsValue> {
    manual::get_editor_api()
}
