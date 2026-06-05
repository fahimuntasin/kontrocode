# Contributing to KontroCode

Thank you for your interest in contributing to KontroCode, the research-first, memory-aware native coding agent.

KontroCode is a **Zed editor fork (GPUI/Rust)** with a custom **OpenCode-style agent brain (Rust)** wired via the Agent Client Protocol. This document explains how to set up your development environment.

---

## Prerequisites

### Rust

We require Rust **1.95.0** (the same as upstream Zed). Install via [rustup](https://rustup.rs):

```bash
rustup toolchain install 1.95.0
rustup default 1.95.0
```

### System dependencies

**Linux (Ubuntu/Debian):**

```bash
sudo apt-get install -y cmake libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf
```

**macOS:**

```bash
xcode-select --install
```

**Windows:**

Install [Visual Studio 2022 Build Tools](https://visualstudio.microsoft.com/downloads/) with the "Desktop development with C++" workload and the Windows 11 SDK.

---

## Repository layout

```
kontrocode/
├── zed/              Zed editor fork (GPUI) — see upstream README in zed/
├── crates/           KontroCode agentic backend (Rust)
│   ├── kontrocode-core/
│   ├── kontrocode-agent/
│   ├── kontrocode-router/
│   ├── kontrocode-research/
│   └── kontrocode-memory/
├── docs/             Architecture, roadmap, design spec
└── Cargo.toml        Root workspace (backend crates only)
```

There are **two** Cargo workspaces:

- **Root** `Cargo.toml` — our 5 `kontrocode-*` crates (the agentic backend). These are fast to build (~5 min cold, ~10 s warm).
- **`zed/Cargo.toml`** — Zed's ~230 crates (the editor). This is a full-featured GPUI editor and requires all Zed system dependencies. First build: ~30–60 min.

The two are separate by design. See `docs/ARCHITECTURE.md` §7 for why.

---

## Quick start

### 1. Build and test the backend

```bash
cd kontrocode
cargo build --workspace
cargo test --workspace           # 72 tests across 5 crates
cargo clippy --workspace --all-targets -- -D warnings
```

### 2. Build the editor (Zed fork)

```bash
cd zed
cargo run --bin kontrocode       # opens KontroCode editor window
```

### 3. Run the agentic brain

```bash
cd kontrocode
cargo run --bin kontrocode-agent ask "build me a Flutter auth screen with Google Sign-In"
```

The agent connects to the editor over the Agent Client Protocol (ACP) — JSON-RPC over stdio (which on Linux/macOS is a `socketpair`, on Windows a named pipe). When both are running, the editor's agent panel will stream responses from our `kontrocode-agent`.

---

## Development workflow

### Backend (crates/)

```
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

### Editor (zed/)

```
cd zed
cargo check --bin kontrocode
cargo clippy --workspace --all-targets
```

---

## PR guidelines

1. Use a clear, imperative PR title.
2. Include a `Release Notes:` section at the bottom of the PR body.
3. Use `Release Notes: - N/A` for docs-only or non-user-facing changes.
4. If changes touch both the editor and the backend, describe the IPC interaction.

---

## Code of Conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) (Zed's upstream CoC applies to the `zed/` subtree; KontroCode-authored code follows it by reference).

---

## License

KontroCode-authored code under `crates/`, `docs/`, and root configuration files is MIT-licensed.

The `zed/` subtree remains under the upstream Apache 2.0 + GPL 3.0 dual license. See [`zed/LICENSE-APACHE`](zed/LICENSE-APACHE) and [`zed/LICENSE-GPL`](zed/LICENSE-GPL).
