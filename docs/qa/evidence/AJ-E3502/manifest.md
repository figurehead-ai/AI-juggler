# Evidence manifest — AJ-E3502 Memory settled: the drawables held while animating

The memory held while frames are presented is Metal's per-process working
memory ("Owned physical footprint (unmapped) (graphics)" in the harness's
footprint breakdown), freed within a second of the last frame. D-79
(2026-11-04, `docs/review/decision-log.md`) settled the budget: while
animating, the 250 MB counts the Floor's own memory, with Metal's working
memory reported beside it per Mac; SPEC §9.7's memory row says the same.
Readings are the harness's release-build records in `docs/qa/floor/`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the ~500 MB held while presenting explained with Instruments' allocations (drawables, IOSurfaces, shadow map, targets) | an Instruments allocations / VM tracker trace of the Floor animating | a trace summary naming where the memory sits | the explanation exists from the harness's footprint categories: `budgets-2026-11-06-second.md` "Memory animating, biggest parts: TOTAL 785 MB, Owned physical footprint (unmapped) (graphics) 630 MB … IOSurface 12 MB" (Full: `-full-second.md`), and D-79 quotes a bare Metal view at about 273 MB and wgpu's bare device at 449 MB. Missing: no Instruments trace or its summary in the repo, and the per-resource split (atlas, shadow map, instances ≈ 40 MB) is only in the task's handover note | UNPROVEN | 67298c34c | 2026-11-06 | Claude (self) |
| R2 "memory added while animating" within SPEC §9.7's ≤ 250 MB | `scripts/floor_harness.py` on the release build; records `docs/qa/floor/budgets-2026-11-06-second.md`, `budgets-2026-11-06-full-second.md`; budget as ruled in D-79 and SPEC §9.7 ("≤ 250 MB animating, the Floor's own (Metal's working memory reported beside it, per Mac)") | ≤ 250 MB, the Floor's own | Pixel art: "+122 MB at its highest; Metal's working memory on this Mac 557 MB beside it (the whole app +679 MB)"; Full resolution: "+85 MB at its highest; … 598 MB beside it (the whole app +683 MB)". Within only under D-79's definition: the whole app still adds ~680 MB while animating, as before; no candidate in req. 2 reduced Metal's share | PASS | 835678880 | 2026-11-06 | Claude (self) |
| R3 until R2 is met: release the surface after 30 s without a frame, and name the gap in the 0.4.0 release notes' Known issues | `crates/juggler-app/src/features/floor/render.rs`, `crates/juggler-floor-render/src/`; `docs/releases/0.4.0.md` Known issues | the 30 s release in code and a Known-issues line, or a ruling that the condition no longer applies | no 30 s surface release in the render loop (the surface is let go only on close, `Msg::LetGo`); `docs/releases/0.4.0.md` has no memory line in Known issues. R2 is met under D-79 (settled 2026-11-04, before 0.4.0), which arguably makes this conditional moot, but no recorded decision says so | UNPROVEN | 2df52c98f | 2026-11-06 | Claude (self) |
| Gate: memory at 30, 150 and 290 s flat and within budget | harness records above | flat; the Floor's own ≤ 250 MB | Pixel art 739 → 734 → 718 MB; Full 715 → 706 → 744 MB (whole-app footprint, before opening 60–61 MB); the Floor's own at most +122 MB | within | 835678880 | 2026-11-06 | Claude (self) |
| Gate: no growth once closed (D-79) | harness open-and-close cycles, `budgets-2026-11-06-full.md` | last 3 of 6 cycles add ≤ 15 MB | "146 → 168 → 174 → 175 → 184 → 187 MB (+13 over the last 3)" | within | fd2637851 | 2026-11-06 | Claude (self) |

## Binary artifacts

None in this folder. The readings are the harness's Markdown and JSON
records in `docs/qa/floor/`. An Instruments trace (R1), when made, goes
here content-addressed: `R1-<sha8>.trace.zip` with a text summary; a re-run
adds a new file, cited artifacts are never overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
