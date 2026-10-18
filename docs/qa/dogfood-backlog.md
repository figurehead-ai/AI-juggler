# Dogfood backlog

Live runs the automated evidence cannot replace. They need the
maintainer's real machines and accounts, and are never run against
live user data from an agent session. AJ-E809 (beta dogfood programme)
works through this list; each row links its task's manifest.

| Task | Live run | Where it lands |
|---|---|---|
| AJ-E1001 | On one real multi-account Mac: launch each Account, then sign one Account's config dir into another account and launch it — the launch must pause with expected vs detected; record the screenshot and the `identity_assertions` rows | `docs/qa/evidence/AJ-E1001/` |
| AJ-E1002 | On a real repo: start two Accounts on it, accept the worktree for the second, edit the same file in both, then remove the worktree with and without uncommitted work | `docs/qa/evidence/AJ-E1002/` |
| AJ-E704..E711 | The Fleet items listed under *Dogfood* in `docs/qa/gate-E7.md` | `docs/qa/gate-E7.md` |
