//! Monaco editor for Rust/WASM applications.
//!
//! The Monaco runtime, its stylesheet, and the editor and JSON workers are
//! bundled in this crate and reach the application through `wasm-bindgen`'s
//! JS snippet support. No npm install, CDN, or worker setup is needed.
//!
//! Workers are created from `blob:` URLs, so a Content Security Policy must
//! allow `worker-src blob:`; editor styles are injected at first use.
//!
//! ```no_run
//! use monaco_wasm::{Editor, EditorConfig, EditorError};
//! use web_sys::HtmlElement;
//!
//! fn mount_editor(host: &HtmlElement) -> Result<Editor, EditorError> {
//!     let mut config = EditorConfig::default();
//!     config.initial_value = "{\n  \"enabled\": true\n}".into();
//!     config.language = "json".into();
//!     Editor::new(host, config)
//! }
//! ```

mod editor;
mod error;
mod options;
pub mod sys;

pub use editor::{ContentChange, Editor, EditorHandle, set_theme};
pub use error::EditorError;
pub use options::{EditorConfig, EditorOptions, LineNumbers, WordWrap};
