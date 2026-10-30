# Evidence manifest — AJ-E2814

Task: `docs/tasks/E28-finding-your-way.md` AJ-E2814, the design-quality review (D-57): a critique beside every new or changed screen, and every popover captured.
Status: **IN-PROGRESS** (with E28's other tasks until the 0.3 release PR). 2026-10-30, claude-opus, branch `polish-0.3`.

## What exists now

1. **A critique per gallery family**: `docs/qa/evidence/_gallery/<family>/critique.md`, seeded from the 0.3 re-render's review (AJ-E803, `docs/qa/evidence/AJ-E803/rerender-0.3.0.md`) for every family it critiqued and every E28 family: 69-rail-narrow, 80-marks, 88-subagents, 91-pools, 92-session-pages, 93-messages, 95-tasks, 96-notification-centre, 97-controls, 97-handover-told, 97-nav, 97-pool-canvas. Each answers the D-57 checklist against the reference screens (the row menus, the rail's order menu, Usage, Processes), says what is good, what is weak and what was fixed, and has one row per capture: what is on that screen and what changed in 0.3. At most one screen of text each.
2. **The check** (`scripts/critique_check.py`, in `just evidence` and CI): a family's captures are the `.png` files git tracks in it; their fingerprint (sha256 over path and blob id, from one `git ls-files -s`) is stamped into its critique (`<!-- captures: … -->`, written by `--stamp <family>`). Captures changed since the stamp fail the check until the critique is reviewed and stamped again. Content-pinned, so it holds in a shallow clone and ignores a local Playwright run that isn't staged. Families from before the review are pinned in `scripts/critique.allow` (101 lines, only shrinks): one whose screens change must get its critique.
3. **The review page** (`scripts/gallery_index.sh` → `docs/qa/evidence/_gallery/index.html`) shows each family's critique above its captures, rendered escaped (`critique_check.py --html`).
4. **Popovers**: the E28 popovers not captured are listed with their reason in `docs/qa/evidence/_gallery/not-captured.md` (a pool member's ⋯ menu; the pool's Agents and hop pickers; Tasks' Project and Pool pickers; the Subagents tab's Type and Sort pickers: the shared Picker is captured open on History's filter and Tasks' Holder). History's Picker open is captured (97-controls `picker-open-history`).
5. **The rule written down**: `docs/plan/QUALITY.md` §2's D-57 line names the check and not-captured.md; `.github/pull_request_template.md`'s checklist mirrors it.
6. **Stale evidence retired**: `91-pools/pool-access-*`, `pool-cards-*`, `pool-sheet-*` and `88-subagents/session-subagents-1280x800-*` (no spec writes them since the 0.3 canvas and Subagents tab); AJ-E1703's and AJ-E1520's manifests point at their 0.3 captures.

## Proof

| Requirement | Proof | Result |
|---|---|---|
| Req.1 a critique per family against the checklist and references | the twelve `critique.md` files above | PASS |
| Req.2 the review page shows each critique beside its captures | `bash scripts/gallery_index.sh`: 12 critiques in `index.html`, opened and read | PASS |
| Req.3 a family whose captures changed without its critique is flagged in `just verify` | `python3 scripts/critique_check.py --selftest` (stamp, unstaged rewrite ignored, staged change flagged, restamp, allowlisted family changed, stale allow lines, escaped HTML); `python3 scripts/critique_check.py` → OK | PASS |
| Req.4 every popover, menu and sheet captured or listed | E28 gallery specs read against the screens' pickers and menus; the rest listed in `not-captured.md` | PASS |
| Req.5 QUALITY.md §2 and the PR template carry the line | `docs/plan/QUALITY.md`, `.github/pull_request_template.md` | PASS |
| Performance: the check in ≤ 2 s | `time python3 scripts/critique_check.py`: 0.1 s | PASS |
| Quality gates | `./scripts/evidence_check.sh`; `python3 scripts/docs_links_check.py` | PASS |

## Not covered here

- The 97-* families' 0.3 captures aren't committed yet (they come with the release's gallery); their critiques are stamped "none" and must be reviewed and stamped again when the captures land.
- The visual check of the review page by eye in both themes: the page has one (dark) style; done in dark.
