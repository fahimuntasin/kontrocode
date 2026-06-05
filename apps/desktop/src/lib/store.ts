/**
 * Global UI state — Solid.js signals/stores shared across components.
 * Uses the `createStore` pattern for fine-grained reactivity.
 */

import { createSignal, createMemo } from "solid-js";
import { createStore } from "solid-js/store";
import { batch } from "solid-js";
import type { AgentEvent, AppInfo, Profile } from "./api";
import { getAppInfo, memoryGetProfile } from "./api";

/* ------------------------------------------------------------------ *
 * Layout                                                             *
 * ------------------------------------------------------------------ */

export type LayoutMode = "default" | "no-sidebar" | "no-agent" | "focus";

const [layout, setLayout] = createSignal<LayoutMode>("default");
const [terminalVisible, setTerminalVisible] = createSignal(true);

export const ui = {
  layout,
  setLayout,
  terminalVisible,
  setTerminalVisible,
  toggleSidebar: () =>
    setLayout((l) => (l === "default" ? "no-sidebar" : "default")),
  toggleAgentPanel: () =>
    setLayout((l) => (l === "default" ? "no-agent" : "default")),
  toggleTerminal: () => setTerminalVisible((v) => !v),
  focusMode: () => setLayout("focus"),
  exitFocusMode: () => setLayout("default"),
};

/* ------------------------------------------------------------------ *
 * App info                                                           *
 * ------------------------------------------------------------------ */

const [appInfo, setAppInfo] = createSignal<AppInfo | null>(null);
export const info = {
  current: appInfo,
  refresh: async () => {
    try {
      setAppInfo(await getAppInfo());
    } catch (e) {
      console.error("getAppInfo failed", e);
    }
  },
};

/* ------------------------------------------------------------------ *
 * Profile                                                            *
 * ------------------------------------------------------------------ */

const [profile, setProfile] = createStore<{ value: Profile | null }>({ value: null });
export const profileStore = {
  current: createMemo(() => profile.value),
  refresh: async () => {
    try {
      setProfile("value", await memoryGetProfile());
    } catch (e) {
      console.error("memoryGetProfile failed", e);
    }
  },
  update: (p: Profile) => setProfile("value", p),
};

/* ------------------------------------------------------------------ *
 * File tree                                                          *
 * ------------------------------------------------------------------ */

export interface FileNode {
  name: string;
  path: string;
  kind: "file" | "directory";
  children?: FileNode[];
}

const [fileTree, setFileTree] = createSignal<FileNode | null>(null);
const [activeFile, setActiveFile] = createSignal<string | null>(null);

export const files = {
  tree: fileTree,
  setTree: (t: FileNode | null) => setFileTree(t),
  active: activeFile,
  setActive: (p: string | null) => setActiveFile(p),
};

/* ------------------------------------------------------------------ *
 * Agent chat                                                         *
 * ------------------------------------------------------------------ */

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  /** True while the assistant is still streaming. */
  streaming?: boolean;
  /** When the message was created. */
  at: number;
  /** Any error message attached to this message. */
  error?: string;
  /** Tool calls made during this message. */
  toolCalls?: Array<{
    name: string;
    args: unknown;
    result?: string;
  }>;
}

const [chat, setChat] = createStore<{ messages: ChatMessage[] }>({
  messages: [],
});

const [agentRunning, setAgentRunning] = createSignal(false);
const [activeSubscription, setActiveSubscription] = createSignal<
  string | null
>(null);

export const chatStore = {
  messages: () => chat.messages,
  running: agentRunning,
  subscription: activeSubscription,
  setRunning: (v: boolean) => setAgentRunning(v),
  setSubscription: (id: string | null) => setActiveSubscription(id),

  append: (msg: ChatMessage) => {
    setChat("messages", (prev) => [...prev, msg]);
  },
  appendDelta: (id: string, delta: string) => {
    setChat("messages", (m) => m.id === id, "content", (c) => c + delta);
  },
  finishStreaming: (id: string) => {
    setChat("messages", (m) => m.id === id, "streaming", false);
  },
  setError: (id: string, error: string) => {
    setChat("messages", (m) => m.id === id, "error", error);
  },
  addToolCall: (id: string, call: { name: string; args: unknown }) => {
    setChat("messages", (m) => m.id === id, "toolCalls", (tc) => [
      ...(tc ?? []),
      call,
    ]);
  },
  setToolResult: (id: string, name: string, result: string) => {
    setChat(
      "messages",
      (m) => m.id === id,
      "toolCalls",
      (tc) => tc?.map((c) => (c.name === name && !c.result ? { ...c, result } : c)),
    );
  },
  clear: () => {
    batch(() => {
      setChat("messages", []);
      setAgentRunning(false);
      setActiveSubscription(null);
    });
  },
};

/* ------------------------------------------------------------------ *
 * Research feed                                                      *
 * ------------------------------------------------------------------ */

export interface ResearchEntry {
  id: string;
  title: string;
  body: string;
  at: number;
}

const [research, setResearch] = createStore<{ entries: ResearchEntry[] }>({
  entries: [],
});

export const researchStore = {
  entries: () => research.entries,
  add: (e: { title: string; body: string }) => {
    setResearch("entries", (prev) => [
      ...prev,
      { ...e, id: crypto.randomUUID(), at: Date.now() },
    ]);
  },
  clear: () => setResearch("entries", []),
};

/* ------------------------------------------------------------------ *
 * Init — call once at app start                                      *
 * ------------------------------------------------------------------ */

export async function initStores(): Promise<void> {
  await Promise.all([info.refresh(), profileStore.refresh()]);
}

/** Apply an `AgentEvent` to the relevant store. */
export function applyAgentEvent(event: AgentEvent): void {
  switch (event.type) {
    case "started": {
      // Append a placeholder assistant message that will be streamed into.
      chatStore.append({
        id: event.message_id,
        role: "assistant",
        content: "",
        streaming: true,
        at: Date.now(),
      });
      return;
    }
    case "text_chunk": {
      chatStore.appendDelta(event.message_id, event.delta);
      return;
    }
    case "tool_call": {
      chatStore.addToolCall(event.message_id, {
        name: event.call.name,
        args: event.call.arguments,
      });
      return;
    }
    case "tool_result": {
      // tool_result events don't carry a message_id; attach to the
      // most recent assistant message that is still streaming.
      const messages = chat.messages;
      const last = messages[messages.length - 1];
      if (last && last.role === "assistant" && last.streaming) {
        chatStore.setToolResult(
          last.id,
          event.result.tool_name,
          typeof event.result.output === "string"
            ? event.result.output
            : JSON.stringify(event.result.output, null, 2),
        );
      }
      return;
    }
    case "research_update": {
      researchStore.add({ title: event.title, body: event.body });
      return;
    }
    case "done": {
      chatStore.finishStreaming(event.message_id);
      chatStore.setRunning(false);
      chatStore.setSubscription(null);
      return;
    }
    case "error": {
      // Attach to the most recent assistant message, or append a system one.
      const messages = chat.messages;
      const last = messages[messages.length - 1];
      if (last && last.role === "assistant" && last.streaming) {
        chatStore.setError(last.id, event.message);
        chatStore.finishStreaming(last.id);
      } else {
        chatStore.append({
          id: crypto.randomUUID(),
          role: "system",
          content: event.message,
          at: Date.now(),
          error: event.message,
        });
      }
      chatStore.setRunning(false);
      chatStore.setSubscription(null);
      return;
    }
  }
}
