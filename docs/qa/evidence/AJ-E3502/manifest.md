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
| R1 the ~500 MB held while presenting explained (drawables, IOSurfaces, shadow map, targets) | `footprint <pid>` and `vmmap --summary <pid>` on the release build animating (the system's VM accounting, the regions Instruments' VM Tracker lists; Instruments isn't installed here: Command Line Tools only); the resources' sizes from the code; `memory-breakdown-2026-11-06.md` | a summary naming where the memory sits | 775 MB in all: 621 MB owned graphics memory, 86 MB heap, 28 MB IOSurface (the three drawables, 3 × 1781 × 1320 × 4 B), 9 MB IOAccelerator. The Floor's own textures and buffers: shadow map 16.8 MB, atlas 8.4 MB, material maps 5.6 MB, targets 7 MB (28 MB in Full), instances and lights < 4 MB. The ~580 MB left is Metal's working memory for the device, as D-79's bare-device readings (449 MB) show; it goes when the Floor closes | PASS (footprint and vmmap in place of Instruments, named) | c74b9970f | 2026-11-06 | Claude (self) |
| R2 "memory added while animating" within SPEC §9.7's ≤ 250 MB | `scripts/floor_harness.py` on the release build; records `docs/qa/floor/budgets-2026-11-06-second.md`, `budgets-2026-11-06-full-second.md`; budget as ruled in D-79 and SPEC §9.7 ("≤ 250 MB animating, the Floor's own (Metal's working memory reported beside it, per Mac)") | ≤ 250 MB, the Floor's own | Pixel art: "+122 MB at its highest; Metal's working memory on this Mac 557 MB beside it (the whole app +679 MB)"; Full resolution: "+85 MB at its highest; … 598 MB beside it (the whole app +683 MB)". Within only under D-79's definition: the whole app still adds ~680 MB while animating, as before; no candidate in req. 2 reduced Metal's share | PASS | 835678880 | 2026-11-06 | Claude (self) |
| R3 until R2 is met: release the surface after 30 s without a frame, and name the gap in the 0.4.0 release notes' Known issues | the requirement's own condition; R2 above; D-79 (settled 2026-11-04: "≤ 250 MB animating, the Floor's own, Metal's working memory reported beside it") | the 30 s release and a Known-issues line while R2 is unmet | R2 is met (PASS above: +122 MB Pixel art, +85 MB Full, the Floor's own, under D-79's settled definition), so this stopgap's condition ("until it does") has lapsed: no 30 s release is owed, and no Known-issues line. Metal's working memory is reported beside it in every record, as D-79 asks | PASS (condition lapsed) | 835678880 | 2026-11-06 | Claude (self) |
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
