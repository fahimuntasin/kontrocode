/**
 * Status bar — 24px tall, always visible. Shows active model, cost,
 * provider health, and current branch.
 */

import { type Component, Show } from "solid-js";
import { info } from "../lib/store";

export const StatusBar: Component = () => {
  return (
    <footer class="statusbar">
      <Show
        when={info.current()}
        fallback={
          <>
            <span class="status-item">
              <span class="dot" />
              <span class="faint">KontroCode</span>
            </span>
          </>
        }
      >
        {(i) => (
          <>
            <span class="status-item" title="Active model">
              <span class="dot ok" />
              <span class="mono">{i().default_model}</span>
            </span>
            <span class="status-item faint" title="Project root">
              <span>root:</span>
              <span class="mono">{i().project_root}</span>
            </span>
            <span class="status-item" title="Providers registered">
              <span class="dot ok" />
              <span>
                {i().provider_count} provider{i().provider_count === 1 ? "" : "s"}
              </span>
            </span>
            <span class="status-item faint" title="Research enabled">
              {i().research_enabled ? "research: on" : "research: off"}
            </span>
            <span style={{ flex: 1 }} />
            <span class="status-item faint">
              v{i().version} · tauri {i().tauri_version}
            </span>
          </>
        )}
      </Show>
    </footer>
  );
};
