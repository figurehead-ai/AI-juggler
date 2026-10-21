# Evidence manifest — AJ-E1404

Task: `docs/tasks/E14-machine-health.md` AJ-E1404 (Home: the This Mac panel
and heavy sessions under Needs you). Status: **DONE-EVIDENCED**.

## What exists now

- **This Mac** beside Runway (`ui/src/components/ThisMacPanel.tsx`). It shows
  the state as a shape and a word (● Plenty of room, ▲ Getting tight,
  ■ Under pressure), with up to two reasons when not plenty, and one memory
  bar (AI tools' share in the accent colour, the rest of what's in use in
  grey). It then gives "AI tools: X of Y memory · Z% of all CPU cores", the
  heaviest three (a session opens on click; a tool AI Juggler didn't start
  is labelled and is not a button), "Also counted: N Claude tools not
  started by AI Juggler" when none of them is in the top three, and leftovers
  with **Clean up** only when there are some. It is hidden until the first
  sample. With nothing running it reads "Nothing running — your Mac has
  room".
- **Needs you**: a Mac under pressure comes first ("See what's using it"
  scrolls to This Mac and focuses it). Heavy sessions follow the existing
  needs ("Working your Mac hard — 3.2 GB", "Take a look") and leave Running.
  The headline counts them, and becomes "Your Mac is under pressure" when
  that is the only thing. "Getting tight" stays in the panel only (calm
  until it matters).
- One subscription for the whole app (`useThisMac` in App, shared through
  `ThisMacContext`), kept current by the backend's 5 s event. No UI timer.

## Walked by hand (built-in browser, vite dev server, mock bridge)

At 1440×900 (dark), 1024×680 (light and dark) and 375×812 (dark):
- Tight + heavy + leftovers (`?aij_machine=tight&aij_load=heavy`): **Take a
  look** opens the session. **Clean up** opens Cleanup. The panel's session
  row is a button; the Claude app row is not (checked in the DOM). The rail
  row shows "3.2 GB".
- Pressure (`?aij_machine=pressure`), light: **See what's using it** scrolls
  the panel to the top and focuses it (`document.activeElement` =
  `this-mac`).
- Found and fixed on the walk: "MacOS" (sentence-casing mangled "macOS"),
  "0 MB" for a tiny tree (now "under 1 MB"), the AI-tools segment painted
  red under pressure (it blamed 1.6 GB of tools for the Mac's state; it
  keeps the accent colour now), the heavy session repeated under Running,
  heavy items ahead of "Waiting for your reply", a scroll that barely moved,
  and names truncated to "ai-…" at phone width (rows now wrap).
- At 375 px the rail does not fold yet (ux-stage4's AJ-E1019, PR #63), so
  Home's column is about 150 px wide. The panel wraps cleanly; the page has
  no horizontal scroll (scrollWidth = 375).

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1–2, 4 the panel | `pnpm --dir ui exec vitest run src/components/this-mac.test.tsx` (unsampled → nothing; nothing running; tight reasons with "macOS"; ours clickable, theirs not; Clean up only with leftovers; "Also counted") | pass | 9 passed | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Req.3 Needs you | same file: heavy once, one action, out of Running; pressure item focuses This Mac; tight stays quiet | pass | pass | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Gallery + axe | `pnpm --dir ui exec playwright test tests/visual/this-mac-gallery.spec.ts` → [`_gallery/72-this-mac/`](docs/qa/evidence/_gallery/72-this-mac/) (plenty, nothing running, tight + heavy + leftovers, pressure × 1280×800 / 390×844 × light / dark; heavy session header; Clean up → Cleanup) | 0 critical/serious | 20 passed, 0 violations | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Full suites | `pnpm --dir ui exec vitest run`; `pnpm --dir ui exec playwright test` | all pass | 483 vitest; 931 Playwright | PASS | 40293b45+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout (fixtures only)
- [x] Transcript excerpts scrubbed of personal content (none)
