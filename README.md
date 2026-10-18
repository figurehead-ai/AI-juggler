# 🤹 AI Juggler

**Start anywhere. Switch when you hit the wall. Never lose the thread — or leave a zombie behind.**

*One console for every Provider, Account, and Instance you juggle.*

[![CI](https://github.com/figurehead-ai/AI-juggler/actions/workflows/ci.yml/badge.svg)](https://github.com/figurehead-ai/AI-juggler/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)

AI Juggler is a free, open-source desktop app (Rust + Tauri 2, macOS
first) for developers who run several AI coding subscriptions —
**Claude Code**, **Codex**, **ZCode**, and more — and are tired of
juggling them by hand. It supervises the command-line tools you already
have installed. It never proxies their traffic, never automates a vendor
sign-in, and never stores vendor credentials.

> **Status: beta development, no releases yet.** The engine, the Claude
> Code adapter, the instance-centric UI with its right-hand Inspector,
> smart onboarding, switching (labelled beta fidelity), process
> ownership and cleanup, and the two-clock usage model are built and
> continuously tested. Fleet (multi-machine) and the derived daily
> workflows are in progress. See [the task board](./docs/tasks/) for
> exactly what is done.

---

## The problem

Modern AI-assisted development means running several coding agents at once:

- Several subscriptions of the same tool (say, two Claude Max accounts) that don't run side by side out of the box
- Different tools for different strengths, each with its own logins, config folders and conventions
- A quota that runs out mid-task, with no smooth way to let another account or tool continue the work
- Sessions scattered across terminals — hard to track, hand over, or notice when one is stuck

Most of us solve this with shell aliases, terminal tabs and config-folder
hacks. AI Juggler replaces that pile with one app.

## What it does

| Pillar | What you get |
| --- | --- |
| 🔀 **Switch** | Move the work (conversation, context, project state) between Accounts and Providers. One click when the pair is proven; an itemised wizard when something would be lost; switch-back resumes the original. Cross-Provider switching is labelled beta with a fidelity badge — never marketed as lossless. |
| 👁️ **See** | A right-hand Inspector for any Provider, Account, Instance, Session or Project: usage, history, quality signals, owned processes and actions. A field shows only when it can actually be measured. |
| 🧹 **Clean** | Every process AI Juggler spawns is owned until it exits. Leftovers are verified before anything is stopped; automatic cleanup is opt-in per Account and every stop is audited. |
| 🎭 **Parallelism** | Many Accounts per Provider, each in its own isolated folder; one-click launches from a project or folder. |
| 🚀 **Smart onboarding** | Scans your machine (including other manager tools' profiles), proposes a set of Accounts, imports read-only copies, and guides each Provider's own sign-in. |
| 📊 **Two clocks** | Usage windows (5-hour/weekly) that trigger a switch suggestion, plus optional renewal dates you enter yourself. |
| 🌐 **Fleet** | Your own machines, paired by code, in one console (in progress). |

## Principles

- **Works with your tools, never replaces them.** AI Juggler orchestrates
  the vendor CLIs you install; it is not another agent.
- **Compliant by design.** It manages only accounts you own, never
  automates sign-in, never shares credentials, and respects each vendor's
  per-account limits and terms. See the [proposal](./docs/PROPOSAL.md).
- **Local-first and private.** Works fully offline with no account. An
  optional Figurehead account adds cloud features you choose to turn on
  — each one says what it sends, and content is end-to-end encrypted.
- **Honest UI.** Unknown values say "unknown"; estimates say "estimate";
  nothing is dressed up as measured.

## Supported Providers

| Provider | Status |
| --- | --- |
| Claude Code | ✅ Adapter built (the `claude` CLI) |
| Codex | 🔬 Researched; adapter not built yet |
| ZCode | 🔬 Researched; adapter not built yet |
| Others (Gemini CLI and successors, …) | 🔌 Via the adapter system |

## Build from source

You need stable Rust (pinned by `rust-toolchain.toml`), Node ≥ 22 with
pnpm 10, and [`just`](https://github.com/casey/just). On macOS, install
the Xcode Command Line Tools.

```bash
git clone https://github.com/figurehead-ai/AI-juggler.git
cd AI-juggler
pnpm --dir ui install
just dev        # the desktop app with UI hot-reload
just verify     # every gate CI runs: fmt, clippy, tests, docs, evidence, terminology
```

[CONTRIBUTING.md](./CONTRIBUTING.md) has the full developer guide.

## Documentation

- [User guide](./docs/guide/) — short how-tos (for example, what to do when a Provider is down)
- [Product proposal](./docs/PROPOSAL.md) — the problem, users, landscape, risks, compliance
- [Technical specification](./docs/SPEC.md) — architecture, adapters, data model
- [Decision log](./docs/review/decision-log.md) — settled product decisions (D-01 …)
- [ADRs](./docs/adr/) — architecture decision records
- [Task board](./docs/tasks/) — every epic and task with its status and evidence
- [Quality standards](./docs/plan/QUALITY.md) — engineering standards and the Definition of Done
- [Research notes](./docs/research/) — Claude Code, Codex, ZCode, Gemini, billing sources
- [Visual states gallery](./docs/qa/evidence/_gallery/) — every UI state in both themes

## Install

Nothing to install yet. The first release will be a signed and notarized
macOS app with built-in updates; early builds ship as `v0.1.0-beta.*`.
Watch [releases](https://github.com/figurehead-ai/AI-juggler/releases).

## Contributing and community

Contributions are welcome — read [CONTRIBUTING.md](./CONTRIBUTING.md)
first. Please follow the [Code of Conduct](./CODE_OF_CONDUCT.md). Report
security problems privately as described in [SECURITY.md](./SECURITY.md).
For questions, see [SUPPORT.md](./SUPPORT.md).

## License

[MIT](./LICENSE) © Faruk Brbovic and AI Juggler contributors
