# Live Claude walks under ADR-0027 condition 7b: P1, P2, P7.3 and E23's Claude steps

The run sheet for the walks that need a live Claude Code session. They run in AI Juggler Dev on the real HOME, with the "Lab" Claude Code App only. Condition 7b's terms bind every step.

## Who does what

- **Driver: sonnet.** It follows this sheet step by step, by Dev's process id only. It clicks with `osascript` and captures with `screencapture -l` on that pid's window. It types only the harmless prompts written below.
- **Checker: Opus.** Before anything is recorded, it checks each FAIL, and each PASS that closes a requirement, against the sheet's expectation and the capture.
- **The maintainer.** Any macOS prompt (Accessibility, Automation) is theirs to click. The host app already has the permission (main's and the Floor's runs clicked Dev's window by pid all day with no prompt), so no sitting is needed. If a prompt does appear, the driver stops the run and tells the main session.

## The fixed facts

- **The Lab App:** AI Juggler Dev's own Claude Code folder `~/Library/Application Support/AI Juggler Dev/accounts/claude-code/claude-code-361780292000` (Max, signed in by the maintainer 2026-11-06). No sign-in. Its credentials and the Keychain are never read.
- **Codex** (P1, P2's second tool): the real Codex sign-in where it is (condition 7), harmless commands only.
- **Projects:** folders under `~/lab/` only: `~/lab/web` (P1), `~/lab/team` (P2), `~/lab/client` (P7). Each is made empty with `git init` at the start and removed at the end.
- **Harmless prompts only:** `ls`; `echo lab`; "write the word lab to lab.txt"; and the pool's own actions (claim a task, mark it in review, send a message). Nothing else is typed into a session.
- **Never touched:** the installed AI Juggler (same process name `juggler-app`: drive by pid only), the maintainer's other Claude profiles and Apps, any other project.

## Before

1. `scripts/qa/runtime_lab_snapshot.sh take before <the Lab App's folder> "$HOME/.codex"` (paths, sizes and times only).
2. Build the Dev bundle (`just app`, or the debug bundle) to its own path. Start it with `open -n` and note its pid. Every later command names that pid.
3. A probe before any step: `osascript -e 'tell application "System Events" to get name of first process whose frontmost is true'`, then one harmless click in Dev's window. Target the window as `item 1 of (processes whose unix id is <pid>)`; its name starts with "AI Juggler". A permission prompt stops the run.
4. To check a Claude app result, capture that profile's window by its CGWindow id (`CGWindowListCopyWindowInfo` by owner pid, then `screencapture -l <id>`): System Events confuses the processes that share the name "Claude".

## The walks

| Walk | Steps (from `docs/qa/scenarios/`) | Expected |
|---|---|---|
| P1 Writer and reviewer | All 9 steps of `P1-writer-and-reviewer.toml`, the Lab App as "Claude Code · Work", Codex as the reviewer | As each step's `expect` |
| P2 Feature team | All 6 steps of `P2-feature-team.toml`, three Lab App sessions w1–w3 | As each step's `expect` (the quit and reopen is the Dev app by its pid) |
| P7 step 3 | `P7-sealed-client-project.toml` step 3: Claude's own cross-session message into c1 | Refused, as AJ-E3002's lab proved |
| E23 steps 3–4 | `docs/qa/runtime-lab.md` steps 3 and 4: hooks and the status line; without a terminal (Beta) | As the runtime lab's sheet |

For each step, record: before and after captures (pid window only), what was expected, what showed, the time from click to the screen settling, and a 1–5 score for works, speed, clarity and layout. The record goes in `docs/qa/0.4-walks/<ID>.md`.

## After

1. Stop every lab session from AI Juggler. Remove the pools made here, then remove `~/lab/web`, `~/lab/team` and `~/lab/client`.
2. Quit the Dev app by its pid. Check that nothing of it runs (`scripts/qa/runtime_lab_snapshot.sh procs`: pid and name only).
3. Take the after snapshot and diff it. The Lab App's folder may change only by its own session files. `config.toml` and the Codex sign-in stay unchanged.
4. Open every capture by eye before it's kept. Redact any that show other Apps or names with the fail-closed tool and its sidecar. Unredacted captures never leave the scratchpad.
