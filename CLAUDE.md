# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

PhoTensor is a Rust workspace for photonic neural networks — a unified framework aiming to switch seamlessly between pure-software simulation and real photonic hardware deployments, including fourth-generation (dendritic) and fifth-generation (embodied) neural networks.

## Environment

All tooling is pinned via Nix flakes (`flake.nix`). Enter the dev shell with `nix develop` (also the default container CMD in the root `Dockerfile`, used by Tencent Cloud CNB via `.cnb.yml`). The shell provides: pinned Rust toolchain (`rust-toolchain.toml`: 1.96.0 with rustfmt, clippy, rust-src, rust-analyzer, x86_64-unknown-linux-gnu target), `maturin`, Python 3 + `twine`, Node.js 24, `pnpm`, `clang`/`lld` (used as the linker via `.cargo/config.toml`).

Toolchain and linker are pinned — do not run `rustup` overrides or change the linker; `.cargo/config.toml` forces `clang` + `lld` for the `x86_64-unknown-linux-gnu` target.

## Workspace layout

Cargo workspace (root `Cargo.toml`, `resolver = "2"`, edition 2024). Crates live under `src/<name>/`, each with its own `Cargo.toml`:

- `photensor-core` (`src/core`) — shared core types
- `photensor-sim` (`src/sim`) — pure-software simulator; MZI array engine lives in `src/sim/src/mzi_array/engine.rs`
- `photensor-hw` (`src/hw`) — real photonic hardware deployment backend
- `photensor-burn` (`src/burn`) — Burn-based tensor backend integration
- `photensor-macros` (`src/macros`) — proc-macros
- `pyphotensor` (`src/pyphotensor`) — Python bindings via PyO3/maturin (`crate-type = ["cdylib"]`)
- `photensor` (`src/photensor`) — top-level facade crate

The intended model: write once against the simulator, then deploy the same program to real photonic hardware by swapping backends.

## Common commands

Rust (run from repo root):

```bash
cargo build                       # build whole workspace
cargo build -p photensor-sim      # build a single crate
cargo test                        # run all tests
cargo test -p photensor-sim       # tests for one crate
cargo test -p photensor-sim engine -- --nocapture   # single test (filter by name)
cargo clippy --workspace --all-targets
cargo fmt --all
cargo doc --workspace --no-deps
```

Python binding (`pyphotensor`, built with maturin):

```bash
cd src/pyphotensor
maturin develop --release          # build & install into current venv/interpreter
maturin build --release            # produce wheel in target/wheels/
# publish: twine upload target/wheels/*
```

Docs site (`docs/`, Rspress + pnpm; lint/format via Biome):

```bash
cd docs
pnpm install
pnpm run dev        # dev server
pnpm run build      # production build
pnpm run preview    # preview build
pnpm run lint       # Biome lint
pnpm run format     # Biome format
```
