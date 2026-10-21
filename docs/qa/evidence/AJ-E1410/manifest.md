# Evidence manifest — AJ-E1410

Task: `docs/tasks/E14-machine-health.md` AJ-E1410 (budget proof and the M2
resource-manager gate). Status: **DONE-EVIDENCED** for the budgets below.
The M2 milestone gate itself is the maintainer's.

| Budget (SPEC) | Command (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Resource monitoring < 1 % CPU (§1) | `cargo test -p juggler-app --test load_budgets monitoring -- --nocapture`: one simulated minute (30 ticks of the 2 s tick + 12 This Mac emissions with alerts and advice), 5 live sessions, this thread's CPU time; debug build, load average 15–23 | < 1 % of one core | 0.41–0.44 % over three runs (a first, colder run 0.71 %); **E14's machine half alone 0.033–0.036 %**, the rest is the pre-existing per-session collector | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| This Mac aggregation ≤ 200 ms at 50 Instances (§5.3) | `cargo test -p juggler-app --test load_budgets this_mac -- --nocapture` | ≤ 200 ms | median 5.9 ms | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Machine tick ≤ 2 ms p95 (AJ-E1401) | `cargo test -p juggler-supervisor --test machine -- --nocapture` | ≤ 2 ms | p95 31–72 µs | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Running-tools scan ≤ 10 ms (AJ-E1402) | `cargo test -p juggler-app --lib machine_health::live -- --ignored --nocapture` | ≤ 10 ms | median 3.0 ms | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Advisor projection within ±20 % of observed peaks (§1) | `cargo test -p juggler-app --test machine_fixtures -- --nocapture` | ≤ 20 % | 8.8 % | PASS | 42012b58+ | 2026-10-20 | claude-opus |

Not measured here: a 10-minute wall-clock run of the shipped release app
with 5 real Claude sessions. The figures above are from the debug test
build, which is slower than the release app.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
