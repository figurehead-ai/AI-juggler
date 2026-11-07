# AJ-E710 — Fleet gate, automated portion

- Commit: `9d96ccd06`
- Host: Darwin arm64, Apple M5 Ultra
- Ran: 2026-11-07T13:04Z in 330s (budget < 15 min)
- Overall: **green**

Loopback timings are from one machine; the gate-hardware numbers come
from the two-Mac dogfood in `docs/qa/gate-E7.md`.

### `-p juggler-link`
- result: **pass** — 63 passed, 0 failed
- [perf] heartbeat RTT p95 per round = [693.416µs]
- [perf] 5.5 MB handover over loopback: 15.115041ms

### `-p juggler-supervisor`
- result: **pass** — 127 passed, 0 failed
- [perf] targeted tick CPU [7.203371ms] (budget 30ms)
- [perf] orphan surfaced 10.062938667s after the harness exited (grace 10000 ms)

### `-p juggler-core`
- result: **pass** — 318 passed, 0 failed
- [perf] 30-day forecast: 3.397667ms (budget ≤500 ms)
- [perf] 100k-event fold: 1.927916ms (budget ≤5 ms release)

### `-p juggler-app --test fleet_identity --test fleet_link --test fleet_sync --test fleet_switch`
- result: **pass** — 44 passed, 0 failed
- [perf] cross-machine target list with preconditions: 154.917µs CPU, 156.041µs wall
- [perf] diverged refusal round trip (detect + report): 30.072209ms
- [perf] outbox write p95 over 1,000: 58.833µs CPU, 70.375µs wall
- [perf] catch-up of 1,000 changes (incl. reconnect): 91.306084ms

