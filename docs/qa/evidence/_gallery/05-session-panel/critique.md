<!-- captures: 6fd8c52aa363 -->
# 05-session-panel: a session's page with its terminal, where it runs and its runway, at four widths in both themes

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

At 1024 to 1440 px the page is well ordered: title, status chip and account chip, a meta line, tabs, then the terminal beside the Where it runs and Runway cards, which stand a step above the canvas; the terminal is a true black panel in both themes with the SF Mono-style text of the DOM renderer, and after the second pass its black runs to the card's edges: the lighter (dark) or white (light) frame around it is gone, and the padding is black, so the text no longer sits flush against the edge of the black. "Switch account…" and "Relaunch" are quiet secondary buttons, so there is no competing violet in the panel. The top bar stays one row, and the lighter dark accent shows on the logo mark, the rail's New session button and the terminal cursor. The rail's selected row now reads "Stopped with an error" with no cut age; the session status still reads red in the header but amber for the same state in the rail beside it.

At 390 px the cleanup pill is now a broom and "7", but the header is still three rows and the page header still breaks: populated-390x844-* have no session title, leave a stray thin glyph before "Switch account…", and shrink the account chip to a dot with no name. The terminal now fills its card at 390 too, but its preview line still wraps mid-word ("live te / rminal)"), and the footer "Updated 1s ago · from this computer's process monitor" still wraps into two ragged columns. All captures were re-rendered; the second pass changed only the terminal's frame.

## Weak

- populated-390x844-*: no session title; a stray glyph before "Switch account…"; account chip shows only a dot.
- populated-390x844-*: terminal text wraps mid-word; footer wraps into two columns.
- populated-1280x800-*, populated-1440x900-*: the same error state is red in the header and amber in the rail.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (pnpm visual:capture, capture.spec's session panel) after F53, F55, F66 and this batch's fixes; opened at full size: populated-1280x800-dark and populated-1024x680-light. Set beside the calm reference's Claude Code view (DESIGN-SYSTEM §5.12's row: panels by tone, no visible card edge, the brand colour only on the primary; the reference's `shots/` holds no Code-view capture, so the row and the round-2 mock are the reference). The page holds up: title, the state chip and the App menu, a facts line, tabs, then the black terminal beside Where it runs and Apps that can take it over, every area by tone with no rim, Watch… and Stop quiet, Switch account… the one tinted action. Against the earlier captures only the rail moved: its lens is F53's solid chosen segment and its narrow rows end their state on "…" (F55).

- **390×844 removed** (F51's rule): below the window's minimum (1024×680), so populated-390x844-* are gone and the family is captured at 1024, 1280 and 1440 (capture.spec `WINDOW_VIEWPORTS`). The 0.3.2 weak items there (no title, stray glyph, cut footer) went with them.
- **Closed since 0.3.2:** the error state is red in the header and in the rail's ✕ (AJ-E1550's one vocabulary).
- **Weak, logged F76:** the state chip "Probably stuck · 3 h" is an amber fill behind words (`SessionHead` TONES.warning), the kit's own amber chip; F59/F66 took the same fill off cards. One kit change, not this page's alone (a one-off here would make the session chip unlike every other amber chip).
- **Weak, kept:** at 1024 the folded rail leaves Apps that can take it over under "More below" (the scroll cue says so).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | See and steer one session: its terminal, where it runs, who can take it over. |
| 2 | Prominence | 4 | The terminal leads; Where it runs and the takeover beside it; rare actions in ⋯. |
| 3 | Default | 4 | Switch account… is the one tinted action; Return goes to the terminal. |
| 4 | Mode | 4 | Calm: tone panels, no rims, as the Code view. |
| 5 | Mock-up | 4 | Matches §5.12's Code-view row and SPEC §9's session page; no mock of its own. |
| 6 | Areas | 4 | Header, terminal, two side cards: by tone, more space between than inside. |
| 7 | Colour | 4 | F76 (6311ad3cd): the state chip is the neutral chip's tone with amber words and its dot ("Probably stuck · 3 h"); accent only on Switch account… and the cursor. |
| 8 | Type | 4 | Title 22 px, labels muted, values ink; the terminal's mono. |
| 9 | Actions | 4 | No filled primary on a running session; Watch… and Stop secondary, Stop not default. |
| 10 | Serious | 4 | Stop is a secondary with its own confirm sheet (77-menus). |
| 11 | Doors | 4 | Account, Model, Project in Where it runs are doors (underlined); the App menu opens its page. |
| 12 | Words | 4 | Plain: "Runs in AI Juggler's terminal", "2 apps can take it over; their room isn't measured yet". |
| 13 | States | 4 | First frame is the cached page; the terminal's preview line says it's a preview. |
| 14 | Consistency | 4 | The same header, chips and cards as 92-session-pages and 99-resume. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | AJ-E2327 (6): the rail keeps its place and the page's App and the rail's agree (Work Max on both). Match. |

Lowest: 3, each line under 4 logged (F76); the rest 4 or 5. After F76: lowest 4.

**Fixed after the critique (F76), re-captured (populated at 1024, 1280 and 1440, both themes; opened 1280 dark):** the header's "Probably stuck · 3 h" is amber words and an amber dot on the neutral chip's grey tone, no amber fill behind them (the kit's one warning chip, `WARNING_CHIP`, which every chip, pill and tag that says a warning now draws). Line 7 3 → 4; lowest now 4.
