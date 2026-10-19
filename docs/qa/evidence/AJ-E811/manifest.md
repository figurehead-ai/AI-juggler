# Evidence manifest — AJ-E811

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E811
(full-sweep visual check, decision D-21). 2026-10-19, reviewer
claude-opus. The sweep's reports themselves are not committed (D-21:
no pile of reference images); their summaries are, verbatim, plus one
triptych of the deliberate change.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 — the whole gallery is rendered at the merge-base and at the head in the pinned container and diffed state by state; no stored baselines | `just visual-sweep origin/main HEAD` (`scripts/visual_sweep.sh`; `scripts/visual_gate.sh --full-sweep` hands off to it) | every gallery state compared | `mcr.microsoft.com/playwright:v1.64.1-noble`, native arm64; 860 gallery tests per render, 921 states, each side rendered twice; 8 min 37 s on the maintainer's Mac. The report: `summary.md`, `summary.json`, `index.html`, `states/…{base,head,diff}.png` under `target/visual-sweep/<base>..<head>/` | PASS | feeb5255 | 2026-10-19 | claude-opus |
| Noise floor — base = head reports no changed state | `scripts/visual_sweep.sh --base HEAD` at dd494e6c | 0 changed | First version (one render a side): 62 changed, mostly 1–2 px anti-aliasing at the edges of rail controls. Rendering each side twice: 10 changed, 55 unstable. Measured the cause: with Chromium's partial raster on, 12 of 22 rail states varied over five identical runs; with `--disable-partial-raster`, 0. Final: **0 changed**, 2 unstable (a live "N of 10" progress counter), 919 unchanged — [noise-run-dd494e6c.md](noise-run-dd494e6c.md) | PASS | dd494e6c | 2026-10-19 | claude-opus |
| Req.2 / quality gate — a deliberate one-pixel change shows up locally | Scratch commit bd82c7ac: `RecoveryCodeSheet` button row `gap-2` → `gap-[9px]` (a screen in the gallery, not in the core baselines); `just visual-sweep origin/main HEAD` | every state that shows the sheet listed as changed, with images | The 4 recovery-code states (`59-vault/recovery-code-*`, `59-vault/confirmed-*`, both themes) changed: 361–367 px in the 66×30 region of the Save button. Triptych: [base](scratch-recovery-code-base-bd82c7ac.png), [head](scratch-recovery-code-head-bd82c7ac.png), [diff](scratch-recovery-code-diff-bd82c7ac.png). Also listed: one live-counter state (see Known limits). Summary: [scratch-local-bd82c7ac.md](scratch-local-bd82c7ac.md). Reverted in the next commit | PASS | bd82c7ac | 2026-10-19 | claude-opus |
| Req.3 — the same in CI, against a real base | `visual-sweep` workflow, [run 39256249682](https://github.com/figurehead-ai/AI-juggler/actions/runs/39256249682), on the scratch branch `claude/visual-sweep-ci-proof` (the scratch change, plus a temporary `push` trigger: `workflow_dispatch` only works once the file is on `main`) | same states caught | Base dd494e6c (merge-base with `main`), head 33902d32: the same 4 recovery-code states, with the **same pixel counts as the arm64 run** (361/367). Two live-counter false positives (see Known limits). Wall time 13.5 min: refs 14 s, four shards 2.6–12 min, report 1 min. Summary: [scratch-ci-33902d32.md](scratch-ci-33902d32.md) | PASS | 33902d32 | 2026-10-19 | claude-opus |
| Req.3 — dispatch inputs resolve a pull request, a branch or the run's own ref | The `Resolve base and head` step run locally against `origin` | correct SHAs; bad input fails | `#41` → head f24a8106 (PR head), base ae37c7bd (merge-base); `pull/40` → 63cbee7b / 5fae76a1; empty → the run's own commit; `tip=true` → `main`'s tip; `#abc` → `::error::Not a pull request number`, exit 1; an unknown branch → exit 128. A real `workflow_dispatch` run needs the file on `main` first | PASS | 40247e0f | 2026-10-19 | claude-opus |
| Req.3 — never a required check; the core gate is unchanged | `.github/workflows/visual-sweep.yml`; `scripts/visual_gate.sh`; `just visual-gate` | `visual` untouched and green | The workflow has only `workflow_dispatch`; its jobs are `sweep refs`, `sweep render (n/4)`, `sweep report` — none named `visual`. `visual_gate.sh` gained one line before its own argument loop (`--full-sweep` hands off); the core path is byte-for-byte the same. `just visual-gate`: 81 passed | PASS | e785df37 | 2026-10-19 | claude-opus |
| Req.4 — QUALITY.md says when to use which mode | `docs/plan/QUALITY.md` § Visual checks: which mode when (D-21), the §5 table and the gate vocabulary; `docs/MAINTAINER-STEPS.md` step 15; `CONTRIBUTING.md` | both modes and their triggers named | Core: every UI pull request, required. Full sweep: a screen first built or rebuilt (the first full pass sweeps every screen), changes to shared tokens/layout/UI dependencies, and on demand | PASS | feeb5255 | 2026-10-19 | claude-opus |

## Known limits

- **Live-progress captures.** A few gallery states photograph a counter
  or progress dialog mid-run (`34-wizard/scan-progress`,
  `36-recommend/proposal`, `37-flows/a2-proposal`, `37-flows/a4-signins`,
  `63-consent/first-run`). Usually the double render marks them
  unstable; when the counter happens to land on the same tick twice on
  each side, one shows as changed with a tiny region (here: 37–44 px,
  and 2,422 px for the sign-in progress row). The fix belongs in those
  capture tests (wait for the run to finish, or pin its timer) — a
  follow-up, not a sweep setting.
- **Motion.** The sweep forces reduced motion, so a change only to an
  animation or transition doesn't show.
- **CI shards are uneven** (2.6–12 min): Playwright splits tests by
  count, and the gallery's tests differ a lot in length.

## Binary artifacts

Binary evidence is content-addressed: `<claim>-<sha8>.png|log|json`.
Re-running a check ADDS a new file; cited artifacts are never
overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
