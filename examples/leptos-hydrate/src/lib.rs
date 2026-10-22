//! cargo-leptos SSR + hydration fixture for monaco-leptos.
//!
//! The server renders an empty editor host; the browser hydrates the page
//! and only then creates the editor. The counter proves hydration attached
//! event handlers.

use leptos::prelude::*;
use monaco_leptos::MonacoEditor;

pub const INITIAL: &str = "{\n  \"hydrated\": true\n}\n";

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <title>"monaco-leptos hydration"</title>
                <link rel="icon" href="data:," />
                <HydrationScripts options />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let clicks = RwSignal::new(0);
    view! {
        <main>
            <h1>"monaco-leptos hydration fixture"</h1>
            <button id="increment" on:click=move |_| clicks.update(|clicks| *clicks += 1)>
                "Clicked " {move || clicks.get()}
            </button>
            <div id="editor">
                <MonacoEditor language="json" initial_value=INITIAL />
            </div>
        </main>
    }
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    leptos::mount::hydrate_body(App);
}
