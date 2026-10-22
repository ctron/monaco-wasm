// Projection of the Monaco 0.57 API surface used by monaco-wasm.
// Structural interfaces only: every member is reached through an object
// returned by getEditorApi(), never through a global namespace.
// facade-check.ts verifies this projection against upstream monaco.d.ts.

interface EditorApi {
  create(domElement: HTMLElement, options: StandaloneEditorConstructionOptions): StandaloneCodeEditor;
  createModel(value: string, language: string): TextModel;
  setModelLanguage(model: TextModel, languageId: string): void;
  setTheme(themeName: string): void;
  getModels(): TextModel[];
}

interface StandaloneCodeEditor {
  updateOptions(newOptions: EditorUpdateOptions): void;
  focus(): void;
  layout(): void;
  dispose(): void;
}

interface TextModel {
  getValue(): string;
  setValue(newValue: string): void;
  onDidChangeContent(listener: (event: ModelContentChangedEvent) => void): Disposable;
  dispose(): void;
}

interface Disposable {
  dispose(): void;
}

interface ModelContentChangedEvent {
  readonly versionId: number;
  readonly isFlush: boolean;
}

interface MinimapOptions {
  enabled?: boolean;
}

// Shared by wordWrap and lineNumbers.
type OnOff = "off" | "on";

interface EditorUpdateOptions {
  readOnly?: boolean;
  fontSize?: number;
  wordWrap?: OnOff;
  minimap?: MinimapOptions;
  lineNumbers?: OnOff;
  ariaLabel?: string;
}

interface StandaloneEditorConstructionOptions extends EditorUpdateOptions {
  model?: TextModel | null;
  automaticLayout?: boolean;
}
