# Evidence manifest — AJ-E1405

Task: `docs/tasks/E14-machine-health.md` AJ-E1405 (the rail notes a heavy
session; the session header shows CPU · memory · helpers). Status:
**DONE-EVIDENCED**.

## What exists now

- Rail row (`RailRow`, comfortable): "· 3.2 GB" in amber, only while the
  session is heavy. One figure, memory first, because the full "CPU 96% ·
  3.2 GB" truncated in a 210 px rail. The full note is in the row's tooltip
  and its accessible name ("working your Mac hard: CPU 96% · 3.2 GB").
  Compact rows show the figure where the low-quota figure goes, when quota
  isn't already low.
- Session header (`SessionResources`), under the subline for a live
  session: "CPU 12% · 1.1 GB · 3 helpers" in quiet mono. It turns amber with
  "— working your Mac hard" while heavy. Unmeasured parts read "—".
- One formatter for all of it (`ui/src/lib/machine.ts` `bytesWords`,
  `resourceLine`), the same rounding as the backend's `bytes_words`.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 rail note only when heavy | `this-mac.test.tsx` "adds one figure only while the session is heavy"; Playwright `this-mac-gallery.spec.ts` (exactly one `rail-heavy` in the all-statuses rail) | pass | pass | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Req.2 header line | Playwright: Take a look → header reads "CPU 96% · 3.2 GB · 5 helpers … working your Mac hard"; capture `_gallery/72-this-mac/session-header-heavy-1280x800-*.png` | pass | pass | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Req.3 one formatter | `src/lib/machine.test.ts` (bytes words incl. "under 1 MB"; resource line incl. unmeasured) and the Rust `words_for_bytes` test | same words | same words | PASS | 40293b45+ | 2026-10-20 | claude-opus |
| Walked by hand | rail row and header in the built-in browser, 1440×900 dark | as described | as described | PASS | 40293b45+ | 2026-10-20 | claude-opus |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none)
