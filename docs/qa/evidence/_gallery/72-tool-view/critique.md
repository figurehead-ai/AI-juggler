<!-- captures: ab40c8ac4a55 -->
# 72-tool-view: a tool's own page (its Overview, and its Apps, Sessions, Projects and Pools tabs), and the tool's header menu

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.4 (AJ-E3104)

The overview captures land on the tool's own page now, not Settings: "Tool" over Claude Code's mark and name, "5 working" under it, and its tabs Overview · Apps 2 · Sessions 9 · Projects 2 · Pools 2, each counted. Overview is two cards that lift off the canvas: "2 apps · 5 running" with the installed version, then each App's runway, least left first (Personal at 0% in red above Work Max at 58%). Tool settings (its install and updates in Settings › Accounts) and Add app… sit at the header's right as quiet secondary buttons. The page no longer opens scrolled, so the stray fragments under the top bar at 1024 are gone. Weak: the runway bars stop at half the card's width, leaving its right half empty at 1280; the Runway card names Apps by their folder ("Personal", "Work Max") without their mark.

## 0.3.2

The overview captures land on Settings › Accounts rather than a page of the tool's own. The Your accounts, Tools on this computer and Renewal dates cards lift off the darker or grey canvas, and New session is solid violet. After the design fixes the dark accent is lighter on the logo mark and the rail's New session outline, and in overview-1280 the rail's feat/read-model row now reads "Stopped with an error" whole instead of losing its age to "· 7…". The Settings nav now shows one selection: Desktop apps no longer carries the hover fill it had in the earlier captures, so Accounts alone is lit. overview-1024x680 still opens scrolled, so the descenders of a cut line show as a row of stray dots above "Tools on this computer". The header-menu captures (1024 and 1280, both themes) were not re-rendered and still show the older tool page and shell, with "Add account" half hidden under the open menu.

## Weak

- overview-*: the runway bars use half the card's width; the App names there have no App mark.
- header-menu-*: not re-rendered; the open menu covers the "Add account" button.
