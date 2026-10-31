<!-- captures: 2ec47694c69b -->
# 62-privacy: Settings › Privacy: usage statistics choices, what is collected, the exact weekly report, the dashboard link, and an upload error

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The privacy card stands off both canvases, the selected option tile is tinted violet, and the explanations read brighter than before. The What's collected tables (monospace keys, values right-aligned) and the exact JSON report in report-json are honest and readable. The card has no heading of its own, though, so it opens straight on "Counting on this computer only. Nothing is sent." under the general Settings title. In overridden, the disabled radios are faint dashed rings that almost vanish in light. "Share weekly usage stats" stays highlighted even though the line above says it is off because DO_NOT_TRACK is set. The upload-error warning ("⚠ Couldn't reach Figurehead…") is plain body text with no colour. In not-included, the "What it reads, on your Mac only" rows use a larger type than the card text above them. report-json at 1024 opens scrolled with "Features used · 2026-W42" cut at the top. All 32 captures were re-rendered.

## Weak

- local, off, overridden, share, synced, upload-error, not-included (all): the privacy card has no heading, so the section starts mid-thought under the generic Settings title.
- overridden (all four): the disabled radios are faint dashed rings (nearly invisible in light), and "Share weekly usage stats" stays highlighted while the stats are off.
- upload-error (all four): the failure line has no amber or red treatment and reads like an ordinary note.
- report-json-1024x680-dark/light: the "Features used · 2026-W42" heading is cut under the top edge.
- not-included (all four): the "What it reads" rows use a bigger type size than the card text above them.
