# Contributing to AI Juggler

Thanks for helping make AI Juggler better! 🤹 This guide covers how the
project is organised, how to build and test it, and what a mergeable
change looks like.

## Ground rules

- **Be kind.** Everyone participating follows the [Code of Conduct](./CODE_OF_CONDUCT.md).
- **This repository is public.** Never commit personal data — real
  account names, e-mail addresses, home-directory paths, hostnames,
  screenshots of a real desktop, transcripts, tokens or keys. Use the
  fixtures (`/Users/you/...`, `Work`, `Personal`, `example.com`) for
  every example, test and evidence file. The evidence gate scans for
  secrets, but it can't recognise your name.
- **Final artifacts only.** No chat logs or AI-assistant session dumps
  in commits, issues or PRs. If you explored several designs, the merged
  doc states the chosen one plus a short "alternatives considered".
- **Security problems go through private reporting** — see [SECURITY.md](./SECURITY.md).

## How the project is organised

| Where | What |
| --- | --- |
| `crates/juggler-core` | Domain model, storage (SQLite), pure engines (ranking, quota, recommendation, funnel). No UI, no OS calls beyond the filesystem. |
| `crates/juggler-adapter` | The `HarnessAdapter` trait every Provider implements. |
| `crates/juggler-adapters` | Provider adapters (Claude Code) and the fake adapters tests use. |
| `crates/juggler-supervisor` | Spawning, PTYs, the spawn ledger and process ownership. |
| `crates/juggler-handover` | Switching: context packs, redaction, fidelity, the switch pipeline. |
| `crates/juggler-app` | The Tauri app: typed commands the UI calls. |
| `ui/` | React + TypeScript webview UI. `ui/src/lib/bridge.ts` is the only place that talks to the backend; it serves deterministic mocks in a plain browser. |
| `docs/` | Proposal, spec, ADRs, decision log, task board, research, QA evidence. |

Product decisions that are settled live in
[docs/review/decision-log.md](./docs/review/decision-log.md) (D-01 …).
Please don't re-open a settled decision in a PR; open a discussion
instead. Architecture decisions are [ADRs](./docs/adr/).

## Development setup

Prerequisites:

- Rust — the version in `rust-toolchain.toml` installs automatically via `rustup`
- Node ≥ 22 and pnpm 10
- [`just`](https://github.com/casey/just)
- macOS: Xcode Command Line Tools. Linux: the Tauri system packages
  listed in `.github/workflows/ci.yml` (`build-linux`)

```bash
pnpm --dir ui install
just dev            # run the desktop app (UI hot-reload + Rust rebuild)
pnpm --dir ui dev   # UI only, in a browser, against the mock bridge
```

On macOS machines with only the Command Line Tools, cargo needs the
Swift runtime shim. `just` sets it for you; for plain `cargo` commands:

```bash
export RUSTFLAGS=$(./scripts/ensure_swift_runtime.sh | tail -1 | sed 's/^RUSTFLAGS=//')
```

## The gates

Every PR must pass the same gates CI runs:

```bash
just verify                              # fmt → clippy -D warnings → tests → docs → evidence → terminology
pnpm --dir ui test                       # vitest component tests
pnpm --dir ui build                      # typecheck + production bundle
pnpm --dir ui exec playwright test       # visual states + a11y (for UI changes)
```

- **Rust:** typed errors, no `unwrap`/`expect`/`panic!` in production
  paths, documented public items, `unsafe` is forbidden (one audited
  exception, recorded as D-17). Rustdoc runs with warnings as errors —
  don't leave a bare `<X>` in a `///` comment.
- **UI:** both themes (Light and Dark) are first-class, colours come only
  from the tokens in `ui/src/styles.css`, and small text uses
  `text-muted` or `text-ink` (`text-faint` fails AA contrast on the
  canvas). Every new state gets a gallery capture in
  `ui/tests/visual/capture.spec.ts` and an axe scan in `a11y.spec.ts`.
- **Words:** the UI says Provider / Account / Instance / Launch settings.
  "Profile" never means an account, and "harness" is internal-only — the
  terminology gate enforces this.
- **Flaky gate?** Don't skip or re-threshold it. Record it and fix the
  cause (see AJ-E806 in `docs/qa/evidence/AJ-E806/manifest.md`).

## Picking up work

The [task board](./docs/tasks/) lists every epic (`E1` … `E10`) and task
(`AJ-E101` …) with its requirements, quality gates and status. A task is
**DONE-EVIDENCED** only when its evidence manifest exists under
`docs/qa/evidence/<task-id>/` and passes at the stated threshold. If you
want to take a task, comment on its issue (or open one) first so work
isn't duplicated. Smaller fixes and docs improvements can go straight to
a PR.

## Commits and pull requests

- **Commits:** [Conventional Commits](https://www.conventionalcommits.org/)
  (`feat:`, `fix:`, `docs:`, `test:`, `refactor:`, `chore:`), optionally
  scoped with the epic and task, e.g. `feat(e3/AJ-E309): …`.
- **PRs:** small and focused, described by the user-facing change. Link
  the issue or task. Fill in the PR template's Definition of Done.
- **Merging:** squash-merge after review and green CI. `main` must always
  build.
- **AI-assisted contributions** are welcome. You are responsible for
  every line you submit: review it, test it, and make sure it follows
  these rules (especially the no-personal-data rule).

## Releases

Releases are cut by the maintainer: [docs/RELEASING.md](./docs/RELEASING.md)
has the signing setup, the release-notes rules, and the rollback steps.
Contributors only add their change to `CHANGELOG.md` under `[Unreleased]`.

## Licensing

By contributing, you agree that your contributions are licensed under
the [MIT License](./LICENSE) that covers this repository.
