# Evidence manifest — AJ-E803

Task: `docs/tasks/E8-validation-visual-performance-quality.md` AJ-E803
(visual regression thresholds and review workflow). 2026-10-19,
reviewer claude-opus. Runs: [`gate-runs.txt`](gate-runs.txt). Self-test
triptych: [`visual/`](visual/). **Status: IN-PROGRESS**: the gate is
live, but it covers the core matrix, not yet every gallery cell.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| Cells compared pixel by pixel with approved baselines: ≤ 0.1 % differing pixels, 0 for text-bearing chrome | Req.1 | `ui/tests/visual/regression.spec.ts`: 96 baselines. Ten surfaces (home with rail, the Instance inspector, Triage, Overview, History, Accounts, Usage, Fleet, Cleanup, Settings) × dark/light × comfortable/compact × 1280×800/1024×680 at `maxDiffPixelRatio 0.001`; rail tabs and a nav button at `maxDiffPixels 0`. Rendered in the pinned container (`scripts/visual_gate.sh`), so baseline and comparison share one renderer: two clean runs pass 81/81, and baselines rendered on arm64 compared on amd64 passed too. **Not yet:** the rest of the evidence gallery (≈ 840 captures, several with live counters), which stays evidence-only until each family is made deterministic and added | PARTIAL | 2026-10-19 | claude-opus |
| A failure produces a triptych (baseline, current, diff), stored and linked; never dismissed in chat | Req.2 | Playwright writes `-expected`/`-actual`/`-diff` per failing cell; CI uploads them as the `visual-triptychs` artifact and lists each failure in the job summary (`scripts/visual_summary.py`). The self-test's triptych is in `visual/` | PASS | 2026-10-19 | claude-opus |
| Baseline updates need a PR section with the task ID, the intentional change per screen, and maintainer approval; bulk updates fail | Req.3 | `scripts/check_baseline_notes.py` (CI `visual` job, on PRs): every changed baseline needs a line naming it with a note, under "## Visual baselines" with an AJ-E task ID. Tried locally: one note for 56 changed files fails, naming the 55 missing. PR templates carry the section; QUALITY.md §5 states the rule, and that raising a threshold needs recorded maintainer approval. This PR is the first baseline PR and notes all 96 | PASS | 2026-10-19 | claude-opus |
| A colour-only state change fails even when its pixel diff is small | Req.4 | `status is never colour-only`: every one of the nine statuses must appear as words or in an accessible name | PASS | 2026-10-19 | claude-opus |
| Runs on UI-changing PRs only, < 5 min, posts a summary | Req.5 | CI `visual` job: skips with a summary line when nothing under `ui/` changed; otherwise install + build + 81 tests (≈ 3 min emulated here; native amd64 in CI) and a passed/failed/not-captured summary | PASS | 2026-10-19 | claude-opus |
| Harness self-test: an intentionally shifted button fails; reverting passes | quality gate | `--selftest` (every button moved 2 px): 80 of 80 screenshot cells fail (6,150–8,166 differing pixels each); the clean tree passes 81/81 | PASS | 2026-10-19 | claude-opus |
| Baseline store < 100 MB | perf budget | 10 MB | PASS | 2026-10-19 | claude-opus |
