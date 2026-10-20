# Evidence manifest — AJ-E1409

Task: `docs/tasks/E14-machine-health.md` AJ-E1409 (recommendations from real
data). Status: **DONE-EVIDENCED**.

## What exists now

`machine_health::recommendations()` works from the footprints (AJ-E1406),
counting only launch settings with ≥ 3 measured sessions:

- "This Mac comfortably runs about N sessions at once (about X each, from
  your last K)". N = (80 % of memory − what isn't AI tools now) ÷ the
  count-weighted typical peak. With no room it says so plainly.
- "“Deep work” (Work Max) is the heaviest launch setting — about X a
  session", when its median is ≥ 1.5× the next measured setting's.

This Mac shows them under "From your history", only when they exist.

After AJ-E1402's review, the room counts only our sessions' measured memory
as AI tools. A tool AI Juggler didn't start is an estimate, so it stays in
"the rest" rather than enlarge the room.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1–2 | `cargo test -p juggler-app --lib -- machine_health` (`recommendations_come_from_enough_real_sessions`: nothing from 2 sessions; "about 5 sessions" on a 32 GB Mac with 14 GB of other use; the heaviest named at ≥ 1.5×; close weights name none; observed footprints never count) | pass | pass | PASS | 42012b58+ | 2026-10-20 | claude-opus |
| Shown only with history | `this-mac.test.tsx` "shows advice from history only when there is some"; gallery `_gallery/74-overload/recommendations-*` | pass | pass | PASS | 42012b58+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
