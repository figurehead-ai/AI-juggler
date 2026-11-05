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
