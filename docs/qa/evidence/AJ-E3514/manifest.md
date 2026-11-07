# Evidence manifest — AJ-E3514 Walked by hand: persona walks on the release build

No persona walk of the Floor on the release build exists yet. What exists
are hand walks of single 0.4.1 items in **AI Juggler Dev** (a debug build)
on the replica home, with no live lab and no persona:

- `docs/qa/floor/walk-0.4.1.md` (captures in `docs/qa/floor/walk-0.4.1/`):
  10 steps on hover, tabs, zoom, the building view's chips, one Floor
  window and adding Apps, scored works / speed / clarity / layout; two bugs
  found and fixed in 644db31d1.
- `docs/qa/evidence/AJ-E3516/walk.md` and `docs/qa/evidence/AJ-E3519/walk.md`:
  the lift, Page keys, tabs, keys 0/2/B, hovering and clicking an office,
  and every View menu item, timed.
- `docs/qa/0.4-walks/` (E33's persona walks) has no Floor scenario; its
  `design.md` scores only the Floor list, menu and settings galleries.

Drafted from the repo as it stands on `release/0.4.1`; nothing was run.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| R1 a persona walk (E33's form) on the release build, with the replica and a live lab of 1–2 real tools: open the Floor; find who waits and answer; spot a crash and an overloaded App; see a swarm of subagents; find the low Account; open a new App and watch the reflow; turn, tilt, reset; change Apps per floor; Re-create; use only the list view | — (to do: a release build, the replica plus the throwaway lab, the walk sheet under `docs/qa/floor/`) | every step walked as a persona | not done. The Dev-build walks above cover opening the Floor, tabs, the lift, zoom and the View menu only; the replica has no live sessions (AJ-E3516's walk: "nothing in the replica is active"), so waiting, crash, overload, subagents and low-Account steps were never walked; no release build, no live lab, no persona | UNPROVEN | — | 2026-11-06 | Claude (self) |
| R2 each step scored for success, value, clarity and feeling; under 4 to the maintainer; bugs fixed before release | — | E33's four scores per step; nothing under 4 | no persona scores exist. The Dev walks score works / speed / clarity / layout, not E33's four; their only score under 4 is AJ-E3516's zoom-out clarity 3, on another item, fixed in 24d92bffc | UNPROVEN | — | 2026-11-06 | Claude (self) |
| R3 the hand walk clicks every element in the Floor and its pane, and measures the response | — | every element clicked, each response timed | not done as a whole. Parts exist in AI Juggler Dev: tabs, lift, Page keys, keys 0/2/B, an office and the lobby clicked and timed (`../AJ-E3516/walk.md`, 340–631 ms with the tool's pauses); every View menu item (`../AJ-E3519/walk.md`); the Settings › The Floor pane opens in 0.5 ms p95 on preview data (`../AJ-E3511/timing.json`). Not clicked: people, helpers, gauges and batteries, the rack, meeting rooms, the whiteboard, the notice board, the props and the cat, Re-create; not on the release build | UNPROVEN | — | 2026-11-06 | Claude (self) |
| Gate: nothing under 4; no budget missed | — | — | no walk to judge; the harness records still carry over lines (open-and-close growth in Full resolution's second pass and the building view, AJ-E3516 R6) | UNPROVEN | — | 2026-11-06 | Claude (self) |

## The plan (agreed with the 0.4.1 session, 2026-11-06)

On the release candidate the main session names, after the preflight and
push of `release/0.4.1`, in a lab HOME, never the installed app:

- **Protocol:** `docs/qa/persona-validation.md` (firewall, leak lint, three
  fresh runs per flow, then a critic; its scores and bar), with the 0.4.1
  session's personas (`docs/qa/0.4-walks/personas/`).
- **The Floor's list view:** walked by the fresh testers through the
  bridge (the webview), as the other flows are.
- **The 3D window:** a native step the testers can't reach (a wgpu window,
  not the webview): Builder-driven by pid (synthetic input and window
  captures), scored on the same sheet and labelled "Builder-driven,
  native".
- **Live sessions:** a person's door (at its question or error), its
  helpers and the meeting room need a live tool in the lab; the sign-in
  is the maintainer's one step.

Until the release candidate exists, R1–R3 have nothing to walk.

## Binary artifacts

None yet. The walk's captures go under `docs/qa/floor/`, checked by eye
and redacted with the fail-closed tool before commit. Binary evidence is
content-addressed: `<claim>-<sha8>.png|log|json`; re-running a check adds
a new file, cited artifacts are never overwritten.

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here
- [x] Paths hashed or generalised where they reveal private layout (none shown)
- [x] Transcript excerpts scrubbed of personal content (none)
