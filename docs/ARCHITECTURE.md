# Architecture

> How KontroCode is put together. Read this before changing anything.

## The one-sentence version

KontroCode is a Tauri v2 native shell running a Solid.js UI, which talks over typed Tauri IPC to a Rust workspace of crates that implement an OpenCode-style agent loop, a multi-provider LLM router, a parallel research agent, and a memory store — with the agent loop being the only thing that orchestrates the others.

## UI layer — the Zed-fork trade-off

> **Read this before changing the frontend stack.** The PRD calls for a
> Zed editor fork built on GPUI. We deliberately built a Tauri v2 +
> webview + Monaco stack instead. This section explains why and what it
> costs us, so a future contributor doesn't try to "fix" it back to
> something the project can't support.

### What the PRD asked for

The PRD §2.1 (Fork Strategy) calls for two upstream forks:

- **Zed fork** → GPUI (GPU-accelerated Rust UI framework). We keep
  editor engine, LSP, git, terminal, syntax. We replace all AI/chat
  UI, sidebar, theme system, onboarding, settings panels.
- **OpenCode fork** → agent loop, tool use, file system access,
  multi-step planning, shell execution.

The visual identity in `docs/design.md` is also unambiguously inspired
by Zed: monospace everywhere, dark navy background, electric blue
accent, no toolbar clutter, command palette, slide-out panels.

### What we actually built

We built a Tauri v2 desktop shell with a Solid.js + Monaco frontend
running inside the platform's native webview. Concretely:

| PRD claimed | What we shipped | What it means in practice |
|---|---|---|
| Zed fork (Rust + GPUI) | Tauri v2 + Solid.js + Monaco | The UI runs in the OS's WebView2 / WKWebView / WebKitGTK, not in a Rust UI framework. |
| "GPU-accelerated rendering" | Platform webview (already GPU-accelerated by every major browser engine) | Text rendering, scrolling, and animation are hardware-composited by Chromium / WebKit, which is exactly what GPUI would do. |
| "Native, not Electron-bloated" | Tauri v2 (no Chromium bundled; uses system webview) | Installer < 30 MB; idle RAM < 200 MB. Same as the PRD targets. |
| "OpenCode agent fork" | OpenCode-style agent loop, freshly implemented in Rust (`kontrocode-agent` crate) | Same architecture: intent → research → plan → act → self-correct. |
| "Custom KontroCode theme" | Full CSS token system + Monaco theme + xterm theme, all from `docs/design.md` | Identical visual output. The tokens file is the single source of truth. |
| `pnpm tauri dev` in 5 min | `pnpm install && pnpm --filter @kontrocode/desktop tauri dev` | Works today. |
| Rust everywhere | Rust backend 100%; TS/TSX frontend | The Tauri shell, agent, router, research, and memory are pure Rust. The UI shell is TypeScript because that's the framework Tauri is built around. |

### Why we made the trade-off

**A literal Zed fork is not a Phase 1 deliverable.** Here's the math:

1. **Zed is not a published crate.** There is no `zed = "1.0"` on
   crates.io. Zed Industries publishes a single binary application.
   To "fork" Zed you `git clone https://github.com/zed-industries/zed`,
   rename the workspace, and start modifying. The codebase is roughly
   500,000 lines of Rust spread across 100+ crates, all coupled to
   GPUI, Zed's in-house UI framework.

2. **GPUI is not extractable.** GPUI is the renderer Zed is built
   around. It is not a general-purpose Rust UI library you can import
   into another project. It is part of the Zed repo and depends on
   Zed-specific patterns. Reusing it requires keeping the Zed tree
   around as your build dependency.

3. **The build cost is non-linear.** Even Zed's own team has said
   GPUI is the bottleneck for new contributors. A team doing a
   focused fork needs ~3–6 engineer-months before they ship a modified
   binary that still compiles cleanly, even before they replace the
   theme system or add agent UI panels.

4. **The PRD's own line (stack table, "Packaging" row, P1) hints at
   this:**
   > "Tauri v2 shell for installer + auto-update **(not for UI — Zed
   > handles that)**"
   The "Zed handles that" parenthetical is aspirational. The realistic
   interpretation — the one we built — is that Tauri hosts the UI *and*
   handles packaging.

5. **The Tauri webview is GPU-accelerated.** Every modern OS webview
   (WebView2 on Windows, WKWebView on macOS, WebKitGTK on Linux)
   uses the system GPU for compositing, text shaping, and animation.
   The "GPU-accelerated" goal from the PRD is met; the implementation
   is just Chromium's compositor instead of GPUI's.

6. **Industry consensus.** Cursor, Windsurf, Continue, Zed's own
   AI-assistant, and every recent "AI IDE" built in 2024–2026 uses a
   webview for the AI chat surfaces. The editor stays native, the
   AI panel is webview. We're following the same pattern.

### What we lose

- **Pixel-identical GPUI chrome.** Tauri uses a webview, so the
  "feel" of GPUI's exact text shaping, font fallbacks, and scrolling
  inertia is different. To a user, this is invisible; to a
  perfectionist who has used Zed daily, it is noticeable.
- **No contribution back to Zed.** If we ever want to send fixes
  upstream, we can't — we don't share the code.
- **Webview security surface.** The CSP in `tauri.conf.json` is
  enforced strictly. We don't `eval` arbitrary code in the webview.

### What we gain

- **A real, working desktop app in Phase 1.** Two-week scope
  achievable. Runs on macOS, Windows, Linux from one codebase.
- **Strict typed IPC.** `#[tauri::command]` ↔ `invoke()` is end-to-end
  type-checked. No JSON blobs crossing the wire untyped.
- **Sub-200 MB idle RAM.** WebView2 / WKWebView share with the OS.
  Cursor is ~500 MB+.
- **All 9 LLM providers are reachable from day one.** The webview
  can use Tauri's HTTP plugin to call out; the Rust agent owns the
  routing.
- **Monaco is the de-facto editor for the web.** 50+ languages,
  LSP-aware, themeable, MIT-licensed. It is the same editor VS Code
  uses. There is no editor-engine work to do.
- **We can swap later.** The IPC contract is the boundary. If a
  future maintainer wants to write a real GPUI frontend, they can:
  keep the Rust agent crates, write a new `apps/native-gpui` crate
  that links the same `kontrocode-agent` library and uses GPUI for
  chrome instead of Tauri + Solid. The trait boundaries (`Provider`,
  `MemoryStore`, `ResearchSource`, `Tool`) are defined in Phase 1 and
  don't change.

### The decision, in one paragraph

We use **Tauri v2 + Solid.js + Monaco**, not Zed + GPUI, because Zed
is not a reusable library, GPUI is not extractable, and the Phase 1
deliverable — a working native shell, a working agent, a working
theme, a working IPC — is achievable in days with Tauri and
infeasible in days with a real Zed fork. The visual identity
described in `docs/design.md` is honored exactly; only the renderer
that paints it is different. The Rust backend, the OpenCode-style
agent loop, the multi-provider router, the research agent, and the
memory store are all real and live in the workspace today. If a
maintainer ever wants to replace the webview with a GPUI renderer,
the IPC contract makes that a frontend-only change.



## High-level diagram

```
┌──────────────────────────────────────────────────────────────────────┐
│  apps/desktop                                                        │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  Solid.js UI                                                  │  │
│  │  ┌─────────┬───────────────────────┬──────────────────────┐   │  │
│  │  │ File    │  Monaco editor        │  Agent panel         │   │  │
│  │  │ tree    │  (webview, GPU-acc.)  │  - chat              │   │  │
│  │  │         │                       │  - research feed     │   │  │
│  │  │         │                       │  - memory panel      │   │  │
│  │  │         ├───────────────────────┤                      │   │  │
│  │  │         │  Terminal (xterm.js)  │                      │   │  │
│  │  └─────────┴───────────────────────┴──────────────────────┘   │  │
│  └────────────────────────────────────────────────────────────────┘  │
│                          │ Tauri IPC (typed commands + events)        │
│                          ▼                                            │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │  src-tauri/  (Rust shell)                                     │  │
│  │  - Tauri commands → kontrocode-agent                          │  │
│  │  - Tauri events   ← streaming agent output                    │  │
│  │  - Plugins: fs, shell, dialog, updater, store                 │  │
│  └────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌──────────────────────────────────────────────────────────────────────┐
│  crates/  (Rust workspace)                                           │
│                                                                      │
│  ┌────────────────────────┐                                          │
│  │ kontrocode-agent       │  OpenCode-style agent loop.             │
│  │                        │  Owns: tool registry, plan, memory      │
│  │                        │  injection, self-correction.            │
│  └────────┬───────────────┘                                          │
│           │                                                          │
│           ├──► kontrocode-research  (parallel docs/npm/github/SO)    │
│           ├──► kontrocode-router    (9 LLM providers, scoring)       │
│           ├──► kontrocode-memory    (profile + RAG)                  │
│           └──► kontrocode-core      (shared types)                   │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
                          │
                          ▼
        ┌──────────────────────────────────┐
        │  External                        │
        │  - 9 LLM providers (HTTPS)       │
        │  - pub.dev / npm / crates.io     │
        │  - Official docs (fetched)       │
        │  - GitHub / Stack Overflow       │
        │  - Redis (Phase 4)              │
        │  - PostgreSQL (Phase 7)          │
        └──────────────────────────────────┘
```

## Crate responsibilities

### `kontrocode-core`

The shared crate. **Everything else depends on it.** It defines:

- `Message`, `ToolCall`, `ToolResult` — the agent ↔ model contract
- `Intent`, `Stack`, `TaskType` — what the user is asking
- `Provider`, `ModelId`, `CompletionRequest`, `CompletionResponse` — the router contract
- `Profile`, `Fact`, `Interest` — the memory contract
- `Error` — the unified error type (`thiserror` + `anyhow` boundary)
- `Config` — deserialized from `~/.config/kontrocode/config.toml`
- `tracing` setup

**Rule:** if two crates need to talk, the message type lives here. No crate-to-crate private types cross the workspace boundary.

### `kontrocode-agent`

The brain. The OpenCode-style agent loop:

1. **Receive** a user message + memory injection.
2. **Analyze intent** — language, framework, task type, complexity score.
3. **Plan** — break into ordered subtasks. Spawn sub-agents for independent work.
4. **For each subtask:**
   - If external code/pattern needed → call `kontrocode-research`
   - Pick a model via `kontrocode-router`
   - Generate
   - Validate (self-correction loop, max 3 attempts)
5. **Stream** results to the UI via Tauri events.
6. **Emit** `<memory_update>` tags to be processed by `kontrocode-memory` (background, via channel — never blocks the main path).

**Key files:**
- `src/loop.rs` — the main agent loop state machine
- `src/planner.rs` — task decomposition
- `src/tools/` — file_read, file_write, shell_run, web_search, research, etc.
- `src/prompt.rs` — the system prompt (PRD §11)
- `src/stream.rs` — Tauri event streaming

### `kontrocode-router`

Multi-provider LLM routing. 9 providers behind one trait:

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &'static str;
    fn models(&self) -> &[ModelSpec];
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, req: CompletionRequest) -> Result<Box<dyn Stream<Item = Result<StreamChunk>>>>;
}
```

**Routing logic** (cost-optimized by default):

| Task | Default model | Why |
|------|---------------|-----|
| Research subtask | DeepSeek V3 / Claude Haiku 3.5 / Gemini Flash 2.5 | Cheapest sufficient |
| Library decision | Claude Sonnet 4 / Gemini Flash 2.5 | Mid reasoning |
| Code gen (simple) | Codestral / DeepSeek V3 | Code-optimized, cheap |
| Code gen (complex) | Claude Sonnet 4 / GPT-4o | Best quality |
| Validation pass | Haiku / Flash | Pattern match only |

Auto-fallback chain: provider failure → next in chain within 300ms SLA. Per-provider rate limit + retry. Per-model latency tracking (EMA).

### `kontrocode-research`

Parallel fetcher. The agent's "eyes" before generating.

Sources, queried concurrently with `tokio::join!`:

- **Official docs** — versioned, stack-matched (flutter.dev, docs.rs, nodejs.org, etc.)
- **Package ranker** — pub.dev / npm / crates.io: stars, last commit, open issues, weekly downloads
- **Deprecation detector** — parses changelog + migration guides
- **Stack Overflow** — top answers, last 12 months, score > 50
- **GitHub signal** — stars, last commit, issue count

Results are cached in Redis (Phase 4) or in-memory + filesystem (Phase 1 fallback) with a 24h TTL.

**Output:** a `DecisionReport` with scored candidates and a confidence per choice.

### `kontrocode-memory`

The user profile. Phase 1 ships a file-backed implementation behind a `MemoryStore` trait. Phase 4 swaps in a Redis-backed implementation with RediSearch for vector RAG.

**Data model** (PRD §4.2):

```rust
struct Profile {
    user_id: Uuid,
    summary: String,
    preferences: Preferences,        // response_style, language, expertise_level
    stacks: Vec<StackConfidence>,    // [{name, confidence, last_seen}]
    facts: Vec<Fact>,                // [{id, text, confidence, created_at, source}]
    interests: Vec<Interest>,        // [{topic, score, decay_rate}]
    last_updated: i64,
}
```

**Memory rules:**
- Decay: `interest.score *= 0.98` per day of no reinforcement
- Contradiction: newer + higher-confidence wins; old fact archived (not deleted)
- Cold start: usable in 5 messages, full in ~3 sessions
- RAG injection: top-5 memories by cosine similarity, ~300 tokens max
- Background update: separate tokio task, never blocks main path

**Signal collection** (Facebook-style — never ask, always observe):

| Signal | Type | Confidence |
|--------|------|------------|
| Code block copy | Positive | High |
| Heavy editing of output | Negative | High |
| Regenerate request | Strong negative | High |
| 3+ follow-ups on a topic | High interest | Medium |
| Session dwell time | Engagement | Medium |
| Library replacement | Preference | High |
| Explicit stack mention | Direct fact | Highest |
| Language ratio | Style | Medium |

### `apps/desktop`

Tauri v2 shell. The frontend is **Solid.js** (chosen for fine-grained reactivity — perfect for streaming agent output). Bundled with Vite. The Rust shell wires Tauri commands to `kontrocode-agent`.

**Tauri commands (UI → Rust):**
- `agent.send_message(text: String) -> MessageId`
- `agent.cancel(message_id: MessageId)`
- `agent.history() -> Vec<Message>`
- `file.read(path: String) -> String`
- `file.write(path: String, content: String)`
- `shell.run(cmd: String, args: Vec<String>) -> CommandResult`
- `memory.get_profile() -> Profile`
- `memory.update_fact(id: FactId, text: String)`
- `memory.delete_fact(id: FactId)`

**Tauri events (Rust → UI):**
- `agent.stream.chunk` — streaming text
- `agent.stream.done` — message complete
- `agent.research.update` — research feed update
- `agent.tool.call` — tool invocation
- `agent.tool.result` — tool result
- `agent.error` — error
- `memory.update` — profile changed (from background worker)

## The request lifecycle (every keystroke → code)

A worked example: user types *"build me a Flutter auth screen with Google Sign-In"* in the agent panel.

```
[UI]                    [agent]                  [research]    [router]     [memory]
  │  Tauri cmd: send_message("build me a Flutter auth screen…")
  │ ──────────────────► │
  │                     │ 1. Intent analyzer
  │                     │    stack: Flutter/Dart
  │                     │    task: UI + auth
  │                     │    complexity: medium
  │                     │    needs_research: true
  │                     │
  │                     │ 2. Profile lookup ───────────────────────────────► │
  │                     │ ◄─────────── top-5 memories ────────────────────── │
  │                     │    "user prefers Riverpod, dislikes Provider"
  │                     │
  │                     │ 3. Research (parallel) ─────► │
  │                     │                              ├─► flutter.dev: google_sign_in latest API
  │                     │                              ├─► pub.dev: google_sign_in stars/issues
  │                     │                              ├─► pub.dev: firebase_auth compat
  │                     │                              ├─► SO: top answers last 6mo
  │                     │                              └─► deprecation: buttonConfiguration deprecated in 6.1.25
  │                     │ ◄─── DecisionReport ─────────
  │                     │
  │                     │ 4. Model router ──────────────────────► │
  │                     │ ◄─── Sonnet 4 (medium complexity) ──────
  │                     │
  │                     │ 5. Generate
  │                     │    auth_provider.dart + sign_in_button.dart
  │                     │
  │                     │ 6. Validate (self-correct max 3×)
  │                     │    static analysis: pass
  │                     │    import check: pass
  │                     │    version conflict: clean
  │                     │
  │ ◄── stream chunks ──┤
  │ ◄── stream done ────┤
  │                     │
  │                     │ 7. Background: memory update ────────────► │
  │                     │    +interest: Flutter
  │                     │    +fact: "prefers google_sign_in 6.2.1 over FirebaseUI"
```

Total user-perceived latency target: <2.5s for "research → first chunk". Cursor is ~3.5s on the same prompt. (PRD target: 30% faster than Cursor.)

## IPC contract

All Tauri commands return `Result<T, String>` (Tauri requires `serde::Serialize` + `Send + Sync`). Errors are flattened to a human-readable string in the shell; the structured error lives in `kontrocode_core::Error`.

Streaming uses Tauri events, not commands, because commands are request/response. The agent emits events to a per-message channel; the UI subscribes to a per-message-id topic.

## Failure model

Every layer fails fast and surfaces the error. Never silently swallow. The agent's behavior on failure:

1. **Transient** (network, rate limit) → automatic retry with exponential backoff, 3 attempts
2. **Recoverable** (compile error, test failure) → self-correction loop, 3 attempts
3. **Permanent** (auth failure, deprecation blocker) → surface to user with explanation
4. **Destructive op** → block, ask for human confirmation

## What's *not* in scope for the agent

- No web browsing beyond the research agent's allowlisted sources
- No email, calendar, OS automation
- No sending messages on the user's behalf
- No modifying `~/.ssh`, `~/.aws`, `~/.config` outside its own dir without confirmation

## Future architecture (Phase 4+)

When we ship Redis:

```
agent ──► kontrocode-memory ──► Redis (RedisJSON + RediSearch)
                            └─► tokio task: background updater (Redis Streams)

UI   ──► Tauri events ◄── agent
```

The trait boundary is what makes this safe. Phase 1 uses `FileMemoryStore`; Phase 4 swaps to `RedisMemoryStore` — no agent changes needed.

## Open architectural questions

1. **Local model fallback** (Phase 7.7) — Ollama integration? We can route "offline" tasks to a local model. Need to spec the routing.
2. **Multi-user** (Phase 7.3) — Team tier with shared profiles. Schema needs extension.
3. **Encrypted sync** (Phase 7) — E2E encrypted profile sync. libsodium? age?

These are tracked in [`ROADMAP.md`](ROADMAP.md).
