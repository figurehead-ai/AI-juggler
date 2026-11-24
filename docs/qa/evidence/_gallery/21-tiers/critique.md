<!-- captures: 38425c18dc75 -->
# 21-tiers: the Processes page when automatic cleanup is paused, suspended after a crash, or about to tidy a process (5 s and 30 s), and the account's cleanup toggles off and on

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

paused-*, suspended-*, tier2-5s-* and tier2-appearance-* each put a one-line notice with one action above the process table ("Automatic cleanup is paused." · "Resume automatic cleanup"; "…waits for you to re-confirm it (up to 24 h)." · "Re-confirm"; "…will be tidied in 5 s unless you keep it." · "Undo: keep it running"), which is clear and calm. In light the notices are a beige fill with a tan edge on the grey canvas, close to it in tone; in dark they have an amber edge and stand out. The process table has coloured state words, small CPU sparklines and right-aligned actions; the second pass re-spaced its columns at 1440, moving State, CPU and Memory right and narrowing the gap before the actions, and a later fix widened State to 11rem, so "Left behind while closed" ends with a clear gap before the python3 row's "0.2 %" in paused-*, suspended-*, tier2-5s-* and tier2-appearance-*, in both themes; in dark the logo mark and rail's New session button take the lighter accent, and the rail's error rows read "Stopped with an error" with no cut age. toggles-off-1440x900-* and toggles-on-1440x900-* are framed on their subject again: the account's Settings card shows Tidying up with "Automatically tidy left-behind dev servers" and "Also tidy after a crash (with notification)" unticked in off and ticked in violet in on, so the pair now proves the toggles. In toggles-on the paragraph under them still begins "Both off by default", which reads oddly beside two ticked boxes, and the top of both captures is a cut Sessions list over an empty card bottom. All captures were re-rendered.

## Weak

- paused-*-light, suspended-*-light, tier2-*-light: the beige notice sits close to the grey canvas.
- toggles-off-1440x900-*, toggles-on-1440x900-*: a cut Sessions list and an empty card bottom fill the top of the panel above the Settings card.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (six states at 1440, both themes); opened at full size: tier2-5s-1440x900-light (before and after the fix). The Processes page: four count tiles, the tabs, then the notice and the process table.

- **Fixed** (51ea3254f): the cleanup notices (paused, held after a crash, about to tidy in 5 s, can't record) were an amber wash with an amber edge (PAT-COLOR-2, PAT-SURF-1; in light a beige band close to the canvas, weak since 0.3.2). They are tone lines; the three that wait on the user carry the amber needs-you dot before their words, a pause the user chose carries none (vitest cleanup-status-look). Process alerts lose their outline too. Gate 23-alerts re-rendered; all six states re-captured, checked by eye in both themes.
- **Weak, kept:** toggles-* open on the account's page with a cut Sessions list above the Settings card (a capture framing); "node vite dev (PID 8123)" names a pid, fair on Processes.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | See what automatic cleanup is doing, and stop it. |
| 2 | Prominence | 4 | The notice above the table, its one action at its end. |
| 3 | Default | 4 | Undo: keep it running is the obvious action in the 5 s window. |
| 4 | Mode | 4 | Calm table, rich tiles. |
| 5 | Mock-up | 4 | AJ-E2404's notices. |
| 6 | Areas | 4 | Tiles, tabs, notice, table: after the fix no outlined band. |
| 7 | Colour | 4 | After the fix amber only as the needs-you dot and the Left behind marks. |
| 8 | Type | 4 | Tabular numbers; state words in their hue with a glyph. |
| 9 | Actions | 4 | One action per notice, a secondary; no filled primary competes. |
| 10 | Serious | 4 | Undo over confirm; the countdown is said aloud (sr-only). |
| 11 | Doors | 4 | Go to session, Clean up (n)…, Stop… on each row. |
| 12 | Words | 4 | "Automatic cleanup is paused.", "…waits for you to re-confirm it (up to 24 h)." |
| 13 | States | 4 | Each state said once; the countdown live. |
| 14 | Consistency | 4 | The notice line as Home's need lines (tone and a mark). |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: nothing is stopped without being said first, with its undo. Match. |

Lowest: 4 (every line 4 or 5).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Rail rows carry a model mark where the model changed (F77). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
