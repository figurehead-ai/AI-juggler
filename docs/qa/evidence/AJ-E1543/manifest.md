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

## The list and the screen agree (2026-10-22)

The coordinator's guide fact-check found the switch list saying "Full
story, fresh start" (F1) for Claude Code→Claude Code and Codex→Codex,
while the switch screen for the same pair said "Same conversation, new
account": the matrix's same-tool rows were still at the first release's
F1. A same-tool transplant resumes the same conversation on the new
account (AJ-E1540), so their researched ceiling is now F3 ("Same
conversation, new account · Beta · Not proven yet"; F2 on the screen when
the session was compacted; a failed live cell lowers it to F2). Z Code→Z
Code stays F1 on purpose: its transplant is off, and the row never
promises a move that can't run. The ranking's fidelity input for
same-tool pairs is 0.75 (was 0.25). Tests: `validation.rs` (the badges,
the failing cell's one-level drop, `advertised_ceiling_number`),
`validation_matrix.rs`, the app's `validation_gate.rs`; the AJ-E808
register's "Switch rows show" column regenerated; the preview's matrix
and default preflight follow.

## Req.4: the screen on real data, up to Confirm (`ff7970c6`, 2026-10-23)

QA walked the switch screen in a Dev build on the real setup, through the read-only Dev bridge (#153
serves `switch_targets`, `switch_preflight` with secret previews masked, and `switch_setup_check`).
The full record is walk batch 28 in `docs/qa/evidence/walks/2026-10-21-real-readonly.md` on
`claude/qa-real-walk-clean` (`e878e5f9`, `18fa9d89`).

- **Setup:** the real setups were brought in where they are, with redacted labels. The screen opens
  from AI Juggler's own sessions, and the real setup has none, so one finished session record was
  added to AI Juggler Dev's own data for a real Claude Code conversation (`tests/qa_switch_session.rs`).
  No process ran and nothing was launched; the record was removed afterwards.
- **The walk:** Target, What's carried and Confirm, to another Claude Code account and to Codex.
  Switch stayed disabled, as no secrets choice was saved, and was never pressed. The real folders were
  unchanged.
- **What holds:**
  - the list's rows;
  - the carried, rebuilt and not-carried lists;
  - the required secrets choice, with 2 detected `sk-key` items whose previews read "•••";
  - the freeze choice ("The source is at rest", finish this turn by default);
  - the set-up list's verdicts and reasons;
  - Codex's headroom (70%, the rollouts' own reading).
- **What doesn't (left below):** the screen's label for a Claude Code → Claude Code switch, and
  duplicate rows in the set-up list.

The switch itself is proven live in the labs (AJ-E1539, labs 9–11). This walk adds the screen on real
data, up to Confirm.

## Left

- **P23 — the screen names a same-tool switch "Full story, fresh start".**
  - Real data, Claude Code → Claude Code: the list says "Same conversation, new account · Beta · Not
    proven yet".
  - `switch_preflight` returns `mechanism: transplant` with `fidelity: F1` and
    `fidelity_words: "Full story, fresh start"`. So Target and Confirm say "Full story, fresh start",
    with "Fidelity is F1: something would translate or drop".
  - The preflight's label comes from `pipeline::preflight` (the tool pair's dispositions and the
    transcript's compaction). It still lands on F1, where the list's same-tool rows were raised to F3
    on 2026-10-22.
  - Its `experimental_copy` ("the whole session moves, with its subagents and file history") is the
    transplant's, and the Codex replay shows it too, beside "Not carried: subagent state".
- **P24 — the set-up list repeats each claude.ai skill.**
  - The 9 skills in `skills/synced` sit under two folders, and each becomes its own item.
  - To Codex: 19 rows for 10 skills, "doesn't have 19 things", with duplicate offer ids.
  - To Claude Code: "already there" counts 18.
- **Wording:** "fidelity 1" and "Fidelity is F1" mean nothing to a user. "Doesn't carry" in the
  list and "Not carried (stays on the source)" on the screen say one thing two ways.
