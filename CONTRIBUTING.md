# Contributing to KontroCode

Thanks for wanting to make KontroCode better. This document is short on purpose — we want you shipping in 15 minutes, not reading 15 pages.

## Code of conduct

Be kind. We're all here to ship good software. Harassment, slurs, or personal attacks are not tolerated and will get you banned.

## Dev setup

```bash
# Prereqs
rustup install stable          # Rust 1.75+
node --version                 # Node 20+
pnpm --version                 # pnpm 9+
# macOS: xcode-select --install
# Linux: sudo apt install libwebkit2gtk-4.1-dev libssl-dev

# Clone & install
git clone https://github.com/fahimuntasin/kontrocode.git
cd kontrocode
pnpm install
pnpm --filter @kontrocode/desktop tauri dev
```

## Repo layout (30-second tour)

| Path | What it is |
|------|------------|
| `apps/desktop` | Tauri v2 native shell — UI lives here |
| `crates/kontrocode-agent` | The OpenCode-style agent loop |
| `crates/kontrocode-router` | Multi-provider LLM routing (9 providers) |
| `crates/kontrocode-research` | Docs/npm/registry scrapers |
| `crates/kontrocode-memory` | User profile + RAG store |
| `crates/kontrocode-core` | Shared types — everyone depends on this |
| `packages/cli` | `@kontrocode/cli` — headless agent for terminal use |
| `docs/` | Design, architecture, roadmap |

**Read these first (in order):**
1. [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) — how it all fits together
2. [`docs/design.md`](docs/design.md) — visual + UX rules (do not violate the theme)
3. [`docs/ROADMAP.md`](docs/ROADMAP.md) — what's being built next, claim a phase

## Branch & commit conventions

- `main` is always green. No direct pushes.
- Branch from `main`: `feat/short-name`, `fix/short-name`, `chore/short-name`
- One logical change per PR
- Commit messages: imperative, lowercase, ≤72 chars
  - `feat(router): add mistral provider with cost-aware fallback`
  - `fix(memory): resolve race in profile updater`
  - `chore: bump tokio to 1.42`

## PR checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` clean
- [ ] `pnpm -r test` passes
- [ ] `pnpm -r lint` passes
- [ ] `pnpm -r typecheck` passes
- [ ] Public APIs have doc comments (`///`)
- [ ] No secrets, API keys, or `.env` files committed
- [ ] Updated relevant docs (design.md, ARCHITECTURE.md, etc.) if behavior changed
- [ ] If you changed the UI, attached a screenshot or short clip

## Coding rules

- **Rust:** Edition 2021. `unsafe` only with a `// SAFETY:` comment. No `unwrap()` outside tests — use `?` or `anyhow!`/`thiserror`.
- **TypeScript:** Strict mode. No `any`. Prefer `unknown` + narrowing. Zod for runtime validation at boundaries.
- **Errors:** `Result<T, E>` everywhere in Rust. Never swallow. Never `panic!` in library code.
- **Async:** `tokio` in Rust. `never` block the runtime. Use `tokio::join!` for parallel work.
- **Tests:** Unit tests next to code (`#[cfg(test)] mod tests`). Integration tests in `tests/`. Frontend: Vitest. Aim for >80% on shared crates, >60% everywhere else.
- **Comments:** Code explains *why*, not *what*. No banner comments. No dead code — delete it.
- **Logging:** `tracing` in Rust, structured fields. Never `println!` in library code.

## Adding a new LLM provider

1. Add a config struct in `crates/kontrocode-router/src/providers/<name>.rs`
2. Implement the `Provider` trait (see `providers/anthropic.rs` as the reference)
3. Register it in `crates/kontrocode-router/src/registry.rs`
4. Add the env var to `.env.example` and `README.md`
5. Add a test in `crates/kontrocode-router/tests/<name>_test.rs` using a recorded HTTP fixture
6. Open PR — CI will run a smoke test against the real API

## Adding a new research source

1. Add a fetcher in `crates/kontrocode-research/src/sources/<name>.rs`
2. Implement the `ResearchSource` trait
3. Add it to the parallel `tokio::join!` in `crates/kontrocode-research/src/runner.rs`
4. Add a fixture test in `tests/fixtures/`
5. Document the source in `docs/ARCHITECTURE.md` § Research

## Issues & discussions

- **Bug reports:** Use the bug report template. Include OS, KontroCode version, repro steps, and a minimal repro.
- **Feature requests:** Use the feature template. We accept very few of these — pitch the *problem* first, the *solution* second.
- **Questions:** GitHub Discussions, not issues.

## Review SLA

We try to review every PR within 48 hours. If you don't hear back, ping us on Discord.

## License

By contributing, you agree your contributions are MIT-licensed.
