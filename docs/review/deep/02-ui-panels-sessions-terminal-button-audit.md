# 02 — UI Panels, Sessions & Terminal: Button-by-Button Audit

> Scope: every control in `ui/src/screens/*`, `ui/src/components/*`,
> and the shell in `ui/src/App.tsx`, read in full (2,538 LOC).
> Method: **Today** = code as-is with `file:line`; **SPEC S9** =
> `docs/SPEC.md:316-353` (§9) plus §5/§6/§7 promises it depends on;
> **Target** = proposed label, placement, shortcut, states
> (hover/focus/disabled/loading/error/empty), destructive guard,
> undo, and telemetry event. Control IDs (`SP-`, `PP-`, `NS-`…)
> are stable handles for tickets. Prior review findings are cited
> as `full-competitive-ux-review.md` (hereafter FCUR) and deepened,
> not restated. Competitor URLs are evidence, not decoration.

## 0. Verdict in one paragraph

The shell is honest and lean — native `<button>`s everywhere,
`:focus-visible` rings (`ui/src/styles.css:76`), `⌘B`/`⌘N`
(`ui/src/App.tsx:205-219`) — but the daily surface is
**action-poor and state-blind**: the 9-state session lifecycle
(`crates/juggler-core/src/model.rs:188-207`) renders as 5 tones
(`ui/src/screens/WorkPanels.tsx:16-31`), there is no Resume,
no search, no confirmation on any destructive action, no undo
anywhere, and the terminal is a bare FitAddon-only xterm
(`ui/src/components/TerminalView.tsx:51-52`,
`ui/package.json:18-19`). SPEC S9 promises an instance panel with
resources, tokens, Attach/Focus/Stop/Hand-over
(`docs/SPEC.md:347`) and a sessions table with Resume
(`docs/SPEC.md:348`); neither exists as specified. Section 9
redraws the three panels; Section 8 specifies the terminal.

## 1. Inventory — what was actually read

| Surface | File | LOC | Controls found |
|---|---|---:|---:|
| App shell + Overview + ComingSoon | `ui/src/App.tsx` | 539 | 14 |
| Workflow rail | `ui/src/components/WorkflowRail.tsx` | 444 | 11 |
| Session/Project/Empty panels | `ui/src/screens/WorkPanels.tsx` | 443 | 13 (+6 dead) |
| New session (live) | `ui/src/screens/NewSession.tsx` | 229 | 12 |
| Accounts & Profiles | `ui/src/screens/Profiles.tsx` | 411 | 18 |
| Onboarding wizard | `ui/src/screens/OnboardingWizard.tsx` | 436 | 19 |
| Ready home | `ui/src/screens/ReadyHome.tsx` | 93 | 3 |
| Terminal | `ui/src/components/TerminalView.tsx` | 116 | 0 (no chrome) |
| Primitives / form / icons | `ui/src/components/primitives.tsx:1-218`, `form.tsx:1-97`, `icons.tsx:1-112` | 427 | shared |

Dead code confirmed: `NewSessionModal` in
`ui/src/screens/WorkPanels.tsx:355-443` duplicates
`ui/src/screens/NewSession.tsx:18-227` and is never imported by
`ui/src/App.tsx:10-14`; it also teaches a *typed path*
(`WorkPanels.tsx:399-407`) that the live modal deliberately
abolished. Delete it (FCUR F2.8) — every audit row below for
"New session" refers to the live `screens/NewSession.tsx`.

## 2. SPEC S9 promise vs. today (the gap map)

| S9 promise (`docs/SPEC.md`) | Today | Gap |
|---|---|---|
| Instance panel: status hero, account, age, resources, tokens (`:347`) | Header + terminal + handover chips only (`WorkPanels.tsx:70-189`) | No RAM/CPU/tokens/age-hero; age is 11px footer text (`:183`) |
| Sessions table with Resume (`:348`) | Rail rows are buttons; no table, no Resume (`WorkflowRail.tsx:397-443`) | Resume/Relaunch entirely absent (FCUR F2.3) |
| Hand-over wizard with tier + headroom (`:287,347`) | Inline chips fire immediately (`WorkPanels.tsx:144-174`) | No tier, no headroom, no confirm (see SP-08) |
| Overview: quota rings + machine gauges (`:345`) | Rings hardcoded `percent={0}` (`App.tsx:513-514`); "Machine OK / Fleet 1" hardcoded (`:461-473`) | Fabricated-zero anti-pattern (FCUR F2.4) |
| Usage / Fleet screens (`:349-350`) | `ComingSoon` cards (`App.tsx:338-341,525-539`) | Dead nav items train non-exploration |
| Terminal dock, one tab per instance (`:351`) | One terminal per panel, no tabs (`TerminalView.tsx:29`) | No multi-terminal surface at all |
| Tray + global hotkey (`:352`) | Absent from `ui/src` | Nothing to audit; spec-only |
| Settings: thresholds, terminal prefs (`:353`) | Absent from `ui/src` | Terminal prefs (font, scrollback) unconfigurable |
| Stuck/WaitingUser/QuotaBlocked (`:220-224`) | Modelled (`model.rs:194-198`), never rendered distinctly | See §7 state machine |

## 3. App shell (`ui/src/App.tsx`) — control by control

Legend: **H/F/D/L/E/Ø** = hover / focus / disabled / loading /
error / empty behaviour today. "—" means the state does not exist.

| ID | Control (today, `file:line`) | H/F/D/L/E/Ø today | Target (label · placement · shortcut · guard/undo · telemetry) |
|---|---|---|---|
| SH-01 | Rail logo = collapse toggle (`App.tsx:259-268`) | H: bg; F: none visible; D/L/E: — | Keep; add `aria-expanded`, tooltip "Collapse rail (⌘B)"; telemetry `rail_toggle` |
| SH-02 | Global nav ×4 Overview/Accounts/Usage/Fleet (`:306-320`) | H: bg; F: —; active: sheen chip | Usage/Fleet get `aria-disabled` + "M3" suffix until real; or hide (FCUR UX.1); `nav_global` |
| SH-03 | Header **New** primary (`:321-324`) | H: brightness; D: — | Keep; label "New session", `⌘N` in tooltip; primary everywhere consistent; `cta_new_session{source:header}` |
| SH-04 | Drag region header (`:298-301`) | No affordance | Keep; ensure drag region never swallows clicks (already `data-tauri-drag-region={false}` on nav, `:305`) |
| SH-05 | Handover error → `window.alert` (`:371`) | Modal OS alert; blocks; no retry | Inline error banner in SessionPanel + toast with Retry; never `alert`; `handover_failed` |
| SH-06 | Sign-in error → `window.alert` (`:384`) | Same | Same banner system (FCUR F2.1); `login_spawn_failed` |
| SH-07 | `refresh()` catch swallows all (`:101-116`) | Stale data, zero signal | App status model `loading/ready/degraded/error`; degraded banner + Retry; `refresh_failed` |
| SH-08 | First paint before fetch | `WorkEmpty` flashes (no loading gate) | Skeleton rail + panel until first `refresh()` resolves; `shell_ttfp` |
| SH-09 | Overview StatCards ×4 (`:448-473`) | Static; Machine/Fleet fabricated | Honest empty: "No data yet — arrives M3"; live counts stay; `overview_view` |
| SH-10 | Overview live rows (`:498-505`) | Non-interactive `<div>`s | Make rows buttons → `selectSession`; keyboard reachable; `overview_open_session` |
| SH-11 | QuotaRing ×2 `percent={0}` (`:513-514`) | Renders real-looking 0% | Replace with dashed "—" + "Quota arrives in M3" (already captioned `:516-518`; ring must match caption) |
| SH-12 | ComingSoon Usage/Fleet (`:525-539`) | Dead end, no CTA | Add "Notify me" (local pref) + link to EPICS milestone; or remove from nav (SH-02) |
| SH-13 | Wizard auto-open at 0 accounts (`:94-99`) | Good; dismissal in `localStorage` | Keep; add "Don't show again" explicit checkbox vs. implicit dismiss |
| SH-14 | Folder-drop → NewSession (`:124-130`) | Silent prefill, no feedback | Toast "Folder added: <name>" + focus moves to profile list; `drop_folder` |

Keyboard today: only `⌘B`/`⌘N` (`App.tsx:206-219`). No `⌘K`,
no `⌘1..3` tab switching, no `Esc` stack discipline. Target:
`⌘K` palette (FCUR WOW-c), `⌘1/2/3` rail tabs, `⌘.` stop
session, `⌘⇧H` handover — all listed in §10.

## 4. Workflow rail (`ui/src/components/WorkflowRail.tsx`)

The rail *is* the navigation (ADR-0007, `App.tsx:38-42`), so its
row actions are the product's primary list actions.

| ID | Control (today) | Audit | Target |
|---|---|---|---|
| WR-01 | Tabs Projects/Instances/Sessions (`:8-12,126-157`) | `role=tab` + `aria-selected` (`:132-133`) but no `tabpanel`, no arrow keys; Instances ≡ Sessions (FCUR F2.2) | Full ARIA tabs (←/→/Home/End); merge Instances→Sessions until resources exist; badge slot per tab |
| WR-02 | Collapsed live dots ×6 (`:164-182`) | Colour-only status; `slice(0,6)` silent cap (`:166`) | Dot + count overflow "+N"; tooltip includes status text; needs-attention count badge |
| WR-03 | Project rows (`:259-289`) | Whole-row button; sub-line "N accts · M runs" (`:283-287`); no age, no branch, no dirty flag | Add last-activity age + branch chip; context menu (Launch…, Reveal in Finder, Rename, Remove from list — guarded); `rail_select_project` |
| WR-04 | InstanceList Live/Recent (`:295-326`) | `rest.slice(0,8)` (`:305`); duplicates SessionList | Delete tab (WR-01) or give it resource columns (RAM/CPU) per SPEC `:347` |
| WR-05 | Session rows (`:397-443`) | Status = raw string (`:433-435`); colour dot only; `slice(0,15)` (`:377`) with no "show more", no search/filter | Status chip with text (see §7); search input atop list; virtualised list past 50; row context actions: Resume/Stop/Handover/Open project |
| WR-06 | Sign-in rows (`:343-368`) | Ended state = grey dot only | Ended rows get "Re-check" + dismiss (×); live rows pulse + elapsed time |
| WR-07 | **New session** rail button (`:200-209`) | Primary, `⌘N` tooltip only when collapsed (`:203`) | Always show `⌘N` hint; loading spinner while modal profiles load |
| WR-08 | Resize separator (`:214-233`) | `role=separator` (`:218`) but not keyboard-operable; double-click collapses (`:217`) | ←/→ adjust (8px, `Home` reset 264px), `aria-valuenow` = width px; persist (already `localStorage`, `:19,92`) |
| WR-09 | Footer health dot (`App.tsx:276-292`) | Hardcoded "this-mac · healthy" | Bind to real supervisor health; degraded → warning text + click → Overview |
| WR-10 | Empty states (`:247-253,372-376`) | Text-only, no CTA | Add inline "New session" button in empty Sessions; Projects empty links to folder picker |
| WR-11 | Truncation (`:276,430-435`) | `truncate` with no tooltip on rows | `title` = full label + status on every truncated row |

Scale note: caps at 6/8/15 rows (`:166,305,377`) mean the rail
silently hides work exactly when juggling matters (15+ sessions,
FCUR F2.5). Target: search + filter chips (All / Needs me /
Running / Ended) above the list; counts on section headers.

## 5. Session panel (`WorkPanels.tsx:44-190`) — the daily surface

Header anatomy today: status dot (`:73-78`), title (`:79`),
status chip (`:80`), kind chip (`:81-83`), project jump button
(`:84-93`), "ran Xs · scrollback replayed" caption (`:95-99`),
Stop (`:100-112`); terminal card (`:117-126`); login-ended card
(`:128-137`); handover strip (`:139-187`).

| ID | Control | Today (evidence) | Target |
|---|---|---|---|
| SP-01 | Status dot + chip (`:73-80`) | 5 tones via `statusTone` (`:16-31`); `waiting_user/stuck/quota_blocked/handed_over` fall to neutral raw text | 9-state chip set (§7) with icon+text; `Stuck?` gets "?" honesty suffix per SPEC `:221` |
| SP-02 | Kind chip terminal/desktop (`:82`) | Text only | Keep; desktop gets "Focus window" primary action (SPEC `:347`) when adapter lands |
| SP-03 | Project jump (`:85-93`) | Small ghost button, no shortcut | Keep; add `⌘↑` "go to project"; breadcrumb form `Project / Session` |
| SP-04 | **Stop** (`:100-112`) | Live-only; dynamic-imports `terminalKill` (`:105`); **no confirm, no undo, no busy state**; failure unhandled (no catch) | Confirm only if session <60s old or has unsent input? No — Stop is recoverable via Resume, so: no confirm, but toast with **Undo (Resume)** for 10s; spinner while killing; error banner on failure; `⌘.`; telemetry `session_stop{age_s}` |
| SP-05 | Terminal card (`:117-126`) | No chrome at all (see §8) | Toolbar: Search (`⌘F`), Copy, Clear, font −/+, reconnect banner, scrollback badge |
| SP-06 | Desktop placeholder (`:121-124`) | "focus controls arrive with the desktop adapter" — dead text | Keep honest copy; add disabled "Focus window" with `title` explaining milestone — or hide panel section entirely |
| SP-07 | Login-ended guidance card (`:128-137`) | Static text pointing to Accounts | Add inline **Re-check login** button here (round-trip to Accounts is the #1 login friction); `login_recheck{from:panel}` |
| SP-08 | Handover chips ×≤6 (`:144-174`) | One click = immediate handover (`:157` → `App.tsx:365-373`); disabled if not logged in (`:156`); `slice(0,6)` (`:147`) hides 7th+ profile; no tier/headroom/fidelity (SPEC `:287` requires both) | Replace with **Hand over…** button opening 3-step wizard (target ranked by headroom → fidelity carry-list → confirm); chips become wizard rows with quota forecast (FCUR WOW-b); overflow scrolls, never slices; destructive? No (source stays resumable, SPEC `:287`) — but confirm step still required because it spawns a billed session; telemetry `handover_start/tier_shown/confirmed/completed` |
| SP-09 | Handover empty (`:175-179`) | "Add another account…" text only | CTA button "Add account" → wizard; deep-link, don't describe |
| SP-10 | Footer meta (`:181-185`) | harness + age + prose claim "transcript copied, then resumed" | Add tokens, cost (Est.), branch, cwd (copyable), native session id; fidelity label must match reality (FCUR F3.2: transcript-only today) |
| SP-11 | Ended-session actions | **None.** No Resume/Relaunch/Fork/Export/Archive/Delete anywhere in the panel | Action bar for ended states: **Resume** (primary, `⌘R`), Relaunch same profile+cwd, Hand over…, Export pack…, Archive, Delete (guarded, see §9). This is the single largest S9 gap (`SPEC.md:348`) |
| SP-12 | Crash/failed state | `failed` = red dot only (`:74-77`) | Recovery card (SPEC `:250`): exit reason, **Resume** / Relaunch / Hand over — one click each; `crash_recovery_shown` |
| SP-13 | Panel loading/error | None: `session=null` renders title "Session" + empty terminal card | Skeleton header + terminal shimmer; stale-session banner if id not found after refresh |

## 6. Project panel (`WorkPanels.tsx:196-330`)

Anatomy: header + active-instance chip (`:217-229`), root path
(`:230`), "On it now" card (`:233-252`), "Launch with another
account" list (`:254-291`), "Sessions in this project" list
(`:294-326`). Two-column grid at `xl` (`:232`).

| ID | Control | Today | Target |
|---|---|---|---|
| PP-01 | Active-instance chip (`:222-228`) | "X on it" / "idle" | Add account + elapsed; quota mini-ring when M3 lands |
| PP-02 | Root path (`:230`) | Mono 11px, not copyable, no Reveal | Copy-on-click + Reveal in Finder icon-button; missing-dir → danger state with Relocate… |
| PP-03 | On-it-now row button (`:236-249`) | Opens session; status chip; "open ⌄" | Keep; add Stop icon-button (with SP-04 rules) without entering panel |
| PP-04 | On-it-now empty (`:251`) | "No live instance — free to juggle." | Keep copy; add primary "Start session" using usual account (F1.4 affinity) |
| PP-05 | Launch-with rows ×≤5 (`:258-285`) | Disabled + "sign in first" chip if logged out (`:259-282`); `slice(0,5)` (`:258`); click launches immediately (`:267` → `App.tsx:348-354`) with **no confirm, no cwd/worktree choice, no quota signal**; failure silent (no catch in `App.tsx:348-354`) | Rows show headroom forecast + usual-account star; click → inline confirm popover (profile, dir, worktree toggle) → Launch; errors banner; list scrolls past 5; telemetry `project_launch{profile,usual?}` |
| PP-06 | Launch empty (`:286-290`) | Text pointing "top right" | Button "Add account" opening wizard in place |
| PP-07 | Session history rows (`:302-323`) | Button rows: dot, label, age, chip; unbounded list, no grouping/search | Group Today/Yesterday/Earlier; per-row Resume on ended; HandedOver rows show lineage link ↔ (SPEC `:302`); search filter at 10+ |
| PP-08 | History empty (`:298-300`) | "Nothing recorded yet." | Keep; if `history_sessions>0` from transcripts but no rows, explain backfill state instead |
| PP-09 | Render-time fetch (`:209-212`) | `if (profiles===null) void profilesList()` during render — setState-in-render risk (FCUR F2.8) | `useEffect` + skeleton rows; same fix for dead modal's twin pattern (`:366-368`) before deletion |
| PP-10 | Missing project actions | No rename, pin, archive project, open-in-editor, copy path | Header `…` menu: Pin, Rename…, Open in editor, Copy path, Archive project (guarded; sessions keep history) |

Worktree gap (FCUR F1.1): no branch/dirty indicator exists
anywhere in this panel. Every competitor with parallel sessions
— T3 Code ([t3.codes](https://t3.codes/)), Conductor
([conductor.build](https://www.conductor.build/docs/concepts/workflow)),
Claude Squad ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad))
— isolates per worktree. PP-05's confirm popover is where
"New worktree `juggler/<profile>/<short>`" belongs.

## 7. Session lifecycle state machine — as the UI sees it

Engine truth: 9 states (`crates/juggler-core/src/model.rs:188-207`,
wire names `:230-238`). UI truth: `statusTone` knows 5
(`WorkPanels.tsx:16-31`); rail rows print the raw string
(`WorkflowRail.tsx:433-435`). Four states are invisible or lying.

```
                 launch                output stall / perm prompt
  (none) ──────► Starting ──► Running ──────────────► WaitingUser
                   │            │  ▲                        │
                   │            │  └──── user replies ──────┘
                   │            ├──► Stuck? ──► Running (recovers)
                   │            ├──► QuotaBlocked ──► HandedOver (wizard)
                   │            │         └──────► queued resume @ reset
                   │            ├──► Finished ──► Resume ──► Starting
                   │            ├──► Failed ────► Resume/Relaunch/Hand over
                   └────────────┴──► Stopped ──► Resume ──► Starting
                        Stop (SP-04)      HandedOver ──► lineage jump ↔
```

| State (wire) | Rail today | Panel today | Target chip / primary action |
|---|---|---|---|
| `starting` (`model.rs:230`) | raw text | accent "starting" (`WorkPanels.tsx:20-21`) | accent, spinner; action: Stop |
| `running` (`:231`) | raw text | live "running" (`:18-19`) | live dot+text; Stop (`⌘.`) |
| `waiting_user` (`:232`) | raw text, neutral dot | neutral fallback (`:28-29`) | **warning "Needs you"**, elapsed-waiting timer; action: Focus terminal; inbox row (FCUR WOW-a) |
| `stuck` (`:233`) | raw text | neutral fallback | **warning "Stuck?"** (SPEC `:221` heuristic honesty); actions: Focus / Stop / Hand over |
| `quota_blocked` (`:234`) | raw text | neutral fallback | **danger "Quota blocked"** + reset countdown; actions: Hand over… / Queue resume at reset |
| `finished` (`:235`) | raw text | success (`:22-23`) | success; **Resume** primary (SP-11) |
| `stopped` (`:236`) | raw text | neutral (`:24-25`) | neutral; Resume primary |
| `failed` (`:237`) | red dot, raw text | danger (`:26-27`) | danger + recovery card (SP-12) |
| `handed_over` (`:238`) | raw text | neutral fallback | accent "Handed over →"; lineage jump both ways (SPEC `:302`) |

Transitions the UI must *show* but currently cannot: Running→
WaitingUser (the daily #1 question, FCUR F2.5), QuotaBlocked→
queued-resume, Finished→Resume. Until the supervisor emits them,
chips are design-only — wire emission first (FCUR F5.2).

## 8. Terminal UX (`TerminalView.tsx`, 116 LOC) — deep dive

What exists: xterm 6 + FitAddon only (`ui/package.json:18-19`;
`TerminalView.tsx:2-3,51-52`), scrollback 5,000 (`:14,39`),
snapshot replay on mount (`:57-61`), live output (`:109-113`),
keystrokes out (`:62-64`), ResizeObserver fit+PTY resize
(`:79-91`), autofocus once (`:93-100`), dispose on unmount
(`:102-106`). Reopening a panel loses nothing (`:24-28`) — the
strongest property here; protect it.

| Capability | Today (evidence) | Gap severity | Target |
|---|---|---|---|
| Search in scrollback | No SearchAddon; no `⌘F` | P1 — debugging agent output without find is the daily pain | `@xterm/addon-search`; `⌘F` bar: input, prev/next (`⌘G`/`⌘⇧G`), match count, highlight-all toggle; telemetry `term_search` |
| Clickable links/paths | No WebLinksAddon; URLs/paths are dead text | P1 | `@xterm/addon-web-links` (http→browser, confirm external) + path regex → Reveal/open-in-editor; hover underline |
| Copy / paste | Selection exists in xterm core; no Copy button, no "copy on select" pref, paste = raw keystrokes | P1 | Toolbar Copy (enabled iff selection), `⌘C` copies when selection else SIGINT (must document + test), bracketed-paste preserved; copy-toast |
| Resize | FitAddon + ResizeObserver (`:79-91`); errors swallowed (`:87-89`) | OK; add cols×rows readout on resize (transient), `⌘+/-` font size persisted per SPEC `:353` | P2 polish |
| Reconnect | Snapshot+attach on mount (`:58-61`); **no detach/ended/dead state** — a killed PTY leaves a frozen, focusable terminal | P0-adjacent: silent death in a supervision tool | Connection pill (Live / Ended / Reconnecting); ended → banner "Session ended — Resume / Relaunch / view scrollback"; output gap → auto re-attach with backoff; `term_reconnect` |
| Scrollback | 5,000 lines client (`:14`); backend ring is source of truth; no indicator, no export | P2 | "Scrollback: last 5,000 lines" caption when truncated; Export transcript… (redacted, F1.6); jump-to-bottom button when scrolled up + "N new lines" pill |
| Focus mgmt | `focus()` once on mount (`:96`); unmount disposes (`:104`) — switching panels remounts and replays (correct but loses cursor/scroll pos) | P2 | Preserve scroll offset across remount via snapshot length; focus ring visible when terminal focused (`:focus-within` on card) |
| A11y | xterm canvas; no `aria-label` on host (`:115`) | P1 | `role=log aria-label="Terminal: <title>"`; font-size pref doubles as zoom; honour reduced-motion for cursor blink |
| Preview mode | Static fake transcript (`:66-76`) | OK | Keep; label stays ("preview — run the app…") |

Reference points: Opcode's checkpoint timeline
([github.com/winfunc/opcode](https://github.com/winfunc/opcode))
and Vibe Kanban's per-workspace terminal+preview
([github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban))
both ship search/copy chrome around the terminal; a bare xterm
is below table stakes for a terminal-first product.

## 9. New session, Profiles, Onboarding, ReadyHome

### 9.1 NewSession modal (`screens/NewSession.tsx:18-227`)

Two steps in one scroll: Where (recents ×6 + picker, `:117-163`)
then Who (profile rows, `:169-209`); footer status + Close
(`:214-223`); `Esc` via wrapper keydown (`:79`).

| ID | Control | Today | Target |
|---|---|---|---|
| NS-01 | Recent project cards (`:118-145`) | Click sets dir (`:122`); selected ring (`:125-127`); shows sessions+accounts (`:139-143`) | Add branch + dirty dot; usual-account hint; keyboard ↑/↓ + typeahead |
| NS-02 | Choose folder… (`:146-162`) | Native picker (`:148-151`); drag state visual (`:153-157`); in-webview drop is a no-op by design (`:105-111`) | Keep; picker failure → inline error (currently silent); recent-missing-dir → prune affordance |
| NS-03 | Profile rows = launch buttons (`:170-200`) | Click **launches immediately** (`:178`); disabled unless `logged_in` (`:171-177`); busy disables all (`:177`) | Split select vs. launch: row selects, footer **Start session** (`↵`) confirms with dir+worktree summary; per-row spinner only on launching row; quota forecast per row (WOW-b) |
| NS-04 | Profiles loading/empty (`:201-208`) | "Loading…" text / add-account prose | Skeleton rows; empty gets "Add account" button (opens wizard, closes modal on success) |
| NS-05 | Error line (`:211`) | Inline danger text — good pattern, reuse app-wide | Keep; add Retry on launch failure |
| NS-06 | Close (`:220-222`) + `Esc` (`:79`) | No focus trap/return-focus; no `role=dialog` | Dialog semantics + trap + return focus (FCUR UX.8 P1); `⌘↵` start, `Esc` cancel |
| NS-07 | Directory chip in header (`:88-95`) | Shows basename only | Full path tooltip + × to clear dir |

### 9.2 Profiles / Accounts (`screens/Profiles.tsx`)

Header + Add account (`:81-93`); per-account card: login chip,
Log in, + Profile (`:195-217`); profile rows: model chip, cwd,
hover-revealed Delete + Launch (`:225-256`).

| ID | Control | Today | Target |
|---|---|---|---|
| PF-01 | Add account (`:89-92`) | Opens wizard (`:123-129`) | Keep; telemetry `account_add_start` |
| PF-02 | Login chip (`:205-207`) | success/warning + breathing when logged out | Add expiry/tier when known (F1.4); "unknown" third state, never guess |
| PF-03 | Log in (`:208-212`) → LoginModal (`:338-411`) | Modal: command box (`:378-380`), Sign in inside (`:382-385`), Open claude.ai (`:386-388`), Copy command (`:389-399`), Re-check (`:400-402`) | Strong flow — keep all four; Sign-in failure uses `window.alert` (`:367`) → banner; Re-check gets spinner + result inline ("still logged out") instead of silent refresh |
| PF-04 | + Profile (`:213-215`) → NewProfileModal (`:278-336`) | Fields: name (`:315-317`, autofocus, required `:294-297`), model free-text (`:318-320`), cwd free-text (`:321-323`); Create disabled while busy (`:329`) | Model becomes combobox (known models + custom); cwd gets folder-picker button (typed paths contradict NS philosophy); validation inline per-field, not one error line (`:324`) |
| PF-05 | Delete profile (trash, `:239-248`) | **Icon-only, hover-revealed (`:238`), no confirm, no undo**; failure unhandled (`Profiles.tsx:111-114` awaits then refreshes) | Confirm popover ("Delete profile X? Sessions keep their history") + Undo toast 10s; always visible on focus-within for keyboard users; `profile_delete/undo` |
| PF-06 | Launch (`:249-252`) | Launches with `null` cwd (`:60`); error → screen banner (`:95-99`, good) | Show cwd that *will* be used (profile default) in tooltip; launching row spinner |
| PF-07 | Screen loading (`:75-77`) | Bare "Loading…" | Skeleton account cards |
| PF-08 | Missing account actions | No rename account, delete account, re-import, set-default | Card `…` menu: Rename…, Delete account… (danger, typed-confirm? No — confirm + Undo; managed dir kept 30d note), Set as default |

### 9.3 Onboarding wizard (`screens/OnboardingWizard.tsx:40-436`)

Best-built surface (FCUR F2.6); audit is protective. Steps:
detect (`:141-215`) → identity (`:218-268`) → login
(`:271-339`) → summary (`:342-381`); progress pips (`:118-128`);
footer Back/primary per step (`:385-432`).

| ID | Control | Today | Target (freeze interaction model; fix only these) |
|---|---|---|---|
| OB-01 | Close ✕ (`:130-136`) | Unlabelled text glyph; no `aria-label` | `aria-label="Close"` + icon; confirm if mid-import busy |
| OB-02 | Scan rows (`:166-195`) | Whole-row button → identity; login chip; first note only (`:189`) | Expandable "details" for all notes; bytes already shown (`:190-192`) |
| OB-03 | Fresh-account link (`:199-213`) | Underline text button | Keep; make it a proper secondary button for discoverability |
| OB-04 | Label input (`:233-242`) | Autofocus; Create disabled until non-empty (`:402`) | Inline hint when empty on submit attempt; max-length + duplicate-label warning |
| OB-05 | Plan tiles ×4 (`:247-262`) | Radio-like buttons, no `radiogroup` role | `role=radiogroup/radio` + arrow keys; keep visuals |
| OB-06 | Import/Create primary (`:398-406`) | Busy label "Setting up…" (`:405`); error inline (`:266,397`) | Progress detail for import (files/bytes live) — import can be slow at 2 GiB ceiling (SPEC `:422`) |
| OB-07 | Login actions ×4 (`:299-328`) | Mirrors PF-03; no try/catch on spawn (`:303-306`) — failure = unhandled rejection | Catch → inline error + Retry; Re-check result line (cf. PF-03) |
| OB-08 | "I'll sign in later" (`:408-418`) | Primary-styled escape hatch | Demote to secondary; primary becomes Continue only when `logged_in` |
| OB-09 | Summary (`:342-381`) | Checkmarks, files/bytes (`:353-359`), untouched-original proof (`:362-369`) | Add "Start a session now" primary next-step; Done demoted |
| OB-10 | Back buttons (`:386-395`) | Present on identity/login only | Keep; `Esc` = Back-or-Close consistently |

### 9.4 ReadyHome (`screens/ReadyHome.tsx:11-93`)

Gate: shown when any account logged out and not skipped
(`App.tsx:194-198`); per-account Sign in (`:72-76`); Skip link
(`:82-90`); skip is `sessionStorage`-only (`App.tsx:77-79,387-390`)
so it re-blocks every launch (FCUR F2.7).

| ID | Control | Target |
|---|---|---|
| RH-01 | Sign in per account (`ReadyHome.tsx:72-76`) | Keep; after terminal ends, auto re-check + flip row to ready without navigation |
| RH-02 | Skip (`:83-89`) | Per-profile sticky "don't ask again" (localStorage per account id); global skip stays session-scoped |
| RH-03 | Progress chip (`:38-40`) | Keep; add per-account last-checked time |

## 10. Cross-cutting state & interaction contract

Shared primitives: `Button` variants primary/secondary/ghost/
danger with `disabled:opacity-40 pointer-events-none`
(`ui/src/components/primitives.tsx:11-48`), `Chip` 6 tones
(`:67-113`), `QuotaRing` (`:164-218`, unused with real data),
`Field/Input/Select/Modal` (`ui/src/components/form.tsx:8-97`).
Icons: 8 glyphs only (`ui/src/components/icons.tsx:22-112`) —
no search, copy, stop, warning, or chevron icons exist yet.

| Dimension | Today (evidence) | Contract for target |
|---|---|---|
| Hover | `hover:bg-surface` rows; Button brightness (`primitives.tsx:16-23`) | 100 ms ease; every interactive row/button has one; icon-buttons get tooltip after 400 ms |
| Focus | `:focus-visible` global (`styles.css:76`); rail rings (`WorkflowRail.tsx:138`) | Visible 1px accent ring everywhere incl. modals, terminal card `:focus-within`; never `outline:none` without replacement |
| Disabled | Opacity 40–50% + reason via `title` or chip (e.g. `WorkPanels.tsx:158-162,266`) | Disabled **with adjacent reason text**, never title-only; disabled buttons stay focusable (`aria-disabled`) when reason matters |
| Loading | Text "Loading…" (`Profiles.tsx:76`, `NewSession.tsx:207`); busy flags disable groups | Skeletons for lists; spinner inside the acting button only; other actions stay live |
| Error | Mixed: inline (good, `NewSession.tsx:211`), banner (`Profiles.tsx:95-99`), `window.alert` ×3 (`App.tsx:371,384`; `Profiles.tsx:367`), silent (`App.tsx:113-115`) | One system: field-inline → banner (screen) → toast (transient); **zero `window.alert`**; every error has Retry or next step |
| Empty | Prose-only in 6 places (WR-10, SP-09, PP-06/08) | Every empty state = icon + one sentence + one CTA button |
| Destructive guard | **None anywhere**: Delete profile (PF-05), Stop (SP-04), handover-spawn (SP-08) all fire on one click | Tiered: recoverable → no confirm + Undo toast (Stop→Resume, Archive); costly → confirm popover (Delete profile, Delete session, Delete account); irreversible → confirm with consequence line |
| Undo | **Zero instances** in `ui/src` | Toast Undo 10 s for Stop/Delete/Archive; lineage jump is the handover "undo" (source never destroyed, SPEC `:287`) |
| Telemetry | **Zero events** in `ui/src` (SPEC `:23` opt-in only) | Opt-in, local-first event log; names in this doc (`session_stop`, `handover_*`, `term_*`…); no content, no paths — ids + enums only |
| Shortcuts | `⌘B`, `⌘N` (`App.tsx:206-219`) | Add: `⌘K` palette, `⌘F` term search, `⌘.` stop, `⌘R` resume, `⌘⇧H` handover, `⌘1/2/3` tabs, `⌘↑` project, `Esc` layered close; discoverability via `?` overlay |
| Motion/contrast | Breathe/glow animations, no reduced-motion (`styles.css:90-104` per FCUR UX.8); faint `#5b6474` (`styles.css:24`) at 10.5px likely <AA | Honour `prefers-reduced-motion`; status never colour-only (chip text, §7); faint text ≥11px or lighter token |

cc-switch's tray row (`name · mode · provider · quota`, one-click
switch — [github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch))
and claude-swap's live dashboard
([github.com/realiti4/claude-swap](https://github.com/realiti4/claude-swap))
set the glanceability bar the rail footer (WR-09) must meet:
account + quota + needs-me count in one line, everywhere.

## 11. ASCII wireframes — redesigned panels

### 11.1 Project panel (target for §6)

```
┌────────────────────────────────────────────────────────────────┐
│ ▸ my-app        main ⎇ · 3 dirty        [Pin] [⋯]  ⌘↑ rail   │
│   ~/Dev/my-app  ⧉ copy  ◉ Reveal                    [idle ▾]  │
├─────────────────────────────┬──────────────────────────────────┤
│ ON IT NOW                   │ SESSIONS IN THIS PROJECT  ⌕ ___  │
│ ┌─────────────────────────┐ │ Today                            │
│ │ ● Work Max · Main  42m  │ │ ● Fix auth race      running 42m │
│ │   running · 18k tok     │ │ ◐ Review PR #12    needs you 12m │
│ │        [Open ⌘⏎] [Stop] │ │ Yesterday                        │
│ └─────────────────────────┘ │ ○ Refactor store   handed → B    │
│ LAUNCH WITH…        headroom│ ○ Spike search     finished      │
│ ★ Work Max · Main  ████ 80% │   [Resume] rows on hover/focus   │
│   Personal · Main  ██░░ 35% │ ──────────────────────────────── │
│   + worktree ☐  [Launch ⏎]  │ 12 sessions · [Archive ended…]   │
└─────────────────────────────┴──────────────────────────────────┘
  ★ = usual account · bars = quota forecast (Measured) · ☐ default off
```

### 11.2 Instance/Session panel (target for §5; merges Instances per F2.2)

```
┌────────────────────────────────────────────────────────────────┐
│ ● Fix auth race   [running] [terminal]  my-app /  [Stop ⌘.]  │
│   Work Max · Main · claude-code · 42m · 18.2k tok · ~$0.41 Est │
│   ⎇ main · ~/Dev/my-app ⧉ · sess 9f3…  [Hand over… ⌘⇧H]      │
├────────────────────────────────────────────────────────────────┤
│ Terminal ──────────────── [⌕ Search ⌘F][⧉ Copy][± font] ●Live │
│ ┌────────────────────────────────────────────────────────────┐ │
│ │ $ claude …agent output… (scrollback 5,000)                 │ │
│ │ ▊                                                          │ │
│ └────────────────────────────────────────────────────────────┘ │
│ [↓ 24 new lines]                                  ended? →     │
│ ┌ Ended: [Resume ⌘R primary][Relaunch][Hand over…][Export…] ┐ │
│ └ Crash:  exit 137 · [Resume][Relaunch][Hand over…]         ┘ │
├────────────────────────────────────────────────────────────────┤
│ Lineage: ← sess-41 (Personal) · Handed over → sess-52 (Max)  │
└────────────────────────────────────────────────────────────────┘
```

### 11.3 Handover wizard (replaces SP-08 chips; SPEC `:287`)

```
┌─ Hand over “Fix auth race” ──────────────────────────── ✕ ─┐
│ ① Target        ② Fidelity        ③ Confirm                 │
│ ○ Work Max · Main    ██████ 80% left · resets 17:00 ★usual │
│ ● Personal · Main    ██░░░░ 35% left · ~40m at current burn │
│ ○ Side · Main        needs sign-in (disabled, reason shown) │
│ Carries: transcript + subagents ✓ · todos ✓                 │
│ Does not carry: running processes, MCP live state           │
│ Tier: T2 transplant (same harness)  [▸ preview pack]        │
│                          [Back]        [Hand over →]        │
└──────────────────────────────────────────────────────────────┘
  After: both panels show lineage ↔; source stays resumable.
```

## 12. Prioritised build order (this surface only)

| Pri | Item (IDs) | Why / unblocks |
|---|---|---|
| P0 | Terminal ended/reconnect state (§8); app error model, kill `window.alert` (SH-05/06/07) | A supervision tool must never fail silently |
| P0 | Ended-session Resume/Relaunch + recovery card (SP-11/12) | SPEC `:348`; cheapest continuity win (T1 already works) |
| P1 | 9-state chips + Waiting/Stuck/QuotaBlocked wiring (§7, SP-01, WR-05) | Answers "which session needs me?" (FCUR WOW-a) |
| P1 | Handover wizard with tier+headroom (SP-08, §11.3) | SPEC `:287`; stops one-click billed spawns |
| P1 | Terminal search + links + copy (§8) | Daily debugging table stakes |
| P1 | Delete guards + Undo toasts (PF-05, SP-04); dialog/tab/splitter a11y (NS-06, WR-01/08) | Trust + keyboard operability |
| P1 | Honest Overview rings (SH-11); merge Instances tab (WR-01/04) | No fabricated data; IA truth |
| P2 | Worktree-on-launch + branch/dirty (PP-05, §11.1); ⌘K palette; session search/grouping (WR-05, PP-07) | Parallel-safety + scale past 15 sessions |
| P2 | Delete dead `NewSessionModal` (`WorkPanels.tsx:355-443`); fix render-fetch (PP-09) | Daily surface stays boring (FCUR F2.8) |

## Appendix — unresolved / not verified

- No runtime, browser, or screen-reader pass: focus-trap,
  contrast (`styles.css:24` vs. AA), and xterm remount scroll
  loss are code-read inferences, matching FCUR Appendix A.
- Whether Stop needs a confirm for sessions with unsent terminal
  input (SP-04) needs a supervisor signal (input-buffer state)
  that does not exist in `ui/src/lib/bridge.ts` today.
- Desktop-kind Focus/Attach (SP-02/06) cannot be specced beyond
  disabled states until the desktop adapter exists (SPEC `:336`).
- Quota forecast UI (PP-05, SP-08, §11) is design-ready but
  blocked on the M3 statusline feed (SPEC `:192`); ship the
  wizard with "headroom unknown" honesty state first.
- Telemetry event names assume the opt-in local log (SPEC `:23`);
  no analytics backend exists to validate against.


