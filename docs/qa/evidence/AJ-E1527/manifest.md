# Evidence manifest — AJ-E1527

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1527, Make it smoother: the
optional permissions on the welcome (P1). Status: **DONE-EVIDENCED**
(IN-REVIEW until the release PR).

Gallery: `docs/qa/evidence/_gallery/01-rail/honest-empty-*` (the welcome
with the card), and every Home capture with an optional permission still
off (the reminder row).

## What exists now

1. **One component for the rows** (`components/OsAccessRows.tsx`): each
   permission's state and its Allow, Open System Settings or Reopen
   action (`lib/osAccess`'s cards, #120's Reopen), re-checked when the
   window comes back. First run's access step (`AccessStep.tsx`), the
   welcome's card and Home's reminder all use it, in a large or a compact
   size.
2. **The welcome's "Make it smoother" card** (`components/SmootherCard.tsx`):
   every permission still off, with "Optional: AI Juggler works without
   them." Never required. "Not now" puts it away for good, and Home's
   reminder with it; AI Juggler then asks only when something needs a
   permission, in context (the focus action's own prompt, as before).
3. **Home's reminder**: after the welcome, while a permission is still off
   and not put away, one compact row for the first one, with its action
   and its own "Not now" (that permission only). Never a modal.
4. **Screen Recording's Reopen** comes through the same rows. Screen
   Recording itself is listed only once a tool can show a live preview
   (the docs fact-check, 2026-10-22), so today the rows hold "Jump to a
   session's own window" alone.
5. "Not now" is kept in the browser's storage (`aij.smoother.dismissed`),
   like the app's other dismissals; a private window forgets it.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 the card, its rows shared with first run | vitest `smoother-card` (the rows, Allow asks the system, hidden when all allowed), `first-run` (the access step through the same rows); gallery `01-rail/honest-empty-*` asserts the card | PASS |
| Req.2 never required; Not now | vitest `smoother-card` (the welcome's Not now hides the card and the reminder, remembered) | PASS |
| Req.3 Home's dismissible reminder, never a modal | vitest `smoother-card` (the first missing one; its Not now puts away that one only); Home's captures | PASS |
| Req.4 Reopen through the same component | `OsAccessRows` carries the Reopen state and its block; `first-run-access-reopen` vitest and the journey `first run — Screen Recording needs a reopen` pass through it | PASS |

## Walked

On the preview at 1280×800, both as a brand-new user (no accounts: the
welcome with the card under its two buttons) and after (Home with the
reminder row above Needs you). "Not now" on the reminder hid it and
stored `["focus"]`; the welcome's stored `["all"]` and hid both.

1. **Does it solve the real problem?** The permissions that make it
   smoother are seen early, without blocking anything.
2. **Is it clear without explanation?** "Optional: AI Juggler works
   without them." The row says what it's for and what the system calls
   it.
3. **Is it consistent with its siblings?** The same rows as first run's;
   the reminder is one row, like Home's other rows, not a second card.
