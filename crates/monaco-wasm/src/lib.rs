//! The [Monaco editor](https://microsoft.github.io/monaco-editor/) for Rust/WASM applications.
//!
//! The Monaco runtime, its stylesheet, and the editor and JSON workers are bundled in this crate and
//! reach the application through `wasm-bindgen`'s JS snippet support. There is no npm install, CDN,
//! or worker configuration.
//!
//! This is the framework-free core. Applications using a framework usually want one of its
//! components instead.
//!
//! # Yew
//!
//! [`monaco-yew`](https://docs.rs/monaco-yew) provides a `MonacoEditor` component for
//! [Yew](https://yew.rs). See the
//! [quickstart](https://github.com/ctron/monaco-wasm/tree/main/examples/quickstart-yew) and the
//! [live demo](https://ctron.github.io/monaco-wasm/yew/).
//!
//! # Leptos
//!
//! [`monaco-leptos`](https://docs.rs/monaco-leptos) provides a `MonacoEditor` component for
//! [Leptos](https://leptos.dev), including server-side rendering and hydration. See the
//! [quickstart](https://github.com/ctron/monaco-wasm/tree/main/examples/quickstart-leptos) and the
//! [live demo](https://ctron.github.io/monaco-wasm/leptos/).
//!
//! # Example
//!
//! ```rust
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
//!
//! Keep the returned [`Editor`] alive, for example in application state. The editor does not size
//! its host element; give the host a height.
//!
//! # Ownership and text
//!
//! - [`Editor`] owns the Monaco editor and its model. Dropping it, or calling [`Editor::dispose`],
//!   disposes both.
//! - [`EditorHandle`] is a cloneable, weak reference obtained from [`Editor::handle`]. It reads or
//!   replaces the text ([`EditorHandle::value`], [`EditorHandle::replace_document`]), changes the
//!   language or options, and focuses the editor. Once the owner is gone, every call returns
//!   [`EditorError::Disposed`]; a retained handle never keeps an editor alive.
//! - [`Editor::set_on_change`] receives a [`ContentChange`] for each change, delivered from a
//!   microtask after Monaco applied it. It carries a version, not the text, so a callback may read or
//!   replace the document, or dispose the editor, without reentrancy problems.
//!
//! The editor owns the document: read it when you need it rather than mirroring it into application
//! state on every change.
//!
//! # Options, languages, and themes
//!
//! [`EditorOptions`] holds the settings that can change at runtime; it is non-exhaustive, so start
//! from [`EditorOptions::default`] and assign fields. [`EditorHandle::update_options`] applies all
//! fields, so setting one back to its default resets it. Invalid values are rejected with
//! [`EditorError::InvalidOptions`] before anything changes.
//!
//! The bundled runtime registers JSON (with validation in a worker), YAML (syntax highlighting,
//! without validation), and plain text. [`set_theme`]
//! switches the theme of every editor on the page; the built-in themes are `vs`, `vs-dark`,
//! `hc-light`, and `hc-black`.
//!
//! # Deployment
//!
//! Deploy the whole build output of Trunk, wasm-pack, or cargo-leptos, including the generated
//! `snippets/` directory, which contains the bundled runtime (about 1.4 MiB gzipped). Nothing else is
//! downloaded at runtime; editor styles are injected on first use.
//!
//! With a Content Security Policy, the editor needs `worker-src blob:` (workers are created from
//! `blob:` URLs), `style-src 'unsafe-inline'`, and `font-src data:`. A complete policy that works:
//!
//! ```text
//! default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; connect-src 'self';
//! worker-src blob:; style-src 'self' 'unsafe-inline'; font-src data:; img-src data:
//! ```
//!
//! `script-src 'wasm-unsafe-eval'` and `connect-src 'self'` are needed by any wasm-bindgen
//! application.
//!
//! The page must not define its own `MonacoEnvironment` (for example from a separately loaded
//! Monaco); creating an editor then fails with a setup error instead of overwriting it.
//!
//! # Low-level access
//!
//! [`sys`] exposes the bindings the safe API is built on, and [`EditorHandle::raw_editor`] returns
//! the underlying editor object for anything the safe API does not cover.
//!
//! # Targets
//!
//! Browser functionality is only available on `wasm32`. On other targets, configuration types still
//! compile, and calls that would need the browser return [`EditorError::UnsupportedTarget`].

mod editor;
mod error;
mod options;
pub mod sys;

pub use editor::{ContentChange, Editor, EditorHandle, set_theme};
pub use error::EditorError;
pub use options::{EditorConfig, EditorOptions, LineNumbers, WordWrap, validate_language};
