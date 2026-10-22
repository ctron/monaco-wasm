//! A page that already configured its own Monaco must get a setup error,
//! not have its MonacoEnvironment overwritten. Separate test binary, so the
//! runtime is uninitialized when the test starts.

#![cfg(target_arch = "wasm32")]

use monaco_wasm::{Editor, EditorConfig, EditorError};
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;
use web_sys::HtmlElement;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen(inline_js = r#"
export function installForeignEnvironment() {
  globalThis.MonacoEnvironment = { getWorkerUrl: () => "foreign.js" };
}
export function environmentIsForeign() {
  return globalThis.MonacoEnvironment?.getWorkerUrl?.() === "foreign.js";
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = installForeignEnvironment)]
    fn install_foreign_environment();
    #[wasm_bindgen(js_name = environmentIsForeign)]
    fn environment_is_foreign() -> bool;
}

#[wasm_bindgen_test]
fn foreign_monaco_environment_is_a_setup_error() {
    install_foreign_environment();
    let document = web_sys::window().unwrap().document().unwrap();
    let host: HtmlElement = document.create_element("div").unwrap().unchecked_into();
    document.body().unwrap().append_child(&host).unwrap();

    match Editor::new(&host, EditorConfig::default()) {
        Err(EditorError::JavaScript { operation, cause }) => {
            assert_eq!(operation, "initialize runtime");
            let message = String::from(cause.unchecked_into::<js_sys::Error>().message());
            assert!(message.contains("foreign MonacoEnvironment"), "{message}");
        }
        Err(other) => panic!("unexpected error {other:?}"),
        Ok(_) => panic!("editor created despite a foreign MonacoEnvironment"),
    }
    assert!(
        environment_is_foreign(),
        "foreign environment left untouched"
    );
    assert!(
        monaco_wasm::set_theme("vs").is_err(),
        "runtime stays uninitialized"
    );
    assert_eq!(host.child_element_count(), 0);
}
