# Evidence manifest — AJ-E2813 The sweep's smaller fixes

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2813. The five faults the
design sweep found (`docs/design/finding-your-way-2026-10-29.html` §2), fixed
in 0.3.0 (ca2247e04) and kept since; re-checked against the code, its tests
and the current gallery on `release/0.4.1`, 2026-11-06.

The "before" captures were never kept: the sweep named the faults in words
(the design's §2), and the 0.2 screens they were found on are gone. Each
"after" below is the committed gallery capture, opened and looked at.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 the limit dialog loses its orphan line and gains a × (Esc closes it, focus returns) | `pnpm -C ui exec vitest run src/components/limit-dialog.test.tsx`: "0.3: closes with its ×, and says switching back inside the Wait card", "Escape closes without doing anything", "a click on the dialog's text keeps focus inside it"; `LimitDialog.tsx` (the × at 175, `IconClose`); the shared `Modal` returns focus to its opener | × and Esc close, focus kept and returned | tests pass; the switch-back line sits inside the Wait card (no orphan line); the gallery family 89-limit-ready shows the × | PASS | 67fb13c0b | 2026-11-06 | Claude (self) |
| R2 New session keeps "Suggested" on one line; its text triangles become the app's chevrons | `ui/src/screens/NewSession.tsx`, `new-session/MoreOptions.tsx`, `new-session/ChoicePickers.tsx`; `grep -n "▸\|▾" ui/src/screens/new-session/ ui/src/screens/NewSession.tsx` (none); gallery `99-new-session/change-app-dark.png` | no text triangles; no wrapped label | no text triangle is left: "More options" and "Change folder" / "Change app" carry the chevron icons; "Suggested" is now the App picker's group heading, one line, in the shared Picker (the 0.3 redesign of New session) | PASS | 67fb13c0b | 2026-11-06 | Claude (self) |
| R3 the tool menu opens beside its button, never over it (placed with the shared rule) | `RowMenu.tsx` `anchoredEdge` (every menu opened from a button); `pnpm -C ui exec vitest run src/components/controls/controls.test.tsx`: "hangs below it, flips to its other side when there's no room, and keeps 8 px from the window's edge" | never over the button | passes: with room it hangs below, without it ends at the button's top (never over it), and keeps 8 px from the window's edge | PASS | 67fb13c0b | 2026-11-06 | Claude (self) |
| R4 Settings › General's squeezed card gets its full width | `GeneralPane.tsx` (Appearance, Editor and Updates cards on the settings column); `controls-gallery.spec.ts` "Settings › General: equal cards"; `_gallery/97-controls/settings-general-cards-{1024x700,1280x800}-{dark,light}.png` | every card the column's full width | each card spans the pane's column at both sizes and themes (1280×800 dark looked at) | PASS | 67fb13c0b | 2026-11-06 | Claude (self) |
| R5 Home's app buttons on one baseline and one grid | `pnpm -C ui exec vitest run src/screens/home-places.test.tsx` (the cards' rows and bottom row); `places-gallery.spec.ts` home-cards with its `LINE_UP` seams; `_gallery/94-places/home-cards-{dark,light}.png` | one bottom row, buttons aligned | every Your apps card has the same rows and one bottom row with its action (0.4.1, 96430fe0a and 97f28fbd5, the maintainer's report on the same fault) | PASS | 97f28fbd5 | 2026-11-06 | Claude (self) |
| Gate: the touched screens' tests | the vitest files above | pass | pass | PASS | 67fb13c0b | 2026-11-06 | Claude (self) |

## Binary artifacts

None added; cited: `_gallery/89-limit-ready/`, `_gallery/99-new-session/change-app-*.png`,
`_gallery/97-controls/settings-general-cards-*.png`, `_gallery/94-places/home-cards-*.png`.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (repo paths only)
- [x] Transcript excerpts scrubbed of personal content (none)
