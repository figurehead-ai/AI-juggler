# Evidence manifest: AJ-E3812 (pre-push walk, 2026-11-20)

Settings > AI > Models.

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| R1-R2 Recommended for this Mac | opened Settings > AI > Models | one pick with reason | Apple's on-device model: Apple Intelligence On, memory 96 GB needs 8 GB, no download; clear, calm, rich design | WORKS / clear / layout good | captures/models-1.jpg |
| R3 Other tested models | read the group | 3-5 cards | "Can't read the list of tested models": the list never loads (finding F-01: config.figurehead.ai/v1/<channel>.json answers 404 on every channel) | BLOCKER for the model check | captures/models-1.jpg |
| R4 Installed, per-job pick | pressed Use it | Beta on for turn reviews and Bot answers on Apple's model | Both switches on, rows say "On · Beta with Apple's on-device model", Apple row "used by Turn reviews and Bot answers"; per-job pickers read Automatic | WORKS | captures/models-2-use-apple.jpg |
| "3 not yet" fold | pressed it | lists feed, digest, watches with reasons | Opens; plain words | WORKS | captures/models-2-use-apple.jpg |
| Per-job override to a downloaded model | needs a downloaded model | | no model could be installed | NOT WALKED | |
| Low-disk messaging | skipped on purpose (do not fill the disk) | | 136 GB free on this Mac | SKIPPED | |
