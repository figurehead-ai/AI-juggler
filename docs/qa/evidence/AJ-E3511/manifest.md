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

## By requirement (2026-11-06, for the release check)

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the pane's choices as `floor.*` keys | `features/floor/store.rs` keys (`floor.open_at_start`, `floor.on_top`, `floor.apps_per_floor`, `floor.motion`, `floor.decay`, the hidden list), `controls.rs`'s `checked`; vitest `pages/floor/floor-list.test.tsx` "each choice is a floor.* key; Apps per floor stays within 4–16"; gallery `99-floor-settings` | every listed choice, Apps per floor 4–16 default 10, Reset the view, Re-create, hidden Apps under More | tests pass; the captures show each choice, Reset the view, Re-create and the Apps hidden under More | PASS | 1fbc74fdf | 2026-11-06 | Claude (self) |
| R2 Re-create asks first, says what changes and what stays, with an example and the guide | vitest "Re-create asks first, saying what changes and what stays"; `99-floor-settings/recreate-*` | the confirmation before anything changes | passes; the sheet says every area's look and layout change, re-placed in the same order, and Apps, sessions, history and the main window stay, with the guide's link | PASS | 1fbc74fdf | 2026-11-06 | Claude (self) |
| R3 the wireframes approved before the UI | `docs/design/the-floor-settings-and-list-2026-11-04.html`, approved as D-78 (decision log) | approved before the build | approved 2026-11-04, built after to them; filed under `docs/design/`, not `docs/wireframes/` as the requirement names | PASS | 1fbc74fdf | 2026-11-06 | Claude (self) |
