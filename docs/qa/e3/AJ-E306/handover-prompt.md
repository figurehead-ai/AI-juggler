# Handover prompt — give this to the next agent (AJ-E306 in progress)

Copy everything below the line into the new agent's first message.

---

Continue building AI Juggler autonomously (repo ~/Dev/AIJuggler). Cold-start protocol: read docs/tasks/AGENT-HANDOVER.md, docs/tasks/TASK-CATALOG.md, docs/review/decision-log.md (D-01..D-16 are law), then `git status` — a branch has in-progress work: **continue `stage3/e3-e306-flows-cde`** (task AJ-E306, flows c/d/e). Implement it fully to its SHALLs (enterprise grade, no shortcuts), run `just verify` + `pnpm --dir ui build` (+ full Playwright for UI changes), store evidence in docs/qa/evidence/AJ-E306/manifest.md, flip the task to DONE-EVIDENCED in the same change, regenerate the evidence + gallery indexes BEFORE the final verify, PR → watch all 5 CI checks (build-macos/build-linux/build-windows/ui/verify; verify ~13 min, poll ~every 8 min) → squash-merge --delete-branch → sync main. End with a short report: task(s) merged, evidence counts, next queued task; or "BLOCKED: reason, AJ-ID".

Never touch live user data (~/.claude, t3 profiles, real app data beyond cleaned test artifacts); never automate/read vendor credentials; never weaken or skip a gate (flaky gate = recorded fix/quarantine per AJ-E806); no force-push; no secrets; keep main green. After AJ-E306: continue AJ-E307, E308, E309 (§6 order), then E7, E10, E8 tail. If all E1..E10 tasks are DONE-EVIDENCED and docs/qa/beta-exit.md gates are green, report "PROGRAMME COMPLETE — awaiting maintainer beta sign-off" and stop.

**Environment (load-bearing):**
- This Mac (CLT) REQUIRES `export RUSTFLAGS=$(./scripts/ensure_swift_runtime.sh | tail -1 | sed 's/^RUSTFLAGS=//')` before EVERY cargo command.
- `just verify` = fmt --check → clippy -D warnings → cargo test → cargo doc (RUSTDOCFLAGS -D warnings; never leave a literal `<X>` in a /// doc comment — CI rustdoc rejects it) → evidence_check → terminology_lint. Task flip + `./scripts/evidence_index.sh` + `./scripts/gallery_index.sh` must run BEFORE the final verify.
- CI = 5 checks on PRs; squash-merge --delete-branch. AJ-E806 flake protocol: record in the manifest, verify 3/3 standalone, CI is arbiter. Known flake classes: the supervisor soak p95-tick budget under parallel UI-suite load, and a light-theme first-scan color-contrast flash while the theme class lands mid-boot (use the `scanSeriousWithConfirm` helper in a11y.spec.ts — never assert on the pre-recheck array).
- UI: `pnpm --dir ui build` (tsc + vite), `pnpm --dir ui test` (vitest, currently 272/272), `pnpm --dir ui exec playwright test tests/visual/` (currently 483/483). Gallery dirs are numbered — E306 is `37-flows`'s successor: add `38-flows-cde` (mkdir list lives in capture.spec.ts beforeAll). Full-suite runs re-capture ALL gallery pngs (~230 byte-drift files) — commit them all (established practice).
- Contrast law: `text-faint` fails AA on canvas — use text-muted/ink; success token on light is #065f46.

**State as of handover (2026-10-17):**
- main = `a1b52a3` (74/107 DONE-EVIDENCED ≈ 69%; E1/E2/E4/E5/E6/E9 complete, E3 5/9, E8 3/10, E7/E10 untouched).
- Branch `stage3/e3-e306-flows-cde` (pushed) has TWO commits:
  1. `6165afe` — `docs/qa/e3/AJ-E306/plan.md`: the full recon + step-ordered implementation plan. READ IT FIRST.
  2. `b0be194` — step 1 DONE and green: the SPEC §5.2 admission verdict wired end-to-end — `headroom_tokens` on `WindowDisplay` (cap−burn, None when cap unknown), `admission: Option<String>` ("ok|tight|no") on `RankedRowInfo` computed via core's pure `admission_verdict()` inside `ranking_rows_inner` (project-scoped median burn vs min-window headroom), `RankedRowData.admission` + `WindowDisplay.headroom_tokens` in bridge.ts, and test `admission_verdict_rides_the_ranked_rows_e306` (ranking_alerts_guardrails.rs 5/5: unknown→None, cap10k+burn2k→ok, cap800→no). Working tree is clean on the branch.
- Remaining E306 work (steps 2–6 in the plan file): the AddProviderSheet (flow c: adapter cards with verified_against badge + desktop kind/limit, provider-scoped scan, ≤4 clicks), the wizard provider-lock mode (flow d, ≤3 clicks), the launch-sheet verdict render + logged-out "Sign in first" detour-return + "headroom unknown" honesty (flow e, ≤2 clicks), the "Profile never a noun" + no-fabricated-% regression tests, Playwright click-count specs, gallery 38, a11y, then gates → evidence → flip → PR.
- Reference implementations to copy patterns from: E305's `NewProjectSheet.tsx` + `new-project-sheet.test.tsx` + `new-project.spec.ts` + `wizard-first-install.spec.ts` + the funnel seam `ui/src/lib/funnel.ts`; E304's wizard proposal block; the `__AIJ_RECIPES__`/`__AIJ_RANKING__`/`__AIJ_QUOTA_DISPLAY__` window seams for Playwright control.
- E303/E304/E305 landed the wizard's current shape: steps install→detect→identity→login→signins→summary, zero-found collapse, proposal + sequential accept, summary exits (launch/add-another/done), sessionStorage draft restore (aij.wizard.state.<provider>).

Work strategically: finish E306 completely on the branch before its single PR; batch later tasks per PR only when practical.
