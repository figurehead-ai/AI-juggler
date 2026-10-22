# 11 — UX Redesign: Build-Ready Screen Spec (Deep)
> Parent: [full review](../full-competitive-ux-review.md#L1) · Prompt: [agent-review-prompt.md](../../internal/agent-review-prompt.md#L30) · Contract: [SPEC.md](../../SPEC.md#L316) §9 + [ADR-0007](../../adr/0007-instance-centric-information-architecture.md#L1). Audience: designer + frontend engineer; every screen is implementable without a follow-up question. Evidence: `file:line` = repo truth, URL = competitor bar. **Keep/Fix/New/Remove** + P0/P1/P2 as in [full review](../full-competitive-ux-review.md#L10).

## 0. How to build from this doc
- Order: §3 shell/state → §4.1 Inbox → §4.17 palette → §4.9 Handover → rest. Quota feed (M3) gates §4.10–4.11 data only; layouts + honest empty states ship first.
- **Never render a number the backend did not send.** Overview hardcodes rings `percent={0}` + `Machine OK/Fleet 1` today ([App.tsx](../../../ui/src/App.tsx#L461)); redesign replaces each with an explicit empty state.
- Passive invariant on every screen: app prepares/suggests/stages, only an explicit click mutates ([SPEC.md](../../SPEC.md#L26)); sole exception is opt-in per-profile crash auto-restart, off by default. No auto-switch/rotate/resume control may be added.

## 1. Design principles
### 1.1 Calm supervision (tower, not cockpit)
1. **Silence is a feature.** Default surface answers “does anything need me?” and otherwise stays quiet — inverting today's count-led Overview ([App.tsx](../../../ui/src/App.tsx#L431)). 2. **One queue, one badge, everywhere:** needs-attention count is identical in rail, tray, title. Statuses `WaitingUser/Stuck/QuotaBlocked` exist in domain ([model.rs](../../../crates/juggler-core/src/model.rs#L188)) but are never emitted ([full review](../full-competitive-ux-review.md#L201)); emitting them is a design dependency. 3. **Every alert carries its clearing action.** 4. **Honesty is visual:** every quota/cost number shows Measured/Computed/Estimated as text ([SPEC.md](../../SPEC.md#L254)). 5. **Source always survives:** handover copies, never moves ([SPEC.md](../../SPEC.md#L287)); destructive controls state what remains.
### 1.2 Glanceable (3-second rule)
Status readable in ≤3 s: icon + word + colour, never colour alone. Today status is a colour dot ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L419)) and `faint #5b6474` at 10.5 px ([styles.css](../../../ui/src/styles.css#L24)) — both fail (see §2.4). Numbers tabular (`tabular-nums`, [primitives.tsx](../../../ui/src/components/primitives.tsx#L156)); times relative + absolute on hover/focus (`resets 17:00 · in 2h 14m`). Max 2 lines/rail row, 5 columns/table at default width; overflow → L3.
### 1.3 Keyboard-first
Every daily action ≤3 keystrokes from anywhere ([full review](../full-competitive-ux-review.md#L855)); today only `⌘B/⌘N` exist ([App.tsx](../../../ui/src/App.tsx#L205)). Focus always visible/logical/returned; terminal focus explicit (`⌘.` enters, `Esc` returns to chrome). Shortcuts never fire while typing except `Esc/⌘K/⌘Enter/⌘.`. No traps except modal dialogs (which must trap — today they do not, §2.5).
### 1.4 Progressive disclosure
L1 glance (tray/rail) → L2 triage (inbox row/panel header: why + forecast + one action) → L3 detail (history, pricing, lineage). Daily loop never requires L3; fixes today's inversion where L1 hides behind `ComingSoon` ([App.tsx](../../../ui/src/App.tsx#L338)). Full rules in §3.4.

## 2. Design-system gaps (build before screens)
Seed: `@theme` tokens ([styles.css](../../../ui/src/styles.css#L12)), `Button/Card/Chip/StatCard/QuotaRing` ([primitives.tsx](../../../ui/src/components/primitives.tsx#L31)), 8 icons ([icons.tsx](../../../ui/src/components/icons.tsx#L22)), form/modal ([form.tsx](../../../ui/src/components/form.tsx#L54)). A good seed, not a system.
### 2.1 Tokens (keep names, add)
| Group | Today | Add |
|---|---|---|
| Colour | Dark only ([styles.css:14](../../../ui/src/styles.css#L14)) | `[data-theme=light]` mirror; soft-pair text ≥4.5:1 both themes |
| Status | Dots only | `--color-status-{waiting,stuck,quota,review}` + icon+label (§2.4) |
| Type | Sans/mono ([styles.css:42](../../../ui/src/styles.css#L42)) | 11/12/13/15/18/24 px; `faint` banned <11.5 px |
| Space | `--radius-card` ([styles.css:49](../../../ui/src/styles.css#L49)) | 4-pt scale; radii 6/10/14/full; row heights 32/40 |
| Motion | Ease + breathe ([styles.css:52](../../../ui/src/styles.css#L52)) | 100/150/250 ms; reduced-motion kills breathe/glow |
| Elevation | Card + glows ([styles.css:55](../../../ui/src/styles.css#L55)) | Popover/dialog shadows; glow = live/primary only |
### 2.2 Density / themes / motion / icons
**Density:** Cozy default / Compact (rail 32 px, table 36 px, terminal stays 12.5 px [TerminalView.tsx](../../../ui/src/components/TerminalView.tsx#L41)); hit targets ≥28 px. **Themes:** dark default, light first-class (not inverted); `prefers-color-scheme` on first run, override in Settings. **Motion:** 100–250 ms ease-out, 100 ms panel cross-fade; live-dot breathe static under reduced motion. **Icons:** keep 24-grid/1.75-stroke ([icons.tsx](../../../ui/src/components/icons.tsx#L8)); add `inbox, search, bell, gauge, coins, branch, diff, archive, play, handover, queue, shield, keyboard, sun/moon`; icon-only banned in primary flows; per-harness glyphs replace single Claude spark ([Profiles.tsx](../../../ui/src/screens/Profiles.tsx#L264)) before adapter #2.
### 2.3 New primitives; deletions
Add `Banner, Toast (6s), Skeleton, EmptyState, StatusChip (all 9 statuses, [model.rs:188](../../../crates/juggler-core/src/model.rs#L188)), QuotaMeter (bar+%+tier+reset; replaces ring-only [primitives.tsx:165](../../../ui/src/components/primitives.tsx#L165)), ConfirmDialog, WizardShell, DataTable, Kbd`. Delete `window.alert` paths ([App.tsx:371](../../../ui/src/App.tsx#L371), [Profiles.tsx:367](../../../ui/src/screens/Profiles.tsx#L367)) + dead duplicate modal ([WorkPanels.tsx:354](../../../ui/src/screens/WorkPanels.tsx#L354)).
### 2.4 Status encoding + contrast (normative)
Text ≥4.5:1, large/icons ≥3:1, both themes; retire `faint` for body text. Map (icon+word always): Running `●`, WaitingUser `◐ Needs you`, Stuck `▲ May be stuck?` (always `?` — heuristic, [SPEC.md:220](../../SPEC.md#L220)), QuotaBlocked `⛔ Quota`, Review `◆ Ready to review`, Finished `✓ Done`, Failed `✕ Failed`, Stopped `■ Stopped`, HandedOver `↗ Handed over`.
### 2.5 Dialog / focus / live-region contract
Every modal/wizard: `role=dialog aria-modal`, labelled close (today bare `✕`, [OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L130)), trapped focus, `Esc` closes (except mid-handover-copy), focus returns to invoker. One polite `aria-live` for results, `role=alert` + Banner for errors. Tabs = full ARIA pattern (arrows + `tabpanel`); today rail lacks it ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L119)). Splitter keyboard-operable (`←/→ ±8px, Home/End`). Keep global `:focus-visible` ring ([styles.css:76](../../../ui/src/styles.css#L76)).

## 3. New IA & navigation model
ADR-0007's two-mode split is right ([ADR-0007](../../adr/0007-instance-centric-information-architecture.md#L1)); contents are not — Instances = Sessions filtered through same rows ([full review](../full-competitive-ux-review.md#L171)).
```
RAIL 60↔208–360px (⌘B)              HEADER (drag region)
┌─────────────────────────┐  ┌─────────────────────────────────┐
│ ◉ AI Juggler     [⌘K ⌕] │  │ <context> [harness]             │
│ ● Needs attention  (3)  │  │   Overview Accounts Usage Cost ⚙│
│ ▸ Projects  ▸ Sessions  │  │                      [+ New ▾]  │
│ ▸ Accounts              │  └─────────────────────────────────┘
│ + New session     ⌘N    │  MAIN: Inbox | Project | Session | global
│ this-mac · healthy v0.x │
└─────────────────────────┘
```
- **Rail (new order):** `Needs attention (badge) → Projects → Sessions → Accounts`. **Remove** Instances tab until resource manager gives it distinct content (CPU/RAM/tokens per [SPEC.md:331](../../SPEC.md#L331)); reintroduce as resource view, not filtered list.
- **Header:** `Overview · Accounts · Usage · Cost · ⚙ Settings`. **Remove** Fleet until M6; `ComingSoon` nav ([App.tsx:338](../../../ui/src/App.tsx#L338)) trains users to stop exploring. `+ New ▾`: Session / Account / Import machine… (disabled with reason pre-M6).
- **Selection:** single selection (project|session|account|inbox) + global override, extending today's model ([App.tsx:53](../../../ui/src/App.tsx#L53)); Search/palette/tray all land here. Below 1100 px rail auto-collapses to icons+badges (window min 1024×680, [tauri.conf.json](../../../crates/juggler-app/tauri.conf.json#L1)); terminal stacks first, min-height 240 px.
- **Global state:** `loading|ready|degraded(reason)|error(retry)`; `refresh()` must surface failure, not swallow it ([App.tsx:101](../../../ui/src/App.tsx#L101)). Skeletons until first fetch (never flash empty); degraded = Banner `Showing last known state from 14:32 — Retry`; `Stale · 12m` chips; offline disables launch/handover with reason, never hides data. Every screen inherits `empty/loading/error/offline/stale`.
- **Keys (also Settings → Shortcuts):** `⌘K` palette · `⌘N` new · `⌘B` rail · `⌘1–4` rail sections · `⌘⇧H` handover · `⌘,` settings · `⌘.` terminal focus · `g then i/p/s/a` go-to · `?` help · `Esc` close/back. Bench: Claude Squad single-key triage ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)) — match it, never copy its auto-accept daemon.
- **Disclosure rules:** L1 count+worst status only; L2 why+forecast+one action; L3 history/pricing/lineage/raw paths; no modal except wizards + destructive confirms; advanced fields behind `▸ Details`, closed by default, remembered.

## 4. Screen-by-screen spec
Per screen: **Purpose · Entry→Exit · Elements** (`→ cmd` via [bridge.ts](../../../ui/src/lib/bridge.ts#L1)) · **States · Keys · A11y · Copy** (verbatim). Wireframes = layout truth at 1360×860 ([tauri.conf.json](../../../crates/juggler-app/tauri.conf.json#L1)).

### 4.1 Home / Needs-attention (Inbox) — New, P0, default landing
- **Purpose:** “which session needs me now?” in one glance; replaces `WorkEmpty`/`ReadyHome` as resting state (sign-in gate folds in as rows, §4.16). Sources: WaitingUser, Stuck?, QuotaBlocked, crashed, review-ready, signed-out ([full review:765](../full-competitive-ux-review.md#L765)). **Entry→Exit:** launch (badge>0) · rail · tray · `⌘1`/`g i` → row/action jumps to Session/Project/Account; clearing returns here.
- **Elements:** `Needs attention (n)` + sort `Waiting longest ▾` (Waiting/Newest/Project) · chips `All·Needs reply·May be stuck·Quota·Review·Sign-in` · rows: status icon+word, label, project, account, ticking `waiting 24m`, one primary action (`Reply…/Hand over…/Review/Sign in/Resume`), `Snooze ▾` (1h/Until reset/Mute), `⋯` · footer `5 running · 2 done today · A 78% · resets 17:00`.
```
┌─ Needs attention (3) ─────────── Sort: Waiting longest ▾ ┐
│ [All][Needs reply·1][Stuck?·1][Quota·1][Review]          │
│ ◐ Needs you billing-migration·payments-api·Work Max      │
│  Waiting 24m — “Allow edit…?”          [Reply…] ⋯        │
│ ⛔ Quota api-refactor·Work Max  Resets 17:00 [Hand over…] │
│ ▲ May be stuck? nightly-tests · no output 18m  [Open] ⋯  │
│ 5 running · 2 done today · Work Max 78% · resets 17:00   │
└──────────────────────────────────────────────────────────┘
```
- **States:** empty `All clear — 5 sessions running, nothing waiting on you. [New session]` · loading 3 skeletons · error Banner+Retry, rows keep data + `Stale` · offline actions disabled (`Backend unavailable`) · stale freezes timers + `Updated 4m ago`. **Keys:** `↑↓ Enter, r` reply, `h` handover, `s` snooze, `e` archive. **A11y:** listbox + `aria-activedescendant`; badge `aria-label="3 sessions need attention"`; arrivals announced ≤1/30s batched. **Copy:** `Needs you`, `May be stuck?`, `Hit its 5-hour limit`, `Snoozed for 1 hour. It will reappear if still waiting.` Never bare “Stuck”/“Error”.

### 4.2 Projects (rail list) — Fix, P0
- **Purpose:** juggling anchor — every directory, who is on it, which account ([ADR-0007](../../adr/0007-instance-centric-information-architecture.md#L1)); today rows show name+counts only ([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L238)). **Entry→Exit:** rail/`⌘2`/`g p` → row §4.3; `⌘N` with project preselected → §4.8.
- **Elements:** `Filter projects…` (`/` focuses) · sort `Recent ▾` (Recent/Name/Most sessions) · rows: icon, name, `⎇ main · ●2 running|idle`, account dots (≤3+`+2`), `last active 12m` · focus actions `▶ New session`, `Open in Finder` · Pinned group (`⌘⇧P`) · footer counts. **States:** empty keeps “Launch a session in any directory — it becomes a project here.” + `Choose folder…` · skeletons · §3 states · no-results `No projects match “xyz”. [Clear]`. **Keys:** type-ahead, `↑↓ Enter`. **A11y:** full row `aria-label` (“payments-api, main, 2 running, active 12m ago”); state in text. **Copy:** `On it now`, `Free to juggle`, `2 running · 1 needs you`.

### 4.3 Project detail — Fix, P0
- **Purpose:** who is/was here, launch-with, hand over, history; evolves [WorkPanels.tsx:196](../../../ui/src/screens/WorkPanels.tsx#L196). **Entry→Exit:** Projects/palette/inbox → session §4.7, account §4.5, `Hand over…` §4.9; `Esc` back.
- **Elements:** header name (inline rename), `⎇ branch · dirty`, root (mono, copy, `▸ Details`: canonical path/first seen) · per-account `QuotaMeter` + forecast · `On it now` rows (StatusChip, age, tokens, `Open/Hand over…/Stop`) · `Launch with…`: ranked profiles, `★ usual`, admission `OK/Tight/No` ([SPEC.md:237](../../SPEC.md#L237)), signed-out reason · `History` timeline (Today/Yesterday/Earlier, lineage links) · L3 `Forget project…` (confirm).
```
┌─ payments-api ⎇ main·3 modified ─────── [Rename][⋯] ─────┐
│ ~/Dev/payments-api [Copy]  Work Max ▓▓▓▓░ 62% Measured   │
│ ON IT NOW ● billing-migration·Work Max·24m [Open][⋯]     │
│ LAUNCH WITH ★=usual  ★ Work Max·opus·OK [Start]          │
│  Personal·Weekend·signed out [Sign in]  HISTORY 3 today  │
└──────────────────────────────────────────────────────────┘
```
- **States:** idle `No live instance — free to juggle.` · no history `Nothing recorded yet.` · launch failure inline Banner (never alert) · stale quota greys + `Quota unknown — last seen 22m ago`. **Keys:** `Enter` open live, `n` new on usual, `h` handover, `c` copy path. **A11y:** disabled reasons via `aria-describedby`; meter text twins. **Copy:** `★ usual for this project`; Tight text `stopping “nightly-tests” (2.9 GB peak) makes this comfortable. [Start anyway]` — advisor never blocks.

### 4.4 Instances / Accounts (rail Accounts) — Fix, P1
- **Purpose:** account fleet at a glance, replacing Instances tab (F2.2); distinct Instances resource view returns only with real data (§3). Bench: cc-switch one-row switcher ([github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)). **Entry→Exit:** rail `Accounts`/`⌘4`/header → row §4.5; `Add account` → §4.16 add-mode.
- **Elements:** rows: glyph, label, plan chip, login chip (`Logged in/Needs login/Expired?`), mini 5h meter, `n live`, last used · `Add account`, `Import…` · group by harness if >1 · row `⋯`: New profile, Clone…, Sign in/Re-check. **States:** empty [Profiles.tsx:157](../../../ui/src/screens/Profiles.tsx#L157) + `Import existing setup` · skeletons · quota-unknown `Quota not available yet` (no fake ring) · signed-out sorts last. **Keys:** `↑↓ Enter, a` add, `/` filter. **A11y/Copy:** never colour-only; `Needs login — Claude's sign-in, ~30 seconds`, `Expired? — re-check`.

### 4.5 Account detail — Fix, P1 (today cards [Profiles.tsx](../../../ui/src/screens/Profiles.tsx#L174))
- **Purpose:** one login's health/quota/profiles/history; wrong-tier/expiry surprises die here (F1.4). **Entry→Exit:** Accounts/Project/palette → Launch §4.7, Sign-in terminal, Usage §4.10.
- **Elements:** header glyph, label (rename), harness, plan chip + `Change plan…`, login chip + `Sign in/Re-check` (→ `terminal_spawn_login` [bridge.ts:378](../../../ui/src/lib/bridge.ts#L378)) · quota card: 5h+weekly meters, tier, reset countdowns, `exhausts ~15:40 at current burn` · health card with honest unknowns (`Tier: Max 20× (you set) · Vendor reports: — · Expires: unknown`; extends metadata-only `login_state`, never credential contents) · profiles table (name/model/default dir/live, `Launch/⋯ Edit·Clone·Delete`; Delete = ConfirmDialog, today one-click [Profiles.tsx:239](../../../ui/src/screens/Profiles.tsx#L239)) · recent sessions (10, `View all →`) · `▸ Details`: state dir, created, storage.
```
┌─ ◈ Work Max [Max 20×▾] ● Logged in ─── [Re-check][⋯] ────┐
│ Claude Code · 3 profiles · 2 live · used 4m ago          │
│ 5h ▓▓▓▓▓░ 78% Measured·resets 17:00  7d ▓▓░░ 31%         │
│ PROFILES Deep work·opus [Launch]  SESSIONS ●/⛔ [All →]   │
└──────────────────────────────────────────────────────────┘
```
- **States:** signed-out quota → `Sign in to see quota — Juggler never sees your credentials. [Sign in]` · skeletons · stale timestamped · delete blocked live (`2 live sessions use this profile — stop them first.`). **Keys:** `l` launch, `⇧L` picker, `u` Usage. **A11y:** meter text twins; plan `listbox`; copy never implies vendor-verified tier: `Plan (you set this — vendor doesn't report it yet)`.

### 4.6 Sessions (list) — Fix, P0
- **Purpose:** every conversation findable/resumable; fixes 15-row cap, no search ([WorkflowRail.tsx:377](../../../ui/src/components/WorkflowRail.tsx#L377)). **Entry→Exit:** rail/`⌘3`/`g s` → row §4.7; filters persist; archived behind toggle.
- **Elements:** search `Search sessions, projects, transcripts…` (§4.12) · chips `All/Live/Needs you/Done/Archived` · project+account filters · sort (Recent/Waiting/Tokens) · `DataTable`: StatusChip, Label (editable), Project, Account, Age/Waiting, Tokens, `⎇ Branch`, `⋯` (Open, Resume/Relaunch, Hand over…, Archive, Export pack…) · bulk Archive/Export · `Live now` pinned group.
```
┌─ Sessions [Search…] [All][Live][Needs you][Done] Sort▾ ──┐
│ ● billing-migration payments-api Work Max 24m 182k [⋯]  │
│ ✓ api-cleanup       payments-api Personal  2h   91k [⋯]  │
│ LIVE NOW pinned · ARCHIVED behind toggle                 │
└──────────────────────────────────────────────────────────┘
```
- **States:** empty `No sessions yet — start one below. [New session]` · no-results + clear · skeleton table · ended always `Resume`+`Relaunch` (missing today F2.3; Resume = T1 `claude --resume` [SPEC.md:271](../../SPEC.md#L271)) · archived empty `Nothing archived. Sessions archive after 30 days (Settings).` **Keys:** `/ ↑↓ Enter, r` resume, `h` handover, `e` archive, `x` select. **A11y:** grid roles, Tab-reachable actions, status text first column. **Copy:** `Resume` (same account, lossless) vs `Relaunch` (fresh, same profile+folder), one-line tooltips, never conflated.

### 4.7 Session + terminal — Fix, P0 ([WorkPanels.tsx:44](../../../ui/src/screens/WorkPanels.tsx#L44) + [TerminalView.tsx](../../../ui/src/components/TerminalView.tsx#L29))
- **Purpose:** watch/answer/stop/hand over without leaving; terminal *is* panel (terminal-kind), desktop-kind shows status + Focus-window, no fake terminal. **Entry→Exit:** Sessions/Inbox/Project/Search → rail/`Esc` back; scrollback preserved (5 000 lines [TerminalView.tsx:14](../../../ui/src/components/TerminalView.tsx#L14)).
- **Elements:** header StatusChip, label (rename), kind, project link, account, age, tokens, quota-impact · actions `Stop` (live; ConfirmDialog if running: `Stop “billing-migration”? Transcript is kept — you can Resume later. [Keep running][Stop]`), `Hand over…`, `Resume/Relaunch` (ended), `⋯` (Open project, Copy ID, Export pack…, Archive) · xterm (`⌘F` find, `⋯` Copy/Clear view, `⌘. to type` hint) · below-strip L2: lineage (`Handed over from sess-42 · Work Max ↗`), branch/worktree dirty warning, `▸ Details` (native ID, transcript path, PID, RSS/CPU when available) · login terminals reuse shell + guidance card ([WorkPanels.tsx:128](../../../ui/src/screens/WorkPanels.tsx#L128)).
```
┌─ ◐ Needs you billing-migration·payments-api·Work Max ────┐
│ 24m·182k tok·5h ~9%        [Hand over…][Stop][⋯]         │
│ ┌ terminal ──────────────────────────────── ⌘F find ─┐   │
│ │ ❯ Allow edit to src/billing.ts? (y/n)              │   │
│ └────────────────────────────────────────────────────┘   │
│ ↗ from sess-42·Personal  ⎇ juggler/billing               │
└──────────────────────────────────────────────────────────┘
```
- **States:** starting (`Starting…`) · live · waiting (pulse once, then static amber) · ended (`Finished · scrollback replayed`) · failed (`Failed (exit 1) — [Resume][View log]`) · detached read-only + `Reconnecting…` · desktop empty honest placeholder. **Keys:** `⌘.` focus, `⌘F`, `⌘⇧H`, `⌘⌫` stop+confirm, `Esc`. **A11y:** focus order header→terminal→strip; offscreen live summary on status change only (`…needs your reply: Allow edit…`; canvas never read raw). **Copy:** handover promise moves to wizard with tier (§4.9), never bare (today [WorkPanels.tsx:181](../../../ui/src/screens/WorkPanels.tsx#L181)).

### 4.8 New Session wizard — Fix, P0 (today [NewSession.tsx](../../../ui/src/screens/NewSession.tsx#L18))
- **Purpose:** folder + account with forecast *before* committing; zero typed paths kept (recents+picker+drop [NewSession.tsx:49](../../../ui/src/screens/NewSession.tsx#L49)). **Entry→Exit:** `⌘N`/New/drop/Project Start → success §4.7; `Esc` returns.
- **Elements:** 1 *Where:* recent cards (name/⎇/last active/prior count), `Choose folder…` (→ [bridge.ts:468](../../../ui/src/lib/bridge.ts#L468)), drop zone, worktree toggle + `juggler/<profile>/<short>` preview + same-branch warning (F1.1). 2 *Who:* ranked profiles (meter, `~40m left`, `★ usual`, admission, signed-out + inline Sign in) + suggest-only banner `Personal has 80% headroom and usually runs this project. [Use Personal][Keep Work Max]`. 3 *Review:* folder/account/model/worktree/admission + `Start session` (→ [bridge.ts:282](../../../ui/src/lib/bridge.ts#L282)).
```
┌─ New session 2/3: Which account? ──────────── [✕] ───────┐
│ payments-api·⎇ main·worktree juggler/work/…   [Change]   │
│ ★ Work Max·opus ▓▓▓▓░ 62%·~40m·Tight      [Select]       │
│   Personal·sonnet 80% free·OK             [Select]       │
│ ⓘ Personal usually runs this project.      [Use it]      │
│ [Back]                                    [Review →]     │
└──────────────────────────────────────────────────────────┘
```
- **States:** no profiles `No profiles yet — add an account first. [Add account]` (routes §4.16, not dead modal) · skeleton · error inline, `Start anyway` only for Tight, never backend-down · preview-mode message kept. **Keys:** `Enter/⌘Enter`, arrow `radiogroup`. **A11y:** `aria-current=step`; forecast in row name. **Copy:** `Which account?`, `No folder picked — the profile's default applies`, Start names destination `Start in payments-api on Work Max`.

### 4.9 Handover wizard — New, P0 (replaces inline buttons)
- **Purpose:** continue at quota wall without re-explaining, fidelity stated *before* click (F3.5); today bare buttons ([WorkPanels.tsx:139](../../../ui/src/screens/WorkPanels.tsx#L139)) → `session_handover` ([bridge.ts:460](../../../ui/src/lib/bridge.ts#L460)). **Entry→Exit:** `Hand over…`/Inbox/`⌘⇧H` → opens target §4.7 with lineage; source stays resumable; return offered inline.
- **Elements:** 1 *Target:* ranked accounts (headroom/forecast/`★ usual`/signed-out disabled), tier per target T1–T4 ([SPEC.md:269](../../SPEC.md#L269)). 2 *Fidelity:* tier badge + carry list + redaction preview for pack/export (F1.6); honest v0 label until sibling-file test passes: `Transcript-only (experimental)`. 3 *Confirm:* source→target, freeze note, single `Hand over to Personal`; progress Freeze→Copy→Resume; success + `Queue return at reset (17:00)` (passive timer; resume still one click, F1.5).
```
┌─ Hand over billing-migration 2/3: What carries? ─────────┐
│ Work Max → Personal · T2 Transplant (same harness)       │
│ ✓ Conversation ✓ Subagents ✓ Todos ✕ MCP (reconnect)     │
│ Source stays resumable — nothing is deleted.             │
│ [Back]                      [Continue to confirm →]      │
└──────────────────────────────────────────────────────────┘
```
- **States:** no other account `Add another account to juggle between them. [Add account]` · target tight + `Hand over anyway` · copy failure stays on step `Nothing was changed. [Retry][Cancel]` · re-handover notes prior lineage. **Keys/A11y:** §2.5; tier/carry as text; one primary; `Esc` blocked only during copy (`Finishing copy…`). **Copy:** never “moves”: `Your original stays exactly as it was.`; success `Running on Personal now. Work Max version is paused where you left it. [Return at 17:00]`.

### 4.10 Usage / Quota — New, P0 data / P1 screen (today `ComingSoon`)
- **Purpose:** burn-down/resets/forecasts; the juggling screenshot (WOW-b); engine + honesty ([SPEC.md:254](../../SPEC.md#L254)). **Entry→Exit:** header/Account/tray → cards §4.5, rows §4.7.
- **Elements:** account cards (5h+weekly meters, tier, countdowns, sparkline) · timeline (uPlot [SPEC.md:349](../../SPEC.md#L349); `5h/7d/30d`, series toggles) · thresholds link (Settings) · `Export CSV` · forecast `At current burn: Work Max exhausts ~15:40 · Personal lasts the window`.
```
┌─ Usage [5h|7d|30d] ─────────────────────── [Export CSV] ─┐
│ Work Max ▓▓▓▓▓░ 78% Measured resets 17:00 ~40m left      │
│ Personal ▓▓░░░░ 22% Measured resets 17:00 lasts window   │
│ [chart | View as table]  Forecast list →                 │
└──────────────────────────────────────────────────────────┘
```
- **States:** pre-feed full layout + `No quota data yet — Claude quota arrives with the Measure feed. Tokens below are real.` (never 0% ring) · Estimated banner `Estimated from token burn — may differ from your vendor limit.` · stale per-card · empty guidance. **Keys:** `←→ a e`. **A11y:** table twin default for screen readers; tier+text everywhere. **Copy:** `Measured/Computed/Estimated` verbatim; `resets 17:00 · in 2h 14m`; forecasts always `~` + `at current burn`.

### 4.11 Cost — New, P1
- **Purpose:** tokens → est. USD per session/account/project (F1.2), “is Max #2 worth it?” Bench: ccusage blocks ([github.com/ryoppippi/ccusage](https://github.com/ryoppippi/ccusage)), cc-switch pricing sync ([github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)). **Entry→Exit:** header `Cost`/Usage toggle/Project footer → rows deep-link.
- **Elements:** cards `5h block ~$x · Today ~$y · Month ~$z` · pricing version `Prices: models.dev sync 2026-10-12 — [Update]` (never silent) · breakdown table (Account/Project/Model; in/out/cache, est. USD, share) · soft budget bar (warn-only) · `Export CSV`.
```
┌─ Cost ─ Month ~$— Estimated ─────── Prices 2026-10-12 ───┐
│ [Account|Project|Model]  Tokens  Est. USD  Share         │
│ Work Max·payments-api    1.2M    ~$18.40   ▓▓▓▓░         │
└──────────────────────────────────────────────────────────┘
```
- **States:** no pricing tokens-only + `Add pricing to see estimated cost — Juggler never bills you.` · unknown model `— / Unknown model — no price` (never $0.00) · stale-prices banner. **Keys/A11y:** table-first; `~$` + `estimated` every figure. **Copy:** `API-equivalent estimate — your subscription price doesn't change per token.`

### 4.12 Search (full screen) — New, P0 surface
- **Purpose:** find + act without navigating (WOW-c); palette (§4.17) is quick surface, this is deep (filters+preview). **Entry→Exit:** `⌘K`→`See all results` · Sessions field → owner screen.
- **Elements:** query (autofocus) · scopes `All·Sessions·Projects·Accounts·Transcripts` · filters (status/account/project/date) · grouped rows (title, owner, status, highlighted snippet, age) · preview pane (summary + `Open/Resume/Hand over…`) · counts footer.
```
┌─ Search [billing webhook______] All Sessions Projects ───┐
│ SESSIONS (3) billing-migration·◐ Needs you·24m           │
│  “…stripe webhook retry…” · 2 matches  Preview [Open]    │
└──────────────────────────────────────────────────────────┘
```
- **States:** empty recents + `Type to search sessions, projects, and transcripts.` · no-results `No matches for “xyz”. [Titles only][Clear]` · indexing `Indexing… results may be incomplete` (never silently partial) · offline disables transcript scope with reason. **Keys:** `↑↓ Enter, ⌘Enter, Tab, Esc`. **A11y:** listbox + complementary preview; bold+colour highlights; count announced. **Copy:** snippets redacted (F1.6 patterns), never credential-like strings.

### 4.13 Review queue — New, P2 [inferred — validate first]
- **Purpose:** finished sessions awaiting a look (diff stat + pack preview). Bar: Vibe inline-comment-to-agent + Conductor Checks ([github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban), [conductor.build/docs](https://www.conductor.build/docs/concepts/workflow)). Demand inferred ([full review:791](../full-competitive-ux-review.md#L791)): ship read-only v1; comment-to-agent only after dogfood. **Entry→Exit:** Inbox Review / Sessions Done → Continue/Hand over/Archive.
- **Elements:** rows (session, project, `⎇`, `+n/−m files k`, age, account) · detail: read-only diff + file list, `PACK.md` preview ([SPEC.md:296](../../SPEC.md#L296)), transcript tail · `Continue session, Hand over…, Archive, Export pack…` · bulk-archive >7d.
```
┌─ Review (2) ─────────────────────────────────────────────┐
│ api-cleanup·payments-api ⎇ juggler/cleanup +4/−1 [View]  │
│ Detail: diff stat · PACK.md · tail  [Continue][Archive]  │
└──────────────────────────────────────────────────────────┘
```
- **States:** empty `Nothing to review. Finished sessions land here.` · no-git `No git changes — transcript only.` · stale-diff banner. **Keys:** `j/k, a, c` (+arrows). **A11y:** selectable diff + table twin. **Copy:** `Ready to review`, never “Approve” for untested code.

### 4.14 Notifications (centre + system policy) — New, P1
- **Purpose:** quiet by default, interrupt only when actionable (WOW-g). **Entry→Exit:** header bell (badge = unsnoozed inbox count, same number everywhere) → click navigates to owner; policy in Settings.
- **Elements:** `Needs action` (mirrors Inbox) · `Earlier` (resets/finishes/crashes/updates) · per-item `Snooze/Mute project/Mute type` · `Mark all read` · digest preview. System alerts only: Waiting>threshold, Quota<threshold, crash, queued-return reset; one action each, mutation still requires in-app click.
```
┌─ Notifications ─────────────────────────── [Mark read] ──┐
│ NEEDS “billing-migration” needs reply (24m) [Open]       │
│ EARLIER Work Max reset 17:00 · 2 finished  [Digest 18:30]│
└──────────────────────────────────────────────────────────┘
```
- **States:** empty `You're all caught up.` · muted `Muted until 09:00 — Inbox still tracks 2 items.` · macOS denied: fix guidance, centre keeps working. **Keys:** `⌘⇧N`; §4.1 batching + reduced-motion. **Copy:** `“billing-migration” needs your reply (waiting 24m)`; digest `Today: 4 finished · 1 handed over · Work Max reset 17:00`. Never bare “Session updated”.

### 4.15 Settings (all tabs) — New, P1 (specced [SPEC.md:353](../../SPEC.md#L353), absent in UI)
- **Purpose:** thresholds/preferences without config files. **Entry→Exit:** `⌘,`/⚙/tray → `Esc`/Done returns to prior selection; save-on-change + `Saved` tick + Undo toast (no Apply).
- **Layout:** left tab list (roving tabindex, §2.5) + pane; `/` filters settings; defaults labelled. Tabs:
```
┌─ Settings [Filter…] ─────────────────────────────────────┐
│ General       │ Appearance (Dark|Light|System) Density…  │
│ Notifications │ Terminal font 12.5 · scrollback 5000     │
│ Supervisor    │ Startup: Open to (Inbox|Last view)       │
│ Usage & Cost  │ Data & Privacy │ Shortcuts │ Advanced    │
└───────────────┴──────────────────────────────────────────┘
```
  *General:* Appearance, Density, Reduce motion, Startup view, update channel Stable/Beta + `Check for updates…` ([SPEC.md:357](../../SPEC.md#L357)). *Notifications:* per-event interrupts, waiting threshold (5 min), quota 80%/95%, digest time, per-project mutes, `Send test`. *Supervisor:* stuck thresholds ([SPEC.md:220](../../SPEC.md#L220)), per-profile crash auto-restart (off, same-profile only), RAM/swap/disk + admission margin. *Usage & Cost:* plan caps (→§4.5), pricing sync, soft budget, CSV defaults, retention `Archive 30d · Delete never/after…` (typed confirm, F1.3). *Data & Privacy:* sizes (DB/accounts/packs/logs), `Back up/Restore…/Export diagnostics (redacted)` (F1.7), telemetry Off ([SPEC.md:23](../../SPEC.md#L23)), paths, Reveal. *Shortcuts:* §3 table, rebind + conflict detection, reset, hotkey toggle. *Advanced:* harness versions/verified ranges ([SPEC.md:184](../../SPEC.md#L184), read-only), harness-update notify-only, logs, plain-language guardrails.
- **States/A11y:** skeleton pane · save-error Banner (value reverts, never silent) · switches with state in name. **Copy:** plain units (`Warn when a session waits longer than 5 minutes`).

### 4.16 Onboarding v2 — Fix, P1 (today wizard [OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L40), best-built surface — protect it)
- **Purpose:** zero-knowledge first run: scan → import/fresh → vendor login → summary ([SPEC.md:409](../../SPEC.md#L409), [ADR-0006](../../adr/0006-smart-onboarding-scan-and-import.md#L1)). Auto-opens at zero accounts ([App.tsx:92](../../../ui/src/App.tsx#L92)). Keep interaction model; v2 fixes a11y + funnel leaks (§5), adds harness step for adapter #2.
- **Entry→Exit:** first run / Accounts `Add account` / Inbox sign-in row → Done lands on §4.1 (or first-project prompt if none); dismiss = `I'll set up later` (not bare ✕) with resumable `Finish setup (1 left)` inbox row. ReadyHome gate becomes sticky per profile (F2.7): skip persists unless login unknown/expired.
- **Elements:** `WizardShell` steps `Detect → Identity → Login → Summary` (+ `Harness` first when >1 harness): Detect auto-scan (metadata-only) with found-setup cards (path/origin/signed-in/bytes), `None of these — add fresh`; Identity plain-words name + plan (kept [OnboardingWizard.tsx:20](../../../ui/src/screens/OnboardingWizard.tsx#L20)); Login vendor flow (embedded terminal primary, Open web, Copy command, Re-check); Summary check-list (created/imported counts, `original not modified`, next action `Start first session`).
```
┌─ Add a Claude account ●─●─○─○ ───────── [I'll set up later]┐
│ First, let's look at what you already have.              │
│ (•) ~/.claude · signed in · 48 MB  [Import this →]       │
│ ( ) None of these — fresh account                        │
│ Scanning is metadata-only; originals are never touched.  │
└──────────────────────────────────────────────────────────┘
```
- **States:** scanning skeleton (kept spinner row) · none-found fresh path · import >2 GiB ceiling explained pre-click · login pending `Not signed in yet — finish in the terminal, then Re-check.` · error inline per step (never blocks Back) · preview-mode labelled. **Keys:** full §2.5 wizard (labelled close, trap, return-focus). **A11y:** progress `Step 1 of 4: Detect`; scan results as radio group. **Copy:** keep `AI Juggler never sees your credentials.`, `Your original … was not modified`; add time hint `Takes about 2 minutes`.

### 4.17 Tray + global hotkey + command palette — New, P1 (tray specced [SPEC.md:352](../../SPEC.md#L352), absent in UI)
- **Tray (menu-bar):** first line = inbox summary `3 need you · Work Max 78% · reset 17:00` (L1, §1.4); sections: Needs-attention items (click → window + owner), Accounts (label · 5h% · `New session here`), `New session…`, `Open AI Juggler`, `Settings…, Quit`. Icon badge = unsnoozed count; icon state also in shape (dot/bang), not colour alone. Bench: cc-switch tray switcher ([github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)) — match glanceability, keep suggest-only (no one-click account switch of running work). **States:** no accounts `Set up an account…` · backend down `Reopen AI Juggler` · quota unknown `Quota —` (dash, never 0%).
```
Tray ▾  3 need you · Work Max 78% · reset 17:00
  ◐ billing-migration (24m)…  ⛔ api-refactor…
  Accounts: Work Max 78% [New] · Personal 22% [New]
  [New session…] [Open] [Settings…] [Quit]
```
- **Global hotkey:** default `⌥Space` (rebindable, Settings → Shortcuts; off if conflict) summons palette over any app (Tauri global-shortcut; capability must be added — today only core+dialog [default.json](../../../crates/juggler-app/capabilities/default.json#L1)). Single press = palette; with window focused = `⌘K` equivalent. Never steals focus from a terminal mid-typing without visual summon animation (suppressed under reduced motion).
- **Palette (`⌘K`):** input + grouped results (Actions / Sessions / Projects / Accounts), recents first, fuzzy over titles + transcript titles; inline actions (`Enter` open, `⌘Enter` primary action, `→` preview). Actions include `New session, Hand over…, Resume, Go to…, Settings…`. Footer: `↑↓ navigate · Enter open · ⌘Enter act · Esc close` + `See all results →` (§4.12). **States:** empty = recents + top actions; no-results `No matches — [Search transcripts for “…”]`; offline = local titles only + note. **A11y:** combobox pattern (`aria-expanded/activedescendant`), labelled icon actions (T3/Opcode bar: per-thread surfaces, [t3.codes](https://t3.codes/), [github.com/winfunc/opcode](https://github.com/winfunc/opcode) — palette must feel as instant).
```
┌─ ⌘K ─────────────────────────────────────────────────────┐
│ > bill_______________________________________________  │
│ ACT  Hand over billing-migration…  SESSION billing-… ◐   │
│ Enter open · ⌘Enter hand over · See all results →        │
└──────────────────────────────────────────────────────────┘
```

## 5. Onboarding funnel redesign (targets are hypotheses — instrument, don't assume)
Funnel stages (event names for analytics; local-only counts, telemetry Off by default [SPEC.md:23](../../SPEC.md#L23) — show counts in Settings → Data, never auto-send):
`wizard_shown → scan_completed → import_or_fresh_chosen → account_created → login_completed → first_session_started → second_account_added → first_handover_completed`.
| Stage transition | Leak today (evidence) | v2 fix (§4.16) | Target* |
|---|---|---|---|
| shown → scan done | Auto-scan, no time hint | `Takes ~2 min` + skeleton with progress text | ≥95% (wait) |
| scan → chosen | Found cards bury fresh path | Radio cards, fresh always last + visible | ≥85% choose |
| chosen → created | Name required, plan unclear | Prefill from path, `Not sure yet` default kept | ≥90% create |
| created → login done | Keychain re-login surprise | Pre-warn on import card (`You'll sign in once — 30s`) | ≥70% same session |
| login → first session | Summary ends at Done | Summary primary = `Start first session` (folder preselect) | ≥60% same session |
| first → second account | No prompt | After first quota-relevant event: inbox suggestion (dismissible) | ≥25% in 7 days |
| second → first handover | Fidelity fear (F3.4) | Wizard tier + carry list (§4.9) | ≥40% of quota-blocks |
\* Targets = first-beta goals to validate with 20-user dogfood, not benchmarks; miss → interview, don't add steps. Instrument drop-off per step + time-in-step; any step < target for 2 cohorts gets a usability fix before new features. Protect what works: freeze wizard interaction model + add first UI regression tests ([full review](../full-competitive-ux-review.md#L213)).
- **Empty-machine path:** no Claude found → Detect explains install check + `Show me how to get Claude Code` (vendor link, never bundled installer), fresh path still completable later.
- **Abandon/resume:** dismissed wizard leaves `Finish setup` inbox row + Accounts banner; state (chosen setup, name) persists locally so resume lands on the same step.
- **Add-account (returning user):** same wizard, Detect pre-collapsed (`2 accounts already set up`), Identity prefills next-free label suggestion; success returns to Accounts, not Inbox.

## 6. Cross-screen flows (build these as tests)
1. **Morning triage:** tray `3 need you` → Inbox (oldest first) → `Reply…` focuses terminal (`⌘.` state) → reply clears row live → badge decrements in rail+tray+title together. Fail state: stale badge = P0 bug.
2. **Quota wall:** session hits limit → QuotaBlocked row + system alert (threshold) → Handover wizard (target ranked, fidelity shown) → confirm → target session opens, source shows lineage + `Queue return 17:00` → reset notification → one-click return. No step may auto-execute.
3. **Launch with forecast:** Project `Start` → New Session step 2 shows Tight + workaround → `Start anyway` launches; admission never hard-blocks except backend-down.
4. **Find anything:** `⌘K` “webhook” → transcript hit → `⌘Enter` Resumes owner session at context (scrollback top + find pre-filled). Offline degrades to titles with note.
5. **Review loop (P2):** Finished → Review queue → diff stat → `Continue session` resumes with terminal focused; Archive removes from queue, never deletes transcript.
Each flow gets a Playwright/webview test per [SPEC.md:442](../../SPEC.md#L442) before the screen is “done”; keyboard-only run of flows 1–4 is the a11y gate (no mouse, screen-reader labels on).

## 7. Build order, acceptance, and open decisions
- **Phase A (P0 shell):** state model + Banner/Toast/Skeleton, rail reorder, Inbox v1 (needs status emission), palette, Session Resume/Relaunch. Exit: flows 1+4 keyboard-only pass; no `window.alert` remains (grep gate).
- **Phase B (P0 loop):** New Session + Handover wizards, Project/Account detail quota strips. Exit: flow 2 dogfood on real Claude (also F3.4 fidelity proof); honest tier label until then.
- **Phase C (P1):** Usage/Cost, Notifications, Settings (all tabs), Tray/hotkey, Onboarding v2 + funnel events. Exit: §5 targets instrumented; light theme + Compact pass contrast audit (both themes, automated axe + manual VoiceOver pass — code-read alone is insufficient, [full review](../full-competitive-ux-review.md#L1014)).
- **Phase D (P2):** Review queue read-only, Instances resource view (only with real CPU/RAM data), Search transcript index. Exit: dogfood validates review demand before comment-to-agent.
- **Per-screen done =** all five states rendered, shortcuts work with terminal focused/unfocused, focus returns correctly, copy verbatim, skeleton (not spinner text — today `Loading…` [Profiles.tsx:75](../../../ui/src/screens/Profiles.tsx#L75)), and L1/L2/L3 disclosure checked. Truncation always has tooltip + full text in `aria-label` (rail today truncates silently [WorkflowRail.tsx:272](../../../ui/src/components/WorkflowRail.tsx#L272)).
- **Open decisions (need owner call, defaults recommended):** (1) Instances timing — default: removed until resource data (above). (2) Global hotkey default `⌥Space` vs `⌘⇧K` — default `⌥Space`, rebindable. (3) Review comment-to-agent — default: deferred post-dogfood. (4) Light theme in beta — default: ship, it's a Settings toggle + token mirror, not new screens. (5) Fleet in nav — default: hidden until M6 pairing exists ([SPEC.md:387](../../SPEC.md#L387)).
- **What not to build:** kanban board (import Vibe/Squad users, not the metaphor, [full review](../full-competitive-ux-review.md#L507)), auto-rotation (claude-swap does it, [github.com/realiti4/claude-swap](https://github.com/realiti4/claude-swap) — Juggler stays suggest-only), checkpoint timeline (Opcode's strength, [github.com/winfunc/opcode](https://github.com/winfunc/opcode) — lineage covers Juggler's job), mobile companion (T3's, [t3.codes](https://t3.codes/) — out of scope macOS-first [SPEC.md:18](../../SPEC.md#L18)).

## 8. State matrices (normative — every cell must render)
Legend: `SK` skeleton, `EM` EmptyState + action, `BN` Banner + Retry, `RO` read-only + reason, `ST` data + `Stale` chip. “—” = state impossible by construction (say why in code comment).
| Screen | Empty | Loading | Error | Offline (backend down) | Stale (>60s) |
|---|---|---|---|---|---|
| 4.1 Inbox | `All clear…` EM | 3× SK rows | BN, rows ST | RO actions, BN | Timers frozen ST |
| 4.2 Projects | EM + Choose folder | SK rows | BN + last list ST | RO, New disabled | `Updated…` ST |
| 4.3 Project detail | Idle + launch list | SK cards | BN inline | RO launches | Meters grey ST |
| 4.4 Accounts | EM + Import | SK cards | BN | RO Add/Sign-in | Login `last checked…` |
| 4.5 Account detail | — (account exists) | SK cards | BN | RO Launch/Sign-in | Quota timestamp ST |
| 4.6 Sessions | EM + New | SK table | BN + table ST | RO Resume/Handover | Age frozen ST |
| 4.7 Session+term | Desktop placeholder | `Starting…` chip | `Failed…` + Resume | Terminal RO + BN | Scrollback + ST chip |
| 4.8 New Session | No-profiles EM→§4.16 | SK profiles | Inline + Retry | Start disabled + reason | Forecast `unknown` |
| 4.9 Handover | No-target EM→Add | Rank SK | Step error, no change | Confirm disabled | Headroom `last seen…` |
| 4.10 Usage | Token-only EM | SK cards/chart | BN | Last rollup ST | Per-card ST |
| 4.11 Cost | Tokens-only EM | SK table | BN | Last table ST | Prices banner ST |
| 4.12 Search | Recents EM | `Searching…` rows | Inline retry | Titles-only + note | `Indexing…` note |
| 4.13 Review | EM `Nothing to review` | SK rows | BN | RO Continue | Diff ST banner |
| 4.14 Notifications | `All caught up` EM | SK panel | Inline retry | Centre local-only | Counts ST |
| 4.15 Settings | — (defaults render) | SK pane | Save BN + revert | RO save + note | — (local prefs) |
| 4.16 Onboarding | None-found fresh EM | Scan SK | Step inline error | Scan cached + note | — (live scan) |
| 4.17 Tray/palette | `Set up…` EM | Recents instant | Inline note | Titles-only + note | Badge ST (= Inbox) |
Rules: (1) Error never replaces data that exists — Banner above, data below with ST. (2) Offline never logs the user out or hides accounts/projects. (3) Every EM has exactly one primary action. (4) Skeletons match final layout (no layout shift >8 px).

## 9. Focus order & ARIA per surface (testable)
- **Shell:** skip-link `Skip to panel` → rail tabs (roving) → rail list → header nav → panel. Rail splitter after list. `⌘K` from anywhere returns focus to invoker element ID (store + restore).
- **Inbox/Sessions/Review lists:** container `listbox/grid`, rows `option/row`, `aria-activedescendant` (Inbox) or roving tabindex (tables); action buttons inside row reachable by `Tab` after row, `Enter` on row = Open (never the destructive action).
- **Wizards (§4.8/4.9/4.16):** heading (`aria-describedby` step count) → step body (first field/radio) → Back → primary. Step change moves focus to step heading (`tabIndex=-1`), announces `Step 2 of 3: Which account?`.
- **Session (§4.7):** header actions → terminal (or Focus-window button) → below-strip toggles. Terminal entry/exit announced (`Terminal focused. Control-dot to exit.` — text, not sound only).
- **Dialogs/Settings:** §2.5 contract; Settings tab list `tablist` vertical (`↑↓`), pane `tabpanel` labelled by tab; switches `aria-checked` in accessible name (`Digest, on`).
- **Live regions:** one `#app-live` polite (results, clears, saves) + one `#app-alert` assertive only for crash/quota-blocked arrival while window unfocused (still batched §4.1). Terminal output never enters live regions.
- **Reduced motion:** `prefers-reduced-motion` + Settings override: breathe → static, cross-fades → instant, palette summon → fade-none, chart animations off. Test with OS setting on.

## 10. Microcopy bank (use verbatim; don't paraphrase in code review)
| Context | String |
|---|---|
| Inbox empty | `All clear — {n} sessions running, nothing waiting on you.` |
| Waiting row | `Waiting {t} — “{prompt excerpt}”` |
| Stuck row | `May be stuck? No output for {t}.` |
| Quota row | `Hit its 5-hour limit. Resets {time}.` |
| Admission Tight | `Tight — stopping “{session}” ({peak}) makes this comfortable.` |
| Admission No | `Not enough memory for another session like this. [Hand over instead] [Start anyway]` |
| Handover success | `Running on {target} now. {source} version is paused where you left it.` |
| Handover failure | `Nothing was changed. [Retry] [Cancel]` |
| Resume vs Relaunch | `Resume — same session, same account.` / `Relaunch — fresh session, same profile and folder.` |
| Signed-out launch | `Needs sign-in first — Claude's sign-in, about 30 seconds.` |
| Quota unknown | `Quota not available yet.` (never `0%`) |
| Cost estimate | `~${amount} estimated (API-equivalent).` |
| Stop confirm | `Stop “{label}”? Its transcript is kept — you can Resume later.` |
| Delete profile blocked | `{n} live sessions use this profile — stop them first.` |
| Offline banner | `Backend unavailable — showing last known state. [Retry]` |
| Update available | `{harness} {new} is available (you run {old}). The app never updates it for you. [Open release notes]` |
Voice rules: sentence case, account/project names in quotes only in confirms, times absolute + relative, `~` on every forecast/estimate, no exclamation marks, no “Oops/Looks like/Simply”.

## 11. Remaining global surfaces (complete the nav — no orphans)
### 11.1 Overview (global) — Fix, P1 (today [App.tsx](../../../ui/src/App.tsx#L431))
- **Purpose:** all-up oversight, *not* the daily home (Inbox is). Overview aggregates; it never duplicates Inbox actions. **Entry→Exit:** header `Overview` → every widget deep-links (quota → §4.10, live rows → §4.7, machine → Settings Supervisor).
- **Elements:** health strip: per-account mini-meters + reset countdowns (honest empty pre-feed) · `Needs attention` summary card (count + top 3, `View inbox →`) · live sessions list (clickable rows — today non-clickable dead end) · machine card (CPU/RAM/swap/disk gauges when resource manager lands; until then `Machine gauges arrive with the resource manager.` + session RSS list if available, never `OK` alone) · recent handover lineage (last 5, jump links) · update card (app/harness, notify-only).
```
┌─ Overview ───────────────────────────────────────────────┐
│ NEEDS 3 need you — oldest 24m              [View inbox →]│
│ QUOTA Work Max 78%·17:00  Personal 22%     [Usage →]     │
│ LIVE  billing-migration ◐ · api-refactor ● [rows → §4.7] │
│ MACHINE gauges arrive with resource manager.             │
└──────────────────────────────────────────────────────────┘
```
- **States:** first-run (1 account, 0 sessions): onboarding-resume card on top · quota pre-feed token summary only · machine pre-manager explainer state (above) · stale per-card timestamps. **Keys:** `o` from header context, rows `↑↓ Enter`. **A11y:** cards are sections with headings; summary numbers are text. **Copy:** `This Mac` (not `this-mac · healthy` jargon in body; rail footer keeps short form).

### 11.2 Instances (future resource view) — Deferred, P2
- Ship only when per-instance RSS/CPU/token attribution is real ([SPEC.md:331](../../SPEC.md#L331), supervisor footprints [SPEC.md:231](../../SPEC.md#L231)). Until then the rail slot stays Accounts (§4.4) — do not ship a filtered Sessions clone.
- When shipped: table (Instance, Kind Terminal/Desktop, Account, Project, RSS, CPU%, Tokens, Age) + `Focus window/Open terminal, Stop` + overload banners (swap thrash, RSS anomaly [SPEC.md:239](../../SPEC.md#L239)). Desktop-kind rows have no terminal action.
```
┌─ Instances (future) ─────────────────────────────────────┐
│ billing-migration Term·Work Max·payments 2.9GB 12% [Open]│
│ (slot hidden until RSS/CPU attribution is real)          │
└──────────────────────────────────────────────────────────┘
```
- **States/Copy:** empty `Nothing running.`; overload `Memory pressure — sessions may stall. [See largest]`.

### 11.3 Fleet placeholders & empty-nav policy — Remove, P0 honesty
- No `ComingSoon` screens. Pre-M6, Fleet appears only as disabled `+ New ▾ → Import machine…` with tooltip `Fleet arrives in M6 — see Roadmap`, and in Settings → Advanced as roadmap text. When M6 lands: machines as filters (not a separate board), pairing wizard (code/QR [SPEC.md:387](../../SPEC.md#L387)), cross-machine handover reuses §4.9 with T2 path warning (same binary version + same repo path required, stated pre-confirm).
- Policy for all future nav: a destination ships only with (a) real data or honest empty, (b) one primary action, (c) all five states. Otherwise it does not enter nav.

## 12. Implementation notes for the frontend engineer
- **Data needs (new/changed bridge calls; names follow [bridge.ts](../../../ui/src/lib/bridge.ts#L282) style):** `inboxList()` (waiting-sorted, snooze-aware), `quotaList()` (per account/window + tier + forecast), `costSummary(range)`, `searchQuery(q, scope)`, `notificationsList()`, `settingsGet/Patch`, `handoverPreview(session, target)` (tier + carry list before execute). Existing `sessionsList/projectsList/accountsList/profilesList` stay; add `last_activity/quota` fields to generated types via ts-rs, never hand-edit types.
- **Events:** extend `session://updated` payload with inbox delta (or add `inbox://updated`); badge counts derive from one selector so rail/tray/title cannot diverge (flow 1 gate).
- **Performance:** virtualize Sessions/Projects lists >100 rows; terminal mount must not block panel paint (lazy xterm chunk already separate [TerminalView.tsx](../../../ui/src/components/TerminalView.tsx#L1)); quota timers tick locally (1/min render, no refetch storm).
- **Testing hooks:** `data-testid` on primary actions only (`inbox-reply`, `handover-confirm`, `palette-input`); Playwright flows §6; axe on every screen both themes; manual VoiceOver + keyboard-only pass per release (Phase C exit).
- **Migration from current UI:** delete dead modal, `ComingSoon`, fabricated Overview values in Phase A PR (separate commit, screenshot diff); keep ReadyHome logic as Inbox sign-in rows (no behaviour loss for signed-out users).

## 13. Screen inventory & shortcut index (one-page builder checklist)
| # | Screen (§) | Verdict | Key backend dependency | Primary shortcut |
|---|---|---|---|---|
| 1 | Inbox/Home (§4.1) | New P0 | Status emission Stuck/Waiting/Quota | `⌘1` |
| 2 | Projects (§4.2) | Fix P0 | Branch/dirty per project | `⌘2` |
| 3 | Project detail (§4.3) | Fix P0 | Quota forecast + admission | `Enter/n/h` |
| 4 | Accounts rail (§4.4) | Fix P1 | Login expiry (metadata) | `⌘4` |
| 5 | Account detail (§4.5) | Fix P1 | QuotaList + health fields | `l/u` |
| 6 | Sessions (§4.6) | Fix P0 | Archive/search index | `⌘3` |
| 7 | Session+terminal (§4.7) | Fix P0 | Resume (T1) exists; wire UI | `⌘./⌘F` |
| 8 | New Session (§4.8) | Fix P0 | Forecast + worktree launch | `⌘N` |
| 9 | Handover (§4.9) | New P0 | `handoverPreview` + sibling copy | `⌘⇧H` |
| 10 | Usage/Quota (§4.10) | New P1 | Measured feed (M3-1/2) | `←→` |
| 11 | Cost (§4.11) | New P1 | Pricing table + rollups | `e` export |
| 12 | Search (§4.12) | New P0 | Transcript index | `⌘K→all` |
| 13 | Review (§4.13) | New P2 | Diff stat + pack preview | `j/k` |
| 14 | Notifications (§4.14) | New P1 | Digest scheduler | `⌘⇧N` |
| 15 | Settings (§4.15) | New P1 | Settings store | `⌘,` |
| 16 | Onboarding v2 (§4.16) | Fix P1 | Scan/import exist; persist step | Wizard keys |
| 17 | Tray/hotkey/palette (§4.17) | New P1 | Tray + global-shortcut caps | `⌥Space/⌘K` |
| 18 | Overview (§11.1) | Fix P1 | Aggregates only | Header |
| 19 | Instances future (§11.2) | Deferred P2 | RSS/CPU attribution | — |
Build rule: no screen merges without its row's dependency stubbed honestly (empty state) — stubbing with fake numbers fails review (§0).

### 13.1 Risks this spec creates (name them, don't discover them)
- **Inbox without emission is empty theatre.** If Stuck/Waiting detection slips, Inbox shows only Quota/crash — ship Inbox v1 scoped to emitted statuses and label the rest `Coming with stuck detection`, don't fake rows.
- **Forecast wrong = trust loss worse than no forecast.** Every forecast needs tier + `at current burn` + stale handling (§4.10); suppress forecast when confidence is Estimated *and* sample <1 h (show `Not enough history yet`).
- **Palette transcript search leaks.** Snippets must pass redaction (§4.12); add a guardrail test with a seeded secret in a fixture transcript asserting it never renders.
- **Tray badge divergence.** Single selector (§12) + flow-1 test; a wrong badge is P0, not polish.
- **Worktree surprise.** Default worktree toggle ON only when a second live session shares the branch; otherwise OFF (don't change solo users' habits silently). Branch name shown pre-launch always (§4.8).

## 14. Decision log (defaults a designer may rely on today)
| # | Decision | Default in this spec | Revisit when |
|---|---|---|---|
| D1 | Default landing | Inbox if badge>0 else last view | Dogfood shows badge usually 0 |
| D2 | Instances in rail | Removed (§3) | RSS/CPU attribution ships |
| D3 | Fleet in nav | Hidden pre-M6 (§11.3) | Pairing wizard exists |
| D4 | Overview role | Oversight only, Inbox owns action | Never merge the two |
| D5 | Worktree default | Conditional ON (§13.1) | Worktree dogfood data |
| D6 | Forecast display | Suppress if Estimated + <1h history | Calibration improves |
| D7 | Review comments | Read-only v1 (§4.13) | Dogfood demand proven |
| D8 | Global hotkey | `⌥Space`, rebindable (§4.17) | Conflict reports |
| D9 | Theme/density | Dark+Cozy default, both user-set | Light usage >20% |
| D10 | Retention | Archive 30d, delete never (opt-in) | Storage complaints |
| D11 | Notifications | Digest default, interrupt actionable only | Mute-rate telemetry (local) |
| D12 | Handover label | `Transcript-only (experimental)` until F3.4 test | Real-Claude dogfood passes |
Each default is reversible in Settings or a follow-up ADR; none requires reworking layout. If an owner overrides a default, update this table + the affected §4 screen in the same change — spec drift is how the current UI got fabricated numbers.

### 14.1 What “5× deeper” changed vs. the parent review
Parent [full review](../full-competitive-ux-review.md#L802) §UX named direction (inbox, forecast, palette, disclosure, a11y list). This file turns each into: named controls + backend calls, five rendered states per screen (§8), focus order + live-region behaviour (§9), verbatim copy (§10), funnel targets with instrumentation (§5), flow-level acceptance tests (§6), and explicit non-goals (§7). Anything still marked `future/deferred/inferred` is a deliberate omission with a trigger to build it, not an oversight.

## Appendix — traceability (claim → source)
Repo: shell/state [App.tsx](../../../ui/src/App.tsx#L1), rail [WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L1), panels [WorkPanels.tsx](../../../ui/src/screens/WorkPanels.tsx#L1), new-session [NewSession.tsx](../../../ui/src/screens/NewSession.tsx#L1), accounts [Profiles.tsx](../../../ui/src/screens/Profiles.tsx#L1), onboarding [OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L1), ready-home [ReadyHome.tsx](../../../ui/src/screens/ReadyHome.tsx#L1), terminal [TerminalView.tsx](../../../ui/src/components/TerminalView.tsx#L1), primitives/icons/form/styles above, bridge [bridge.ts](../../../ui/src/lib/bridge.ts#L1), domain [model.rs](../../../crates/juggler-core/src/model.rs#L1), spec/ADR above. Competitors: [t3.codes](https://t3.codes/), [opcode](https://github.com/winfunc/opcode), [claude-squad](https://github.com/smtg-ai/claude-squad), [conductor](https://www.conductor.build/docs/concepts/workflow), [vibe-kanban](https://github.com/BloopAI/vibe-kanban), [cc-switch](https://github.com/farion1231/cc-switch), [ccusage](https://github.com/ryoppippi/ccusage), [claude-swap](https://github.com/realiti4/claude-swap). Limits: no runtime/screen-reader test was run for this spec; contrast/focus claims need Phase C empirical pass. Unresolved backend gaps (quota feed, status emission, transcript search index, global-shortcut capability) are named per screen — layouts do not depend on inventing their data.
Reading order for implementers: §0–§3 (shell contract) →
§4.1/4.7/4.8/4.9 (daily loop) → §8–§10 (states, a11y, copy
gates) → remaining §4 screens → §5–§7 (funnel, flows, phases).
Designers: mock L2 states first; L3 tables can start as
placeholders with honest empties. Reviewers: check any new
screen mock against §8 (five states), §9 (focus), §10 (copy)
before discussing visuals — missing states fail the review
regardless of how the happy path looks.
*End of spec. Companion audits: shell/rail, panels/terminal,
onboarding/handover/settings, competitor and persona files
in this `deep/` folder.*



