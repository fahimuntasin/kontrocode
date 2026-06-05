/**
 * App shell. Composes the four regions: titlebar, sidebar, main, agent
 * panel, statusbar. Layout is data-driven via `ui.layout` so it can
 * collapse to focus mode.
 */

import { type Component, onMount } from "solid-js";
import { TitleBar } from "./components/TitleBar";
import { FileTree } from "./components/FileTree";
import { Editor } from "./components/Editor";
import { Terminal } from "./components/Terminal";
import { AgentPanel } from "./components/AgentPanel";
import { StatusBar } from "./components/StatusBar";
import { ui } from "./lib/store";

export const App: Component = () => {
  onMount(() => {
    // Global keyboard shortcuts.
    window.addEventListener("keydown", onKeydown);
    return () => window.removeEventListener("keydown", onKeydown);
  });

  function onKeydown(e: KeyboardEvent): void {
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key === "b") {
      e.preventDefault();
      ui.toggleSidebar();
    } else if (mod && e.shiftKey && e.key.toLowerCase() === "p") {
      e.preventDefault();
      // CommandPalette is a Phase 6 deliverable; placeholder for now.
      console.log("command palette (Phase 6)");
    } else if (mod && e.key === "`") {
      e.preventDefault();
      ui.toggleTerminal();
    } else if (mod && e.key === "k" && e.shiftKey) {
      e.preventDefault();
      ui.focusMode();
    } else if (e.key === "Escape" && ui.layout() === "focus") {
      ui.exitFocusMode();
    }
  }

  return (
    <div class="app" data-layout={ui.layout()} data-terminal-hidden={!ui.terminalVisible()}>
      <TitleBar />
      <aside class="sidebar" data-visible={ui.layout() !== "no-sidebar" && ui.layout() !== "focus"}>
        <FileTree />
      </aside>
      <main class="main" data-terminal-hidden={!ui.terminalVisible()}>
        <Editor />
        <Terminal />
      </main>
      <aside class="agent-panel" data-visible={ui.layout() !== "no-agent" && ui.layout() !== "focus"}>
        <AgentPanel />
      </aside>
      <StatusBar />
    </div>
  );
};
