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
- **Choosing a folder:** wherever a step opens a folder picker, press ⌘⇧G and type the exact path (`/Users/<you>/lab/web`, `/lab/team`, `/lab/client`), Return, then Open. Never browse, and never select anything in Documents or any real folder. If that can't be done cleanly, Cancel the dialog and report. (The first run's driver left the picker open on Documents.)
- **Harmless prompts only:** `ls`; `echo lab`; "write the word lab to lab.txt"; and the pool's own actions (claim a task, mark it in review, send a message). Nothing else is typed into a session.
- **No connectors, no artifacts, no Docs, no sharing.** A Lab session uses only its own folder's files and the pool's own tools (`send_message`, `share_doc`, tasks). It never uses a claude.ai connector, an artifact, Docs or any way of sharing outside AI Juggler: the Lab App is signed in to the maintainer's account, so anything made there is theirs (the rc5 walk's P8 scout made a private artifact on it). Every prompt that asks a session to message or share names the pool's tool. Before the first Lab session of a walk, give the Lab App's launch settings the environment `ENABLE_CLAUDEAI_MCP_SERVERS=false` (claude.ai's connectors off for its sessions) and check with `/mcp` in one session that no claude.ai connector is listed; if it can't be set, stop and say so.
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

## Incidents and the click guard

**2026-11-07, P1 step 5, about 11:40 to 11:50 (local).** AI Juggler Dev's window (pid 10642) left the active Space mid-step. Three of the driver's clicks were sent by screen point through System Events' global "click at": about (523, 108), (403, 355) and (1433, 201), meant for Pools, the pool card and Tasks. They landed in the frontmost app, the Claude app, not Dev. What they hit there is unknown (the driver didn't look inside it, and nothing of the Claude app was read). The maintainer was told at once. In the same step, a prefix-matched press ("Cancel") opened "Cancel task" on aj-79f4; "Keep it" was chosen and the task wasn't cancelled.

**The guard, from then on (`guardclick <pid> <x> <y>` in the walk's scratch tools).** No click by screen point is sent except through it. Before each click it reads, at that moment:
- the frontmost app (NSWorkspace) is Dev's pid;
- the topmost on-screen window at the point, on the active Space (CGWindowList, on-screen only), belongs to Dev's pid.

If either fails, the click is refused, the reason is printed, and the driver stops and reports. Tested dry on 2026-11-07: inside Dev's window, OK; a point outside it (another app's window), refused; a pid that isn't in front, refused. Presses go by exact title only, never a prefix.
