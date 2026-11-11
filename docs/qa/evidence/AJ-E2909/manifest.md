# Evidence manifest — AJ-E2909

Task: `docs/tasks/E29-trust-and-the-finished-basics.md` AJ-E2909, A
keyboard-operable rail splitter. Built in `ui/src/components/RailSplitter.tsx`.
Checked again on 2026-11-06 (release/0.4.1); the vitest files below pass.
Status: DONE-EVIDENCED (2026-11-06, release/0.4.1): its gap
was closed the same day.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | The handle's attributes are proven: `rail-splitter.test.tsx` "is a focusable separator that says its width" asserts `tabIndex` 0, role separator, `aria-valuenow` 264, min 208, max 360, `aria-valuetext` "264 pixels wide" and `aria-controls` "rail-list". **Closed (2026-11-06):** `ui/src/components/rail-splitter.test.tsx` "is the rail's last stop, after every row of the list and its foot": in the rendered `WorkflowRail` the separator is the last tabbable element, after every focusable row in the list, with no positive tabindex. |
| R2 | PASS | `rail-splitter.test.tsx` "moves by 8, by 32 with shift, to the ends with Home and End, and keeps it" (→ 272, ⇧← 240, End 360, Home 208) and "never passes its bounds". Playwright `release-04-screens-gallery.spec.ts` "AJ-E2909: the rail's width set by keys, at its ends, kept after a reload": Home 208, End 360, ⇧← 328. |
| R3 | PASS | Stored under `STORE_WIDTH` like a drag: the vitest "moves by 8…" reads the stored width back after the 200 ms wait, and "keeps a waiting change when the window goes" holds it on `pagehide`. The Playwright test above reloads and finds 328. |
| R4 | PASS | Gallery `99-rail-splitter` (`release-04-screens-gallery.spec.ts` "The rail's edge — 1280x800-…"): the handle focused by keys, captured dark and light; `focused-1280x800-dark.png` shows the accent line along the edge (checked by eye). axe 0 critical/serious. The grip and tooltip: vitest "says the edge moves: a tooltip and a grip". |
| R5 | PASS | `rail-splitter.test.tsx` finds it by `getByRole("separator", { name: "Resize the list" })`; `rail-narrow.spec.ts` finds it by the same name. |
| R6 | PASS | Double-click still cycles the density: `rail-narrow.spec.ts` "narrow: double-clicking the rail's edge closes the list it opened; Settings is never written". Its ⌃D twin: "at full width the density shortcut still changes the setting". |

## The first part (2026-11-05)

### AJ-E2909 evidence: a keyboard-operable rail splitter

- vitest `src/components/rail-splitter.test.tsx` — pass (2026-11-05),
  including the new "keeps a waiting change when the window goes".
- Playwright "AJ-E2909: the rail's width set by keys, at its ends, kept
  after a reload": Home → 208, End → 360, ⇧← → 328, reload → 328 — pass.
- Gallery `99-rail-splitter`: focused, narrowest, widest (1280; the rail
  is folded at 1024 and has no edge); axe 0 critical/serious; critique
  stamped.
- Found and fixed: a width changed then left within 200 ms (a quit or a
  reload) was lost; it's now kept on pagehide.
- Verdict: PASS.

Gaps closed on 2026-11-06: the whole vitest suite passed (`pnpm exec vitest run --maxWorkers=4`: 281 files, 2135 tests) with the tests named above.
