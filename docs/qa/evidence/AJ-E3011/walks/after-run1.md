# AJ-E3011 newcomer walk, after the redesign: run 1 (2026-11-15)

- **Build:** release/0.4.6 at 340362186, from a clean snapshot of HEAD so no uncommitted work is in it. The P10 lab (seed state) through the lab write-mode bridge, in the built-in browser at 1280×800.
- **Tester:** the same brief and persona as before, a fresh Sonnet tester, browser tools only. Firewall checked: 47 calls, all `mcp__Claude_Browser__*`.

## Path and scores

| # | Step | Worked | Value | Clarity | Feeling | Key question |
|---|---|---|---|---|---|---|
| 1 | Pools › New pool: the eight steps and the templates | 5 | 4 | 4 | 4 | New pool yes; the template cards' chips ("Lead", "talk: allowed", "member↔member: ask first", "sealed", "level: Read") **no** |
| 2 | Start: "Start from scratch" (already chosen) › Next | 5 | 3 | 4 | 4 | yes ("A name and its projects make a working pool") |
| 3 | Where: "web team", web chosen | 5 | 4 | 5 | 5 | yes, all |
| 4 | Who's in: tick Claude Code CLI · Work and Codex CLI · Personal | 4 | 3 | 3 | 3 | checkboxes yes; the tabs "Tools / Macs / Accounts" **no**; "In this pool 3" (counts "Everyone in web") and "can act in the pool: 0 full, 0 shell only, 0 watched" **no** |
| 5 | Who leads: Claude's next session (from "No lead for now", preselected) | 4 | 4 | 3 | 4 | radios yes; "Lead" confirmed by "plans the work and hands out tasks"; "next session" odd: "I wanted in charge now" |
| 6 | Everyone else: Codex › Reviewer (each role explained in its menu) | 5 | 4 | 4 | 3 | role menus **yes**; Claude's own row still "Member" while the Lead card says Claude: confusing; the role cards look clickable and aren't; "Same as the pool (Work together)" **no** |
| 7 | Who may message whom (kept) | 4 | 3 | 3 | 4 | partly; Allowed/Ask first/Blocked yes |
| 8 | On their own (kept Work together) | 4 | 4 | 3 | 4 | partly: it talks about messaging, not what they may do to my files; "wake each other" **no** |
| 9 | Review in sentences with Change links › Make pool | 5 | 4 | 4 | 4 | yes |

**Result:** **S** (no detour). Ease 5/7, value 5/7 (before: 3, 3, 4). Problem solved: partly ("the shape is right … but I can't see it do anything yet", "what they may do without asking is unclear"). Feeling +1 ("guided, wordy, hopeful"; before −1, −1, 0).

**What the tester thinks the pool does** (correct): Claude's next session in web leads and hands out tasks; Codex reviews and doesn't build; everyone may message everyone, an Observer may not send; anything beyond messages, notes and tasks waits for the user's OK; not sealed; nothing runs until sessions start.

## Opus verification (2026-11-15)

Checked on the lab's screen after the run: the pool "web team" (scope web), a Lead card "Claude Code CLI · Work · work's next session — Leads once it starts here", an App card "Claude Code CLI · Work · work" as **Member**, Codex as Reviewer, "Everyone in web" as Member, each App's "Its sessions read messages on their next turn". **Confirmed.** The double Claude (the Lead placeholder plus the App as Member) is the confusion in step 6.

## Still "no" → round 2 of the design (AJ-E3011)

1. **Claude appears twice** (Lead placeholder and App "Member"), and its row in Everyone else says Member. The App leading next should read as one thing: "Claude Code · Work: its next session leads; its others join as Member".
2. **"On their own" must say what it doesn't cover:** this sets what they do with each other through AI Juggler (messages, notes, tasks); what each may change in your files stays with its own app's permissions.
3. **Who's in:** the count includes "Everyone in web" without saying so; "can act in the pool: full / shell only / watched" is jargon; the Tools / Macs / Accounts tabs are unexplained.
4. **The role cards look like buttons and aren't** (either assign from them, or don't style them as controls).
5. **The member's second control "Same as the pool (Work together)"** needs its label ("On its own:").
6. **The template cards' chips** use the old jargon ("talk: allowed", "level: Read", "member↔member: ask first").
7. **"Wake each other"** needs its example visible, not only behind "What does this mean?".
8. "No lead for now" preselected while Apps are in: preselect the first App's next session? (see round 2).
