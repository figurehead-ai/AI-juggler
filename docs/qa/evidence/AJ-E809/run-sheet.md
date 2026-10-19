# AJ-E809 — run sheet

One row in [`runs.csv`](runs.csv) per scripted task run (see
[`programme.md`](programme.md)). Fill it during the session, check it
against the privacy list, then run `python3 scripts/dogfood_report.py`.

| Column | Values | Meaning |
|---|---|---|
| `week` | `W1` … `W4` | Programme week |
| `date` | `YYYY-MM-DD` | Day of the run |
| `participant` | `P01` … | Pseudonymous ID, never a name |
| `persona` | `maya`, `devon`, `priya` | The participant's persona |
| `task` | `quota-wall-switch`, `switch-back`, `zombie-cleanup`, `cross-provider-switch`, `fleet-join` | Which scripted task |
| `completed` | `yes`, `no` | Success condition reached |
| `unaided` | `yes`, `no` | No help given at any point |
| `minutes` | number | Trigger to success, one decimal |
| `errors` | integer | Error messages or wrong turns seen |
| `continuity` | `yes`, `no`, `na` | `switch-back` only: did the participant judge the thread intact? `na` otherwise |
| `false_kill` | `yes`, `no` | `zombie-cleanup`: did anything stop that shouldn't have? `no` otherwise |
| `friction` | text | The participant's words, redacted (`[project]`, `[path]`, `[client]`); empty if none. Quote it in double quotes when it has a comma |
| `filed_task` | `AJ-E…` or issue link | Where the friction was filed (within 48 h) |
| `capture` | path or empty | `captures/<task>-<date>-<n>.png`, consented and redacted |

Example (illustrative only, not a real run):

```csv
W1,2026-11-02,P01,maya,quota-wall-switch,yes,yes,1.4,0,na,no,"I wasn't sure the todo list would come along",AJ-E8xx,
```
