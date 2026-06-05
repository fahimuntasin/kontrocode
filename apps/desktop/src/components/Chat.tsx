/**
 * Chat — the agent's conversational UI. A scrollable message list
 * with a sticky input at the bottom.
 */

import { type Component, createSignal, For, onMount, Show } from "solid-js";
import { agentSend, onAgentEvent, type AgentEvent } from "../lib/api";
import {
  applyAgentEvent,
  chatStore,
  type ChatMessage,
} from "../lib/store";

export const Chat: Component = () => {
  const [input, setInput] = createSignal("");
  let scrollEl: HTMLDivElement | undefined;
  let inputEl: HTMLTextAreaElement | undefined;

  // Auto-scroll on new messages.
  function scrollToBottom() {
    if (scrollEl) scrollEl.scrollTop = scrollEl.scrollHeight;
  }

  async function send() {
    const text = input().trim();
    if (!text || chatStore.running()) return;
    setInput("");

    // Append the user message immediately.
    chatStore.append({
      id: crypto.randomUUID(),
      role: "user",
      content: text,
      at: Date.now(),
    });
    scrollToBottom();

    try {
      chatStore.setRunning(true);
      const subId = await agentSend(text);
      chatStore.setSubscription(subId);
      const unlisten = await onAgentEvent(subId, (event: AgentEvent) => {
        applyAgentEvent(event);
        scrollToBottom();
      });
      // The unlisten is tied to the subscription lifetime; the
      // `done` / `error` event will end the run and the subscription
      // id will be cleared, but we still register the unlisten in a
      // registry to call it on subsequent runs (Phase 6 enhancement).
      void unlisten;
    } catch (e) {
      chatStore.append({
        id: crypto.randomUUID(),
        role: "system",
        content: String(e),
        at: Date.now(),
        error: String(e),
      });
      chatStore.setRunning(false);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void send();
    }
  }

  onMount(() => {
    inputEl?.focus();
  });

  return (
    <div class="chat" style={{ display: "flex", "flex-direction": "column", flex: 1, "min-height": 0 }}>
      <div
        ref={scrollEl}
        class="chat-scroll"
        style={{
          flex: 1,
          "overflow-y": "auto",
          padding: "12px",
          display: "flex",
          "flex-direction": "column",
          gap: "12px",
        }}
      >
        <Show
          when={chatStore.messages().length > 0}
          fallback={
            <div
              class="muted"
              style={{
                "text-align": "center",
                "margin-top": "32px",
                "font-size": "12px",
                "line-height": 1.6,
              }}
            >
              Ask KontroCode anything about your project.
              <br />
              Try: <em>"add a README with a hello world"</em>
            </div>
          }
        >
          <For each={chatStore.messages()}>
            {(m) => <MessageRow msg={m} />}
          </For>
        </Show>
      </div>
      <form
        class="chat-input"
        onSubmit={(e) => {
          e.preventDefault();
          void send();
        }}
        style={{
          display: "flex",
          gap: "8px",
          padding: "8px 12px",
          "border-top": "1px solid var(--border-subtle)",
          background: "var(--bg-void)",
        }}
      >
        <textarea
          ref={inputEl}
          value={input()}
          onInput={(e) => setInput(e.currentTarget.value)}
          onKeyDown={onKeydown}
          placeholder="Ask KontroCode…"
          rows={1}
          style={{
            flex: 1,
            resize: "none",
            background: "var(--bg-surface)",
            color: "var(--text-primary)",
            border: "1px solid var(--border-subtle)",
            "border-radius": "var(--radius-md)",
            padding: "8px 10px",
            "font-family": "var(--font-ui)",
            "font-size": "13px",
            "max-height": "120px",
            outline: "none",
          }}
        />
        <button
          type="submit"
          disabled={chatStore.running() || input().trim().length === 0}
          style={{
            background: "var(--accent-primary)",
            color: "var(--text-on-accent)",
            "border-radius": "var(--radius-md)",
            padding: "0 14px",
            "font-weight": 600,
            opacity:
              chatStore.running() || input().trim().length === 0 ? 0.5 : 1,
            cursor:
              chatStore.running() || input().trim().length === 0
                ? "not-allowed"
                : "pointer",
          }}
        >
          {chatStore.running() ? "…" : "Send"}
        </button>
      </form>
    </div>
  );
};

const MessageRow: Component<{ msg: ChatMessage }> = (props) => {
  const isUser = () => props.msg.role === "user";
  const isSystem = () => props.msg.role === "system";

  if (isSystem()) {
    return (
      <div
        style={{
          padding: "8px 12px",
          background: "var(--bg-elevated)",
          "border-left": "2px solid var(--accent-error)",
          "border-radius": "var(--radius-sm)",
          "font-size": "12px",
          color: "var(--accent-error)",
        }}
      >
        {props.msg.content}
      </div>
    );
  }

  return (
    <div
      style={{
        display: "flex",
        "flex-direction": "column",
        gap: "4px",
        "align-items": isUser() ? "flex-end" : "flex-start",
      }}
    >
      <div
        style={{
          "max-width": "92%",
          padding: "8px 10px",
          background: isUser() ? "var(--accent-primary)" : "var(--bg-elevated)",
          color: isUser() ? "var(--text-on-accent)" : "var(--text-primary)",
          "border-radius": "var(--radius-md)",
          "font-size": "13px",
          "line-height": 1.55,
          "white-space": "pre-wrap",
          "word-break": "break-word",
        }}
      >
        {props.msg.content}
        <Show when={props.msg.streaming}>
          <span
            style={{
              display: "inline-block",
              width: "6px",
              height: "12px",
              background: "var(--accent-primary)",
              "margin-left": "2px",
              "vertical-align": "text-bottom",
              animation: "blink 1s steps(2) infinite",
            }}
          />
        </Show>
      </div>
      <Show when={props.msg.toolCalls && props.msg.toolCalls.length > 0}>
        <div
          style={{
            "max-width": "92%",
            display: "flex",
            "flex-direction": "column",
            gap: "4px",
          }}
        >
          <For each={props.msg.toolCalls}>
            {(tc) => (
              <div
                style={{
                  padding: "4px 8px",
                  background: "var(--bg-surface)",
                  "border-left": "2px solid var(--accent-secondary)",
                  "border-radius": "var(--radius-sm)",
                  "font-family": "var(--font-code)",
                  "font-size": "11px",
                  color: "var(--text-muted)",
                }}
              >
                <span style={{ color: "var(--accent-secondary)" }}>→</span>{" "}
                {tc.name}
                <Show when={tc.result}>
                  <pre
                    style={{
                      "margin-top": "4px",
                      "white-space": "pre-wrap",
                      "word-break": "break-word",
                      color: "var(--text-primary)",
                    }}
                  >
                    {tc.result}
                  </pre>
                </Show>
              </div>
            )}
          </For>
        </div>
      </Show>
      <Show when={props.msg.error}>
        <div
          style={{
            "max-width": "92%",
            padding: "4px 8px",
            "border-left": "2px solid var(--accent-error)",
            "font-size": "11px",
            color: "var(--accent-error)",
          }}
        >
          {props.msg.error}
        </div>
      </Show>
    </div>
  );
};
