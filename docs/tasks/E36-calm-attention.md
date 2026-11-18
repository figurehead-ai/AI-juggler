# E36 — Calm attention: only what needs you is loud, on every screen (0.5, D-117)

> Source: the maintainer, 2026-11-14, on Home and the rail: they "look like
> a Christmas tree… if users feel everything is a disaster then nothing is
> a disaster". On the round-2 mock: "calm it even more… subheadings and
> borders… more subtle but still readable", and "app icons different color
> tones, so those that have alerts and need attention are brighter… or
> those that are selected". Approving it: "Yes that is good, but don't lose
> the collapsable sections and subagent and session collapsables… I don't
> see them in your designs". On scope: "0.5 should revamp every screen we
> have with this new calm model." Ruled as D-117.
>
> The research (ISA-101, ISA-18.2, EEMUA 191, alarm fatigue, calm
> technology), the audit of today's Home and rail, and the proposal with
> every token and contrast number are in
> [`docs/research/calm-attention/`](../research/calm-attention/). The
> round-2 mock's captures: `docs/qa/evidence/AJ-E3601/mock-r2-{dark,light}.png`.
>
> **The rule this epic builds.** Every fact stays on screen; only its
> loudness changes. Four levels, read from one place (`attentionLevel()`):
> - **L3 Broken:** red, a glyph, at most one tinted block per area.
> - **L2 Needs you:** an amber word and dot, no fills; an inferred
>   ("probably") state draws a hollow ring.
> - **L1 Note:** a quiet glyph beside neutral text.
> - **L0 Calm:** the grey hierarchy (ink, ink-2, section, muted, faint).
>
> Each need is loud **once per area**. One filled primary button per view.
> Nothing loops (0.4.5's `.dot-live` three-iteration rule and
> `idle.spec.ts` stay). Round 2's look: section labels in
> `--color-section`, no card borders, no row dividers, filled controls,
> ghost secondary buttons, App icons muted at rest and in full colour at
> L2/L3, when selected or current, and on hover or focus.
>
> **Every collapsible stays** (D-117 point 3): the rail's sections, App and
> profile group headers, session rows that unfold their subagents, and
> every other disclosure. Chevrons stay, muted at rest, always visible on
> hover and focus, operable from the keyboard. A screen pass that loses one
> fails.
>
> **The screen pass** (AJ-E3604…AJ-E3616; Home's is AJ-E3701). Every screen
> task carries these requirements, in this order, and adds its own after
> them:
> 1. An audit of the screen's loud elements against the budget (the
>    audit's table form: element, condition, kind, loud or not, what it
>    repeats), written before any change.
> 2. The calm pass: tokens, levels and round 2's look applied, every
>    collapsible and chevron kept.
> 3. The attention-budget test (AJ-E3601) extended to the screen's
>    fixtures.
> 4. Contrast and axe, both themes.
> 5. A Sonnet persona glance (5 s and 10 s) and walk by hand, its verdicts
>    checked by Opus before they're recorded.
> 6. The design-quality critique (PRINCIPLES' reviewer checklist, ≥ 4 on
>    every line).
> 7. Before and after captures on the review page, each with a note on
>    what changed.
>
> **Order.** AJ-E3601 and AJ-E3602 first; then Home's new layout
> (AJ-E3701, which carries Home's calm pass); then the rail and the
> screens by traffic ([0.5-order.md](../plan/0.5-order.md) batch 5);
> AJ-E3617 closes the epic before the 0.5 Stable.

### AJ-E3601 The calm foundation: tokens, attentionLevel(), the OS settings, the beta-only classic flag and the validation kit | Status: IN-PROGRESS (R1–R7, R9 done; R8's baseline run and the Increase Contrast captures left: docs/qa/evidence/AJ-E3601/manifest.md) | Priority P0 | Effort L | Depends: —
**Goal**: One source for how loud anything is, the tokens that draw it, and the tests that keep the noise from growing back.
**User story**: As someone with eight sessions on three Apps, I see at a glance the two things that need me, because nothing else on screen is shouting.
**Requirements**:
1. `attentionLevel(item): 0 | 1 | 2 | 3` SHALL live in `ui/src/lib/attention.ts`, beside `attentionWords`, and be the one source the rail, Home, the bell, the tray and the legend read. It is fed by the item's need, state, "probably" flag, runway, This Mac's pressure and notices, with no per-tool branch (the architecture standard). The mapping is the proposal's §(a) table: L3 for stopped with an error or crashed, a limit with no Account that has room, an Account out while a session runs on it, a failed or rolled-back switch, degraded readings, critical memory pressure; L2 for asks you, probably asks you, probably stuck, a limit with somewhere to continue, cut off, a sign-in an App's sessions need, a budget cap hit, an agent's message waiting, ≤ 10 % left on a live session, warning pressure; L1 for done and to review, low runway no live session depends on, leftovers, an update, not measured, renewal within 7 days, kept alive or paused, uncommitted files; L0 for the rest.
2. The tokens SHALL be added to `ui/src/styles.css` under the names agreed with the 0.4 line (which reuses them for 0.4.6's pools): `--color-section`, `--color-fill`, `--tone-saturate`, `--tone-opacity`, `--color-divider`, `--color-count` / `--color-count-ink`, `--color-count-alarm` and its ink, `--level-3`, `--level-2`, `--level-1-done`, `--level-1-live`; the dark status colours retuned to the proposal's §(c) values (danger `#ff8578`, warning `#e8b25a`, live `#7bbfcc`, success `#79c3a0`, danger-soft `#ff857824`), light unchanged; the body's radial washes and the logo glow removed.
3. The App tone SHALL be one CSS treatment (`.app-tone`, lit by `.is-lit`) on the image `AppIcon` renders and, separately, on the `ProfileBadge` dot: muted at rest, full colour at L2/L3, when selected or current, and on hover or focus. Nothing is baked into assets (the run-time icons ruling).
4. macOS Increase Contrast (`prefers-contrast: more`) SHALL raise dividers to hairline and hairline to faint, turn ghost buttons into bordered ones and show App icons at `saturate(.5)` and full opacity; Reduce Motion SHALL turn off the 600 ms entry fade and the tone transition. No setting of AI Juggler's own.
5. A hidden `aij.attention=classic` flag (localStorage, read in a try/catch) SHALL restore the 0.4 look for side-by-side walks during the 0.5 beta only: no Settings UI, no baselines, and a test that fails once the version is a 0.5 Stable while the flag's code is still present (AJ-E3617 removes it).
6. The attention-budget test SHALL exist (`ui/tests/visual/attention-budget.spec.ts`), on the `allStatuses`, a typical and a flood fixture, per area, both themes, reading computed styles: (i) warning- or danger-painted elements ≤ the distinct L2/L3 items plus the Needs you count; (ii) no amber fill; (iii) ≤ 1 `danger-soft` block; (iv) exactly one filled accent button per view; (v) no infinite animation (`document.getAnimations()`); (vi) every row's `data-level` equals `attentionLevel()`; (vii) every chevron of a collapsible present and visible on hover and focus. Each screen task adds its fixtures to it.
7. The colourfulness metric SHALL be a script in the repo (`scripts/qa/colourfulness.py`, Hasler–Süsstrunk M and the loud-pixel share per area), run on the gallery captures with the proposal's §(f) targets (rail loud ≤ 1.5 %, main loud ≤ 1.2 %, rail M ≤ main M + 1, the all-clear captures unchanged).
8. The glance test (5 s and 10 s, both themes; "How many things need you? Which first? Is anything broken? What is running fine?") and the colour-vision simulation (protan, deutan, tritan; L2 and L3 told apart by shape) SHALL be written as a reusable walk sheet in `docs/qa/0.5-walks/calm-glance.md`, with today's build as the baseline run.
9. Contrast SHALL be gated: `ui/tests/visual/contrast.spec.ts` gains the count-badge pairs, `--color-section`, and an APCA check in dark (warning ≥ Lc 60, danger ≥ Lc 52 on every surface); the `lib/mark-hues` tests re-run against the new status hues (ΔE ≥ 0.12).
**UI/UX reasoning**: ISA-101's grey baseline and ISA-18.2's priority by consequence and time to respond (research §2); preattentive pop-out needs a unique feature (research §1.4); WCAG 1.4.1 (never colour alone: the word, glyph and shape carry every level); PRINCIPLES §7 (a colour hierarchy, not one grey band) and §9 (one primary action).
**Performance budget**: `attentionLevel()` ≤ 0.05 ms per item (10,000 items in ≤ 500 ms in vitest); the tone change ≤ 150 ms; no added layout or paint at rest (`idle.spec.ts` stays green).
**Quality gates**: verify-static; the new vitest table covers every row of the proposal's §(a) mapping; contrast.spec and axe green in both themes; the attention-budget test green on its three fixtures; `idle.spec.ts` green.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3601/manifest.md`, R1–R9; the colourfulness report before and after on the gallery's Home and rail captures; the glance sheet's baseline run on today's build (Sonnet, verdicts checked by Opus).
**Visual testing**: the gallery's Home, rail and status states at 1024 and 1280, both themes, with and without Increase Contrast; the classic flag's captures only on the review page, never as baselines.
**Files to touch**: `ui/src/lib/attention.ts`, `ui/src/styles.css`, `ui/src/components/AppIcon.tsx`, `ui/src/components/ProfileBadge.tsx`, `ui/src/components/RailHeader.tsx`, `ui/src/lib/mark-hues.ts`, `ui/tests/visual/attention-budget.spec.ts` (new), `ui/tests/visual/contrast.spec.ts`, `ui/tests/visual/fixtures/`, `scripts/qa/colourfulness.py` (new), `docs/qa/0.5-walks/calm-glance.md` (new).
**Research basis**: `docs/research/calm-attention/proposal.md` §(a)–(c2), §(f); `audit.md` §1 (root causes) and §6 (the count); `research.md` §§1–3.
**Handover note**: another session may already be editing `ui/src/styles.css` and an `attention-tokens` test on `release/0.5`: read the tree and its log first, build on what is there, and never commit its files (`git commit -- <paths>`). The level is computed in the UI from data the backend already sends; if a fact is missing, add it to the read model, not a tool check in the UI. Keep `.dot-live` at three iterations; the proposal's "remove it" is replaced by 0.4.5's rule.

### AJ-E3602 One count of what needs you: L2 and L3, in the rail, the bell, the title and the tray | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3601
**Goal**: The number that says "things waiting on you" means exactly that, and agrees everywhere.
**User story**: As someone glancing at the bell, the number is how many things wait on me, not how many I could look at.
**Requirements**:
1. The rail's Needs you count, the bell, Home's title and pinned zone, the window title and the tray SHALL count L2 and L3 items only, each once (D-117 point 5), from `attentionLevel()`.
2. "Done, review it" items (L1) SHALL stay listed at the end of Needs you under a faint "To review" label and not be counted; Home's counts line names them ("1 to review").
3. A count SHALL take the colour of the highest level it holds: neutral (`--color-count`) at L0–L2 with an amber dot for L2, red (`--color-count-alarm`) only when it holds an L3 item.
4. In Open now, a session that needs you SHALL keep its place and its word, drawn as an amber dot with the word in normal ink; the loud copy is its Needs you row (D-47 point 2 as amended by D-117).
5. The flood rule SHALL apply: past 5 open L2/L3 items, Needs you groups them by kind ("4 ask you · 2 probably stuck · 1 limit reached") with amber on the group heads only, L3 items never grouped away; Home's title says the same.
6. `ui/src/count-agreement.test.tsx` SHALL prove the numbers agree on the `allStatuses` and flood fixtures, reviews included and excluded.
**UI/UX reasoning**: "one queue, one badge" (deep/11 §1.1, AJ-E3104); ISA-18.2's flood handling; NN/g on notification badges (a count that includes non-actionable items trains users to ignore it).
**Performance budget**: the count recomputes in ≤ 1 ms for 500 items; no extra IPC.
**Quality gates**: verify-static; the count-agreement test; the attention-budget test's count assertions.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3602/manifest.md`, R1–R6; captures of the rail, bell and Home title with 3 needs and 1 review, and with the flood fixture.
**Visual testing**: the bell at L0, L2 and L3; Needs you with reviews; the flood fixture; both themes, 1280 and 1024.
**Files to touch**: `ui/src/lib/attention.ts`, `ui/src/components/NotificationBell.tsx`, `ui/src/components/RailSections.tsx`, `ui/src/components/PlaceBoardParts.tsx`, `ui/src/screens/Home.tsx`, `crates/juggler-app/src/features/notifications/tray.rs`, `ui/src/count-agreement.test.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(a) rules 4–5, §(d) "The one change that needs a ruling"; D-47; D-117 point 5.
**Handover note**: the tray reads its count in Rust; compute the level there from the same rules (one table, tested against the TypeScript one with a shared fixture) rather than a second mapping that can drift.

### AJ-E3603 The rail and the header, calm | Status: IN-PROGRESS (fix 8 built: one quiet mark per need; audit, densities, persona walk and review page left: docs/qa/evidence/AJ-E3603/manifest.md) | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3602
**Goal**: The rail is calmer than the content, and the header spends no colour on chrome.
**User story**: As someone with the rail open all day, I see my sessions in quiet rows and only the ones that need me stand out.
**Requirements**:
1. An audit of the rail's and the header's loud elements against the budget (the audit's R and H rows refreshed on the current tree), before any change.
2. The calm pass on the rail and the header: section labels in `--color-section`; Open now and every section head without icon or cyan; no row left edge in the App colour; a 6 px App dot on rows; Working a static calm dot; the ↗, pin and drag handle on hover, focus or the selected row; row titles medium ink-2 (selected and Needs you rows semibold ink); the rail-foot New session neutral; the clean-up control a ghost button; the avatar neutral; one hairline between the rail and the main area. Every collapsible kept: Needs you, Open now, Inactive apps, Recent, App and profile group headers, session rows that unfold subagents, the folded rail's groups; each chevron muted at rest, visible on hover and focus, and operable with the keyboard (Enter, Space, ← and →).
3. The attention-budget test SHALL cover the rail and the header on its three fixtures.
4. Contrast and axe, both themes, every rail density (Comfortable, Compact, Minimal).
5. A Sonnet persona glance and walk by hand (the vibe-coder with 8 sessions on 3 Apps, a newcomer with 1 App, a heavy day with 14 open, 7 needs and a crash), verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
8. The legend (`LegendPopover`, `indicator.tsx`) SHALL describe the four levels and drop "a row's left edge is its app's colour".
**UI/UX reasoning**: the rail is periphery (calm technology, research §4); Linear's dimmer sidebar; PRINCIPLES §5 (say a thing once) and D-69 (one colour, its App's dot).
**Performance budget**: no new render on hover beyond the hovered row; rail scroll at 60 fps with 50 rows.
**Quality gates**: verify-static; the rail's existing keyboard, drag and order tests green; the attention-budget test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3603/manifest.md`, R1–R8; the colourfulness report on the rail captures (rail M ≤ main M + 1).
**Visual testing**: the rail in all three densities, folded and open, App, Project and Account lenses, all-clear, typical and flood, both themes, 1024 and 1280.
**Files to touch**: `ui/src/components/RailSections.tsx`, `rail-groups.tsx`, `RailFolded.tsx`, `RailHeader.tsx`, `PlaceBoardParts.tsx`, `PlaceHeader.tsx`, `row/SessionRow.tsx`, `AppHeader.tsx`, `MainNav.tsx`, `UserMenu.tsx`, `LegendPopover.tsx`, `ui/src/lib/rail-items.ts`.
**Research basis**: `docs/research/calm-attention/audit.md` §§3–4 (the rail and header inventory); `proposal.md` §(c2), §(d) rows R1–R33 and H1–H6.
**Handover note**: the maintainer could not see the chevrons in the round-2 mock; the captures on the review page must show each collapsible with its chevron, at rest and on hover.

### AJ-E3604 Projects, calm: the Projects screen and the project page | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3603
**Goal**: Projects shows where work is happening, with only the projects that need you standing out.
**User story**: As someone with twelve projects, I find the one with a stuck session at a glance.
**Requirements**:
1. An audit of the loud elements on the Projects screen and the project page (its header chips, tabs and lists) against the budget.
2. The calm pass: project state glyphs muted at L0–L1, amber or red only at L2–L3; the project's colour ring replaced by a neutral count; related lists without borders or dividers; one filled primary per view. Every collapsible kept (project groups, the sessions list's App groups and subagent rows), chevrons as D-117 point 3.
3. The attention-budget test extended to the Projects fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: PRINCIPLES §11 (active first) and §12 (every object has a door); the budget's one loud mark per fact per area.
**Performance budget**: no regression on the Projects screen's first paint (the gallery's timing).
**Quality gates**: verify-static; the attention-budget test; the projects screen tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3604/manifest.md`, R1–R7.
**Visual testing**: Projects and a project page, typical and busy fixtures, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/projects/`, `ui/src/pages/ProjectPage.tsx`, `ui/src/pages/project/`, `ui/src/components/RelatedList.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(b), §(g) phase C.
**Handover note**: —

### AJ-E3605 Pools, calm, on 0.4.6's calm pool redesign | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3603, 0.4.6's calm pool redesign merged into release/0.5
**Goal**: Pools reads as one calm system with the rest of the app, built on the redesign the 0.4 line ships.
**User story**: As someone running a pool of four agents, I see which one waits on me, and the canvas doesn't flash at me otherwise.
**Requirements**:
1. An audit of the loud elements on the Pools screen, a pool's page and canvas, and the new-pool wizard, made on 0.4.6's calm pool redesign once it is merged into `release/0.5`, never on the older look.
2. The calm pass, reusing the token names 0.4.6 already uses: a member waiting on you amber once per area, a paused pool or failed delivery at its level, lines and nodes calm at rest; the wizard's steps calm with one primary. Every collapsible kept (member groups, threads, the canvas's group nodes, task lists), chevrons as D-117 point 3.
3. The attention-budget test extended to the pool fixtures (canvas included).
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: the same attention grammar everywhere (UI consistency); the canvas teaches by structure, not colour.
**Performance budget**: the canvas keeps its current frame budget; no looping animation on lines or nodes.
**Quality gates**: verify-static; the attention-budget test; the pools tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3605/manifest.md`, R1–R7.
**Visual testing**: the Pools screen, a pool page with its canvas, the wizard, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/pools/`, `ui/src/pages/PoolPage.tsx`, `ui/src/pages/pool/`, `ui/src/components/PoolCards.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(g) phase C; D-116 (desktop sessions as full pool members, 0.4.6).
**Handover note**: don't start before 0.4.6's pool redesign is merged into `release/0.5`; check which tokens it already defines and reuse them, never a second set.

### AJ-E3606 Tasks, calm | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: The task board shows what is blocked on you, quietly showing everything else.
**User story**: As someone with thirty tasks across two pools, I see the one awaiting my approval first.
**Requirements**:
1. An audit of the loud elements on the Tasks screen (board, cards, filters, a task's detail) against the budget.
2. The calm pass: a held change awaiting you L2 once; in review and blocked L1; done L0; cards without borders; status chips muted; filters as filled segmented controls. Every collapsible kept (columns, pool groups, a task's events), chevrons as D-117 point 3.
3. The attention-budget test extended to the Tasks fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: one loud mark per fact per area; PRINCIPLES §11.
**Performance budget**: the board with 500 tasks paints in ≤ 200 ms (no regression).
**Quality gates**: verify-static; the attention-budget test; the tasks tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3606/manifest.md`, R1–R7.
**Visual testing**: the board and a task, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/tasks/`, `ui/src/pages/tasks/`, `ui/src/components/TaskNeeds.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(g) phase C.
**Handover note**: —

### AJ-E3607 Accounts and Apps, calm: the Accounts screen, the Account page and the App page | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3603
**Goal**: Runway reads as calm grey until an Account is really short or out with work on it.
**User story**: As someone on four subscriptions, I see which Account is out while a session waits, and the others don't compete for my eye.
**Requirements**:
1. An audit of the loud elements on the Accounts screen (Accounts and the Apps section), the Account page and the App page against the budget.
2. The calm pass: runway meters neutral when healthy, amber at ≤ 10 % with a live session, red when out with live work; "Out" with nothing running muted with its reset time; "Measure its limits" an ink-2 link; Account discs kept (D-69); App icons in the App tone; cards without borders. Every collapsible kept (Account groups, the Apps section's profile groups, launch settings), chevrons as D-117 point 3.
3. The attention-budget test extended to the Accounts fixtures, the Out fixtures included.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: a meter is a value, not an alarm, until it crosses a threshold that matters (research §2, ISA-101 "normal is grey").
**Performance budget**: no regression on the Accounts screen's first paint.
**Quality gates**: verify-static; the attention-budget test; the accounts and Out-filter tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3607/manifest.md`, R1–R7.
**Visual testing**: Accounts, an Account page and an App page, healthy, low and out, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/accounts/`, `ui/src/pages/AccountPage.tsx`, `ui/src/pages/account/`, `ui/src/pages/app/`, `ui/src/components/AccountRunway.tsx`, `ui/src/components/RenewalChip.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(d) rows M11, M24–M26.
**Handover note**: —

### AJ-E3608 Usage, calm | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: Usage keeps its charts' colours (they are data) and drops the status colour around them.
**User story**: As someone reading last week's spend, the charts are the colourful part and nothing else distracts.
**Requirements**:
1. An audit of the loud elements on Usage (dashboard tiles, runway cards, cost review, wasted spend) against the budget.
2. The calm pass: chart series keep their categorical colours (data, not status); tiles and cards without borders; only wasted spend on a crashed or stuck session or a cap hit takes a level colour; filters filled. Every collapsible kept (per-Account and per-project sections, the cost review's rows), chevrons as D-117 point 3.
3. The attention-budget test extended to the Usage fixtures, with chart marks excluded by role.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: data colour is categorical and stays (proposal §(b)); the "Readable and visual" ruling.
**Performance budget**: no regression on the Usage first paint.
**Quality gates**: verify-static; the attention-budget test; the usage tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3608/manifest.md`, R1–R7.
**Visual testing**: Usage at a quiet and a busy week, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/Usage.tsx`, `ui/src/components/usage/`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(b) "Charts on Usage keep their categorical colours".
**Handover note**: the attention-budget test must not count chart marks as loud; mark them with a role or test id rather than a colour exception.

### AJ-E3609 History, calm | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: History is a quiet record, with failures marked once.
**User story**: As someone looking for yesterday's session, I scan a calm list and spot the one that crashed.
**Requirements**:
1. An audit of the loud elements on History against the budget.
2. The calm pass: ended sessions L0; a crash or failed switch L3 glyph and word, no fills; rows without dividers; the corner dot only the App's (the audit's root cause 6). Every collapsible kept (day groups, a session's subagents), chevrons as D-117 point 3.
3. The attention-budget test extended to the History fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: history is past, so at most L3 for what failed; PRINCIPLES §5.
**Performance budget**: paged as today; no regression.
**Quality gates**: verify-static; the attention-budget test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3609/manifest.md`, R1–R7.
**Visual testing**: History, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/History.tsx`, `ui/src/pages/SessionsList.tsx`.
**Research basis**: `docs/research/calm-attention/audit.md` §1 root cause 6.
**Handover note**: —

### AJ-E3610 This Mac, calm: This Mac, Processes and your Macs | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: This Mac is grey when the Mac is fine, amber under warning pressure, red at critical.
**User story**: As someone whose Mac is busy, I see the one session holding 9 GB, not a dashboard of coloured gauges.
**Requirements**:
1. An audit of the loud elements on This Mac, Processes, Cleanup and the Macs (Fleet) screen against the budget.
2. The calm pass: "Plenty of room" a faint static dot with muted text; warning pressure L2, critical L3; leftovers L1 (L2 when heavy); gauges neutral until a threshold; Fleet chips muted. Every collapsible kept (process trees, per-session groups, peers), chevrons as D-117 point 3.
3. The attention-budget test extended to the This Mac fixtures (calm, warning, critical).
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: ISA-101 (normal is grey); proposal §(d) H1–H2.
**Performance budget**: no regression; no added polling.
**Quality gates**: verify-static; the attention-budget test; the machine-state agreement test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3610/manifest.md`, R1–R7.
**Visual testing**: This Mac calm, warning and critical; Processes; Macs; both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/ThisMacScreen.tsx`, `ui/src/screens/processes/`, `ui/src/screens/FleetScreen.tsx`, `ui/src/components/this-mac-parts.tsx`, `ui/src/components/this-mac-memory.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(a) L3/L2 rows for This Mac; §(d) H1–H2.
**Handover note**: keep the `.dot-live` three-iteration rule on the status dot (0.4.5); never an infinite pulse.

### AJ-E3611 The session pages, calm | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3601, AJ-E3603
**Goal**: A session's page says its state once, loudly only when it needs you, and keeps the conversation the brightest thing.
**User story**: As someone reading a session, I see what it asks me in one place and read the rest without competing badges.
**Requirements**:
1. An audit of the loud elements on the session page (head, chips, tabs, the conversation, the subagents tab, usage, the reply area) against the budget.
2. The calm pass: the state word at its level once in the head; chips muted; the reply area's one primary; the turn marks calm; the App icon in the App tone. Every collapsible kept: the subagents tree and session rows that unfold their subagents, folded turns and tool calls, the conversation's long outputs; chevrons as D-117 point 3.
3. The attention-budget test extended to the session-page fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: the conversation is the subject (E34); one loud mark per fact per area.
**Performance budget**: no regression on opening a session (AJ-E3401's budgets).
**Quality gates**: verify-static; the attention-budget test; the session page and subagents keyboard tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3611/manifest.md`, R1–R7.
**Visual testing**: a working, an asking, an ended and a crashed session; the subagents tab unfolded; both themes, 1024 and 1280.
**Files to touch**: `ui/src/pages/session/`, `ui/src/pages/subagents/`, `ui/src/components/ConversationPage.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(b), §(g) phase C.
**Handover note**: E34's new blocks (AJ-E3401…AJ-E3405) land on the same pages; whichever lands second follows the other's tokens, never a parallel style.

### AJ-E3612 Settings, calm | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: Settings is quiet, with only a setting that blocks work (a sign-in, a permission) at its level.
**User story**: As someone changing one setting, I find it without a page of coloured cards.
**Requirements**:
1. An audit of the loud elements in Settings (every pane, search, the Privacy page) against the budget.
2. The calm pass: panes without card borders; switches and segmented controls filled; one primary per pane; a missing permission or sign-in at L2 once; Serious-choice tips muted until opened. Every collapsible kept (advanced sections, per-Account rows), chevrons as D-117 point 3.
3. The attention-budget test extended to the Settings fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: Apple HIG settings (grouped, quiet); PRINCIPLES §4 (fold the rare).
**Performance budget**: no regression.
**Quality gates**: verify-static; the attention-budget test; the settings tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3612/manifest.md`, R1–R7.
**Visual testing**: each Settings pane, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/Settings.tsx`, `ui/src/screens/settings/`, `ui/src/screens/PrivacyScreen.tsx`, `ui/src/components/SettingsSections.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(c2) usage changes.
**Handover note**: —

### AJ-E3613 First run, calm | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: The first minutes feel calm and confident, with one primary action per step.
**User story**: As a newcomer, I see what was found and the one next step, without a wall of badges.
**Requirements**:
1. An audit of the loud elements in first run (Welcome, the scan and its found rows, Found / Likely / Guessed, the access step, Add provider and the fresh-Account step) against the budget.
2. The calm pass: confidence levels as quiet glyphs and words (L1), a sign-in needed L2 once, one primary per step, cards without borders, App icons in the App tone. Every collapsible kept ("More found", advanced options), chevrons as D-117 point 3.
3. The attention-budget test extended to the first-run fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand (a newcomer with 1 App, one with 3), verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: PRINCIPLES §3 and §14; a first impression sets the meaning of colour for everything after.
**Performance budget**: no regression on the first-run journey's timings.
**Quality gates**: verify-static; the attention-budget test; the first-run journey test.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3613/manifest.md`, R1–R7.
**Visual testing**: every first-run step, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/FirstRun.tsx`, `FirstRunSteps.tsx`, `FirstRunFoundRow.tsx`, `FirstRunMore.tsx`, `FreshAccountStep.tsx`, `AddProviderSheet.tsx`, `ui/src/components/Welcome.tsx`, `AccessStep.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(g) phase C.
**Handover note**: D-118 keeps first run unchanged in layout; this task changes only its look.

### AJ-E3614 Sheets, dialogs and toasts, calm | Status: TODO | Priority P1 | Effort M | Depends: AJ-E3601, AJ-E3603
**Goal**: Every sheet and dialog has one primary, and a toast is loud only when something broke.
**User story**: As someone switching a session, the sheet tells me the one choice to make without a row of coloured buttons.
**Requirements**:
1. An audit of the loud elements in every sheet, dialog, popover and toast (New session, switch and continue flows, quit, delete, restart, pool and project sheets, the palette, the Ask card, banners) against the budget.
2. The calm pass: one filled primary per sheet; destructive actions red text on a ghost or bordered button, filled only in a destructive confirm; toasts neutral except an L3 one; banners at their level, one tinted block at most. Every collapsible kept (folded options, "Why?" tips, advanced sections), chevrons as D-117 point 3.
3. The attention-budget test extended to the sheet and dialog fixtures.
4. Contrast and axe, both themes.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: PRINCIPLES §9; Apple HIG alerts (one default button).
**Performance budget**: a sheet opens in ≤ 150 ms (no regression).
**Quality gates**: verify-static; the attention-budget test; the dialogs' keyboard tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3614/manifest.md`, R1–R7.
**Visual testing**: each sheet and dialog in the gallery, both themes, 1024 and 1280.
**Files to touch**: `ui/src/screens/NewSession.tsx`, `ui/src/screens/new-session/`, `ui/src/components/SwitchFlow.tsx`, `ContinueDialog.tsx`, `QuitSheet.tsx`, `DeleteInstanceDialog.tsx`, `RestartApp.tsx`, `Toast.tsx`, `ToastStack.tsx`, `Banner.tsx`, `AskCard.tsx`, `GoToPalette.tsx`, `ui/src/components/palette/`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(a) rule 3, §(d) M27.
**Handover note**: —

### AJ-E3615 The menu-bar menu and notifications, calm | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3601, AJ-E3602
**Goal**: The menu-bar icon and macOS notifications speak up for L3, for L2 only if the user asks, and never for L0 or L1.
**User story**: As someone working in another app, I'm interrupted when a session breaks, not when one finishes normally, unless I chose that.
**Requirements**:
1. An audit of the menu-bar icon's states, the menu's rows and every notification kind against the four levels.
2. The calm pass: the menu-bar icon's alert state only at L3 (and L2 when the user's existing notification settings ask); the menu's rows at their levels with one count (AJ-E3602); notifications and their sound tied to the level through the existing settings and mutes; no new setting.
3. The attention-budget rules applied to the menu's rendered rows (a test over the tray's model: one loud row per L2/L3 item, none for L0–L1).
4. The menu's contrast in both macOS appearances, and VoiceOver labels naming the level in words.
5. A Sonnet persona walk (a day with a crash and two asks), verified by Opus.
6. The design-quality critique.
7. Before and after captures of the menu and each notification kind on the review page, each with what changed.
**UI/UX reasoning**: Apple HIG notifications (interrupt for what's time-sensitive); research §3 (alarm fatigue).
**Performance budget**: the menu opens in ≤ 100 ms; no added wake-ups at rest.
**Quality gates**: verify-static; the tray model tests; the notification mute tests.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3615/manifest.md`, R1–R7.
**Visual testing**: the menu bar icon at L0, L2 and L3; the menu; each notification kind; light and dark.
**Files to touch**: `crates/juggler-app/src/features/notifications/tray.rs`, `crates/juggler-app/src/features/notifications/`, `ui/src/screens/settings/NotificationsPane.tsx`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(g) phase C ("Tray icon states and notification sounds tied to L3 only").
**Handover note**: the tray computes its level in Rust from the shared table AJ-E3602 introduces; no second mapping.

### AJ-E3616 The Floor's HUD and list, calm where it applies | Status: TODO | Priority P1 | Effort S | Depends: AJ-E3601, AJ-E3603
**Goal**: The Floor keeps its playful scene, and its signals follow the same four levels.
**User story**: As someone watching the Floor, the desk that needs me stands out the same way its row does in the rail.
**Requirements**:
1. An audit of the Floor's HUD (labels, flags, the View menu, hover cards, counts) and the Floor list against the four levels.
2. The calm pass where it applies: the HUD's colour accents aligned to L2 and L3 only; the scene's tool looks and logos untouched (they are identity and art, not status); the Floor list rows on the rail's grammar. Every collapsible kept (the Floor list's groups, the View menu's sections), chevrons as D-117 point 3.
3. The attention-budget test extended to the Floor list; the HUD's signal table checked against `attentionLevel()` in the Floor's own tests.
4. Contrast and axe on the Floor list; the HUD's text contrast over the scene, day and night.
5. A Sonnet persona glance and walk by hand, verified by Opus.
6. The design-quality critique.
7. Before and after captures on the review page, each with what changed.
**UI/UX reasoning**: the Floor already shows states by its people; colour accents follow the shared levels so the two views agree.
**Performance budget**: the Floor's frame and memory budgets unchanged (AJ-E3513).
**Quality gates**: verify-static; the Floor's golden frames updated once, each checked by eye.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3616/manifest.md`, R1–R7.
**Visual testing**: the Floor's golden frames with a need and a crash, day and night; the Floor list, both themes.
**Files to touch**: `crates/juggler-floor-render/`, `crates/juggler-floor/`, `ui/src/pages/floor/`.
**Research basis**: `docs/research/calm-attention/proposal.md` §(g) phase C; E35's signal table (AJ-E3509).
**Handover note**: the Floor is native (wgpu); pass the level from the feed (AJ-E3505), never recompute it in the renderer.

### AJ-E3617 The calm app closed: the classic flag removed, the principles and the guide written, the whole app walked | Status: TODO | Priority P0 | Effort S | Depends: AJ-E3602…AJ-E3616, AJ-E3701
**Goal**: The 0.5 Stable ships one calm look on every screen, documented, with nothing of the old look left.
**User story**: As the maintainer, I promote 0.5 to Stable knowing no screen was left on the old look and the beta's comparison flag is gone.
**Requirements**:
1. The `aij.attention=classic` flag and every code path behind it SHALL be removed before 0.5 goes Stable; AJ-E3601's guard test passes.
2. PRINCIPLES.md SHALL gain "15. An attention budget" with the four levels and the budget's rules, and its reviewer checklist a line for it.
3. The user guide SHALL gain "What the colours mean" (the four levels, words first, the App tone), linked from the legend.
4. A whole-app glance and walk on the release candidate (every screen of D-117 point 6, the three personas), Sonnet, verified by Opus, with nothing scored under 4.
5. The colourfulness report across the whole gallery, before (0.4.x) and after, on the review page.
**UI/UX reasoning**: one grammar for the legend, the docs, the captures and the tests (proposal §(e)).
**Performance budget**: n/a.
**Quality gates**: verify-static; `docs_links_check`; the attention-budget test across every screen's fixtures.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3617/manifest.md`, R1–R5.
**Visual testing**: the review page complete: every screen's before and after.
**Files to touch**: `ui/src/lib/attention.ts`, `docs/design/PRINCIPLES.md`, `docs/guide/`, `ui/src/components/LegendPopover.tsx`.
**Research basis**: D-117 point 4 and point 6.
**Handover note**: if the maintainer asks for a user-facing choice after living with the calm default, the proposal's §(e) "Emphasis: Standard / Strong" is the 0.6 candidate; it needs its own ruling.

### AJ-E3618 Making and editing screens, rich: the audit's worst list and the shared kit (D-126) | Status: TODO | Priority P0 | Effort M | Depends: AJ-E3601
**Goal**: Every wizard, create or edit page and setup flow is classed and ranked, so the worst are fixed first, with 0.4.6's pool-wizard kit as the one design language.
**User story**: As someone adding an account or an app, I see a screen that looks as considered as the rest of the product, not a bare form.
**Requirements**:
1. Every screen SHALL be audited and classed as monitoring (calm, D-117) or making and editing (rich, D-126), and the worst 25 listed with a reason each, in `docs/research/screen-audit.md`. (The audit's list was asked for by D-126's widened scope on 2026-11-16 and is not in the repository at 2026-11-18; this requirement produces it.)
2. Recurring problems SHALL be fixed once at the token and component level, reusing the 0.4.6 kit (FlowSteps, choice cards with previews, tinted surfaces, a live summary), before any screen is touched.
3. Mock-ups in both themes SHALL go to the maintainer before building (D-126 point 3).
**UI/UX reasoning**: one design language, with scarce status colour on monitoring screens and the accent used for selection, progress and previews on making and editing screens (D-126's clarification).
**Performance budget**: unchanged.
**Quality gates**: verify-static; the design-quality critique; ui-consistency.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3618/manifest.md`, R1 to R3; the audit file.
**Visual testing**: the mock-ups, both themes.
**Files to touch**: `docs/research/screen-audit.md`, `ui/src/components/` (the kit), `ui/src/lib/`.
**Research basis**: D-126; 0.4.6's pool wizard.
**Handover note**: read the wireframes and the UX spec first (read-specs rule).

### AJ-E3619 Making and editing screens, rich: the worst 25 rebuilt, starting with Notifications, Add app and Add account (D-126) | Status: TODO | Priority P0 | Effort XL | Depends: AJ-E3618
**Goal**: The screens where a user creates or edits things look great and read easily, as D-126 asks, in 0.5.
**User story**: As a newcomer, adding my first account or app feels guided and polished, with a preview of what I'm about to do.
**Requirements**:
1. Notifications, Add app and Add account SHALL be rebuilt first, then the rest of AJ-E3618's list, each with the app's accent and tinted surfaces, icons and logos on every choice, choice cards with previews instead of bare checkboxes, clear step progress, a live summary and short copy with depth behind "Learn more".
2. Every other wizard, create or edit page and setup flow in AJ-E3618's list SHALL follow, none left on the plain look.
3. Each rebuilt screen SHALL pass the a11y gate and the design-quality critique, with before and after captures that say what changed.
4. A newcomer persona SHALL walk the three first screens by hand (Sonnet, verified by Opus).
**UI/UX reasoning**: D-126: calm never means plain; a wizard next to Home must read as the same product.
**Performance budget**: each screen opens in under 150 ms from cache; no loops at idle.
**Quality gates**: verify-static; the a11y gate; the attention-budget test; baselines once per round.
**Evidence/validation proof**: `docs/qa/evidence/AJ-E3619/manifest.md`, R1 to R4.
**Visual testing**: each screen, both themes, narrow and wide.
**Files to touch**: `ui/src/screens/`, `ui/src/components/`, `ui/src/lib/`.
**Research basis**: D-126; the screen audit (AJ-E3618).
**Handover note**: 0.4.6 has the pool wizard; 0.5 covers everything else.
