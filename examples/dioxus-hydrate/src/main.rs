//! Dioxus fullstack fixture for monaco-dioxus, built with `dx`.
//!
//! The server renders an empty editor host; the browser hydrates the page
//! and only then creates the editor. The counter proves hydration attached
//! event handlers.

use dioxus::prelude::*;
use monaco_dioxus::MonacoEditor;

const INITIAL: &str = "{\n  \"hydrated\": true\n}\n";

#[component]
fn App() -> Element {
    let mut clicks = use_signal(|| 0);
    rsx! {
        main {
            h1 { "monaco-dioxus hydration fixture" }
            button { id: "increment", onclick: move |_| clicks += 1, "Clicked {clicks}" }
            div { id: "editor",
                MonacoEditor { language: "json", initial_value: INITIAL }
            }
        }
    }
}

fn main() {
    dioxus::launch(App);
}
