# KontroCode — System Architecture

> **The Agent That Knows Before It Codes.**

This document describes how KontroCode is built. It is the technical counterpart to `docs/ROADMAP.md` (which describes **what** is built in each phase) and the PRD (which describes **why**).

---

## 1. Top-level shape

KontroCode is a **single Rust codebase** that ships as a native desktop editor on macOS, Windows, and Linux. There is no Electron, no Tauri shell, no webview. The only runtime dependencies are the system GPU drivers and a handful of platform libraries (WebKit2GTK on Linux, WebView2 on Windows, WKWebView on macOS — all consumed by Zed's GPUI via `wry`).

```
┌─────────────────────────────────────────────────────────────────────┐
│  zed/  — Zed editor fork (GPUI / Rust)                             │
│  ┌──────────┬──────────────────────┬─────────────────────────────┐  │
│  │ File tree│  GPUI editor         │  Agent panel (docked right) │  │
│  │ + git    │  KontroCode syntax   │  - chat                     │  │
│  │ (left)   │  theme               │  - research feed            │  │
│  │          │                      │  - memory panel             │  │
│  │          ├──────────────────────┤                             │  │
│  │          │  Zed terminal        │                             │  │
│  └──────────┴──────────────────────┴─────────────────────────────┘  │
│                          │ Agent Client Protocol (ACP)             │
│                          │ (stdin/stdout = socketpair / ConPTY)    │
└──────────────────────────┼──────────────────────────────────────────┘
                           ▼
┌─────────────────────────────────────────────────────────────────────┐
│  crates/  — KontroCode agentic backend (Rust)                       │
│                                                                     │
│  kontrocode-agent      Agent loop, tool use, ACP server             │
│  kontrocode-research   Docs / package / deprecation scrapers        │
│  kontrocode-router     Multi-provider LLM routing (9 providers)     │
│  kontrocode-memory     Profile store, RAG, decay, signal collection  │
│  kontrocode-core       Shared types, error model, config            │
└─────────────────────────────────────────────────────────────────────┘
                           │
                           ▼
                  9 LLM providers
```

---

## 2. The two forks

PRD §2.1 specifies the architecture as **two forks, zero reinvention of fundamentals**.

### 2.1 Zed fork — the editor

We ship a fork of [zed-industries/zed](https://github.com/zed-industries/zed) at `zed/`. We **keep** upstream:

- The editor engine, buffer model, multi-cursor, code folding
- LSP integration (50+ languages)
- Git integration
- The native terminal emulator
- The syntax highlighter and tree-sitter grammars
- The command palette, breadcrumb navigation, file tree

We **replace**:

- The default theme (we ship `assets/themes/kontrocode/kontrocode.json` — see PRD §3.3)
- The default `One Dark` is swapped to `KontroCode Dark` (`crates/theme/src/theme.rs` → `DEFAULT_DARK_THEME`)
- The application name is set to `KontroCode` (`crates/paths/src/paths.rs` → `APP_NAME`)
- The agent server registration — Zed's `agent_servers` registry is pointed at our `kontrocode-agent` binary by default

We **add**:

- The KontroCode visual identity (deep navy `#0D0D1A`, electric blue `#3A3AFF`, electric mint `#00FFB2` — PRD §3.3)
- A memory-panel section in the right agent panel (under the chat tab)
- A live research-feed accordion (under the chat tab)

### 2.2 OpenCode-style agent fork — the brain

[anomalyco/opencode](https://github.com/anomalyco/opencode) is written in TypeScript, but the PRD specifies the agent in Rust. We follow the **OpenCode agentic patterns** (multi-step planning, tool use, diff-first edits, self-correction loop, parallel sub-agents) in our `crates/kontrocode-agent`. We do not ship OpenCode as a dependency; we reimplement the patterns in pure Rust.

What we **keep** from OpenCode's design:

- Multi-step task planning — request → subtask graph → ordered execution
- File system tool — read, write, diff-first edits
- Shell execution tool — with destructive-command blocklist
- Multi-file coordinated generation
- Self-correction loop — 3 retries max on compile/test errors
- Parallel sub-agents — `tokio::join!` fan-out

What we **add** per the PRD:

- Research tools — official-docs, pub.dev/npm/crates.io, GitHub, Stack Overflow
- Memory injection — top-K facts from `kontrocode-memory` injected into every request
- Multi-provider routing — see `crates/kontrocode-router` and PRD §6
- Profile system — see `crates/kontrocode-memory` and PRD §4

The full 12-rule system prompt from PRD §11 is the base prompt for the agent.

### 2.3 Custom KontroCode layer — the glue

The "Custom KontroCode layer sits between the two — orchestrates everything" from PRD §2.1 is implemented as the **Agent Client Protocol server** inside `kontrocode-agent`. ACP is JSON-RPC over stdio (Zed spawns our process and speaks ACP). On Linux/macOS this is a `socketpair` (Unix socket); on Windows it is a ConPTY (named pipe). The PRD's "Unix socket / named pipe (Windows)" maps exactly to ACP's transport.

---

## 3. Request lifecycle

PRD §2.3 specifies a 9-step pipeline. Here is how each step is implemented:

| # | Step             | Where it lives                              | Status     |
|---|------------------|---------------------------------------------|------------|
| 1 | Intent analyzer  | `kontrocode-core::intent`                   | Phase 1 ✓  |
| 2 | Profile lookup   | `kontrocode-memory::FileMemoryStore`        | Phase 4 (file now, Redis Phase 4) |
| 3 | Research agent   | `kontrocode-research::ResearchRunner`       | Phase 3 (interfaces now, real fetchers Phase 3) |
| 4 | Decision engine  | `kontrocode-router::scorer`                 | Phase 5    |
| 5 | Model router     | `kontrocode-router::Router`                 | Phase 2    |
| 6 | Code generator   | `kontrocode-agent::loop_`                   | Phase 1 (mock provider, real Phase 2) |
| 7 | Validator        | `kontrocode-agent::tools::static_check`     | Phase 5    |
| 8 | Stream to editor | ACP over stdio/socketpair                   | Phase 1 ✓  |
| 9 | Background       | `kontrocode-memory::signal::extract_facts`  | Phase 4    |

Step 8 is the only one that touches the editor. Everything else runs in the agent process, in Tokio. The agent emits JSON-RPC notifications on every step and the Zed panel renders them.

---

## 4. IPC — Agent Client Protocol

PRD §2.2: *"IPC: Zed ↔ OpenCode via local Unix socket / named pipe (Windows)"*.

This is exactly what the [Agent Client Protocol](https://github.com/zed-industries/agent-client-protocol) (ACP, crate `agent-client-protocol = "0.13.1"`) provides. Zed's `crates/agent_servers` already implements the client side. Our `kontrocode-agent` implements the server side.

Transport:

- **Linux / macOS**: the OS gives the parent a Unix socket pair (`socketpair(AF_UNIX, SOCK_STREAM, 0)`) when `Command::new(...).stdin(Stdio::piped()).stdout(Stdio::piped())` is invoked. From the kernel's perspective this **is** a Unix domain socket.
- **Windows**: `tokio::process::Command` uses ConPTY, which is a named-pipe-based pseudoconsole. From the user's perspective this **is** a named pipe.

So the PRD's wording is satisfied by ACP's existing transport; we do not need to write a custom IPC layer. We reuse upstream Zed's client (in `crates/agent_servers`) and only write the server (in `crates/kontrocode-agent`).

The 8 IPC messages we use today (Phase 1):

| Direction       | Method                  | Meaning                                  |
|-----------------|-------------------------|------------------------------------------|
| Zed → Agent     | `initialize`            | Handshake + capability negotiation      |
| Zed → Agent     | `authenticate`          | User provides API keys                   |
| Zed → Agent     | `session/new`           | Start a new conversation                 |
| Agent → Zed     | `session/update`        | Streaming token chunk                    |
| Agent → Zed     | `tool_call`             | Request approval (destructive op)        |
| Zed → Agent     | `tool_call/response`    | Approve / deny                           |
| Agent → Zed     | `message/complete`      | End of turn                              |
| Either          | `cancel`                | Abort in-flight turn                     |

PRD §2.2 also lists Tauri v2 (P1) for packaging. Tauri is reserved for the **installer and auto-updater** (Phase 7), not the UI.

---

## 5. Trait boundaries (the parts that will not change)

These four traits are the public surface of the agentic backend. They are stable from Phase 1 onward; everything else can be swapped without touching the editor.

```rust
#[async_trait]
pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
    async fn stream(&self, req: CompletionRequest) -> Result<StreamReceiver>;
    fn cost_per_token(&self) -> Cost;
}

#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn load(&self) -> Result<Profile>;
    async fn save(&self, profile: &Profile) -> Result<()>;
    async fn add_fact(&self, fact: Fact) -> Result<()>;
    async fn search(&self, query: &str, top_k: usize) -> Result<Vec<Fact>>;
}

#[async_trait]
pub trait ResearchSource: Send + Sync {
    fn id(&self) -> &str;
    fn supports(&self, stack: Stack) -> bool;
    async fn fetch(&self, query: &ResearchQuery) -> Result<Vec<ResearchCandidate>>;
}

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn definition(&self) -> ToolDefinition;
    async fn execute(&self, args: serde_json::Value) -> Result<ToolResult>;
}
```

Phase 1 ships `MockProvider`, `FileMemoryStore`, `NullSource`, and three tools (`file_read`, `file_write`, `shell_run`). Phase 2+ swaps `MockProvider` for real Anthropic / Groq / xAI / Google / OpenAI / DeepSeek / Mistral / Together / Fireworks implementations behind the same trait; Phase 4 swaps `FileMemoryStore` for Redis; Phase 3 adds real research sources.

The ACP agent binary (`kontrocode-agent acp`) is implemented in `crates/kontrocode-agent/src/acp.rs` and tested against the JSON-RPC wire format. It accepts `initialize`, `session/new`, and can be registered as a custom agent server in the Zed fork via settings.

The editor never sees any of this. It only sees JSON-RPC over ACP.

---

## 6. Why a Zed fork and not a custom editor

The PRD says: *"GPUI GPU-accelerated rendering (native speed)"*, *"Syntax highlight for 50+ languages"*, *"LSP integration"*, *"Git integration"*, *"Terminal emulator (multi-pane)"*, *"Split editor panes"*, *"Command palette (Cmd+K)"*, *"Multi-cursor editing"*, *"Code folding"*, *"Breadcrumb navigation"*. That is the existing Zed editor. Writing this from scratch is 50+ engineer-months of work and has nothing to do with what makes KontroCode special (which is the agent brain, not the text-rendering pipeline).

GPUI is published on crates.io (`gpui = "0.2"`) and its rendering primitives (text layout, soft-buffer, GPU-accelerated shape rendering) are public. But the editor, LSP, terminal, and git layers above GPUI are not extractable into a thin library without a year of refactoring. Forking the whole repo is the only way to keep "Zed-level density, not IDE-bloat" (PRD §3.1) without writing 200k lines of editor code.

The fork strategy is therefore: keep the editor, replace the brain, apply our theme, and stay close enough to upstream to track their releases.

---

## 7. Two Cargo workspaces, on purpose

The repo has two Cargo workspaces:

- Root `Cargo.toml` — our `kontrocode-*` crates (the agentic backend)
- `zed/Cargo.toml` — Zed's ~230 crates (the editor)

The two are **not** merged. Reasons:

1. Zed's `Cargo.lock` is huge and changes weekly. Our crates are stable.
2. `cargo build --release` on the root should be fast (Phase 2 has 9 providers + 1 agent = ~5 min cold, ~10 s warm). Pulling in all of Zed's crates would inflate every CI run.
3. We can bump Zed's `gpui` version without forcing a rebuild of the agent.
4. We can swap the editor for a different GPUI-based fork (e.g. [IBM's lattice](https://github.com/IBM/lattice) if it stabilises) without touching the agent.

The two crates that need to talk to each other (`kontrocode-agent` and `acp_thread`) only do so through the JSON-RPC boundary — they are not Rust dependencies of each other.

---

## 8. What Phase 1 ships

Phase 1 (Week 1–2 in the PRD, 6–8 weeks solo in practice — see `docs/ROADMAP.md` for the realistic estimate) delivers:

1. `zed/` checkout with `kontrocode.json` theme registered
2. `crates/paths` returning `KontroCode` (data dir, config dir, log dir)
3. `kontrocode-agent` binary that speaks ACP and runs the agent loop with `MockProvider` + `NullSource` + `FileMemoryStore`
4. The 6 OpenCode-style rules from PRD §11 already in `kontrocode-agent::prompt`
5. 72 unit tests across the 5 kontrocode crates passing
6. `cargo run --bin kontrocode` opens the editor with the KontroCode dark theme by default

What Phase 1 does **not** ship (and what you should not expect from a `git clone` today):

- Real LLM providers (Phase 2)
- Real research fetchers (Phase 3)
- Redis-backed memory (Phase 4)
- The Decision Engine scoring weights (Phase 5)
- Tauri v2 packaging / auto-update (Phase 7)
- Multi-crate (FreeBSD), snap/flatpak, signing (Phase 7)

The realistic MVP that exercises **all** PRD features is Phase 5 (Week 9–10).
