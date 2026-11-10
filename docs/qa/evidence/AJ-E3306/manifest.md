# Evidence manifest: AJ-E3306, the release candidate walked whole

Task: `docs/tasks/E33-validated-by-use.md` AJ-E3306. The plan is
`docs/qa/0.4-walks/release-candidate.md`. The rows below come from the
release session's walk results (2026-11-09..10), each checked by Opus, on
0.4.1 release candidates rc17 to rc31 of AI Juggler Dev built with `just app`
(never `tauri dev`). The final candidate is rc31 (35c181e2d): preflight full
green, static, bundle, first-open, `second_launch` 13/13. Walk captures show
real names and stay out of the repo.

## By requirement

| Req | Result | Evidence | Commit | Date |
|---|---|---|---|---|
| R1 every scenario walked again on the release-candidate build, in one run | PARTIAL | **Walked on release candidates** (table below): K1–K4 (rc24/25, findings re-walked on rc25, rc26, rc30); N1–N5 (rc26, N4's fix re-walked); P1–P4 and P6–P11 (rc26 lab, rc27 live, rc29 re-walks; AJ-E3304); PM3 and PF1 (rc26, PF1's fixes re-checked on rc30); the runtime lab (rc27, rc29, rc31); X1 and X2 (rc26, rc27); OpenCode and Grok live (rc17, rc20). **Not proven:** not one run (the rows span rc17 to rc31, with fixes between); A1–A10 walked on a 0.4.1 debug build with the lab bridge (2026-11-06), not on a release candidate; FL1 (the Floor) not walked, its sheet blank; P5 never walked (tests only); P1 S9's Continue, P1S6 S2, P8 S4 and P11 S1 not walked (see AJ-E3304); NY1 steps 1–2 not recorded as walked, step 3 not reachable live (aij-test conversations never read "Done, review it"); X1 S2 (below). Several sheets (X1, NY1, PF1, PM3, P1S6, the runtime lab) carry no results in the repo: their results are only in the walk results | — | 2026-11-10 |
| R2 X1 (a day: holds, a switch, notices, mutes, a cache clear, a crash and a restart) and X2 (a 50-member pool, 1 000+ ended sessions) | PARTIAL | **X1** (rc26 lab, rc27 live): S1, S3, S4, S5, S6 met. S2, an Account running out live and its session switched, not walked: no Account ran out during the walk; the out-of-room flow is walked only in labs (N5, P1S6 S1). **X2** (rc26): S1 opened in about 130 ms, over its 100 ms budget; fixed by the read-ahead c68fb8eb8 (`crates/juggler-app/tests/pool_open_budget.rs` `a_pool_of_fifty_opens_inside_its_budget`, 56 ms median), not re-walked on a candidate. S2, the pool's debug-log line missing: fixed in eb611a9c6 (`crates/juggler-app/tests/pool_debug_log.rs`), not re-walked on a candidate. S3 met: CPU peak 14 %, under 20 % for 2 minutes | c68fb8eb8, eb611a9c6 | rc26, rc27 |
| R3 a summary for the maintainer: what passed, what was fixed, what's known and why | NOT DONE | `release-candidate.md` §6 "The summary" is empty, and §2–§4's result columns are blank. The walk results exist only in the release session's scratch notes | — | 2026-11-10 |

## By scenario

| Scenario | Candidate | Observed | Verdict | Fix commits |
|---|---|---|---|---|
| K1 The chain, K4 Pickers | rc24/25 | met | PASS | — |
| K2 Every page, every tab; K3 Notices land | rc24/25 | partly met; findings fixed (rail counts, State counts, the teach card, Home's build line, the ⊖ mark) and re-walked, met | PASS (re-walked rc25, rc26, rc30) | — |
| N1, N2, N3, N5 | rc26 | met | PASS | — |
| N4 First time | rc26 | the newcomer's project was a temporary one; fixed | fixed, re-walked | c67692ed1 |
| P1–P11 | rc26, rc27, rc29 | see `docs/qa/evidence/AJ-E3304/manifest.md`; P1 S9, P1S6 S2, P5, P8 S4, P11 S1 not walked | PARTIAL | b6ad1013a, b00b60179, 72bf704da |
| PM3 Which account has room | rc26 | met | PASS | — |
| PF1 Persona first run | rc26 | scored 3 on works; first run fixed and simplified (D-97), re-checked on rc30: empty Accounts, a three-step first run, Next steps, Rename account | PASS (rc30) | 5971d9fe3, 4049d6c56, e4c192799, 08199ac60 |
| The runtime lab | rc27, rc29, rc31 | steps 1, 4, 5 met on rc27; steps 2–3 sheet corrected (a503e72b5), met on rc29 (writes ask, Allow once); step 6a fixed (add8d201d), met on rc31 | PASS | a503e72b5, add8d201d |
| X1 A day | rc26, rc27 | S2 not walked live | PARTIAL | fe5853482 |
| X2 Load | rc26 | S1 and S2 fixed, proven by tests only | PARTIAL | c68fb8eb8, eb611a9c6, 000875428 |
| NY1 Archive leaves Needs you | — | sheet and scenario made (69a6ab661); no walk recorded | NOT WALKED | 69a6ab661 |
| OpenCode and Grok live | rc17, rc20, rc27 | see `docs/qa/evidence/AJ-E3002/manifest.md` | PASS | 0c9a4711a |
| A1–A10 | 0.4.1 debug, 2026-11-06 | walked and fixed on the debug build with the lab bridge; not on a candidate | NOT ON A CANDIDATE | — |
| FL1 The Floor | — | not walked | NOT WALKED | — |

**The freeze report.** Its cause was the `second_launch` tests' visible windows (4668df70c, 5db41ffc7). No freeze in any walk, with a sample watch on.

## Still to do for DONE

- R1: A1–A10, FL1, NY1 and P5 walked on a candidate; P1 S9, P1S6 S2, P8 S4 and P11 S1 walked or moved by the maintainer; and the whole set walked on one candidate build.
- R2: X1 S2 walked live (an Account out, its session switched); X2 S1 and S2 re-walked on a candidate.
- R3: `release-candidate.md` §6 written.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] No captures here
- [x] No conversation content
