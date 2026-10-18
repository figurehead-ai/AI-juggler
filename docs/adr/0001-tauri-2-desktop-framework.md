# ADR-0001: Desktop framework is Tauri 2

**Status:** Accepted
**Date:** 2026-10-12
**Deciders:** maintainer + spec review (round 1)

## Context

AI Juggler needs a native desktop shell for a Rust core with a data-dense UI: live session board, usage dashboards (charts), embedded terminals, tray, notifications, and a signed self-updater. macOS-first, cross-platform later. Evidence in [research/landscape-and-stack.md §4](../research/landscape-and-stack.md).

## Decision

Build on **Tauri 2** (v2.12.x line): Rust backend core in the Tauri process, system-webview UI with React 18 + TypeScript + Tailwind + shadcn/ui, xterm.js terminals fed by portable-pty, uPlot for time-series charts, `tauri-plugin-updater` (signature verification cannot be disabled) for self-updates.

## Alternatives considered

- **egui** — strong for dense diagnostics, weak rich-text/terminal/chart ecosystem; would require building a PTY canvas widget and charts from scratch.
- **GPUI (Zed)** — fastest renderer, but sparsely documented for external use and tightly coupled to Zed's stack.
- **Dioxus** — promising Rust-native option; younger ecosystem, fewer proven desktop apps of this shape.
- **Slint / Flutter** — embedded/mobile strengths; Flutter is not Rust; neither has the terminal+chart story.

## Consequences

- The only mature path that gives HTML canvas (xterm.js + uPlot) *and* a Rust core; validated by the two closest comparable OSS apps (cc-switch, opcode), which use exactly this stack.
- Updater has first-class signed feeds from GitHub Releases; OS signing (Developer ID via Vuxtra) composes cleanly.
- We carry a JS/TS frontend: the UI stays logic-free (typed events from Rust, QUALITY.md §4) to keep the system auditable and testable from the Rust side.
