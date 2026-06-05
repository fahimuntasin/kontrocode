# KontroCode — Build Roadmap

> **Status:** Phase 1 in progress. Realistic MVP is Phase 5 (Week 9–10 of the PRD's 16-week plan).

This file replaces the original Tauri-based plan. We are now building a **Zed fork (GPUI, Rust) + OpenCode-style agent (Rust) wired via Agent Client Protocol** — exactly as PRD §2.1 specifies. See [`docs/ARCHITECTURE.md`](ARCHITECTURE.md) for the technical breakdown.

---

## Phase 1 — Zed fork + KontroCode theme + agent via ACP  ·  *current*

**Goal**: `cargo run --bin kontrocode` opens the Zed editor with the KontroCode dark theme by default, and the right-side agent panel connects to our `kontrocode-agent` binary over the Agent Client Protocol. All 5 backend crates compile, pass tests, and the agent runs an end-to-end mock request → response loop.

| # | Deliverable                                                                          | Status   |
|---|---------------------------------------------------------------------------------------|----------|
| 1 | `zed/` cloned at upstream HEAD                                                       | ✓ done   |
| 2 | `assets/themes/kontrocode/kontrocode.json` shipped (141 style keys, PRD §3.3 colors)  | ✓ done   |
| 3 | `crates/theme/src/theme.rs::DEFAULT_DARK_THEME = "KontroCode Dark"`                   | ✓ done   |
| 4 | `crates/paths::APP_NAME = "KontroCode"` (data/config dirs rebased)                    | ✓ done   |
| 5 | `crates/zed/Cargo.toml::name = "kontrocode"` (binary rename)                          | ✓ done   |
| 6 | `kontrocode-agent` binary that speaks ACP and runs the agent loop                     | 🔄 wip   |
| 7 | `kontrocode-agent` registered as default ACP server in `agent_servers`               | 🔄 wip   |
| 8 | 72 unit tests across the 5 backend crates passing                                     | ✓ done   |
| 9 | First `cargo run --bin kontrocode` opens a window                                     | 🔄 wip   |
| 10 | Phase 1 system prompt (6 of 12 PRD §11 rules) wired into the agent                    | ✓ done   |
| 11 | Docs: `ARCHITECTURE.md`, `ROADMAP.md`, `README.md` match the new direction           | 🔄 wip   |
| 12 | CI: GitHub Actions builds the editor and runs the test suite                          | 📋 todo  |

**What works today (without the editor)**:

- `cargo test --workspace` at the repo root: 72 / 72 tests pass
- `kontrocode-agent` is a runnable binary that accepts CLI invocations like `kontrocode-agent ask "build me a Flutter auth screen"`
- Mock provider streams text chunks; FileMemoryStore persists a profile; NullSource returns no research; the router has a 300 ms fallback SLA
- The 6 OpenCode-style rules from PRD §11 govern all behaviour

**What's blocking the editor opening**:

- The first `cargo check --bin kontrocode` is downloading Rust 1.95.0 toolchain components (~5 min on a clean machine) and will then compile ~230 Zed crates from scratch (~30 min on a modern machine)

---

## Phase 2 — Multi-provider backend  ·  *week 3–4*

| # | Deliverable                                                                            |
|---|----------------------------------------------------------------------------------------|
| 1 | 9 provider implementations behind the `Provider` trait                                  |
| 2 | `Router` with the 6 routing categories from PRD §6.2 (research, decision, code-simple, code-complex, validation, fallback) |
| 3 | Auto-fallback chain with 300 ms SLA                                                     |
| 4 | Cost estimator + budget cap                                                             |
| 5 | BYOK key handling (env vars + settings file)                                           |
| 6 | Provider health monitor (rolling 1-min p95 latency)                                    |

Deliverable: every model category in PRD §6.2 has a working provider. The `MockProvider` is removed from the default registry.

---

## Phase 3 — Research agent  ·  *week 5–6*

| # | Deliverable                                                                            |
|---|----------------------------------------------------------------------------------------|
| 1 | `OfficialDocsSource` — flutter.dev, docs.rs, nodejs.org, etc. (stack-matched)           |
| 2 | `PubDevSource`, `NpmSource`, `CratesIoSource` — stars, recency, weekly downloads        |
| 3 | `DeprecationDetector` — changelog + migration-guide parser                             |
| 4 | `StackOverflowSource` — recency + score filter (last 12 mo, score > 50)               |
| 5 | `GitHubSource` — stars, last commit, issue count                                       |
| 6 | `VersionConflictResolver` — full dep-graph scan                                        |
| 7 | `ResearchCache` — Redis 24h TTL (or in-memory fallback for dev)                        |
| 8 | Research feed live UI in the agent panel                                               |

Deliverable: a real Flutter / Node / Rust request triggers ≥3 real research sources in parallel via `tokio::join!` and the results land in the agent panel within ~200 ms.

---

## Phase 4 — Memory system  ·  *week 7–8*

| # | Deliverable                                                                            |
|---|----------------------------------------------------------------------------------------|
| 1 | `RedisMemoryStore` (RedisJSON + RediSearch + Streams)                                  |
| 2 | `Embedding` trait + OpenAI text-embedding-3-small + local Nomic fallback               |
| 3 | RAG retrieval (top-5 cosine similarity, ~300 tokens max)                              |
| 4 | Cold start: usable profile in 5 messages, full profile in ~3 sessions                  |
| 5 | Background async fact extraction via Redis Streams worker                              |
| 6 | Decay (× 0.98/day)                                                                     |
| 7 | Contradiction resolver (timestamp + confidence arbitration)                           |
| 8 | Memory panel UI (view / edit / delete facts)                                           |

Deliverable: a session's facts land in Redis, decay over days of inactivity, and are auto-injected into the next request's system prompt.

---

## Phase 5 — Decision engine + validator  ·  *week 9–10*

| # | Deliverable                                                                            |
|---|----------------------------------------------------------------------------------------|
| 1 | Decision engine: `score = stars*0.3 + recency*0.3 + official*0.25 + so*0.15`            |
| 2 | Confidence threshold (≥ 0.7 to auto-pick, else surface alternatives)                   |
| 3 | `Validator` — static analysis pass (import check, version conflict scan)                |
| 4 | Self-correction loop (3 retries, silent unless all 3 fail — PRD §11 rule 5)             |
| 5 | Benchmark harness — 50 prompts × 5 stacks, measure first-compile rate, deprecated-API rate, P95 latency, token cost |

**Realistic MVP end** — at this point all 154 PRD features have at least a stub.

---

## Phase 6 — UI polish  ·  *week 11–12*

- Research feed live accordion (PRD §3.2 — currently a tab in the agent panel)
- Memory panel docked under chat
- KontroCode dark theme final pass (iconography, line numbers, breadcrumb colours)
- Slash-command inside chat (`/research`, `/memory`, `/route`)
- Inline ghost-text completions (Zed already has this; we tune the prompt)
- Multi-language response (Banglish / EN / BN — uses the profile's `language` preference)

---

## Phase 7 — Billing + accounts + packaging  ·  *week 13–14*

- Stripe billing integration
- Free / Pro / Team / BYOK tiers
- Usage dashboard (per model, per day)
- Tauri v2 wrapper for installer + auto-updater (PRD §2.2 — packaging only, not UI)
- Code signing on macOS (notarytool) and Windows (Azure Trusted Signing)

---

## Phase 8 — Benchmarks  ·  *week 15–16*

Targets from PRD §10:

| Metric                       | Target       |
|------------------------------|--------------|
| Agent response P95           | < Cursor by 30 % |
| First-compile rate           | > 92 %       |
| Deprecated API rate          | 0 %          |
| Token cost per task          | 5–10× lower than Cursor |
| Memory accuracy              | > 80 %       |
| Research cache hit rate      | > 65 %       |
| Provider uptime SLA          | 99.9 %       |
| Cold start UX                | Useful in 5 messages |
| Binary size                  | < 50 MB installer |
| Memory footprint             | < 200 MB idle RAM |

Run the benchmark suite weekly. Block releases on regressions.
