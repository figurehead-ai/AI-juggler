# Evidence manifest — AJ-E808

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E808
(provider-pair fidelity validation matrix — operations). 2026-10-19,
reviewer claude-opus; brought up to date 2026-11-07 (release/0.4.1), when
the live labs since had decided 25 cells and the rest were placed.

Evidence: the register [`register.md`](register.md), rendered from
`crates/juggler-handover/data/fidelity-register.json` by
`just fidelity-register`. A test fails when the document drifts from the
data.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| R1 (Req.1): every ordered pair is covered on transcript resume, rules carry, todos, subagents, file history, MCP references, cwd/worktree edges (dotted and long paths) and switch-back A→B→A | Req.1 | 11 pairs (every row of the validation matrix) × 8 areas. `the_committed_register_is_well_formed_and_covers_the_matrix` fails if a pair or an area is missing or duplicated | PASS | 2026-10-19 | claude-opus |
| R2 (Req.2): each cell is PASS/FAIL/UNVALIDATED with binary versions, date, fixture ID and evidence path; UNVALIDATED is the default and shows as unvalidated in the product | Req.2 | `register::Register::problems` rejects a PASS or FAIL missing any of those. On 2026-10-19 every cell was UNVALIDATED; on 2026-11-07, 25 are decided live (each with versions, date, fixture and evidence) and every other one stays UNVALIDATED. The register shows the product's exact words, rendered by the same code: "F1 · Beta · Not yet validated" (switch rows) and "Beta — not yet validated" (Settings › Beta) | PASS | 2026-10-19 | claude-opus |
| R3 (Req.3): live-binary cells only; fixture and fake-harness results never promote a cell | Req.3 | A PASS or FAIL needs `run: "live"` (`only_live_runs_with_full_provenance_decide_a_cell`). `validated()` now also requires every area of the pair PASS in the register, so a live cell recorded in the app can no longer promote a badge on its own (`validation_gate` and `validation_matrix` tests updated) | PASS | 2026-10-19 | claude-opus |
| R4 (Req.4): a Provider upgrade resets affected cells automatically | Req.4 | Re-checked 2026-11-07 against the installed Claude Code 2.1.310 and codex-cli 0.163.0: Claude→Claude's seven cells recorded on 2.1.302 had reset; lab 19 (2026-10-28) ran them on 2.1.310 and they're re-dated (32b5f492b). `Register::effective` reads a PASS recorded against other versions than the installed ones as UNVALIDATED ("reset: claude-code is 2.2.0, the run used 2.1.296"). The app passes the versions found by Provider discovery and evaluates strictly: an unknown version can't confirm a PASS (`a_provider_upgrade_resets_its_cells_by_itself`). Discovery runs only when the register has a decided cell for the pair, so there's no added cost today | PASS | 2026-10-19 | claude-opus |
| R5 (Req.5): beta-exit — same-Provider pairs PASS on every cell; cross-Provider pairs PASS or FAIL with a drop list | Req.5 | `Register::beta_exit_open` (`beta_exit_needs_same_provider_pass_and_cross_provider_drop_notes`, pinned at 25 decided cells). Claude Code ↔ Codex both ways: every area PASS or FAIL with a drop note (Claude→Codex subagents and file history, lab 14; Codex→Claude subagents and file history, J2 2026-11-07, 8ce5b7240). Claude→Claude: seven PASS on 2.1.310, file history UNVALIDATED (a second Claude Code terminal sign-in; proven on fixtures). Codex→Codex: two PASS, six deferred by D-91. ZCode and the "+service" pairs: credential-only. Every remaining cell is in `docs/qa/0.4-completion.md`, "Credential-only steps" | PASS (shipped Claude/Codex pairs closed or placed) | 2026-11-07 | Claude (self) |
| Evidence: register complete for shipped pairs, each PASS traceable to a live run | evidence | Each decided cell names its lab and evidence file (AJ-E1539's labs 5–14 and 19; `cells/codex-claude-code-2026-11-07.md`). Live runs now go through the switch lab (`switch_lab_live.rs`, the clean `lab_env.sh`), on the Lab CLI App (ADR-0027 7b) and the Mac's Codex home; never `bookauth-luka`'s terminal sign-in or a `/login` | PASS | 2026-11-07 | Claude (self) |
| Visual: the badge and drop list for one PASS and one UNVALIDATED pair, 1280x800, both themes, wording matching the register | visual | Retaken 2026-11-07 (`capture.spec.ts` "beta —", the matrix now scrolled into view): `_gallery/33-beta/matrix-validated-1280x800-{dark,light}.png` "Proven ✓"; `matrix-default-*` "Beta — not proven yet"; `badge-in-situ-*` "Same conversation, another app · Beta · Not proven yet" with its drop list. These are the register's "Settings › Beta shows" and "Switch rows show" words, rendered by the same code (`register.md`). The real register can't be shown through the lab bridge: `validation_matrix` is a Write command (it may run Provider discovery), so the bridge refuses it, rightly. No pair is proven in the real register yet (every pair has a FAIL or an open area) | PASS | 2026-11-07 | Claude (self) |

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

2026-11-07: the register, its rules and the automatic reset are evidenced,
and every shipped Claude Code / Codex cell is decided live or placed: the
cross-tool pairs closed (PASS or FAIL with a drop note), Claude→Claude's
file history fixture-proven and waiting for a second Claude Code terminal
sign-in, Codex→Codex beyond lab 11 deferred (D-91). ZCode's and the
"+service" pairs wait for an account (credential-only steps). The badge's words
are captured in both themes (33-beta).
