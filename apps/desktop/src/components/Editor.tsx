/**
 * Code editor. Uses Monaco Editor in a webview. The KontroCode theme
 * is applied via `monaco-theme.css` and the `defineTheme` call.
 *
 * Phase 1: read-only display of the active file. Edit support comes
 * in Phase 5 with the diff/accept UI.
 */

import { type Component, createEffect, onCleanup, onMount, Show } from "solid-js";
import * as monaco from "monaco-editor";
import { files } from "../lib/store";
import { fileRead } from "../lib/api";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";

// Monaco requires explicit worker registration. We point each language
// at a bundled worker to avoid the no-eval fallback.
(self as unknown as { MonacoEnvironment?: unknown }).MonacoEnvironment = {
  getWorker(_: string, label: string) {
    if (label === "json") return new jsonWorker();
    if (label === "css" || label === "scss" || label === "less") return new cssWorker();
    if (label === "html" || label === "handlebars" || label === "razor") return new htmlWorker();
    if (label === "typescript" || label === "javascript") return new tsWorker();
    return new editorWorker();
  },
};

let themeDefined = false;

function defineKontroCodeTheme(): void {
  if (themeDefined) return;
  monaco.editor.defineTheme("kontrocode-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "comment", foreground: "6666AA", fontStyle: "italic" },
      { token: "keyword", foreground: "3A3AFF" },
      { token: "string", foreground: "00FFB2" },
      { token: "number", foreground: "FFB347" },
      { token: "type", foreground: "3A3AFF" },
      { token: "function", foreground: "00FFB2" },
      { token: "variable", foreground: "E8E8FF" },
      { token: "constant", foreground: "FFB347" },
    ],
    colors: {
      "editor.background": "#0D0D1A",
      "editor.foreground": "#E8E8FF",
      "editorCursor.foreground": "#3A3AFF",
      "editor.lineHighlightBackground": "#13132B",
      "editor.selectionBackground": "#3A3AFF33",
      "editorLineNumber.foreground": "#44447A",
      "editorLineNumber.activeForeground": "#6666AA",
      "editorIndentGuide.background": "#13132B",
      "editorIndentGuide.activeBackground": "#2A2A4E",
      "editorBracketMatch.background": "#3A3AFF22",
      "editorBracketMatch.border": "#3A3AFF",
      "scrollbarSlider.background": "#2A2A4E",
      "scrollbarSlider.hoverBackground": "#3F3F70",
      "scrollbarSlider.activeBackground": "#3A3AFF",
    },
  });
  themeDefined = true;
}

export const Editor: Component = () => {
  let host: HTMLDivElement | undefined;
  let editor: monaco.editor.IStandaloneCodeEditor | undefined;
  let currentModel: monaco.editor.ITextModel | undefined;

  onMount(() => {
    if (!host) return;
    defineKontroCodeTheme();
    editor = monaco.editor.create(host, {
      theme: "kontrocode-dark",
      automaticLayout: true,
      fontFamily: '"JetBrains Mono", "Zed Mono", "Menlo", monospace',
      fontSize: 13,
      fontLigatures: true,
      lineHeight: 1.55 * 13,
      lineNumbers: "on",
      glyphMargin: true,
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      renderLineHighlight: "line",
      cursorBlinking: "smooth",
      cursorSmoothCaretAnimation: "on",
      smoothScrolling: true,
      folding: true,
      readOnly: true,
      contextmenu: true,
      wordWrap: "off",
      tabSize: 2,
      guides: { indentation: true, bracketPairs: true },
      padding: { top: 8, bottom: 8 },
    });
  });

  // React to active file changes.
  createEffect(() => {
    const path = files.active();
    if (!editor || !path) return;
    void (async () => {
      try {
        const content = await fileRead(path);
        const language = detectLanguage(path);
        if (currentModel) currentModel.dispose();
        currentModel = monaco.editor.createModel(content, language);
        editor!.setModel(currentModel);
      } catch (e) {
        console.error("fileRead failed", e);
      }
    })();
  });

  onCleanup(() => {
    currentModel?.dispose();
    editor?.dispose();
  });

  return (
    <div style={{ position: "relative", height: "100%", "min-height": 0 }}>
      <div ref={host} style={{ position: "absolute", inset: 0 }} />
      <Show when={!files.active()}>
        <EmptyState />
      </Show>
    </div>
  );
};

const EmptyState: Component = () => (
  <div
    style={{
      position: "absolute",
      inset: 0,
      display: "flex",
      "align-items": "center",
      "justify-content": "center",
      "pointer-events": "none",
      "flex-direction": "column",
      gap: "8px",
    }}
  >
    <div
      style={{
        "font-family": "var(--font-code)",
        "font-size": "48px",
        "font-weight": "700",
        color: "var(--accent-primary)",
        opacity: 0.3,
      }}
    >
      K
    </div>
    <div class="muted" style={{ "font-size": "12px" }}>
      Open a file from the sidebar to start editing.
    </div>
  </div>
);

function detectLanguage(path: string): string {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  const map: Record<string, string> = {
    ts: "typescript",
    tsx: "typescript",
    js: "javascript",
    jsx: "javascript",
    mjs: "javascript",
    cjs: "javascript",
    json: "json",
    rs: "rust",
    toml: "ini",
    py: "python",
    md: "markdown",
    css: "css",
    scss: "scss",
    html: "html",
    htm: "html",
    yaml: "yaml",
    yml: "yaml",
    sh: "shell",
    bash: "shell",
    zsh: "shell",
    dart: "dart",
    go: "go",
    swift: "swift",
    kt: "kotlin",
    java: "java",
    c: "c",
    h: "c",
    cpp: "cpp",
    hpp: "cpp",
    cs: "csharp",
    php: "php",
    rb: "ruby",
    sql: "sql",
  };
  return map[ext] ?? "plaintext";
}
