//! Plain Rust/WASM example: no framework, exported to a static page.

use monaco_wasm::{Editor, EditorConfig, EditorError, EditorHandle, EditorOptions};
use wasm_bindgen::prelude::*;
use web_sys::{HtmlElement, console};

const INITIAL: &str = "{\n  \"enabled\": true,\n  \"items\": [1, 2, 3]\n}\n";
const REPLACEMENT: &str = "{\n  \"replaced\": true\n}\n";

fn js_error(error: EditorError) -> JsValue {
    JsValue::from_str(&error.to_string())
}

/// Owns the mounted editor for the demo page.
#[wasm_bindgen]
pub struct App {
    host: HtmlElement,
    editor: Option<Editor>,
    read_only: bool,
}

#[wasm_bindgen]
impl App {
    #[wasm_bindgen(constructor)]
    pub fn new(host: HtmlElement) -> App {
        App {
            host,
            editor: None,
            read_only: false,
        }
    }

    pub fn mount(&mut self) -> Result<(), JsValue> {
        if self.editor.is_some() {
            return Ok(());
        }
        let mut config = EditorConfig::default();
        config.initial_value = INITIAL.into();
        config.language = "json".into();
        config.options.read_only = self.read_only;
        let mut editor = Editor::new(&self.host, config).map_err(js_error)?;
        editor.set_on_change(|change| {
            console::log_1(&format!("change: {change:?}").into());
        });
        self.editor = Some(editor);
        Ok(())
    }

    pub fn unmount(&mut self) -> Result<(), JsValue> {
        match self.editor.take() {
            Some(mut editor) => editor.dispose().map_err(js_error),
            None => Ok(()),
        }
    }

    pub fn mounted(&self) -> bool {
        self.editor.is_some()
    }

    pub fn save(&self) -> Result<String, JsValue> {
        self.handle()?.value().map_err(js_error)
    }

    #[wasm_bindgen(js_name = replaceDocument)]
    pub fn replace_document(&self) -> Result<(), JsValue> {
        self.handle()?
            .replace_document(REPLACEMENT)
            .map_err(js_error)
    }

    #[wasm_bindgen(js_name = toggleReadOnly)]
    pub fn toggle_read_only(&mut self) -> Result<bool, JsValue> {
        let read_only = !self.read_only;
        if let Some(editor) = &self.editor {
            let mut options = EditorOptions::default();
            options.read_only = read_only;
            editor.handle().update_options(&options).map_err(js_error)?;
        }
        self.read_only = read_only;
        Ok(read_only)
    }

    fn handle(&self) -> Result<EditorHandle, JsValue> {
        self.editor
            .as_ref()
            .map(Editor::handle)
            .ok_or_else(|| "editor is not mounted".into())
    }
}
