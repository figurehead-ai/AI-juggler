# Evidence manifest — AJ-E1023

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1023 (choosing a
tool inside Add account, D-25). Status: **DONE-EVIDENCED**. 2026-10-20,
reviewer claude-opus.

## What exists now

- Accounts has one Add account button (the separate "Add Provider" is
  gone).
- With more than one tool installed or found-but-unverified, Add account
  first shows "Add an account — Which tool is it for?": one card per tool
  with its install state, verified range, kind, a desktop tool's
  one-session limit and Install → for missing ones. Choosing a card opens
  Add account for that tool: its scan, a fresh account named after it
  ("Codex"), its sign-in.
- With one tool, Add account opens directly.
- `AddProviderSheet` is now that chooser (`onChoose(provider)`), a
  labelled modal dialog that takes focus and closes on Escape. `FirstRun`
  takes a `provider` and says the tool's name in its words.
- No adapter code changed (the Codex adapter is another session's work).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 the choice | vitest `add-provider-sheet.test.tsx` (cards, never-selectable missing tools, onChoose with the tool, Escape, dialog name); `flows-cde.spec.ts` flow (c); capture `38-flows-cde/c-provider-cards-*` via Add account | PASS |
| Req.2 one tool | `flows-cde.spec.ts` flow (d): no cards, straight to a fresh account | PASS |
| Req.3 dialog, budget | a11y "Add account's tool choice": axe 0, Enter on the Codex card opens Add account for Codex (name "Codex"), axe 0 again; flow (c) = 4 primary clicks to the sign-in, funnel `c-add-provider:open` and `:choose` | PASS |
| Full suites | counts in the PR | PASS |

## Not covered here

- A real second tool: with only Claude Code installed on this Mac, the
  choice is exercised through fixtures until the Codex adapter lands.
