# Probe C2: staging record (2026-11-16)

Staged by an Opus session, headless, with `scripts/qa/c2_probe.py` (runbook:
`c2-runbook.md`). No sign-in, no model call, no ChatGPT app. codex-cli
0.163.0 (`/opt/homebrew/bin/codex`); probe `bin/juggler-agent` sha256
`07952f1fe0a891ff…` (the debug build of this commit's probe, `AIJ_PROBE_V1`); `config.toml`
sha256 `6e042fac710adb30…`. Reports: `/tmp/aij-lab-c2/evidence/`.

## `~/.codex`, names and dates only

- Baseline taken 14:10:14 by `stage`: 71,382 entries, digest
  `dbeffb7a65b4b419…`. Every step since (nine dry runs and five
  preflights, with and without the guard) found it
  **unchanged**, before and after.
- Its top level (`ls -la` names and dates) is identical to a listing taken
  at 13:54, before this session ran any codex.
- **One earlier change, not this session's:** `~/.codex/tmp/arg0-macos/codex-arg0y5tVfw/`
  (a lock and two links to the homebrew codex) appeared at 13:54:58,
  before the baseline. It is what a homebrew codex run **without**
  `CODEX_HOME` leaves: a control with a throwaway `HOME` made the same
  folder only when `CODEX_HOME` was unset, and every codex command this
  session ran (`--version`, `--help`, `login --help`, `app-server --help`,
  `mcp --help`, `generate-json-schema`) left nothing there with it set.
  Its source is another process on this Mac (another session's harness
  started at 13:55:00; AI Juggler's own Codex discovery is a candidate).
  Left in place: `~/.codex` is never written, and Codex clears stale arg0
  folders itself at its next start on that home.

## The dry app-server (start, `initialize`, `thread/list`, `mcpServerStatus/list`, stop)

Without the guard (`dry --no-guard`, 14:27:09, the final build), as asked: ps and lsof
every 0.5 s (5 samples) plus 2 snapshots.

| pid | ppid | CODEX_HOME | CODEX_SQLITE_HOME | HOME | command |
|---|---|---|---|---|---|
| 70871 | 70851 (the driver) | /tmp/aij-lab-c2/codex-home | /tmp/aij-lab-c2/codex-home | ~ | `node /opt/homebrew/bin/codex app-server --listen stdio:// --strict-config` |
| 70876 | 70871 | /tmp/aij-lab-c2/codex-home | /tmp/aij-lab-c2/codex-home | ~ | `…/codex-darwin-arm64/vendor/aarch64-apple-darwin/bin/codex app-server --listen stdio:// --strict-config` |
| 70883 | 70876 | not read (lived under 0.5 s) | | | the probe, `--mcp --probe` (its records: `initialize`, `notifications/initialized`, `tools/list`) |

- `initialize`: `codexHome` = `/private/tmp/aij-lab-c2/codex-home`.
- lsof: 152 open names, **0 under `~/.codex`**, 0 under
  `~/Library/Keychains`, **0 internet sockets**.
- After: `~/.codex` unchanged; nothing left running (`pgrep` of the lab
  and of `codex app-server`: none).
- The same with the guard (`dry`, 14:21:32, the build before the last
  refactor): the same two processes and answers, 0 under `~/.codex`, 0
  sockets.
- The very first dry run (before the `[features]` block) showed codex's
  own plugin sync: `git ls-remote` and a clone of github.com/openai/plugins
  (93 MB) into the lab home's `.tmp`. Deleted; the features above stop it.

## Preflight (no model, no sign-in): what the app-server gives its MCP servers

`preflight` (14:27:15, guarded, the final build): one app-server (pid 70953), two threads,
the probe called directly on each (`mcpServer/tool/call`, the client's
path, not the model's), and `<probe> --probe-shell` through
`thread/shellCommand` on each.

| | Thread A | Thread B |
|---|---|---|
| Probe process | 70957 | 71350 |
| Its parent (the driver's ps) | 70953 `codex app-server` | 70953 `codex app-server` |
| Environment names | HOME, LANG, LOGNAME, PATH, SHELL, TMPDIR, USER, `__CF_USER_TEXT_ENCODING` | the same |
| `initialize` `clientInfo` | `codex-mcp-client` "Codex" 0.163.0 | the same |
| `_meta` keys | `progressToken`, `threadId` | the same |
| `_meta.threadId` | thread A's id | thread B's id |
| Approval prompt | none | none |
| `--probe-shell` `CODEX_THREAD_ID` | thread A's id | thread B's id |
| Its parent | 70953 (`zsh -c` execs it) | 70953 |

- One MCP server process **per thread**, each the app-server's child; no
  thread id in their environment; `_meta.threadId` names the calling
  thread on the direct path; no connection lease key (0.163). The model's
  path (`run`, after the sign-in) decides C2.
- The unguarded preflight (14:17) gave the same, with the probe's own ps
  chain (juggler-agent, codex, node, the driver).
- ~/.codex unchanged; 0 open under it in 25 samples; every process reaped.
