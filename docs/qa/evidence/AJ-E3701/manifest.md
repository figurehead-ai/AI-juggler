# Evidence manifest — AJ-E3701

Task: AJ-E3701 (Home in two columns, E37, D-118). Status: **IN-PROGRESS**:
the layout is built and unit-tested; captures, the a11y pass and the persona
walk are pending.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 two columns: the pinned zone and feed in the left column, Your apps and Runway in an `aside` "Apps and runway" on the right | vitest `ui/src/screens/home-places.test.tsx` "the pinned zone and the feed share the left column; Apps and Runway the right (D-118 R1)" | feed and pinned zone inside `home-main`; runway inside `home-side` (an ASIDE, aria-label "Apps and runway") | pass | PASS | 8cf35fecb | 2026-11-15 | Claude (self) |
| R1 sizes: feed `minmax(480px, 1fr)` capped at 720 px, right column `clamp(320px, 30%, 400px)`, 24 px gap, each column its own scroller with the scroll-edge hairline | CI capture of Home at 1024, 1280 and 1440, plus a computed-style check in `ui/tests/visual/home-gallery.spec.ts` | the measured widths and gap match R1 | jsdom has no layout; not run yet | PENDING | — | — | — |
| R2 Your apps: a compact row per running app and profile, most recent first, count, needs mark, Open; past 8 "Show all"; nine show all nine | vitest `ui/src/screens/home-places.test.tsx` "Your apps (D-118): a compact row per running app and profile, most recently used first, with its count, needs mark and Open; eight, then Show all" | rows, `n open`, `1 needs you`, Open button; 10 places give 8 rows and `Show all 10`; 9 give 9 | pass | PASS | 8cf35fecb | 2026-11-15 | Claude (self) |
| R2 Your apps: the muted "Not running" group, a ghost Start for a closed app, the App tone icon | Home capture with a closed app in the fixture (`home-gallery.spec.ts`); a vitest for the group if the captures show a gap | closed apps listed last under a muted group, Start offered | no committed test names the group | PENDING | — | — | — |
| R2 Runway by account: each Account's rows with its profiles; "Not measured yet" and Measure its limits; the Out chip rather than "0% left"; an App's row never repeats the runway | vitest `ui/src/screens/home-places.test.tsx` "Runway by account: each with its profiles; the way to measure one that isn't", "Runway by account groups the profiles signed in to one account", "an App's room is Runway by account's alone (D-118): its row never repeats it"; `ui/src/components/home-runway-out.test.tsx` "an Account with nothing left shows the Out chip, not \"0% left\"; a low one keeps its figure" | rows, Measure button, one row for shared profiles, no runway words in app rows | pass | PASS | 8cf35fecb | 2026-11-15 | Claude (self) |
| R2 Runway colours: amber at 10 % or less with a live session, red when out with live work | Home capture with the amber and out fixtures (`home-gallery.spec.ts`), checked against D-117's rules | amber and red only in those two states | only the Out chip is unit-tested; colours not captured yet | PENDING | — | — | — |
| R3 Your apps, Runway and the pinned zone fold, the state kept per viewer in localStorage (try/catch), chevrons as D-117 point 3 | Built by `HomeSection` and `useFold`; proof is the attention-budget spec's chevron check (`ui/tests/visual/attention-budget.spec.ts`, item vii) and a fold-and-reopen step in the Home capture; add a vitest for the kept fold if the capture run shows none | each section toggles `aria-expanded`, reopens as left, chevron visible at rest and focused | no committed test folds a Home section yet | PENDING | — | — | — |
| R4 stacking: under about 1,000 px (threshold 960 px of main area) the feed comes first, one page of 12 then Show more with no automatic paging, "Apps & runway ↓" in the head, a strip of App icons under 640 px | vitest `ui/src/lib/feed-rollup.test.ts` "AJ-E3701 R4 — Home's layout by its main area's width"; `ui/src/screens/home-places.test.tsx` "stacks under about 1,000 px, the feed first, with Apps & runway ↓ in the head; a strip of App icons under 640 (AJ-E3701 R4)"; `ui/src/components/home-feed.test.tsx` "stacked: one page's first 12 cards, then Show more; never paging by itself" | homeLayout: 1280 and 990 columns, 959 and 900 and 740 stacked, 639 strip; feed first in document order; 12 cards, then 30, one request | pass | PASS | 8cf35fecb | 2026-11-15 | Claude (self) |
| R5 no Accounts keeps the Welcome and first run; the rail's Needs you is unchanged | vitest `ui/src/screens/home.test.tsx` "no accounts: the welcome, with Find my apps first" proves the Welcome; the rail's count by `ui/src/count-agreement.test.tsx` and the rail gallery staying unchanged in the CI diff | Welcome shown; no rail baseline changes | Welcome test is committed; rail baseline diff not reviewed yet | PENDING | — | — | — |
| R6 Home's calm pass: loud-element audit, calm look, attention-budget test on Home's fixtures, contrast and axe in both themes | `ui/tests/visual/attention-budget.spec.ts` (all-clear, typical, busy, flood) and `ui/tests/visual/a11y.spec.ts` on CI; the design-quality critique; before and after captures on the review page | one loud mark per need; axe 0 critical or serious; CI visual and a11y captures (Playwright, both themes), then the review page | specs updated in f842c9cf6, not run on CI yet | PENDING | — | — | — |
| R6 a Sonnet persona glance and walk by hand, verified by Opus | persona walk of Home as the multi-subscription vibe-coder, scored for success, value, clarity and feeling | glance and walk recorded; low scores to the maintainer | not run | PENDING | — | — | — |
| Performance: layout paints from cached data in 100 ms or less; columns at 60 fps; a fold in 16 ms or less | a timed Playwright step in the home gallery plus `ui/tests/visual/idle.spec.ts` (Home draws only on a change) | budgets met; idle green | not run | PENDING | — | — | — |

## Design reference (not evidence)

The Home feed mock the maintainer approved on 2026-11-14 (D-118), rendered
from `docs/research/home-feed/mock-home-feed.html` with generic fixture
names only. Each image shows the wide layout (1280 px window) and the
stacked narrow one (about 900 px). Opened and checked by eye on
2026-11-14 before it was committed.

- `mock-home-feed-dark.png`: dark theme.
- `mock-home-feed-light.png`: light theme.

## Notes: design calls taken in the build (D-118)

- Stacking threshold: 960 px of the main area, so a 1280 px window with the
  rail open keeps two columns. Below 640 px Your apps is a strip of icons.
- Level-1 notes left Home's needs zone: "to review" lives under the feed's
  Finished chip, leftovers under Clean up.
- The feed's chips are Home's only filter row.
- Built in 8cf35fecb; the Playwright specs followed in f842c9cf6 and have not
  run on CI yet. Every PENDING line waits on those captures, the persona walk
  or the review page.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
