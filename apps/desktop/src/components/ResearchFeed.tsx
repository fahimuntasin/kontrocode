/**
 * Research feed — the live accordion inside the agent panel that
 * shows what the agent is looking up.
 */

import { type Component, For, Show } from "solid-js";
import { researchStore } from "../lib/store";

export const ResearchFeed: Component = () => {
  return (
    <div
      class="research-feed"
      style={{
        flex: 1,
        "overflow-y": "auto",
        padding: "8px 0",
        display: "flex",
        "flex-direction": "column",
      }}
    >
      <Show
        when={researchStore.entries().length > 0}
        fallback={
          <div
            class="muted"
            style={{
              "text-align": "center",
              "margin-top": "32px",
              "font-size": "12px",
              padding: "0 16px",
              "line-height": 1.6,
            }}
          >
            Research lookups will appear here when the agent investigates
            libraries, APIs, or deprecations.
          </div>
        }
      >
        <For each={researchStore.entries()}>
          {(entry) => (
            <div
              class="research-entry"
              style={{
                padding: "10px 12px",
                "border-bottom": "1px solid var(--border-subtle)",
                "border-left": "2px solid var(--accent-secondary)",
                "background": "var(--bg-elevated)",
                "margin-bottom": "4px",
              }}
            >
              <div
                style={{
                  "font-size": "11px",
                  "font-weight": 600,
                  "text-transform": "uppercase",
                  "letter-spacing": "0.06em",
                  color: "var(--accent-secondary)",
                  "margin-bottom": "4px",
                }}
              >
                {entry.title}
              </div>
              <div
                class="mono"
                style={{
                  "font-size": "12px",
                  color: "var(--text-primary)",
                  "white-space": "pre-wrap",
                  "word-break": "break-word",
                }}
              >
                {entry.body}
              </div>
              <div
                class="faint"
                style={{
                  "font-size": "10px",
                  "margin-top": "6px",
                  "font-variant-numeric": "tabular-nums",
                }}
              >
                {new Date(entry.at).toLocaleTimeString()}
              </div>
            </div>
          )}
        </For>
      </Show>
    </div>
  );
};
