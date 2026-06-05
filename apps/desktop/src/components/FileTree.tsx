/**
 * File tree. Phase 1: shows the project root and a flat list. Real
 * recursive tree comes in Phase 2 with the LSP integration. For now
 * we list the immediate children.
 */

import { type Component, createSignal, For, onMount, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { files, info, type FileNode } from "../lib/store";
import { setProjectRoot } from "../lib/api";

interface DirListing {
  path: string;
  entries: Array<{
    name: string;
    path: string;
    kind: "file" | "directory";
  }>;
}

export const FileTree: Component = () => {
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function loadRoot(path: string) {
    setLoading(true);
    setError(null);
    try {
      const listing = await invoke<DirListing>("plugin:fs|read_dir", {
        path,
      }).catch(async () => {
        // Fallback: shell `ls`-like listing via our own command.
        return null;
      });
      if (listing) {
        files.setTree({
          name: path.split("/").pop() ?? path,
          path,
          kind: "directory",
          children: listing.entries
            .sort((a, b) => {
              if (a.kind !== b.kind) return a.kind === "directory" ? -1 : 1;
              return a.name.localeCompare(b.name);
            })
            .map((e) => ({
              name: e.name,
              path: e.path,
              kind: e.kind,
            })),
        });
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function pickFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Open project folder",
    });
    if (typeof selected === "string") {
      await setProjectRoot(selected);
      await info.refresh();
      await loadRoot(selected);
    }
  }

  onMount(() => {
    const root = info.current()?.project_root;
    if (root) void loadRoot(root);
  });

  return (
    <>
      <div class="section-header">
        <span>Files</span>
        <button
          type="button"
          class="icon-button"
          aria-label="Open folder"
          title="Open folder"
          onClick={pickFolder}
        >
          +
        </button>
      </div>
      <div class="file-tree-body" style={{ flex: 1, "overflow-y": "auto", padding: "4px 0" }}>
        <Show when={loading()}>
          <div class="muted" style={{ padding: "8px 12px" }}>Loading…</div>
        </Show>
        <Show when={error()}>
          <div style={{ padding: "8px 12px", color: "var(--accent-error)" }}>
            {error()}
          </div>
        </Show>
        <Show when={!loading() && !error()}>
          <For each={files.tree()?.children ?? []}>
            {(entry) => <FileRow entry={entry} depth={0} />}
          </For>
          <Show when={(files.tree()?.children ?? []).length === 0}>
            <div class="muted" style={{ padding: "8px 12px" }}>
              Empty folder. Open one to start.
            </div>
          </Show>
        </Show>
      </div>
    </>
  );
};

const FileRow: Component<{ entry: FileNode; depth: number }> = (props) => {
  const isActive = () => files.active() === props.entry.path;
  return (
    <div
      class="file-row"
      data-active={isActive()}
      onClick={() => {
        if (props.entry.kind === "file") files.setActive(props.entry.path);
      }}
      style={{
        display: "flex",
        "align-items": "center",
        gap: "6px",
        padding: "3px 12px 3px " + (12 + props.depth * 12) + "px",
        "font-family": "var(--font-code)",
        "font-size": "12px",
        color: isActive() ? "var(--text-primary)" : "var(--text-muted)",
        background: isActive() ? "var(--bg-elevated)" : "transparent",
        "border-left": "2px solid " + (isActive() ? "var(--accent-primary)" : "transparent"),
        cursor: "pointer",
      }}
    >
      <span style={{ width: "10px", "text-align": "center" }}>
        {props.entry.kind === "directory" ? "▸" : " "}
      </span>
      <span style={{ "white-space": "nowrap", overflow: "hidden", "text-overflow": "ellipsis" }}>
        {props.entry.name}
      </span>
    </div>
  );
};
