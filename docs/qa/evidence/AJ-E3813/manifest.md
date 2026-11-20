# Evidence manifest: AJ-E3813 (pre-push walk, 2026-11-20)

Model download (Qwen3-1.7B, the maintainer's one approved download).

Build: Dev release .app from release/0.5 at 75a5aaca6, lab K2 (generic fixtures: 5 Apps, 1000 ended sessions), Sonnet walk through lab_drive (guarded accessibility driver). Walk cut short: the Mac was in use by the maintainer (Aside frontmost, BetterStage covering the lower window), and the driver correctly refused to type or click. Rows say what was and was not walked. Opus verifies. Whole list: ../0.5-prepush-walk.md.

| Req | Step (exact) | Threshold | Observed | Verdict | Captures |
|---|---|---|---|---|---|
| Download, progress, hash check, install | the card is not offered | | No Download button exists: the tested-model list is unread (F-01). No bytes were fetched; no download attempted | NOT RUN (blocked) | |
| Helper sandboxed (ps under sandbox-exec, hello says sandboxed) | needs a running local helper | | not started: no downloaded model | NOT RUN | |
| Remove, file gone, disk back | nothing installed | | Models screen: "Installed: nothing downloaded yet", 0 GB; lab folder has no .gguf; free disk 136 GB before and after | NOT RUN (nothing to remove) | |
