/**
 * Agent panel — the right-side dock with chat, research feed, and memory.
 *
 * Layout: three sub-panels stacked. The user can resize the chat
 * sub-panel by dragging the divider.
 */

import { type Component, createSignal, Show } from "solid-js";
import { Chat } from "./Chat";
import { ResearchFeed } from "./ResearchFeed";
import { MemoryPanel } from "./MemoryPanel";
import { researchStore, profileStore } from "../lib/store";

type Tab = "chat" | "research" | "memory";

export const AgentPanel: Component = () => {
  const [tab, setTab] = createSignal<Tab>("chat");

  return (
    <div class="agent-panel-inner" style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      <div
        class="agent-tabs"
        style={{
          display: "flex",
          "border-bottom": "1px solid var(--border-subtle)",
          background: "var(--bg-void)",
        }}
      >
        <TabButton
          active={tab() === "chat"}
          onClick={() => setTab("chat")}
          label="Chat"
        />
        <TabButton
          active={tab() === "research"}
          onClick={() => setTab("research")}
          label="Research"
          badge={researchStore.entries().length}
        />
        <TabButton
          active={tab() === "memory"}
          onClick={() => setTab("memory")}
          label="Memory"
          badge={profileStore.current()?.facts.length ?? 0}
        />
      </div>
      <Show when={tab() === "chat"}>
        <Chat />
      </Show>
      <Show when={tab() === "research"}>
        <ResearchFeed />
      </Show>
      <Show when={tab() === "memory"}>
        <MemoryPanel />
      </Show>
    </div>
  );
};

const TabButton: Component<{
  active: boolean;
  onClick: () => void;
  label: string;
  badge?: number;
}> = (props) => (
  <button
    type="button"
    onClick={props.onClick}
    class="agent-tab"
    data-active={props.active}
    style={{
      flex: 1,
      padding: "8px 12px",
      "font-size": "11px",
      "font-weight": 600,
      "text-transform": "uppercase",
      "letter-spacing": "0.08em",
      color: props.active ? "var(--accent-primary)" : "var(--text-muted)",
      "border-bottom": props.active
        ? "2px solid var(--accent-primary)"
        : "2px solid transparent",
      transition: "color var(--t-fast) var(--ease), border-color var(--t-fast) var(--ease)",
      display: "flex",
      "align-items": "center",
      "justify-content": "center",
      gap: "6px",
    }}
  >
    <span>{props.label}</span>
    <Show when={(props.badge ?? 0) > 0}>
      <span
        style={{
          "font-size": "10px",
          background: props.active ? "var(--accent-primary)" : "var(--bg-elevated)",
          color: props.active ? "var(--text-on-accent)" : "var(--text-muted)",
          padding: "1px 6px",
          "border-radius": "8px",
          "font-weight": 500,
          "letter-spacing": "0",
        }}
      >
        {props.badge}
      </span>
    </Show>
  </button>
);
