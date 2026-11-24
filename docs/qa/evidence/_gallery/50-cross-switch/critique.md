<!-- captures: 6152003dea47 -->
# 50-cross-switch: continuing a session on another account or Mac: the target list, the switch wizard's steps, progress, conflicts and switching back

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

targets opens the switch list as a "Continue on another account · Now on Work Max" dialog over the session page, with accounts first (Codex, Lab) and then machines (Desktop Max and Personal on Studio Mac, Team on Mac mini), each with an amber "what carries" badge and a one-line readiness note; the capture opens scrolled to the other-machine list (a capture setup, not the app), so the top accounts are cut under the dialog header. The wizard dialogs (wizard-t2-ready, wizard-t2-blocked, path-mapping, carried, confirm) are well paced, with a step label, pill stepper, a large title and one decision per step, and the amber "Stays behind" box and the blocked version mismatch stand out properly. The design fix lands here: Continue and Switch now are now solid violet (the lighter accent in dark), so the next step clearly outranks Cancel and Back. The second pass carries it to the dialogs outside the wizard: Apply on Laptop in conflict-keep-both and Switch back to Laptop… in switch-back-preview are now the same solid violet primary, and in conflict-sheet, where no choice is made yet, "Choose one" is that primary faded (in dark a dim violet with grey text), so the dialog says it is waiting on a radio. progress still draws a focus outline around its "Switching to Team on Mac mini…" title, which looks like a stray box; its progress bar now takes the lighter violet in dark. conflict-sheet and resolved are clear and readable in both themes. Elsewhere the second pass changed only the dimmed session page behind the dialogs, whose terminal is now black to its card's edge. Every capture was re-rendered.

## Weak

- progress: a focus outline boxes the dialog title.
- targets: the dialog is titled "Continue on another account" but half of it is "Continue on another machine".

## 0.3.5

Re-shot with the switch dialog's shared frame. The step pills are tinted now: done steps green with a check, the current one accent (confirm: Target, Project, What's carried, then Confirm). The steps themselves are unchanged. A switch to another Mac continues there in a terminal, as before.

Its last button names where it continues, as on this Mac: "Switch to CLI on Mac mini" (it was "Switch now"); a session on another Mac starts there in a terminal.

## AJ-E710 retake (2026-11-07)

Retaken on release/0.4.1 for the Fleet gate's screenshot set (AJ-E710), both themes, every size the family has. First pass by a Sonnet agent against a written sheet; the points marked (checked) were confirmed by eye by Opus.

The dialog is titled "Continue in another app · Now in Claude Code · Work Max" (the earlier notes quote "Continue on another account"); badges "Same conversation, another app · Not proven yet"; the confirm step's stepper tinted with checks and one filled "Switch to CLI on Mac mini". The keyboard sweep reached every step and the final button by Tab (Esc closed it). 1280 only.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53 and this batch's fix, every state, both themes at 1280×800, opened as dark|light pairs. The wizard steps (wizard-t2-ready, -blocked, path-mapping, carried, confirm, progress) are the switch frame of 28-switch with "Switch to CLI on Mac mini"; they changed only behind (F53).

**Fixed (c3fd24f78), targets:** under "Continue on another machine" each paired Mac's App was a bordered row with an amber-ringed "Same conversation, another app · Beta" chip on every row (PAT-SURF-1, PAT-COLOR: amber that means nothing), unlike this Mac's target cards above it. They are now the same `TargetButton` cards: the App's logo, "Claude Code CLI · Desktop Max", its launch settings, how much carries as a quiet line, the readiness ("✓ Ready — same version, same project path", "◐ Needs you to pick the project there"), a tone step, no line. Gate baseline `50-cross-switch/targets-1280x800-dark` re-rendered.

| # | Line | Targets + wizard | Switch back + diverged | Note |
|---|---|---|---|---|
| 1 | Job | 5 | 4 | Continue on another Mac; come back; settle a divergence. |
| 2 | Prominence | 4 | 4 | Machines after this Mac's Apps, under their own heading. |
| 3 | Default | 5 | 4 | Diverged waits on a choice ("Choose one" faded), its reason in the title. |
| 4 | Mode | 5 | 3 | Switch back's preview and the diverged sheet have no icon tile or from → to strip, outlined boxes and option rows: F71. |
| 5 | Mock-up | 4 | 3 | Not on §8's frame (F71). |
| 6 | Areas | 4 | 3 | Boxes in a box (Here / On Laptop cards beside three outlined option rows). |
| 7 | Colour | 4 | 4 | After the fix, no amber chips; the diverged title's ⚠ glyph. |
| 8 | Type | 5 | 4 | The preview's summary is a monospace block. |
| 9 | Actions | 5 | 4 | "Switch back to Laptop…" acts but ends in "…". |
| 10 | Serious | 5 | 4 | Diverged says nothing was merged or run. |
| 11 | Doors | 4 | 4 | |
| 12 | Words | 4 | 3 | "[user]" / "[assistant]" lines, "switch pack", "10/13/2024, 1:43:20 AM" (F71). |
| 13 | States | 4 | 4 | Ready, blocked, mapping, progress, conflict, resolved. |
| 14 | Consistency | 4 | 3 | F71. |
| 15 | Accessibility | 4 | 4 | Keyboard sweep (a11y.spec) reaches every step; the progress heading's ring (F75). |
| 16 | Hand walk | 4 | 4 | Preview. |
| 17 | System prompts | 5 | 5 | None. |
| 18 | Usability | 4 | 4 | |

Lowest: 3 (switch back and the diverged sheet: lines 4, 5, 6, 12, 14), logged as F71. Targets fixed to 4.
