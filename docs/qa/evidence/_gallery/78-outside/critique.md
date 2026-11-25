<!-- captures: db4ccee89cde -->
# 78-outside: sessions started outside AI Juggler, on home, Accounts and their own pages

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

home-rail shows "All clear — nothing needs you" with six app cards and a dashed "Show 2 more apps" card; in dark the cards stand off the canvas, but in light the dashed card's border nearly disappears on the grey canvas. Every app card is titled "An app", so the profile line under it does all the work of telling them apart. conversation-page reads well: the transcript card, Where it runs and Runway are separate cards, and Go to it is the solid violet primary at the top right. In its rail, the selected row's drag, pin and open buttons still sit over the title and the "Idle" state, cutting both. accounts shows the account cards lifted off the canvas, with the long "No current reading…" sentence brighter than labels. project-page is now re-rendered on the 0.3.2 shell: the Projects tab is current in the one-row header, payments-api has its breadcrumb, mark, monospace path and "New session here", and its Sessions tab is a raised card of rows (title, "Ended · main · in Claude Code", profile dot and name, age right-aligned) under "24 sessions · 1 open". The rail beside it groups by project with account-dot stacks, and every row's meta is cut ("Idle · outside AI Juggler ·…"). In dark the lighter accent shows on the logo mark, New session and the selected tab. The second pass touched only the rail's ages in home-rail, conversation-page and accounts, which after its last fix stay whole beside short projects ("blog · 1 min ago", "infra · 1 min ago"); nothing else on those screens changed.

## Weak

- conversation-page-1280x800 (both themes): the selected rail row's hover buttons overlap "Write tests for the par…" and its "Idle" state.
- home-rail-1280x800-light: the dashed "Show 2 more apps" card is barely visible on the grey canvas.
- home-rail-1280x800: all app cards read "An app"; only the second line tells them apart.
- project-page-1280x800 (both themes): every rail row's meta line is cut at "Idle · outside AI Juggler ·…".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: home-rail-1280x800 light and conversation-page-1280x800 dark, beside the 0.3.2 ones. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **home-rail:** eight sessions started outside AI Juggler, each under its App ("claude Claude Code CLI", "claude-client-acme …") with "+27 recent"; Your apps 9 names each by its App and account ("Claude Code CLI · claude-client-acme · 1 open"). The 0.3.2 weak points are gone: no "An app" cards, no dashed "Show 2 more apps".
- **conversation-page:** "Write tests for the parser", Idle, "Claude Code CLI · claude", "Go to it ↗ ↵" the one filled button; the conversation from its transcript; Where it runs "Runs in Not known · Folder /tmp/aij-replica-docs/Dev/ac…" and "Started outside AI Juggler: shown here to read and follow. AI Juggler never stops or changes it."
- **Weak:** Apps that can take it over says "WORK MAX  Not measured" then "Not measured yet" right under it (said twice). The empty feed (F56, Home's) shows its chips.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Sessions you started elsewhere, read and followed, never touched. |
| 2 | Prominence | 4 | Each under its App; their page leads with Go to it. |
| 3 | Default | 4 | Go to it ↵ the default. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | Matches calm-ui's rail and Home. |
| 6 | Areas | 4 | Two columns; the session page's cards by tone. |
| 7 | Colour | 4 | App hues once per row. |
| 8 | Type | 4 | As Home. |
| 9 | Actions | 5 | Go to it the one filled button. |
| 10 | Serious | n/a | Nothing destructive: AI Juggler never stops an outside session (D-08). |
| 11 | Doors | 4 | Every row opens its page; Go to it to its App. |
| 12 | Words | 4 | "Not measured" twice in the take-over card (weak). |
| 13 | States | 4 | Eight real-sized; "+27 recent" folds the rest. |
| 14 | Consistency | 4 | The rail's and the session page's patterns. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | An outside session is followed without surprise; its limit is said. |

Lowest: 4.

**Re-captured after F56 (2026-11-24, Opus):** all eight re-rendered; opened home-rail-1280x800 in both themes and the conversation page. Home's empty feed is its card alone with Include my actions in it (F56: its chips are gone). Apps that can take it over still says "Not measured" twice (kept above). Lowest stays 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The system-prompt card is restyled slightly (tone, no bright border). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
