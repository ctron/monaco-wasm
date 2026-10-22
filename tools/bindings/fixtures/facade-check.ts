/// <reference path="../node_modules/monaco-editor/monaco.d.ts" />
/// <reference path="./facade.d.ts" />
// Compile-only proof that facade.d.ts is a faithful projection of upstream
// monaco.d.ts: upstream values satisfy the facade results, and facade
// arguments satisfy upstream parameters. Facade object types (TextModel, ...)
// only ever hold upstream objects, so parameters of those types are compared
// as their upstream equivalents.
import editorApi = monaco.editor;
type UpstreamModel = monaco.editor.ITextModel;

export const create: (
  domElement: HTMLElement,
  options: Omit<StandaloneEditorConstructionOptions, "model"> & { model?: UpstreamModel | null },
) => StandaloneCodeEditor = editorApi.create;
export const createModel: EditorApi["createModel"] = editorApi.createModel;
export const setModelLanguage: (model: UpstreamModel, languageId: string) => void =
  editorApi.setModelLanguage;
export const setTheme: EditorApi["setTheme"] = editorApi.setTheme;
export const getModels: EditorApi["getModels"] = editorApi.getModels;

declare const upstreamEditor: monaco.editor.IStandaloneCodeEditor;
declare const upstreamModel: UpstreamModel;
declare const upstreamEvent: monaco.editor.IModelContentChangedEvent;
export const editor: StandaloneCodeEditor = upstreamEditor;
export const model: TextModel = upstreamModel;
export const event: ModelContentChangedEvent = upstreamEvent;

declare const facadeUpdate: EditorUpdateOptions;
export const update: monaco.editor.IEditorOptions & monaco.editor.IGlobalEditorOptions = facadeUpdate;
