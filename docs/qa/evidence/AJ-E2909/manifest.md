# AJ-E2909 evidence: a keyboard-operable rail splitter

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
