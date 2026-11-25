<!-- captures: b87052a6b187 -->
# 83-keep-going: restarting crashed sessions and keeping the Mac awake

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

settings-keep-going groups the options under "Keep work going" on Settings › Sessions as checkboxes with a sentence each (restart sessions that crash, keep the Mac awake, on battery too, reopen AI Juggler, open at login); the settings card stands off the canvas in both themes and the explanations are a clear step below the labels. In dark the ticked checkboxes and the violet Desktop apps and Switching tiles in the settings list now use the lighter accent. launch-setting-restart shows the "If a session crashes: Restart it" choice inside a launch setting, with solid violet Launch buttons and Add an account (lighter in dark) and a pale, clearly disabled Save overrides. this-mac-awake is meant to show that the Mac is kept awake, but its "Kept awake while 1 AI Juggler…" line still sits at the very bottom of the AI Juggler card and is cut by the window edge, so the state it documents is half visible. The header stays one row in every capture. After the second pass the rail's "Refactor the settings screen" row keeps its time whole ("ai-ju… · viewed 2 min ago" where it read "ai-juggler · viewed 2 mi…"); nothing else changed.

## Weak

- this-mac-awake-1280x800 (both themes): the "Kept awake while 1 AI Juggler…" line is cut by the bottom of the window.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: this-mac-awake-1280x800 light (before the fix), and dark (local and the gate's, after it), settings-keep-going (the gate's, both renders), launch-setting-restart-1280x800 dark.

- **Before the fix:** this-mac-awake didn't show its line: "Kept awake while 1 AI Juggler session works" sat below the fold at 1280x800 (the 0.3.2 note said cut; in 0.5.1 it was out of the frame).
- **Fixed** (c148ecc70, 4060fca74): the capture scrolls the page's own scroller to the line; the AI Juggler card shows "AI Juggler itself: 204 MB · with its crash watchdog" and "● Kept awake while 1 AI Juggler session works Change". (The first try used scrollIntoView, which also moved the window's shell; the second scrolls only the page.) Re-rendered (gate this-mac-awake-1280x800-dark; local, both themes) and checked by eye.
- **settings-keep-going:** Settings › Sessions › Keep work going: Restart sessions that crash ✓, Keep the Mac awake while sessions work ✓ (On battery too, off by default), Reopen AI Juggler if it quits unexpectedly ✓, Open AI Juggler at login; each with its consequence.
- **launch-setting-restart:** a launch setting's "If a session crashes: Restart it", in Accounts.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Work keeps going: crashed sessions restart, the Mac stays awake while work runs, and This Mac says so. |
| 2 | Prominence | 4 | The awake line on This Mac; the settings in Sessions. |
| 3 | Default | 4 | On by default with each consequence; battery off. |
| 4 | Mode | 4 | Calm. |
| 5 | Mock-up | 4 | No mock; Settings' rows. |
| 6 | Areas | 4 | This Mac's cards; Settings' sections. |
| 7 | Colour | 4 | Live dot for awake; accent only on checks. |
| 8 | Type | 4 | Row titles 13 px, consequences 12 px ink-2. |
| 9 | Actions | 5 | No primary in Settings; Change is a link. |
| 10 | Serious | 4 | "Never after a user's stop or a failed sign-in." Battery drain said. |
| 11 | Doors | 4 | Change opens Settings at the row. |
| 12 | Words | 4 | Plain. |
| 13 | States | 4 | After the fix: the capture shows its line. |
| 14 | Consistency | 4 | Settings' checkbox rows. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 4 | Login items named where macOS lists them; no prompt. |
| 18 | Usability | 4 | Always on (D-28) said where it acts. |

Lowest: 4 (line 13 was 3: the capture's subject was out of frame).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: The renewal chip is the neutral chip with amber words (F76). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
