# Evidence manifest — AJ-E1515

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1515, Fleet: start a session on another Mac, and usage per machine (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). 2026-10-23, claude-opus.
The two-Mac proof stays the maintainer's (AJ-E710).

Gallery: `docs/qa/evidence/_gallery/88-fleet-launch/` (New session on a
paired Mac; the last day by machine on Usage; both themes).

## What exists now

1. **New session › Runs on** (only with paired Macs): This Mac, then each
   paired Mac. Picking one swaps the sheet for that Mac's own choices —
   its projects (names only) and launch settings (account, tool, name),
   from the catalog it already shares — and **Start there →**. A Mac that
   can't start one says why and Start is off: "… isn't connected right
   now", "View only: on …, let this Mac control it in Fleet", "… hasn't
   shared its launch settings yet".
2. **Over the existing link** (`features/fleet`): `fleet_launch_start`
   sends the node's `LaunchRecipe` (ids only — never a path, binary or
   environment) through the command path every remote action uses; the
   node re-checks the pairing, control permission, "Ask each time",
   replay and expiry, and starts it from its own launch setting in its
   own project. Its answer names the session it started
   (`CommandResult.started_instance_id`, new and optional), so this Mac
   follows it: the session opens here as seen from this Mac
   (`app://open` with the machine). Refusals come back in that Mac's
   words; an unconnected Mac is refused here before anything is sent.
3. **Usage by machine**: each Mac adds its last 24 hours to the catalog
   it sends (new optional `CatalogDelta.last_day`: units used, the
   list-price estimate as Usage prices it, sessions — totals only).
   Usage and Fleet show "Last 24 hours, by machine": This Mac, then each
   paired Mac with "as of", "Hasn't reported yet" before its first
   report, and the total in all. Each Mac's sessions and costs in detail
   stay on its own Usage (not sent over the link); the rail already
   groups sessions by machine.
4. **The protocol**: two optional additions to the unreleased v1
   (snapshot updated; the guard refuses token-like field names, so the
   count is `used_units`). The node's handler now returns a
   `CommandDone` (message, started session).

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 New session offers the paired machines; the launch goes over the link and the node handles `LaunchRecipe` | `fleet_switch` `new_session_starts_on_the_other_mac_and_follows_it_within_five_seconds` (two nodes over loopback: the options list the desktop's launch setting and project; Start is accepted, names the session, the laptop sees it in the desktop's telemetry, and it ran on the desktop in its own copy of the project); vitest `remote-launch` (ids only, closes, that Mac's words on a refusal); Playwright `fleet-launch` | PASS |
| Offline refusal in words | `a_view_only_or_unconnected_mac_says_why_and_starts_nothing` (view only: the reason and `permission_denied`; Link off: "Studio Mac isn't connected right now", refused here, nothing ran); vitest (the reason, Start off) | PASS |
| Performance: remote launch to tracked < 5 s on a LAN | the first test times Start to the session in this Mac's view (loopback) | PASS |
| Req.2 total by machine (Usage and the fleet view) | `each_mac_reports_what_it_used_in_the_last_day` (the desktop's two sessions today, priced; one from two days ago left out; this Mac's own); vitest (the words); Playwright (Usage's card and Fleet's) | PASS |
| The machine picker; per-machine totals, both themes, axe | Playwright `fleet-launch` (axe 0 critical/serious) | PASS |
| The link contract stays | `juggler-link` `proto_contract` (snapshot, no secret-like field), transport, pairing | PASS |

## Walked

On the preview at 800×600 with two paired Macs seeded: New session shows
"Runs on This Mac · Studio Mac · MacBook Air"; Studio Mac swaps to "New
session on Studio Mac" with its projects and accounts; MacBook Air says
"MacBook Air isn't connected right now." with Start off; Start there →
closed the sheet and opened that machine's view.

## Notes

- "Filter by machine": the totals are per machine on Usage and Fleet,
  and the rail and Fleet already show each machine's sessions apart; a
  paired Mac's per-session, per-model costs aren't sent over the link
  (only its totals), so they stay on that Mac's Usage.
