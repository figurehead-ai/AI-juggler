# Evidence manifest — fixes for the maintainer's reports

Not a task's evidence: each section is one bug the maintainer reported
while dogfooding a release, with the fix's proof. One branch per batch of
reports.

## 0.0.11 — Codex not found when AI Juggler is opened from Finder

**Report.** A fresh install of 0.0.11, opened from Finder: importing
offered only Claude profiles, although the Codex CLI and `~/.codex` are
there (the ChatGPT desktop app, `com.openai.codex`, shares `~/.codex`).

**Cause.** An app opened from Finder inherits
`PATH=/usr/bin:/bin:/usr/sbin:/sbin`. `/opt/homebrew/bin/codex` is a
`#!/usr/bin/env node` script, so `codex --version` failed with
"env: node: No such file or directory" (exit 127); discovery reported
Codex as not installed, and Add account went straight to Claude. Nothing
gave launches the user's `PATH` either, so a Codex session or its sign-in
(and any npm-installed CLI, Claude Code included) would have failed the
same way.

**Fix.** `juggler_adapter::user_path` resolves the user's `PATH` once at
startup, off the UI thread: the login shell's (`$SHELL -l -c`, printed
between markers so startup-file output is ignored, killed after 2 s),
else the inherited `PATH`, plus `/opt/homebrew/bin`, `/usr/local/bin`,
`~/.local/bin`, `~/.npm-global/bin`, `~/.volta/bin`, `~/.bun/bin` and the
active nvm `node`; the found binary's own folder always comes first. It
is used by every discovery probe, every supervised launch (PTY and piped,
the sign-in PTY included) and the desktop launch, and the copyable
command names the found binary by its full path when the user's terminal
`PATH` wouldn't find it. Its value is never logged. A binary found whose
`--version` still fails counts as found: Add account offers it and first
run names it, "Found; couldn't check its version".

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a node-script tool is found and starts with the Finder `PATH` | `cargo test -p juggler-supervisor --test node_script_tool` (fixture: a temp `bin/` holding a stand-in `node` and a `#!/usr/bin/env node` `codex`; the checks run in a child with `env_clear`, `PATH=/usr/bin:/bin:/usr/sbin:/sbin` and a throwaway `HOME`: discovery, a piped headless launch and the PTY sign-in through the real supervisor) | discovery `Installed 0.163.0`; both launches exit 0 having run through `node` | 1 passed | PASS | FILLED_AT_COMMIT | 2026-10-21 | claude-opus |
| R1 fails on the old code | the same test with `supervisor.rs`, `codex.rs`, `claude_code.rs` reverted to `0f4dc8e3`; then with only `supervisor.rs` reverted | fails | `left: Failed { message: "--version exited with exit status: 127" }`; with only the launch change reverted: `s-headless: printed "env: node: No such file or directory\n"` | PASS | FILLED_AT_COMMIT | 2026-10-21 | claude-opus |
| R2 the login shell's `PATH`, else the fallback | `cargo test -p juggler-adapter user_path` (a stand-in shell that prints noise then its `PATH`; one that hangs, with a 300 ms limit; one that fails; one that's missing; fallback folders, de-duplication, nvm's default alias, the binary's folder first, the copyable command's program word) | 9 pass; the hanging shell gives up within the limit | 9 passed | PASS | FILLED_AT_COMMIT | 2026-10-21 | claude-opus |
| R3 a binary whose version check fails is found | `cargo test -p juggler-adapters --test codex_adapter a_binary_whose_version_check_fails_is_found` (a `codex` that exits 127) | `Failed`, `is_found()`, binary kept; a missing path isn't found | passed | PASS | FILLED_AT_COMMIT | 2026-10-21 | claude-opus |
| R4 the UI offers it | `pnpm --dir ui exec vitest run src/app-found-tool.test.tsx` (Codex `probe_failed` with a binary: Accounts › Add account asks which tool and offers Codex, "Found; couldn't check its version"; first run lists `~/.codex` under Codex with "Codex found; couldn't check its version", no "isn't installed" note) | 2 pass; both fail on the old UI | 2 passed; with `App.tsx`, `AddProviderSheet.tsx`, `FirstRun.tsx`, `RightInspector.tsx` reverted: 2 failed | PASS | FILLED_AT_COMMIT | 2026-10-21 | claude-opus |
| R4 gallery state (axe runs on it in the a11y gate) | `pnpm --dir ui exec playwright test tests/visual/capture.spec.ts -g "provider cards"` → [the chooser, dark](docs/qa/evidence/_gallery/38-flows-cde/c-provider-cards-unchecked-1280x800-dark.png) | renders; Codex choosable | 8 passed | PASS | FILLED_AT_COMMIT | 2026-10-21 | claude-opus |
| R5 walk by hand | see below | | | | | | |

## Gates

FILLED_AFTER_GATES

## Walk by hand

FILLED_AFTER_WALK
