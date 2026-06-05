/**
 * Memory panel — view, edit, and delete stored facts and interests.
 * Wired to the `cmd_memory_*` Tauri commands.
 */

import { type Component, createSignal, For, Show } from "solid-js";
import { memoryDeleteFact, memoryUpdateFact, type Profile, type Fact } from "../lib/api";
import { profileStore } from "../lib/store";

export const MemoryPanel: Component = () => {
  return (
    <div
      class="memory-panel"
      style={{
        flex: 1,
        "overflow-y": "auto",
        padding: "12px",
      }}
    >
      <Show when={profileStore.current()} fallback={<EmptyState />}>
        {(p) => <MemoryContent profile={p()} />}
      </Show>
    </div>
  );
};

const MemoryContent: Component<{ profile: Profile }> = (props) => {
  return (
    <>
      <Section title="Summary">
        <p
          class="mono"
          style={{
            "font-size": "12px",
            color: "var(--text-primary)",
            "line-height": 1.6,
            "white-space": "pre-wrap",
          }}
        >
          {props.profile.summary || <span class="muted">No summary yet.</span>}
        </p>
      </Section>

      <Section title="Preferences">
        <div
          style={{
            display: "grid",
            "grid-template-columns": "auto 1fr",
            gap: "6px 12px",
            "font-size": "12px",
          }}
        >
          <span class="faint">Style</span>
          <span>{props.profile.preferences.response_style}</span>
          <span class="faint">Language</span>
          <span>{props.profile.preferences.language}</span>
          <span class="faint">Expertise</span>
          <span>{(props.profile.preferences.expertise_level * 100).toFixed(0)}%</span>
        </div>
      </Section>

      <Section title={`Stacks (${props.profile.stacks.length})`}>
        <Show
          when={props.profile.stacks.length > 0}
          fallback={<Empty>No stacks detected yet.</Empty>}
        >
          <For each={props.profile.stacks}>
            {(s) => (
              <div
                style={{
                  display: "flex",
                  "align-items": "center",
                  gap: "8px",
                  padding: "4px 0",
                  "font-size": "12px",
                }}
              >
                <span style={{ flex: 1 }}>{s.name}</span>
                <span
                  style={{
                    width: "60px",
                    height: "4px",
                    background: "var(--bg-elevated)",
                    "border-radius": "2px",
                    overflow: "hidden",
                  }}
                >
                  <span
                    style={{
                      display: "block",
                      width: `${s.confidence * 100}%`,
                      height: "100%",
                      background: "var(--accent-primary)",
                    }}
                  />
                </span>
                <span class="faint" style={{ width: "32px", "text-align": "right" }}>
                  {(s.confidence * 100).toFixed(0)}%
                </span>
              </div>
            )}
          </For>
        </Show>
      </Section>

      <Section title={`Facts (${props.profile.facts.length})`}>
        <Show
          when={props.profile.facts.length > 0}
          fallback={<Empty>No facts learned yet.</Empty>}
        >
          <For each={props.profile.facts}>
            {(f) => <FactRow fact={f} />}
          </For>
        </Show>
      </Section>

      <Section title={`Interests (${props.profile.interests.length})`}>
        <Show
          when={props.profile.interests.length > 0}
          fallback={<Empty>No interests tracked yet.</Empty>}
        >
          <For each={props.profile.interests}>
            {(i) => (
              <div
                style={{
                  display: "flex",
                  "align-items": "center",
                  gap: "8px",
                  padding: "3px 0",
                  "font-size": "12px",
                }}
              >
                <span style={{ flex: 1 }}>{i.topic}</span>
                <span class="faint">
                  {(i.score * 100).toFixed(0)}%
                </span>
              </div>
            )}
          </For>
        </Show>
      </Section>
    </>
  );
};

const FactRow: Component<{ fact: Fact }> = (props) => {
  const [editing, setEditing] = createSignal(false);
  const [draft, setDraft] = createSignal(props.fact.text);
  const [busy, setBusy] = createSignal(false);

  async function save() {
    setBusy(true);
    try {
      await memoryUpdateFact(props.fact.id, draft());
      const p = profileStore.current();
      if (p) {
        profileStore.update({
          ...p,
          facts: p.facts.map((f) =>
            f.id === props.fact.id ? { ...f, text: draft() } : f,
          ),
        });
      }
      setEditing(false);
    } catch (e) {
      console.error(e);
    } finally {
      setBusy(false);
    }
  }

  async function remove() {
    setBusy(true);
    try {
      await memoryDeleteFact(props.fact.id);
      const p = profileStore.current();
      if (p) {
        profileStore.update({
          ...p,
          facts: p.facts.filter((f) => f.id !== props.fact.id),
        });
      }
    } catch (e) {
      console.error(e);
    } finally {
      setBusy(false);
    }
  }

  return (
    <div
      style={{
        display: "flex",
        gap: "8px",
        "align-items": "flex-start",
        padding: "6px 0",
        "border-bottom": "1px solid var(--border-subtle)",
        "font-size": "12px",
      }}
    >
      <div style={{ flex: 1 }}>
        <Show
          when={editing()}
          fallback={
            <div style={{ color: "var(--text-primary)" }}>{props.fact.text}</div>
          }
        >
          <textarea
            value={draft()}
            onInput={(e) => setDraft(e.currentTarget.value)}
            rows={2}
            style={{
              width: "100%",
              resize: "vertical",
              background: "var(--bg-surface)",
              color: "var(--text-primary)",
              border: "1px solid var(--border-subtle)",
              "border-radius": "var(--radius-sm)",
              padding: "4px 6px",
              "font-family": "var(--font-ui)",
              "font-size": "12px",
              outline: "none",
            }}
          />
        </Show>
        <div
          class="faint"
          style={{
            "font-size": "10px",
            "margin-top": "2px",
            "text-transform": "uppercase",
            "letter-spacing": "0.06em",
          }}
        >
          {props.fact.source} · {(props.fact.confidence * 100).toFixed(0)}%
        </div>
      </div>
      <div style={{ display: "flex", gap: "4px" }}>
        <Show
          when={editing()}
          fallback={
            <button
              type="button"
              class="icon-button"
              aria-label="Edit"
              disabled={busy()}
              onClick={() => setEditing(true)}
            >
              ✎
            </button>
          }
        >
          <button
            type="button"
            class="icon-button"
            aria-label="Save"
            disabled={busy()}
            onClick={save}
            style={{ color: "var(--accent-secondary)" }}
          >
            ✓
          </button>
          <button
            type="button"
            class="icon-button"
            aria-label="Cancel"
            disabled={busy()}
            onClick={() => {
              setDraft(props.fact.text);
              setEditing(false);
            }}
          >
            ×
          </button>
        </Show>
        <button
          type="button"
          class="icon-button"
          aria-label="Delete"
          disabled={busy() || editing()}
          onClick={remove}
          style={{ color: "var(--accent-error)" }}
        >
          🗑
        </button>
      </div>
    </div>
  );
};

const Section: Component<{ title: string; children: unknown }> = (props) => (
  <section style={{ "margin-bottom": "16px" }}>
    <h3
      style={{
        "font-size": "10px",
        "font-weight": 600,
        "text-transform": "uppercase",
        "letter-spacing": "0.08em",
        color: "var(--text-muted)",
        "margin-bottom": "8px",
      }}
    >
      {props.title}
    </h3>
    {props.children as never}
  </section>
);

const Empty: Component<{ children: string }> = (props) => (
  <div class="faint" style={{ "font-size": "12px", "font-style": "italic" }}>
    {props.children}
  </div>
);

const EmptyState: Component = () => (
  <div
    class="muted"
    style={{
      "text-align": "center",
      "margin-top": "32px",
      "font-size": "12px",
      "line-height": 1.6,
    }}
  >
    Your developer profile will be built from implicit signals.
    <br />
    The more you use KontroCode, the smarter it gets.
  </div>
);
