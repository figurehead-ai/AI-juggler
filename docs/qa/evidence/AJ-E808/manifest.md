# Evidence manifest — AJ-E808

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E808
(provider-pair fidelity validation matrix — operations). 2026-10-19,
reviewer claude-opus.

Evidence: the register [`register.md`](register.md), rendered from
`crates/juggler-handover/data/fidelity-register.json` by
`just fidelity-register`. A test fails when the document drifts from the
data.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Req.1: every ordered pair is covered on transcript resume, rules carry, todos, subagents, file history, MCP references, cwd/worktree edges (dotted and long paths) and switch-back A→B→A | Req.1 | 11 pairs (every row of the validation matrix) × 8 areas. `the_committed_register_is_well_formed_and_covers_the_matrix` fails if a pair or an area is missing or duplicated | PASS | 2026-10-19 | claude-opus |
| Req.2: each cell is PASS/FAIL/UNVALIDATED with binary versions, date, fixture ID and evidence path; UNVALIDATED is the default and shows as unvalidated in the product | Req.2 | `register::Register::problems` rejects a PASS or FAIL missing any of those. Every cell is UNVALIDATED today. The register shows the product's exact words, rendered by the same code: "F1 · Beta · Not yet validated" (switch rows) and "Beta — not yet validated" (Settings › Beta) | PASS | 2026-10-19 | claude-opus |
| Req.3: live-binary cells only; fixture and fake-harness results never promote a cell | Req.3 | A PASS or FAIL needs `run: "live"` (`only_live_runs_with_full_provenance_decide_a_cell`). `validated()` now also requires every area of the pair PASS in the register, so a live cell recorded in the app can no longer promote a badge on its own (`validation_gate` and `validation_matrix` tests updated) | PASS | 2026-10-19 | claude-opus |
| Req.4: a Provider upgrade resets affected cells automatically | Req.4 | `Register::effective` reads a PASS recorded against other versions than the installed ones as UNVALIDATED ("reset: claude-code is 2.2.0, the run used 2.1.296"). The app passes the versions found by Provider discovery and evaluates strictly: an unknown version can't confirm a PASS (`a_provider_upgrade_resets_its_cells_by_itself`). Discovery runs only when the register has a decided cell for the pair, so there's no added cost today | PASS | 2026-10-19 | claude-opus |
| Req.5: beta-exit — same-Provider pairs PASS on every cell; cross-Provider pairs PASS or FAIL with a drop list | Req.5 | `Register::beta_exit_open` (`beta_exit_needs_same_provider_pass_and_cross_provider_drop_notes`). Today: 72 cells open (9 pairs × 8 areas; pack-only pairs excluded as F0 by definition). Feeds criterion B1 of the beta gate packet | OPEN (72 cells) | 2026-10-19 | claude-opus |
| Evidence: register complete for shipped pairs, each PASS traceable to a live run | evidence | The register is complete and every cell is honestly UNVALIDATED: no live run has been made. Live cells need real Provider accounts and binaries with a sacrificial project, which agents never automate. This is the maintainer's dogfood work (AJ-E809) | PENDING (maintainer) | 2026-10-19 | claude-opus |
| Visual: the badge and drop list for one PASS and one UNVALIDATED pair, 1280x800, both themes, wording matching the register | visual | UNVALIDATED: gallery [`33-beta/badge-in-situ-1280x800-light.png`](../_gallery/33-beta/badge-in-situ-1280x800-light.png) (and `-dark`) shows "F2 · Beta · Not yet validated" in the switch wizard, asserted in `capture.spec.ts`. These are the register's words. The wizard used to hard-code "· beta" without consulting validation at all; it now reads the preflight's `validated` (the same strict verdict as the switch rows). PASS: none exists yet; it is captured with the first live PASS | PARTIAL | 2026-10-19 | claude-opus |

## Recording a live cell

1. Run the area on real installed Providers with a sacrificial project
   (pass2/02 §10 cases; deep/09 test plan). Keep the log or recording in
   `docs/qa/evidence/AJ-E808/cells/<from>-<to>-<area>.<ext>`.
2. Edit the cell in `data/fidelity-register.json`: `verdict`,
   `run: "live"`, `source_version`, `target_version`, `date`, `fixture`,
   `evidence`, plus `drop_note` for a FAIL, in the words the product
   should show.
3. `just fidelity-register`, then `cargo test -p juggler-handover`. The
   tests refuse a cell without full provenance.

## Status

IN-PROGRESS. The register, its rules and the automatic reset are
evidenced. Every live cell is the maintainer's to run.
