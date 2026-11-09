# Evidence manifest — AJ-E3308

Task: `docs/tasks/E33-validated-by-use.md` AJ-E3308, Releases that don't stall (ROADMAP 0.4's row; #198's follow-up; ADR-0025). Status: **DONE-EVIDENCED**: built across 0.4's release work; this manifest gathers it for 0.4.1's release check.

## What exists now

`justfile` (`preflight`, `preflight-full`, `verify-static`); `docs/adr/0025-heavy-gates-on-github-runners.md`; `.github/workflows/ci.yml` (cargo-nextest's archive built once, its partitions, the visual and accessibility jobs); `docs/RELEASING.md` (step 4, the merge commit); `docs/plan/QUALITY.md` (Determinism, #198); `scripts/machine_state_lint.py`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a fast local preflight | `justfile` recipes `preflight` (line 83: static checks, then the packages a change can affect) and `preflight-full` (line 89, the old serial run); `docs/RELEASING.md` step 2 ("Before each push: `just preflight`") and its timing table (3–5 min on a static failure, 15–21 min when every package's tests run, against about 55 min serial) | a scoped run, the full one kept | both recipes present and documented; used for every 0.4.1 push | PASS | d881d5a65 | 2026-11-09 | Opus (verified) |
| R2 the heavy gates on GitHub's free runners | `docs/adr/0025-heavy-gates-on-github-runners.md`; `.github/workflows/ci.yml` (the test binaries built once as cargo-nextest's archive, line 290, downloaded by every partition; the visual and accessibility gates as their own jobs); `python3 scripts/ci_parity.py` (`just verify` runs every command of CI's verify jobs) | the heavy work off the Mac; one definition of each gate | ADR and jobs present; ci-parity OK | PASS | d881d5a65 | 2026-11-09 | Opus (verified) |
| R3 a release PR merged as a merge commit, pinned to the commit CI passed | `docs/RELEASING.md` step 4 (`gh pr merge <PR> --merge --match-head-commit <sha>`); read-only `gh api repos/figurehead-ai/AI-juggler/rulesets/25344214` ("main protection": `pull_request` allows `merge` and `squash`) and `gh api repos/figurehead-ai/AI-juggler` (`allow_merge_commit: true`) | main's ruleset allows merge commits | allowed (2026-11-09) | PASS | d881d5a65 | 2026-11-09 | Opus (verified) |
| R4 deterministic tests | `docs/plan/QUALITY.md` "Determinism (#198)" (no timer waits for what a test can be told; machine state through a seam, `scripts/machine_state_lint.py` with an allowlist that only shrinks; one UI wait setting, `ui/src/test/setup.ts`; a test that fails once is fixed or quarantined the same day); `python3 scripts/machine_state_lint.py` | the rule written and enforced | lint passes | PASS | d881d5a65 | 2026-11-09 | Opus (verified) |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths generalised where they reveal private layout (none)
- [x] No transcript excerpts
