# 01 — UI Shell & Rail: Button-by-Button Audit (Deep)

> Scope: every interactive element in `ui/src` at audit time.
> Method: full read of all 27 files under `ui/src` (see §1),
> cross-checked against `docs/review/full-competitive-ux-review.md`
> and `docs/review/agent-review-prompt.md`. No runtime/browser
> test was run; a11y claims are code-read only.
> Verdict key: **Keep** = ship as-is · **Fix** = keep, change spec
> below · **Remove** = delete · **Add** = missing, create.
> Backend column names the Tauri command via `ui/src/lib/bridge.ts`;
> `—` means purely local React state. Line refs are `file:line`.

## How to read each entry

Each `B-nn` entry compresses nine fields in fixed order:
`Loc · Label → Action · Backend · Success · Error ·
Disabled/Loading/Empty · A11y · Verdict → Redesign`.
Where a field is `none` that absence *is* the finding.
Prior-review anchors (F2.1 etc.) point to
[full-competitive-ux-review.md](../full-competitive-ux-review.md#L1).

## 0. Executive verdict (read this first)

1. The shell has exactly two real state-changing verbs from
   chrome — **New** and **Stop/Handover** — and both fail
   silently or modally: `refresh()` swallows errors
   ([App.tsx](../../../ui/src/App.tsx#L101))
   and handover/sign-in use `window.alert`
   ([App.tsx](../../../ui/src/App.tsx#L371),
   [App.tsx](../../../ui/src/App.tsx#L384)).
   A supervision tool whose failures are invisible fails F2.1.
2. The rail promises three entities and delivers two:
   Instances is Sessions filtered live/recent through the same
   `SessionRow` ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L295)),
   contradicting the SPEC distinction cited in F2.2. Users
   cannot tell what an Instance *is* from any button label.
3. There is no undo, no confirm, and no toast anywhere.
   Destructive **Stop** and **Delete profile** fire on one
   click ([WorkPanels.tsx](../../../ui/src/screens/WorkPanels.tsx#L100),
   [Profiles.tsx](../../../ui/src/screens/Profiles.tsx#L239));
   every competitor with a review loop — Vibe Kanban's
   inline-comment-to-agent, Conductor's Checks tab —
   gates destructive steps. See §9 for the competitor bar.
4. Keyboard story is two shortcuts (`⌘B`, `⌘N`,
   [App.tsx](../../../ui/src/App.tsx#L205))
   plus Escape-by-wrapper; tabs, splitter, and modals do not
   implement their ARIA patterns (§8). `⌘K` does not exist.
5. Two of four global-nav destinations are dead ends
   (`ComingSoon`, [App.tsx](../../../ui/src/App.tsx#L525))
   and Overview fabricates numbers (`percent={0}`,
   [App.tsx](../../../ui/src/App.tsx#L510)).
   First-run trust is spent in the top-right corner.

## 1. File inventory — everything inspected

All 27 files under `ui/src` were read in full:

| Group | Files (lines) |
|---|---|
| Shell | `App.tsx` (539), `main.tsx` (10), `styles.css` (156) |
| Components | `WorkflowRail.tsx` (444), `primitives.tsx` (218), `TerminalView.tsx` (116), `form.tsx` (97), `icons.tsx` (112) |
| Screens | `WorkPanels.tsx` (443), `OnboardingWizard.tsx` (436), `Profiles.tsx` (411), `NewSession.tsx` (229), `ReadyHome.tsx` (93) |
| Bridge/types | `lib/bridge.ts` (495) + 12 generated types in `types/generated/` (no UI logic; verified by listing) |

Notes on inventory honesty:

- `main.tsx:6-10` only mounts `<App/>` in `StrictMode`; it owns
  no button, but it means a double-render in dev can double-fire
  the mount effects in `App.tsx:87-145` — relevant to B-03.
- `icons.tsx:22-112` exports 8 icons; `IconDashboard` (:22) is
  never imported anywhere (grep-verified) — dead export, §7.
- `styles.css:76-79` gives a global `:focus-visible` ring —
  the single strongest a11y asset in the codebase. Keep it.
- Generated types carry no behaviour; they are cited only
  where a missing field (e.g. no `quota` on `AccountInfo`)
  explains a missing button state.

## 2. Screen inventory

| Screen | Entry condition | Exit paths | Dead end? |
|---|---|---|---|
| WorkEmpty | No selection, no global, ReadyHome passed ([App.tsx:393](../../../ui/src/App.tsx#L393)) | New session (B-31) | No |
| ReadyHome | `global===null` + any account not `logged_in` + not skipped ([App.tsx:194](../../../ui/src/App.tsx#L194)) | Sign in (B-20), Skip (B-21) | No, but re-blocks every launch until session-skip (F2.7) |
| Overview | Global nav Overview ([App.tsx:334](../../../ui/src/App.tsx#L334)) | None — live rows are not clickable (B-14) | **Yes** |
| Accounts (Profiles) | Global nav Accounts ([App.tsx:330](../../../ui/src/App.tsx#L330)) | Launch/Login/Profile modals | No |
| Usage | Global nav Usage ([App.tsx:338](../../../ui/src/App.tsx#L338)) | None | **Yes — ComingSoon** |
| Fleet | Global nav Fleet ([App.tsx:340](../../../ui/src/App.tsx#L340)) | None | **Yes — ComingSoon** |
| ProjectPanel | Rail project click ([App.tsx:342](../../../ui/src/App.tsx#L342)) | Open session, Launch-with | No |
| SessionPanel | Rail session/login click ([App.tsx:356](../../../ui/src/App.tsx#L356)) | Stop, Handover, Open project | Partial — no Resume (F2.3) |
| OnboardingWizard | Zero accounts auto-open ([App.tsx:94](../../../ui/src/App.tsx#L94)) or Add account | Close (B-40), Done | No |
| NewSessionModal (live) | `⌘N` / New buttons / folder drop ([App.tsx:409](../../../ui/src/App.tsx#L409)) | Profile click launches, Close | No |
| NewSessionModal (dead) | Never rendered — duplicate in ([WorkPanels.tsx:355](../../../ui/src/screens/WorkPanels.tsx#L355)) | — | **Yes — dead code (F2.8)** |

Priority consequence: 3 of 11 surfaces cannot move the user
forward. Prior review F2.4 already flags Usage/Fleet; this
audit adds Overview's non-clickable live list (B-14).

## 3. Global chrome — App shell (`App.tsx`)

### B-01 · Rail brand button (collapse toggle)
- Loc: [App.tsx:259](../../../ui/src/App.tsx#L259) · Label: Juggler icon, `title=Expand/Collapse (⌘B)` → toggles `railCollapsed`, persists `aij.rail.collapsed` ([App.tsx:61](../../../ui/src/App.tsx#L61)).
- Backend: — · Success: rail animates 60px↔width · Error: none possible · Disabled: never.
- A11y: native `<button>`, tooltip via `title` only; no `aria-expanded`, no `aria-controls`, icon is decorative but unlabelled when expanded (text `AI Juggler` sits beside it, :269).
- **Verdict: Fix.** Redesign: keep position; add `aria-expanded={ !collapsed }`, `aria-controls="workflow-rail"`, tooltip component (not `title`) reading `Collapse rail · ⌘B`. Shortcut stays `⌘B`. No confirm/undo needed.

### B-02 · Global nav ×4 (Overview / Accounts / Usage / Fleet)
- Loc: [App.tsx:306](../../../ui/src/App.tsx#L306), data at [App.tsx:44](../../../ui/src/App.tsx#L44) · Label = item label → `setGlobal(id)`, unmounts detail panel.
- Backend: — (Overview then calls `dashboard_stats`, B-14) · Success: active pill `bg-surface sheen` · Error: none · Disabled: never, even for empty Usage/Fleet.
- A11y: plain `<button>`s in a `<div>`; no `aria-current`, no `nav` landmark, no arrow-key roving; focus ring from global CSS only.
- **Verdict: Fix (Overview/Accounts), Remove (Usage/Fleet until real).** Redesign: wrap in `<nav aria-label="Global">`, add `aria-current="page"` on active; **remove Usage and Fleet from nav** and re-add with badge `M3`/`M6` disabled + tooltip `Usage arrives in M3 — quota first` only when the milestone ships a read model. Exact new labels: `Overview`, `Accounts`. Do not keep dead destinations for symmetry — Claude Squad and T3 both keep nav to live surfaces only ([t3.codes](https://t3.codes/), prior §T3).

### B-03 · Header “New” (primary)
- Loc: [App.tsx:321](../../../ui/src/App.tsx#L321) · Label: `New` + terminal icon → `setNewSession(true)` (same as `⌘N`).
- Backend: — until profile click (`launch_session` via bridge:289) · Success: modal opens · Error: none at this step · Disabled: never — opens even with zero accounts/profiles, deferring failure into the modal (B-24 empty state).
- A11y: uses shared `Button` ([primitives.tsx:31](../../../ui/src/components/primitives.tsx#L31)); no shortcut hint in label/tooltip; `data-tauri-drag-region={false}` wrapper (:305) is correct.
- **Verdict: Fix.** Redesign: label `＋ New session`, tooltip `New session · ⌘N`; when `profiles.length===0`, click routes to Accounts/Add-account instead of an empty modal, with inline hint. Keep `⌘N`.

### B-04 · Keyboard: ⌘B / ⌘N (window listener)
- Loc: [App.tsx:206](../../../ui/src/App.tsx#L206) · Action: `⌘/Ctrl+B` toggle rail, `⌘/Ctrl+N` new session; listener added once but `toggleRail` dep omitted (stale-closure risk, deps `[]` at :219).
- Backend: — · Success: as B-01/B-03 · Error: none · Disabled: fires even while typing in inputs/modals — `⌘N` inside NewProfile name field still opens NewSession.
- A11y: undiscoverable (only in `title` attrs); no `aria-keyshortcuts` anywhere in repo (grep-verified).
- **Verdict: Fix.** Redesign: guard `event.target` editable check; add `aria-keyshortcuts="Meta+N"` / `"Meta+B"` on B-03/B-01; add `⌘K` palette (Add, §10), `⌘1/2/3` rail tabs, `Esc` closes topmost layer only. Document in `?` shortcuts popover (Add).

### B-05 · Rail footer status (this-mac · healthy / dot)
- Loc: [App.tsx:276](../../../ui/src/App.tsx#L276) · Label: green dot + `this-mac · healthy` + `v{version}` → **not a button**; `title="this-mac · healthy"` when collapsed.
- Backend: version from `app_info` (bridge:136) · Success/Error: static text — `healthy` is hardcoded, never derived from any probe.
- A11y: colour-only health signal; collapsed state is a dot with a `title`.
- **Verdict: Fix.** Redesign: make it a button `Machine: OK · v0.x` opening Overview/Machine; health computed from supervisor probe; text label always paired with dot (never colour-alone, UX.8). If probe absent, render `Machine: unknown` — honest empty state, never fabricated `healthy` (F2.4).

### B-06 · Context title + “Claude Code” chip (non-interactive)
- Loc: [App.tsx:302](../../../ui/src/App.tsx#L302) · Shows `contextTitle` + static `Chip Claude Code`. No action.
- **Verdict: Fix.** Redesign: chip becomes harness switcher (disabled, tooltip `Codex arrives in Phase 2`) only when a second adapter registers; until then keep static but add `aria-label="Harness: Claude Code (only harness)"`. Prevents the multi-harness promise reading as shipped (F5.5).

### B-07 · Drag region header
- Loc: [App.tsx:298](../../../ui/src/App.tsx#L298) `data-tauri-drag-region`. Double-click/maximise behaviour is OS-default; no custom button.
- **Verdict: Keep.** Redesign: none, except ensure B-02/B-03 keep `data-tauri-drag-region={false}` (:305) after any refactor — regression test: buttons clickable at window top edge.

## 4. WorkflowRail — tabs, lists, resize (`WorkflowRail.tsx`)

### B-08 · Rail tabs ×3 (Projects / Instances / Sessions)
- Loc: [WorkflowRail.tsx:126](../../../ui/src/components/WorkflowRail.tsx#L126), data [WorkflowRail.tsx:8](../../../ui/src/components/WorkflowRail.tsx#L8) · Label = tab label+icon → `onTab(id)`; list below swaps.
- Backend: — · Success: accent pill + list swap · Error: none · Disabled: never; counts/badges absent — no live/needs-attention count on any tab.
- A11y: `role=tablist/tab` + `aria-selected` (:124,:132) but no `aria-controls`, no `tabpanel`, no Arrow/Home/End keys; labels vanish below 244px (icon-only + `title`, :106).
- **Verdict: Fix + Remove (Instances tab).** Redesign: two tabs `Projects` / `Sessions` until Instances gains resources (F2.2); add leading `Inbox` tab with count badge (Add, §10). Implement full tabs pattern: `aria-controls`, arrow keys, `aria-keyshortcuts="Meta+1..3"`. Tooltip on icon-only: `Sessions · ⌘3`.

### B-09 · Rail “New session” (bottom CTA)
- Loc: [WorkflowRail.tsx:200](../../../ui/src/components/WorkflowRail.tsx#L200) · Label `New session` (icon-only collapsed, `title=New session (⌘N)`) → `onNewSession`.
- Backend: — · Success: modal · Error: none · Disabled: never (same zero-profile deferral as B-03).
- A11y: `focus-visible:ring` present (:204); shortcut only in collapsed `title`.
- **Verdict: Keep + Fix.** This duplicates B-03; keep rail as primary (daily loop lives here), demote header New to `⌘N` hint only or remove header duplicate. Add `aria-keyshortcuts`. Behaviour otherwise identical to B-03 redesign.

### B-10 · Project row (rail list)
- Loc: [WorkflowRail.tsx:259](../../../ui/src/components/WorkflowRail.tsx#L259) · Label: project name + `N accts · M runs` + live dot → `onSelectProject(id)`.
- Backend: — (data from `projects_list`, bridge:300) · Success: `bg-surface` + accent bar · Error: none · Empty: text `Launch a session…` (:249), not actionable.
- A11y: native button, focus ring; status is dot-only; truncated name has no tooltip (`title` absent).
- **Verdict: Fix.** Redesign: add `title={name + ' · ' + root}`, right-side text chip for live state (`● Live` / `idle`), `aria-current` when selected. Empty state becomes button `＋ Start first session · ⌘N`.

### B-11 · SessionRow (Instances + Sessions lists)
- Loc: [WorkflowRail.tsx:407](../../../ui/src/components/WorkflowRail.tsx#L407) · Label: session label + raw `status` string + `app` tag for desktop → `onSelectSession(id)`.
- Backend: — (`sessions_list`, bridge:307) · Success: selected bg · Error: `failed` = red dot only · Disabled: never.
- A11y: status conveyed by dot colour + lowercase raw status text (:434); no `Stuck/Waiting/QuotaBlocked` chips exist because backend never emits them (F2.5).
- **Verdict: Fix.** Redesign: second line becomes `Chip` with text (`Running · 12m`, `Needs you · waiting 4m`, `Quota blocked · resets 17:00`); add waiting-time sort + Needs-attention section on top. Tooltip = label + project + account.

### B-12 · Collapsed-rail live dots (icon rail)
- Loc: [WorkflowRail.tsx:168](../../../ui/src/components/WorkflowRail.tsx#L168) · Up to 6 live sessions as 8px dots, `title={label}` → select session.
- Backend: — · Success: brighter dot when selected · Error: failed sessions invisible (live filter, :165) · Empty: blank column.
- A11y: `title`-only identity; dot has no text alternative; 32px target (`size-8`) below 40px guidance.
- **Verdict: Fix.** Redesign: each dot becomes avatar button with initials + `aria-label="Open {label}, running"`; failed/needs-you get `!` badge; target `size-9` min. Keep cap 6 + `+N` overflow button (Add).

### B-13 · Resize handle (drag + double-click collapse)
- Loc: [WorkflowRail.tsx:215](../../../ui/src/components/WorkflowRail.tsx#L215) · Drag resizes 208–360px persisted `aij.rail.width` (:19); double-click → `onToggleCollapsed`.
- Backend: — · Success: accent hairline while dragging · Error: none · Disabled: hidden when collapsed.
- A11y: `role=separator` + `aria-label` (:218) but **not focusable, not keyboard-operable**; no `aria-valuenow/min/max`.
- **Verdict: Fix.** Redesign: `tabIndex=0`, `aria-valuemin=208 aria-valuemax=360 aria-valuenow={width}`, ArrowLeft/Right ±8px (Shift ±32), `Home` reset 264. Tooltip `Drag to resize · double-click to collapse`.

### B-14 · Overview live-instance rows + quota rings (display-only)
- Loc: rows [App.tsx:498](../../../ui/src/App.tsx#L498), rings [App.tsx:513](../../../ui/src/App.tsx#L513) · Rows show label/kind/status chip but are `<div>`s — **not clickable**, despite copy `Select one in the rail` (:481).
- Backend: `dashboard_stats` (bridge:321) fetched then discarded (`void stats`, [App.tsx:442](../../../ui/src/App.tsx#L442)); counts computed locally from props. Rings hardcoded `percent={0}`.
- Success/Error/Empty: empty state (:487) is honest; rings are not — `0%` reads as measured quota.
- A11y: rings are `<svg>` without `role=img`/`aria-label` ([primitives.tsx:165](../../../ui/src/components/primitives.tsx#L165)); numbers have no text equivalent.
- **Verdict: Fix.** Redesign: rows become buttons → `selectSession(id)`; rings render `— No data · M3` empty state (dashed ring, text) until Measured quota lands; add `role="img" aria-label="5 hour quota: no data yet"`. Delete `void stats` — either render stats or stop fetching.

## 5. ReadyHome — get-ready gate (`ReadyHome.tsx`)

### B-20 · Per-account “Sign in” (primary, ×N)
- Loc: [ReadyHome.tsx:72](../../../ui/src/screens/ReadyHome.tsx#L72) · Label `Sign in` → `onSignIn(account)` → `terminal_spawn_login` (bridge:379) → login terminal selected ([App.tsx:378](../../../ui/src/App.tsx#L378)).
- Success: terminal opens, rail jumps to Sessions · Error: `window.alert` at [App.tsx:384](../../../ui/src/App.tsx#L384) · Disabled: never; no busy state — double-click spawns twice (guard exists only in `openLoginTerminal` dedupe, [App.tsx:165](../../../ui/src/App.tsx#L165)).
- A11y: shared Button; plan chip + dot + text state is good (not colour-only).
- **Verdict: Keep + Fix.** Redesign: label `Sign in · {label}` for screen readers (`aria-label`), busy spinner `Opening…` disabled during spawn, error → toast with Retry (never alert). This flow beats most competitors' first-run (prior F2.6) — protect it.

### B-21 · “Skip for now — show projects” / “Continue to Projects →”
- Loc: [ReadyHome.tsx:83](../../../ui/src/screens/ReadyHome.tsx#L83) · Text button → `sessionStorage aij.ready.skipped=1` ([App.tsx:387](../../../ui/src/App.tsx#L387)).
- Backend: — · Success: gate dismissed for session · Error: none · Disabled: never.
- A11y: underline-on-hover text button; touch target ~16px tall.
- **Verdict: Fix.** Redesign: label when partial: `Skip — 1 of 3 ready`; make skip sticky per-profile (`Don't ask again for this account` checkbox) per F2.7, keep re-block only for unknown/expired. Min height 32px.

## 6. Detail panels (`WorkPanels.tsx`)

### B-22 · SessionPanel “Stop” (live only)
- Loc: [WorkPanels.tsx:100](../../../ui/src/screens/WorkPanels.tsx#L100) · Label `Stop` → dynamic `terminal_kill` (bridge:403).
- Success: `onStopped()` → `refresh()` re-renders · Error: **none** — `await` uncaught, failure silent · Disabled: only rendered when `live`; no busy state, repeat clicks re-kill.
- A11y: shared Button `secondary`; no confirm, no undo; adjacent text `ran {age} · scrollback replayed` (:96) is informative.
- **Verdict: Fix (P0 for error handling).** Redesign: label `Stop session`, confirm sheet `Stop “{label}”? Running work will end.` with `Stop` (danger, `⌘.`) / `Cancel` (Esc); busy `Stopping…`; error → toast Retry; success → toast `Stopped` + **Undo (relaunch same profile+cwd, 10s)**. Competitor bar: Claude Squad's `c` checkout explicitly pauses vs kills ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)) — copy the explicitness.

### B-23 · Handover target buttons (×≤6, “Continue on another account”)
- Loc: [WorkPanels.tsx:153](../../../ui/src/screens/WorkPanels.tsx#L153) · Label `{account_label}` (+`· sign in first`) → `sessionHandover` (bridge:464, `session_handover`).
- Success: refresh + select new session ([App.tsx:365](../../../ui/src/App.tsx#L365)) · Error: `window.alert` ([App.tsx:371](../../../ui/src/App.tsx#L371)) · Disabled: `!ready` targets disabled with `title` explanation (:158); self excluded (:146); `slice(0,6)` silently hides 7th+ profile.
- A11y: native buttons, disabled reason only in `title`; no fidelity/headroom info before committing (F2.3).
- **Verdict: Fix (replace with wizard).** Redesign: single button `Continue on another account…` opening 3-step wizard (Target ranked by headroom → Fidelity tier + carry list → Confirm). Each target row: `label · 72% headroom · resets 17:00 · usual for this project`. Overflow: searchable list, never silent slice. Error → inline step error + Retry, lineage shown after (`Handed over from …`).

### B-24 · Project chip (session header → project)
- Loc: [WorkPanels.tsx:85](../../../ui/src/screens/WorkPanels.tsx#L85) · Label: folder icon + project name → `onOpenProject(id)`.
- Backend: — · Success: panel swaps · Error: none · A11y: hover-only affordance, no `aria-label`.
- **Verdict: Keep + Fix.** Redesign: add `aria-label="Open project {name}"`, chevron affordance, tooltip with full root path.

### B-25 · ProjectPanel “On it now” row
- Loc: [WorkPanels.tsx:236](../../../ui/src/screens/WorkPanels.tsx#L236) · Whole card-row button → open active session.
- Backend: — · Success: session panel · Empty: `No live instance — free to juggle.` text (:251).
- A11y: nested `Chip` inside `<button>` (interactive-adjacent semantics muddy); `open` hint good.
- **Verdict: Keep + Fix.** Redesign: keep; add `aria-label="Open live session {label}"`; empty state gains inline `Launch ↓` anchor scrolling to B-26 list.

### B-26 · ProjectPanel “Launch with another account” rows (×≤5)
- Loc: [WorkPanels.tsx:262](../../../ui/src/screens/WorkPanels.tsx#L262) · Label: account · profile · model → `launchSession(profile.id, project.root)` via [App.tsx:348](../../../ui/src/App.tsx#L348) (`launch_session`, bridge:289).
- Success: refresh + select new session · Error: **uncaught** — `onLaunch` await has no try/catch in App or panel · Disabled: `!ready` disabled + `sign in first` chip; `slice(0,5)` hides rest.
- A11y: disabled reason in `title` only; rows are buttons (good).
- **Verdict: Fix.** Redesign: label suffix `· 72% free` when quota exists; per-row busy spinner; error toast; `Show all N accounts` expander; usual-account row pinned + badged `Usual`.

### B-27 · ProjectPanel session-history rows
- Loc: [WorkPanels.tsx:303](../../../ui/src/screens/WorkPanels.tsx#L303) · Row button (dot+label+age+status chip) → open session. No Resume/Relaunch distinction for ended sessions (F2.3).
- Backend: — · Empty: `Nothing recorded yet.` (:299).
- A11y: adequate; age via `since()` ([WorkPanels.tsx:33](../../../ui/src/screens/WorkPanels.tsx#L33)) is relative-only, no absolute tooltip.
- **Verdict: Fix + Add.** Redesign: ended rows get trailing `Resume` (same account, `⌘R`) and `Relaunch` buttons; live rows keep Open only. Add `title={absolute date}`. This is T1 resume — mechanically free via `--resume` (prior F3.5).

### B-28 · WorkEmpty “New session”
- Loc: [WorkPanels.tsx:346](../../../ui/src/screens/WorkPanels.tsx#L346) · Primary `New session` → `onNew` (same modal as B-03/B-09).
- **Verdict: Keep.** Redesign: add sub-actions `Open project…` and `⌘K Search` once palette exists; keep single primary today. Empty copy is honest — keep verbatim.

### B-29 · Dead `NewSessionModal` duplicate (WorkPanels)
- Loc: [WorkPanels.tsx:355](../../../ui/src/screens/WorkPanels.tsx#L355) (443-line file's second half) · Typed-path `<input>` (:401), profile buttons (:411), `Close` (:436). Never imported (App imports from `screens/NewSession`, [App.tsx:12](../../../ui/src/App.tsx#L12)).
- Harm: contradicts `never typed paths` daily-loop rule (agent prompt :53-54); stale `profilesList` during render (:366 pattern also in ProjectPanel :209).
- **Verdict: Remove.** Delete :354-443 entirely (F2.8). No redesign — the live modal (B-30) supersedes it.

## 7. Launch & accounts modals/screens

### B-30 · NewSession (live): recent-project tiles, Choose folder, profile rows
- Loc: tiles [NewSession.tsx:119](../../../ui/src/screens/NewSession.tsx#L119), picker [NewSession.tsx:146](../../../ui/src/screens/NewSession.tsx#L146), profiles [NewSession.tsx:174](../../../ui/src/screens/NewSession.tsx#L174), Close [NewSession.tsx:220](../../../ui/src/screens/NewSession.tsx#L220).
- Action/Backend: tile → `setDirectory(root)`; picker → `pickDirectory()` native dialog (bridge:468); profile click → `launch_session` (bridge:289). Drag-drop prefill via Tauri event ([App.tsx:125](../../../ui/src/App.tsx#L125)).
- Success: modal closes, Sessions tab + new session selected ([App.tsx:418](../../../ui/src/App.tsx#L418)) · Error: inline `text-danger` (:211) incl. preview-mode message · Disabled: `busy||!ready`; `Loading…` text (:207); recents capped 6 (:55).
- A11y: Escape via wrapper `onKeyDown` (:79) only if focus inside; no `role=dialog`, no focus trap/return; selected tile has accent border (not colour-only — good).
- **Verdict: Keep + Fix.** Redesign: `role="dialog" aria-modal="true" aria-label="New session"`, initial focus on first recent, trap + return-focus to invoker; profile row adds headroom forecast `· ~40m left` (WOW-b); footer primary becomes `Start session` enabled only when directory+ready profile chosen (today launch is the row click itself — keep row-click but add explicit footer confirm for keyboard clarity). Shortcut: `⌘↵` start.

### B-31 · Profiles: Add account / Empty-state Add account
- Loc: header [Profiles.tsx:89](../../../ui/src/screens/Profiles.tsx#L89), empty [Profiles.tsx:167](../../../ui/src/screens/Profiles.tsx#L167) → opens `OnboardingWizard` as modal (:123).
- Backend: — · Success: wizard · Error: screen-level banner (:95) · Loading: bare `Loading…` (:76).
- **Verdict: Keep + Fix.** Redesign: skeleton cards while loading; `Add account` gets `aria-haspopup="dialog"`. Empty copy is good — keep.

### B-32 · Profiles: per-profile Launch / Delete (trash) / + Profile / Log in
- Loc: Launch [Profiles.tsx:249](../../../ui/src/screens/Profiles.tsx#L249), Delete [Profiles.tsx:239](../../../ui/src/screens/Profiles.tsx#L239), `+ Profile` [Profiles.tsx:213](../../../ui/src/screens/Profiles.tsx#L213), `Log in` [Profiles.tsx:209](../../../ui/src/screens/Profiles.tsx#L209).
- Backend: Launch `launch_session` (note: success calls `openTerminal` with session_id as terminal id, :66 — conflates session/terminal ids); Delete `profile_delete` (bridge:279) **no confirm, no undo, no catch** (:111); `+ Profile` → B-33; Log in → B-34.
- Success: Launch opens terminal panel · Error: banner `setError` (:49) except Delete (silent) · Disabled: Launch never disabled by login state (can launch logged-out profile).
- A11y: Delete is icon-only with `title="Delete profile"`; hover-reveal `opacity-60→100` (:238) hides it from keyboard/touch until hover.
- **Verdict: Fix (Delete = P0 data-trust).** Redesign: Delete → confirm `Delete profile “{name}”? Sessions keep history.` + toast Undo (re-create) 10s; always-visible at `opacity-100` on focus-within; `aria-label="Delete profile {name}"`. Launch disabled when `!loggedIn` with chip `Sign in first` (mirror B-30). `+ Profile` label → `New profile`.

### B-33 · NewProfileModal: inputs + Cancel / Create profile
- Loc: modal [Profiles.tsx:278](../../../ui/src/screens/Profiles.tsx#L278), fields via `Field/Input` ([form.tsx:8](../../../ui/src/components/form.tsx#L8)), submit [Profiles.tsx:329](../../../ui/src/screens/Profiles.tsx#L329).
- Backend: `profile_create` (bridge:270) · Success: close+refresh · Error: inline (:324), empty-name guard (:294) · Disabled: `busy` on Create only; Cancel always live.
- Inputs: Name (autoFocus, placeholder `Deep work`), Model (`opus`), Default directory typed path (`~/Projects/app`) — typed path contradicts zero-typing rule; no picker button.
- A11y: `Field` renders `<label>` wrapping (good); no `aria-describedby` for hint/error; Modal lacks dialog semantics (B-36).
- **Verdict: Fix.** Redesign: directory field gains `Choose…` picker button (reuse `pickDirectory`); error gets `role="alert"`; Create label `Create profile · ⌘↵`; Enter submits.

### B-34 · LoginModal: Sign in inside / Open claude.ai / Copy command / Re-check
- Loc: [Profiles.tsx:338](../../../ui/src/screens/Profiles.tsx#L338); buttons :382/:386/:389/:400. Command from `account_login_command` (bridge:247).
- Backend: Sign-in `terminal_spawn_login` (:379); Web `login_open_web` (:240, **no mock guard — throws in preview**); Copy `navigator.clipboard`; Re-check `account_login_state` (:254) then refresh (result discarded, :148).
- Success: terminal opens + modal closes · Error: Sign-in `window.alert` (:367); others silent · Disabled: `starting` on primary only; `Copied` feedback 1.5s (:395) good.
- Copy bug: footer copy at :404 says terminal `opens at the bottom` — stale (terminal is now the detail panel). Fix copy to `opens in the main panel`.
- **Verdict: Fix.** Redesign: Re-check shows result chip `Still signed out — finish in terminal, then re-check`; Web button disabled in preview with tooltip; all errors → toast. Order: primary Sign-in, secondary Re-check, tertiary Copy/Web.

### B-35 · OnboardingWizard: scan rows, plan tiles, footer nav, login quartet
- Loc: scan row [OnboardingWizard.tsx:167](../../../ui/src/screens/OnboardingWizard.tsx#L167), fresh link (:200), label input (:235), plan tiles (:248), close `✕` (:130), Back/Create/Continue/Done (:387-431), login buttons (:300-327).
- Backend: `environment_scan` (:208) auto on mount; `account_import`/`account_create` (:235/:170); login quartet mirrors B-34 (`terminal_spawn_login`, `login_open_web`, clipboard, `account_login_state`).
- Success: step advance + summary checklist (:350) — best success feedback in app · Error: inline danger (:266) · Disabled: Create disabled until label non-empty (:402) — exemplary; scan loading is dot+text (:154).
- A11y: close is unlabelled `✕` text button (no `aria-label`); progress dots (:118) have no `aria` step text; plan tiles are toggle buttons without `aria-pressed`.
- **Verdict: Keep + Fix (protect per F2.6).** Redesign (surgical only): close → icon button `aria-label="Close wizard"`; add visually-hidden `Step {n} of 4: {name}`; plan tiles `aria-pressed`; login primary gets busy state (currently spawns on every click, :303 unguarded). Freeze all other interaction.

### B-36 · Shared Modal / Field / Input / Select / Button primitives
- Loc: Modal [form.tsx:54](../../../ui/src/components/form.tsx#L54), Field/Input/Select (:8/:28/:38), Button [primitives.tsx:31](../../../ui/src/components/primitives.tsx#L31).
- Modal: Escape via wrapper keydown (:68); close button **has** `aria-label="Close"` (:86) — the only labelled close in app. No `role=dialog`/`aria-modal`/trap/return-focus; backdrop click does **not** close.
- Button: `disabled:opacity-40 pointer-events-none` (:43) — disabled buttons vanish from tab order with no reason announced; no `loading` prop, every caller hand-rolls busy text.
- Select (:38) is exported but **never used** (grep-verified) — dead primitive.
- **Verdict: Fix + Remove (Select).** Redesign: Modal gains dialog semantics + trap + backdrop-click-close + `aria-labelledby`; Button gains `loading` (spinner + `aria-busy`, keeps width) and `title`-driven disabled-reason pattern; delete `Select` until a real select ships.

### B-37 · TerminalView (xterm host)
- Loc: [TerminalView.tsx:29](../../../ui/src/components/TerminalView.tsx#L29) · No buttons; keystrokes → `terminal_write` (bridge:389), resize → `terminal_resize` (:393), snapshot+attach on mount (:58).
- Success: live stream · Error: snapshot/attach failures uncaught (silent blank terminal) · Empty (preview): writes fake transcript (:66) — honest `(preview…)` label, good.
- A11y: xterm canvas; auto-focus on mount (:96) can steal focus from rail navigation; no `aria-label` on host div (:115).
- **Verdict: Fix.** Redesign: host `role="application" aria-label="Terminal: {title}"`; focus only on explicit panel activation, not every mount; attach failure → inline banner `Terminal disconnected · Reconnect` button (Add).

### B-38 · Icons & dead code
- `IconDashboard` ([icons.tsx:22](../../../ui/src/components/icons.tsx#L22)) unused; `hasLiveBackend` voided at [NewSession.tsx:229](../../../ui/src/screens/NewSession.tsx#L229); `stopSession` (bridge:314) exported, unused (Stop uses `terminalKill`).
- **Verdict: Remove** all three (or wire `stopSession` as B-22 backend and delete dynamic import). Dead affordances erode audit trust.

## 8. Cross-cutting systems: feedback, keyboard, a11y

### Toasts / alerts / banners — current state (no toast system exists)

| Channel | Where | Problem |
|---|---|---|
| `window.alert` ×3 | [App.tsx:371](../../../ui/src/App.tsx#L371), [App.tsx:384](../../../ui/src/App.tsx#L384), [Profiles.tsx:367](../../../ui/src/screens/Profiles.tsx#L367) | Modal, unstyled, blocks terminal; no Retry/Undo |
| Silent swallow | `refresh()` catch [App.tsx:113](../../../ui/src/App.tsx#L113); Stop (B-22); Delete (B-32) | Stale data presented as fresh (F2.1) |
| Inline danger text | NewSession :211, Profiles banner :95, Wizard :266 | Inconsistent placement; no `role=alert` |
| Success toasts | **None** — success inferred from navigation | Launch/handover give no confirmation copy |

**Target (Add):** one toast stack (bottom-right, `aria-live=polite`,
auto-dismiss 6s, persistent for errors) + one app banner for
`degraded` state (`Couldn't refresh · showing last data · Retry`).
Every B-entry above routes Success/Error through these two.
Destructive actions (B-22, B-32) add Undo in-toast. This single
system closes F2.1's UI half.

### Keyboard map — today vs target

| Key | Today | Target |
|---|---|---|
| `⌘B` / `⌘N` | Rail / New ([App.tsx:206](../../../ui/src/App.tsx#L206)) | Keep + editable-guard (B-04) |
| `Esc` | Closes modal only if focus inside wrapper (NewSession :79, form :68); wizard has **no Esc** | Close topmost layer; wizard Esc = close with unsaved guard |
| Arrows in tabs/splitter | None | Tabs Arrow/Home/End; splitter arrows (B-08/B-13) |
| `⌘K`, `⌘1-3`, `⌘R`, `⌘.`, `⌘↵` | Absent | Palette, tab jump, Resume, Stop-confirm, submit (Add) |
| `Tab` order | DOM order; disabled buttons skipped silently | Keep; add skip-link `Skip to panel` (Add) |

### Accessibility debt (code-read, extends UX.8)

- Contrast: faint `#5b6474` on canvas ([styles.css:24](../../../ui/src/styles.css#L24)) at 10.5px rail subtext likely <4.5:1 — raise to `#8b95a7` (muted) or 11.5px. Not empirically verified (no runtime test).
- Motion: `breathe` ([styles.css:90](../../../ui/src/styles.css#L90)) + glows ignore `prefers-reduced-motion` — add media query disabling animation.
- Colour-only: rail dots (B-10/B-11/B-12), footer health (B-05), quota ring colour ([primitives.tsx:178](../../../ui/src/components/primitives.tsx#L178)) — pair every dot/ring with text (redesigns above).
- Focus: global ring ([styles.css:76](../../../ui/src/styles.css#L76)) is strong; modal trap/return is the gap (B-36).
- Targets: collapsed dots 32px (B-12), Skip link ~16px (B-21) — bring interactive minima to 36px.

### Dead ends & fabricated data (must-fix list)

1. Usage/Fleet `ComingSoon` ([App.tsx:525](../../../ui/src/App.tsx#L525)) — remove from nav (B-02); if kept, copy must name milestone + what lands (`Usage · M3: 5h/weekly quota per account`), not generic `M2+`.
2. Quota rings `percent={0}` (B-14) and `Machine OK / Fleet 1` StatCards ([App.tsx:461](../../../ui/src/App.tsx#L461)) — honest empty states until probes exist.
3. Dead modal (B-29), dead Select/IconDashboard/stopSession (B-36/B-38).
4. ComingSoon competitor contrast: Opcode ships a real Usage dashboard with cost-by-model ([github.com/winfunc/opcode](https://github.com/winfunc/opcode)); cc-switch shows tray quota inline (prior §cc-switch). A placeholder nav item compares worse than no nav item.

## 9. Competitor bar for shell chrome (what “good” looks like)

- **T3 Code** ([t3.codes](https://t3.codes/)): worktree-per-thread + diff viewer in shell; Juggler rail has neither — B-27 Resume + future diff entry must live in SessionPanel, not new nav.
- **Claude Squad** ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)): single-key actions (`s` commit+push, `c` checkout, `r` resume) — model for B-22/B-27 shortcuts; its `-y` auto-accept crosses Juggler's click-bound invariant — do not copy.
- **Conductor** ([conductor.build/docs](https://www.conductor.build/docs/concepts/workflow)): Checks tab (git/CI/todos) beside terminal — model for SessionPanel L2 detail (§10 wireframe).
- **Vibe Kanban** ([github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban)): card=worktree=agent, inline comment→agent — now sunsetting (prior F7.4); its lesson is review-in-panel, not kanban. Do not import the board.
- **cc-switch** (prior §cc-switch): tray `name · quota` one-click switch — model for B-05 footer becoming quota glance (`A 22% · B 81% · reset 17:00`).

## 10. Target shell wireframe (ASCII)

```
┌──────────────────────────────────────────────────────────────────┐
│ ◉ AI Juggler  Session: fix-auth · Work Max        ⌘K  ＋ New ⌘N │ header (B-01..B-04)
├────────────┬─────────────────────────────────────────────────────┤
│ INBOX (3)  │  fix-auth  ● Needs you · waiting 4m   [Reply] [⋯]   │
│ PROJECTS   │ ┌─ terminal (xterm, B-37) ───────────────────────┐ │
│ SESSIONS   │ │  $ claude …                                     │ │
│ ─────────  │ └─────────────────────────────────────────────────┘ │
│ ● api  2   │  Harness claude-code · Age 22m · Project api        │
│ ○ web      │  [Continue on another account…]  [Stop session ⌘.]  │ (B-22/B-23)
│ ─────────  │  Quota: A ██████░░ 78% · resets 17:00 (Measured)    │
│ ＋ New ⌘N  │                                                     │
│ A22% B81%  │  ← B-05 footer: quota glance, click → Overview      │
└────────────┴─────────────────────────────────────────────────────┘
  rail 264px (B-13 splitter)        toast stack ↗ bottom-right
```

Changes vs today: Inbox first with badge; Instances gone;
footer shows quota glance not `healthy`; SessionPanel carries
the clearing action + quota line; `⌘K` in header. Everything
else (rail lists, terminal, wizard) keeps its current geometry.

### Missing controls to Add (no B-number today)

| Add | Placement | Spec |
|---|---|---|
| `⌘K` palette | Header + global key | Fuzzy projects/sessions/accounts + actions (Resume/Handover/Stop); recent-first |
| Needs-Attention Inbox | Rail tab 1 | Waiting/Stuck/QuotaBlocked/crashed rows, waiting-desc, snooze/mute, badge count |
| Toast + degraded banner | App root | See §8; replaces all alerts/silence |
| Resume / Relaunch | Ended session rows + panel (B-27) | `⌘R`; same-account `--resume` first (T1) |
| Search/filter in rail lists | Above rail list | Filters B-10/B-11; fixes `slice(0,15)` hiding (Sessions :377, Instances :305) |
| Session overflow `Show all` | Rail + B-23/B-26 lists | Replaces silent slices (15/8/6/5) everywhere |
| Shortcuts popover `?` | Header | Lists B-04 target map; discoverability fix |

## 11. Prioritised fix list & acceptance checks

**P0 (trust):** toast/banner system (§8); Stop confirm+Undo (B-22);
Delete confirm+Undo (B-32); launch/handover error handling
(B-23/B-26); remove fabricated rings/stats (B-14); remove
Usage/Fleet nav (B-02). **P1:** handover wizard (B-23); Resume
(B-27); modal/tabs/splitter a11y (B-08/B-13/B-36); `⌘K` +
Inbox (Add table); delete dead modal/code (B-29/B-38).
**P2:** quota glance footer (B-05), skeletons (B-31),
shortcuts popover, contrast/motion fixes (§8).

Acceptance (testable): (1) kill backend → banner + Retry, no
stale-as-fresh; (2) Stop/Delete each show confirm and Undo
restores; (3) keyboard-only: `⌘K`→session→`⌘R` resume without
mouse; (4) screen-reader pass: every B-entry has accessible
name + state; (5) `grep -r "window.alert" ui/src` returns zero;
(6) rail with 30 sessions: search finds #30 (no slice hiding).

*End of audit 01. Siblings: 02 (sessions/handover), 03 (accounts/
onboarding), 04 (quota/usage) — cross-refs F1–F7 in prior review.*





