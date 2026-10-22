# Evidence manifest — AJ-E1543

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1543 (one switch screen). Status: **IN-PROGRESS**. 2026-10-22, claude-opus.

## Done

| Requirement | Proof | Result |
|---|---|---|
| Req.1 one screen: where it goes, what carries and what doesn't, the checklist, the freeze choice, one confirm | The usual account gets the one-click view. Otherwise there are three short steps (Target, What's carried, Confirm), and only the last has the button. The checklist (AJ-E1542) and the freeze choice sit on the confirm step (vitest `switch-flow`, `switch-setup`). | PASS |
| Req.1 in plain words | Walked by hand (below). The internal label ("F1"), "one switch hop", "the pack" and the raw tool id ("claude-code") are gone. The screen says "Full story, fresh start · Beta · Not proven yet", "Only what's listed here moves", "Redaction for Claude Code". | PASS |
| Req.2 progress in steps and a clear result, with switch back and undo | "Personal is picking up the conversation. The original has stopped, and you can switch back to it any time." Each set-up is listed under "Set up on Personal", with its own Undo ("Removed the skill release-notes"). The switch's own undo window comes from AJ-E206. | PASS |
| Req.3 a proven pair says so; otherwise Beta; never "lossless" | Badges come from the register: no pair has every cell PASS yet, so every pair reads "Beta · Not proven yet" (`ranking` `badges_carry_beta_and_never_claim_lossless`). | PASS |

## Walked by hand

In the browser preview (1280×800, dark), against the switch screen's own samples, on 2026-10-22:

- **Opening the switch:** a running session's "Switch account…" opens the ranked list. Personal opens the three-step wizard, because it isn't the usual account.
- **Confirm step:** it shows the checklist. One item is set up as it is, one needs a token and starts unticked, and one can't be set up and has no box.
- **Running it:** the result lists "Set up the skill release-notes", and its Undo works.
- **Switching back:** "Go back to" offers Personal (the previous account, preselected) and Work Max "(where it started)".

The three questions:
1. **Does it solve the problem?** It moves the conversation and offers to set up what's missing, and the user can undo either.
2. **Is it clear without explanation?** It is now. The walk found jargon, a raw tool id, results with no heading, and set-ups that said "Created X". All are fixed, in commits 2100f037 and cca9b2f5.
3. **Is it consistent?** The new parts reuse the dialog's legends, radios, rows and muted labels.

The walk also found the preview's samples claiming every switch drops the task list and subagents; they now match the fidelity tables.

## Left

- **Req.4 once on the real lab.** The switch screen still has to be walked in the real Dev app during a live switch. The lab drives the same commands without the screen.
