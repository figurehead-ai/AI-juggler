# Evidence manifest — AJ-E1020

Task: `docs/tasks/E10-derived-workflows-and-beta.md` AJ-E1020 (the rail
folds to icons in a narrow window, D-25). Status: **DONE-EVIDENCED**.
2026-10-20, reviewer claude-opus.

## What exists now

- Below 900 px the rail shows its icon column (Minimal) by itself; at
  900 px and wider it shows the density chosen in Settings › Appearance.
  The stored density (`aij.density`) never changes.
- While narrow, ⌘B or the logo button opens the list for now; the logo's
  tooltip says "Show the list (⌘B)" / "Icons only (⌘B)". Widening forgets
  it, so the next narrow window starts folded again.
- A chosen Minimal stays minimal at every width.
- The rail carries `data-testid="rail"` and `data-density` (what it shows
  now), which the suites read.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 fold and restore | `rail-narrow.spec.ts`: 1280 → 899 folds (60 px wide), 900 restores, storage still comfortable; compact comes back as compact | PASS |
| Req.2 ⌘B while narrow | `rail-narrow.spec.ts`: opens and closes the list without writing the setting; after a wide window the narrow one starts folded | PASS |
| Req.3 Minimal kept | `rail-narrow.spec.ts` | PASS |
| Narrow captures | `capture.spec.ts` 390×844 rail and session panel expect the icon column; the switch, door, graph and beta captures at 800×600 select from it (graph opens the list with ⌘B for its project row); `zoom-motion.spec.ts` at 200 % (640 px) expects the folded rail and still has no horizontal scroll | PASS |
| Full suites | vitest 472/472; Playwright 921/921 (on the merge with main); a11y gate 940 states, 0 critical/serious; no core baseline changed by this task (all are ≥ 1024 px) | PASS |
| axe | `rail-narrow.spec.ts` gallery: folded at 390 and 860, opened at 860, both themes, 0 critical/serious | PASS |

Gallery: `docs/qa/evidence/_gallery/69-rail-narrow/`.

## Not covered here

- The icon column lists sessions only. A project row needs the list (⌘B),
  as it always did in Minimal.

## Review fixes (2026-10-20, after #63)

- **The view never writes Settings by itself.** While the rail is
  folded, the density shortcut (Ctrl+D, ⌘D in the app, a double-click on
  the rail) opens the list instead of silently cycling Settings ›
  Appearance. Open, it cycles for real and shows what it stored. With
  Minimal stored, ⌘B in a narrow window stores Comfortable (as it does
  wide), so the rail and Settings always agree.
- **Stricter shared check.** `expectRail` (the shared check of about 70
  captures) now asserts that a rail at 900 px or wider shows exactly the
  stored density, and that narrower it is folded unless Minimal is
  stored. It no longer accepts a folded rail at any width.
- **New tests.** `rail-narrow.spec.ts`: compact folds, opens as compact
  and comes back as compact; the shortcut while folded leaves storage
  alone; a stored Minimal and ⌘B agree with Settings.
- **Walked by hand** at 860×700, dark. Folded → Ctrl+D opened the list
  (storage still comfortable) → Ctrl+D again gave compact (stored
  compact) → ⌘B folded (storage compact). I chose Minimal in Settings,
  then ⌘B: rail and Settings both Comfortable. Widening to 1280 showed
  Comfortable.
