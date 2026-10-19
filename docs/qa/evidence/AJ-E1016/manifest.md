# Evidence manifest — AJ-E1016

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1016 (Accounts
with runway, Usage, the limit decision, D-25). Status: **DONE-EVIDENCED**.
2026-10-19, reviewer claude-opus.

## What exists now

- **`AccountRunway`** (shared by Accounts and Usage): the 5-hour window
  and the week, each with a bar and words ("58% left · resets Sun 04:13",
  "Used up until …", "Measuring — shows after a little use", "Not
  measured yet"). With no account reading, the newest session's own quota
  line stands in, labelled "From the latest session". "At this pace it
  runs out in about 2 h" appears only when the forecast says it will.
- **Accounts**: the summary names the best account for new work
  (signed in, most left in its tighter window). Each card shows what runs
  on it ("3 running · 2 need you"), its runway, and Start a session /
  Edit launch settings. "Needs sign-in" and Sign in show only when needed.
  A disabled Start says why. Pack secrets is a link that opens Settings
  at Context packs & secrets; the Beta matrix sits behind a disclosure.
- **Usage**: every account's runway, then what sessions used. With no
  accounts: what will appear, and Find my accounts.
- **The limit decision** (`LimitDialog`, from Home's "Continue
  elsewhere"): when it resets and that nothing is lost; Continue now on
  the ranker's pick (Recommended when suggested, its runway, the pair's
  badge and, on request, what doesn't carry over), which hands off to the
  existing switch flow; Wait for the reset (parks through AJ-E1003's
  `instance_park`, then says when it comes back); Choose another (the
  session view). A modal dialog with focus inside, Escape and Not now
  close it, and nothing happens until a choice.
- Settings accepts a section to open at (`aij:open-settings` with a
  `section`).

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 runway words | vitest `account-runway.test.tsx` (window words, used up, learning, unmeasured, the session fallback, the pace line) | PASS |
| Req.2 best pick, activity | vitest (`activityWords`, `bestForNewWork` skips signed-out accounts and takes the tighter window) | PASS |
| Req.3 link and disclosure | a11y and capture suites follow the link into Settings (the choice group is there, axe 0) and open the matrix disclosure | PASS |
| Req.4 Usage | `accounts-gallery.spec.ts`: Usage with accounts and with none | PASS |
| Req.5 the decision | vitest `limit-dialog.test.tsx` (5: the pick, the reset in words, Continue hands on the target, what carries over, Park parks and confirms, Escape does nothing); the gallery opens it from Home, focus lands on Continue, Escape closes it | PASS |
| axe | `accounts-gallery.spec.ts`: 0 critical/serious on all 10 states | PASS |
| Full suite | Playwright 907/907; vitest 461/461 | PASS |

## Not covered here

- The real switch and park on a real account: the dialog hands off to
  the switch flow (AJ-E206) and the park command (AJ-E1003), which carry
  their own evidence; a dogfood run of an actual limit is pending.
- The limit decision is reached from Home. The rail and the session view
  still open the session, whose switch door offers the same targets.
