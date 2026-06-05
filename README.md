# KontroCode

> **The Agent That Knows Before It Codes.**

A native PC coding agent that researches official docs, package registries, and community signals **before** generating a single line of code. It builds a persistent memory of every developer it works with — learning stack, style, and preferences from implicit signals.

Faster than Cursor. Smarter than Copilot. **Never ships deprecated code.**

---

## What is KontroCode?

KontroCode is a research-first, memory-aware, native coding agent. It is not a chatbot.

It is an autonomous agent loop that:

1. **Listens** to your prompt and detects intent, language, framework.
2. **Researches** the live ecosystem — official docs, package registries (pub.dev / npm / crates.io), GitHub signals, Stack Overflow recency — in parallel.
3. **Decides** which library, version, and pattern to use, with a confidence score.
4. **Generates** code using only current, non-deprecated APIs.
5. **Validates** imports, version conflicts, and patterns.
6. **Streams** the result into the editor.
7. **Learns** from your behavior in the background — building a profile you never have to fill in.

It runs as a **native desktop app** (Tauri v2 + Rust + TypeScript). No Electron. No terminal-only trade-off.

---

## Architecture at a glance

```
┌────────────────────────────────────────────────────────────┐
│  apps/desktop  (Tauri v2 — native shell)                   │
│  ┌─────────────┬──────────────────┬────────────────────┐    │
│  │ File tree   │  Monaco editor   │  Agent panel       │    │
│  │ (left)      │  (center)        │  (right)           │    │
│  │             │                  │  - chat            │    │
│  │             │                  │  - research feed   │    │
│  │             │                  │  - memory panel    │    │
│  │             ├──────────────────┤                    │    │
│  │             │  Terminal (xterm)│                    │    │
│  └─────────────┴──────────────────┴────────────────────┘    │
│                       │ Tauri IPC                          │
└───────────────────────┼────────────────────────────────────┘
                        ▼
┌────────────────────────────────────────────────────────────┐
│  crates/kontrocode-agent   — OpenCode-style agent loop     │
│  crates/kontrocode-research — docs/npm/registry scrapers   │
│  crates/kontrocode-router  — multi-provider LLM routing   │
│  crates/kontrocode-memory  — profile + RAG                 │
│  crates/kontrocode-core    — shared types, config, errors  │
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

See [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the full breakdown.

---

## Status

**Phase 1** of 8 — *Zed-equivalent native shell with KontroCode theme + OpenCode-style agent wired via IPC.*

| Phase | Deliverable                                         | Status |
|-------|-----------------------------------------------------|--------|
| 1     | Native shell + theme + agent + IPC                  | **In progress** |
| 2     | Multi-provider backend (9 providers, routing)       | Planned |
| 3     | Research agent (docs, package ranker, deprecation)  | Planned |
| 4     | Memory system (Redis profile, RAG, background)      | Planned |
| 5     | Decision engine + code validator                    | Planned |
| 6     | UI polish (research feed, memory panel)             | Planned |
| 7     | Billing + accounts + auto-updater                   | Planned |
| 8     | Benchmarks vs Cursor                                | Planned |

See [`docs/ROADMAP.md`](docs/ROADMAP.md).

---

## Quick start (Phase 1 dev)

```bash
# Prereqs: Rust 1.95+, Node 20+, pnpm 9+, Tauri v2 deps
git clone https://github.com/fahimuntasin/kontrocode.git
cd kontrocode
pnpm install
pnpm --filter @kontrocode/desktop tauri dev
```

The first build is slow (Rust compilation). Subsequent runs are fast.

### CLI (headless agent)

```bash
# After publish:
npx @kontrocode/cli "build me a Flutter auth screen with Google Sign-In"
```

---

## Repository layout

```
kontrocode/
├── apps/
│   └── desktop/              Tauri v2 native shell
│       ├── src/              TypeScript + Solid frontend
│       └── src-tauri/        Rust shell
├── crates/                   Rust workspace
│   ├── kontrocode-core/      Shared types, config, errors
│   ├── kontrocode-agent/     OpenCode-style agent loop
│   ├── kontrocode-research/  Docs / package scrapers
│   ├── kontrocode-router/    Multi-provider LLM routing
│   └── kontrocode-memory/    Profile + RAG store
├── packages/
│   └── cli/                  @kontrocode/cli (Node headless)
├── docs/
│   ├── design.md             Visual + UX design spec
│   ├── ARCHITECTURE.md       System architecture
│   └── ROADMAP.md            Build phases
├── Cargo.toml                Rust workspace
├── pnpm-workspace.yaml       Node workspace
└── package.json              Root scripts
```

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). All PRs go through CI (cargo test, pnpm test, pnpm lint).

## Security

See [`SECURITY.md`](SECURITY.md). Report vulnerabilities to **security@kontrocode.dev** (or open a private advisory on GitHub).

## License

MIT — see [`LICENSE`](LICENSE).

---

Built by the KontroCode team. *Knows before it codes.*
