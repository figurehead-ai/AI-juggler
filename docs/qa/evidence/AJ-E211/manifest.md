# AJ-E211 evidence: leaving out the start of a conversation (D-76)

## Automated

| What | Where | Result |
|---|---|---|
| Turns, the plan (before a day, the last N), a compaction near the cut, the chain re-rooted, title lines kept | `crates/juggler-adapters/src/claude_cut.rs` tests | pass |
| A Codex rollout keeps `session_meta` and the turns from the cut | `crates/juggler-adapters/src/codex_cut.rs` tests | pass |
| Through the app's switch command: the last 3 of 10 turns land, the chain starts at the cut and is whole, the left-out turns stay behind, the source keeps every byte; the preview said so first | `crates/juggler-app/tests/switch_cut.rs` | pass |
| The window: hidden where it can't apply, closed at first, each choice's words and what's lost, only a supported choice sent | `ui/src/components/switch/carry-choice.test.tsx` | pass |
| Without a cut, the switch is unchanged (the copy-never-move check reads the carried file as before; only a cut file is hashed again) | `crates/juggler-handover/tests/pipeline.rs`, `crates/juggler-app/tests/switch_*` | pass |

## Live (req. 5): `crates/juggler-adapters/tests/cut_live.rs`

A four-turn conversation (a codename in the first turn, another in the third), cut to its last two turns, installed beside the original as a fork, resumed and asked which codenames it was told.

| Tool | Date | Result |
|---|---|---|
| Codex (codex-cli 0.163.0, the Mac's Codex home, a new throwaway folder) | 2026-11-04 | **pass**: the preview said 2 of 4 turns left out (83 KB of 96 KB, the first turn holding Codex's own instructions); the resumed fork answered only the late codename |
| Claude Code (2.1.310, the lab profile) | 2026-11-04 | **not run**: the lab profile's sign-in has expired ("OAuth session expired and could not be refreshed"); signing in is the maintainer's. Run again once it's signed in |
