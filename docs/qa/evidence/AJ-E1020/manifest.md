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

## Review fixes (2026-10-20, after #63; revised after the #75 review)

- **In a narrow window the view never writes Settings.** ⌘B, the logo,
  the density shortcut (Ctrl+D, or ⌘D in the app) and a double-click on
  the rail only open or close the list for now, whatever is stored:
  - Open, the list shows the stored density, or Comfortable when Minimal
    is stored.
  - Closed, it shows icons.
  - Settings › Appearance changes only in Settings and in a wide window.

  The first version of this fix stored Comfortable when ⌘B was pressed
  with Minimal stored, and let Ctrl+D cycle a stored Minimal unseen. The
  #75 review caught both.
- **Stricter shared check.** `expectRail` (the shared check of about 70
  captures) asserts that at 900 px and wider the rail shows exactly the
  stored density, and that narrower it is folded unless Minimal is
  stored.
- **Regression tests in `rail-narrow.spec.ts`.** Each of these fails on
  the code before its fix:
  - the shortcut while narrow only opens and closes the list, and
    storage stays as it was
  - with Minimal stored, ⌘B opens the list (as Comfortable) and closes
    it, Settings keeps Minimal, and widening shows Minimal
  - with Minimal stored, Ctrl+D does the same

  Also new, and passing on the old code too (so not counted as
  regression proof): compact opens and widens as compact; at full width
  the shortcut still changes the setting.
- **Walked by hand** (vite preview, 860×700 then 1280×800, dark).
  Folded with Comfortable stored: Ctrl+D opened the list, Ctrl+D closed
  it, storage stayed comfortable. Then Minimal stored: ⌘B opened the
  list as Comfortable, ⌘B closed it, Ctrl+D the same, storage stayed
  minimal. Widened: Minimal.
