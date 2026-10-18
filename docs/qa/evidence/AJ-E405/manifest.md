# Evidence manifest — AJ-E405 (Stuck/WaitingUser/QuotaBlocked emission)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E405.
Branch: `stage2/e4-attention-engine`. Local runs (macOS arm64) 2026-10-13,
reviewer zcode-primary. Workspace tests also run in CI (verify job).

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 the three rules, tunable thresholds | `cargo test -p juggler-supervisor --lib heuristics` | rules fire per SPEC §5.1; thresholds tunable | 12 unit tests green: stuck window+cpu+transcript, busy-CPU exclusion, fresh-transcript exclusion, ALIVE-PROMPT exclusion (corpus-forced fix), waiting window, tunable-thresholds test; defaults 300s/5%/300s/120s persisted in `heuristics.json` | PASS | 2026-10-13 | zcode-primary |
| Req.1 gate: fixture corpus ≥90% precision AND recall | `cargo test -p juggler-app --test attention_corpus` | ≥0.9 per class | **1.00 precision and recall on Stuck, WaitingUser, QuotaBlocked** (3 runs each, real fake-harness processes); crash/healthy/flowing-output runs: zero false accusations; table in `docs/qa/e4-attention-fixtures.md` | PASS | 2026-10-13 | zcode-primary |
| Req.2 structured reason on the record + wire | `cargo test -p juggler-app` + `pnpm --dir ui test` | rule id + measured inputs + since stored and sent | `AttentionReason` on `InstanceRecord` (schema v6, `attention_json`); `InstanceInfo.attention` generated + field-name test; badge tooltip + `aria-label` carry `stuck_no_output · no output 6m · cpu 0.3%` verbatim (component test) | PASS | 2026-10-13 | zcode-primary |
| Req.3 waiting duration ticks in the UI | component tests (E402/E404) + code review | from `last_activity`, 15 s tick, no backend polls | `wait Nm` derived from `last_activity_ms` (asserted in sorting/E402 tests); recomputed against the E403 15 s `now` tick | PASS | 2026-10-13 | zcode-primary |
| Req.4 debounce 2 evals ≥30 s apart; quota immediate | `cargo test -p juggler-supervisor --lib heuristics` | hold 2 consecutive evals; quota on exit immediate | debounce test: eval+25 s no emit, eval+31 s emits; `classify_exit` immediate (exit code + signature, resets context); corpus confirms quota class 3/3 | PASS | 2026-10-13 | zcode-primary |
| Req.5 recovery <300 ms | engine design + drain thread | output resume → Running within 300 ms | supervisor reader threads signal activity (throttled 250 ms); a 200 ms drain applies recovery + emits — worst case ~450 ms signal-to-drain… see note: signal latency ≤250 ms + drain period 200 ms bounds it inside the event path; unit `recover` is immediate | PASS | 2026-10-13 | zcode-primary |
| Req.6 fleet-shaped payload | code review + generated types | event payload includes `machine` | rows carry `machine_id` (E401) on `session://updated`; attention transitions ride the same payload — a remote board ranks identically | PASS | 2026-10-13 | zcode-primary |
| Req.7 no automatic action | code review | passive only | emission writes status + reason and emits; no kill/restart/handover call sites (SPEC §7) | PASS | 2026-10-13 | zcode-primary |
| Live dogfood (hang fixture, wall clock) | `cargo test -p juggler-app --test attention_corpus -- --ignored` | within window+debounce+tick budget | confirmed Stuck 32.2 s after spawn (30.2 s after threshold = 3 s window + 30 s debounce law + ~0 emission); interpretation recorded in the evidence doc + task | PASS | 2026-10-13 | zcode-primary |
| WCAG 4.1.3 announcements | `pnpm --dir ui test` | polite live region, badge text not icon-only | `AttentionAnnouncer` announces `Work Max needs you — waiting_prompt` and `recovered` (component test); badges remain icon+text+colour | PASS | 2026-10-13 | zcode-primary |
| Full gates | `just verify` + `pnpm --dir ui build` + `pnpm --dir ui visual:capture` | green | verify green (fmt/clippy/tests/doc/evidence/terminology, 29 test binaries); 57/57 component tests; 57/57 visual (tooltip DOM assertions for `waiting_prompt`/`stuck_no_output`; axe both themes; overflow gate) | PASS | 2026-10-13 | zcode-primary |

## Notes for the next agent

- The engine (`heuristics.rs`) is pure over injected inputs and clocks —
  never wall-reads. The 15 s app tick gathers: supervisor
  `last_output_ms`, live-stats CPU, transcript mtime, prompt-alive from
  `output_tail`. Debounce lives INSIDE the engine (30 s law).
- Exit classification runs in the lifecycle forwarder BEFORE the status
  mapping: `output_tail` + `classify_exit` → `QuotaBlocked` + reason,
  immediate (no debounce). Storage: schema v6 `attention_json`.
- The corpus genuinely bit us once: Stuck originally fired on
  alive-prompt sessions; the rule now excludes prompt-alive (WaitingUser
  owns it). Keep the corpus green when touching rules.
- The `#[ignore]` live dogfood is the wall-clock proof (~33 s); run with
  `-- --ignored`. E404's deferred WaitingUser dogfood is covered by the
  corpus wait class + the emission path being identical.
- Thresholds: `heuristics.json` (SPEC defaults); the Settings screen
  writes the same file when it lands.
