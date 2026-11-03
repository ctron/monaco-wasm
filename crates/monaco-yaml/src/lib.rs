//! Optional YAML support. Pass [`Yaml`] in an editor's [`monaco_wasm::Modules`].
//! Registration happens before model creation, once per page, and persists
//! after editors are disposed. No npm or worker configuration is needed.
use monaco_wasm::{EditorError, Module, ModuleContext};
use wasm_bindgen::prelude::*;

/// YAML highlighting, validation, completion, hover, and formatting.
/// Remote schema downloads are disabled.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Yaml;

#[wasm_bindgen(module = "/js/yaml.js")]
extern "C" {
    #[wasm_bindgen(catch, js_name = install)]
    fn install_yaml(monaco: &JsValue) -> Result<(), JsValue>;
    #[wasm_bindgen(js_name = createWorker)]
    fn worker_factory() -> js_sys::Function;
}

impl Module for Yaml {
    fn id(&self) -> &'static str {
        "monaco-yaml"
    }

    fn initialize(&self, context: &ModuleContext) -> Result<(), EditorError> {
        if !cfg!(target_arch = "wasm32") {
            return Err(EditorError::UnsupportedTarget);
        }
        let monaco = context.monaco()?;
        // Validate the worker label before installing language providers.
        context.register_worker("yaml", &worker_factory())?;
        install_yaml(&monaco).map_err(|cause| EditorError::JavaScript {
            operation: "initialize YAML",
            cause,
        })
    }
}
