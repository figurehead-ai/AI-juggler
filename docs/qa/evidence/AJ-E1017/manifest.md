# Evidence manifest — AJ-E1017

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1017 (the session
view leads with where and what, D-25). Status: **DONE-EVIDENCED**.
2026-10-19, reviewer claude-opus.

## What exists now

- The session header is the session's place ("feat/read-model", else the
  folder) with its state, and a line: "ai-juggler · Work Max · started 8h
  ago" (plus "18% left" when low). The top bar shows the project.
- **Switch account…** sits beside Stop. The ranked list folds behind
  "Continue on another account". It opens by itself when the session hit
  its limit or a way back is on offer. Switch account… and ⌘⇧H open it,
  scroll to it and focus its summary (the shortcut was always "focus the
  hand-over section").
- The summary card keeps only when it ended and the identity check; the
  state and account live in the header.
- The six tabs are unchanged (D-06).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 header and subline | vitest `inspector-shell.test.tsx` (title is the place; the subline names the account and how long; Switch account… present) | PASS |
| Req.2 the folded list | vitest (closed by default, opened by Switch account…, open for a quota-blocked session); `keyboard-flows.spec.ts` reaches the first ranked row with ⌘⇧H and Tab; the switch, door and cross-machine captures open it through `openSwitch` | PASS |
| Req.3 no repeats | `inspector-instance.test.tsx` (how long reads from the header line) | PASS |
| Req.4 tabs | `inspector-shell.test.tsx` six-tab assertions unchanged | PASS |
| Full suite | vitest 466/466; Playwright 909/909 | PASS |
