# AJ-E211 evidence: leaving out the start of a conversation (D-76)

## Automated

| What | Where | Result |
|---|---|---|
| Turns, the plan (before a day, the last N), a compaction near the cut, the chain re-rooted, title lines kept | `crates/juggler-adapters/src/claude_cut.rs` tests | pass |
| A Codex rollout keeps `session_meta` and the turns from the cut | `crates/juggler-adapters/src/codex_cut.rs` tests | pass |
| Through the app's switch command: the last 3 of 10 turns land, the chain starts at the cut and is whole, the left-out turns stay behind, the source keeps every byte; the preview said so first | `crates/juggler-app/tests/switch_cut.rs` | pass |
| The window: hidden where it can't apply, closed at first, each choice's words and what's lost, only a supported choice sent | `ui/src/components/switch/carry-choice.test.tsx` | pass |
| Without a cut, the switch is unchanged (the copy-never-move check reads the carried file as before; only a cut file is hashed again) | `crates/juggler-handover/tests/pipeline.rs`, `crates/juggler-app/tests/switch_*` | pass |

## Req.6's handover notes (2026-11-24, zcode-primary)

`SwitchRecord` now carries `CutRecord` — the choice in words
("turns before 2026-11-01" / "last 400 turns"), the turns and bytes
left out, the bytes carried, the cut's effective date, and whether it
landed on the tool's own compaction — and `record_edge_succeeded`
appends it to the edge's artefact manifest as a `note` row
("Start left out from <date>: N turns, <choice> — X of Y left out").
Pinned by `switch_cut.rs :: the_cut_lands_on_the_edge_as_a_note`
(the whole suite 2/2). Switch back never re-adds: the note rides the
edge, the cut rides the carried file only (the byte-identical
default test unchanged).

## The gallery captures (2026-11-24, zcode-primary)

`docs/qa/evidence/_gallery/41-carry/` = 12 captures: the switch's
advanced carry choice on the wizard's confirm step — closed (the
Disclosure, default everything), open (each choice's radio, the date
and last-N fields), and the last-N choice's words ("Carries the last
200 turns of 4,213") — 1280×800 + 390×844 × light+dark, through the
wizard door (`__AIJ_SWITCH_PREFLIGHT__`) with the cut seam set
inside `addInitScript` (a function can't cross its structured clone).

## Live (req. 5): `crates/juggler-adapters/tests/cut_live.rs`

A four-turn conversation (a codename in the first turn, another in the third), cut to its last two turns, installed beside the original as a fork, resumed and asked which codenames it was told.

| Tool | Date | Result |
|---|---|---|
| Codex (codex-cli 0.163.0, the Mac's Codex home, a new throwaway folder) | 2026-11-04 | **pass**: the preview said 2 of 4 turns left out (83 KB of 96 KB, the first turn holding Codex's own instructions); the resumed fork answered only the late codename |
| Claude Code (2.1.310, the lab profile) | 2026-11-04 | **not run**: the lab profile's sign-in has expired ("OAuth session expired and could not be refreshed"); signing in is the maintainer's. Run again once it's signed in |
