# Contributing to AI Juggler

Thanks for helping make AI Juggler better. This guide covers how the
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
- **Follow the engineering guide and the security guidelines.** Read
  [docs/engineering/ENGINEERING-GUIDE.md](./docs/engineering/ENGINEERING-GUIDE.md)
  (patterns, rules, limits, the review checklist) and
  [docs/engineering/SECURITY-GUIDELINES.md](./docs/engineering/SECURITY-GUIDELINES.md)
  (MUST and MUST NOT rules per area, and the threat model) before your
  first PR. The architecture contract is
  [ADR-0019](./docs/adr/0019-extension-architecture.md): tools are data,
  features are modules, and nothing outside an adapter names a tool.

## How the project is organised

| Where | What |
| --- | --- |
| `crates/juggler-core` | Domain model, storage (SQLite), pure engines (ranking, quota, recommendation, funnel). No UI, no OS calls beyond the filesystem. |
| `crates/juggler-adapter` | The `HarnessAdapter` contract every tool implements: capabilities, discovery, launch specs, session watching. |
| `crates/juggler-adapters` | The tool adapters (Claude Code, Codex) and the fake tool the tests use. |
| `crates/juggler-supervisor` | Spawning, PTYs, the spawn ledger, process ownership and This Mac's measurements. |
| `crates/juggler-handover` | Switching: context packs, redaction, fidelity, the switch pipeline. |
| `crates/juggler-link` | Fleet: discovery, pairing and the encrypted link between your Macs. |
| `crates/juggler-account` | The optional Figurehead account client. |
| `crates/juggler-stats` | Opt-in usage statistics, counted on the Mac. |
| `crates/juggler-scap-init` | The audited `unsafe` call the screen-capture path needs (decision D-17). |
| `crates/juggler-login-item` | Open at login: macOS's login-item calls, `unsafe` in their bindings, behind a safe API (decision D-26). |
| `crates/juggler-app` | The Tauri app: the typed commands the UI calls. |
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
- The Tauri CLI: `cargo install tauri-cli --version "^2" --locked`
- macOS: Xcode Command Line Tools. Linux: the Tauri system packages
  listed in `.github/workflows/ci.yml` (`build-linux`)

```bash
pnpm --dir ui install
just dev            # run the app as AI Juggler Dev (UI hot-reload + Rust rebuild);
                    # its own data folder, so an installed AI Juggler is never touched
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
  paths, documented public items, `unsafe` is forbidden (two audited
  exceptions, each its own small crate: D-17 and D-26). Rustdoc runs with warnings as errors —
  don't leave a bare `<X>` in a `///` comment.
- **UI:** both themes (Light and Dark) are first-class, colours come only
  from the tokens in `ui/src/styles.css`, and small text uses
  `text-muted` or `text-ink` (`text-faint` fails AA contrast on the
  canvas). Every new state gets a gallery capture in
  `ui/tests/visual/capture.spec.ts` and an axe scan in `a11y.spec.ts`.
  A new or rebuilt screen also gets a full visual sweep
  (`just visual-sweep`, D-21): the whole gallery, your branch against
  `main`, with every changed state listed — see
  [QUALITY.md](docs/plan/QUALITY.md) § Visual checks.
- **Words:** plain words on screen. "Profile" never means an account, and
  "harness" is a code name only; `just terminology` checks both.
- **Flaky gate?** Don't skip it or loosen its threshold. Open an issue with
  the failure and fix the cause (the flake log is
  `docs/qa/evidence/AJ-E806/manifest.md`).

## Picking up work

Issues labelled [`good first issue`](https://github.com/figurehead-ai/AI-juggler/labels/good%20first%20issue)
are a good place to start. The [task board](./docs/tasks/) lists every epic
and task (`AJ-E101` …) with its requirements, quality gates and status. A task is
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

## Licensing and sign-off

By contributing, you agree that your contributions are licensed under
the [MIT License](./LICENSE) that covers this repository. You keep the
copyright to your contribution.

**Sign off every commit** with the
[Developer Certificate of Origin](https://developercertificate.org)
(DCO). It's one line at the end of the commit message:

```
Signed-off-by: Your Name <you@example.com>
```

`git commit -s` adds it for you. The sign-off certifies that you wrote
the change, or otherwise have the right to submit it under the
project's license. Use a name you're known by. Your GitHub `noreply`
address is fine if you'd rather not publish your e-mail. Forgot? Run
`git commit --amend -s` for the last commit, or
`git rebase --signoff main` for a branch.

Contributing doesn't grant rights to the AI Juggler or Figurehead names,
logos or app icon. See [TRADEMARKS.md](./TRADEMARKS.md).
