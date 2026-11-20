# Evidence manifest: AJ-E3712 (pre-push walk, 2026-11-20)

Digest.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| burst of finishes gives one notification; digest item on Home | not walked | | needs live sessions; Mac in use | NOT WALKED | |

## Second walk (2026-11-20, Sonnet; Opus to verify)

Build: Dev **debug lab-bridge** .app of release/0.5 at 9d15cc578 (`scripts/build_dev_app.sh lab`), run from `~/Library/Caches/aij-walk-05/` (copied out of /tmp: from /tmp the Apple helper's sandbox extension is refused, so no Apple model shows and Bot/Models read as "no model"; walk-kit note W-02). Lab K2 (`lab_scenario.sh setup K2`, generic fixtures), driven by `lab_drive` by pid, never the pointer. Whole list: ../0.5-prepush-walk.md, "Second walk".

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| A burst of finishes gives one notification; the digest item on Home; digest on | needs a fleet of live agents | | BLOCKED as in AJ-E3817: no live session without a sign-in. The digest switch itself was turned on in T16 ("You'll get a notification when a digest is ready" shows) | NOT WALKED (credential-gated) | |
