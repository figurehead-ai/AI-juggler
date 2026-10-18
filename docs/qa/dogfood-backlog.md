# Dogfood backlog

Live runs the automated evidence cannot replace. They need the
maintainer's real machines and accounts, and are never run against
live user data from an agent session. AJ-E809 (beta dogfood programme)
works through this list; each row links its task's manifest.

| Task | Live run | Where it lands |
|---|---|---|
| AJ-E1001 | On one real multi-account Mac: launch each Account, then sign one Account's config dir into another account and launch it — the launch must pause with expected vs detected; record the screenshot and the `identity_assertions` rows | `docs/qa/evidence/AJ-E1001/` |
| AJ-E1002 | On a real repo: start two Accounts on it, accept the worktree for the second, edit the same file in both, then remove the worktree with and without uncommitted work | `docs/qa/evidence/AJ-E1002/` |
| AJ-E1003 | Overnight (W1): let an Account hit its 5 h limit in the evening, park the Instance, and check in the morning that exactly one offer came at the reset and Resume continued the same conversation; keep `parked/offers.jsonl` | `docs/qa/evidence/AJ-E1003/` |
| AJ-E1004 | Same overnight run as AJ-E1003, with three or four Instances: in the morning the summary must list each one in the right group (one crashed or stuck, one finished, one still running), with its history and leftovers matching the Cleanup Centre; keep a screenshot and the `instance_transitions` rows for those Instances | `docs/qa/evidence/AJ-E1004/` |
| AJ-E1005 | Timed triage: with 15 real Instances across projects, triage them keyboard-only (J/K, Enter, A) and time it against the 2-minute target; file each friction point as a task | `docs/qa/evidence/AJ-E1005/` |
| AJ-E704..E711 | The Fleet items listed under *Dogfood* in `docs/qa/gate-E7.md` | `docs/qa/gate-E7.md` |
