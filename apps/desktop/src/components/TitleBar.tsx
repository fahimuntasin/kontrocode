/**
 * Custom titlebar. The OS chrome is hidden (see tauri.conf.json
 * `decorations: false`) and we draw our own. Drag region is set on
 * the parent; buttons opt out.
 */

import { type Component, onMount } from "solid-js";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { info, ui } from "../lib/store";

export const TitleBar: Component = () => {
  onMount(() => {
    // Expose window controls once mounted.
  });

  async function minimize() {
    await getCurrentWindow().minimize();
  }
  async function toggleMaximize() {
    await getCurrentWindow().toggleMaximize();
  }
  async function close() {
    await getCurrentWindow().close();
  }

  return (
    <header class="titlebar" data-tauri-drag-region>
      <div class="window-controls">
        <button
          type="button"
          aria-label="Close"
          onClick={close}
          style={{ background: "var(--accent-error)" }}
        />
        <button
          type="button"
          aria-label="Minimize"
          onClick={minimize}
          style={{ background: "var(--accent-warning)" }}
        />
        <button
          type="button"
          aria-label="Maximize"
          onClick={toggleMaximize}
          style={{ background: "var(--accent-secondary)" }}
        />
      </div>
      <div class="title">
        KontroCode {info.current() ? `· ${info.current()!.version}` : ""}
      </div>
      <div class="window-controls">
        <button
          type="button"
          class="icon-button"
          aria-label="Toggle sidebar"
          title="Toggle sidebar (⌘B)"
          onClick={() => ui.toggleSidebar()}
        >
          <SidebarIcon />
        </button>
        <button
          type="button"
          class="icon-button"
          aria-label="Toggle agent panel"
          title="Toggle agent (⌘⇧P)"
          onClick={() => ui.toggleAgentPanel()}
        >
          <ChatIcon />
        </button>
        <button
          type="button"
          class="icon-button"
          aria-label="Focus mode"
          title="Focus mode (⌘⇧K)"
          onClick={() => ui.focusMode()}
        >
          <FocusIcon />
        </button>
      </div>
    </header>
  );
};

const SidebarIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
    <rect x="3" y="4" width="18" height="16" rx="2" />
    <line x1="9" y1="4" x2="9" y2="20" />
  </svg>
);

const ChatIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
    <path d="M21 12a8 8 0 1 1-3.1-6.3L21 4l-1 4.7A8 8 0 0 1 21 12z" />
  </svg>
);

const FocusIcon: Component = () => (
  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
    <polyline points="4 9 4 4 9 4" />
    <polyline points="20 9 20 4 15 4" />
    <polyline points="4 15 4 20 9 20" />
    <polyline points="20 15 20 20 15 20" />
  </svg>
);
