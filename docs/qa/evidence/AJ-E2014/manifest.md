# Evidence manifest — AJ-E2014

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2014, slow commands
off the main thread. Status: **DONE-EVIDENCED** (agent self-check,
2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | Every registered command is `async` and does its blocking work through `crate::ipc::blocking` (or `spawn_blocking`): `scripts/ipc_audit.py` reports 0 sync and 0 main-thread commands. The audit had a blind spot — a command whose attribute line ended in a comment wasn't seen — fixed 2026-11-06; it then found `account_import` running on the main thread (a whole setup's copy) and it was moved off. With no command left on the main thread, none can pass the 16 ms budget there; `tests/page_open_off_main.rs` keeps the page opens off it. |
