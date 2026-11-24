<!-- captures: 4cf613200e3c -->
# 14-focus: the "Go to a session" palette, and a desktop session's panel after a focus attempt and with the permission explainer

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

palette-1024x680-light and palette-1280x800 are clear: a focused search field with a violet ring (in dark now the lighter accent), an Actions group with "New session… ⌘N", a Sessions group with tool icon, name, account · tool and "open" at the right, and a footer of key hints; the page behind is blurred and dimmed so the palette is the only thing to read. In dark the palette's edge is soft against the blurred page but still distinct. panel-outcome-1280x800 and permission-explainer-1280x800 are still pixel-for-pixel the same running Fake Desk panel with no focus outcome and no permission explainer in view, so neither proves what it is named for. That panel itself still carries two solid violet buttons ("Go to it" and "Open / Focus window"), both in the lighter accent; the top bar stays one row and the rail's error rows read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- panel-outcome-1280x800, permission-explainer-1280x800: identical captures; no outcome message or explainer is shown.
- panel-outcome-1280x800, permission-explainer-1280x800: two solid violet buttons on one panel.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 and this batch's capture fix. The palettes are ⌘K as in 99-palette (Actions, Suggested, Open now; the field's hint names what it finds).

**Fixed (c3fd24f78), the captures:** panel-outcome and permission-explainer had been the same picture since 0.3.2: both states sit below the fold on the Window preview. Each is now framed on its state: panel-outcome shows "Focused window · window “aij-e414-a.txt”", permission-explainer "App opened — window not found" and the Accessibility explainer with Open System Settings… and Don't show again. Gate baseline `14-focus/panel-outcome-1280x800` re-rendered.

What they show, scored:

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Bring the session's own window forward, and say what happened. |
| 2 | Prominence | 4 | "Go to it" first in the header. |
| 3 | Default | 4 | |
| 4 | Mode | 4 | |
| 5 | Mock-up | 4 | |
| 6 | Areas | 4 | |
| 7 | Colour | 4 | F70 (059097f12): the one-account note is a plain line with an info mark; the explainer a needs-you Alert (an amber bar and bell, no amber behind words). |
| 8 | Type | 4 | |
| 9 | Actions | 4 | Open System Settings… is a secondary; Go to it the one filled button. |
| 10 | Serious | 4 | |
| 11 | Doors | 4 | Open System Settings…. |
| 12 | Words | 4 | "One account at a time: AI Juggler can't open a second copy of this app, so one session at a time with it.", "Needs Accessibility · AI Juggler opened the app, but bringing this session's own window forward needs Accessibility permission for AI Juggler.", "AI Juggler follows its process and never draws it here." |
| 13 | States | 4 | Focused, window not found, permission missing. |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | The explainer is a named note. |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 4 | Accessibility is explained before Settings opens (PAT-OSP). |
| 18 | Usability | 4 | |

Lowest: 3 (lines 7, 9, 12), logged as F70.

**Fixed after the critique (F70), re-captured (panel-outcome, permission-explainer):** as the rows above say; the palettes unchanged. Lowest now: 4.
