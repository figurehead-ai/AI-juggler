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
| R1 a node-script tool is found and starts with the Finder `PATH` | `cargo test -p juggler-supervisor --test node_script_tool` (fixture: a temp `bin/` holding a stand-in `node` and a `#!/usr/bin/env node` `codex`; the checks run in a child with `env_clear`, `PATH=/usr/bin:/bin:/usr/sbin:/sbin` and a throwaway `HOME`: discovery, a piped headless launch and the PTY sign-in through the real supervisor) | discovery `Installed 0.163.0`; both launches exit 0 having run through `node` | 1 passed | PASS | b6d6ad95 | 2026-10-21 | claude-opus |
| R1 fails on the old code | the same test with `supervisor.rs`, `codex.rs`, `claude_code.rs` reverted to `0f4dc8e3`; then with only `supervisor.rs` reverted | fails | `left: Failed { message: "--version exited with exit status: 127" }`; with only the launch change reverted: `s-headless: printed "env: node: No such file or directory\n"` | PASS | b6d6ad95 | 2026-10-21 | claude-opus |
| R2 the login shell's `PATH`, else the fallback | `cargo test -p juggler-adapter user_path` (a stand-in shell that prints noise then its `PATH`; one that hangs, with a 300 ms limit; one that fails; one that's missing; the environment allowlist; only a `$SHELL` listed in `/etc/shells`; fallback folders, de-duplication, nvm's default alias, the binary's folder first, the copyable command's program word) | 12 pass; the hanging shell gives up within the limit; nothing outside the allowlist reaches the shell or a probe | 12 passed | PASS | b6d6ad95 | 2026-10-21 | claude-opus |
| R3 a binary whose version check fails is found | `cargo test -p juggler-adapters --test codex_adapter a_binary_whose_version_check_fails_is_found` (a `codex` that exits 127) | `Failed`, `is_found()`, binary kept; a missing path isn't found | passed | PASS | b6d6ad95 | 2026-10-21 | claude-opus |
| R4 the UI offers it | `pnpm --dir ui exec vitest run src/app-found-tool.test.tsx` (Codex `probe_failed` with a binary: Accounts › Add account asks which tool and offers Codex, "Found; couldn't check its version"; first run lists `~/.codex` under Codex with "Codex found; couldn't check its version", no "isn't installed" note) | 2 pass; both fail on the old UI | 2 passed; with `App.tsx`, `AddProviderSheet.tsx`, `FirstRun.tsx`, `RightInspector.tsx` reverted: 2 failed | PASS | b6d6ad95 | 2026-10-21 | claude-opus |
| R4 gallery state (axe runs on it in the a11y gate) | `pnpm --dir ui exec playwright test tests/visual/capture.spec.ts -g "provider cards"` → [the chooser, dark](docs/qa/evidence/_gallery/38-flows-cde/c-provider-cards-unchecked-1280x800-dark.png) | renders; Codex choosable | 8 passed | PASS | b6d6ad95 | 2026-10-21 | claude-opus |
| R5 walk by hand: the Dev build, opened with `open`, under Finder's environment | see "Walk by hand" | the stand-in `node` runs both tools' `--version`; the real data folder untouched | both ran, twice each; untouched | PASS | b6d6ad95 | 2026-10-21 | claude-opus |

## Gates

Run locally on the branch, 2026-10-21:

- `just verify`: every step passes (fmt, clippy with and without default features, `cargo test --workspace`, leaked fixtures, rustdoc, evidence, terminology, task counts, stats doc and events, polling audit, release self-tests). The first run was stopped by a SIGTERM from outside this session during `cargo test`; on the second, rustdoc caught a public doc linking a private constant (fixed); the remaining steps then passed.
- `cargo deny check`: advisories, bans, licenses and sources ok.
- vitest: 553 of 553 after merging main. One earlier run had one failure (first run's "Stop while bringing in"), under a parallel cargo build; that file passed 3 of 3 alone.
- Playwright: 909 passed.
- a11y gate (`scripts/a11y_gate.sh`): **PASS**; 852 gallery states scanned (848 before: the four new chooser states), 0 critical, 0 serious, 0 moderate, 0 minor. Its run had one failure, "determinism — two captures are byte-identical", under a parallel release build; it passed 3 of 3 alone. Its regenerated evidence (`AJ-E804/`) isn't committed here.
- visual gate (`scripts/visual_gate.sh`, pinned container, after merging main): 137 passed against the approved baselines.
- No visual baseline changes: first run's regression baselines show the Claude-only screen, whose header words are unchanged ("Claude Code 2.1.296 installed").

## Walk by hand

AI Juggler Dev, built from this branch (`cargo tauri build --config crates/juggler-app/tauri.dev.conf.json --bundles app`), was opened the way Finder opens an app, with a throwaway `HOME`:

```bash
env -i HOME=$H USER=$USER PATH=/usr/bin:/bin:/usr/sbin:/sbin /usr/bin/open -n --env HOME=$H "AI Juggler Dev.app"
```

The fixture `$H` (in the session's scratchpad, never the real home):

- `fakebrew/bin/node`, a stand-in: it logs each call (its arguments only) to `$H/node-calls.log` and answers `--version`;
- `fakebrew/bin/codex` and `fakebrew/bin/claude`, both `#!/usr/bin/env node` scripts (Homebrew's layout: the tools next to `node`);
- `.zprofile` putting `fakebrew/bin` first, as Homebrew's `shellenv` does;
- `.codex/` with a `config.toml` and a synthetic `auth.json` (field shape only; no real token).

Observed:

1. The running app's own environment (`ps -E`): `PATH=/usr/bin:/bin:/usr/sbin:/sbin`, `HOME=$H`, `SHELL=/bin/zsh`, as Finder gives it.
2. The same fixture outside the app, with that `PATH`: `env: node: No such file or directory`, exit 127. This is the report.
3. On first run, `$H/node-calls.log` read `node ran codex --version`, `node ran claude --version`, twice each: discovery found both tools through the login shell's `PATH` and ran them through `node`.
4. The app kept its data under `$H/Library/Application Support/AI Juggler Dev`; the real `~/Library/Application Support/AI Juggler Dev` was not created. Nothing under the real `~/.codex` or `~/.claude` was read or written. The app was quit with SIGTERM (no Apple Events, no permission prompt).

A first attempt with a plain `open` passed the calling shell's full `PATH` to the app (Homebrew and nvm included), so it didn't show the Finder case; the `env -i` launch above does.

Not walked: clicking through first run, Add account and a Codex session in the real window. This Mac gives agent sessions no Screen Recording or Accessibility grant (and none was requested). Those paths are covered by `node_script_tool` (discovery, a piped launch and the PTY sign-in through the real supervisor, under the Finder `PATH`) and by `app-found-tool.test.tsx` and the gallery state above.
