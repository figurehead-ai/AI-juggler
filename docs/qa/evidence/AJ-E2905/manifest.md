# AJ-E2905 evidence: settings you can search, keys you can rebind

- vitest: `src/lib/shortcut-bindings.test.ts`, `src/screens/settings/settings-search.test.tsx`
  (the drift test: every Settings row findable), `src/app-keys.test.tsx`
  — pass (2026-11-05).
- Playwright, `tests/visual/release-04-screens-gallery.spec.ts`
  "AJ-E2905: a rebound key shows on the ? sheet and in ⌘K": New project
  rebound to ⌥⌘J in Settings › Keyboard; the ? sheet and the palette's
  New project row then show ⌥⌘J — pass.
- Galleries `99-settings-keyboard` (changed, capturing, refused, the go-to
  key) and `99-settings-search` (results, nothing found); both themes,
  1280 and 1024; axe 0 critical/serious; critiques stamped.
- Verdict: PASS.
