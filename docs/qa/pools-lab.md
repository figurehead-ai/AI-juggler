# Pools labs (AJ-E3002)

Which of D-62's talk rules AI Juggler can actually enforce, from live runs.
A "no" is shown in the app as not enforced, never drawn as if it were.

## 1. Held and refused messages between sessions AI Juggler started — YES

Enforced by the agent bridge for each receiving session and each pool.

- Live, 2026-11-07, P2 on AI Juggler Dev with the Lab App (ADR-0027 7b):
  four Claude Code sessions in one pool, member → member "Ask first".
- w1's message to w2 was held. Claude said the message "stays queued until you deliver it
  in AI Juggler". Allow once delivered it to w2's inbox. Deny left w3's copy
  "Not sent", and it never reached w3. Record: `docs/qa/0.4-walks/P2.md`, steps 1–2.
- Codex through app-server: P1 step 5 (`docs/qa/0.4-walks/P1.md`).

## 2. Claude Code's own cross-session messages — YES, from the launch (re-walk pending)

Claude Code (v2.1.224 and later) lets its sessions on one Mac message
each other with its own `SendMessage` and `ListAgents` tools, over a
per-session socket that never passes through AI Juggler.

- **Before (live, 2026-11-07, P7 step 3):** a session in another pool used
  Claude's own SendMessage on c1 in the sealed Client pool. c1 received
  it and ran it: `"echo lab" to client session → sent to client-99`.
  AI Juggler's bridge never sees that channel.
- **The documented settings** (code.claude.com/docs/en/cross-session-messaging):
  - `crossSessionInbound`: `"accept"`, `"hold"` or `"refuse"`. `refuse` drops other sessions' messages.
  - `permissions.deny: ["SendMessage", "ListAgents"]` stops the session reaching others.
  - A value from `--settings` takes precedence over the user's own settings.
  - The Claude desktop app's Code tab can't show the hold dialog: a held message waits until `dialogExpiry`. `refuse` also drops the desktop surface's messages.
- **Built (57f89e1bb, a3903836e):** a Claude Code launch in a sealed pool gets both keys in a per-launch settings file in AI Juggler's overlay folder.
  - It is the launch's last step (the adapter's `sealed_launch`), and it applies whether or not the agent tools are on.
  - The file holds every `--settings` before it (the Account's overlay, the hooks), so it reads the same whether Claude merges several or keeps the last.
  - It applies from the session's next start; a running session keeps its launch's settings.
  - Other apps' sessions are Watched and aren't covered.
  - The wizard says so.
- **Still to run:** P7 step 3 again on the RC build (expect: refused).
- **Pair rules other than sealed:** Ask first or Blocked between two members can't be set on Claude's channel per pair, because the setting covers the whole session. Pair rules hold on AI Juggler's channel only.

## 3. OpenCode and Grok Build tools per launch — NOT RUN

- OpenCode's adapter is parked (E1602), and no Grok Build is on this Mac.
- Kimi Code stays on its shell.
