/**
 * Tauri IPC bridge. Every call into the Rust shell goes through this
 * module. The shapes here must match the `#[tauri::command]` return
 * types in `apps/desktop/src-tauri/src/lib.rs`.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/* ------------------------------------------------------------------ *
 * Types — must match `kontrocode_core::Profile` on the Rust side.   *
 * ------------------------------------------------------------------ */

export type ResponseStyle = "concise" | "balanced" | "detailed";
export type Language = "banglish" | "english" | "bengali";

export interface Preferences {
  response_style: ResponseStyle;
  language: Language;
  expertise_level: number;
}

export interface Fact {
  id: string;
  text: string;
  confidence: number;
  created_at: string;
  source: "explicit" | "implicit" | "inferred" | "imported";
}

export interface StackConfidence {
  name: string;
  confidence: number;
  last_seen: string;
}

export interface Interest {
  topic: string;
  score: number;
  decay_rate: number;
}

export interface Profile {
  user_id: string;
  summary: string;
  preferences: Preferences;
  stacks: StackConfidence[];
  facts: Fact[];
  interests: Interest[];
  last_updated: string;
}

export interface AppInfo {
  version: string;
  tauri_version: string;
  project_root: string;
  default_model: string;
  provider_count: number;
  research_enabled: boolean;
}

/* ------------------------------------------------------------------ *
 * Agent events — must match `kontrocode_agent::AgentEvent`.          *
 * ------------------------------------------------------------------ */

export interface ToolCall {
  id: string;
  name: string;
  arguments: unknown;
}

export interface ToolResult {
  tool_call_id: string;
  tool_name: string;
  success: boolean;
  output: string | unknown;
}

export type AgentEvent =
  | { type: "started"; message_id: string }
  | { type: "text_chunk"; message_id: string; delta: string }
  | { type: "tool_call"; message_id: string; call: ToolCall }
  | { type: "tool_result"; tool_call_id: string; result: ToolResult }
  | { type: "research_update"; title: string; body: string }
  | { type: "done"; message_id: string }
  | { type: "error"; message: string };

/* ------------------------------------------------------------------ *
 * Commands                                                           *
 * ------------------------------------------------------------------ */

/** Send a user message to the agent. Returns a subscription id. */
export async function agentSend(text: string): Promise<string> {
  return invoke<string>("cmd_agent_send", { text });
}

/** Return the conversation history (Phase 1: empty). */
export async function agentHistory(): Promise<unknown[]> {
  return invoke<unknown[]>("cmd_agent_history");
}

/** Cancel an in-flight agent run. Phase 1: no-op. */
export async function agentCancel(subscriptionId: string): Promise<void> {
  return invoke("cmd_agent_cancel", { subscriptionId });
}

/** Read the current user profile. */
export async function memoryGetProfile(): Promise<Profile> {
  return invoke<Profile>("cmd_memory_get_profile");
}

/** Update a fact's text. */
export async function memoryUpdateFact(id: string, text: string): Promise<void> {
  return invoke("cmd_memory_update_fact", { id, text });
}

/** Delete a fact. */
export async function memoryDeleteFact(id: string): Promise<void> {
  return invoke("cmd_memory_delete_fact", { id });
}

/** Read a file (rooted at the project root). */
export async function fileRead(path: string): Promise<string> {
  return invoke<string>("cmd_file_read", { path });
}

/** Write a file (rooted at the project root). */
export async function fileWrite(path: string, content: string): Promise<string> {
  return invoke<string>("cmd_file_write", { path, content });
}

/** Run a shell command. */
export async function shellRun(
  command: string,
  args: string[],
): Promise<string> {
  return invoke<string>("cmd_shell_run", { command, args });
}

/** Update the project root. */
export async function setProjectRoot(path: string): Promise<void> {
  return invoke("cmd_set_project_root", { path });
}

/** Get runtime information for the status bar. */
export async function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>("cmd_get_app_info");
}

/* ------------------------------------------------------------------ *
 * Event subscriptions                                                *
 * ------------------------------------------------------------------ */

/**
 * Subscribe to the stream of agent events for a single run.
 * Returns an unlisten function.
 */
export async function onAgentEvent(
  subscriptionId: string,
  handler: (event: AgentEvent) => void,
): Promise<UnlistenFn> {
  const eventName = `agent:event:${subscriptionId}`;
  return listen<AgentEvent>(eventName, (e) => handler(e.payload));
}
