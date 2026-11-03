use monaco_yaml::Yaml;
use monaco_yew::{Modules, MonacoEditor};
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    html! {
        <main>
            <h1>{"YAML module"}</h1>
            <p>{"Edit the YAML to see validation, completion, and hover information."}</p>
            <MonacoEditor language="yaml" modules={Modules::from([Yaml])}
                initial_value={"enabled: true\nitems: [one, two]\n"} />
        </main>
    }
}
fn main() {
    yew::Renderer::<App>::new().render();
}
