# Evidence manifest — AJ-E3508 The floor logo: the tool's mark and the Account's initials

The main window draws each logo in `ui/src/lib/floor-logos.ts`
(`logoParts`, `logoPrint`, `drawLogo`, `logosToDraw`, `useFloorLogos`,
mounted in `ui/src/shell/use-shell.ts`), using `drawableMark` from
`ui/src/components/ToolMark.tsx` and `appIconRead` for a desktop App's own
icon. `floor_logo_put` (`crates/juggler-app/src/features/floor/commands.rs`)
keeps it through `features/floor/logos.rs` in the registered local cache
"Floor logos" (`features/cache/mod.rs::register_defaults`). The scene
prints it in `crates/juggler-floor/src/scene/area.rs::area_floor`.

Not here yet: the task's evidence proof, captures of the sample's floors
from the default angle with the window-drawn logos, and every family with
its logo from the default angle and the building view. The golden frames
(`crates/juggler-floor-render/tests/golden/`) are built with no window
logos, so they show the Floor's own lettered fallback.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the main window draws each needed logo (the tool's mark as `ToolMark` draws it, or a desktop App's own icon; the Account's initials on a pill below; a ring of the App's colour), renders it to a 256 px PNG once, stores it through a command in the registered cache "Floor logos"; redrawn only when the mark, the Account's name or the colour slot changes | `pnpm -C ui exec vitest run src/lib/floor-logos.test.ts`; `cargo test -p juggler-app --lib features::floor::logos::tests::a_logo_is_kept_by_its_print_and_an_older_one_goes features::floor::logos::tests::only_a_real_logo_is_kept`; code: `drawLogo` (`LOGO_SIDE = 256`, rings in `colour(slot)`, the mark, a white pill with the initials), `floor_logo_put`, `logos::put`, `register_defaults` | one PNG per App, redrawn only on a new print | vitest: a desktop App's mark is its own icon; a new colour slot or new initials make a new print; an App whose kept print matches is not drawn again; prints match the backend's FNV-1a. Rust: a put is kept by its print, a new print replaces the old file (one file left) and reads back; a non-hex print, a non-PNG or a 300 px image is refused. The drawing itself (canvas) is code only. The print covers the initials, so a renamed Account with the same initials is not redrawn, which leaves the picture unchanged | PASS | bc1693084 | 2026-11-06 | Claude (self) |
| R2 the Floor prints it into the area's floor at 35 % opacity, centred and large (62 % of the short side, at most 6.5 m), blended into the floor, below desks and people, with no pick id | code: `LOGO_OPACITY = 0.35` (`scene/mod.rs`); in `area_floor`: `side = (rect.w().min(rect.d()) * 0.62).min(6.5)`, a `glass: true` quad 0.012 m above the floor; the glass pipeline blends with alpha and leaves the id target's write mask empty (`gpu.rs`) | each figure as specified | 35 %, 62 % capped at 6.5 m, blended, just above the floor (desks and people draw over it by depth) and no pick id: as specified, by code only (no test asserts them). Not centred everywhere: in a large (L) area whose first row is row 0 the logo is centred on that row (`rect.z0 + ROW_D * 0.5`), not on the area. Needs a ruling on that placement or a fix | UNPROVEN | 835678880 | 2026-11-06 | Claude (self) |
| R3 fallbacks: no known Account → the App's own initials; no drawable mark → the tool's name in letters; no logo yet → the Floor draws the letters itself | `pnpm -C ui exec vitest run src/lib/floor-logos.test.ts` ("a desktop App draws its own icon; an App with no Account its own initials", "a tool with no drawable mark writes its name in letters"); `cargo test -p juggler-floor scene::tests::an_unused_app_dims_by_how_long_and_decay_can_be_switched_off` (asserts `mark_letters`); code: `area_floor`'s `None` branch → `tex::logo_img(mark_letters(name), initials, accent)`; golden frames | each fallback | vitest: no initials → "CC" from the App's name, `ownInitials("Codex · Home") == "CH"`; an unknown tool → `letters`. The Floor's own letters: `mark_letters("Claude Code · Work Max") == "CC"`, and every golden frame (built with no window logos) shows the lettered logo in each area's floor (checked by eye on `turn-2.png`) | PASS | bc1693084 | 2026-11-06 | Claude (self) |
| R4 two Apps of one tool on one Account told apart by the ring's colour and the App's full name on the sign; the tooltip on the floor names the App | code: each App's own colour slot (least-used slot on insert, `apps_colour_slot` trigger in `crates/juggler-core/src/storage/entities.rs`) feeds the ring (`drawLogo`) and the kit's accent (`kit.rs`, `mark_colour(slot)`); the sign shows `a.app.name` in full (`area.rs`, `Scene::sign`); the area's floor pick is labelled with the App's name (`area.rs`: `self.pick(Target::App(..), app.name.clone(), ..)`) and the overlay's tooltip prints the pick's label (`overlay.rs`) | told apart by colour and name | by code: two Apps get different slots while fewer than 8 are in use, the sign carries the App's full name (e.g. "Tool A · Work Max" and "Tool A app · b-work" in the feed test), and hovering the floor names the App. No test or capture puts two such Apps side by side | PASS | 835678880 | 2026-11-06 | Claude (self) |
| Gate: a test of the fallbacks | vitest above | passes | exists; not re-run for this manifest | UNPROVEN | bc1693084 | 2026-11-06 | Claude (self) |
| Gate: the cache registered and clearable, and rebuilt on demand after a clear | `register_defaults` (`features/cache/mod.rs`); `logos::local_cache` (`Trim::Partly`, max 2000 entries) | registered; cleared; drawn again | registered: yes. After a clear `logos::prints` finds no file, the snapshot's `logo` is `None`, and `logosToDraw` draws it again at the next `floor://changed` (code). No test or walk clears it and watches it come back | UNPROVEN | bc1693084 | 2026-11-06 | Claude (self) |
| Gate: no image added to the repo | `git ls-files` for logo images | none | no `*logo*.png/svg/jpg/webp` is tracked; logos are drawn at run time | PASS | bc1693084 | 2026-11-06 | Claude (self) |
| Budget: drawing all logos ≤ 50 ms once; nothing per frame | — | ≤ 50 ms | not measured. Nothing per frame holds by code: logos are drawn only for a new print and added to the atlas when the scene is built | UNPROVEN | — | 2026-11-06 | Claude (self) |
| Evidence: captures of the sample's floors from the default angle with their logos; every family from the default angle and the building view | — | captured | not captured with window-drawn logos | UNPROVEN | — | 2026-11-06 | Claude (self) |

## Binary artifacts

None added here. Binary evidence is content-addressed:
`<claim>-<sha8>.png|log|json`. Re-running a check ADDS a new file; cited
artifacts are never overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
