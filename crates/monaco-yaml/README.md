# monaco-yaml

Optional YAML highlighting and worker-backed language services for monaco-wasm 0.2.

```rust
use monaco_wasm::EditorConfig;
use monaco_yaml::Yaml;

let mut config = EditorConfig::default();
config.language = "yaml".into();
config.modules = [Yaml].into();
```

`Modules` accepts a single module (`Yaml.into()`), an array (`[Yaml].into()`),
a vector (`vec![Yaml].into()`), or a borrowed slice (`slice.into()`). Borrowed
slices and arrays clone their elements; owned inputs need no `Clone` bound.
Iterators of modules can also use `.collect::<Modules>()`.

Leptos accepts `modules=Yaml` or `modules=[Yaml]`, Dioxus accepts `modules: Yaml`
or `modules: [Yaml]`, and Yew accepts `modules={Modules::from(Yaml)}` with `Modules` imported from `monaco_yew`.

Framework components accept the same `modules` property. It is read once on mount.
Modules initialize before the editor and model. Successful registration happens
once per page and is visible to all editors. Disposing an editor does not unregister
YAML. Initialization failures use the editor's existing error path.

Includes validation, completion, hover, and formatting. Remote schema requests
are disabled. Runtime and worker assets are bundled; consumers need no npm,
CDN, or worker configuration. Uses the core runtime's documented CSP.
