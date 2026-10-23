# Evidence manifest — release-0.1

The 0.1 acceptance checklist (`docs/qa/0.1-acceptance.md`), run by the
coordinator on the release head. Journeys and the soak run on replicas
(`scripts/replica.py`, throwaway HOMEs), never on real data. The
real-data pass (§3) is the QA session's, in `release-0.1-real/`.

Head: release/0.1 at `f4181eee` plus this branch's fixes (below); each row names the commit its build was made at. E1522
(#169) changes tests only; a PASS here still counts on a head that adds
it.

How the replicas were driven: the macOS window can't be captured here
without a Screen Recording grant, so the UI was walked through the
read-only Dev bridge (`docs/qa/DEV-BRIDGE.md`), a build with its own
identifier beside the soak. A replica's setups were brought in by the
product's own first-run code (`qa_real_readonly`'s import, which is what
"Use them where they are" calls), straight into the Dev data folder the
app then opened.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| 2.2 upgrade from 0.0.12 | v0.0.12's Dev build (own identifier) brought in a fresh replica's setups with its own first-run import and ran 120 s beside 8 live sessions; the 0.1 build then opened the folder for 120 s. Row counts of every table read from a backup copy, before and after: [`upgrade-0.0.12-to-0.1.txt`](docs/qa/evidence/release-0.1/upgrade-0.0.12-to-0.1.txt) | schema migrates; the pre-migration copy kept; nothing lost | schema 29 → 32, integrity ok; `juggler.sqlite.before-v29` kept; accounts 9 → 9, launch settings 9 → 9, usage events 3404 → 3404. Projects 18 → 16: the three agent worktrees are folded into their repos (AJ-E1519), their sessions with them (301 sessions known before, 345 after: Codex-only work is found too) | PASS | f80fa4f3 | 2026-10-23 | coordinator |
| 2.3 many accounts plus Codex | the replica (7 Claude, 2 Codex) brought in where they are, then `real_truth.py --compare`: [`replica-oracle.txt`](docs/qa/evidence/release-0.1/replica-oracle.txt) | every account found and brought in; the backend matches the oracle | 9 of 9 found, 9 of 9 in place; 16 projects and every project's conversations as the oracle; each account's tokens ×1.00; Codex all-time ×1.00; 340 sessions' costs within 5 %. One MISMATCH: 43 of 398 cost rows unpriced, all `gpt-5-codex`, a model the replica names that today's price list doesn't carry (shown honestly as no cost; real rollouts are §3.2's) | PASS | 58dc1043 | 2026-10-23 | coordinator |
| 2.4 live sessions outside AI Juggler | the bridge walk with 8 live fake sessions | listed under their projects and opened | Home: "8 running", each "outside AI Juggler" with its account; the rail lists them under their projects; a session's page opens with its folder, branch and tokens, and Continue in AI Juggler waits, in words, while it's open elsewhere. Continuing is a launch, which the bridge refuses: AJ-E1529's tests cover it | PASS (continue: AJ-E1529) | 7f1168e7 | 2026-10-23 | coordinator |
| 2.5 memory pressure | the same walk; the host Mac was under memory pressure | one alert, the same figures everywhere, This Mac | one alert; the alert, Home's This Mac card and the This Mac page all read the same GB for the Claude app's 8 windows, grouped as one row | PASS | 7f1168e7 | 2026-10-23 | coordinator |
| 2.6 permissions | `os_access_list` through the bridge; Home's Make it smoother card | each optional permission; Screen Recording not offered without a live-preview tool | one offered (jump to a session's window: Accessibility and Automation); Screen Recording not listed. Granting needs a macOS Allow, which is the maintainer's: AJ-E1527's tests | PASS (grant: AJ-E1527) | 7f1168e7 | 2026-10-23 | coordinator |
| 2.8 going back | the product's `going_back_prepare_impl(state, 29)` on the upgraded folder, then the real v0.0.12 build opened it for 40 s (same file) | a working 0.0.12 | the schema-29 copy in place, the newer data kept beside it; 0.0.12 ran, with schema 29, 9 accounts, 18 projects, 3404 usage events, integrity ok: exactly its data before the upgrade | PASS | f80fa4f3 | 2026-10-23 | coordinator |
| 5.2 50 sessions, 15+ projects | `cargo test --release -p juggler-app --test load_budgets -- --nocapture --test-threads=1`, and `ui/tests/visual/load.spec.ts` (10 Accounts, 50 sessions, 20 projects): [`load-budgets-release.txt`](docs/qa/evidence/release-0.1/load-budgets-release.txt) | the SPEC budgets | This Mac at 50 sessions 1.6 ms; the rail paints all 50 in 38 ms (300), a session's page 40 ms (200), search 7 ms (200), Cleanup 36 ms (300), scroll to the last 17 ms (300); layouts hold at scale in both themes at 1440 and 1024 | PASS | d95e97eb | 2026-10-23 | coordinator |
| 5.3 cold start | the same runs | backend 200 ms; webview 2.5 s | backend at scale 12.9 ms median; webview to the painted rail 132 ms. Against the 2026-10-19 reference (64 ms) that's slower, and real (the old UI, run now on the same Mac, paints in 62 ms): 0.1's added UI. Whether to move the baseline is the maintainer's | PASS (budget) | d95e97eb | 2026-10-23 | coordinator |
| 5.4 monitoring under 1 % of a core, release build | `CARGO_INCREMENTAL=0 cargo test --release -p juggler-app --test load_budgets monitoring -- --nocapture` (in the run above) | both under 1 % | 0.344 % (5 sessions), 0.405 % beside a busy Claude app (8928 of its processes read), at a load of 13 on 30 cores | PASS | d95e97eb | 2026-10-23 | coordinator |
| 6.1 Dev bridge absent from release | `./scripts/dev_bridge_check.sh` | exit 0 | "off by default, in no release workflow, refused by release builds" | PASS | 7f1168e7 | 2026-10-23 | coordinator |
| 6.2 capability lint | `cargo test -p juggler-app --test trust` (`the_window_can_reach_nothing_but_the_app`), now also refusing a capability's `remote` URLs (7f1168e7); proven by planting one | no shell, fs, http, opener or remote | 11 passed; the planted `remote` failed it | PASS | 7f1168e7 | 2026-10-23 | coordinator |
| 6.3 guardrails | the tests the Privacy page's "What it changes" lines name, and the guardrail suites: [`security-tests.txt`](docs/qa/evidence/release-0.1/security-tests.txt) | all pass | 94 tests in 11 binaries, all passed | PASS | 7f1168e7 | 2026-10-23 | coordinator |
| 6.4 release signing split | AJ-E2001 is parked, not on main | only if on main | not on main | PASS (n/a) | 7f1168e7 | 2026-10-23 | coordinator |

## Not a product failure

§3.2's 12 cost rows reading high (up to ×1.65) are conversations a switch copied into a second account. The app leaves a copy's lines older than the switch out of the target's usage, from its own switch records (P19, `tests/usage_carried.rs`). The real-data harness starts from an empty data folder, which has no switch records, so it can't tell a copy from a conversation. To check it on real data, the harness would need the real data folder's switch records (a read-only copy).

## Found and fixed on the way

| # | Found | Fix |
|---|---|---|
| F1 | First run said "No Claude Code setups found yet." when looking for setups failed, and offered to add an account, on a Mac with seven | 58dc1043: it says AI Juggler couldn't look, gives the reason, and offers Try again beside adding or importing (vitest) |
| F2 | The Dev bridge refused 11 of release/0.1's reads (monthly caps, the search index's status and estimate, the switch set-ups…), so QA couldn't walk them | f80fa4f3: routed, and a new test makes every Read either served or named as not served, with why |
| F3 | The capability lint didn't refuse remote URLs | 7f1168e7 |
| F4 | The load gate's Cleanup timing counted opening More (Cleanup moved there after the baseline) | d95e97eb: timed from its own click |
| F5 | (§3.1, real data) Codex subagent threads were listed as conversations of their own: 399 extra on one account | 5ebfbe61: a rollout whose `session_meta` names `source.subagent` is part of the conversation that started it, as Claude Code's subagents are; its usage still counts |
| F6 | macOS ran out of application memory, juggler-app near 42 GB: the maintainer's Mac has a 5 GB Codex rollout and Claude transcripts of 200–670 MB, and listing conversations read each rollout whole and copied it into a parse (6.9 GB peak in a test) | 486980e7, e12f9022, 646b309e: every routine read holds one line or one chunk at a time (conversation lists, the transcript watcher, usage ingest, the session page's quality scan, the model scan, a switch's checks, ranking's estimate); what still needs a whole parse is refused past 512 MB, in words. A synthetic 2 GB rollout lists in 1.6 s at a 39 MB peak, tokens exact; streamed reads equal the whole-file code on 500 mixes of lines |
| F7 | (§3.4, real data) This Mac's memory used read 3.0–3.4 GB low (3.2 % of physical memory) against `vm_stat`: its compressed term was the compressed data, not the pages the compressor occupies | D-27 (the maintainer's approval): `juggler-vm-stats`, an audited `host_statistics64` shim; its test holds it to `vm_stat`'s "Pages occupied by compressor" |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content
