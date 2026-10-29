# Spec gap audit: specs and wireframes against `release/0.2` (2026-10-27)

> Read-only audit of `origin/release/0.2` at `3646146e`, made after the maintainer's note of 2026-10-27: "I think you keep skipping our specs and wireframes, I keep having to remind you of things."
>
> **Sources.**
> - `docs/wireframes/index.html` (27 screens), `docs/wireframes/README.md` (its rules: every control shown must exist, every named state must be reachable) and `wireframe-approvals.json`. All 27 screens were approved on 2026-10-13, with three change requests built into them.
> - `docs/review/deep/11-ux-redesign-screen-spec.md`, cited as "§"; wireframes are cited as "wfNN".
> - `docs/SPEC.md` §6, §7, §9, §10, §11 and §12.1.
> - `docs/review/decision-log.md` (D-01..D-44).
> - `docs/tasks/E*.md`.
>
> **Evidence** is `file:line` under `ui/src/` unless a path says otherwise; backend paths start with `crates/`. Test and `ui/src/preview/` files were not counted as evidence of UI. Nothing was built or run: this is a code reading. Items marked "check by hand" need a live walk.

## Status at 0.2.0 (added 2026-10-29)

This audit read `release/0.2` on 2026-10-27, before 0.1.9 and 0.2.0
shipped. Its tables below are kept as written. The top 15, as of 0.2.0:
"per the notes" means a release note says so and the row wasn't re-read;
"re-checked" means the code was read again on 2026-10-29 (0.3 review).

| # | Gap | Status at 0.2.0 |
|---|---|---|
| 1 | Session History tab never appears | Fixed in 0.1.9, per the notes |
| 2 | History search snippets not redacted | Fixed in 0.1.9 (redacted before cutting), per the notes |
| 3 | Internal ids and codes on screen | Mostly fixed in 0.1.9 (one error-to-sentence mapping, D-25 sweep), per the notes; not re-checked row by row |
| 4 | No notification centre, menu-bar item or badge | Fixed in 0.2.0 (D-49, E25), per the notes |
| 5 | Backend failures are silent | Not re-checked |
| 6 | Switch dialog unsafe at the edges | Mostly fixed in 0.1.9 (Cancel and × on every step, failures with Retry), per the notes; Esc mid-switch not re-checked |
| 7 | Quota wall not one click | Re-checked: mostly fixed (Needs you opens it; New session passes over an out-of-room account and says why; Queue for the reset). The second confirm is the maintainer's choice (0.1.8). Still open, in 0.3: a limit hit in a Claude Code session that stays open, and Switch back when the account has room again |
| 8 | Stop sheet shows no proof, promises no record | Re-checked: the checks run but aren't shown. 0.3 adds one plain proof line (details on demand), the unsaved-work warning for single processes, "Saved in Cleanup history", plain words, and fixes a missing per-process re-check in "whole session tree" |
| 9 | Fleet permissions | Re-checked: the receiving Mac enforces every level, and Control acting without asking is ADR-0013's design. 0.3 adds No access, a confirm on remote Stop and the right pointer to Processes › Your other Macs; the rest waits for the two-Mac test (AJ-E710) |
| 10 | ⌘K is a go-to list | Partly fixed in 0.2.0 (actions first, then every screen and New session), per the notes; the dialog contract not re-checked |
| 11 | Go-to hotkey may not bring the window forward | Fixed in 0.1.9, per the notes |
| 12 | Cost can say "$0"; wf20's cuts missing | "$0" fixed in 0.1.9, per the notes; the cuts are still missing |
| 13 | Density does nothing in the rail | Fixed in 0.1.9 (and ⌘B keeps it), per the notes |
| 14 | First run hides its guesses; dropped steps | The Figurehead step is back (D-46, 0.1.9); Found / Likely / Guessed not re-checked |
| 15 | No context-pack view | Re-checked: a viewer for saved handovers exists (New session › Start from a saved handover). 0.3 adds "Open the handover" and a "hidden / left out" line after a cross-tool switch, and Save handover on the session page; a full pack screen was judged low value |

"Needs a ruling": ranking by headroom is settled by D-45; the Figurehead
step by D-46.

## How to read the tables

- **built**: present and reachable from `App.tsx`. Different words count as built, and the note says so.
- **partial**: some of it exists.
- **missing**: absent.
- **bug**: present but wrong.
- **deviates (X)**: built differently because a later decision or task says so (X names it). These are intentional and are not counted as gaps. They are also listed together in [Intentional deviations](#intentional-deviations).

Decisions win over the wireframes (README rule 3). The ones that reshape the most:

- **D-25**: Home/Accounts/History/Settings; Triage and Overview folded into Home; "session" on screen; first run in three steps; no internal codes on screen; unbuilt actions hidden.
- **D-36**: the desktop-first rail by place; View ⇧V; no default go-to hotkey, and ⌥Space refused.
- **D-35**: profiles are not accounts.
- **AJ-E1533**: one page model. Each item type gets its own tabs instead of SPEC §9.2's six.
- **AJ-E2110**: the Settings panes.
- **AJ-E2404**: Cleanup lives inside Processes.

## Summary

### Counts

Rows across every table below: **425**, counted by script over each table's Status column.

| Status | Rows |
|---|---|
| built | 167 |
| partial | 123 |
| missing | 96 |
| bug | 8 |
| deviates (intentional) | 31 |

So 227 specified items, about half of the 394 that no decision changed, are missing, partial or wrong.

### Top 15 gaps, ranked by user impact

1. **A session's History tab can never appear.** `hasHistory` is read at `pages/registry.ts:108-109` but nothing sets it, and `RightInspector.tsx:306-332` has no `"history"` case. AJ-E1533 and AJ-E2108 Req.4 still require the tab. So a session's switch history, its Switch Graph link and Replay can't be reached from its own page. (wf09, AJ-E1533)
2. **History search shows raw transcript text.** The snippets are not redacted: `crates/juggler-app/src/transcript_text.rs:101-137` and `crates/juggler-app/src/history.rs:441` never call the redaction engine. Credential-like strings in a transcript can appear on the History screen. This is a privacy and security finding. (§4.12 copy, §13.1 risk)
3. **Internal ids and codes reach the screen.** Examples:
   - quota alerts read "Work ≈40 min left (Measured), five_hour window" and "… five_hour window exhausted" (`crates/juggler-app/src/commands.rs:486-520,662-668`, shown at `components/NotificationRegion.tsx:232-233`);
   - every backend error shows its raw `Display` string, such as "account … not found" or "no adapter for harness …" (`crates/juggler-app/src/error.rs:19-45,127`, through `errorText` in 91 files);
   - provider ids like `claude-code` appear in Home's overnight summary and in Fleet;
   - detector ids ("sk-key") appear in the switch flow;
   - a raw session id appears in the switch-back done line;
   - the tooltip "Needs sign-in first (Ready home)" names an internal screen.

   (D-25 amendment AJ-E1025, §10)
4. **There is no notification centre, no menu-bar item and no badge.** That means no bell, no Needs action / Earlier, no Snooze or Mute, no Mark all read and no ⌘⇧N. There is no tray, no Dock badge and no window-title count, so "one badge everywhere" doesn't exist. (§4.14, §4.17, §1.1, §6 flow 1)
5. **Backend failures are silent.** `refresh()` swallows errors (`App.tsx:379-381`). There is no "Backend unavailable — showing last known state. [Retry]" banner, and nothing is disabled with a reason while the backend is down. (§3, §8, §10)
6. **The switch dialog is unsafe at the edges.**
   - Esc closes it in every phase, including mid-switch, losing the result (`components/SwitchFlow.tsx:220-226,276-279`).
   - A failed switch has no Retry.
   - The lossy wizard's primary button is "Switch ▸", never "Continue — I understand the drop list".
   - There is no "Change in Settings" link to the secrets policy.

   (§4.9, wf12)
7. **The quota-wall moment is neither one click nor easy to find.**
   - LimitDialog opens only from a Home "limit" row and then needs a second confirm.
   - It shows no "estimated time".
   - "Queue return at reset" has a backend (`lib/bridge.ts:1516`) but no UI caller.
   - The launch sheet still starts a quota-blocked account, with no disabled Start, no reason and no switch hint.

   (wf10 state, wf11, §4.9, §6 flow 2)
8. **The kill sheet shows no proof and promises no record.**
   - The four named checks of wf22 (spawned by, same process, parent still running, unsent work) are not shown.
   - Nothing says the stop is recorded in the audit log (`components/KillSheet.tsx:376-396`).
   - Leftover rows carry no "spawned by … · verified start time + command line" evidence.

   (wf21, wf22)
9. **Fleet permissions don't match §11 and wf24.** "No access" can't be set. "View + control" acts without asking (`screens/FleetScreen.tsx:593-605`). Remote cleanup uses its own simpler sheet instead of the verify sheet (`components/RemoteCleanup.tsx:167`). The remote inspector points to a "Cleanup → All machines" view that doesn't exist (`components/RemoteTabs.tsx:491`).
10. **The ⌘K palette is a go-to list, not a command palette.** There are no global commands when nothing is selected, no Accounts or apps group, no recents, no fuzzy match and no "See all results". It also breaks the dialog contract: no `role=dialog`, no focus trap, and focus doesn't return to the invoker (`components/GoToPalette.tsx:215`). (§4.17, §2.5, wf27)
11. **The global go-to hotkey probably doesn't bring AI Juggler forward.** The handler only emits `app://go-to` (`crates/juggler-app/src/lib.rs:724-729`), and nothing calls `show()` or `set_focus()`. Pressed from another app, the palette may open in a background window. Check by hand. (D-36, §4.17)
12. **Cost can say "$0", and wf20's useful cuts are missing.**
    - The Estimated cost tile shows "$0" when no prices are known (`components/usage/Dashboard.tsx:129`), against "never $0.00".
    - There is no cost per finished task, no wasted spend (crashed or stuck), no weekly cost review, no top project per account and no pricing version with [Update].

    (wf20, §4.11)
13. **Density does nothing in the default rail.** The place lens ignores density (`components/PlaceBoard.tsx:117`; rows are a fixed 44 px), so Settings › Appearance › Density only works under Group by Project. ⌘B also turns Compact into Comfortable (`App.tsx:161`). (AJ-E403, D-25)
14. **First run hides its guesses, and its dropped steps were never recorded.**
    - Scan results no longer say Found, Likely or Guessed, or where each guess came from, though the backend still sends this (`types/generated/DiscoveredInfo.ts:44`).
    - Rescan exists only after a failed scan.
    - The optional "Connect a Figurehead account" step (AJ-E1203 Req.3) and the renewal-date step are gone. No decision records dropping them, while SPEC §12.1 and E12 still promise them.

    (wf16, wf17, D-18)
15. **There is no context-pack view.**
    - No pack contents or manifest screen (wf14).
    - No "View full pack…" in switch-back (wf13).
    - No "View pack" or "pack staged ✓" on a parked card (wf27); the backend stages packs.
    - "Export context pack" lives only deep in a project's Switch Graph, not on the session.

Next after these:

- Quality covers only part of D-07: no retry rate, stuck episodes, context at finish or cost per completed task, and nothing per account.
- Several notification controls are missing: waiting threshold, quota 80/95 %, digest time, Send test, per-project mute.
- Settings saves that fail don't revert (`components/NotificationSettings.tsx:37,78`).
- Shortcuts can't be rebound and Settings can't be searched.
- Deleting launch settings is one click with no confirm (`screens/accounts/ProfileRow.tsx:208`).
- An ended session's own page has no Resume, Archive, Replay or Copy ID.

### Needs a ruling (not counted as gaps)

- **Ranking accounts by headroom in the launch sheet and switch targets.**
  - What the code does: it ranks and labels accounts "Suggested", "Recommended" or "Ranked for you" by remaining usage (`components/RankedRows.tsx:42`, `screens/NewSession.tsx:132-142`, `components/LimitDialog.tsx:42-46`, `crates/juggler-core/src/ranking.rs:16` `W_HEADROOM = 0.45`). Accounts › "Best for new work: X, N% left" does the same (`components/AccountRunway.tsx:118-150`).
  - Why that looks right: §4.8, §4.9 and wf19 ask for exactly this.
  - Why it might not be: D-31 says "Nothing ranks or chooses accounts by remaining usage, for agents or tasks". As written, that is scoped to agents and tasks, so this audit counts the user-facing ranking as built.
  - The question: does D-31 also cover the user's own launch and switch sheets?
- **The Figurehead first-run step.** Was it dropped on purpose with D-25's three-step first run? If so, record it and update SPEC §12.1 item 5 and AJ-E1203's status. If not, it is missing.
- **Fleet pairing codes last 2 minutes.** wf23 says 10. ADR-0013 (`docs/adr/0013-fleet-at-launch.md:179`) chose 2 on purpose, so this is counted as a deviation.

### Docs out of step with the code

| Doc | What's stale |
|---|---|
| `docs/SPEC.md` §12.1 items 1, 2 and 5 | Still promise first-run confidence labels, the redaction choice, renewal dates and the Figurehead step. |
| `docs/tasks/E3-smart-onboarding.md:54,74,457` | AJ-E303/E304/E309 are DONE-EVIDENCED, but their screens were removed with the old wizard (AJ-E1022, commit `91b640d6`). No "superseded" note. |
| `docs/tasks/E12-optional-account.md:69` | AJ-E1203 is DONE-EVIDENCED, but its Req.3 onboarding step no longer exists. |
| `docs/tasks/E6-quota-cost-and-billing-research.md:3` | The epic header says TODO; every task in it is DONE-EVIDENCED. |
| SPEC/EPICS tray reference | Cites AJ-E1411 for the tray. No task file defines AJ-E1411. |
| `docs/SPEC.md` §9.2, D-06 | Still say "six tabs for every selection". AJ-E1533 replaced that; the SPEC section has no supersession note. |

---

## 1. Shell: header, rail, Home, keys, palette (wf01, wf02, wf27; §1.3, §2.2, §2.5, §3, §4.1, §4.2, §4.6, §4.17, §9, §11.1)

### 1.1 Header and top bar

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Header nav: Overview · Accounts · Usage · Cost · ⚙ | §3 | deviates (D-25) | `lib/nav.ts:63-69`; `components/MainNav.tsx:35-46` | Home, Accounts, Usage, History, Settings. This Mac, Fleet, Processes and Privacy are under More (`lib/nav.ts:70-76`). |
| Title + the context's tool chip | §3 | built | `App.tsx:694-720` | |
| This Mac line in the top bar | AJ-E2105 | built | `App.tsx:723` | |
| Fleet once a Mac is paired | §11.3, D-25 | built | `components/MainNav.tsx:44-48` | |
| "Fleet: 2 machines" count | wf01 | partial | `components/MainNav.tsx:44` | A Fleet link, no count. |
| Cleanup pill when something is left | D-25 | built | `components/MainNav.tsx:49-58` | |
| ⌘K search affordance in the header | wf01, §3 | partial | `components/RailToolbar.tsx:65` ("Search ⌘K") | In the rail toolbar, not the header. |
| `+ New ▾` (Session / Account / Import machine…) | §3 | missing | `App.tsx:741-744` | A plain "New session" button. |
| Notification bell + badge | §4.14 | missing | `App.tsx:694-746` | See section 2. |
| Edition badge (Nightly/Dev) | D-23 | built | `App.tsx:657-666` | |
| Figurehead account in the shell | D-18, AJ-E1203 | built | `screens/settings/AdvancedPane.tsx:36-43`; Cloud chip `screens/FleetScreen.tsx:365-375` | The docs specify one home in Settings, Cloud chips, and no pop-ups, banners or reminders. No doc specifies a header account or avatar menu, and none is built. |

### 1.2 Rail

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| ⌘B collapses to an icon strip; width remembered | README global, wf01 | partial | `App.tsx:159-162`; `components/WorkflowRail.tsx:69-81,202` | Width is persisted. **Bug:** ⌘B toggles minimal↔comfortable, so a Compact rail comes back as Comfortable. |
| Icon strip shows state dots + type icons | wf01, D-36 | partial | `components/WorkflowRail.tsx:941-1028` | Only AI Juggler's own active sessions; sessions open in other apps (D-36) and place icons are missing. |
| Auto-fold when narrow | §3 (1100 px) | built | `App.tsx:145-158`; `lib/window.ts:7` | Folds at 900 px. |
| Focus filter All/Projects/Instances/Sessions, ⌘\ | wf01 | deviates (D-36) | `lib/rail-view.ts:30-34`; `components/ViewPopover.tsx:46` | View ⇧V: Group by Place/Project/Pool/Account, Show, Only. |
| Tabs Needs me / Running / Parked / Finished | wf01 | deviates (D-36) | `components/PlaceBoard.tsx:42-45,74-108` | Needs you / Open now / "Also:" line. |
| Rail search: ⌘F focuses, Esc clears | wf01, AJ-E404 | partial | `components/RailToolbar.tsx:52-70`; `lib/app-keys.ts:52-56` | Esc clears. ⌘K focuses it; ⌘F does not. |
| ⌘1..⌘3 section jumps | wf01, §3 | deviates (D-36, AJ-E2104) | `lib/shortcuts.ts:111` | ⌘1–9 bring a place's app forward; screens moved to ⌘0, ⌘⇧A, ⌘⇧U, ⌘⇧F, ⌘⇧C. |
| Arrow keys in the rail | wf01 | built | `components/PlaceBoard.tsx:244-262` | Place lens. |
| Legend behind "?" | D-25, AJ-E403 | built | `App.tsx:681`; Home heading | |
| Row grammar: icon + profile badge, title, place line, state dot + word | D-36, SPEC §9.3 amended | built | `components/PlaceRows.tsx:147-226` | |
| Per-row provider/model/quota bar/child count | wf01, SPEC §9.3 | deviates (D-25, D-36) | `components/PlaceRows.tsx:182-188` | Only as exceptions: low runway, heavy load. |
| Truncated title has a tooltip and a full aria-label | §7 | partial | `components/PlaceRows.tsx:131-133` | Accessible name complete; no hover `title`. |
| Loading skeleton | wf01 | built | `components/WorkflowRail.tsx:461` | |
| Empty state | wf01 | built | `components/PlaceBoard.tsx:343-347` | |
| Stale chip | wf01, SPEC §9.3 | partial | `components/WorkflowRail.tsx:866-868` | Project lens only; the default place lens has none. |
| Error banner with data kept | wf01, §3 | missing | `App.tsx:379-381` | |
| ↗ badge on popped-out terminals | README change log, wf05 | missing | `lib/ipc/terminal.ts:49` used only at `components/TerminalView.tsx:278` | |
| Splitter is keyboard-operable | §2.5 | missing | `components/WorkflowRail.tsx:284` | `role=separator` with no tabIndex or keys. |
| Projects: filter, sort, pinned (⌘⇧P), no-results | §4.2 | partial | `components/WorkflowRail.tsx:490-504,641-720` | Project lens, pins and Clear filters exist. No sort, no ⌘⇧P. |
| ⌘⇧I folds the inspector to an edge tab with label + badge | README global | partial | `App.tsx:516-522`; `pages/PageShell.tsx:107-114,225-228` | Folding hides the tabs and keeps the header line. No edge tab, no badge. |

### 1.3 Home (wf02, §4.1, §11.1, D-25)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Needs you | D-25, §4.1 | built | `screens/Home.tsx:470-485`; `components/HomeNeeds.tsx:139` | |
| Runway | D-25 | built | `components/HomeRunway.tsx:61` | |
| Running | D-25 | partial | `components/HomeApps.tsx:28` | AI Juggler's own sessions are left out of "Your apps"; only a count shows. |
| Pick up where you left off + Mark reviewed | D-25 amendment | built | `components/HomeRecent.tsx:69,90` | |
| Parked until a reset + Resume | D-25 amendment | built | `screens/Home.tsx:486-531` | |
| Overnight summary (finished, crashed, cost, leftovers) | wf02, AJ-E1004 | partial | `components/MorningSummary.tsx:12-18,36,41,92` | Cost hard-coded "cost: Unavailable". Shows the raw provider id. No Cleanup link. |
| "Mark all finished as reviewed" | wf02 | missing | `components/HomeRecent.tsx` | Per row only. |
| "Open Cleanup Centre" | wf02 | built | `components/MainNav.tsx:49-58`; `lib/home-needs.ts:104-105` | Pill + Needs you row. |
| Sort control | §4.1 | deviates (D-25, AJ-E1021) | `docs/review/decision-log.md:480` | "Sorting is the rail's." |
| Chips All / Needs reply / May be stuck / Quota / Review / Sign-in | §4.1 | missing | `screens/Home.tsx`, `components/HomeNeeds.tsx` | No decision drops them. |
| Per-row Snooze ▾ (1 h / Until reset / Mute) on Home | §4.1 | partial | `lib/action-specs.ts:50-81` | In the rail row menu only. |
| Keys J/K/↑↓/Enter/A | D-25, AJ-E1005 | built | `screens/Home.tsx:236-274`; `lib/shortcuts.ts:144-159` | |
| Keys r/h/s/e | §4.1 | missing | `lib/shortcuts.ts` | |
| Empty "All clear — {n} sessions running…" | §4.1, §10 | built | `lib/home-needs.ts:86` | Reads "All clear — nothing needs you"; the count is on its own line. |
| Loading skeleton | §4.1 | built | `screens/Home.tsx:365-370` | |
| Error banner + stale timers | §4.1, §8 | missing | `screens/Home.tsx:441-448` | Only the ledger-degraded banner. |
| Overview folded: machine card, update card, recent lineage | §11.1, D-25 | partial | `App.tsx:723`; `components/NotificationRegion.tsx:79-108` | No recent-lineage card. |
| Sessions list (§4.6): search, chips, table, bulk, Resume/Relaunch | §4.6 | partial | `screens/History.tsx:270-306`; `pages/SessionsList.tsx` | History has Ended/Archived tabs, search and filters. No sort, no bulk archive/export; Resume/Relaunch are not in the row menu. |

### 1.4 Keyboard map (§3, §13, wf keys)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Registered keys | — | built | `lib/shortcuts.ts:82-170`; `lib/app-keys.ts` | ? ⌘K ⌘N ⌘B ⌘⇧I ⌃D ⌘⇧L ⌘Z Esc ⇧F10 ↩ ⌘↩ ⌘1–9 ⇧V ⌘0 ⌘⇧A ⌘⇧U ⌘⇧F ⌘, ⌘⇧C ⌘. ⌘⇧H ⌥⌘1–6 ⌘[ ⌃Tab; terminal ⌘F ⌘+ ⌘− ⌘0 ⌘↩; Home J/K/↑↓/Enter/A. |
| ⌘N new session | §3, wf01, wf10 | deviates (AJ-E305) | `lib/shortcuts.ts:86` | ⌘N opens New project; the "New session" buttons have no key. |
| ⌘⇧S switch | wf05, wf09 | deviates (§13) | `lib/shortcuts.ts:123` | ⌘⇧H, per the later spec. |
| ⌘. focuses the terminal | §1.3, §4.7 | deviates (deep-02 SP-04) | `lib/shortcuts.ts:122` | ⌘. is Stop; no key moves focus into the terminal. |
| `g` then i/p/s/a go-to | §3 | missing | `lib/shortcuts.ts` | |
| ⌘⇧N, ⌘⌫, ⌘⇧P, ⇧⧉ pop-out key | §4.14, §4.7, §4.2, wf05 | missing | `lib/shortcuts.ts` | |
| `?` shortcut sheet | AJ-E1504 | built | `components/ShortcutSheet.tsx`; `App.tsx:1158` | |
| Settings › Shortcuts: rebind + conflict check + reset | §4.15 | missing | `screens/settings/panes.ts:37-49` | Only the go-to hotkey rebinds (`screens/settings/DesktopAppsPane.tsx:139-171`). |

### 1.5 Palette, menu-bar item, hotkey, native menu

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| ⌘K palette: input, arrows/Enter/⌘Enter, section labels | §4.17, wf27 | built | `components/GoToPalette.tsx:200-209,227,349` | |
| Groups Actions / Sessions / Projects / Accounts (+ apps, AJ-E2106) | §4.17 | partial | `components/GoToPalette.tsx:349` | No Accounts or apps group. |
| Recents, fuzzy match, → preview, "See all results →" | §4.17 | missing | `components/GoToPalette.tsx` | Substring match only. |
| Global commands with nothing selected (New session, Settings…, Cleanup) | wf27, §4.17 | missing | `components/PaletteActions.tsx`; `App.tsx:1055` | Actions come only from the selected item. |
| No-results "Search transcripts for…" | §4.17 | partial | `components/GoToPalette.tsx:249` | "No live sessions — start one from the rail." |
| Dialog contract (role=dialog, trap, focus returns) | §2.5, §9 | missing | `components/GoToPalette.tsx:215` | Esc works; Tab escapes; focus isn't restored. |
| Menu-bar (tray) item | §4.17, SPEC (AJ-E1411) | missing | `crates/juggler-app/Cargo.toml:60` (`tauri` features `[]`); no `TrayIconBuilder` | |
| Global go-to hotkey: no default, user picks, ⌥Space refused | D-36, AJ-E2110 | built | `crates/juggler-app/src/lib.rs:720-731`; `lib/hotkey.ts:56-67`; `screens/settings/DesktopAppsPane.tsx:135-202` | |
| Hotkey brings the window forward | §4.17 | bug | `crates/juggler-app/src/lib.rs:724-729` emits only; `lib/outside.ts:23` → `App.tsx:554` opens the palette | No `show()` or `set_focus()`. Check by hand. |
| Global hotkey default ⌥Space | §4.17 | deviates (D-36) | `lib/hotkey.ts:1-5` | |
| Native app menu with Settings… and app actions | AJ-E2106 | partial | `crates/juggler-app/src/quit.rs:295-314` | Default menu with Quit replaced. `lib/native-menu.ts` is unreachable outside a test. |
| Open at login | D-26 | built | `components/SessionsSettings.tsx:45-83` | |

### 1.6 Themes, density, motion, global state, focus

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Light / Dark / System + ⌘⇧L | D-16 | built | `components/AppearanceCard.tsx:18-51`; `theme.ts:23-44` | |
| First run follows prefers-color-scheme | §2.2 | deviates (D-16) | `theme.ts:10` | Dark by default, per D-16. |
| Density in Settings › Appearance | D-25, AJ-E403 | built | `components/DensityChoice.tsx`; `components/AppearanceCard.tsx:52` | |
| Density applies to the default rail | AJ-E403 | bug | `components/PlaceBoard.tsx:117` | The place lens ignores it. |
| Reduce motion: OS setting + in-app override | §2.2, §9, §4.15 | partial | `styles.css:289-297` | OS only. |
| Global state loading/ready/degraded/error | §3 | missing | `App.tsx:379-381` | |
| Offline: launch/handover disabled with a reason | §3, §8 | missing | — | |
| Stale chips | §3 | partial | `components/indicator.tsx:305`; `components/WorkflowRail.tsx:866-868` | Not in the default lens. |
| Skip link "Skip to panel" | §9 | missing | grep `Skip to`: none | |
| One `#app-live` polite + one `#app-alert` assertive | §9 | partial | `components/indicator.tsx:444,478`; `components/Toast.tsx:271`; `components/NotificationRegion.tsx:220` | Several polite regions, no assertive one. |
| Shared dialog contract | §2.5 | partial | `components/form.tsx:60-150` (`Modal`) | About 20 files use Modal; about 30 hand-roll `aria-modal` (e.g. LimitDialog, SwitchFlow, FirstRun, the palette). |
| No `window.alert/confirm/prompt` | §2.3, §7 | built | grep: none in non-test UI | |
| Tabs ARIA (arrows + tabpanel) | §2.5 | partial | `pages/PageShell.tsx:233-261` | History's tablist has no arrow keys. |

## 2. Notifications: centre and system policy (§4.14, wf27, §4.15)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Header bell; badge = unsnoozed count, the same everywhere | §4.14, §1.1 | missing | `App.tsx:694-746`; no `set_badge*`/`document.title` | |
| "Needs action" section | §4.14 | missing | — | Home's Needs you is the only list. |
| "Earlier" (resets, finishes, crashes, updates) | §4.14 | partial | `components/NotificationRegion.tsx:219-` | A band of current alerts; no history. |
| Snooze / Mute project / Mute type per item | §4.14 | missing | grep snooze/mute | Parked offers have "Remind me in 30 min" / "Stop reminding me". |
| Mark all read | §4.14 | missing | — | |
| Digest preview and time | §4.14 | partial | `components/NotificationRegion.tsx:232`; `crates/juggler-core/src/alerts.rs:219-286` | Hourly "Digest · …" lines; no preview, no time setting. |
| ⌘⇧N | §4.14 | missing | `lib/shortcuts.ts` | |
| Empty "You're all caught up." | §4.14, §8 | missing | — | |
| Muted state line | §4.14 | partial | `components/NotificationSettings.tsx` quiet-hours fieldset | Quiet hours exist; no muted state in the shell. |
| macOS denied: guidance, centre keeps working | §4.14 | partial | `components/NotificationSettings.tsx:57-60` | A fixed hint; denial isn't detected. |
| System kinds off by default; once per state; never while in front; quiet hours; click opens owner | §4.14, AJ-E1510 | built | `lib/ipc/notifications.ts:15-22,52-69`; `crates/juggler-app/src/features/notifications/mod.rs:61-96,243-273` | Plain copy ("Waiting for your reply"). |
| System alert: waiting longer than a threshold | §4.14 | partial | `crates/juggler-app/src/features/notifications/engine.rs:31` | Fires on any wait. |
| System alert: quota below a threshold | §4.14 | missing | `crates/juggler-app/src/features/notifications/engine.rs:33` | Only at the limit. |
| System alert: crash | §4.14 | missing | `crates/juggler-app/src/features/notifications/engine.rs:29-36` | No failed kind. |
| System alert: queued-return reset | §4.14, wf27 | missing | `crates/juggler-app/src/features/notifications/engine.rs:29-36` | |
| Settings: Send test | §4.15 | missing | `components/NotificationSettings.tsx` | |
| Settings: per-project mutes | §4.15 | partial | `components/NotificationSettings.tsx` digest-accounts | Per account only. |
| Settings: waiting threshold 5 min; quota 80 % / 95 % | §4.15 | missing | `types/generated/NotificationPrefsInfo.ts` | Only quiet hours and muted accounts are stored. |

## 3. Design-system primitives (§2.3)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Banner | §2.3 | missing | — | Hand-written `role="alert"` in 72 files. |
| Toast (6 s) | §2.3 | built | `components/Toast.tsx:24,43,144`; `App.tsx:1157` | 5 s default. |
| Skeleton | §2.3, §7 | partial | `components/Placeholder.tsx:58`; local one in `pages/tasks/PoolTasks.tsx` | "Loading…" text remains (e.g. `screens/FleetScreen.tsx:380`, `screens/NewSession.tsx:450-452`). |
| EmptyState | §2.3, §8 | missing | — | |
| StatusChip (9 statuses) | §2.3, §2.4 | deviates (D-36) | `components/indicator.tsx:45,319` | Five words, with "Probably" for inferred states. |
| QuotaMeter | §2.3 | partial | `components/primitives.tsx:185`; `components/charts/Gauge.tsx` | Several drawings; `components/QuotaRings.tsx` is dead code. |
| ConfirmDialog | §2.3 | partial | `components/form.tsx:60` (`Modal`) | Contract met; no named primitive. |
| WizardShell | §2.3 | missing | — | |
| DataTable | §2.3 | missing | — | |
| Kbd | §2.3 | missing | — | |

## 4. Copy

### 4.1 Internal ids and codes on screen (D-25 amendment AJ-E1025)

| Leak | Where it's made | Where it shows | Status |
|---|---|---|---|
| `five_hour`/`weekly` and "(Measured)"/"(Computed)"/"(estimate)" in quota alerts; "hand over or queue at reset" | `crates/juggler-app/src/commands.rs:486-520,662-668` | `components/NotificationRegion.tsx:232-233` | bug |
| Raw error text: "{entity} {id} not found", "no adapter for harness {id}", "invalid {subject}: {message}", "{context}: {message}", "internal state unavailable", "… (no audit, no kill) …" | `crates/juggler-app/src/error.rs:19-45,127`; e.g. `crates/juggler-app/src/commands.rs:3062` | `errorText` (`lib/bridge.ts:2685`) in 91 files | missing |
| Provider id (`claude-code`) | `crates/juggler-app/src/overnight.rs:168`; `crates/juggler-app/src/remote.rs:650` | `components/MorningSummary.tsx:36`; `screens/FleetScreen.tsx:550`; `components/RemoteTabs.tsx:422`; `components/RemoteBoard.tsx:90`; `components/RemoteSwitchTargets.tsx:71` | missing |
| Detector ids "sk-key", "bearer", "env-value" | `crates/juggler-handover/src/redaction.rs:153,171,247` | `components/SwitchFlow.tsx:167` | missing |
| Raw session id in the switch-back done line | — | `components/SwitchBackFlow.tsx:313` | missing |
| Internal screen name "(Ready home)" in a tooltip | — | `components/RightInspector.tsx:739`; `screens/WorkPanels.tsx:166` | missing |
| "Claude session:" hard-coded for every tool | — | `components/RightInspector.tsx:482` | missing |
| "Ladder: {platform} — {grace}s of grace … KILL only what ignored SIGTERM" | — | `components/KillSheet.tsx:470` | partial (correct, not plain words) |
| Lower-case kinds "terminal"/"desktop" | `crates/juggler-app/src/commands.rs:13276` | `components/SwitchGraph.tsx:63`; `screens/WorkPanels.tsx:87` | partial |
| Stale pointer "Cleanup → All machines" | — | `components/RemoteTabs.tsx:491` | missing |
| "Re-confirm in Settings › Advanced" (the control is on Processes) | — | `screens/processes/RulesPanel.tsx:93` | missing |

Checked and clean: Usage, runway and session usage map `five_hour` to words. Cleanup history maps outcomes to words (`screens/processes/HistoryView.tsx:89`). System notification text is plain.

### 4.2 §10 microcopy bank, verbatim check

| String (§10) | Status | Evidence | Sorted as |
|---|---|---|---|
| Inbox empty "All clear — {n} sessions running, nothing waiting on you." | built | `lib/home-needs.ts:86` "All clear — nothing needs you" | Reworded on purpose (D-25 calm); the count is on its own line. |
| Waiting row "Waiting {t} — “{prompt}”" | built | `screens/Home.tsx:99` "Waiting for your reply" | Reworded (D-36 "Asks you" grammar). The excerpt now comes from last messages (AJ-E2108). |
| Stuck row "May be stuck? No output for {t}." | built | `components/indicator.tsx:58` "Probably stuck"; `screens/Home.tsx:101` | Reworded on purpose (D-36: inferred states say "probably"). |
| Quota row "Hit its 5-hour limit. Resets {time}." | built | `screens/Home.tsx:103` "{account} hit its limit" | Reworded; the reset time shows on the parked row. |
| Admission Tight "Tight — stopping “{session}” ({peak}) makes this comfortable." | partial | `screens/NewSession.tsx:504` | Reworded, and the actionable part (which session to stop) is missing. |
| Admission No "… [Hand over instead] [Start anyway]" | partial | `screens/NewSession.tsx:510` | Reworded; "Hand over instead" is missing. |
| Handover success "Running on {target} now. {source} version is paused…" | built | `components/SwitchFlow.tsx:660-667` | Reworded. |
| Handover failure "Nothing was changed. [Retry] [Cancel]" | partial | `components/SwitchFlow.tsx:680-701` | Reworded; Retry is missing. |
| "Resume — same session, same account." / "Relaunch — fresh session…" | missing | grep: none; `pages/ItemHeader.tsx:267` | The tooltips are missing, and Resume itself is missing for ended terminal sessions. |
| Signed-out launch "Needs sign-in first — Claude's sign-in, about 30 seconds." | partial | `screens/NewSession.tsx:372-383`; `pages/ProjectPage.tsx:240` | Reworded; the time hint is missing; one tooltip leaks "(Ready home)". |
| Quota unknown "Quota not available yet." (never 0 %) | built | `screens/Home.tsx:408`; `screens/accounts/AccountCard.tsx:49-56` | Reworded on purpose ("not measured yet", the wording of D-39). |
| Cost estimate "~${amount} estimated (API-equivalent)." | partial | `components/CostMatrix.tsx:114` | Reworded; the dashboard tile has no "~" and can read "$0". |
| Stop confirm "Stop “{label}”? Its transcript is kept…" | deviates (AJ-E1503 Req.2) | `lib/stop.ts:1-28` | A 5 s Undo instead of a confirm. |
| "{n} live sessions use this profile — stop them first." | partial | `lib/bridge.ts:515` (account remove) | Reworded for accounts; deleting launch settings has no guard (`screens/accounts/ProfileRow.tsx:208`). |
| Offline banner "Backend unavailable — showing last known state. [Retry]" | missing | grep: none | |
| Update available "{harness} {new} is available … never updates it for you" | built | `components/ToolUpdateLine.tsx`; app: `components/NotificationRegion.tsx:299` | Reworded. |
| §4.14 "You're all caught up." / "Muted until …" | missing | grep: none | No notification centre. |

## 5. Session page, inspector and terminal (wf05–wf09; SPEC §9.2; §4.7)

Tabs per selection kind (`pages/registry.ts:105-123`; renderer `components/RightInspector.tsx:303-358`). The deciding source is AJ-E1533 (`docs/tasks/E15-spec-gaps.md:362-379`): each type gets its own tabs, there are no empty tabs, Actions sit in the header and ⋯ menu, and Quality is merged into Usage.

| Kind | Overview | Usage | History | Quality | Processes | Actions | Other |
|---|---|---|---|---|---|---|---|
| Session (terminal) | terminal body, labelled "Overview" | ✓ | **declared, never reachable** | inside Usage | ✓ | header + ⋯ | Messages, Tasks when present |
| Session (desktop app) | ✓ | ✓ | **never reachable** | inside Usage | ✓ | header + ⋯ | Window preview |
| Session from another app | ✓ | when it has usage | — | — | while live | ⋯ | — |
| Project | ✓ | when it has usage | Sessions body | removed (AJ-E1533) | removed (AJ-E1533) | header + ⋯ | Sessions |
| Account | ✓ | — | — | — | — | ⋯ | Sessions |
| Tool | ✓ | — | — | — | — | ⋯ | — |
| Instance | merged into Session (D-25) | | | | | | |
| Paired Mac | ✓ | — | — | — | ✓ | tab | — |
| Session on a paired Mac | ✓ | — | — | ✓ | ✓ | tab | — |

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Six tabs for every selection | SPEC §9.2, D-06, wf06 | deviates (AJ-E1533) | `pages/registry.ts:105-123` | |
| Session History tab (lineage, Switch Graph link, Replay) | wf09, AJ-E1533, AJ-E2108 Req.4 | bug | `pages/registry.ts:68,108-109`; `pages/session/session-tabs.tsx:24-28`; `components/RightInspector.tsx:306-332` | Never set, never rendered. |
| Actions tab | wf09, SPEC §9.2 | deviates (AJ-E1533) | `pages/ItemHeader.tsx:222-275`; `components/InspectorHeaderMenu.tsx` | |
| Overview: status, identity, running model, machine | wf06 | partial | `components/RightInspector.tsx:416-456`; `pages/session/WhereItRuns.tsx:28-36` | No machine row for local sessions. |
| Overview: quota snapshot | wf06 | built | `pages/session/Runway.tsx`; `pages/session/OwnTerminalOverview.tsx:61` | |
| Overview: attention block (stuck, leftovers, last error) | wf06 | partial | `components/RightInspector.tsx:1100-1181`; Processes tab | Spread over tabs; no "last error" anywhere. |
| "Unavailable — reason" fields | SPEC §9.2, D-06 | partial | `components/RemoteTabs.tsx:119-145`; `components/RightInspector.tsx:449-451,504` | Plain-words reasons are fine (D-25); `cost: —` has no reason. |
| Loading / error / stale states | SPEC §9.2, §8 | partial | `components/RightInspector.tsx:258-266,1058-1064`; `App.tsx:842` | The footer time falls back to `now`, so it never looks stale. |
| Usage: two clocks, burn, data source | wf07 | built | `components/AccountRunway.tsx:163-169`; `pages/account/Settings.tsx:28-38` | |
| Quality signals (D-07) | wf07, D-07 | partial | `components/RightInspector.tsx:1033-1220` | Missing: retry rate, 7-day stuck episodes, context at finish, tokens/cost per completed task, the "score: Unavailable — by design" row, and quality per account. |
| No composite score | D-07 | built | `components/RightInspector.tsx:69` | |
| Processes tab: tree, sparklines, CPU/RAM, ports, owner | wf08, D-41 | built | `components/RightInspector.tsx:1227-1244` → `screens/processes/cells.tsx:171,183,188` | |
| Sub-agents | wf08 | built | `components/SubagentsSection.tsx` via SessionNotes | |
| Open Cleanup Centre | wf08 | deviates (AJ-E2404) | `screens/processes/ProcessesView.tsx:1-8` | |
| Export process snapshot | wf08 | missing | grep in `screens/processes` | |
| Header: provider/account/model/state/context/branch | wf05 | deviates (AJ-E2108 Req.1, D-25) | `pages/session/SessionHead.tsx:181-249` | Model, context and branch are one step deeper. |
| Park until reset | wf05, wf09 | built | `components/ParkCard.tsx:22` | Shown when it applies (D-25). |
| Duplicate session | wf09 | missing | grep: none | |
| Export context pack from a session | wf09, §4.7 | partial | `components/SwitchGraph.tsx:186-191,403-408` | Only per epoch in a project's graph. |
| Archive / Replay / Delete from an ended session's page | wf09, §4.7 | partial | `lib/action-specs.ts:122-143`; `components/InspectorHeaderMenu.tsx:73` | Only from History's list. |
| Copy ID | §4.7 | partial | `components/RightInspector.tsx:482-486` | Plain text, no copy action. |
| Stop confirm | §4.7 | deviates (AJ-E1503 Req.2) | `lib/stop.ts:20-28` | |
| Header Stop while waiting or stuck | §4.7 | partial | `App.tsx:953` | Only for running/starting; ⋯ has it. |
| Resume vs Relaunch on an ended session | §4.7, §10 | partial | `pages/ItemHeader.tsx:180-183,255-270`; `pages/session/DrivenPanel.tsx:105` | No in-place Resume for ended terminal sessions. |
| Lineage line ("↗ from …") | §4.7 | missing | grep in `pages/session` | |
| Branch/worktree warning, ▸ Details | §4.7 | built | `components/RightInspector.tsx:459-462` | |
| Terminal Maximize | wf05 | built | `components/TerminalView.tsx:280-304,332`; `components/TerminalToolbar.tsx:82` | ⌘↩ / Esc instead of ⌥⤢. |
| Pop-out to its own OS window | wf05 | built | `components/TerminalToolbar.tsx:85-90`; `components/TerminalView.tsx:312-330`; `screens/TerminalWindow.tsx` | Same PTY. |
| ⌘F find / Copy / clickable links | wf05, §4.7 | built | `components/TerminalView.tsx:148,306-310,347` | |
| Reconnect keeps scrollback | wf05, §4.7 | built | `components/TerminalView.tsx:186-197`; `components/TerminalToolbar.tsx:11-15` | |
| Detached / adopted after restart | §4.7 | built | `components/RightInspector.tsx:1231-1240` | |
| Desktop placeholder, no fake terminal | §4.7 | built | `components/RightInspector.tsx:545-711` | |
| Crashed: exit code + leftovers link | wf05, §4.7 | partial | `components/RightInspector.tsx:700-704` | Terminal sessions just say "Ended". |
| Offscreen summary on a state change | §4.7, §9 | partial | `components/NotificationRegion.tsx:220` | No announcement on the session page. |

## 6. Projects, Instances, History, Search, Replay, Switch Graph (wf03, wf04, wf15, wf25; §4.3, §4.12, §4.13)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Add project: picker + drag-drop | wf03 | built | `screens/NewProjectSheet.tsx:203-269` | |
| Worktree table | wf03 | partial | `pages/ProjectPage.tsx:262-276` | Only with more than one checkout; no dirty/state/quota per row. |
| Shared-checkout warning + one-click own worktree | wf03, AJ-E1002 | partial | `pages/project/Callouts.tsx:109-110`; `App.tsx:1033` | The warning has no button; the offer comes at launch. |
| Open in Finder / editor | wf03 | built | `pages/ItemHeader.tsx:234-249` | |
| Project history | wf03, §4.3 | built | `pages/ProjectPage.tsx` (ProjectSessions); `pages/project/Overview.tsx:158` | |
| Non-git project stated | wf03 | built | `pages/project/Callouts.tsx:84` | "Not a git checkout". |
| Launch with… ranked, ★ usual | §4.3 | built | `pages/ProjectPage.tsx:205-260` | No OK/Tight/No admission. See "Needs a ruling". |
| Forget project, confirmed | §4.3 | built | `components/ManageWork.tsx:166,209-257` | |
| Instances screen | wf04 | deviates (D-25) | — | Sessions by place (D-36). |
| Identity verified/unverified per session | wf04, AJ-E1001 | built | `components/RightInspector.tsx:425-431` | |
| Start/Stop + resources per session | wf04 | built | `App.tsx:953`; `pages/ItemHeader.tsx:208-210,255` | |
| Switch Graph: epochs, edges, fork | wf15 | built | `components/SwitchGraph.tsx:23-123` | Only on project Overview after a move; fidelity in the edge label. |
| Open epoch | wf15 | built | `components/SwitchGraph.tsx:246-261,384-399` | |
| Replay any epoch | wf15 | missing | `components/SwitchGraph.tsx` | |
| Export graph | wf15 | missing | grep: none | |
| Transcript search, local index | wf25, §4.12 | built | `screens/History.tsx:162-201,331-356` | History is the search screen. |
| Search scopes All/Sessions/Projects/Accounts/Transcripts | §4.12 | deviates (D-25) | `screens/History.tsx` | No full-screen search; History + palette. |
| Search filters status/account/project/date | §4.12 | partial | `screens/History.tsx:288-317` | Project, Profile and App filters only. |
| Visible results count, time, "N indexed" | wf25, §4.12 | partial | `screens/History.tsx:322-331`; `components/OutsideSearchSetting.tsx:26-27` | Count is screen-reader only. |
| Preview pane | §4.12 | missing | — | |
| "Indexing…" state | §4.12, §8 | missing | `screens/History.tsx` | |
| Redacted snippets | §4.12 | missing | `crates/juggler-app/src/transcript_text.rs:101-137`; `crates/juggler-app/src/history.rs:441` | Security. |
| Archive / unarchive with Undo | wf25 | built | `screens/History.tsx:230-262` | |
| Read-only Replay | wf25, AJ-E1007 | built | `components/ReplayDialog.tsx`; `screens/History.tsx:257,369` | |
| Fork from any point in a replay | wf25 | missing | `components/ReplayDialog.tsx` | |
| Delete separate and confirmed | wf25 | built | `screens/History.tsx:370-380` | |
| Retention per account + dry run | wf25 | built | `components/StorageRetentionCard.tsx:23-29,96-160` | |
| Review queue | §4.13 (P2) | deviates (D-25, AJ-E1021) | `components/HomeRecent.tsx:90` | |

## 7. Launch sheet (wf10, §4.8)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Recent project cards | wf10, §4.8 | partial | `screens/NewSession.tsx:179-187,271-296` | No branch, no "last active". |
| Choose folder… | wf10, §4.8 | built | `screens/NewSession.tsx:297-313` | |
| Drag-drop onto the sheet | wf10, §4.8 | bug | `screens/NewSession.tsx:250-261` | Highlights but does nothing; only a window drop works (`App.tsx:79`). |
| Provider → Account cascade with quota hint | wf10 | partial | `screens/NewSession.tsx:329-444` | One flat list with runway words. |
| Launch settings + Edit… | wf10, D-10, D-35 | partial | `screens/NewSession.tsx:339,414`; `components/ExpertSetup.tsx:277-285` | Under the profile (D-35); no dropdown in the sheet. |
| Model picker | wf10 | partial | `screens/NewSession.tsx:340-342` | Shown, not pickable. |
| Own worktree default + branch preview + same-branch warning | wf10, §4.8, §13.1 | partial | `components/IsolationOfferDialog.tsx:85-150`; `App.tsx:1033` | Offered only when sharing (follows §13.1); no toggle; branch not always shown. |
| Start from: Fresh | wf10 | built | `screens/NewSession.tsx:207` | Implicit. |
| Start from: Resume recent | wf10 | missing | `screens/NewSession.tsx` | |
| Start from: context pack… | wf10, AJ-E1511 | built | `components/OpenPack.tsx:66-117` | |
| Identity pre-flight line | wf10, AJ-E1001 | partial | `components/IdentityMismatchDialog.tsx`; `App.tsx:1032` | The guard works; the sheet never says it will check. |
| Start ⏎ / Cancel Esc | wf10 | built | `screens/NewSession.tsx:190-200,229,354-362,477-479` | "Launch →" / "Close". |
| Start names its destination | §4.8 copy | missing | `screens/NewSession.tsx:476` | |
| ≤ 3 clicks from ⌘N | wf10 | partial | `lib/shortcuts.ts:86` | ⌘N opens New project. |
| Quota-blocked → Start disabled with reason + switch hint | wf10, §8 | missing | `screens/NewSession.tsx:357,508-511` | Starts anyway. |
| 3 steps Where / Who / Review, aria-current=step | §4.8, §9 | partial | `screens/NewSession.tsx:267-269,326` | One sheet, two headings. |
| Ranked rows: meter, ~ time left | §4.8 | built | `components/RankedRows.tsx:81` | See "Needs a ruling". |
| Suggest banner ("Personal usually runs this project") | §4.8 | partial | `components/RankedRows.tsx:42` ("Suggested") | Suggests by headroom, not by habit. |
| ★ usual | §4.8 | built | `screens/NewSession.tsx:345-350` | The user-set default. |
| Admission OK / Tight / No, never blocks | §4.8, §10 | partial | `screens/NewSession.tsx:493-517` | See §10 copy. |
| Signed-out + inline Sign in | §4.8 | built | `screens/NewSession.tsx:372-383,427-440` | |
| No-profiles empty state → [Add account] | §4.8, §8 | partial | `screens/NewSession.tsx:445-449` | Text only, no button. |
| Skeleton / Retry / offline-disabled | §8 | partial | `screens/NewSession.tsx:450-452,471,209` | "Loading…" text; no Retry. |
| "No folder picked — the profile's default applies" | §4.8 | missing | grep: none | |
| Runtime choice per session | D-40 | built | `screens/NewSession.tsx:54,206-207,264` | |

## 8. Switching, switch-back, context pack, park (wf11–wf14, wf27; §4.9; SPEC §7)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Limit prompt title | wf11 | built | `components/LimitDialog.tsx:149-151` | "{account} hit its limit". |
| Destination quota | wf11 | partial | `components/LimitDialog.tsx:191-195` | Not in the SwitchFlow popover. |
| What carries | wf11 | built | `components/LimitDialog.tsx:196-212`; `components/SwitchFlow.tsx:52-61` | |
| Estimated time | wf11 | missing | grep | |
| One primary "Continue on X ⏎" | wf11, D-03 | partial | `components/LimitDialog.tsx:213-215` → `components/SwitchFlow.tsx:433` | Two clicks. |
| Choose another… | wf11 | built | `components/LimitDialog.tsx:244-250` | |
| Park until reset | wf11 | built | `components/LimitDialog.tsx:230-239` | Not in the popover. |
| 10-second Undo | wf11 | built | `components/SwitchFlow.tsx:716-745` | |
| Door: one click only for a proven pair | wf11, D-03, SPEC §7.3 | built | `crates/juggler-handover/src/door.rs`; `components/SwitchFlow.tsx:374,439` | |
| Limit prompt reachable when the limit hits | §6 flow 2 | partial | `components/HomeNeeds.tsx:66-67` → `App.tsx:1100` | Only from a Home row. |
| "Beta — not proven" badge | wf12, D-04, D-11 | built | `components/SwitchFlow.tsx:183-185,475-477`; `components/SwitchTargetRows.tsx:22-46` | |
| Wizard steps | wf12 | built | `crates/juggler-handover/src/door.rs:76-80`; `components/SwitchFlow.tsx:443-469` | Only the steps that apply. |
| Copy / Translate / Regenerate / Drop per item | wf12, SPEC §7.1 | partial | `components/SwitchFlow.tsx:497-524` | Grouped; Translate is not shown as such. |
| Destination's secrets policy shown | wf12, D-14 | built | `components/SwitchFlow.tsx:124-128` | |
| "Change in Settings" link | wf12, AJ-E208 | missing | `components/SwitchFlow.tsx` | |
| Primary "Continue — I understand the drop list" | wf12 | missing | `components/SwitchFlow.tsx:620-633` | "Switch ▸". |
| Back / Cancel | wf12 | partial | `components/SwitchFlow.tsx:601-609,276-279` | No Cancel button in the wizard. |
| Policy asked at the first switch | D-25, D-14 | built | `components/SwitchFlow.tsx:90-122` | |
| Fidelity tier + carry list + redaction preview | §4.9 | built | `components/SwitchTargetRows.tsx:115-125`; `components/SwitchFlow.tsx:154-171` | |
| Freeze → Copy → Resume progress | §4.9 | built | `components/SwitchFlow.tsx:43-49,646-656` | Not ticked live. |
| Success copy | §4.9, §10 | built | `components/SwitchFlow.tsx:660-667` | Reworded. |
| Queue return at reset | §4.9, §6 flow 2 | missing | `lib/bridge.ts:1516` has no UI caller | |
| No other account → [Add account] | §4.9, §8 | partial | `components/HandoverSection.tsx:157-160` | No button. |
| Failure with Retry | §4.9, §10 | partial | `components/SwitchFlow.tsx:680-701` | No Retry. |
| Esc blocked during the copy | §4.9 | bug | `components/SwitchFlow.tsx:220-226,276-279` | Closes in every phase. |
| ⌘⇧H | §4.9 | built | `lib/shortcuts.ts:123`; `components/HandoverSection.tsx:59-67` | Focuses the section. |
| Switch-back prompt | wf13, AJ-E207 | built | `crates/juggler-handover/src/switch_back.rs:209-213`; `components/HandoverSection.tsx:118-131` | |
| Delta summary | wf13, D-05 | built | `components/SwitchBackFlow.tsx:190-195` | |
| "Original history preserved" before the click | wf13 | partial | `components/SwitchBackFlow.tsx:302-304` | Only while running. |
| Divergence shown, never merged silently | wf13, SPEC §7.4 | partial | `components/SwitchBackFlow.tsx:187-189` | Always says "Both sides moved". |
| Diverged → compare view | wf13 | partial | `components/SwitchBackFlow.tsx:197-224` | Radios, no side-by-side. |
| Resume + delta / Stay | wf13 | built | `components/SwitchBackFlow.tsx:267-287` | "Stay" is "Cancel". |
| View full pack… | wf13 | missing | grep | |
| Pack preview screen (contents, manifest) | wf14 | missing | grep | |
| Policy in effect (global + overrides) in context | wf14, D-14 | partial | `components/PackSecretsCard.tsx:106-135`; `components/SwitchFlow.tsx:126-128` | One line in the switch. |
| Detected items: treatment + why | wf14, AJ-E208 | partial | `components/SwitchFlow.tsx:154-171` | No "why". |
| Change policy for this pack… | wf14 | partial | `components/SwitchFlow.tsx:129-171` | Only under "Ask". |
| Open Settings → Redaction | wf14 | missing | grep | |
| Project context sync, diff-previewed | SPEC §7.5 | built | `components/ProjectSyncSheet.tsx:1-15` | |
| Parked card "parked until …" | wf27, AJ-E1003 | built | `screens/Home.tsx:486-520`; `components/ParkCard.tsx:31-60` | |
| "pack staged ✓" | wf27 | missing | backend `crates/juggler-app/src/parked.rs:39-40,168-209` | |
| Resume now | wf27 | partial | `screens/Home.tsx:513-515` | Only once the reset is reached. |
| View pack | wf27 | missing | grep | |
| Reset notification: Resume / Stay / Snooze once | wf27 | built | `components/NotificationRegion.tsx:363-394` | |
| Palette park action | wf27 | missing | `lib/action-specs.ts:33-43` | |

## 9. First run and adding (wf16–wf18, §4.16, §5, SPEC §12.1)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Three steps: what's here → sign in → OS access | D-25 | built | `screens/FirstRunSteps.tsx:12-19`; `screens/FirstRun.tsx:1236-1290` | |
| Found / Likely / Guessed + source per item | wf16, SPEC §12.1, AJ-E303 | missing | `types/generated/DiscoveredInfo.ts:44` (sent, never shown) | Removed in `91b640d6`; no decision drops it. |
| Per-item opt-in, preselected | wf16, D-25 | built | `screens/FirstRun.tsx:129,430-438` | |
| Continue with selected | wf16 | built | `screens/FirstRun.tsx:604-617` | "Bring in N accounts". |
| Rescan | wf16 | partial | `screens/FirstRun.tsx:379-383` | Only after a failed scan. |
| Start empty instead | wf16 | deviates (D-25) | `screens/FirstRun.tsx:584,1230` | Add a fresh account / Not now. |
| "Nothing is modified" | wf16 | built | `screens/firstRunScan.ts:60-69` | |
| Nothing found → install path | wf16, §5 | built | `screens/firstRunScan.ts:50-55`; `components/InstallHow.tsx` | |
| Where we looked | AJ-E302 | built | `screens/FirstRun.tsx:590-600,623-636` | |
| Recommended set, 1-click accept, Adjust… | wf17 | deviates (D-25) | `screens/FirstRun.tsx:604` | |
| Required redaction choice | wf17, D-14 | deviates (D-25) | moved to the first switch | |
| Optional renewal dates | wf17, SPEC §12.1 | missing | grep in `screens/FirstRun*.tsx` | Not recorded as dropped. |
| Optional "Connect a Figurehead account" step | D-18, AJ-E1203 Req.3, SPEC §12.1 | missing | grep: none | Removed with the old wizard; see "Needs a ruling". |
| "Step N of M" text + announcement | §4.16 | partial | `screens/FirstRunSteps.tsx:22-46` | |
| "I'll set up later", not ✕ | §4.16 | built | `screens/FirstRun.tsx:1224-1233` | "Not now". |
| Resumable "Finish setup (1 left)" | §4.16, §5 | partial | `components/Welcome.tsx:112`; `screens/Profiles.tsx:292` | No count; doesn't resume at the same step. |
| "Takes about 2 minutes" | §4.16 | missing | grep | |
| Import ceiling explained before the click | §4.16 | built | `screens/FirstRun.tsx:1006-1009` | |
| Sign-in pending + Re-check | §4.16 | partial | `screens/FirstRun.tsx:707-709` | No Re-check. |
| "AI Juggler never sees your credentials." | §4.16 | built | `screens/Profiles.tsx:389`; `screens/FirstRun.tsx:806` | |
| Scanning placeholder | §4.16 | built | `screens/FirstRun.tsx:364` | |
| Funnel events | §5 | partial | `lib/funnel.ts:22-39` | No `wizard_shown`, `account_created`, `first_session_started`, `second_account_added`, `first_handover_completed`; `login_completed` is never emitted. |
| Funnel counts in Settings → Data | §5 | missing | `screens/settings/*` | |
| Add Account entry | wf18, §4.4 | built | `screens/Profiles.tsx:182`; `App.tsx:278-290` | |
| Add Provider entry | wf18 | deviates (AJ-E1023) | `screens/AddProviderSheet.tsx:14-27` | Asked only with more than one tool installed. |
| Add Instance entry | wf18 | deviates (D-25) | — | |
| Import… entry on Accounts | wf18, §4.4 | partial | `screens/FirstRun.tsx:588` | Inside the flow only. |
| Sign-in methods | wf18 | built | `screens/FirstRun.tsx:184-201` | ADR-0018 words. |
| Label | wf18 | built | `screens/FreshAccountStep.tsx:115-125` | |
| Isolation verified and shown after sign-in | wf18, SPEC §12.1 | missing | `crates/juggler-app/src/commands.rs:1962` has no UI caller | |

## 10. Accounts and an account's page (§4.4, §4.5)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Rows: glyph, label | §4.4 | built | `screens/accounts/AccountCard.tsx:97-112`; `screens/accounts/ProfileRow.tsx:86-93` | |
| Plan chip | §4.4 | partial | `screens/accounts/AccountCard.tsx:84-86` | Text only. |
| Login chip incl. "Expired?" | §4.4 | partial | `screens/accounts/ProfileRow.tsx:139-145` | |
| Mini 5h meter | §4.4 | built | `screens/accounts/AccountCard.tsx:146` | |
| n live | §4.4 | built | `screens/accounts/ProfileRow.tsx:131` | |
| Last used | §4.4 | missing | grep | |
| Group by harness | §4.4 | deviates (D-35) | `screens/Profiles.tsx:153-163` | |
| Row ⋯ incl. Re-check | §4.4 | partial | `lib/action-specs-setup.ts:89-139` | Re-check only in the sign-in modal. |
| Empty state + Import | §4.4 | built | `screens/Profiles.tsx:278-296` | |
| Loading placeholders | §8 | built | `screens/Profiles.tsx:148` | |
| Quota unknown never 0 % | §4.4 | built | `screens/accounts/AccountCard.tsx:49-56` | |
| Signed-out sorts last | §4.4 | built | `screens/accounts/AccountCard.tsx:1-6` | |
| Keys `a`, `/` | §4.4 | missing | `lib/shortcuts.ts` | |
| Rename | §4.5 | built | `pages/ItemHeader.tsx:136`; `screens/AccountManage.tsx:17-55` | |
| Plan chip + Change plan… | §4.5 | partial | `pages/account/Settings.tsx:24` | Set inside Rename. |
| Login chip + Sign in / Re-check on the page | §4.5 | partial | `pages/ItemHeader.tsx:302-304`; `pages/AccountPage.tsx:52-62` | Sends you to Accounts. |
| Quota card: 5h + weekly + countdowns | §4.5 | built | `pages/AccountPage.tsx:171` | |
| "Exhausts ~ at current burn" on the page | §4.5 | missing | `components/usage/RunwayCards.tsx:92` (Usage only) | |
| Health card with honest unknowns | §4.5 | partial | `pages/account/Settings.tsx:24-35` | |
| Delete launch settings asks first | §4.5 | missing | `screens/accounts/ProfileRow.tsx:208` | One click. |
| Remove blocked while live | §4.5 | built | `screens/AccountManage.tsx:100-103` | |
| Recent sessions + View all | §4.5 | built | `pages/registry.ts:112`; `pages/AccountPage.tsx:107-124` | |
| ▸ Details (folder, created, storage) | §4.5 | missing | `pages/AccountPage.tsx` | |
| Keys l, ⇧L, u | §4.5 | missing | `lib/shortcuts.ts` | ⇧L would clash with ⌘⇧L. |
| "Ask Claude Code for its limits" | D-39 | built | `pages/AccountProbe.tsx:48-72`; `crates/juggler-app/src/features/quota/probe.rs:69` | |

## 11. Usage and Cost (wf19, wf20, §4.10, §4.11, SPEC §6)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Account cards + sparkline | wf19, §4.10 | built | `components/usage/RunwayCards.tsx:68-103`; `components/usage/Runway.tsx:178` | |
| Smart suggestion naming the freshest target | wf19 | partial | `components/AccountRunway.tsx:118-150` (Accounts only) | See "Needs a ruling". |
| Window clock 5h + weekly with reset | wf19 | built | `components/usage/Runway.tsx:44-68` | |
| Burn / forecast + Estimated | wf19, §4.10 | built | `components/BurnDown.tsx:32-36,85-104` | Tag reads "estimate". |
| Forecast suppressed with little history | §13.1 (D6: < 1 h) | partial | `crates/juggler-core/src/quota_forecast.rs:9,23` | < 10 min. |
| Subscription clock beside the window clock on Usage | wf19, SPEC §6, D-12 | partial | `screens/accounts/AccountCard.tsx:86,118-122`; `pages/account/Settings.tsx:142-175` | Never together on Usage. |
| "No renewal date … Add date" | wf19, D-12 | partial | `pages/account/Settings.tsx:29` | "Expiry: unknown". |
| Renewal warning N days before | D-12, wf26 | built | `lib/renewal.ts:39-50` | Fixed thresholds. |
| Range 5h / 7d / 30d | §4.10 | partial | `lib/usage/range.ts:11-16` | No 5h. |
| Series toggles | §4.10 | partial | `components/usage/Dashboard.tsx:160` | |
| View as table | §4.10 | built | `components/usage/Dashboard.tsx:172` | Off by default. |
| Model tokens / share / trend | wf19 | partial | `components/usage/Dashboard.tsx:190-194` | |
| Thresholds link | §4.10 | missing | `screens/Usage.tsx` | |
| Export CSV (usage) | §4.10 | partial | `components/CostMatrix.tsx:99` | Cost CSV only. |
| Pre-feed state | §4.10 | built | `components/usage/Runway.tsx:80-95` | |
| Estimated banner | §4.10 | partial | inline word | |
| Per-card stale | §4.10 | built | `components/usage/Runway.tsx:98-105` | |
| Keys ←→ a e | §4.10 | partial | `components/usage/controls.tsx:48-51` | |
| Soft budget (warn only) | §4.11 | built | `components/BudgetCaps.tsx:1-5` | |
| Cards: 5h block / Today / Month | wf20, §4.11 | partial | `components/usage/Dashboard.tsx:126-136` | One tile for the range. |
| Cost per finished task | wf20 | missing | grep in UI and crates | |
| Wasted spend (crashed/stuck) + links | wf20 | missing | grep | |
| Per-account table: top project, confidence, cache tokens | wf20, §4.11 | partial | `components/CostMatrix.tsx:30-71` | |
| Export CSV | wf20, §4.11 | built | `components/CostMatrix.tsx:99-101` | Inside "Prices and details". |
| Weekly cost review | wf20 | missing | grep | |
| Subscription fees kept separate | wf20 | built | `components/CostMatrix.tsx:110-115` | |
| Pricing version + [Update] | §4.11 | missing | `crates/juggler-core/src/quality/pricing.rs:38` never shown | |
| No-pricing state | §4.11 | missing | grep | |
| Unknown model never $0.00 | §4.11 | bug | `components/usage/Dashboard.tsx:129`; `components/charts/scale.ts:98` | The table is right (`components/CostMatrix.tsx:60-64`); the tile reads "$0". |
| "~" on every estimate | §4.11, §10 | partial | `components/CostMatrix.tsx:104` | |

## 12. Cleanup and verify-before-kill (wf21, wf22, SPEC §5.2, §9.4)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Leftovers list | wf21, §9.4 | built | `screens/processes/Toolbar.tsx:34`; `lib/process-states.ts:37` | "Left behind". |
| Port, MB, CPU, age | wf21, §9.4 | built | `screens/processes/cells.tsx:163-191` | |
| Command line + cwd in the inventory | §9.4 | partial | `components/KillSheet.tsx:378-383` | Only in the kill sheet. |
| Ownership proof per leftover | wf21 | missing | grep ledger/start time | |
| "Blocks" impact | wf21 | partial | `components/PortMap.tsx:161`; Ports view | Not on the row. |
| Verify & clean… | wf21 | built | `screens/processes/RowActions.tsx:74-77,84-88` | "Stop…". |
| Keep (not a leftover) | wf21, AJ-E2405 | built | `screens/processes/RowActions.tsx:143-145` | |
| Remind me later | wf21 | built | `screens/processes/RowActions.tsx:152-154` | 1 h fixed. |
| Not-owned as context only | wf21, D-08 | deviates (D-41) | `components/KillSheet.tsx:133-135` | |
| Empty "Nothing left behind" | wf21 | built | `screens/processes/ProcessesView.tsx:172` | |
| Hog alert | wf21, AJ-E512 | built | `screens/processes/CleanupStatus.tsx:122-144` | |
| Per-account Notify only / Clean safe | wf21, D-08 | built | `screens/processes/RulesPanel.tsx`; `components/SettingsSections.tsx:89-115` | Different words. |
| Pause / re-confirm after a crash / Undo | SPEC §5.2 | built | `screens/processes/CleanupStatus.tsx:78-116` | |
| Audit log of every kill | §9.4 | partial | `screens/processes/HistoryView.tsx:72-89`; `crates/juggler-core/src/storage/cleanup_audit.rs:262,340` | 7 days, not "ever". |
| Four named checks | wf22 | missing | `components/KillSheet.tsx:376-396` | |
| TERM → 5 s → KILL explained | wf22 | built | `components/KillSheet.tsx:470` | Jargon (section 4.1). |
| "Recorded in the audit log" promise | wf22 | missing | `components/KillSheet.tsx` | |
| Clean / Cancel (Esc) | wf22 | built | `components/KillSheet.tsx:241,576,590` | |
| Fail-closed with reason | wf22, AJ-E503 | built | `components/KillSheet.tsx:310,360-373,579-583` | |
| Scope, countdown, per-process outcome, port re-probe | SPEC §5.2 | built | `components/KillSheet.tsx:401-560` | |

## 13. Fleet (wf23, wf24, SPEC §11)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Hidden until paired | §11.3, D-25 | built | `components/MainNav.tsx:44-48` | |
| Machines: this / online / offline + last seen | wf23 | built | `components/FleetThisMac.tsx`; `screens/FleetScreen.tsx:252,300-305` | |
| Pair a machine… | wf23 | built | `screens/FleetScreen.tsx:132,141` | |
| 6-digit code, 10 min | wf23 | deviates (ADR-0013) | `screens/PairingWizard.tsx:355-359,732` | 2 min; link instead of QR. |
| Mutual confirmation | wf23 | built | `screens/PairingWizard.tsx:628-655` | |
| mTLS shown | wf23 | built | `components/FleetChips.tsx:16` | |
| Table: Instances / Needs you / Leftovers per Mac | wf23 | partial | `screens/FleetScreen.tsx:520-563` | No counts. |
| Expired / rejected / offline states | wf23 | built | `screens/PairingWizard.tsx:729-775`; `screens/FleetScreen.tsx:232-266` | |
| Permission tiers incl. No access; never silent | wf24, SPEC §11 | partial | `screens/FleetScreen.tsx:593-605`; `components/RemoteTabs.tsx:383` | No access can't be set; "View + control" acts silently. |
| Remote inspector, six tabs | wf24 | built | `components/RemoteTabs.tsx:24` | |
| Open remote session | wf24 | partial | `components/RemoteTabs.tsx:290-335` | Desktop sessions only. |
| Switch on that machine… | wf24 | partial | `components/HandoverSection.tsx:176` | Not in the remote inspector. |
| Clean a remote leftover with the same verify sheet | wf24 | partial | `components/RemoteCleanup.tsx:122-127,167-200` | Own sheet. |
| Auto-clean remote: Off, stated | wf24 | partial | `components/RemoteCleanup.tsx` | Implicit. |
| Machines as filters across the board and usage | SPEC §11.4 | partial | `components/RemoteBoard.tsx`; `screens/Usage.tsx:85` | Sections, not filters. |

## 14. Settings (wf26, §4.15)

| Item | Spec ref | Status | Evidence | Note |
|---|---|---|---|---|
| Tabs General / Notifications / Supervisor / Usage & Cost / Data & Privacy / Shortcuts / Advanced | §4.15 | deviates (AJ-E2110) | `screens/settings/panes.ts:38-52` | Panes mapped below. |
| Vertical tablist, ↑↓ | §9 | partial | `screens/Settings.tsx:77-101` | Buttons with `aria-current`. |
| `/` filter | §4.15 | missing | grep | |
| Saved tick + Undo toast | §4.15 | partial | `components/SettingsSections.tsx:37-51` | No Undo. |
| Save error reverts the value | §4.15, §8 | bug | `components/NotificationSettings.tsx:37,78` | Optimistic; no rollback. |
| Skeleton pane | §8 | missing | `lib/lazy.tsx:43` | |
| Appearance (Light/Dark/System) | wf26, D-16 | built | `components/AppearanceCard.tsx:18-51` | |
| Density | D-25 | built | `components/DensityChoice.tsx` | |
| Reduce motion | §4.15 | partial | `styles.css:289` | OS only. |
| Startup view | §4.15 | missing | grep | |
| Update channel + Check for updates + off switch | SPEC §10, §4.15 | built | `components/UpdatesCard.tsx:275-309,403-417` | |
| Open at login | D-26 | built | `components/SessionsSettings.tsx:47-83` | |
| Notifications: per-event | §4.15 | built | `components/NotificationSettings.tsx:19-26` | |
| Notifications: waiting threshold / quota 80 %/95 % / digest time / Send test | §4.15 | missing | `types/generated/NotificationPrefsInfo.ts` | Section 2. |
| Stuck threshold | §4.15 | built | `components/SessionsSettings.tsx:234-245` | |
| Crash auto-restart (off) | §4.15 | built | `components/SessionsSettings.tsx:93-99`; `components/RecipeOptions.tsx:17` | |
| RAM/swap/disk + admission margin | §4.15 | missing | grep | |
| Plan caps / soft budget | §4.15 | deviates (moved) | `screens/Usage.tsx:84` | On Usage. |
| Pricing sync, CSV defaults | §4.15 | missing | grep | |
| Retention | wf26, §4.15 | built | `components/StorageRetentionCard.tsx:23-29` | No separate archive step. |
| Backup / Restore | wf26 | built | `components/BackupRestoreCard.tsx:133,142` | |
| Export diagnostics (redacted) | §4.15 | built | `components/DiagnosticsCard.tsx:1-60` | |
| Data sizes, paths, Reveal | §4.15 | partial | `components/StorageRetentionCard.tsx:71` | |
| Usage statistics off/local/share | D-19 | built | `components/UsageStatsChooser.tsx:14-26` | |
| Shortcuts table / rebind / reset | §4.15 | partial | `components/ShortcutSheet.tsx` | Read-only sheet; no rebind. |
| Go-to hotkey with conflict refusal | D-36 | built | `screens/settings/DesktopAppsPane.tsx:135-202` | |
| Context packs & secrets: global + per-provider | wf26, D-14 | built | `components/PackSecretsCard.tsx:106-135` | |
| Cleanup default + per-account auto-clean | wf26, D-08 | built | `components/SettingsSections.tsx:89-115,150-160` | |
| Renewal dates + warn 7 days | wf26, D-12 | built | `components/SettingsSections.tsx:166-259` | |
| Fleet permissions | wf26 | built | `components/SettingsSections.tsx:262-321` | |
| Agent access | D-28 | built | `screens/settings/AgentAccessPane.tsx` | |
| Tool versions / verified ranges | §4.15 | built | `screens/settings/AccountsPane.tsx:42` | |
| Tool update notice (notify only) | SPEC §10 | partial | `components/RightInspector.tsx:977` | Inspector only. |
| Logs | §4.15 | partial | `components/DiagnosticsCard.tsx:57` | Inside the zip. |
| Figurehead & Cloud section, consent | D-18, AJ-E1203 | built | `screens/settings/AdvancedPane.tsx:36-43`; `components/FigureheadCloudCard.tsx:112-113` | Renamed per D-35. |

---

## Intentional deviations

These follow a later decision or task and are not gaps:

| Deviation | Ruling | Evidence |
|---|---|---|
| Header: Home, Accounts, Usage, History, Settings + More | D-25 | `lib/nav.ts:63-76` |
| Triage and Overview folded into Home; Review queue folded into "Mark reviewed" | D-25 (AJ-E1021) | `screens/Home.tsx` |
| "Session" on screen; no Instances screen; no Add Instance | D-25 | — |
| Rail by place with View ⇧V; Needs you / Open now / "Also:"; ⌘1–9 bring apps forward | D-36, AJ-E2104 | `components/PlaceBoard.tsx` |
| Row details only as exceptions | D-25, D-36 | `components/PlaceRows.tsx:182-188` |
| Five state words with "Probably" | D-36 | `components/indicator.tsx:319` |
| No default global hotkey; ⌥Space refused | D-36 | `lib/hotkey.ts` |
| Per-type tabs instead of six everywhere; Actions in the header; Quality in Usage | AJ-E1533 | `pages/registry.ts:105-123` |
| Session header shows less; model/context/branch one step deeper | AJ-E2108, D-25 | `pages/session/SessionHead.tsx` |
| Cleanup inside the Processes view | AJ-E2404 | `screens/processes/` |
| User may stop another app's verified leftover | D-41 | `components/KillSheet.tsx:133-135` |
| Stop held 5 s with Undo instead of a confirm | AJ-E1503 Req.2 | `lib/stop.ts` |
| First run in three steps; pack-secrets choice at the first switch; one-button bring-in | D-25, AJ-E1022, AJ-E1023 | `screens/FirstRun.tsx` |
| Accounts grouped by subscription, profiles beneath | D-35 | `screens/Profiles.tsx:153-163` |
| Settings as System-Settings panes | AJ-E2110 | `screens/settings/panes.ts` |
| Dark by default, not the OS scheme | D-16 | `theme.ts:10` |
| Pairing code 2 min, link instead of QR | ADR-0013 | `screens/PairingWizard.tsx:732` |
| ⌘N = New project; ⌘. = Stop; ⌘⇧H = switch | AJ-E305, deep-02 SP-04, §13 | `lib/shortcuts.ts` |

## Method

1. Each source was read in full. That covered the wireframes as text, the README rules, the approvals, spec 11 §0–§14, SPEC §9 and the related sections, and decisions D-01..D-44.
2. Every control, named state, key and copy line was then searched for in `ui/src`, and followed to `App.tsx` to confirm it is reachable.
3. Five areas were checked in parallel by read-only agents: shell, inspector/session, launch/switching, onboarding/usage, and cleanup/fleet/settings.
4. Their highest-impact claims were checked again by hand before they went in here:
   - the `hasHistory` producer;
   - snippet redaction;
   - the hotkey handler;
   - the Esc handling in SwitchFlow;
   - the cost tile's `?? 0`;
   - the launch-settings delete;
   - the switch-back done line.
5. One finding was changed by that check. D-31 is scoped to agents and tasks, so ranking in the user's own launch and switch sheets is raised as a question for a ruling, not counted as a violation.
