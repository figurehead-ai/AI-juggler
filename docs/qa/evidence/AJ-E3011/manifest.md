# Evidence manifest — AJ-E3011 (and AJ-E3012)

Pool setup a newcomer understands, and the canvas's selection and inspector title (`docs/design/pool-setup-0.4.6.md`). Built on release/0.4.6, 2026-11-15. The newcomer walks after the build, P10 and the AJ-E3304 pool scenarios are walked separately and added here; until then this task is not done.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Walks before (all F) | `walks/before-run1.md`, `walks/before-run2.md`, `walks/before-run3.md` | — | the same five walls each run, each verified by Opus | recorded | a1885cdb4 | 2026-11-15 | Opus |
| §3 lead_next: an App's next session leads, D-92 holds | `cargo test -p juggler-core --test pools_lead_next`; `cargo test -p juggler-app --test pools_lead_next` | all pass; two sessions starting at once: exactly one Lead | 6 + 5 passed; the race run 20× (two connections) and 10× (two threads), one Lead each time | PASS | 13a5c2764 | 2026-11-15 | Opus |
| R1–R9 the setup in order, every control labelled and read back, the lead picker's three kinds, the inspector title per member type | `pnpm --dir ui exec vitest run` | every test passes | 424 files, all passed (new: `setup-controls.test.tsx`, `inspector-title.test.tsx`, `canvas-teach.test.tsx`, `new-pool.test.tsx`, `new-pool-roles-to-choose.test.ts`) | PASS | 4151bb9e0 | 2026-11-15 | Opus |
| a11y: labels on every control, aria-current on the step and the selected card, the inspector's heading names the selection; axe 0 serious | `ui/tests/visual/a11y.spec.ts` "the pool setup and the canvas's selection"; `pool-setup-gallery.spec.ts` (axe on each capture) | 0 unnamed controls, 0 critical/serious | passed, both themes | PASS | 4151bb9e0 | 2026-11-15 | Opus |
| Gallery: every setup step empty and filled, the canvas with each kind selected, both themes, in the pinned container | `./scripts/visual_gate.sh --update tests/visual/pool-setup-gallery.spec.ts --workers=1` | 50 baselines, each opened and checked (no real names, no clipping) | `ui/tests/visual/__baselines__/linux/gallery/98-pool-setup/` (32), `…/99-pool-selection/` (18); evidence copies in `_gallery/98-pool-setup` and `_gallery/99-pool-selection` with their critiques | PASS | this commit | 2026-11-15 | Opus (a Sonnet capture check, verified) |
| Design-quality critique | `docs/qa/evidence/_gallery/98-pool-setup/critique.md`, `docs/qa/evidence/_gallery/99-pool-selection/critique.md`, the 0.4.6 sections of 97-pool-canvas and 92-agent-tools | written for the captures as they are | stamped | PASS | this commit | 2026-11-15 | Opus |
| Walks after (two newcomer walks, scored the same way) | the walk brief of `walks/before-run*.md` on this build | every screen "yes" | not yet walked | TODO | — | — | — |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
