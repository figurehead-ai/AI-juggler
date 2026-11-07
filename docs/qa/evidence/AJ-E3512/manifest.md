# AJ-E3512 evidence: the Floor list

- Captures: gallery `99-floor-list`, both themes, 1280×800 and 1024×700.
  axe 0 critical/serious on each (the a11y gate, run by the gallery).
- [keyboard-walk.md](keyboard-walk.md): reached with ⌘K, every control in
  reading order, each named for what it does.
- [timing.json](timing.json): 150 rows in 10–15 ms p95 (budget 16 ms),
  timed on the release Mac (skipped on CI's shared runners).
- New waits said once: `ui/src/pages/floor/floor-list.test.tsx` (the list
  says desktop apps' conversations; the main window's AttentionAnnouncer
  says sessions AI Juggler runs).
- Req. 3: the Floor window's title is `Scene::summary` ("Floor 1: 6 Apps,
  2 waiting on you"), set as it changes (`features/floor/render.rs`).
- Not covered: a pass with VoiceOver itself.
- Verdict: PASS.

## By requirement (2026-11-06, for the release check)

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the list by floor and App: state, session, subagents, tasks, project, usage, Show on the Floor; from the Floor button and ⌘K | vitest `pages/floor/floor-list.test.tsx` ("groups by floor…", "shows each state as a glyph and a word…", "one header per floor, the room once…"); gallery `99-floor-list`; `keyboard-walk.md` | every field, both ways in | tests pass; the captures show each floor's header with its Accounts' room, each App's header and Show on the Floor, every row's glyph, word, session, project, subagents and tasks; reached by ⌘K and the Floor button's menu | PASS | 07bbee8f3 | 2026-11-06 | Claude (self) |
| R2 new waits said politely, once each | vitest "shows each state as a glyph and a word, and says a new wait once"; the main window's AttentionAnnouncer for sessions AI Juggler runs | once each, polite | passes | PASS | 07bbee8f3 | 2026-11-06 | Claude (self) |
| R3 the Floor window's accessibility label sums up the floor in view | `scene::tests::the_window_says_the_floor_in_view_in_words` ("Floor 1: 2 Apps, 1 waiting on you"); `features/floor/render.rs` sets it as the title when it changes; the Dev walk saw "The Floor — Floor 2: 4 Apps" and "The Floor — The building: 9 Apps" | the floor in view in words | passes; seen in AI Juggler Dev (AJ-E3516's walk) | PASS | 1fbc74fdf | 2026-11-06 | Claude (self) |
