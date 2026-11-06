# Evidence manifest — AJ-E2015

Task: `docs/tasks/E20-engineering-and-security.md` AJ-E2015,
ARCHITECTURE.md. Branch `release/0.4.1` (D-83).
Status: **DONE-EVIDENCED** (agent self-check, 2026-11-06).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `ARCHITECTURE.md` at the root: a bird's-eye view, a codemap of every crate and of `ui/`, invariants stated as absences (each naming the check that holds it), boundaries, and cross-cutting concerns. |
| R2 | PASS | Every crate's `src/lib.rs` doc has a "Must not know:" line (`scripts/architecture_fresh.py` fails one without it). |
| R3 | PASS | "Revisited for: 0.4" at the top; `scripts/architecture_fresh.py` (in `just verify-static` and CI) fails once the workspace version moves to a newer minor, so the next milestone gate reads and updates it. |
