<!-- captures: a5ecc24345ed -->
# 78-outside: sessions started outside AI Juggler, on home, Accounts and their own pages

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

home-rail shows "All clear — nothing needs you" with six app cards and a dashed "Show 2 more apps" card; in dark the cards stand off the canvas, but in light the dashed card's border nearly disappears on the grey canvas. Every app card is titled "An app", so the profile line under it does all the work of telling them apart. conversation-page reads well: the transcript card, Where it runs and Runway are separate cards, and Go to it is the solid violet primary at the top right. In its rail, the selected row's drag, pin and open buttons still sit over the title and the "Idle" state, cutting both. accounts shows the account cards lifted off the canvas, with the long "No current reading…" sentence brighter than labels. project-page is now re-rendered on the 0.3.2 shell: the Projects tab is current in the one-row header, payments-api has its breadcrumb, mark, monospace path and "New session here", and its Sessions tab is a raised card of rows (title, "Ended · main · in Claude Code", profile dot and name, age right-aligned) under "24 sessions · 1 open". The rail beside it groups by project with account-dot stacks, and every row's meta is cut ("Idle · outside AI Juggler ·…"). In dark the lighter accent shows on the logo mark, New session and the selected tab. The second pass touched only the rail's ages in home-rail, conversation-page and accounts, which after its last fix are cut beside short projects ("blog · 1 min …", "infra · 1 min a…"); nothing else on those screens changed.

## Weak

- conversation-page-1280x800 (both themes): the selected rail row's hover buttons overlap "Write tests for the par…" and its "Idle" state.
- home-rail-1280x800-light: the dashed "Show 2 more apps" card is barely visible on the grey canvas.
- home-rail-1280x800: all app cards read "An app"; only the second line tells them apart.
- project-page-1280x800 (both themes): every rail row's meta line is cut at "Idle · outside AI Juggler ·…".
