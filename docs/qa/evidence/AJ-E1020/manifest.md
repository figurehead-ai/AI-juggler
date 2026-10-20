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
| Full suites | vitest 470/470; Playwright 919/919; a11y gate 940 states, 0 critical/serious; no core baseline changed by this task (all are ≥ 1024 px) | PASS |
| axe | `rail-narrow.spec.ts` gallery: folded at 390 and 860, opened at 860, both themes, 0 critical/serious | PASS |

Gallery: `docs/qa/evidence/_gallery/69-rail-narrow/`.

## Not covered here

- The icon column lists sessions only. A project row needs the list (⌘B),
  as it always did in Minimal.
