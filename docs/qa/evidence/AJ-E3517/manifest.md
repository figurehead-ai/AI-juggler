# Evidence manifest — AJ-E3517 Hover cards

Built in 11027899a (card text, driver timing, overlay card, Go to it in the
snapshot; R4 amended there: the question and step are read when the card
opens, never carried in snapshots). Captures drawn offscreen (Metal); the
recording of both buttons is the Dev walk in `../AJ-E3516/walk.md`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 after 250 ms: state, title, question / step / idle, project, subagents | `cargo test -p juggler-floor driver::tests::a_person_s_card_opens_after_a_rest_stays_while_near_and_closes`; `card-*.png` (`AJ_FLOOR_EVIDENCE=… cargo test -p juggler-floor-render --test golden hover_cards_stay_on_screen_with_their_buttons`) | none before 250 ms, open at 250 | test: no card at 200 ms, open at 100 + 250 ms; captures: "! ASKS", the title, "ASKS: SHOULD I ALSO UPDATE THE SNAPSHOT TEST BEFORE I PUSH?", "WORKSHOP TOOL · WORK · SHOP · 1 TASKS" (asks); the latest step for working, stuck, crashed, done, limit; the idle time for idle | PASS | 11027899a | 2026-11-06 | Claude (self) |
| R2 stays on subject, card or the way between, +300 ms; Esc closes | same driver test | stays while on the card; not closed before 300 ms; Esc closes | stays at 1000 ms with the pointer on the card; still open at +290 ms off it, closed at +300; Esc closes it | PASS | 11027899a | 2026-11-06 | Claude (self) |
| R3 Open in AI Juggler; Open in the tool's app only where Go to it can | same driver test (clicks give Effect::Open / Effect::GoTo); `hover_cards_stay_on_screen_with_their_buttons` | Open always; Go to only with `go_to` | test passes for every sitter state at several points of a 1280×800 and a 640×400 window (cards inside the window, buttons inside the card, "Open in its app" only when `go_to` is set); card-asks shows OPEN IN AI JUGGLER and OPEN IN CLAUDE APP | PASS | 11027899a | 2026-11-06 | Claude (self) |
| R4 question ≤ 120 and step ≤ 160 read on open through the last-messages reader | `cargo test -p juggler-app --lib features::floor::card::tests::a_card_says_the_question_or_the_latest_step_within_its_length`; `FloorSession.go_to` only in the snapshot | cut at 120 / 160 with "..", nothing stored | passes: question cut at 120, step at 160; a reply after the question hides it; the snapshot carries `go_to` only | PASS | 11027899a | 2026-11-06 | Claude (self) |
| Gate: feed tests for the new fields | `cargo test -p juggler-app --lib features::floor` | all pass | 25 passed | PASS | 11027899a | 2026-11-06 | Claude (self) |
| Gate: card layout test (stays on screen) | `hover_cards_stay_on_screen_with_their_buttons` | inside the window at every size | passes (1280×800 and 640×400, every sitter state, subjects near each edge) | PASS | this commit | 2026-11-06 | Claude (self) |
| Gate: the a11y list carries the same words | the Floor list (`ui/src/pages/floor/FloorList.tsx`, AJ-E3512): `npx vitest run src/pages/floor` ("resting on a row reads what it asks…"); gallery `99-floor-list/said-*`; `cargo test -p juggler-app --lib features::floor::card` | the card's words in the list | the list had each session's state word, title and project but not the card's question or step; now resting on a row (focus, or the pointer for 250 ms) reads that one session's words through `floor_said` (the card's own reader, its setting and redaction) into a polite bar at the list's foot ("Checkout copy · Asks: …"), gone 300 ms after leaving; nothing read for other rows | PASS | this commit | 2026-11-06 | Claude (self) |

## Binary artifacts

`card-{asks,working,stuck,crashed,done,limit,idle}-{pixel,full}.png` (a
selection, 1280×800 at 2×): the card beside its sitter in each state.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (fixture only)
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (generic fixture: Workshop tool, Studio app…)
