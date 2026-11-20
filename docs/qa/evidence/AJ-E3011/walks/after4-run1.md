# AJ-E3011 done-gate walk, after round 4: Sam through the diagram path (2026-11-16)

- **Build:** release/0.4.6 at 95d240c7f; both the UI (vite, the snapshot's dev server, restarted) and the bridge are from a clean snapshot.
- **Lab:** P10, seed state.
- **Tester:** a fresh Sonnet tester with the same persona, pointed at the project, using browser tools only.

## Result

**S** (no detour). Ease 5/7, value 5/7, solved partly, feeling +1 ("guided, slightly unsure").

| # | Step | Worked | Value | Clarity | Feeling | Note |
|---|---|---|---|---|---|---|
| 1 | Projects › web: its Pools tab, "web's own pool: every session in web, as one team" | 5 | 4 | 4 | 4 | the line explained the word |
| 2 | Open the pool | 4 | 3 | 3 | 3 | ~2–3 s with no feedback on the dev server (0–80 ms measured in the bundled Dev window) |
| 3 | D-30's question card › "Yes, let them work together" | 5 | 4 | 4 | 4 | what it does: yes. Whom it affects: partly ("your pools" vague; "per project or app in Settings" helped). |
| 4 | Add Claude, then Codex | 5 | 4 | 5 | 5 | the toasts "…is in the pool · Its sessions here join by themselves, now and later" |
| 5 | Who leads › Claude's next session | 5 | 4 | 4 | 4 | "Pick one to be in charge" confirmed it; the card reads "Lead when a session starts" |
| 6 | Codex › Its role › Reviewer | 5 | 5 | 5 | 5 | "Reads everything, checks work in review and gives its verdict. Doesn't build." |
| 7 | The header's "On their own" › Custom | 3–4 | 4 | **2** | 3 | Custom changed only the label ("they may Custom"); the per-action list was under the side panel's Change |

**Ready banner** read correctly: "This pool is ready. What happens next: Open Claude Code CLI · Work · work in web: it leads; Codex … checks its work; their messages show here."

## Opus verification

Firewall checked: browser tools only. The end state the tester read off the screen is consistent with the build's round 4: the question answered, the team tools on, Claude "Lead when a session starts", Codex Reviewer, Custom with "Give a task to someone else" ticked, and the ready banner. Confirmed.

## What holds it at 5/7

1. **Custom in the header does nothing visible**, and "they may Custom" reads like a bug. Fix: choosing Custom opens the per-action list in place (the same shared control as the inspector's), and the sentence reads "they may do the 11 things you picked".
2. **Scope of the question card:** "your pools" is vague. Fix: "This is for every pool you make; you can switch it off for a project or an app in Settings."
3. **"Which do I open first?"** Fix: the ready line starts "Open Claude Code CLI · Work in web first: it leads…".
4. **Not design: the environment.** The dev server compiles each screen on first open (2–4 s); in the bundled Dev window the same clicks measured 0–80 ms. The walks will be served from a bundled build with the bridge in it.
5. **Not design: the brief.** "Can't tell whether they really work together until I start a session": value is capped while the persona may not start one.
