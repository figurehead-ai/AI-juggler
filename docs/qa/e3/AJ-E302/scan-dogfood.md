# AJ-E302 scan dogfood — anonymised signal counts (no paths)

Run on the maintainer machine (2026-10-17, this PR's working tree).
Counts only, per the task's evidence rule — no paths, no names of
projects, no credential-adjacent anything.

| Signal | Probe | Observed |
|---|---|---|
| S1 | Config dirs | 2 candidate setups |
| S2 | Env overrides | none set in the app env |
| S3 | t3 profiles | not present |
| S4 | cc-switch | not present |
| S5 | Shell aliases | 2 assignments, 1 dangling (flagged Partial) |
| S6 | Installed CLIs | claude-code: installed |
| S7 | Login metadata | 1 of 2 signed in |
| S8 | Usage recency | 1 setup with recent activity |
| S9 | Projects | 12 project names |
| S10 | Desktop apps | Claude.app |

Wall time: ~1.2 s (budget ≤ 3 s p95). Cancellation exercised via the
wizard close mid-scan (report carried `cancelled`).
