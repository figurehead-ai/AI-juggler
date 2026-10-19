# Evidence manifest — AJ-E1013

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1013 (first run,
rebuilt on the UX vision, D-25). Status: **DONE-EVIDENCED**. 2026-10-19,
reviewer claude-opus.

## What exists now

- **First run opens by itself** when there is no Account: full window, on
  what the scan found — grouped as "Claude desktop app", "Claude Code in
  the terminal" and "Other setups", each row with its name, how recently
  it was used and whether it is signed in. Rows signed in or used in the
  last 30 days are preselected; one already in AI Juggler is shown as
  such and cannot be picked twice. One button counts the picks ("Bring in
  N accounts"); the copy says they are copies and the originals stay as
  they are. "Where we looked" lists every probe and what it found.
- **Sign in**: one row per copy that still needs the vendor's sign-in
  (name, status, Sign in; the login command and "Sign in later" behind
  More), refreshed every 2 s, and "Sign in to all N, one after another",
  which moves to the next row by itself.
- **Computer access** (new backend `os_access`): the list comes from the
  platform. macOS: "Jump to a session's own window" (Accessibility and
  Automation) and "See a live preview" (Screen Recording). Each entry has
  one line of why, its live status, Allow (the system prompt) or, once
  declined, Open System Settings at the right pane. The list is refreshed
  every 1.5 s, so a change made in System Settings shows at once. Where
  the OS needs nothing the step is skipped. The probes use the safe
  `core-graphics` and `macos-accessibility-client` crates (no `unsafe` in
  our code; `cargo deny` clean).
- **No pack-secrets question in first run**: the first switch asks while
  no choice exists (D-14 unchanged).
- The older wizard stays for Accounts › Add account.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 grouped, preselected, one button, where we looked | vitest `first-run.test.tsx` (groups, names, preselection, words, the count, where we looked, nothing found); journey `journey-first-run.spec.ts` (terminal default plus five Claude desktop profiles: "You already have 6 Claude Code setups", "Bring in 6 accounts") | PASS |
| Req.2 sign-in rows, polled, Sign in to all | vitest (rows after bringing in, Sign in opens the login terminal titled "Sign in · …", Sign in to all offered for more than one) | PASS |
| Req.3 computer access per platform | `juggler-app --test os_access` (3: the list per platform and its shape, declined → settings, unknown id refused; none of them prompts); vitest (Allow asks the system and flips to Allowed; declined opens settings; empty list skips the step) | PASS |
| Req.4 no pack-secrets question | the journey reaches Home with no policy question; `first-run-policy` suite unchanged | PASS |
| Budget | the journey: at most 5 primary clicks and nothing typed from opening to Home | PASS |
| axe | the journey scans each step (what's here, sign-in, computer access, Home): 0 critical/serious | PASS |
| Gallery | `66-first-run`: found, where we looked, after bringing in, access, one allowed, Home after | PASS |

## Not covered here

- The real system prompts: tests never press Allow against the OS (it
  would raise a real dialog). The grant flow on a real Mac is not yet
  checked by hand; it is on the maintainer's dogfood list.
- The maintainer's report that the installed app found only the terminal
  setup is not reproduced: the same scan finds all five desktop profiles
  from a shell and from a launchd-started app. "Where we looked" now shows
  every probe's result on screen, so the next report can say which one
  came back empty.
