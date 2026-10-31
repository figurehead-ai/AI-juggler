<!-- captures: 3df1f73d515b -->
# 05-session-panel: a session's page with its terminal, where it runs and its runway, at four widths in both themes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

At 1024 to 1440 px the page is well ordered: title, status chip and account chip, a meta line, tabs, then the terminal beside the Where it runs and Runway cards, which stand a step above the canvas; the terminal is a true black panel in both themes with the SF Mono-style text of the DOM renderer, and after the second pass its black runs to the card's edges: the lighter (dark) or white (light) frame around it is gone, and the padding is black, so the text no longer sits flush against the edge of the black. "Switch account…" and "Relaunch" are quiet secondary buttons, so there is no competing violet in the panel. The top bar stays one row, and the lighter dark accent shows on the logo mark, the rail's New session button and the terminal cursor. The rail's selected row now reads "Stopped with an error" with no cut age; the session status still reads red in the header but amber for the same state in the rail beside it.

At 390 px the cleanup pill is now a broom and "7", but the header is still three rows and the page header still breaks: populated-390x844-* have no session title, leave a stray thin glyph before "Switch account…", and shrink the account chip to a dot with no name. The terminal now fills its card at 390 too, but its preview line still wraps mid-word ("live te / rminal)"), and the footer "Updated 1s ago · from this computer's process monitor" still wraps into two ragged columns. All captures were re-rendered; the second pass changed only the terminal's frame.

## Weak

- populated-390x844-*: no session title; a stray glyph before "Switch account…"; account chip shows only a dot.
- populated-390x844-*: terminal text wraps mid-word; footer wraps into two columns.
- populated-1280x800-*, populated-1440x900-*: the same error state is red in the header and amber in the rail.
