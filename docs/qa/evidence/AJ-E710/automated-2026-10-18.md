# AJ-E710 — Fleet gate, automated portion

- Commit: `1bb9e42`
- Host: Darwin arm64, Apple M5 Ultra
- Ran: 2026-10-18T20:25Z in 101s (budget < 15 min)
- Overall: **green**

Loopback timings are from one machine; the gate-hardware numbers come
from the two-Mac dogfood in `docs/qa/gate-E7.md`.

### `-p juggler-link`
- result: **pass** — 53 passed, 0 failed
- [perf] heartbeat RTT p95 = 708.834µs
- [perf] 5.5 MB handover over loopback: 51.945333ms

### `-p juggler-supervisor`
- result: **pass** — 62 passed, 0 failed

### `-p juggler-core`
- result: **pass** — 119 passed, 0 failed
- [perf] 30-day forecast: 3.503292ms (budget ≤500 ms)
- [perf] 100k-event fold: 1.5685ms (budget ≤5 ms release)

### `-p juggler-app --test fleet_identity --test fleet_link --test fleet_sync --test fleet_switch`
- result: **pass** — 29 passed, 0 failed
- [perf] cross-machine target list with preconditions: 370.416µs
- [perf] diverged refusal round trip (detect + report): 25.294958ms
- [perf] catch-up of 1,000 changes (incl. reconnect): 166.929625ms

