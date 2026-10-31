<!-- captures: b6c620ad4673 -->
# 05-session-panel: a session's page with its terminal, where it runs and its runway, at four widths in both themes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

At 1024 to 1440 px the page is well ordered: title, status chip and account chip, a meta line, tabs, then the terminal beside the Where it runs and Runway cards, which now stand a step above the canvas; the terminal is a true black panel in both themes with the SF Mono-style text of the DOM renderer. "Switch account…" and "Relaunch" are quiet secondary buttons, so there is no competing violet in the panel. At 390 px the header breaks: populated-390x844-* lose the session title entirely, leave a stray thin glyph before "Switch account…", and shrink the account chip to a dot with no name. The terminal's preview line wraps mid-word ("live te / rminal)"), and the footer "Updated 1s ago · from this computer's process monitor" wraps into two ragged columns. The session status reads red ("Stopped with an error") in the header but amber for the same state in the rail beside it. All captures were re-rendered.

## Weak

- populated-390x844-*: no session title; a stray glyph before "Switch account…"; account chip shows only a dot.
- populated-390x844-*: terminal text wraps mid-word; footer wraps into two columns.
- populated-1280x800-*, populated-1440x900-*: the same error state is red in the header and amber in the rail.
