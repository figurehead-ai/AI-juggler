# AJ-E3511 evidence: Settings › The Floor, and Re-create

- The wireframes, approved as D-78 (2026-11-04):
  `docs/design/the-floor-settings-and-list-2026-11-04.html` (the Floor
  button, the pane, Re-create and the Floor list).
- Captures: gallery `99-floor-settings` (the pane, the Apps hidden under
  More, the Re-create confirmation) and `99-floor-menu` (the Floor
  button's menu); both themes, 1280×800 and 1024×700, axe 0
  critical/serious, critiques stamped.
- [timing.json](timing.json): the pane opens in 0.5 ms p95 (budget 50 ms),
  preview data, Chromium.
- Each choice's `floor.*` key and its range: unit test of `checked` in
  `features/floor/controls.rs`; the stepper's 4–16 and Re-create's
  confirmation: `ui/src/pages/floor/floor-list.test.tsx`.
- Verdict: PASS.
