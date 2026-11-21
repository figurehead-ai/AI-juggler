# AJ-E1612 steps 2-6 on AI Juggler Dev's own Apps (0.4.6 build)

Run 2026-11-20, UTC, by the Sonnet walk driver (Opus checks each result). Dev: pid 98313, the 0.4.6 soak build, real HOME, Dev's own data folder. Driven only through `scripts/qa/lab_drive` (frontmost+topmost guard, no waiting needed, no PAUSED). No T16 lab: the Apps are Dev's existing ones. Every result is "0.4.6 build"; the 0.5 re-check comes later. Driver log: `dev-046/drive-log/lab_drive-dev046.log`. Project used: `~/lab/web`. Prompt: "reply with the word lab".

Apps found (Accounts page, read only): `OpenCode CLI · OpenCode · R3` (folder opencode-27633337000), `Grok Build CLI · R3` (folder grok-build-752866214000). Both show "Account unknown · this Provider exposes no account metadata". The OpenCode App sits behind "Show 1 more" in the rail.

| Tool | Step | Result | Time (UTC) | What showed | Capture |
|---|---|---|---|---|---|
| OpenCode | 2 session starts | PASS (0.4.6 build) | 21:18:06 start | Home: no "needs its sign-in" row (signed in). New session on the App in ~/lab/web started; terminal answered "lab" (after a skill call "aij-lab-probe" and 9 s). Header "Working" stayed after the turn ended. | dev-046/opencode-2-session.jpg |
| OpenCode | 3 resumes | PASS (0.4.6 build) | 21:19:43 stop, 21:20:06 resume | The page's own Stop (held 5 s with Undo) ended the process (pgrep: none), page said "Stopped by you 4:19 PM", Resume appeared. Resume brought the same session back (`opencode --session ses_e052...`) with its first turn on screen. Second "reply with the word lab" got the answer "PEREGRINE", not "lab": the model's answer (tool/skill), not AI Juggler. Resume itself works. | dev-046/opencode-3b-resumed.jpg, opencode-3c-second-turn.jpg (no capture of the 5 s Stopping state; the first attempt's shot command failed on shell quoting) |
| OpenCode | 4 usage read | PASS, with a note (0.4.6 build) | 21:21:09 global, 21:21:58 session | Global Usage page: OpenCode listed under "Account unknown", 0 tokens in the last 7 days there; by tool 149k tokens, "No list price"; "OpenCode doesn't report its limits to AI Juggler yet". Session Usage tab: 32k new tokens, 3 turns, 15k cache read, "No list price", model opencode/big-pickle, "Its account's window: OpenCode doesn't report its limits to AI Juggler yet". It does not say "not measured"; it says "doesn't report its limits yet". NOTE (possible 0.5 finding): "Its share of the limits" shows This 5-hour window 100% and This week 100% (32k of 32k) while no limit is known; that reads as a full window and may mislead. Source line on the page: "from usage tracking". The global Usage page shows real account emails, so it was read as text only, not captured. | dev-046/opencode-4-usage-session.jpg |
| OpenCode | 5 quota exhaustion | not seen | 21:22 | Never caused; nothing showed. | none |
| OpenCode | 6 lab B stays out | n/a: no lab B App on Dev's folder | | No stand-in App made. | none |
| Grok Build | 2 session starts | SKIPPED: credential step for the maintainer | 21:17 | Home's Needs-you has "Grok Build CLI · R3 needs its sign-in" (read-label: not-signed-in) although the App folder holds an auth.json (name only, content not read). "New session" opened the sheet; no session started (sheet closed with Escape). | none |
| Grok Build | 3 resumes | SKIPPED (depends on 2) | | | none |
| Grok Build | 4 usage read | not run in a session. The global Usage text says "Grok Build doesn't report its limits to AI Juggler yet", 0 tokens (last 7 days), Account unknown. | 21:21:09 | text only | none |
| Grok Build | 5 quota exhaustion | not seen | | | none |
| Grok Build | 6 lab B stays out | n/a: no lab B App on Dev's folder | | | none |

Credential step for the maintainer: Grok Build CLI · R3 shows "needs its sign-in" (Sign in to Grok Build CLI · R3 on Home). Not signed in by this walk.

Cleanup: the OpenCode session was stopped with AI Juggler's own Stop (pgrep for `bin/opencode`: none left); pid 98313 still running (elapsed 06:54). The session row "New session - 2026-11-20T21:18:49.830Z" stays in Dev's History as ended. Kept: this file, the four captures (each opened and confirmed), the driver log.
