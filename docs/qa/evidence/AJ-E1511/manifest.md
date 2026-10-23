# Evidence manifest — AJ-E1511

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1511, Open a context pack in any tool (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.

Gallery: `docs/qa/evidence/_gallery/86-open-pack/` (New session with a pack
chosen: where it came from, its fidelity, what carries and doesn't, what
was withheld, its brief open; both themes).

## What exists now

1. **New session › Start from a context pack…** picks a pack's folder.
   The pack is read and checked whole before anything is shown
   (`juggler_handover::pack_read::read_pack`):
   - its `manifest.json` lists `PACK.md`, `conversation.md` and
     `state.json`; every listed file is a plain file inside the pack (no
     link, no `..`), within 16 MB (64 MB in all), and its bytes match the
     manifest's sha256;
   - `state.json` is this format (a newer one says "update to open it")
     with a fidelity AI Juggler gives (F0–F4);
   - no file holds credential material (the same always-on guard that
     writing a pack runs).
   A refused pack says why, in a line, and nothing else happens.
2. **What it holds**, as the switch screen shows a switch: the tool and
   account it came from and when, its folder and branch (and whether
   that folder is on this Mac), what it continues as (the switch screen's
   words, Beta for a pair never proven), what it carries, what it doesn't
   (each with its reason), and what redaction withheld when it was made
   ("2 API keys"). Its brief opens in full.
3. **The user picks the account and the folder** in New session as for
   any session (the pack's folder is chosen when it's on this Mac).
   Launch starts that account's session primed with the pack: the pack,
   checked again and unchanged since the preview (its manifest's hash),
   becomes the session's handover file (`switches/<edge>/HANDOVER.md`,
   this user only; AJ-E1541), quoted as data, with the folder's rules
   files named; the session starts with a short prompt pointing at it.
4. **Lineage** (Req.3): the switch graph gets the pack's work item and
   source epoch (mirrored when they came from elsewhere) and a `pack`
   edge with the pack's id, the fidelity shown, the manifest's hash and
   every manifest row, linked to the pack's own edge when that edge is in
   this Mac's graph; launched and confirmed with the new session's epoch.
   A failed write or launch marks the edge failed at that stage.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 validate before anything is written; show what it holds and what redaction withheld | `pack_v1` `a_written_pack_reads_back_whole`, `a_changed_file_is_refused`, `a_file_outside_the_pack_or_a_link_is_refused`, `a_pack_holding_a_key_is_refused_even_with_a_matching_manifest`, `a_newer_format_or_not_a_pack_is_refused`; `codex_switch` `a_pack_changed_after_the_preview_or_unsafe_starts_nothing` (a changed digest, a hand-edited file and a folder that isn't a pack: refused, nothing launched, no handover written); vitest `open-pack` (shown before anything starts; a refusal says why) | PASS |
| Req.2 pick the tool, account and project; the session starts primed, labelled with its fidelity | `codex_switch` `a_pack_opens_in_every_tool_primed_labelled_and_in_the_graph`: packs exported from a Claude Code and a Codex session each open in Claude Code and in Codex (four pairs); the brief carries the conversation (redacted when made), never on the command line; "Full story, fresh start · Beta"; vitest (starts only with a folder, with the digest previewed); Playwright `open-pack` (launched into the chosen account and folder, the session opens) | PASS |
| Req.3 lineage in the switch graph | same round trip: a `pack` edge with the pack id and `F1`, confirmed | PASS |
| Performance: validation < 2 s | `pack_v1` `a_large_pack_is_checked_in_under_two_seconds` (2,000 turns, 4.8 MB) | PASS |
| The open sheet with its withheld list, both themes, axe | Playwright `open-pack` (axe 0 critical/serious) | PASS |

## Walked

On the preview at 800×600: New session › Start from a context pack…
shows "Starting from a context pack" with From, Where, Continues as,
Carries, Doesn't carry and Withheld ("2 API keys, when it was made. They
stay out."), the brief behind a disclosure, and Don't use it. The pack's
folder is chosen below. Launch → started it on the preselected account in
that folder, closed the sheet and opened the session.

## Notes

- The colleague side of AJ-E1011 (team packs) stays demand-gated.
- Packs are folders (as AI Juggler has always written them), so the
  picker chooses a folder.
