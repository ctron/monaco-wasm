import { configureMonacoYaml } from "monaco-yaml";
import {
  conf,
  language,
} from "monaco-editor/esm/vs/languages/definitions/yaml/yaml.js";
export function install(monaco) {
  // monaco-worker-manager uses the pre-0.57 worker options. Adapt them
  // to the bundled Monaco protocol without replacing the shared API.
  const editor = {
    ...monaco.editor,
    createWebWorker(options) {
      const worker = Promise.resolve(
        globalThis.MonacoEnvironment.getWorker("", options.label),
      ).then((worker) => {
        worker.postMessage("ignore");
        worker.postMessage(options.createData);
        return worker;
      });
      return monaco.editor.createWebWorker({
        worker,
        host: options.host,
        keepIdleModels: options.keepIdleModels,
      });
    },
  };
  configureMonacoYaml({ ...monaco, editor }, { enableSchemaRequest: false });
  monaco.languages.setLanguageConfiguration("yaml", conf);
  monaco.languages.setMonarchTokensProvider("yaml", language);
}
