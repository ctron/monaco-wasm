use js_sys::Error as JsError;
use std::fmt;
use wasm_bindgen::{JsCast, JsValue};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum EditorError {
    /// The editor was disposed; handles outlived their owner.
    Disposed,
    /// Browser functionality was called outside `wasm32`.
    UnsupportedTarget,
    /// Configuration was rejected before any mutation.
    InvalidOptions(String),
    /// A JavaScript call threw; `cause` is the original thrown value.
    JavaScript {
        operation: &'static str,
        cause: JsValue,
    },
}

impl EditorError {
    pub(crate) fn js(operation: &'static str) -> impl FnOnce(JsValue) -> Self {
        move |cause| Self::JavaScript { operation, cause }
    }
}

impl fmt::Display for EditorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disposed => f.write_str("the editor has been disposed"),
            Self::UnsupportedTarget => f.write_str("Monaco requires a wasm32 browser target"),
            Self::InvalidOptions(reason) => write!(f, "invalid editor options: {reason}"),
            Self::JavaScript { operation, cause } => {
                let message = cause
                    .dyn_ref::<JsError>()
                    .map(|error| String::from(error.message()))
                    .or_else(|| cause.as_string())
                    .unwrap_or_else(|| format!("{cause:?}"));
                write!(f, "{operation} failed: {message}")
            }
        }
    }
}

impl std::error::Error for EditorError {}
