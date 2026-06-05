# KontroCode

> **The Agent That Knows Before It Codes.**

A native PC coding agent that researches official docs, package registries, and community signals **before** generating a single line of code. It builds a persistent memory of every developer it works with — learning stack, style, and preferences from implicit signals.

Faster than Cursor. Smarter than Copilot. **Never ships deprecated code.**

---

## What is KontroCode?

KontroCode is a research-first, memory-aware, native coding agent. It is not a chatbot.

It is a **fork of the Zed editor** (GPUI, Rust) with a custom **OpenCode-style agent brain** (also Rust) wired over the Agent Client Protocol — Zed's standard interface for talking to external agent servers. The result is a single Rust codebase that ships as a native desktop app on macOS, Windows, and Linux, with a sub-200 MB idle footprint.

It:

1. **Listens** to your prompt and detects intent, language, framework.
2. **Researches** the live ecosystem — official docs, package registries, GitHub signals, Stack Overflow recency — in parallel.
3. **Decides** which library, version, and pattern to use, with a confidence score.
4. **Generates** code using only current, non-deprecated APIs.
5. **Validates** imports, version conflicts, and patterns.
6. **Streams** the result into the editor.
7. **Learns** from your behavior in the background — building a profile you never have to fill in.

---

## Architecture at a glance

```
┌────────────────────────────────────────────────────────────┐
│  zed/  — Zed editor fork (GPUI, Rust)                      │
│  ┌─────────────┬──────────────────┬────────────────────┐    │
│  │ File tree   │  GPUI editor     │  Agent panel       │    │
│  │ + git (L)   │  KontroCode      │  (R, docked)       │    │
│  │             │  syntax theme    │  - chat            │    │
│  │             │                  │  - research feed   │    │
│  │             ├──────────────────┤  - memory panel    │    │
│  │             │  Zed terminal    │                    │    │
│  └─────────────┴──────────────────┴────────────────────┘    │
│                       │ Agent Client Protocol (stdin/stdout)│
└───────────────────────┼────────────────────────────────────┘
                        ▼
┌────────────────────────────────────────────────────────────┐
│  crates/kontrocode-agent    — OpenCode-style agent loop     │
│  crates/kontrocode-research — docs/npm/registry scrapers   │
│  crates/kontrocode-router   — multi-provider LLM routing   │
│  crates/kontrocode-memory   — profile + RAG                 │
│  crates/kontrocode-core     — shared types, config, errors  │
└────────────────────────────────────────────────────────────┘
                        │
                        ▼
        ┌───────────────────────────────┐
        │  9 LLM providers              │
        │  Anthropic, Groq, xAI,        │
        │  Google, OpenAI, DeepSeek,    │
        │  Mistral, Together, Fireworks │
        └───────────────────────────────┘
```

The Agent Client Protocol is what `crates/agent_servers` already speaks in upstream Zed. We register our `kontrocode-agent` binary as the default server, so when the user opens the agent panel, Zed spawns our process and streams JSON-RPC over its stdio (which on Linux/macOS is a Unix socket pair from `socketpair`, and on Windows is a named pipe via ConPTY — the **IPC** described in PRD §2.2).

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full breakdown.

---

## Status

**Phase 1** of 8 — *Zed fork with KontroCode theme + OpenCode-style agent wired via Agent Client Protocol.*

| Phase | Deliverable                                         | Status |
|-------|-----------------------------------------------------|--------|
| 1     | Zed fork + KontroCode theme + agent + IPC          | **In progress** |
| 2     | Multi-provider backend (9 providers, routing)       | Planned |
| 3     | Research agent (docs, package ranker, deprecation)  | Planned |
| 4     | Memory system (Redis profile, RAG, background)      | Planned |
| 5     | Decision engine + code validator                    | Planned |
| 6     | UI polish (research feed, memory panel)             | Planned |
| 7     | Billing + accounts + Tauri v2 packaging             | Planned |
| 8     | Benchmarks vs Cursor                                | Planned |

See [`docs/ROADMAP.md`](docs/ROADMAP.md).

---

## Quick start

### Build the Zed-fork editor (Phase 1)

```bash
git clone https://github.com/fahimuntasin/kontrocode.git
cd kontrocode
cd zed
cargo run --release --bin kontrocode
```

The first build compiles ~230 Rust crates. Expect 30–60 minutes on a modern machine.

### Build the agentic backend

```bash
# In a separate terminal, from repo root:
cargo build --release --workspace
./target/release/kontrocode-agent --help
```

### Run with the editor pointing at the local agent

```bash
cd zed
KONTROCODE_AGENT_BIN=$(pwd)/../target/release/kontrocode-agent cargo run --bin kontrocode
```

When you open the KontroCode Agent panel (Cmd+? or via the command palette), Zed will spawn our agent process. Our agent's CLI speaks the Agent Client Protocol and uses our own `kontrocode-research`, `kontrocode-router`, and `kontrocode-memory` crates for the intelligence layer.

---

## Repository layout

```
kontrocode/
├── zed/                         Zed editor fork (GPUI, Rust)
│   ├── crates/                  230+ Zed crates (editor, LSP, terminal, …)
│   ├── assets/themes/kontrocode/  KontroCode dark theme
│   └── Cargo.toml               Zed workspace
├── crates/                      KontroCode agentic backend
│   ├── kontrocode-core/         Shared types, config, errors
│   ├── kontrocode-agent/        OpenCode-style agent loop + ACP server
│   ├── kontrocode-research/     Docs / package scrapers
│   ├── kontrocode-router/       Multi-provider LLM routing
│   └── kontrocode-memory/       Profile + RAG store
├── docs/
│   ├── design.md                Visual + UX design spec
│   ├── ARCHITECTURE.md          System architecture
│   └── ROADMAP.md               Build phases
├── Cargo.toml                   KontroCode-only workspace
└── README.md
```

The two Cargo workspaces (root + `zed/`) are intentionally separate. Zed is shipped as a contiguous fork and evolves at its own cadence; our `kontrocode-*` crates are versioned independently.

---

## License

MIT for the KontroCode-authored code under `crates/` and `docs/`. The `zed/` subtree remains under the upstream Apache 2.0 + GPL 3.0 dual license — see [`zed/LICENSE-APACHE`](zed/LICENSE-APACHE) and [`zed/LICENSE-GPL`](zed/LICENSE-GPL).
