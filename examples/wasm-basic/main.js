// Relative URL, so the page works under any deployment path such as /demo/.
import init, { App } from "./pkg/wasm_basic.js";

await init();
const output = document.getElementById("output");
const app = new App(document.getElementById("editor"));
const report = (action) => {
  try {
    const result = action();
    if (result !== undefined) output.textContent = String(result);
  } catch (error) {
    output.textContent = `Error: ${error}`;
  }
};
report(() => app.mount());

document.getElementById("save").onclick = () => report(() => app.save());
document.getElementById("replace").onclick = () => report(() => app.replaceDocument());
document.getElementById("read-only").onclick = () =>
  report(() => `read-only: ${app.toggleReadOnly()}`);
const mountButton = document.getElementById("mount");
mountButton.onclick = () =>
  report(() => {
    app.mounted() ? app.unmount() : app.mount();
    mountButton.textContent = app.mounted() ? "Unmount" : "Mount";
  });
window.demoApp = app;
