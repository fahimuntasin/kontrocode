# Roadmap

> Where we're going. 16 weeks, 8 phases.

> **Note on the UI stack.** Phase 1 ships a Tauri v2 + Solid.js + Monaco
> frontend, not a Zed + GPUI fork. The trade-off is fully documented
> in [`ARCHITECTURE.md` § "UI layer — the Zed-fork trade-off"](ARCHITECTURE.md#ui-layer--the-zed-fork-trade-off).
> Read it before proposing a frontend rewrite.

## Principles

- **Ship every 2 weeks.** No "big bang" releases. Each phase ends with something a user can run.
- **Phase 1 must be useful.** Even if it's "just an editor with an offline agent that streams", it has to feel good.
- **No rewrites between phases.** The trait boundaries (`MemoryStore`, `Provider`, `ResearchSource`) are decided in Phase 1 and don't change.

## Phase overview

| Phase | Weeks | Deliverable | Exit criteria |
|-------|-------|-------------|---------------|
| 1 | 1–2  | Native shell + theme + OpenCode-style agent + IPC working end-to-end | User can run `pnpm tauri dev`, open a folder, send a prompt, see a streaming response, and have it edit a file. |
| 2 | 3–4  | Multi-provider backend (9 providers, routing, fallback) | Switch providers at runtime. Cost tracking visible in status bar. Fallback chain within 300ms. |
| 3 | 5–6  | Research agent (docs, package ranker, deprecation detector) | Research feed shows live lookups. `pub.dev` / `npm` / `docs.*` fetches parallel. Deprecation blocked in generated code. |
| 4 | 7–8  | Memory system (profile, RAG, background updater) | Cold start: usable in 5 messages. Memory panel viewable. Profile injected into every request. |
| 5 | 9–10 | Decision engine + code validator | Library decision confidence shown. Static analysis + import + version check integrated. Self-correction loop working. |
| 6 | 11–12 | UI polish (research feed, memory panel, theme final) | All 154 features spec'd. Visual identity locked. |
| 7 | 13–14 | Billing + accounts + auto-updater + packaging | Stripe tiers. Auto-updater via Tauri v2. <50MB installer. |
| 8 | 15–16 | Benchmarks vs Cursor | Public benchmark suite. P95 latency, first-compile rate, cost, deprecation rate published. |

---

## Phase 1 — Foundation (current)

**Goal:** A native shell, in the KontroCode visual identity, with an OpenCode-style agent wired up via IPC. The agent runs in-process, can use tools (file_read, file_write, shell_run), and streams output to the UI. No real LLM yet — a mock provider that echoes. No real research. No real memory. The wiring is the deliverable.

### Tasks

- [x] GitHub repo + monorepo skeleton
- [x] Core docs (README, design, ARCHITECTURE, this file, CONTRIBUTING, SECURITY, LICENSE)
- [ ] `kontrocode-core` crate — types, config, errors, tracing
- [ ] `kontrocode-router` crate — `Provider` trait + mock provider
- [ ] `kontrocode-agent` crate — agent loop, tools, streaming
- [ ] `kontrocode-memory` crate — `MemoryStore` trait + file-backed impl
- [ ] `kontrocode-research` crate — `ResearchSource` trait + stub impl
- [ ] `apps/desktop` Tauri shell + Solid frontend
- [ ] KontroCode theme tokens in CSS
- [ ] Monaco editor with KontroCode theme
- [ ] File tree + git status
- [ ] Agent panel (chat, research feed, memory — UI only)
- [ ] Terminal (xterm.js)
- [ ] Status bar
- [ ] Tauri IPC: commands + events
- [ ] `@kontrocode/cli` for headless use
- [ ] CI: cargo test, pnpm test, pnpm lint, pnpm typecheck

### Out of scope for Phase 1

- Real LLM providers (we ship a mock that simulates streaming)
- Real research fetches (we ship a stub returning empty)
- Real memory persistence (we ship a JSON file)
- Diff/accept UI for AI edits (we ship raw output)
- Multi-window, command palette, keyboard customization
- Auto-updater
- Onboarding flow
- Session history

These all come in later phases, but the trait boundaries are decided now so we don't paint ourselves into a corner.

### Definition of done

A new contributor can:

1. Clone the repo
2. Run `pnpm install && pnpm --filter @kontrocode/desktop tauri dev`
3. See the KontroCode-themed window
4. Open a folder
5. Type *"add a README.md with a hello world"* in the agent panel
6. See the agent stream a response and write the file
7. Open the file in the editor and see it highlighted in the KontroCode theme

If they can do all of that in under 5 minutes, Phase 1 is done.

---

## Phase 2 — Multi-provider backend

**Goal:** Real LLM providers, routed by complexity, with cost tracking and fallback.

- [ ] Anthropic provider (Claude Haiku 3.5, Sonnet 4, Opus 4.5)
- [ ] Groq provider (Llama 3.3 70B, Mixtral 8x22B)
- [ ] xAI provider (Grok 3, Grok 3 Mini)
- [ ] Google provider (Gemini 2.5 Flash, Gemini 2.5 Pro)
- [ ] OpenAI provider (GPT-4o, GPT-4o Mini, o3-mini)
- [ ] DeepSeek provider (V3, R1)
- [ ] Mistral provider (Mistral Large 2, Codestral)
- [ ] Together AI provider
- [ ] Fireworks AI provider
- [ ] Task complexity scorer
- [ ] Cost / speed / quality optimizer modes
- [ ] Auto fallback chain (300ms SLA)
- [ ] Per-provider rate limit + retry
- [ ] Real-time cost tracker
- [ ] Monthly budget cap + alerts
- [ ] BYOK (keychain storage)
- [ ] Cost estimate before heavy tasks
- [ ] Provider status dashboard
- [ ] Usage export (CSV)

### Definition of done

A user can configure any of the 9 providers, route by complexity, see live cost in the status bar, and a provider failure does not break the agent.

---

## Phase 3 — Research agent

**Goal:** Real-time ecosystem awareness. Never ship deprecated code.

- [ ] Official docs scraper (flutter.dev, docs.rs, nodejs.org, etc.)
- [ ] pub.dev ranker
- [ ] npm ranker
- [ ] crates.io ranker
- [ ] GitHub signal collector
- [ ] Stack Overflow parser (last 12 months, score > 50)
- [ ] Deprecation detector (changelog + migration guide parser)
- [ ] Version conflict resolver
- [ ] Research cache (24h TTL)
- [ ] Parallel execution (`tokio::join!`)
- [ ] Research feed live UI
- [ ] Decision engine (scoring + ranking)
- [ ] Confidence score output
- [ ] Research report exportable per session

### Definition of done

Prompt *"use a state management library for Flutter"* returns a scored comparison: Riverpod 2.5.1 (98/100), with sources, recency, and a confidence score — all visible in the research feed.

---

## Phase 4 — Memory system

**Goal:** Persistent developer profile. The agent knows you.

- [ ] Redis: RedisJSON + RediSearch + Streams
- [ ] PostgreSQL: accounts, billing, audit
- [ ] Profile data model (PRD §4.2)
- [ ] Interest graph with exponential decay
- [ ] Fact store with confidence scoring
- [ ] Contradiction resolver
- [ ] Cold start handler (5 messages)
- [ ] RAG injection (top-K cosine, ~300 tokens)
- [ ] Background async update
- [ ] Memory panel UI (view/edit/delete)
- [ ] Profile export (JSON)
- [ ] GDPR delete (full wipe)

### Definition of done

A new user is useful in 5 messages and fully profiled in ~3 sessions. Their stated preferences and detected patterns are visible in the memory panel and honored by the agent.

---

## Phase 5 — Decision engine + code validator

**Goal:** Smarter code, fewer compile errors, self-correction.

- [ ] Stack auto-detector
- [ ] Library auto-selector (research-informed)
- [ ] Official pattern enforcer
- [ ] Deprecated API blocker
- [ ] Multi-file coordinated generation
- [ ] Diff-first targeted edits
- [ ] Self-correction loop (3 attempts, silent)
- [ ] Test generation alongside implementation
- [ ] Inline doc generation
- [ ] Import auto-resolution
- [ ] Version-pinned output
- [ ] Context-window-aware chunking

### Definition of done

> 92% of generated code compiles without manual edit. 0% deprecation rate (PRD target).

---

## Phase 6 — UI polish

**Goal:** Lock the visual identity. Ship all 154 features spec'd.

- [ ] Research feed live accordion
- [ ] Memory panel view/edit/delete
- [ ] Command palette (Cmd+K) — every action accessible
- [ ] Diff view for AI edits (accept/reject per hunk)
- [ ] Multi-cursor editing
- [ ] Code folding
- [ ] Breadcrumb navigation
- [ ] LSP integration
- [ ] Git integration (diff, blame, branch UI)
- [ ] Split editor panes
- [ ] Session history (searchable)
- [ ] Pinned context
- [ ] Keyboard shortcut customization
- [ ] Multi-language UI response (Banglish, EN, BN)

---

## Phase 7 — Billing + accounts + packaging

**Goal:** Ship to real users.

- [ ] Free tier (limited monthly tokens)
- [ ] Pro tier (unlimited + priority routing)
- [ ] Team tier (shared billing, admin dashboard)
- [ ] BYOK tier (own keys, lowest cost)
- [ ] Usage dashboard
- [ ] Invoice export
- [ ] Stripe integration
- [ ] API key management UI
- [ ] Session audit log
- [ ] Auto-updater (Tauri v2)
- [ ] <50MB installer
- [ ] <200MB idle RAM
- [ ] macOS + Windows + Linux binaries
- [ ] Data export + account deletion

---

## Phase 8 — Benchmarks

**Goal:** Public benchmark suite. Proving the "30% faster than Cursor" claim.

- [ ] Benchmark suite vs Cursor on identical prompts
- [ ] Latency (P50, P95, P99)
- [ ] First-compile rate
- [ ] Deprecated API rate
- [ ] Cost per task
- [ ] Memory accuracy
- [ ] Research cache hit rate
- [ ] Provider uptime SLA
- [ ] Cold start UX timing
- [ ] Binary size
- [ ] Memory footprint
- [ ] Public dashboard at `kontrocode.dev/bench`

---

## How to claim work

Each phase is broken into issues on GitHub. Pick one, comment "I'll take this", assign yourself, ship a PR.

If a task is not in the current phase, it's deferred. Don't start it.

## How to amend this doc

PRs welcome. The roadmap is a living document, but changes require sign-off from a maintainer — we don't want scope drift.
