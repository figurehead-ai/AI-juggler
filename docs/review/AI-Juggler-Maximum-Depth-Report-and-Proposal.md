# AI Juggler — Maximum-Depth Report and Proposal

> Front matter for the 13-part deep review assembled as
> `docs/review/AI-Juggler-Maximum-Depth-Report-and-Proposal.md`.
> Every claim below is grounded in a named deep part; this file
> adds no features of its own. Read the part before citing it
> externally — caveats and evidence levels travel with each part.
> Date of assembly: 2026-10-13. Line counts re-verified at assembly.

## How to read this report (10 minutes, 1 hour, full)

| Reader | Read first | Then | ~Time |
|---|---|---|---|
| Maintainer deciding scope | This file §Executive proposal + §Revised roadmap | 13-critic, 12-roadmap §1/§10 | 10 min |
| Implementer (frontend) | 01 + 02 + 03 audits, then 11 screen spec | 09 quota/handover design | 1 hour |
| Implementer (backend) | 08 architecture, 09 engines | 10 proposal §§1–9 | 1 hour |
| Reviewer challenging claims | 13-critic caveats K1–K10 | Spot-check part's `file:line` cites | any |

### Part map (all line counts via `wc -l`, verified at assembly)

| # | File | Lines | Question it answers |
|---|---|---:|---|
| 01 | `01-ui-shell-and-rail-button-audit.md` | 413 | What does shell/rail chrome do per button? |
| 02 | `02-ui-panels-sessions-terminal-button-audit.md` | 426 | What do panels/sessions/terminal do? |
| 03 | `03-ui-onboarding-handover-settings-audit.md` | 492 | Where do onboarding/handover/settings leak? |
| 04 | `04-competitor-t3-and-opcode.md` | 585 | T3 / Opcode — steal, beat, or avoid? |
| 05 | `05-competitor-claude-squad-and-conductor.md` | 599 | Squad / Conductor — how do they scale? |
| 06 | `06-competitor-vibe-crystal-switchers.md` | 597 | What do switcher/orphan users already expect? |
| 07 | `07-personas-and-journeys.md` | 452 | Who is this for, and where does trust break? |
| 08 | `08-architecture-data-supervisor-security.md` | 600 | Will the backend hold 10 accounts / 50 sessions? |
| 09 | `09-handover-and-quota-engine-design.md` | 542 | How should the two signature engines work? |
| 10 | `10-product-proposal-missing-features.md` | 505 | Which 20 features, in what priority? |
| 11 | `11-ux-redesign-screen-spec.md` | 454 | What exactly should each screen be? |
| 12 | `12-roadmap-metrics-risks-gtm.md` | 548 | What order, what metrics, what risks? |
| 13 | `13-critic-gaps-and-crosschecks.md` | 313 | Where do the parts contradict or overclaim? |

### Evidence language used across parts

- `file:line` (repo path) — code-read fact, not runtime-tested.
- `[NEG-EV]` / negative evidence — feature not found in docs/search;
  absence is not a vendor denial (see 04, 05, 06 appendices).
- `[composite]` — persona quotes/diaries in 07 are verbatim-style
  composites, not field interviews; validation is in 07 §11.
- `[inferred]` / `[target]` / `[judgement]` — demand or durations
  that need dogfood/interviews, flagged where they occur.
- Honesty tiers for quota (09 §Q2): Measured / Computed /
  Estimated — never blur subscription quota with USD cost.

## Executive proposal — top 15 decisions

These compress the parts; wording stays with the source part.

1. **Quota-first (Claude-Measured) before second adapter.**
   SPEC orders quota at M3 after M2 adapters; 12 §1 recommends
   Option B (quota-first, Claude-only) against the promise in
   PROPOSAL. No second adapter buys trust while `percent={0}`.
2. **Relabel handover now; rebuild before marketing.**
   V0 copies one mtime-newest JSONL and drops sibling state
   (09 D1, 03 §6); downgrade `TRANSPLANT` until H8 fixtures pass.
3. **Ship fidelity levels F0–F4 with plain labels (09 §H6).**
   Keep T1–T4 as engine internals; users need "what carries".
4. **Move transplant into `juggler-handover` (09 D3).**
   The crate is doc-only today; `commands.rs` is the wrong home.
5. **Needs-attention inbox as default landing (11 §3, 10 §2).**
   Emit `Stuck/WaitingUser/QuotaBlocked` first (model.rs:188
   modelled, never emitted); the inbox is empty without them.
6. **Suggest, never auto-rotate (09 D6, 12, ADR-0003).**
   Recommend + one click; cswap-style auto-rotation is avoided
   under Juggler's click-bound invariant (05, 06 §8).
7. **Worktree-on-launch (06 S9, 05 F1.1, 10 §4).**
   Session is `cwd`-only today; Squad/T3/Conductor prove
   worktrees are table stakes for parallel work.
8. **No fabricated numbers anywhere (01 F-, 03 §7).**
   Remove hardcoded `percent={0}`, "Machine OK" and Fleet
   placeholders; Usage/Fleet ship honest empty states or teasers.
9. **Destructive guards + undo as a system (01 §0, 02 §0).**
   Stop, Delete profile, and handover chips all fire on one
   click today; toast/confirm/undo must land together. Note the
   C1 tension (13 §3): 01 prefers confirm, 02 prefers undo —
   the maintainer must rule (see 13 ship-gate).
10. **⌘K palette + keyboard system (10 §§10/18, 11 §13).**
    Only `⌘B`/`⌘N` exist; T3/Conductor set the ⌘K bar (04, 05).
11. **Explainable routing score, never silent switching (09 Q6).**
    `0.45 headroom + 0.20 affinity + 0.15 fidelity + 0.10 cost
    + 0.10 calm`, shown to the user before any one-click action.
12. **USD ledger is always Estimated (09 D7, 10 §6).**
    ccusage-style Computed USD is the reference (06 §6);
    subscription quota and USD cost stay separate views.
13. **Trust blockers before charts (10 Finding 3).**
    Account health, backup/restore, and audit log gate adoption
    for Priya/Sara even where their RICE scores rank lower.
14. **Review/diff queue is dogfood-gated, read-only v1 (10 §11).**
    Conductor's diff→PR→merge cockpit is the steal target (05
    §6), but Juggler demand for it is inferred, not validated.
15. **90-day plan starts only after D1/D2 rulings (12 §10–11).**
    Stop-confirm (C1) and canonical migration ruling (C3, 13 §3)
    are maintainer decisions; durations marked `[target]` until
    those rulings land. North star: Weekly Juggling Loops Completed.

## Persona summary (from 07; quotes are composites)

**P1 Maya — Solo Indie Multi-Maxer.** Pays for her own stack
(~$450–620/mo in 07's order-of-magnitude estimate) and runs 4–8
sessions/day across 2–3 repos; her quota wall is a momentum wall.
She pays, reluctantly, for anything that keeps shipping when one
account dies mid-feature and tells her the truth about the cost.

**P2 Devon — Staff Engineer, 10+ Parallel.** Employer-funded,
10–20 sessions/day; his wall is throughput and review capacity.
Waiting sessions are invisible today, so admission ("which needs
me?") matters more than charts; he contributes OSS time, not money.

**P3 Priya — Agency/Consultant, Multi-Client.** 6–14 sessions/day,
$600–1,200/mo pass-through billing; her wall is the budget wall.
Per-client separation and redaction are existential, not cosmetic —
she pays (via clients) for provenance, audit, and renewal artefacts.

**P4 Jonas — OSS Maintainer, Bursty & Broke.** $20–100/mo, 3–10
bursty sessions; a free-tier user who contributes code. Refugee
windows from Vibe (Bloop shutdown 2026-04-10) and Crystal
(deprecated Feb 2026) make him a beta-recruit cohort (12 §5).

**P5 Sara — Enterprise-Locked Developer.** 2–6 approved sessions,
no visible spend, procurement-only payment; she cannot self-pay.
Her gate is security review: redaction, audit, and credential
handling are admission tickets, not features she ranks by RICE.

**P6 Alex — Student/Hobbyist Cost-Optimiser.** $20–38/mo, 1–4
sessions/day, free-only and fast to churn; GLM-credit behaviour
(zcode research) proves the segment. He converts only on honest,
cheap forecasting — never on dashboards.

Cross-persona truth (07 §0): quota-blindness is #1 for 5/6, and
nobody pays for charts — they pay for provenance, ROI, and
renewal artefacts. Three walls (momentum / throughput / budget)
need different UI: time-to-exhaust, per-session burn, sessions-left.

## Competitor verdict summary

| Competitor | Verdict (source part) | Juggler move |
|---|---|---|
| T3 Code | Multi-harness surface, no named accounts (04 §1) | Steal worktree/PR lifecycle + ⌘K; beat on honest quota |
| Opcode (ex-Claudia) | Deep but dormant since Oct 2025, AGPL (04 §§4–7) | Steal checkpoint/MCP ideas, never code |
| Claude Squad | `1 task = 1 tmux + 1 worktree` (05 §3) | Steal atomicity; avoid `-y` autoyes |
| Conductor | Diff→PR→merge cockpit; pricing now free (05 §§6–7) | Steal cockpit pattern; do not envy closed polish |
| Vibe Kanban / Crystal | Orphaned parallel-worktree users (06 §§3–4, 9) | Capture via worktrees/import before quota pitch |
| cc-switch / ccusage / cswap / aisw | Switching stakes S1–S10 (06 §8) | Meet S1–S9; S10 (live-session supervision) is unclaimed |

Headline: no competitor owns named multi-account profiles with
upstream quota (04, 05, 06 agree independently). Juggler's wedge
is intact but time-boxed, because T3/Squad/Conductor already
ship the parallel-work surface Juggler lacks.

## P0 checklist (ship-gate; sources in brackets)

- [ ] Relabel handover `F1 Transcript-only (experimental)`; set
  `TRANSPLANT=false` until H8 fixtures pass [09 D1/D4, 03 §6]
- [ ] Emit `Stuck/WaitingUser/QuotaBlocked` status events [11, 07]
- [ ] Claude-Measured quota feed (statusline `rate_limits`) [06 §2, 09 Q1]
- [ ] Remove fabricated `percent={0}` / "Machine OK" / Fleet fakes [01, 03]
- [ ] Toast + confirm/undo for Stop / Delete / handover [01, 02]
- [ ] Fix PID-reuse adoption risk (journal compares PID only) [08 F2]
- [ ] Worktree-on-launch [10 §4, 05]
- [ ] Settings surface replacing the absent SPEC §9.2 page [03 §7]
- [ ] Delete dead `NewSessionModal` duplicate [01, 02]
- [ ] Rule on C1 (Stop confirm vs undo) and C3 (canonical V5) [13 §3]

## Revised roadmap at-a-glance (from 12 §2; durations are targets)

| Phase | Theme | Entry gate | Exit gate |
|---|---|---|---|
| Phase 0 | Honesty fixes (P0 checklist above) | This report accepted | No fabricated numbers; handover relabelled |
| v0.1-beta | Claude quota + inbox + honest handover | Phase 0 green | Dogfood loops (Maya/Devon cohorts of 5) |
| v0.2 | Worktrees, ⌘K, ledger, reset queue | Beta metrics (12 §6) | Return-on-reset works end-to-end |
| v0.5 | Review queue (read-only), MCP/settings sync | Dogfood demand proven | Priya/Sara gates pass security review |
| v1.0 | Fleet teaser → full, Codex/ZCode depth | H8 dogfood matrix green | 10/50 scale test measured, not estimated |

GTM in one line (12 §8): Show HN + five migration guides, beta
recruited in cohorts of 5 from Vibe/Crystal refugee pools; cloud
relay pricing rejected (12 §7). The 13-week week-by-week plan is
12 §11 — it does not start until maintainer D1/D2 rulings land.
# 01 — UI Shell & Rail: Button-by-Button Audit (Deep)

> Scope: every interactive element in `ui/src` at audit time.
> Method: full read of all 27 files under `ui/src` (see §1),
> cross-checked against `docs/review/full-competitive-ux-review.md`
> and `docs/internal/agent-review-prompt.md`. No runtime/browser
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
[full-competitive-ux-review.md](full-competitive-ux-review.md#L1).

## 0. Executive verdict (read this first)

1. The shell has exactly two real state-changing verbs from
   chrome — **New** and **Stop/Handover** — and both fail
   silently or modally: `refresh()` swallows errors
   ([App.tsx](../../ui/src/App.tsx#L101))
   and handover/sign-in use `window.alert`
   ([App.tsx](../../ui/src/App.tsx#L371),
   [App.tsx](../../ui/src/App.tsx#L384)).
   A supervision tool whose failures are invisible fails F2.1.
2. The rail promises three entities and delivers two:
   Instances is Sessions filtered live/recent through the same
   `SessionRow` ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L295)),
   contradicting the SPEC distinction cited in F2.2. Users
   cannot tell what an Instance *is* from any button label.
3. There is no undo, no confirm, and no toast anywhere.
   Destructive **Stop** and **Delete profile** fire on one
   click ([WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx#L100),
   [Profiles.tsx](../../ui/src/screens/Profiles.tsx#L239));
   every competitor with a review loop — Vibe Kanban's
   inline-comment-to-agent, Conductor's Checks tab —
   gates destructive steps. See §9 for the competitor bar.
4. Keyboard story is two shortcuts (`⌘B`, `⌘N`,
   [App.tsx](../../ui/src/App.tsx#L205))
   plus Escape-by-wrapper; tabs, splitter, and modals do not
   implement their ARIA patterns (§8). `⌘K` does not exist.
5. Two of four global-nav destinations are dead ends
   (`ComingSoon`, [App.tsx](../../ui/src/App.tsx#L525))
   and Overview fabricates numbers (`percent={0}`,
   [App.tsx](../../ui/src/App.tsx#L510)).
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
| WorkEmpty | No selection, no global, ReadyHome passed ([App.tsx:393](../../ui/src/App.tsx#L393)) | New session (B-31) | No |
| ReadyHome | `global===null` + any account not `logged_in` + not skipped ([App.tsx:194](../../ui/src/App.tsx#L194)) | Sign in (B-20), Skip (B-21) | No, but re-blocks every launch until session-skip (F2.7) |
| Overview | Global nav Overview ([App.tsx:334](../../ui/src/App.tsx#L334)) | None — live rows are not clickable (B-14) | **Yes** |
| Accounts (Profiles) | Global nav Accounts ([App.tsx:330](../../ui/src/App.tsx#L330)) | Launch/Login/Profile modals | No |
| Usage | Global nav Usage ([App.tsx:338](../../ui/src/App.tsx#L338)) | None | **Yes — ComingSoon** |
| Fleet | Global nav Fleet ([App.tsx:340](../../ui/src/App.tsx#L340)) | None | **Yes — ComingSoon** |
| ProjectPanel | Rail project click ([App.tsx:342](../../ui/src/App.tsx#L342)) | Open session, Launch-with | No |
| SessionPanel | Rail session/login click ([App.tsx:356](../../ui/src/App.tsx#L356)) | Stop, Handover, Open project | Partial — no Resume (F2.3) |
| OnboardingWizard | Zero accounts auto-open ([App.tsx:94](../../ui/src/App.tsx#L94)) or Add account | Close (B-40), Done | No |
| NewSessionModal (live) | `⌘N` / New buttons / folder drop ([App.tsx:409](../../ui/src/App.tsx#L409)) | Profile click launches, Close | No |
| NewSessionModal (dead) | Never rendered — duplicate in ([WorkPanels.tsx:355](../../ui/src/screens/WorkPanels.tsx#L355)) | — | **Yes — dead code (F2.8)** |

Priority consequence: 3 of 11 surfaces cannot move the user
forward. Prior review F2.4 already flags Usage/Fleet; this
audit adds Overview's non-clickable live list (B-14).

## 3. Global chrome — App shell (`App.tsx`)

### B-01 · Rail brand button (collapse toggle)
- Loc: [App.tsx:259](../../ui/src/App.tsx#L259) · Label: Juggler icon, `title=Expand/Collapse (⌘B)` → toggles `railCollapsed`, persists `aij.rail.collapsed` ([App.tsx:61](../../ui/src/App.tsx#L61)).
- Backend: — · Success: rail animates 60px↔width · Error: none possible · Disabled: never.
- A11y: native `<button>`, tooltip via `title` only; no `aria-expanded`, no `aria-controls`, icon is decorative but unlabelled when expanded (text `AI Juggler` sits beside it, :269).
- **Verdict: Fix.** Redesign: keep position; add `aria-expanded={ !collapsed }`, `aria-controls="workflow-rail"`, tooltip component (not `title`) reading `Collapse rail · ⌘B`. Shortcut stays `⌘B`. No confirm/undo needed.

### B-02 · Global nav ×4 (Overview / Accounts / Usage / Fleet)
- Loc: [App.tsx:306](../../ui/src/App.tsx#L306), data at [App.tsx:44](../../ui/src/App.tsx#L44) · Label = item label → `setGlobal(id)`, unmounts detail panel.
- Backend: — (Overview then calls `dashboard_stats`, B-14) · Success: active pill `bg-surface sheen` · Error: none · Disabled: never, even for empty Usage/Fleet.
- A11y: plain `<button>`s in a `<div>`; no `aria-current`, no `nav` landmark, no arrow-key roving; focus ring from global CSS only.
- **Verdict: Fix (Overview/Accounts), Remove (Usage/Fleet until real).** Redesign: wrap in `<nav aria-label="Global">`, add `aria-current="page"` on active; **remove Usage and Fleet from nav** and re-add with badge `M3`/`M6` disabled + tooltip `Usage arrives in M3 — quota first` only when the milestone ships a read model. Exact new labels: `Overview`, `Accounts`. Do not keep dead destinations for symmetry — Claude Squad and T3 both keep nav to live surfaces only ([t3.codes](https://t3.codes/), prior §T3).

### B-03 · Header “New” (primary)
- Loc: [App.tsx:321](../../ui/src/App.tsx#L321) · Label: `New` + terminal icon → `setNewSession(true)` (same as `⌘N`).
- Backend: — until profile click (`launch_session` via bridge:289) · Success: modal opens · Error: none at this step · Disabled: never — opens even with zero accounts/profiles, deferring failure into the modal (B-24 empty state).
- A11y: uses shared `Button` ([primitives.tsx:31](../../ui/src/components/primitives.tsx#L31)); no shortcut hint in label/tooltip; `data-tauri-drag-region={false}` wrapper (:305) is correct.
- **Verdict: Fix.** Redesign: label `＋ New session`, tooltip `New session · ⌘N`; when `profiles.length===0`, click routes to Accounts/Add-account instead of an empty modal, with inline hint. Keep `⌘N`.

### B-04 · Keyboard: ⌘B / ⌘N (window listener)
- Loc: [App.tsx:206](../../ui/src/App.tsx#L206) · Action: `⌘/Ctrl+B` toggle rail, `⌘/Ctrl+N` new session; listener added once but `toggleRail` dep omitted (stale-closure risk, deps `[]` at :219).
- Backend: — · Success: as B-01/B-03 · Error: none · Disabled: fires even while typing in inputs/modals — `⌘N` inside NewProfile name field still opens NewSession.
- A11y: undiscoverable (only in `title` attrs); no `aria-keyshortcuts` anywhere in repo (grep-verified).
- **Verdict: Fix.** Redesign: guard `event.target` editable check; add `aria-keyshortcuts="Meta+N"` / `"Meta+B"` on B-03/B-01; add `⌘K` palette (Add, §10), `⌘1/2/3` rail tabs, `Esc` closes topmost layer only. Document in `?` shortcuts popover (Add).

### B-05 · Rail footer status (this-mac · healthy / dot)
- Loc: [App.tsx:276](../../ui/src/App.tsx#L276) · Label: green dot + `this-mac · healthy` + `v{version}` → **not a button**; `title="this-mac · healthy"` when collapsed.
- Backend: version from `app_info` (bridge:136) · Success/Error: static text — `healthy` is hardcoded, never derived from any probe.
- A11y: colour-only health signal; collapsed state is a dot with a `title`.
- **Verdict: Fix.** Redesign: make it a button `Machine: OK · v0.x` opening Overview/Machine; health computed from supervisor probe; text label always paired with dot (never colour-alone, UX.8). If probe absent, render `Machine: unknown` — honest empty state, never fabricated `healthy` (F2.4).

### B-06 · Context title + “Claude Code” chip (non-interactive)
- Loc: [App.tsx:302](../../ui/src/App.tsx#L302) · Shows `contextTitle` + static `Chip Claude Code`. No action.
- **Verdict: Fix.** Redesign: chip becomes harness switcher (disabled, tooltip `Codex arrives in Phase 2`) only when a second adapter registers; until then keep static but add `aria-label="Harness: Claude Code (only harness)"`. Prevents the multi-harness promise reading as shipped (F5.5).

### B-07 · Drag region header
- Loc: [App.tsx:298](../../ui/src/App.tsx#L298) `data-tauri-drag-region`. Double-click/maximise behaviour is OS-default; no custom button.
- **Verdict: Keep.** Redesign: none, except ensure B-02/B-03 keep `data-tauri-drag-region={false}` (:305) after any refactor — regression test: buttons clickable at window top edge.

## 4. WorkflowRail — tabs, lists, resize (`WorkflowRail.tsx`)

### B-08 · Rail tabs ×3 (Projects / Instances / Sessions)
- Loc: [WorkflowRail.tsx:126](../../ui/src/components/WorkflowRail.tsx#L126), data [WorkflowRail.tsx:8](../../ui/src/components/WorkflowRail.tsx#L8) · Label = tab label+icon → `onTab(id)`; list below swaps.
- Backend: — · Success: accent pill + list swap · Error: none · Disabled: never; counts/badges absent — no live/needs-attention count on any tab.
- A11y: `role=tablist/tab` + `aria-selected` (:124,:132) but no `aria-controls`, no `tabpanel`, no Arrow/Home/End keys; labels vanish below 244px (icon-only + `title`, :106).
- **Verdict: Fix + Remove (Instances tab).** Redesign: two tabs `Projects` / `Sessions` until Instances gains resources (F2.2); add leading `Inbox` tab with count badge (Add, §10). Implement full tabs pattern: `aria-controls`, arrow keys, `aria-keyshortcuts="Meta+1..3"`. Tooltip on icon-only: `Sessions · ⌘3`.

### B-09 · Rail “New session” (bottom CTA)
- Loc: [WorkflowRail.tsx:200](../../ui/src/components/WorkflowRail.tsx#L200) · Label `New session` (icon-only collapsed, `title=New session (⌘N)`) → `onNewSession`.
- Backend: — · Success: modal · Error: none · Disabled: never (same zero-profile deferral as B-03).
- A11y: `focus-visible:ring` present (:204); shortcut only in collapsed `title`.
- **Verdict: Keep + Fix.** This duplicates B-03; keep rail as primary (daily loop lives here), demote header New to `⌘N` hint only or remove header duplicate. Add `aria-keyshortcuts`. Behaviour otherwise identical to B-03 redesign.

### B-10 · Project row (rail list)
- Loc: [WorkflowRail.tsx:259](../../ui/src/components/WorkflowRail.tsx#L259) · Label: project name + `N accts · M runs` + live dot → `onSelectProject(id)`.
- Backend: — (data from `projects_list`, bridge:300) · Success: `bg-surface` + accent bar · Error: none · Empty: text `Launch a session…` (:249), not actionable.
- A11y: native button, focus ring; status is dot-only; truncated name has no tooltip (`title` absent).
- **Verdict: Fix.** Redesign: add `title={name + ' · ' + root}`, right-side text chip for live state (`● Live` / `idle`), `aria-current` when selected. Empty state becomes button `＋ Start first session · ⌘N`.

### B-11 · SessionRow (Instances + Sessions lists)
- Loc: [WorkflowRail.tsx:407](../../ui/src/components/WorkflowRail.tsx#L407) · Label: session label + raw `status` string + `app` tag for desktop → `onSelectSession(id)`.
- Backend: — (`sessions_list`, bridge:307) · Success: selected bg · Error: `failed` = red dot only · Disabled: never.
- A11y: status conveyed by dot colour + lowercase raw status text (:434); no `Stuck/Waiting/QuotaBlocked` chips exist because backend never emits them (F2.5).
- **Verdict: Fix.** Redesign: second line becomes `Chip` with text (`Running · 12m`, `Needs you · waiting 4m`, `Quota blocked · resets 17:00`); add waiting-time sort + Needs-attention section on top. Tooltip = label + project + account.

### B-12 · Collapsed-rail live dots (icon rail)
- Loc: [WorkflowRail.tsx:168](../../ui/src/components/WorkflowRail.tsx#L168) · Up to 6 live sessions as 8px dots, `title={label}` → select session.
- Backend: — · Success: brighter dot when selected · Error: failed sessions invisible (live filter, :165) · Empty: blank column.
- A11y: `title`-only identity; dot has no text alternative; 32px target (`size-8`) below 40px guidance.
- **Verdict: Fix.** Redesign: each dot becomes avatar button with initials + `aria-label="Open {label}, running"`; failed/needs-you get `!` badge; target `size-9` min. Keep cap 6 + `+N` overflow button (Add).

### B-13 · Resize handle (drag + double-click collapse)
- Loc: [WorkflowRail.tsx:215](../../ui/src/components/WorkflowRail.tsx#L215) · Drag resizes 208–360px persisted `aij.rail.width` (:19); double-click → `onToggleCollapsed`.
- Backend: — · Success: accent hairline while dragging · Error: none · Disabled: hidden when collapsed.
- A11y: `role=separator` + `aria-label` (:218) but **not focusable, not keyboard-operable**; no `aria-valuenow/min/max`.
- **Verdict: Fix.** Redesign: `tabIndex=0`, `aria-valuemin=208 aria-valuemax=360 aria-valuenow={width}`, ArrowLeft/Right ±8px (Shift ±32), `Home` reset 264. Tooltip `Drag to resize · double-click to collapse`.

### B-14 · Overview live-instance rows + quota rings (display-only)
- Loc: rows [App.tsx:498](../../ui/src/App.tsx#L498), rings [App.tsx:513](../../ui/src/App.tsx#L513) · Rows show label/kind/status chip but are `<div>`s — **not clickable**, despite copy `Select one in the rail` (:481).
- Backend: `dashboard_stats` (bridge:321) fetched then discarded (`void stats`, [App.tsx:442](../../ui/src/App.tsx#L442)); counts computed locally from props. Rings hardcoded `percent={0}`.
- Success/Error/Empty: empty state (:487) is honest; rings are not — `0%` reads as measured quota.
- A11y: rings are `<svg>` without `role=img`/`aria-label` ([primitives.tsx:165](../../ui/src/components/primitives.tsx#L165)); numbers have no text equivalent.
- **Verdict: Fix.** Redesign: rows become buttons → `selectSession(id)`; rings render `— No data · M3` empty state (dashed ring, text) until Measured quota lands; add `role="img" aria-label="5 hour quota: no data yet"`. Delete `void stats` — either render stats or stop fetching.

## 5. ReadyHome — get-ready gate (`ReadyHome.tsx`)

### B-20 · Per-account “Sign in” (primary, ×N)
- Loc: [ReadyHome.tsx:72](../../ui/src/screens/ReadyHome.tsx#L72) · Label `Sign in` → `onSignIn(account)` → `terminal_spawn_login` (bridge:379) → login terminal selected ([App.tsx:378](../../ui/src/App.tsx#L378)).
- Success: terminal opens, rail jumps to Sessions · Error: `window.alert` at [App.tsx:384](../../ui/src/App.tsx#L384) · Disabled: never; no busy state — double-click spawns twice (guard exists only in `openLoginTerminal` dedupe, [App.tsx:165](../../ui/src/App.tsx#L165)).
- A11y: shared Button; plan chip + dot + text state is good (not colour-only).
- **Verdict: Keep + Fix.** Redesign: label `Sign in · {label}` for screen readers (`aria-label`), busy spinner `Opening…` disabled during spawn, error → toast with Retry (never alert). This flow beats most competitors' first-run (prior F2.6) — protect it.

### B-21 · “Skip for now — show projects” / “Continue to Projects →”
- Loc: [ReadyHome.tsx:83](../../ui/src/screens/ReadyHome.tsx#L83) · Text button → `sessionStorage aij.ready.skipped=1` ([App.tsx:387](../../ui/src/App.tsx#L387)).
- Backend: — · Success: gate dismissed for session · Error: none · Disabled: never.
- A11y: underline-on-hover text button; touch target ~16px tall.
- **Verdict: Fix.** Redesign: label when partial: `Skip — 1 of 3 ready`; make skip sticky per-profile (`Don't ask again for this account` checkbox) per F2.7, keep re-block only for unknown/expired. Min height 32px.

## 6. Detail panels (`WorkPanels.tsx`)

### B-22 · SessionPanel “Stop” (live only)
- Loc: [WorkPanels.tsx:100](../../ui/src/screens/WorkPanels.tsx#L100) · Label `Stop` → dynamic `terminal_kill` (bridge:403).
- Success: `onStopped()` → `refresh()` re-renders · Error: **none** — `await` uncaught, failure silent · Disabled: only rendered when `live`; no busy state, repeat clicks re-kill.
- A11y: shared Button `secondary`; no confirm, no undo; adjacent text `ran {age} · scrollback replayed` (:96) is informative.
- **Verdict: Fix (P0 for error handling).** Redesign: label `Stop session`, confirm sheet `Stop “{label}”? Running work will end.` with `Stop` (danger, `⌘.`) / `Cancel` (Esc); busy `Stopping…`; error → toast Retry; success → toast `Stopped` + **Undo (relaunch same profile+cwd, 10s)**. Competitor bar: Claude Squad's `c` checkout explicitly pauses vs kills ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)) — copy the explicitness.

### B-23 · Handover target buttons (×≤6, “Continue on another account”)
- Loc: [WorkPanels.tsx:153](../../ui/src/screens/WorkPanels.tsx#L153) · Label `{account_label}` (+`· sign in first`) → `sessionHandover` (bridge:464, `session_handover`).
- Success: refresh + select new session ([App.tsx:365](../../ui/src/App.tsx#L365)) · Error: `window.alert` ([App.tsx:371](../../ui/src/App.tsx#L371)) · Disabled: `!ready` targets disabled with `title` explanation (:158); self excluded (:146); `slice(0,6)` silently hides 7th+ profile.
- A11y: native buttons, disabled reason only in `title`; no fidelity/headroom info before committing (F2.3).
- **Verdict: Fix (replace with wizard).** Redesign: single button `Continue on another account…` opening 3-step wizard (Target ranked by headroom → Fidelity tier + carry list → Confirm). Each target row: `label · 72% headroom · resets 17:00 · usual for this project`. Overflow: searchable list, never silent slice. Error → inline step error + Retry, lineage shown after (`Handed over from …`).

### B-24 · Project chip (session header → project)
- Loc: [WorkPanels.tsx:85](../../ui/src/screens/WorkPanels.tsx#L85) · Label: folder icon + project name → `onOpenProject(id)`.
- Backend: — · Success: panel swaps · Error: none · A11y: hover-only affordance, no `aria-label`.
- **Verdict: Keep + Fix.** Redesign: add `aria-label="Open project {name}"`, chevron affordance, tooltip with full root path.

### B-25 · ProjectPanel “On it now” row
- Loc: [WorkPanels.tsx:236](../../ui/src/screens/WorkPanels.tsx#L236) · Whole card-row button → open active session.
- Backend: — · Success: session panel · Empty: `No live instance — free to juggle.` text (:251).
- A11y: nested `Chip` inside `<button>` (interactive-adjacent semantics muddy); `open` hint good.
- **Verdict: Keep + Fix.** Redesign: keep; add `aria-label="Open live session {label}"`; empty state gains inline `Launch ↓` anchor scrolling to B-26 list.

### B-26 · ProjectPanel “Launch with another account” rows (×≤5)
- Loc: [WorkPanels.tsx:262](../../ui/src/screens/WorkPanels.tsx#L262) · Label: account · profile · model → `launchSession(profile.id, project.root)` via [App.tsx:348](../../ui/src/App.tsx#L348) (`launch_session`, bridge:289).
- Success: refresh + select new session · Error: **uncaught** — `onLaunch` await has no try/catch in App or panel · Disabled: `!ready` disabled + `sign in first` chip; `slice(0,5)` hides rest.
- A11y: disabled reason in `title` only; rows are buttons (good).
- **Verdict: Fix.** Redesign: label suffix `· 72% free` when quota exists; per-row busy spinner; error toast; `Show all N accounts` expander; usual-account row pinned + badged `Usual`.

### B-27 · ProjectPanel session-history rows
- Loc: [WorkPanels.tsx:303](../../ui/src/screens/WorkPanels.tsx#L303) · Row button (dot+label+age+status chip) → open session. No Resume/Relaunch distinction for ended sessions (F2.3).
- Backend: — · Empty: `Nothing recorded yet.` (:299).
- A11y: adequate; age via `since()` ([WorkPanels.tsx:33](../../ui/src/screens/WorkPanels.tsx#L33)) is relative-only, no absolute tooltip.
- **Verdict: Fix + Add.** Redesign: ended rows get trailing `Resume` (same account, `⌘R`) and `Relaunch` buttons; live rows keep Open only. Add `title={absolute date}`. This is T1 resume — mechanically free via `--resume` (prior F3.5).

### B-28 · WorkEmpty “New session”
- Loc: [WorkPanels.tsx:346](../../ui/src/screens/WorkPanels.tsx#L346) · Primary `New session` → `onNew` (same modal as B-03/B-09).
- **Verdict: Keep.** Redesign: add sub-actions `Open project…` and `⌘K Search` once palette exists; keep single primary today. Empty copy is honest — keep verbatim.

### B-29 · Dead `NewSessionModal` duplicate (WorkPanels)
- Loc: [WorkPanels.tsx:355](../../ui/src/screens/WorkPanels.tsx#L355) (443-line file's second half) · Typed-path `<input>` (:401), profile buttons (:411), `Close` (:436). Never imported (App imports from `screens/NewSession`, [App.tsx:12](../../ui/src/App.tsx#L12)).
- Harm: contradicts `never typed paths` daily-loop rule (agent prompt :53-54); stale `profilesList` during render (:366 pattern also in ProjectPanel :209).
- **Verdict: Remove.** Delete :354-443 entirely (F2.8). No redesign — the live modal (B-30) supersedes it.

## 7. Launch & accounts modals/screens

### B-30 · NewSession (live): recent-project tiles, Choose folder, profile rows
- Loc: tiles [NewSession.tsx:119](../../ui/src/screens/NewSession.tsx#L119), picker [NewSession.tsx:146](../../ui/src/screens/NewSession.tsx#L146), profiles [NewSession.tsx:174](../../ui/src/screens/NewSession.tsx#L174), Close [NewSession.tsx:220](../../ui/src/screens/NewSession.tsx#L220).
- Action/Backend: tile → `setDirectory(root)`; picker → `pickDirectory()` native dialog (bridge:468); profile click → `launch_session` (bridge:289). Drag-drop prefill via Tauri event ([App.tsx:125](../../ui/src/App.tsx#L125)).
- Success: modal closes, Sessions tab + new session selected ([App.tsx:418](../../ui/src/App.tsx#L418)) · Error: inline `text-danger` (:211) incl. preview-mode message · Disabled: `busy||!ready`; `Loading…` text (:207); recents capped 6 (:55).
- A11y: Escape via wrapper `onKeyDown` (:79) only if focus inside; no `role=dialog`, no focus trap/return; selected tile has accent border (not colour-only — good).
- **Verdict: Keep + Fix.** Redesign: `role="dialog" aria-modal="true" aria-label="New session"`, initial focus on first recent, trap + return-focus to invoker; profile row adds headroom forecast `· ~40m left` (WOW-b); footer primary becomes `Start session` enabled only when directory+ready profile chosen (today launch is the row click itself — keep row-click but add explicit footer confirm for keyboard clarity). Shortcut: `⌘↵` start.

### B-31 · Profiles: Add account / Empty-state Add account
- Loc: header [Profiles.tsx:89](../../ui/src/screens/Profiles.tsx#L89), empty [Profiles.tsx:167](../../ui/src/screens/Profiles.tsx#L167) → opens `OnboardingWizard` as modal (:123).
- Backend: — · Success: wizard · Error: screen-level banner (:95) · Loading: bare `Loading…` (:76).
- **Verdict: Keep + Fix.** Redesign: skeleton cards while loading; `Add account` gets `aria-haspopup="dialog"`. Empty copy is good — keep.

### B-32 · Profiles: per-profile Launch / Delete (trash) / + Profile / Log in
- Loc: Launch [Profiles.tsx:249](../../ui/src/screens/Profiles.tsx#L249), Delete [Profiles.tsx:239](../../ui/src/screens/Profiles.tsx#L239), `+ Profile` [Profiles.tsx:213](../../ui/src/screens/Profiles.tsx#L213), `Log in` [Profiles.tsx:209](../../ui/src/screens/Profiles.tsx#L209).
- Backend: Launch `launch_session` (note: success calls `openTerminal` with session_id as terminal id, :66 — conflates session/terminal ids); Delete `profile_delete` (bridge:279) **no confirm, no undo, no catch** (:111); `+ Profile` → B-33; Log in → B-34.
- Success: Launch opens terminal panel · Error: banner `setError` (:49) except Delete (silent) · Disabled: Launch never disabled by login state (can launch logged-out profile).
- A11y: Delete is icon-only with `title="Delete profile"`; hover-reveal `opacity-60→100` (:238) hides it from keyboard/touch until hover.
- **Verdict: Fix (Delete = P0 data-trust).** Redesign: Delete → confirm `Delete profile “{name}”? Sessions keep history.` + toast Undo (re-create) 10s; always-visible at `opacity-100` on focus-within; `aria-label="Delete profile {name}"`. Launch disabled when `!loggedIn` with chip `Sign in first` (mirror B-30). `+ Profile` label → `New profile`.

### B-33 · NewProfileModal: inputs + Cancel / Create profile
- Loc: modal [Profiles.tsx:278](../../ui/src/screens/Profiles.tsx#L278), fields via `Field/Input` ([form.tsx:8](../../ui/src/components/form.tsx#L8)), submit [Profiles.tsx:329](../../ui/src/screens/Profiles.tsx#L329).
- Backend: `profile_create` (bridge:270) · Success: close+refresh · Error: inline (:324), empty-name guard (:294) · Disabled: `busy` on Create only; Cancel always live.
- Inputs: Name (autoFocus, placeholder `Deep work`), Model (`opus`), Default directory typed path (`~/Projects/app`) — typed path contradicts zero-typing rule; no picker button.
- A11y: `Field` renders `<label>` wrapping (good); no `aria-describedby` for hint/error; Modal lacks dialog semantics (B-36).
- **Verdict: Fix.** Redesign: directory field gains `Choose…` picker button (reuse `pickDirectory`); error gets `role="alert"`; Create label `Create profile · ⌘↵`; Enter submits.

### B-34 · LoginModal: Sign in inside / Open claude.ai / Copy command / Re-check
- Loc: [Profiles.tsx:338](../../ui/src/screens/Profiles.tsx#L338); buttons :382/:386/:389/:400. Command from `account_login_command` (bridge:247).
- Backend: Sign-in `terminal_spawn_login` (:379); Web `login_open_web` (:240, **no mock guard — throws in preview**); Copy `navigator.clipboard`; Re-check `account_login_state` (:254) then refresh (result discarded, :148).
- Success: terminal opens + modal closes · Error: Sign-in `window.alert` (:367); others silent · Disabled: `starting` on primary only; `Copied` feedback 1.5s (:395) good.
- Copy bug: footer copy at :404 says terminal `opens at the bottom` — stale (terminal is now the detail panel). Fix copy to `opens in the main panel`.
- **Verdict: Fix.** Redesign: Re-check shows result chip `Still signed out — finish in terminal, then re-check`; Web button disabled in preview with tooltip; all errors → toast. Order: primary Sign-in, secondary Re-check, tertiary Copy/Web.

### B-35 · OnboardingWizard: scan rows, plan tiles, footer nav, login quartet
- Loc: scan row [OnboardingWizard.tsx:167](../../ui/src/screens/OnboardingWizard.tsx#L167), fresh link (:200), label input (:235), plan tiles (:248), close `✕` (:130), Back/Create/Continue/Done (:387-431), login buttons (:300-327).
- Backend: `environment_scan` (:208) auto on mount; `account_import`/`account_create` (:235/:170); login quartet mirrors B-34 (`terminal_spawn_login`, `login_open_web`, clipboard, `account_login_state`).
- Success: step advance + summary checklist (:350) — best success feedback in app · Error: inline danger (:266) · Disabled: Create disabled until label non-empty (:402) — exemplary; scan loading is dot+text (:154).
- A11y: close is unlabelled `✕` text button (no `aria-label`); progress dots (:118) have no `aria` step text; plan tiles are toggle buttons without `aria-pressed`.
- **Verdict: Keep + Fix (protect per F2.6).** Redesign (surgical only): close → icon button `aria-label="Close wizard"`; add visually-hidden `Step {n} of 4: {name}`; plan tiles `aria-pressed`; login primary gets busy state (currently spawns on every click, :303 unguarded). Freeze all other interaction.

### B-36 · Shared Modal / Field / Input / Select / Button primitives
- Loc: Modal [form.tsx:54](../../ui/src/components/form.tsx#L54), Field/Input/Select (:8/:28/:38), Button [primitives.tsx:31](../../ui/src/components/primitives.tsx#L31).
- Modal: Escape via wrapper keydown (:68); close button **has** `aria-label="Close"` (:86) — the only labelled close in app. No `role=dialog`/`aria-modal`/trap/return-focus; backdrop click does **not** close.
- Button: `disabled:opacity-40 pointer-events-none` (:43) — disabled buttons vanish from tab order with no reason announced; no `loading` prop, every caller hand-rolls busy text.
- Select (:38) is exported but **never used** (grep-verified) — dead primitive.
- **Verdict: Fix + Remove (Select).** Redesign: Modal gains dialog semantics + trap + backdrop-click-close + `aria-labelledby`; Button gains `loading` (spinner + `aria-busy`, keeps width) and `title`-driven disabled-reason pattern; delete `Select` until a real select ships.

### B-37 · TerminalView (xterm host)
- Loc: [TerminalView.tsx:29](../../ui/src/components/TerminalView.tsx#L29) · No buttons; keystrokes → `terminal_write` (bridge:389), resize → `terminal_resize` (:393), snapshot+attach on mount (:58).
- Success: live stream · Error: snapshot/attach failures uncaught (silent blank terminal) · Empty (preview): writes fake transcript (:66) — honest `(preview…)` label, good.
- A11y: xterm canvas; auto-focus on mount (:96) can steal focus from rail navigation; no `aria-label` on host div (:115).
- **Verdict: Fix.** Redesign: host `role="application" aria-label="Terminal: {title}"`; focus only on explicit panel activation, not every mount; attach failure → inline banner `Terminal disconnected · Reconnect` button (Add).

### B-38 · Icons & dead code
- `IconDashboard` ([icons.tsx:22](../../ui/src/components/icons.tsx#L22)) unused; `hasLiveBackend` voided at [NewSession.tsx:229](../../ui/src/screens/NewSession.tsx#L229); `stopSession` (bridge:314) exported, unused (Stop uses `terminalKill`).
- **Verdict: Remove** all three (or wire `stopSession` as B-22 backend and delete dynamic import). Dead affordances erode audit trust.

## 8. Cross-cutting systems: feedback, keyboard, a11y

### Toasts / alerts / banners — current state (no toast system exists)

| Channel | Where | Problem |
|---|---|---|
| `window.alert` ×3 | [App.tsx:371](../../ui/src/App.tsx#L371), [App.tsx:384](../../ui/src/App.tsx#L384), [Profiles.tsx:367](../../ui/src/screens/Profiles.tsx#L367) | Modal, unstyled, blocks terminal; no Retry/Undo |
| Silent swallow | `refresh()` catch [App.tsx:113](../../ui/src/App.tsx#L113); Stop (B-22); Delete (B-32) | Stale data presented as fresh (F2.1) |
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
| `⌘B` / `⌘N` | Rail / New ([App.tsx:206](../../ui/src/App.tsx#L206)) | Keep + editable-guard (B-04) |
| `Esc` | Closes modal only if focus inside wrapper (NewSession :79, form :68); wizard has **no Esc** | Close topmost layer; wizard Esc = close with unsaved guard |
| Arrows in tabs/splitter | None | Tabs Arrow/Home/End; splitter arrows (B-08/B-13) |
| `⌘K`, `⌘1-3`, `⌘R`, `⌘.`, `⌘↵` | Absent | Palette, tab jump, Resume, Stop-confirm, submit (Add) |
| `Tab` order | DOM order; disabled buttons skipped silently | Keep; add skip-link `Skip to panel` (Add) |

### Accessibility debt (code-read, extends UX.8)

- Contrast: faint `#5b6474` on canvas ([styles.css:24](../../ui/src/styles.css#L24)) at 10.5px rail subtext likely <4.5:1 — raise to `#8b95a7` (muted) or 11.5px. Not empirically verified (no runtime test).
- Motion: `breathe` ([styles.css:90](../../ui/src/styles.css#L90)) + glows ignore `prefers-reduced-motion` — add media query disabling animation.
- Colour-only: rail dots (B-10/B-11/B-12), footer health (B-05), quota ring colour ([primitives.tsx:178](../../ui/src/components/primitives.tsx#L178)) — pair every dot/ring with text (redesigns above).
- Focus: global ring ([styles.css:76](../../ui/src/styles.css#L76)) is strong; modal trap/return is the gap (B-36).
- Targets: collapsed dots 32px (B-12), Skip link ~16px (B-21) — bring interactive minima to 36px.

### Dead ends & fabricated data (must-fix list)

1. Usage/Fleet `ComingSoon` ([App.tsx:525](../../ui/src/App.tsx#L525)) — remove from nav (B-02); if kept, copy must name milestone + what lands (`Usage · M3: 5h/weekly quota per account`), not generic `M2+`.
2. Quota rings `percent={0}` (B-14) and `Machine OK / Fleet 1` StatCards ([App.tsx:461](../../ui/src/App.tsx#L461)) — honest empty states until probes exist.
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


# 03 — Onboarding, Import/Login, Handover & Settings: Step-by-Step Audit

> Slice: first-run funnel, vendor-login handholding, session handover,
> and the Usage / Fleet / Settings placeholder surface.
> Method: full read of `OnboardingWizard`, `ReadyHome`, `NewSession`,
> `Profiles` (+ `LoginModal`), `WorkPanels` handover buttons, `App`
> shell, `bridge.ts`, Tauri commands, Claude scan/import/login code,
> and SPEC §7–§9 + §12 against the prior review.
> Prior review: `docs/review/full-competitive-ux-review.md` (F2.1–F2.8,
> F3.1–F3.5, F4.1, UX.5, UX.8). This file goes deeper: every button,
> every field, every failure branch, and the proposed replacement
> flows. No runtime or browser test was run; all UI claims are
> code-read. Competitor claims cite URLs in §10.
> Verdict in one line: **onboarding is the strongest funnel in the
> app and still leaks at login; handover is a single unguarded click
> wearing a wizard's name; Usage/Fleet/Settings promise three
> control surfaces and ship zero controls.**

---

## 0. Funnel map — what the code actually does

```
Cold start (App.tsx:87-116)
  ├─ fetchAppInfo() + onAppReady() ──────────────► version chip
  ├─ accountsList() == 0 && !dismissed ──────────► OnboardingWizard (auto)
  ├─ projectsBackfill() ─► refresh() (4 parallel lists, errors swallowed)
  └─ selection == none
       ├─ any account logged_out && !session-skip ─► ReadyHome (gate)
       └─ else ───────────────────────────────────► WorkEmpty

OnboardingWizard (OnboardingWizard.tsx:49) — 4 steps, linear, no branch back to scan after identity
  detect ──(tap card)──► identity ──(Import/Create)──► login ──(Continue / "I'll sign in later")──► summary ──(Done)──► close
    │                        │                            │                                        │
    └─ "None of these" ──────┘                            ├─ Sign in inside AI Juggler (PTY)       └─ onAccountCreated + onClose
         (fresh)                                          ├─ Open claude.ai (browser)
                                                          ├─ Copy command (clipboard)
                                                          └─ Re-check (login_state probe)

Daily re-entry points to the same login machinery (three doors, one engine):
  ReadyHome "Sign in" (ReadyHome.tsx:72) ─► App.tsx:378-385 terminalSpawnLogin
  Profiles "Log in" (Profiles.tsx:208-212) ─► LoginModal (Profiles.tsx:338-411) ─► terminalSpawnLogin
  Wizard login step (OnboardingWizard.tsx:300-310) ─► terminalSpawnLogin
  All three ─► crates/juggler-app/src/terminal.rs:56-73 spawn_login_terminal_impl
            ─► adapter.login_spec (claude_code.rs:354-372, empty argv + CLAUDE_CONFIG_DIR)

Handover (today — no wizard):
  SessionPanel profile chips (WorkPanels.tsx:144-180, max 6, disabled if logged out)
    └─ click ─► App.tsx:365-373 onHandover ─► bridge sessionHandover (bridge.ts:460-465)
              ─► commands.rs:1070-1079 session_handover ─► 1088-1194 impl ─► --resume (claude_code.rs:294-296)
              └─ failure ─► window.alert (App.tsx:371) — the only feedback channel
```

Key structural facts that shape everything below:

- The wizard is a **fixed overlay modal**, not a route: `fixed inset-0
  z-50` with an unlabelled `✕` close
  ([OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L108)).
  Dismissal is persisted as `aij.onboarding.dismissed`
  ([App.tsx](../../ui/src/App.tsx#L94)).
  There is no "resume where I left off": closing mid-import discards
  `label`/`plan` state because the component unmounts.
- Auto-open fires **once per dismissal**, not once per zero-account
  state: if the user closes with zero accounts and relaunches, the
  wizard does not reappear (`App.tsx:94-99`). The only re-entry is
  Accounts → Add account ([Profiles.tsx](../../ui/src/screens/Profiles.tsx#L89)).
  For a zero-knowledge user this is a silent dead end.
- `HARNESS` is hard-coded to `"claude-code"`
  ([OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L18)).
  SPEC §12.1 and ADR-0006 describe a harness-generic scan
  ([docs/SPEC.md](../SPEC.md#L414));
  the wizard cannot onboard Codex/ZCode even when adapters land.
- Every backend call in the funnel returns a **bare string error**
  (`AppError` serializes via `to_string()`,
  [error.rs](../../crates/juggler-app/src/error.rs#L52)).
  The UI therefore cannot distinguish "retryable IO" from "fatal
  corrupt" — it can only print the string. This single constraint
  explains most of the error-recovery gaps in §2–§5.

---

## 1. Step 1 — Detect: auto-scan

### 1.1 What happens, control by control

| Control | Code | Behaviour | Drop-off / failure risk |
|---|---|---|---|
| (auto) scan on mount | `OnboardingWizard.tsx:63-71` | `environmentScan()` fires in `useEffect`; no user action, no timeout, no cancel | If the Tauri command hangs (large home), the spinner is forever; no timeout or retry exists |
| Scanning placeholder | `OnboardingWizard.tsx:153-157` | Pulsing dot + "Scanning your machine…" | No progress detail (how many dirs checked); no skeleton of expected result shape |
| Result card (per install) | `OnboardingWizard.tsx:166-195` | Whole card is a `<button>`; tap → `identity` with `label` pre-filled from basename | Card shows only `notes[0]` (`:189`) — the second note (keychain caveat, origin detail) is dropped; see §1.3 |
| Login chip on card | `OnboardingWizard.tsx:182-186` | `logged_in` → success "signed in", else neutral "not signed in" | Binary only; no "login unknown / probe failed" state — probe failure is indistinguishable from logged-out |
| Bytes hint | `OnboardingWizard.tsx:190-192` | `humanBytes(found.bytes)` when `> 0` | `estimate_bytes` caps at depth 2 + 20k entries (`claude_environment.rs:300-327`); a 3 GB transcript tree can display as "48 MB" and then fail the 2 GiB import ceiling later — expectation mismatch at the worst moment |
| "None of these — add a fresh account" | `OnboardingWizard.tsx:199-213` | Text link; clears `importing`, goes to `identity` | Discoverability is weak (underlined text link under cards); zero-state copy differs ("Continue with a fresh account →") but the affordance weight is identical — no primary CTA on this step at all |
| Close `✕` | `OnboardingWizard.tsx:130-136` | `onClose` → sets dismissed flag | No `aria-label`; no confirm when a scan result was already selected; misclick loses the funnel |

### 1.2 What the scan actually probes (backend truth)

`environment_scan` iterates registered adapters and calls
`scan_environment()`
([commands.rs](../../crates/juggler-app/src/commands.rs#L347)).
The Claude implementation assembles candidates from four origins —
vendor default `~/.claude`, `$CLAUDE_CONFIG_DIR` override, t3.codes
`ClaudeCodeProfiles` children, and home `​.claude*` pattern matches —
then canonicalizes, dedupes, filters by `is_claude_config_dir`
(any of `.credentials.json`, `settings.json`, `projects`, …),
and sorts default → logged-in → most-recent
([claude_environment.rs](../../crates/juggler-adapters/src/claude_environment.rs#L50)).
Three audit findings the prior review did not surface:

1. **The `managed` flag is computed and then ignored by the UI.**
   `DiscoveredInfo.managed` is populated
   ([commands.rs](../../crates/juggler-app/src/commands.rs#L200))
   and the scan appends "Already managed by AI Juggler." to notes
   (`claude_environment.rs:98-100`), but the wizard renders every card
   identically and lets the user tap a managed dir into `identity`.
   The refusal only arrives **after** name + plan entry, as a raw
   error string from `import_account` ("already managed by AI
   Juggler — launch it from Profiles instead",
   `claude_environment.rs:175-179`). Cost: two steps of user effort
   spent to learn a fact the first screen already knew. Fix: disable
   managed cards in Detect with the note inline.
2. **Scan failure is impossible by construction — and therefore
   invisible.** `environment_scan` "never fails — an empty list is a
   valid nothing-found" (`commands.rs:336-343`). An unreadable home,
   a permission-denied `ClaudeCodeProfiles`, and a genuinely clean
   machine all render the same "No existing Claude setups found"
   empty state (`OnboardingWizard.tsx:158-163`). The user with a
   broken-permissions install is told they have nothing to import.
3. **Legacy shared-keychain installs get a caveat the UI truncates.**
   When the pre-2.1.56 shared keychain exists, ManagerTool installs
   gain a third note about re-login
   ([claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L230)).
   The wizard shows `notes[0]` only, so the one note that predicts
   the login-step surprise is never seen before the tap.

### 1.3 Detect wireframe — today vs. required

```
TODAY (OnboardingWizard.tsx:141-214)              REQUIRED (delta only)
┌──────────────────────────────────┐            ┌──────────────────────────────────┐
│ ◈ Import ".claude"          ●○○○ ✕│            │ ◈ Add a Claude account   ●○○○ [✕]│
│ First, let's look at what you... │            │ Found 3 setups · scanned just now │
│ ┌──────────────────────────────┐ │            │ ┌──────────────────────────────┐ │
│ │ 👤 ~/.claude      [signed in]│ │            │ │ ~/.claude          [signed in]│ │
│ │ Claude Code's standard...48MB│ │            │ │ Standard install · 48 MB      │ │
│ └──────────────────────────────┘ │            │ │ History + settings will copy. │ │
│ ┌──────────────────────────────┐ │            │ │ Login stays (Keychain ✓)      │ │
│ │ 👤 ~/.claude-side [not signed│ │            │ └──────────────────────────────┘ │
│ │ A separate Claude config...  │ │            │ ┌──────────────────────────────┐ │
│ └──────────────────────────────┘ │            │ │ ~/…/ClaudeCodeProfiles/acct-2 │ │
│ None of these — add fresh        │            │ │ [managed — open in Profiles]  │ │
└──────────────────────────────────┘            │ └──────────────────────────────┘ │
  ✕ unlabelled, notes[1..] hidden,              │ [ Import selected ]  [ Add fresh ]│
  no primary CTA, managed not disabled          └──────────────────────────────────┘
                                                  managed disabled, all notes shown,
                                                  explicit primary CTA per path
```

Drop-off verdict for Detect: **low on the happy path** (prior F2.6), **high** for already-managed-only, permission-broken-home, and shared-keychain t3 users.

---

## 2. Step 2 — Identity: name + plan

### 2.1 Control-by-control

| Control | Code | Behaviour | Risk |
|---|---|---|---|
| Context banner (import) | `OnboardingWizard.tsx:220-225` | Shows source path; appends history/login sentence by probe | Path is raw absolute (`font-mono`); no truncation strategy for deep t3 paths — layout break on long paths |
| Explainer (fresh) | `OnboardingWizard.tsx:227-230` | "fully isolated — never touches the others" | Accurate per `prepare_managed_dir` 0700 + marker (`claude_code.rs:389-403`); no mention that a *fresh* account still needs the `claude` binary (see §5.1) |
| Label input | `OnboardingWizard.tsx:233-242` | Free text, `autoFocus`, placeholder examples; pre-filled with basename on import | No max length, no duplicate-label check; two accounts both labelled "Work" are indistinguishable in every picker downstream (rail, handover chips, NewSession) |
| Plan selector | `OnboardingWizard.tsx:20-25, 244-264` | 4 radio-cards: Not sure / Free / Pro / Max 20×; default `null` ("Not sure") | Missing Max 5× (the common paid tier); plan is stored verbatim (`PlanTier::new`, `commands.rs:410`) but never validated or used for quota math yet — user effort with zero visible payoff, and no "you can change this later" affordance on the cards themselves (only the Not-sure hint implies it) |
| Error line | `OnboardingWizard.tsx:266` | Raw `String(cause)` in `text-danger` | Backend strings are developer English ("destination already exists", adapter messages); no mapping to recovery actions |
| Back | `OnboardingWizard.tsx:386-390` | Returns to Detect; `importing`/`label` preserved | Good — the only reversible step transition in the wizard |
| Import/Create CTA | `OnboardingWizard.tsx:398-407` | Disabled iff `busy` or label empty; label switches Import/Create; busy text "Setting up…" | No progress for large imports (a 1.9 GiB copy is a single blocking Tauri call, `commands.rs:392-434`); "Setting up…" for 30 s reads as hung; no cancel |

### 2.2 The import click — what "Import account" actually runs

`createTheAccount` (`OnboardingWizard.tsx:82-100`) branches:
import → `accountImport` → `account_import_impl`
([commands.rs](../../crates/juggler-app/src/commands.rs#L392));
fresh → `createAccount` → `create_account` (`commands.rs:280-302`).
Both create the account row **and** a default `Main` profile inside
the same storage lock (`commands.rs:413-425`), fulfilling SPEC
§12.1 step 3 ("immediately launchable",
[docs/SPEC.md](../SPEC.md#L423)).
Findings beyond the prior review:

- **Duplicate import of the same source succeeds silently.** The
  onboarding test asserts re-import works and only managed-dir
  import is refused
  ([onboarding.rs](../../crates/juggler-app/tests/onboarding.rs#L128)).
  There is no source-path dedupe: importing `~/.claude` twice yields
  two managed accounts with identical history and (by default) the
  same basename-derived label. Downstream, handover chips and
  NewSession rows become ambiguous. The wizard needs a Detect-level
  "already imported as «Work»" badge, which requires the backend to
  remember import provenance — today `Account` stores no source path
  ([model.rs](../../crates/juggler-core/src/model.rs#L93)).
- **Failure leaves a partially-created managed dir.** Import writes
  the marker first ("even a partially imported dir is identifiable
  as ours", `claude_environment.rs:204-205`), then copies file by
  file with no rollback. A disk-full mid-copy returns an error to
  Identity, but the half-copied dir now exists on disk, is marked
  managed, and will be **refused** on retry ("destination already
  exists" cannot occur since each attempt mints a fresh
  `unique_suffix` dir — `commands.rs:403` — so retries leak orphan
  managed dirs instead). No cleanup, no "retry resumes" story.
- **Ceiling failure is discovered late and worded for engineers.**
  `collect_files` fails before any copy when the tree exceeds
  2 GiB (`claude_environment.rs:378-384`), with the message
  "configuration is larger than the 2 GiB import ceiling — add a
  fresh account instead". The wizard shows it verbatim under the
  label field. The user who waited through a scan that displayed a
  depth-capped byte estimate (§1.1) gets no per-folder breakdown
  (e.g. "projects/ is 2.4 GiB") and no "import settings only, skip
  transcripts" partial-import option. SPEC §12.1 promises a ceiling
  ([docs/SPEC.md](../SPEC.md#L422))
  but no degraded-import path; this is the single largest
  recoverable drop-off in the funnel.

### 2.3 Read-only import — proof audit

The invariant "source untouched" is the best-proven claim in the
slice:

- `import_with_cap` canonicalizes the source, refuses non-config
  and managed sources, requires `dest` to not exist, collects the
  full file list **before** writing, skips the five cache names,
  then copies (`claude_environment.rs:148-253`).
- The integration test snapshots the source tree (path → size)
  before/after and asserts byte-identity, plus launchability of the
  copy (`onboarding.rs:61-143`).
- The copy never opens source files for writing anywhere in the
  call graph; the only source-side operations are `read_dir`,
  `metadata`, and `copy` (read).

Two honest caveats the UI copy overstates:

1. "The original is never touched" (wizard Detect copy,
   `OnboardingWizard.tsx:148-149`) is true for file *contents and
   metadata*, but the scan itself runs `security
   find-generic-password` probes and `canonicalize` — harmless, yet
   the stronger claim the Summary repeats ("was not modified",
   `OnboardingWizard.tsx:362-368`) should stay scoped to the import,
   not imply zero reads.
2. **§8 vs. code contradiction (prior F4.1) bites here, not just in
   docs.** SPEC §8 states credentials are "never read, copied, or
   stored" ([docs/SPEC.md](../SPEC.md#L310)),
   while `import_claude_config` byte-copies `.credentials.json`
   when present and then chmods the copy 0600
   (`claude_environment.rs:224-230`). On file-credential installs
   the imported account is silently logged in; on keychain installs
   it is not. The wizard's Login step handles both (see §3), but the
   Detect card's "signed in" chip describes the **source's** login,
   not the copy's future login — for keychain users the chip
   over-promises. The fix is copy, not just a SPEC amendment:
   Detect should read "signed in here — you'll sign in once more
   after import" whenever the probe is keychain-bound.

---

## 3. Step 3 — Login: vendor-login handholding

### 3.1 Control-by-control (wizard instance)

| Control | Code | Behaviour | Risk |
|---|---|---|---|
| Signed-in success pane | `OnboardingWizard.tsx:273-280` | Green dot + `login_note` or generic text | Reached from either live `loginState` or stale `account.login_state` snapshot (`:273`); the snapshot can claim "ready" for an account whose keychain entry was since revoked |
| Command display | `OnboardingWizard.tsx:295-297` | Shows `CLAUDE_CONFIG_DIR='…' claude` from `account_login_command` (`commands.rs:325-334`) | Correct and copyable, but rendered as decoration — users cannot tell it is the fallback for their own terminal vs. the embedded path |
| **Sign in inside AI Juggler** (primary) | `OnboardingWizard.tsx:300-310` | `terminalSpawnLogin` → PTY, then `openLoginTerminal` selects it in the rail | **No error handling**: `await` with no `try/catch` — a missing `claude` binary (`AdapterError::InvalidLaunch`, `claude_code.rs:355-358`) becomes an unhandled rejection; the button appears dead. Compare `Profiles.tsx:367` and `App.tsx:384`, which at least `window.alert` |
| Open claude.ai | `OnboardingWizard.tsx:311-313` | `loginOpenWeb(HARNESS)` → opener plugin (`commands.rs:525-546`) | Opens the generic login page, not an OAuth hand-off tied to the CLI flow; no guidance that the browser login and the CLI login are the same Anthropic account step ("sign in in the browser tab that opens, then return here") |
| Copy command | `OnboardingWizard.tsx:314-324` | Clipboard + 1.5 s "Copied" | `navigator.clipboard` can reject (permissions/focus); rejection is unhandled and "Copied" never shows — silent failure with no fallback (no selectable text alternative) |
| Re-check | `OnboardingWizard.tsx:325-327, 102-106` | `accountLoginState` probe + `onAccountCreated` refresh | Probe is metadata-only and correct (`claude_code.rs:332-344`: non-empty `.credentials.json` or namespaced keychain existence, never contents). But the logged-out hint (`:330-335`) only appears after a failed re-check; first-time users get no "what to do in the terminal" steps (type nothing? wait for browser? paste code?) |
| Continue / "I'll sign in later" | `OnboardingWizard.tsx:408-419` | Primary CTA doubles as skip; never disabled | Honest escape hatch (SPEC §12.1 step 4 allows it), but the label flip depends on `loginState` only — after a successful embedded login the user must still click Re-check before the CTA reads "Continue"; nothing auto-detects PTY exit → re-probe |
| Back | `OnboardingWizard.tsx:391-395` | Returns to Identity | The account **already exists** at this point (`createTheAccount` ran); Back implies undo that does not happen — re-entering Identity and clicking again creates a second account |

### 3.2 The same login, three doors — consistency audit

| Aspect | Wizard (`OnboardingWizard.tsx:270-339`) | ReadyHome (`ReadyHome.tsx:11-93`) | Profiles LoginModal (`Profiles.tsx:338-411`) |
|---|---|---|---|
| Entry | Auto (wizard step) | Gate screen when any account logged out (`App.tsx:194-198`) | Account card "Log in" (`Profiles.tsx:208-212`) |
| Primary action | Sign in inside AI Juggler | Sign in (per-account row) | Sign in inside AI Juggler |
| Web fallback | Open claude.ai | — (none) | Open claude.ai |
| Copy fallback | Copy command | — | Copy command |
| Re-check | Re-check button | — (relies on refresh after PTY exit, `App.tsx:137-145`) | Re-check login |
| Error surface | Unhandled rejection | `window.alert` (`App.tsx:384`) | `window.alert` (`Profiles.tsx:367`) |
| Post-login detection | Manual Re-check only | `onTerminalExit` → `refresh()` re-probes all accounts | Manual Re-check; modal stays open |
| Skip | "I'll sign in later" | "Skip for now" (session-only, `App.tsx:387-390`) | Close `✕` |

Findings:

- **ReadyHome is a gate with amnesia.** `showReadyHome` is true
  whenever *any* account is logged out and the session skip is unset
  (`App.tsx:194-198`). The skip is `sessionStorage` (`App.tsx:77-79`),
  so every app relaunch re-blocks the daily loop until the user
  re-skips — exactly prior F2.7. Worse, ReadyHome lists **all**
  accounts (ready ones included, `ReadyHome.tsx:44-79`) with no
  filter, search, or "only show accounts that need action", and its
  per-account Sign in buttons have no busy/error state at all.
- **Terminal-exit is the only auto-detection, and it is wired to the
  wrong screens.** `onTerminalExit` marks the login terminal ended
  and refreshes (`App.tsx:137-145`), which eventually flips
  ReadyHome/Profiles chips. The wizard — the screen where first-run
  users actually log in — does not subscribe; it depends on the
  manual Re-check. First-run success therefore requires: complete
  OAuth in PTY → notice PTY ended → find Re-check → click → click
  Continue. Two of those five steps are undiscoverable.
- **`login_spec` runs bare `claude` with no login subcommand.**
  `login_spec_inherent` builds empty argv + `CLAUDE_CONFIG_DIR` and
  the account dir as cwd (`claude_code.rs:354-372`); the vendor's
  interactive first-run *is* the login flow. That is spec-correct
  (SPEC §4.1, ADR-0003 "we spawn; the user completes"), but the UI
  never says so: no "Claude will ask you to sign in — follow its
  prompts; this window is Claude, not AI Juggler" framing beyond one
  muted sentence (`OnboardingWizard.tsx:285-286`). Users who have
  never run `claude` meet a third-party TUI with zero scaffolding.
- **No harness-missing path exists anywhere in the funnel.**
  `discover()` can report `NotFound` with searched paths
  (`claude_code.rs:126-137`), but no onboarding screen calls it.
  A user without `claude` installed completes Detect → Identity →
  account creation (which succeeds — it only makes a dir), then
  hits Login where every path fails: embedded spawn errors, web
  login succeeds but authorizes nothing local, copy-command fails
  in their terminal. The funnel needed a Step 0: "Claude Code isn't
  installed — install it / show me how" with the searched-paths
  detail. Today that user churns at the last step with maximum
  sunk effort.

### 3.3 Login wireframe — required delta

```
┌─ Sign in to finish "Work" ────────────────────────┐
│ This is Claude's own sign-in. AI Juggler never    │
│ sees your credentials.                            │
│ ┌───────────────────────────────────────────────┐ │
│ │ 1  Click "Sign in" — Claude opens below       │ │
│ │ 2  Follow Claude's prompts (browser opens)    │ │
│ │ 3  Return here — we detect it automatically   │ │
│ └───────────────────────────────────────────────┘ │
│ CLAUDE_CONFIG_DIR='…/claude-code/…' claude  [Copy]│
│ [ Sign in inside AI Juggler ] [ Open claude.ai ]  │
│ ○ Waiting…  ● Signed in ✓ (auto-detected)         │
│ [ Continue ]              I'll sign in later      │
└───────────────────────────────────────────────────┘
  numbered steps, live status line, auto re-probe on
  PTY exit + 2 s poll while step is visible
```

---

## 4. Step 4 — Summary, and what happens after Done

| Element | Code | Audit |
|---|---|---|
| Ready line + plan chip | `OnboardingWizard.tsx:344-348` | Good: label + plan confirmed |
| Import stats line | `OnboardingWizard.tsx:350-361` | `files_copied` + `humanBytes` + skipped-cache count — the only place import scope is ever quantified; counts come from `ImportOutcome` (`claude_environment.rs:246-252`) |
| "Original not modified" | `OnboardingWizard.tsx:362-369` | True for contents (proven, §2.3); keep |
| "Launch via New session" | `OnboardingWizard.tsx:370-374` | Text-only directions; no **Launch now** button — the funnel ends one click short of the aha moment (first terminal). Highest-leverage single-button add in the slice |
| Preview-mode note | `OnboardingWizard.tsx:377-379` | Honest mock disclosure via `hasLiveBackend()` (`bridge.ts:46-48`); good pattern, used only here |
| Done | `OnboardingWizard.tsx:420-431` | `onAccountCreated` + `onClose`; lands on rail/WorkEmpty or ReadyHome |

Post-wizard the account is launchable, but the user saw no project picker, no quota expectation, and no invitation to add the **second** account the juggling premise needs (title is singular, `OnboardingWizard.tsx:79`; no "Add another" loop-back) — the funnel optimizes for N=1 and leaves N=2 to Accounts.

---

## 5. Edge-case matrix — funnel

| # | Case | Backend truth | UI today | Required |
|---|---|---|---|---|
| E1 | No `claude` binary | `discover()` NotFound (`claude_code.rs:126-137`) never called by wizard | Fails at Login, all paths (§3.2) | Step-0 install gate with searched paths + install link |
| E2 | No existing setups | Scan returns `[]` (`commands.rs:341-343`) | Dashed empty box + fresh link (`OnboardingWizard.tsx:158-163`) | Same + reassurance that fresh needs Claude installed (ties E1) |
| E3 | Partial import (disk full / copy error mid-tree) | Marker written first, no rollback (`claude_environment.rs:204-233`) | Raw error at Identity; orphan managed dir leaked | Cleanup-on-failure or "incomplete import — remove / resume" recovery card |
| E4 | Duplicate import (same source twice) | Allowed; asserted in test (`onboarding.rs:128-142`) | Two identical accounts, same label | Detect badge "already imported as X"; confirm-before-duplicate |
| E5 | Import already-managed dir | Refused (`claude_environment.rs:175-179`) | Refusal only after Identity entry | Disable in Detect (§1.2) |
| E6 | Corrupt / non-config dir picked via scan | Filtered by `is_claude_config_dir` (`claude_environment.rs:294-296`); import double-checks | Cannot occur via scan; raw error if raced | Keep double-check; map error to plain language |
| E7 | Source > 2 GiB | Fail-fast before copy (`claude_environment.rs:378-384`) | Verbatim engineer string; no breakdown, no partial option | Per-folder sizes + "settings-only" degraded import |
| E8 | Source deleted/moved between Detect and Import | `canonicalize` fails → `dir_error` (`claude_environment.rs:167-169`) | Raw OS error string | "That setup moved — Rescan" with one-click return to Detect |
| E9 | Keychain-locked login (macOS) | `login_note` explains re-login (`claude_environment.rs:236-244`) | Shown in Login (`OnboardingWizard.tsx:287-291`) — good; but Detect chip over-promised (§2.3) | Fix Detect copy; keep Login note |
| E10 | Offline during login | Web login fails in browser; PTY `claude` fails in-terminal | No offline detection or copy anywhere in funnel | "You're offline — sign-in needs network; your account is saved, sign in later from Ready home" + dismiss-to-summary |
| E11 | Login PTY closed without completing | Terminal marked ended (`App.tsx:139-142`); account stays logged_out | Wizard still waits on manual Re-check; no "terminal closed — reopen?" prompt | On PTY exit while wizard is on Login: inline reopen + re-probe |
| E12 | Label collision / empty-ish label | Only empty-trim blocked (`OnboardingWizard.tsx:402`); backend accepts any label (`commands.rs:285-302`) | Duplicate labels propagate to all pickers | Uniqueness warning (non-blocking) + auto-suffix suggestion |
| E13 | Wizard closed mid-import | Blocking invoke; close button always live (`OnboardingWizard.tsx:130-136`) | Import continues backend-side; UI unmounted; result lost (no account-created refresh until reopen) | Disable close while `busy`, or confirm "import continues — find it in Accounts" |
| E14 | Second account onboarding | Re-entry via Profiles Add account (`Profiles.tsx:89-92`) reuses same wizard | Works, but wizard has no memory of account #1 (no "second account" framing, no plan pre-fill) | "Add another" loop from Summary (§4) |

---

## 6. Handover — today, button by button

### 6.1 The entire UI (there is no wizard)

Entry is up to six profile chips under "Continue on another account" ([WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx#L139)); logged-out targets disabled ("· sign in first", `:149-172`), ready targets fire immediately — no confirmation or preview — via `sessionHandover` ([bridge.ts](../../ui/src/lib/bridge.ts#L460)) → `session_handover_impl` ([commands.rs](../../crates/juggler-app/src/commands.rs#L1088)). Footer claims "transcript copied, then resumed" (`WorkPanels.tsx:184`) with no tier/size/loss disclosure; failure is only `window.alert` ([App.tsx](../../ui/src/App.tsx#L371)).

### 6.2 Backend truth vs. SPEC §7 promise

SPEC requires: targets listed **with fidelity tier + quota headroom**,
freeze source, lineage, source remains resumable
([docs/SPEC.md](../SPEC.md#L287)).
Code does: newest-JSONL-by-mtime lookup ignoring the session's own
`transcript_ref` (`commands.rs:1120-1133` vs.
[model.rs](../../crates/juggler-core/src/model.rs#L415)),
cwd mangled by `replace('/','-')` (`commands.rs:1121`) instead of the
transcript's recorded `cwd` (the unambiguous decode the same file
already implements at `commands.rs:1048-1060`), single-file
`std::fs::copy` (`commands.rs:1142-1146`) while Claude is potentially
still appending (no freeze/stop, no torn-line check), then
`--resume <id>` (`claude_code.rs:294-296`) with no collision handling
despite SPEC's `--fork-session`/rewrite requirement
([docs/SPEC.md](../SPEC.md#L280)).
Lineage is one status flip to `HandedOver` (`commands.rs:1166-1169`)
plus a continuation row (`:1171-1187`) — no pack, no A→B→A chain
([docs/SPEC.md](../SPEC.md#L302)),
no return path. Capabilities already over-advertise:
`TRANSPLANT|FORK` are declared (`claude_code.rs:111-120`) while
`crates/juggler-handover/src/lib.rs:1-13` contains zero functions
and the adapter trait has no export/prime methods
([adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L26)).
Concrete failure modes: wrong-session pick under parallel activity,
torn final JSONL line, dotted/long-cwd mangle miss, target-ID
collision, silent sibling loss (separate subagent files, file-history,
todos — inline `isSidechain` lines do survive inside the main file,
prior F3.4). The golden-path test proves only the stub happy path
([golden_path.rs](../../crates/juggler-app/tests/golden_path.rs#L93)).

### 6.3 Proposed multi-step wizard (replaces chips; SPEC §7.2 takeover)

Steps: **Source → Target → Fidelity preview → Redaction → Diff/confirm
→ Progress → Done (lineage + rollback)**. Every step is reversible
until Confirm; execution stays one explicit click (automation policy,
[docs/SPEC.md](../SPEC.md#L263)).

```
Step 2 Target              Step 3 Fidelity           Step 5 Confirm
┌───────────────────────┐  ┌───────────────────────┐  ┌───────────────────────┐
│ Hand over "api-fix"   │  │ Tier T2 · Transplant  │  │ Copy 1 transcript     │
│ From Work Max (live●) │  │ ✓ conversation (128k) │  │ + 3 sibling files     │
│ ○ Personal  80% free  │  │ ✓ subagents (inline)  │  │ 4.2 MB → Personal     │
│ ● Team      35% free  │  │ ✗ MCP state, running  │  │ Source stays resumable│
│ ○ Old       sign in → │  │   processes stay here │  │ [Back] [ Hand over ]  │
│ [Back]    [Continue]  │  │ [Back]    [Continue]  │  └───────────────────────┘
└───────────────────────┘  └───────────────────────┘  Step 7 Done: lineage
  ranked by headroom,        plain-language carry      A ──► B · Return on
  usual-account marked       list + pack preview       reset [Resume A]
```

Button spec (abridged): Source shows live/frozen state + "Stop &
hand over" vs "Copy while running (risk: torn tail)" choice — never
silent. Target disables quota-blocked/logged-out rows with reasons,
shows headroom forecast (depends on M3 quota, prior WOW-b). Fidelity
renders tier badge + carry/drop list + `PACK.md` preview
([docs/SPEC.md](../SPEC.md#L292)).
Redaction scans secrets with approve-diff, default-on for export,
off for same-machine (prior F1.6). Progress streams copy/verify/
launch phases with cancel-before-launch. Rollback = delete copied
target file + mark lineage "aborted" (source never moved, SPEC
`:287`); post-success both sessions cross-link lineage with
"Resume original" / "Return on reset" (SPEC `:288`). T1
same-account Resume/Relaunch buttons ship first — `LaunchMode::Resume`
already exists, only UI is missing (prior F2.3).

---

## 7. Usage / Fleet / Settings — placeholder vs. target, per control

Global nav is Overview/Accounts/Usage/Fleet
([App.tsx](../../ui/src/App.tsx#L44));
**Settings does not exist at all** — no nav entry, screen, or route.

| Surface | Today (code) | SPEC target (§9.2) | Gap per control |
|---|---|---|---|
| Usage | `ComingSoon` card, zero controls (`App.tsx:338-341,525-539`) | Per-account timelines, 5h/weekly/credit windows, per-session breakdown, CSV export (`docs/SPEC.md:349`) | No chart, window selector, account filter, honesty-tier label, export, or empty-state guidance; nav click is a dead end that trains users to stop exploring (prior UX.7) |
| Fleet | Same `ComingSoon` (`App.tsx:340-341`) | Machine list (state/resources/permissions), filters across board/overview/usage, pairing wizard, cross-machine handover (`docs/SPEC.md:350,387-389`) | No machine card, pairing code/QR, permission toggle, or filter; Overview hardcodes Fleet=1 (`App.tsx:468-473`) |
| Overview quota | `QuotaRing percent={0}` ×2 (`App.tsx:513-514`) + copy admitting M2 deferral (`:516-518`) | Rings with reset countdowns per account (`docs/SPEC.md:345`) | Fabricated 0% reads as "no usage"; needs explicit "No data yet" state (prior F2.4). Ring itself is sound (`primitives.tsx:165-218`) but has no % text, tier label, or countdown |
| Overview machine | Hardcoded "OK / nominal (live gauges: M3)" (`App.tsx:461-467`); rail footer "this-mac · healthy" (`App.tsx:282-290`) | CPU/RAM/swap/disk gauges (`docs/SPEC.md:345`) | Static success claim with no sensor behind it — worst placeholder pattern in the app |
| Settings | **Absent** (grep: no Settings screen in `ui/src`) | Stuck thresholds, alert levels, terminal prefs, update channel, telemetry off (`docs/SPEC.md:353`) | Every specced control missing: threshold sliders, alert toggles, font/scrollback, channel picker, telemetry switch, plus unspecced-but-needed: storage size, retention, backup/restore (prior F1.3/F1.7) |
| Tray / hotkey | Absent in `ui/src` | Aggregate quota, alerts, quick-launch, global hotkey (`docs/SPEC.md:352`) | No glance surface; daily loop requires full window (prior F7.2) |

Recommendation: remove Usage/Fleet from nav until each has one real control, ship Settings (thresholds + terminal + updates + backup) before M3, and never render a numeric ring/gauge without data + confidence label ([docs/SPEC.md](../SPEC.md#L259)).

---

## 8. Modal, toast & error system

There is **no toast/banner system**. Channels today: inline `text-danger` strings (wizard `OnboardingWizard.tsx:266`, NewSession `NewSession.tsx:211`), card banners (Profiles `Profiles.tsx:95-99`), and `window.alert` ×3 (`App.tsx:371,384`; `Profiles.tsx:367`) — modal, unstyled, unrecoverable. `refresh()` swallows errors and keeps stale data (`App.tsx:101-116`, prior F2.1). Modals (`form.tsx:54-97`, wizard, `NewSession.tsx:77-80`) handle Escape inconsistently and lack `role="dialog"`/focus-trap/return-focus (prior UX.8). Required: one status model (`loading|ready|degraded|error` + Retry), one non-blocking toast, deletion of every `window.alert`, and coded errors beyond `to_string()` (`error.rs:52-60`).

## 9. Fix list (priority; acceptance in italics)

**P0:** (1) Auto re-probe login on PTY exit + poll; wire wizard to `onTerminalExit` (*login completes without finding Re-check*). (2) Step-0 harness/install gate (*no-binary machine never reaches dead Login*). (3) Handover: `transcript_ref`, freeze, sibling copy, verify, lineage (*golden-path gains subagent fixtures*). (4) Remove fabricated Overview numbers. **P1:** Detect badges + full notes; partial import; label guard; Summary Launch/Add-another; ReadyHome sticky skip; handover wizard §6.3 (T1 first); nav gating; Settings v1; toast system. **P2:** Max 5×, offline copy, wizard resume, redaction, fleet-ready lineage.

## 10. Competitor check (bounded)

Orchestrators delegate accounts/quota to CLIs — [T3 Code](https://t3.codes/), [Opcode](https://github.com/winfunc/opcode), [Claude Squad](https://github.com/smtg-ai/claude-squad), [Conductor](https://www.conductor.build/docs/concepts/workflow), [Vibe Kanban](https://github.com/BloopAI/vibe-kanban) — while switchers own quota without sessions: [cc-switch](https://github.com/farion1231/cc-switch) (tray quota, one-click switch), [claude-swap](https://github.com/realiti4/claude-swap) (rotation/dashboard). Onboarding already beats orchestrator first-run; the gap is switcher table-stakes (tray glance, headroom at decision time) — hence §6.3/§7 outrank Usage charts. *Unresolved: live fidelity on real Claude 2.1.296 (cross-dir `--resume`, dotted cwds, MCP) needs dogfood; contrast/focus unverified without browser/screen-reader pass.*
# Deep competitor review 04 — T3 Code & Opcode vs AI Juggler
>
> Date: 2026-10-13. Scope: T3 Code (`pingdotgg/t3code`,
> [t3.codes](https://t3.codes/)) and Opcode, formerly Claudia
> (`winfunc/opcode`, [opcode.sh](https://opcode.sh/)).
> Method: official site/docs/GitHub/README + release/issue primary
> sources via web search, cross-checked against the parent review
> [full-competitive-ux-review.md](full-competitive-ux-review.md#L522)
> and Juggler source. Every competitor claim carries a URL; every
> Juggler claim carries `file:line`. Star counts are snapshot-only.
> Prior review lines cited below: T3 at
> [full-competitive-ux-review.md](full-competitive-ux-review.md#L522),
> Opcode at
> [full-competitive-ux-review.md](full-competitive-ux-review.md#L545).
> This file goes deeper on history, onboarding, execution model,
> review loop, and churn; it does not restate the parent summary.
>
> Verdict keys: **STEAL** = copy the pattern, **BEAT** = Juggler can
> win by doing the honest/juggling version, **AVOID** = do not copy.
> Gap severity: **P0** blocks Juggler's core promise, **P1** before
> v0.1-beta, **P2** later. Negative evidence is marked **[NEG-EV]**.
>
> Juggler anchors used throughout: only Claude adapter registered
> ([state.rs](../../crates/juggler-app/src/state.rs#L91)),
> adapter trait at
> ([adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L26)),
> quota model at
> ([model.rs](../../crates/juggler-core/src/model.rs#L277)),
> Usage/Fleet still `ComingSoon`
> ([App.tsx](../../ui/src/App.tsx#L339)).
---

## 1. Executive judgment (read this first)

- T3 and Opcode attack opposite ends: T3 is a *multi-harness
  control surface* with mobile/remote reach
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md));
  Opcode is a *single-harness depth tool* (checkpoints, agents,
  MCP, usage) for Claude Code only
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- Neither owns multi-account juggling. T3 uses provider instances,
  not named account profiles — parent finding at
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L475);
  Opcode multi-account is **[NEG-EV]** — no feature found in
  README/docs, per
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L554).
- Juggler's wedge is therefore intact but time-boxed: T3 ships
  worktrees, diff, PR linking, palette, and remote today, while
  Juggler's quota UI is hardcoded `percent={0}`
  ([App.tsx](../../ui/src/App.tsx#L513)).
- Opcode proves demand for checkpoints/agents/MCP depth, then
  stalls — last dated commits Oct 2025
  ([commit](https://github.com/winfunc/opcode/commit/70c16d8a4910db48cd9684aeacdd431caefd7d71)).
  Its users are migratable, but its AGPL depth is not copyable
  wholesale (see §7).
- The single most stealable ideas: T3's thread/worktree/PR
  lifecycle and ⌘K palette; Opcode's checkpoint-fork and MCP
  registry UX. The single most beatable gap: honest quota —
  both competitors blur cost vs remaining subscription quota.

## 2. T3 Code — history, team, traction, funding

### 2.1 Origin and timeline

- Repo `pingdotgg/t3code`, created 2026-02-08, by Theo Browne /
  T3 Tools Inc. (Ping.gg)
  ([research mirror](https://github.com/2002bishwajeet/yantra/blob/HEAD/docs/research/15-t3code-for-the-chat.md)).
- Public launch 2026-03-07: "T3 Code is now available for
  everyone… Fully open source"
  ([launch summary](https://github.com/seth-torrence/artemis/blob/HEAD/docs/research/T3-DESIGN-RESEARCH.md)).
- Positioned in its own `AGENTS.md` as an open-source
  "bring-your-own-subscription" alternative to Claude Desktop,
  Codex App, Cursor Glass and Conductor
  ([AGENTS.md](https://github.com/jaiir320/t3code/blob/HEAD/AGENTS.md)).
- Self-described as an "agent harness control surface" for
  agents already on your machine — web, Electron desktop, and
  mobile clients
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
- Release cadence is fast/nightly: `v0.0.42` stable plus nightly
  builds through Oct 2026
  ([release v0.0.42](https://github.com/pingdotgg/t3code/releases/tag/v0.0.42)).

### 2.2 Team and funding

- Built by Theo Browne (t3.gg, T3 Stack, T3 Chat) and the T3
  Tools team incl. Julius Marminge
  ([contributor note](https://github.com/lachlanstuart/l6claw)).
- **[NEG-EV]** No venture funding announcement for T3 Code
  itself was found in official sources searched; Ping.gg/T3
  Chat is the operating company
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
  Do not cite a raise without a primary source.
- Monetisation stance is explicitly anti-markup: "no paid tier
  and will not monetize usage… bring your own subscription"
  ([analysis](https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-08-08-t3-code-doubles-down-on-zero-markup-pricing-as-a-trust-play/)).
- That stance is a trust play, not a business model — it
  pressures every paid orchestrator (cf. Conductor pricing in
  parent review
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L625)).

### 2.3 Traction (snapshot, not ranking)

- ~21,893 stars / 5,385 forks at 2026-09-07 snapshot, latest
  release `v0.0.39` then
  ([research mirror](https://github.com/2002bishwajeet/yantra/blob/HEAD/docs/research/15-t3code-for-the-chat.md)).
- Parent review snapshot: ~24k stars (2026-09-24), "alpha
  data-loss ToS, poor CLI sync" as weaknesses
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L543)).
- AGENTS.md-derived claim of ">200k users" circulates via
  mirrors — treat as vendor claim, not audited metric
  ([research](https://github.com/seth-torrence/artemis/blob/HEAD/docs/research/T3-DESIGN-RESEARCH.md)).
- Fork ecosystem is itself a traction signal: ACP-provider
  forks, TTS forks, TUI clients all track upstream weekly
  ([fork example](https://github.com/jarmen423/t3code)).
- **Implication for Juggler:** T3's distribution (Theo's
  audience + `npx t3`) is the moat, not any single feature.
  Juggler cannot out-shout it; it must out-juggle it.

## 3. T3 Code — product deep dive

### 3.1 Install and onboarding

- Zero-install trial: `npx t3@latest` (Node 22.16+/23.11+/
  24.10+) starts backend + local web app
  ([README via fork](https://github.com/palenza/raptor)).
- If Claude Code/Codex is already authenticated, first session
  is near-instant — "begin using the interface almost
  immediately" ([stealwhatworks](https://stealwhatworks.com/blogs/news/why-t3-code-growing-so-fast)).
- Per-provider auth is delegated: `claude auth login`,
  `agent login` (Cursor), `grok login`, `opencode auth login`
  ([README](https://github.com/kossnocorp/t3code)).
- Desktop (Electron), hosted web `app.t3.codes`, iOS/Android
  apps all front the same local server
  ([README](https://github.com/barretodiego/t3code/blob/HEAD/README.md)).
- **Vs Juggler:** Juggler's onboarding wizard is the parent
  review's "best-built surface"
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L213)).
  T3 wins on *time-to-first-token* (`npx` one-liner); Juggler
  wins on *account hygiene* (scan/import/login guidance).
  Verdict: **STEAL** the one-command trial path for beta.

### 3.2 Supported harnesses, models, accounts

- Harnesses: Claude Code, Codex, Cursor, Grok Build, OpenCode,
  Google Antigravity
  ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
- Independent comparison confirms Claude/Codex/OpenCode/Cursor
  today ([compare](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-t3-code.mdx)).
- Models are picked per provider from that provider's catalog;
  forks add "full model catalog, slash commands, approval
  modes" via ACP — evidence the core picker is extensible
  ([fork](https://github.com/jarmen423/t3code)).
- Accounts: **[NEG-EV for named profiles]** — multi-account is
  provider *instances* / env auth, not first-class named
  profiles (parent finding
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L530)).
- Continuation/resume compatibility "varies by harness"
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L532)).
- **Vs Juggler:** Juggler has exactly one registered adapter
  ([state.rs](../../crates/juggler-app/src/state.rs#L91))
  but first-class `Profile = account + settings`
  ([model.rs](../../crates/juggler-core/src/model.rs#L173)).
  Breadth: T3 wins 6-to-1. Depth of account model: Juggler
  wins. Verdict: **BEAT** on accounts, **AVOID** chasing 6
  harnesses before quota (parent F6.1).

### 3.3 Project / session / terminal model

- Core unit is the *thread* inside a project; a Node
  WebSocket server wraps provider CLIs and serves all clients
  ([AGENTS.md](https://github.com/jaiir320/t3code/blob/HEAD/AGENTS.md)).
- Thread sidebar tracks PR per unsettled thread from its saved
  branch/worktree, with manual link/unlink override
  ([thread-sidebar](https://github.com/9cb14c1ec0/t3code/blob/HEAD/docs/user/thread-sidebar.md)).
- Terminal history is capped at 5,000 lines, control sequences
  stripped, one log per terminal, reloaded on first open
  ([terminal research](https://github.com/ticketry-hq/ticketry/blob/HEAD/docs/research/t3-code-terminals.md)).
- Lifecycle vocabulary: settle / unsettle / snooze / interrupt
  / delegate ([t3code-threads skill](https://github.com/arrrnmp/t3code-tui/blob/HEAD/skills/use-t3code-cli/SKILL.md)).
- Settlement is persisted server-side; clients render it, and
  a `thread.settled` event can stop an idle provider session
  ([internals](https://github.com/ochen1/t3dpcode/blob/HEAD/docs/internals/overview.md)).
- **Vs Juggler:** Juggler sessions carry only `cwd` + profile
  ([model.rs](../../crates/juggler-core/src/model.rs#L394));
  rail caps at `slice(0,15)` with no search
  ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L377)).
  Verdict: **STEAL** settle/archive + snooze semantics (P1).

### 3.4 Parallel execution and worktrees

- One task/thread = one git worktree + branch is the headline
  parallel workflow ("worktree-based parallel project
  workflow… standout" [bestofai](https://bestofai.com/our-tool-picks)).
- Atomic thread bootstrap for new worktrees (`--checkout
  worktree`): create thread, prepare worktree, run setup
  script, start prompt
  ([t3code-tui](https://github.com/arrrnmp/t3code-tui/blob/HEAD/README.md)).
- `newWorktreesStartFromOrigin` config controls base ref;
  default flipped `false`→`true` in 0.0.29
  ([t3code-tui](https://github.com/arrrnmp/t3code-tui/blob/HEAD/README.md)).
- Orchestrator pattern: workers in worktrees, "preserve
  history and worktrees unless deletion explicitly authorized"
  ([orchestrator prompt](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- **Vs Juggler:** no worktree/branch concept outside a pack
  field — parent F1.1
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L63)).
  This is a **P0 gap**: parallel accounts on one repo without
  isolation will collide. Verdict: **STEAL** (P0/P1).

### 3.5 Diff, review, and PR flow

- Diff viewer with staging/commit per thread (parent review
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L536)).
- Sidebar auto-links the PR for each unsettled thread's branch;
  right-click to link a different PR or unlink
  ([thread-sidebar](https://github.com/9cb14c1ec0/t3code/blob/HEAD/docs/user/thread-sidebar.md)).
- Orchestrator guidance: "Settle completed PRs and archive
  finished worker threads… Do not archive active review
  owners" ([prompt](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- **[NEG-EV]** Inline comment → agent round-trip (Vibe Kanban's
  signature) was not found in official T3 docs searched.
- **Vs Juggler:** no diff/review loop at all (parent F7.3
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L497)).
  Verdict: **STEAL** read-only diff + PR link first (P1);
  **AVOID** full staging/commit until review demand is proven.

### 3.6 Agents, subagents, and delegation

- Threads support delegate/interrupt/snooze operations exposed
  to external orchestrators via CLI skills
  ([skill](https://github.com/arrrnmp/t3code-tui/blob/HEAD/skills/use-t3code-cli/SKILL.md)).
- A `t3code-handoff` skill hands a conversation to a fresh
  thread ([skill index](https://github.com/arrrnmp/t3code-tui/blob/HEAD/skills/use-t3code-cli/SKILL.md)).
- Community MCP bridge (`t3-code-agent-mcp`) orchestrates
  worker threads externally — evidence T3 is scriptable, not
  that subagents are a native first-class entity
  ([repo](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- **[NEG-EV]** No native named subagent library/permissions
  model like Opcode's was found in official T3 README/docs.
- **Vs Juggler:** Juggler has no agent/subagent concept; its
  adapter trait is launch/watch only
  ([adapter.rs](../../crates/juggler-adapter/src/adapter.rs#L26)).
  Verdict: **BEAT** via handover lineage, not agent library.

### 3.7 MCP support

- **[NEG-EV]** Official T3 README positions MCP as something
  third parties build *on* T3 (agent-mcp bridge), not a
  built-in MCP registry UI
  ([bridge](https://github.com/gfsaaser24/t3-code-agent-mcp/blob/HEAD/examples/orchestrator-prompt.md)).
- Provider-level MCP/skills pass through the underlying CLIs;
  forks surface provider slash commands/approvals
  ([fork](https://github.com/jarmen423/t3code)).
- **Vs Juggler:** no MCP sync/portability (parent F1.9
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L148)).
  Both are absent; Opcode (not T3) is the MCP benchmark.
  Verdict: **AVOID** racing T3 here; learn from Opcode §5.7.

### 3.8 Usage, quota, and cost

- A usage panel shows token + API-cost analytics (parent
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L537)).
- Whether it reflects *official remaining subscription quota*
  is disputed; a 5h/weekly probe exists only in forks
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L538)).
- Independent quota matrix scores T3 official quota % as
  absent (❌), reset countdowns ❌
  ([matrix](https://github.com/superset-sh/superset/blob/HEAD/plans/20260816-quota-tools-marketing-research.md)).
- Internal usage design tracks `total_cost_usd` passthrough
  with "no cost tables" plus context/cache counters
  ([DECOUPLE](https://github.com/arrrnmp/t3code-tui/blob/HEAD/DECOUPLE.md)).
- Pricing posture: free, zero markup, "exactly what they pay"
  as trust wedge
  ([analysis](https://enterprisedna.co/resources/ai-pulse/ai-pulse-2026-08-08-t3-code-doubles-down-on-zero-markup-pricing-as-a-trust-play/)).
- **Vs Juggler:** Juggler models `QuotaWindow` + honesty tiers
  Measured/Computed/Estimated in SPEC
  ([SPEC.md](../SPEC.md#L256))
  but UI rings are hardcoded zero
  ([App.tsx](../../ui/src/App.tsx#L513)).
  Verdict: **BEAT** — ship Measured-labelled quota; never blur
  cost into quota the way the disputed T3 panel does (P0).

### 3.9 Notifications, remote, and mobile

- Remote session control: pair phone/tablet via QR/pairing
  token or hosted `app.t3.codes`; server is local Node,
  optionally LAN/Tailscale/SSH
  ([ade-arena](https://github.com/fcamblor/ade-arena/issues/4)).
- `REMOTE.md`-backed pairing is a documented feature, not a
  hack ([summary](https://github.com/fcamblor/ade-arena/issues/4)).
- Mobile apps (iOS/Android) are first-class store listings
  ([README](https://github.com/barretodiego/t3code/blob/HEAD/README.md)).
- Background-service mode and remote-access docs ship in-repo
  ([docs index](https://github.com/jarmen423/t3code)).
- **[NEG-EV]** Granular notification policy (quiet digest,
  per-project mute) was not found in official docs searched.
- **Vs Juggler:** Fleet is `ComingSoon`
  ([App.tsx](../../ui/src/App.tsx#L341));
  tray is specced quota-only
  ([SPEC.md](../SPEC.md#L352)).
  Verdict: **STEAL** QR pairing model for Fleet (P2);
  **BEAT** on notification discipline (WOW-g, parent §UX).

### 3.10 Search, history, keyboard, palette

- ⌘K command palette searches commands, projects, threads;
  `>` prefix filters to actions; ⌘1–⌘9 jump to threads
  ([keybindings](https://github.com/onkarj012/t3code/blob/HEAD/docs/user/keybindings.md)).
- iPad hardware-keyboard shortcuts + palette shipped in
  v0.0.42 ([release](https://github.com/pingdotgg/t3code/releases/tag/v0.0.42)).
- Keybindings are user-editable via Settings → Keybindings,
  persisted to `~/.t3/userdata/keybindings.json`
  ([keybindings](https://github.com/dbalders/tritonai-harness/blob/HEAD/docs/user/keybindings.md)).
- Import of external Claude Code/Codex conversations is a
  documented user flow
  ([docs index](https://github.com/ochen1/t3dpcode/blob/HEAD/docs/README.md)).
- CLI-sync gap is the known wound: resume picker for external
  sessions requested (#5346), `/resume` broken in CLI-used
  folders (#4519)
  ([discussion #6994](https://github.com/pingdotgg/t3code/discussions/6994)).
- **Vs Juggler:** no palette/search; rail truncated lists only
  ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L377)).
  Verdict: **STEAL** ⌘K wholesale (P0 WOW-c in parent review).

### 3.11 Settings, platforms, pricing, licence

- Settings surface includes keybindings, project icons,
  permission modes, telemetry opt-out, mobile appearance
  ([docs index](https://github.com/ochen1/t3dpcode/blob/HEAD/docs/README.md)).
- Anonymous usage telemetry exists and is documented — note
  the contrast with Juggler's local-first/no-telemetry stance
  ([README.md](../../README.md#L38)).
- Platforms: Electron desktop + web + iOS + Android, one
  server ([README](https://github.com/pingdotgg/t3code/blob/main/README.md)).
- Price: free; licence MIT (T3 Tools Inc.)
  ([compare](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-t3-code.mdx)).
- Maturity warning from comparators: "very early alpha,
  expect rapid change"
  ([compare](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-t3-code.mdx)).
- Known churn drivers: V2 closing sessions mid-batch (#16697),
  T3 Connect failures on Windows (#14070, #14836), Codex
  plugin blindness (#9044)
  ([issue #16697](https://github.com/pingdotgg/t3code/issues/16697),
  [issue #14070](https://github.com/pingdotgg/t3code/issues/14070)).
- **Lesson:** velocity + audience hides alpha breakage for
  T3; Juggler (no audience buffer) cannot afford the same
  data-loss reputation. Verdict: **AVOID** alpha-ToS posture.

## 4. Opcode — history, team, traction, funding

### 4.1 Origin: Claudia → Opcode

- Repo `winfunc/opcode` (org also seen as `getAsterisk`),
  created 2025-06-19, by Asterisk Labs ("Made with ❤️ by
  Asterisk" [README](https://github.com/winfunc/opcode)).
- Formerly **Claudia**; rebranded to Opcode mid-2025
  ([competitive note](https://github.com/hamed-farag/claudeck/blob/HEAD/docs/COMPETITIVE-ANALYSIS.md)).
- Company described as Y Combinator-backed Asterisk Labs
  ([research](https://github.com/michaellinhardt/grimoire/blob/HEAD/_bmad-output/research/opcode-backend-ai-features.md)).
- Tauri 2 + React + TypeScript + Rust desktop app wrapping
  the Claude Code CLI
  ([landscape](https://github.com/roman10/mcode/blob/HEAD/docs/open-source/competitive-landscape.md)).
- Data model: spawns `claude` with stream-json and parses
  `~/.claude/projects/*.jsonl`
  ([research](https://github.com/enerage/vibespace/blob/HEAD/RESEARCH-CLAUDE-DATA.md)).

### 4.2 Traction and dormancy

- ~21–22k stars, ~1.7k forks, AGPL-3.0 (snapshots vary)
  ([facts](https://agentskillshub.top/skill/winfunc/opcode/),
  [landscape](https://github.com/farol-team/workroom/blob/HEAD/docs/spikes/agent-ui-landscape.md)).
- Parent review: "~21-22k stars… dormant since Oct 2025,"
  strengths checkpoints/MCP/agents
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L575)).
- Last dated commits cluster 2025-10-05…2025-10-13 (edition
  fix, wss fix) ([commit](https://github.com/winfunc/opcode/commit/70c16d8a4910db48cd9684aeacdd431caefd7d71)).
- 332 open issues at snapshot; build-broken reports on the
  last commit (#369) are a churn accelerant
  ([issue #369](https://github.com/winfunc/opcode/issues/369),
  [facts](https://agentskillshub.top/skill/winfunc/opcode/)).
- Discovery chain explicitly flags it "largest-by-star Claude
  Code GUI" *and* stale in the same breath
  ([discovery](https://github.com/julo15/seshctl/blob/HEAD/.agents/discoveries/2026-05-25-1700-seshctl-alternatives.md)).
- **Implication:** Opcode validates the GUI-wrapper market,
  then vacates it. Its feature checklist is a free product
  spec; its maintenance vacuum is Juggler's opening (same
  pattern as Vibe Kanban/Crystal, parent F7.4).

## 5. Opcode — product deep dive

### 5.1 Install and onboarding

- First run auto-detects `~/.claude`; prerequisites are
  Rust/bun/git per-platform per README install section
  ([README](https://github.com/winfunc/opcode)).
- No `npx`-style one-liner; build-from-source friction is
  documented in issue #369's clone/bun/tauri-build path
  ([issue #369](https://github.com/winfunc/opcode/issues/369)).
- **Vs Juggler:** Juggler's wizard (scan → read-only import →
  guided vendor login) is strictly better for multi-account
  users (parent
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L215)).
  Verdict: **BEAT** — keep wizard; never regress to
  detect-one-dir onboarding.

### 5.2 Harnesses, models, accounts

- Single-harness by design: Claude Code only
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L554)).
- Models: per-agent model choice among Claude models
  ([README mirror](https://github.com/khj-nextground/opcode)).
- Accounts: **[NEG-EV]** no multi-account feature in
  README/docs; swapping means OS/CLI-level `~/.claude` swap
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L556)).
- **Vs Juggler:** Juggler's whole thesis (named profiles,
  isolated config dirs) is the feature Opcode lacks.
  Verdict: **BEAT** (core wedge, P0 to make visible).

### 5.3 Project / session / terminal model

- Visual project → session browser over
  `~/.claude/projects/`, with search and resume
  ([research](https://github.com/enerage/vibespace/blob/HEAD/RESEARCH-CLAUDE-DATA.md)).
- Multiple interactive sessions; CLAUDE.md editor built in
  ([landscape](https://github.com/roman10/mcode/blob/HEAD/docs/open-source/competitive-landscape.md)).
- Experimental web-server mode for mobile access exists in
  analysis forks ([analysis](https://github.com/canyavall/nori/blob/HEAD/analysis/opcode/README.md)).
- **Vs Juggler:** Juggler embeds live PTY terminals per
  session; Opcode centres the chat/session browser. Both
  parse the same JSONL. Verdict: **STEAL** project→session
  two-level browsing clarity (P2).

### 5.4 Parallel execution and worktrees

- Parallelism = multiple interactive/background sessions,
  not worktree isolation. **[NEG-EV]** no worktree/kanban
  orchestration layer
  ([research](https://github.com/3esign/coddess/blob/HEAD/docs/research-track2.md)).
- Background agents run sandboxed (seccomp/Seatbelt per
  parent [full-competitive-ux-review.md](full-competitive-ux-review.md#L564)).
- **Vs Juggler:** neither has worktrees today; T3 does.
  Verdict: **AVOID** copying session-only parallelism —
  pair Juggler parallelism with worktrees (F1.1, P0).

### 5.5 Diff, review, PR

- Diff viewer *between checkpoints* (not branch/PR diff)
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- **[NEG-EV]** no PR creation/branch-review flow found in
  official README/docs searched.
- **Vs Juggler:** both lack PR flow; T3/Conductor own it.
  Verdict: **STEAL** checkpoint-diff *idea* as session-replay
  diff (WOW-i, P2), not as the review queue.

### 5.6 Agents and subagents (Opcode's signature depth)

- Custom agents: system prompt, model choice, file/network
  permissions, agent library, import/export `.opcode.json`
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md),
  [mirror](https://github.com/khj-nextground/opcode)).
- Secure background agents with sandboxing (parent
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L562)).
- Third-party notices credit opcode for "MCP integration,
  custom agents, checkpoint/timeline, zstd+SHA-256 storage"
  ([notices](https://github.com/soul667/muxfleet/blob/HEAD/THIRD_PARTY_NOTICES.md)).
- **Vs Juggler:** Juggler deliberately supervises harnesses,
  not agent personas
  ([README.md](../../README.md#L36)).
  Verdict: **STEAL** import/export-shareable-config pattern
  for Juggler *profiles* (F1.9, P1); **AVOID** agent library.

### 5.7 MCP management

- MCP server registry/management UI; import server configs
  from Claude Desktop
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- "Better visualization of MCP responses" in execution UI
  ([README mirror](https://github.com/VasilyPolyuhovich/opcode)).
- **Vs Juggler:** zero MCP surface; only specced profile sync
  (parent F1.9). Verdict: **STEAL** registry + Desktop-import
  UX when M-sync lands (P1/P2). This is Opcode's most
  transferable feature.

### 5.8 Timeline and checkpoints (the feature to respect)

- Create a checkpoint at any point; branching visual
  timeline; one-click restore; fork sessions from any
  checkpoint; diff between checkpoints
  ([README](https://github.com/winfunc/opcode/blob/HEAD/README.md)).
- Parent verdict: "Nothing in Juggler's SPEC matches this;
  closest is the handover pack (different problem)"
  ([full-competitive-ux-review.md](full-competitive-ux-review.md#L572)).
- Checkpoints solve *time travel within* a session; handover
  solves *moving across* accounts. Users conflate them.
- **Vs Juggler:** handover v0 is transcript byte-copy +
  `--resume` (parent F3.3). Verdict: **STEAL** fork-from-
  point interaction for WOW-i replay (P2); **BEAT** by
  labelling handover vs checkpoint honestly in UI copy.

### 5.9 Usage, quota, cost

- Usage Dashboard: cost by model/project/date, token charts,
  export; alerts "coming soon" (parent
  [full-competitive-ux-review.md](full-competitive-ux-review.md#L566)).
- It is *cost analytics from JSONL*, not remaining
  subscription quota — same blur as T3, honestly labelled
  or not. **[NEG-EV]** no 5h/weekly remaining-quota feed
  found in official docs.
- **Vs Juggler:** Juggler SPEC promises Measured quota via
  statusline feed
  ([SPEC.md](../SPEC.md#L192))
  — strictly stronger claim, currently unbuilt. Verdict:
  **BEAT** (P0): ship quota+cost as separate labelled views;
  **STEAL** cost-by-project export shape for WOW-e (P1).

### 5.10 Search, keyboard, settings, platform, licence

- Session search in the browser; slash-command picker with
  cross-command search (`/clear` gap fixed via #338 fork)
  ([commit](https://github.com/lusipad/claudia/commit/5cf9957529c20890623b37a642d6857a88648e90)).
- **[NEG-EV]** no ⌘K global palette / editable keybindings
  found in official README/docs (contrast T3 §3.10).
- Platforms macOS/Linux/Windows; licence AGPL-3.0, free
  ([facts](https://agentskillshub.top/skill/winfunc/opcode/)).
- AGPL matters: Juggler is MIT
  ([README.md](../../README.md#L83)).
  Copy *ideas*, never Opcode code, into Juggler. **AVOID**.
- Churn reasons: dormancy + broken HEAD build + 332 open
  issues ([issue #369](https://github.com/winfunc/opcode/issues/369)).
  Users don't churn *to* a feature; they churn *from*
  abandonment. Verdict: **BEAT** by simply shipping.

## 6. Feature-by-feature: Them vs AI Juggler

| Feature | Them (T3 / Opcode) | Us | Gap | Verdict |
|---|---|---|---|---|
| Worktree isolation | T3 yes; Opcode no [NEG-EV] | Absent (F1.1) | P0 | STEAL T3 |
| Named multi-account | Neither (instances/dir-swap) | Built profiles | — (lead) | BEAT |
| Quota remaining | Neither (cost only) | Planned M3, UI zero | P0 | BEAT |
| Cost USD | Both: cost analytics | Absent (F1.2) | P1 | STEAL shape |
| Diff/PR | T3 PR-link; Opcode ckpt-diff | Absent | P1 | STEAL T3 |
| Checkpoints/fork | Opcode signature | Absent (pack only) | P2 | STEAL idea |
| Agents library | Opcode yes; T3 no [NEG-EV] | Absent by design | P2 | AVOID |
| MCP registry | Opcode yes; T3 no [NEG-EV] | Absent | P1 | STEAL Opcode |
| ⌘K palette | T3 yes; Opcode no [NEG-EV] | Absent | P0 | STEAL T3 |
| Mobile/remote | T3 store apps+QR | Fleet ComingSoon | P2 | STEAL QR |
| Handover lineage | T3 thread-handoff skill | v0 transcript-only | P0 | BEAT honest |
| Telemetry | T3 anonymous opt-out | None (local-first) | — (lead) | BEAT |

- Row evidence: T3 worktree/PR/palette §3.4/§3.5/§3.10;
  Opcode checkpoints/agents/MCP §5.6–§5.8; Juggler gaps at
  parent F1.1/F1.2/F1.9 and
  ([App.tsx](../../ui/src/App.tsx#L339)).
- Juggler capabilities advertised (`TRANSPLANT|FORK`) exceed
  implementation — downgrade per parent F3.1
  ([claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L111)).

## 7. Strengths, weaknesses, churn — synthesis

- **T3 strengths:** breadth (6 harnesses), `npx` trial,
  worktree/PR thread model, mobile reach, MIT + zero-markup
  trust. **Weaknesses:** alpha breakage (#16697/#14070),
  CLI-sync wound (#6994), quota blur (§3.8), telemetry.
  **Churn trigger:** data-loss/session-kill bugs, not price.
- **Opcode strengths:** checkpoint timeline, agents+MCP
  depth, cost dashboard, polished single-harness UX.
  **Weaknesses:** Claude-only, no accounts/worktrees,
  AGPL, dormant since Oct 2025 with broken HEAD (#369).
  **Churn trigger:** abandonment itself.
- **Juggler's play:** take T3's lifecycle + palette, Opcode's
  MCP/config portability, beat both on the one axis neither
  attempts — named-account, honesty-labelled quota juggling
  ([SPEC.md](../SPEC.md#L259)).
  Sequence per parent Phase 1: quota + needs-attention +
  honest handover before harness #2. Anything else is
  building Opcode's checklist while T3 eats the market.

## Appendix — unresolved / negative evidence log

- T3 funding: no primary raise found **[NEG-EV]** (§2.2).
- T3/Opcode notification policy depth: not in official docs.
- Live star counts: snapshot mirrors only, no GitHub API
  fetch this session (parent Appendix A caveat applies).
- In-app (non-doc) settings for both tools were not run;
  absence claims are docs/README-level **[NEG-EV]** only.
# Deep competitor profile — Claude Squad & Conductor
> 5× deeper than `docs/review/full-competitive-ux-review.md`
> §Q7 (Squad 577–602, Conductor 604–626). Official GitHub/docs/
> site only for competitor facts (URL per claim); Juggler claims
> carry `repo file:line`. `[negative evidence]` = not found.
> Snapshot: 2026-10-13. Squad = OSS Go TUI (`cs`); Conductor =
> closed-source macOS app (Melty Labs). Same worktree primitive,
> opposite bets on accounts, review, and openness.

---

## 0. Verdict in 30 seconds

| Question | Answer |
|---|---|
| Closest threat to Juggler's wedge? | Neither. Both delegate accounts/quota to the CLI. |
| Most stealable idea? | Squad's `1 task = 1 tmux + 1 worktree` atomicity; Conductor's diff→PR→merge cockpit. |
| Most dangerous trap? | Squad's `-y/--autoyes` auto-accept; Conductor's closed-source polish envy. |
| What the parent review got wrong? | Conductor pricing ($50/$60) is stale — current official pricing is free. See §7. |
| What Juggler must do first? | Worktree-on-launch (F1.1 in parent review) — both competitors prove it is table stakes. |

Parent claim survives: all orchestrators "delegate
quota/accounts to the underlying CLIs"
(`docs/review/full-competitive-ux-review.md:46-48`). §5 proves
it is architectural in both tools, not a missing toggle.

## 1. Claude Squad — history & traction

### 1.1 What it is, precisely

Claude Squad is a terminal UI that "runs Claude Code, Codex,
Gemini, Aider, OpenCode, Amp, and other local programs in
separate tmux sessions and git worktrees"
[https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
It is **not an agent**: it spawns the user's CLI in tmux with
an isolated worktree (parent review
`docs/review/full-competitive-ux-review.md:579-582`). Breadth
("any CLI via `-p`") is the feature — the opposite bet from
Juggler's typed adapter (`crates/juggler-adapter/src/adapter.rs:26-27`).

### 1.2 Origin & maintenance signal

- Repository: `smtg-ai/claude-squad`, written in Go, shipped
  as a single binary installed via `brew install claude-squad`
  or `curl -fsSL https://raw.githubusercontent.com/smtg-ai/claude-squad/main/install.sh | bash`
  [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
- Release cadence: v1.0.19 published 2026-06-17; v1.0.20 on
  2026-08-20 is the latest cited release — actively maintained,
  not dormant
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md),
  [https://github.com/gaby/munder-difflin/blob/HEAD/blog/src/posts/herdr-alternatives.md](https://github.com/gaby/munder-difflin/blob/HEAD/blog/src/posts/herdr-alternatives.md).
- Traction magnitude: ~7,171 stars in a 2026-03 snapshot with
  51 open issues and 12 commits/90d
  [https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md](https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md);
  ~8,347 stars in a 2025-05 commit view
  [https://github.com/smtg-ai/claude-squad/commit/b4a95e186d024f840e5f4b3565120c7df66896da](https://github.com/smtg-ai/claude-squad/commit/b4a95e186d024f840e5f4b3565120c7df66896da);
  parent review's "8.1–8.5k" range
  (`docs/review/full-competitive-ux-review.md:602`) is consistent.
  Treat all counts as magnitude, not ranking — no direct
  GitHub API fetch was available in-session (parent review
  Appendix A, `docs/review/full-competitive-ux-review.md:1047-1048`).
- Issue-tracker depth: ~133 issues mined in one independent
  sweep [https://github.com/wilsonsousajr/omatty/commit/d70858998ed3b5ecf6894ef9b1e079173d07e272](https://github.com/wilsonsousajr/omatty/commit/d70858998ed3b5ecf6894ef9b1e079173d07e272) —
  small enough to read fully, large enough to show real
  dogfooding. Juggler has no comparable public signal yet.

### 1.3 Lesson: zero-to-first-session time

Squad proves demand for parallelism without a GUI tax (`cs`
to launch). Juggler's richer onboarding
(`ui/src/screens/OnboardingWizard.tsx`;
`docs/review/full-competitive-ux-review.md:213-220`) must
approach that two-command story on the Claude-only path.

---

## 2. Conductor — history & traction

### 2.1 What it is, precisely

Conductor (conductor.build) is "a Mac app for running several
agents at once through a chat interface" supporting "Claude
Code, Codex, Cursor, and OpenCode," where "a task lives in its
own workspace, meaning its own branch, files, terminal, and
diff, and the app walks it from review to pull request, merge,
and archive" [https://www.agentastic.dev/alternatives/paseo](https://www.agentastic.dev/alternatives/paseo).
It is built by Melty Labs (YC S24), which "built the Melty
editor first, then pivoted" to the parallel runner and
"essentially pioneered the category of the Mac 'agentic
parallel runner'" [https://github.com/geraschenko/pictl/blob/HEAD/docs/agent-harness-landscape.md](https://github.com/geraschenko/pictl/blob/HEAD/docs/agent-harness-landscape.md).
The parent review's one-liner ("native macOS app (YC S24) that
pioneered the agentic parallel runner,"
`docs/review/full-competitive-ux-review.md:606-608`) holds.

### 2.2 Funding & velocity

Melty Labs, YC S24; "$22M Series A" / "$24M raised"
`[unresolved exact total]` [https://github.com/broomva/broomva.tech/blob/HEAD/apps/broomva/content/writing/agent-cockpit-wars.mdx](https://github.com/broomva/broomva.tech/blob/HEAD/apps/broomva/content/writing/agent-cockpit-wars.mdx),
[https://github.com/antoinedc/mantaui/blob/HEAD/docs/specs/website-readme-revamp.md](https://github.com/antoinedc/mantaui/blob/HEAD/docs/specs/website-readme-revamp.md);
founders ex-Replicate/ex-Netflix ML (same source). "Used by
engineers at Linear, Vercel, Notion, Stripe"
[https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md](https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md)
is marketing social proof, not audited traction. Changelog
to v0.87.x ships bundled model updates (e.g. Codex 0.156.1)
[https://www.conductor.build/changelog](https://www.conductor.build/changelog) —
a burden Juggler avoids by spawning the user's own CLI
(`crates/juggler-adapters/src/claude_code.rs:298-305`).

### 2.3 Closed source is the product decision

Conductor is "closed source, no public GitHub repo"
[https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md](https://github.com/dpeluche/exegol/blob/HEAD/docs/project_definition/RESEARCH.md),
"Proprietary, free - you bring your own agent subscription
or API key" [https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md](https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md).
That means: no audit path, no fork path, no offline guarantee.
Juggler is MIT (`Cargo.toml:17`, `README.md:83`) and local-only
by invariant (`docs/internal/agent-review-prompt.md:70`). The
contrast is a positioning asset Juggler under-uses today.

---

## 3. The tmux/worktree model — the shared foundation

Both tools converge on the same isolation primitive. The
differences are in lifecycle and cleanup, where Juggler's
current gap (no worktree at all) is most exposed.

### 3.1 Claude Squad: tmux session + worktree, atomically

- "Each instance = tmux session (`claudesquad_<title>_<timestamp>`)
  + git worktree in `~/.claude-squad/worktrees/` on branch
  `<prefix>_<name>_<timestamp>`"
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md).
- "Claude Squad creates a unique branch/worktree for every
  session and starts that agent's tmux program inside that
  isolated worktree"
  [https://github.com/opensiro/vsm-harness-index/blob/HEAD/assessments/claude-squad.md](https://github.com/opensiro/vsm-harness-index/blob/HEAD/assessments/claude-squad.md).
- State persists in `~/.claude-squad/state.json` (instance
  metadata, branch ownership, base commit) plus `daemon.pid`;
  config in `~/.claude-squad/config.json`
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md).
- Pause vs delete are distinct: "pause can retain the branch
  for resume, while deletion removes only worktrees/branches
  that Claude Squad owns"
  [https://github.com/giuseppecrj/pi-herdr-agents/blob/HEAD/docs/research/worktree-subagent-orchestration.md](https://github.com/giuseppecrj/pi-herdr-agents/blob/HEAD/docs/research/worktree-subagent-orchestration.md).
  Ownership-scoped deletion is a safety property Juggler must
  copy verbatim if it adds worktrees.

### 3.2 Conductor: workspace = worktree + branch + lifecycle scripts

- "Local workspaces as git worktrees created under
  `~/conductor/workspaces/<repo name>/<workspace name>`"
  [https://www.conductor.build/docs/concepts/git-worktrees](https://www.conductor.build/docs/concepts/git-worktrees)
  (via [https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md](https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md)).
- "Conductor documents a separate branch and working tree
  for each workspace" [https://www.conductor.build/docs/concepts/parallel-agents](https://www.conductor.build/docs/concepts/parallel-agents)
  (via [https://github.com/phong711/agent-orchestrator-p/blob/HEAD/frontend/src/landing/content/compare/ao-vs-conductor.mdx](https://github.com/phong711/agent-orchestrator-p/blob/HEAD/frontend/src/landing/content/compare/ao-vs-conductor.mdx)).
- Lifecycle scripts are part of workspace creation:
  "`scripts.setup` runs after Conductor creates a workspace.
  `scripts.run` runs from Conductor's Run button.
  `scripts.archive` runs before Conductor archives a workspace"
  [https://github.com/cmer/skills/blob/HEAD/plugins/orchestrator-setup/skills/orchestrator-setup/references/orchestrators/conductor.md](https://github.com/cmer/skills/blob/HEAD/plugins/orchestrator-setup/skills/orchestrator-setup/references/orchestrators/conductor.md),
  with env vars `CONDUCTOR_WORKSPACE_NAME`, `CONDUCTOR_ROOT_PATH`,
  and `$CONDUCTOR_PORT` port allocation
  [https://github.com/biw/keychain-store/blob/HEAD/.agents/skills/conductor-setup/references/scripts-and-environment.md](https://github.com/biw/keychain-store/blob/HEAD/.agents/skills/conductor-setup/references/scripts-and-environment.md).
- Cloud variant: "cloud workspaces are Vercel sandboxes,
  8-core / 16 GB" running Linux, drivable from the Mac app
  [https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md](https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md);
  desktop remains Mac-only, cloud workspaces run on Linux
  [https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md](https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md).

### 3.3 Juggler today: cwd only, no worktree, no branch

Juggler's session model carries only a working directory:
`pub cwd: PathBuf` (`crates/juggler-core/src/model.rs:113`,
mirrored in `docs/SPEC.md:113`). There is no worktree, branch,
or isolation field in `crates/` or `ui/src/` — a `grep` for
`worktree` across both trees returns zero hits (verified
in-session). The parent review's F1.1 is therefore exact:
"SPEC.md:113 gives a session only a `cwd`. PROPOSAL.md:75
promises parallel accounts on the same repo"
(`docs/review/full-competitive-ux-review.md:64-66`), and
"Juggler has no branch concept outside a pack field
(SPEC.md:301)" (`docs/review/full-competitive-ux-review.md:68-69`).
Consequence: two Juggler sessions on one repo share one
working tree. Both competitors make that state impossible
by construction. This is the single largest structural gap
in this file — see severity table in §10.

**Steal:** Squad's atomic `tmux+worktree` creation and
ownership-scoped cleanup. **Beat:** add account awareness
Squad lacks (worktree branch named by profile, §10).
**Avoid:** Conductor's mandatory worktree for *every*
workspace — Juggler should offer worktree-on-launch as an
option (parent review F1.1 recommendation,
`docs/review/full-competitive-ux-review.md:70-74`), because
read-only/exploratory sessions do not need isolation and
forced worktrees surprise users on non-git folders.

---

## 4. Parallel sessions — how each tool scales the daily loop

### 4.1 Claude Squad: flat list, preview pane, background daemon

- Interaction model: "flat **instance list + preview pane +
  diff tab**" in one TUI window
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md);
  "Central instance list, preview pane, diff tab, auto-accept
  background mode" [https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/04-agent-managers.md).
- Background execution: instances run inside tmux, so closing
  the TUI does not kill agents; a polling daemon (`daemon.pid`,
  `daemon_poll_interval` default 1000 ms) watches for prompts
  when autoyes is on
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md).
- Concurrency ceiling is human, not technical: "4-8 worktrees
  per developer reliably; above that you're bottlenecked on
  review, not on Claude"
  [https://github.com/t1mdurden/prd-pipeline/blob/HEAD/plugins/prd-pipeline/skills/prd-pipeline/references/research-notes.md](https://github.com/t1mdurden/prd-pipeline/blob/HEAD/plugins/prd-pipeline/skills/prd-pipeline/references/research-notes.md).
  Squad offers no triage queue to raise that ceiling — the
  list *is* the queue.

### 4.2 Conductor: workspace list per repo, dashboard triage

"Workspace list per repo, multiple repos in one window"
[https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md);
start from branch/PR/GitHub/Linear issue (parent review
`docs/review/full-competitive-ux-review.md:620-621`) or open
in Cursor/VS Code [https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx);
changelog shows ongoing workspace-status investment
[https://www.conductor.build/changelog](https://www.conductor.build/changelog).

### 4.3 Juggler today: rail + embedded terminal, capped lists

Juggler's daily loop is a left rail (Projects / Instances /
Sessions) with embedded live terminals
(`docs/internal/agent-review-prompt.md:51-53`). Two scale
defects are already documented and confirmed in code:
the rail "caps at `slice(0,15)` sessions / `slice(0,8)` recents
with no search or filter (WorkflowRail.tsx:184-195,328-384)"
(`docs/review/full-competitive-ux-review.md:205-207`), and
`Stuck`/`WaitingUser`/`QuotaBlocked` "are modelled
(model.rs:188-207) but never emitted (commands.rs:782-810)"
(`docs/review/full-competitive-ux-review.md:29-32`).
Neither Squad nor Conductor has a needs-attention inbox
either — but Conductor's Checks tab and Squad's preview
pane at least surface *state*. Juggler currently surfaces
*presence* (running/stopped) without *need* (waiting/stuck).
At 15 sessions that is the difference between supervision
and a process list. Parent review WOW-a (Needs-Attention
inbox, `docs/review/full-competitive-ux-review.md:765-768`)
is the correct response and neither competitor blocks it.

---

## 5. Multi-account & quota — the explicit test

This is Juggler's wedge (G2/G3, `docs/PROPOSAL.md:32-33`).
The tasking asked to *explicitly test* whether either
competitor does multi-account juggling. Result: **neither
does, in upstream/official sources.** Evidence below;
forks that add it prove the gap by existing.

### 5.1 Claude Squad upstream: profiles are programs, not accounts

- Config schema: `default_program`, `auto_yes`,
  `daemon_poll_interval`, `branch_prefix`, and
  `profiles[{name,program}]` in `~/.claude-squad/config.json`
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md);
  example: `{"name": "aider-gpt4", "program": "aider --model gpt-4"}`
  [https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md](https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md).
  A "profile" selects *which binary + flags* to launch. It
  carries no credential, no `CLAUDE_CONFIG_DIR`, no quota.
- CLI surface confirms it: `-p/--program string  Program to
  run in new instances (e.g. 'aider --model ollama_chat/gemma3:1b')`
  and `-y/--autoyes` are the only session-shaping flags
  [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
- Parent review verdict stands: "Quota/accounts: absent
  upstream — config holds no rotation/pooling/quota; auth is
  delegated to each CLI's login/env"
  (`docs/review/full-competitive-ux-review.md:589-591`).
  `[negative evidence]` — no rotation/pooling/quota key
  exists in the documented config schema above; absence is
  inferred from the complete key list, not from a vendor
  "not supported" statement.

### 5.2 The forks prove the gap (do not attribute to upstream)

- `terrene-foundation/claude-squad` adds exactly what upstream
  lacks: "`cc login N`: saves current account's creds to
  credentials/N.json; `/rotate`: writes best account's creds
  to ~/.claude/.credentials.json," later refactored to
  per-terminal `CLAUDE_CONFIG_DIR` isolation with a quota
  statusline [https://github.com/terrene-foundation/claude-squad/commit/d3608765830da91ef4ef92ca3ee6bceb08db54d0](https://github.com/terrene-foundation/claude-squad/commit/d3608765830da91ef4ef92ca3ee6bceb08db54d0),
  [https://github.com/terrene-foundation/claude-squad/commit/faa2fe375903ae392d120a0f641025730f871127](https://github.com/terrene-foundation/claude-squad/commit/faa2fe375903ae392d120a0f641025730f871127).
  The fork even records the failure mode Juggler must avoid:
  "kill auto-rotate to stop random account switching"
  [https://github.com/terrene-foundation/claude-squad/commit/035e748a04e12c44c51a23c6e7a05371b981dbb3](https://github.com/terrene-foundation/claude-squad/commit/035e748a04e12c44c51a23c6e7a05371b981dbb3).
- `almanies2025/claude-squad` markets itself as "Multi-account
  rotation for Claude Code — intelligent quota management
  across Claude Max subscriptions" with "per-terminal
  isolation, shared history, and a statusline"
  [https://github.com/almanies2025/claude-squad](https://github.com/almanies2025/claude-squad) —
  again a fork, again absent upstream.
- A private fork notes the same: "The multi-account separation
  via CLAUDE_CONFIG_DIR per profile is the most valuable
  enhancement — it solves a problem Conductor itself cannot
  solve" [https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md](https://github.com/franzfelberer/claude-squad-fork/blob/HEAD/CLAUDE.md).
  That sentence, from a Squad+Conductor user, is the wedge
  in one line.

### 5.3 Conductor: BYO subscription, no switcher, no quota UI

- Model: "you bring your own subscription" / "reusing your
  existing agent subscriptions"
  [https://github.com/laurence-nz/superset-pwsh/blob/HEAD/apps/marketing/content/compare/conductor-vs-cursor.mdx](https://github.com/laurence-nz/superset-pwsh/blob/HEAD/apps/marketing/content/compare/conductor-vs-cursor.mdx);
  "Both require you to pay the underlying AI providers
  (Anthropic, OpenAI, etc.) for agent usage"
  [https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx).
- Parent review: "**No Conductor-native multi-account switcher
  or quota dashboard found** in docs/searches (negative
  evidence; not confirmed absent from in-app settings)"
  (`docs/review/full-competitive-ux-review.md:614-616`).
  This deep dive re-tested with docs/changelog/pricing
  queries (§5 queries in research log): no official page
  describes account switching, quota rings, or reset
  countdowns. `[negative evidence]` remains the correct
  label — in-app settings cannot be inspected from docs.
- The pain underneath is real and unaddressed by Conductor:
  "a Claude Pro $20 5h cap is easily hit by 3–5 parallel
  agents" (`docs/review/full-competitive-ux-review.md:616-617`).
  Parallelism *multiplies* quota burn; Conductor accelerates
  the burn without instrumenting it.

### 5.4 Juggler: modelled, specced, not yet shipped

- Domain model already carries the shape: `QuotaWindow`
  with `used_percentage` + `resets_at`
  (`crates/juggler-core/src/model.rs:279-284`) and honesty
  tiers `Measured/Computed/Estimated`
  (`crates/juggler-core/src/model.rs:286-297`), attached per
  session via `UsageSnapshot` (`crates/juggler-core/src/model.rs:301-307`).
- Spec promises the feed: statusline JSON carrying
  `rate_limits.five_hour/seven_day.used_percentage/resets_at`
  captured as **Measured** (`docs/SPEC.md:192`), with quota
  rings + reset countdowns in Overview (`docs/SPEC.md:345`)
  and tray (`docs/SPEC.md:352`) — but milestones place the
  usage engine at M3 (`docs/SPEC.md:453`), after M2 adapters.
- Accounts themselves are first-class: named profiles with
  isolated state dirs via `CLAUDE_CONFIG_DIR`-style env
  overlay (`crates/juggler-adapter/src/adapter.rs:44`,
  `crates/juggler-adapter/src/environment.rs:34`), versus
  Squad's program-strings and Conductor's single ambient
  login. Only Claude is registered today
  (`crates/juggler-app/src/state.rs:91-96`).

**Test conclusion:** the juggling layer is unclaimed by both
competitors *architecturally* — Squad would need a credential
model it deliberately omits; Conductor would need to break
its "one login, many workspaces" simplicity. Juggler's risk
is not being beaten to quota — it is shipping adapters (M2)
before quota (M3) and arriving at the same blind spot.
Parent review F6.1 (`docs/review/full-competitive-ux-review.md:427-439`)
is the fix; this section is its competitive proof.

---

## 6. Diff / accept / merge flow — where Conductor wins outright

### 6.1 Claude Squad: preview, then keyboard commit verbs

- Flow: "Tab preview/diff, review-before-apply; `s` = commit+push
  branch, `c` = checkout (commit+pause: keeps branch, removes
  worktree), `r` = resume" (parent review,
  `docs/review/full-competitive-ux-review.md:595-598`).
- Semantics matter: "`c` (checkout) commits work locally with a
  templated message, removes the worktree, preserves the branch,
  and copies the branch name to the system clipboard. `r` (resume)
  recreates the worktree and reattaches the tmux session"
  [https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md).
- The verbs were buggy in exactly the trust-critical spot:
  "`c` Command Checkout Locally Only" — users found checkout
  pushed to GitHub first, contrary to "checkout changes before
  pushing them"; fix made `c` commit locally only, leaving push
  to the separate command [https://github.com/smtg-ai/claude-squad/pull/123](https://github.com/smtg-ai/claude-squad/pull/123).
  Lesson: in a review flow, *when* a push happens is a trust
  boundary. Juggler's passive invariant
  (`docs/internal/agent-review-prompt.md:64-66`) must make push
  an explicit, labelled click — never a side effect of pause.
- Diff itself is terminal-grade: "preview diffs, commit/push
  (`s`), commit-and-pause (`c`)" with isolation as the safety
  story [https://github.com/kruxia/foundry/blob/HEAD/docs/research/2026-10-03-agentic-coding-harness-notes/harness_and_orchestration.md](https://github.com/kruxia/foundry/blob/HEAD/docs/research/2026-10-03-agentic-coding-harness-notes/harness_and_orchestration.md).
  No inline comments, no comment-to-agent loop. `[negative
  evidence]` — no comment feature appears in README/help
  sources above.

### 6.2 Conductor: the review cockpit (steal target #1)

- "Conductor's strongest code-surface feature is review
  ergonomics: docs and changelog emphasize a dedicated diff
  viewer and review pipeline (`Cmd+D`, review/fix/merge
  actions), plus 'checks' support"
  [https://docs.conductor.build/core/diff-viewer](https://docs.conductor.build/core/diff-viewer)
  (via [https://github.com/debuglebowski/slayzone/blob/HEAD/comparison/_legacy/conductor.md](https://github.com/debuglebowski/slayzone/blob/HEAD/comparison/_legacy/conductor.md)).
- Official diff docs: "The Diff Viewer shows the code changes
  in a workspace. Use it before creating a pull request, after
  an agent makes changes, and whenever review comments need
  another pass" — checking "Files changed by the agent.
  Unrelated or accidental edits. Missing tests or docs. Local
  comments and GitHub review comments"
  [https://www.conductor.build/docs/reference/diff-viewer](https://www.conductor.build/docs/reference/diff-viewer).
- The killer detail: "clicking a line in the diff and adding
  a comment routes that comment to the agent as **targeted
  context with file+line metadata**, not as a chat message"
  [https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md](https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md).
  That closes the review loop without context-switching —
  the exact loop parent review flags as missing in Juggler
  (F7.3, `docs/review/full-competitive-ux-review.md:497-505`).
- Full pipeline in one surface: "Diff Viewer, terminal/Spotlight
  tests, Checks tab (git status, CI, deployments, comments,
  todos), create PR, merge, archive" (parent review,
  `docs/review/full-competitive-ux-review.md:620-623`); third
  parties confirm "Review workspace changes, comments, and
  PR-ready diffs" as the documented purpose
  [https://www.conductor.build/docs/reference/diff-viewer](https://www.conductor.build/docs/reference/diff-viewer).
- Independent verdict: Conductor's "in-app review-and-merge
  cockpit [is] the single feature that most clearly beats
  [a competitor] on product craft"
  [https://github.com/thefrederiksen/devthrottle/issues/1146](https://github.com/thefrederiksen/devthrottle/issues/1146).

### 6.3 Juggler today: terminal only, no diff surface

A `grep` for `diff|Diff|branch` across `ui/src/components/*.tsx`
and `ui/src/screens/*.tsx` returns zero hits (verified
in-session). Sessions embed their live terminal
(`docs/internal/agent-review-prompt.md:52`) and handover copies
a transcript (`crates/juggler-app/src/commands.rs:1088-1194`),
but there is no changed-files view, no diff, no PR action.
The spec's only diff mention is inside the handover pack
manifest (`docs/SPEC.md:301`). So Juggler today asks users
to review agent work in an external editor/`git diff` —
while both competitors keep review in-app. Severity: P1
(not P0 — review can happen externally; quota blindness
cannot). Sequence: worktree first (makes diff meaningful),
read-only diff second, comment-to-agent last (parent review
F7.3 ordering, `docs/review/full-competitive-ux-review.md:502-505`).

---

## 7. Multi-harness, pricing/licence, platform

### 7.1 Multi-harness breadth vs depth

| Axis | Claude Squad | Conductor | Juggler |
|---|---|---|---|
| Harnesses | Any CLI via `-p` [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad) | Claude Code, Codex, Cursor, OpenCode [https://www.agentastic.dev/alternatives/paseo](https://www.agentastic.dev/alternatives/paseo) | Claude only (`crates/juggler-app/src/state.rs:91-96`) |
| Contract | Program string, no adapter API | Built-in, closed | Typed trait (`crates/juggler-adapter/src/adapter.rs:26`) |
| Per-harness depth | Shallow (spawn + tmux) | Deep (bundled Codex updates [https://www.conductor.build/changelog](https://www.conductor.build/changelog)) | Deep per harness, narrow count |
| Continuation | tmux reattach + `claude --continue` [https://github.com/3jins/claude-squad-restore](https://github.com/3jins/claude-squad-restore) | Workspace persists; open in IDE [https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/yzgershon/gatedspace/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx) | Transcript transplant + `--resume` (`crates/juggler-app/src/commands.rs:1088-1194`) |

Squad breadth is thin (same spawn path, no transcript/quota/
handover semantics). Juggler's trait
(`crates/juggler-adapter/src/lib.rs:6-7`) trades breadth for
juggling depth — correct only if adapter #2 ships with the
conformance kit (parent review F1.8/F5.5).

### 7.2 Pricing & licence — parent review correction

- **Claude Squad: free, AGPL-3.0.** "AGPL-3.0 — *forking is
  viral-license territory; read-for-ideas only*"
  [https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md](https://github.com/gregce/tortie/blob/HEAD/docs/research/10-multi-project-ux.md);
  licence + 7,171 stars in the structured snapshot
  [https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md](https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md).
  Parent review's platform table ("Claude Squad | macOS/Linux
  TUI | AGPL-3.0, free," `docs/review/full-competitive-ux-review.md:724`)
  is correct. Implication: Juggler (MIT, `Cargo.toml:17`) can
  *read* Squad for ideas but must not copy AGPL code. Ideas
  (verbs, lifecycle) are free; implementation must be original.
- **Conductor: free today; parent review's $50/$60 is stale.**
  Parent review states "Free local / Pro $50/mo / Teams $60"
  (`docs/review/full-competitive-ux-review.md:625`) and
  "Free / $50 Pro" (`docs/review/full-competitive-ux-review.md:725`).
  Current sources say "Conductor is free. Both require you to
  pay the underlying AI providers"
  [https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/per-simmons/damon-ade/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx),
  "Proprietary, free" [https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md](https://github.com/rodgi040/secret-ai-beaver-sauce/blob/HEAD/skills/secret-ai-beaver-source/references/TOOLS.md),
  and pricing is gated/unclear behind the official page
  [https://www.conductor.build/pricing](https://www.conductor.build/pricing)
  (via [https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md](https://github.com/indosaram/ferryx/blob/HEAD/docs/SITE_SEARCH_PRIORITIES_2026-09-19.md)).
  One comparison still lists team/cloud gating
  [https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx).
  **Correction:** treat Conductor as freeware-with-paid-team/
  cloud upsell `[unresolved exact tiers]` — do not repeat
  $50/$60 as fact. Either way, Juggler cannot win on price
  against free; it wins on openness + juggling.
- **Juggler: MIT, free, signed macOS distribution planned**
  (`README.md:83`, `docs/SPEC.md:359-360`). Against AGPL
  (Squad) and proprietary freeware (Conductor), MIT +
  local-only is the trust-maximising licence — say so.

### 7.3 Platform

- Squad: macOS/Linux TUI; requires `tmux` + GitHub CLI `gh`
  ("It needs tmux and the GitHub CLI"
  [https://github.com/agentlistio/awesome-agent-clients](https://github.com/agentlistio/awesome-agent-clients)).
  No Windows story in official sources. `[negative evidence]`
  for Windows support.
- Conductor: "The desktop app is Mac-only; cloud workspaces
  run on Linux and can be driven from the Mac app or the iOS
  app" [https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md](https://github.com/thomaschristopherking/conductor-android/blob/HEAD/docs/conductor-skill.md);
  iOS is "Coming soon" in one comparison
  [https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-conductor.mdx) —
  conflicting signals, flag `[unresolved]` for iOS status.
- Juggler: "macOS 13+ (Universal). Windows/Linux post-1.0;
  the Rust core and adapters are cross-platform from day one"
  (`docs/SPEC.md:18`). Squad already covers Linux terminal
  users Juggler defers; Conductor covers cloud/Linux compute
  Juggler defers to Fleet (M6+). Both deferrals are fine —
  but marketing must not imply parity today.

---

## 8. Review UX, notifications & failure modes (compact)

**Review UX.** Squad: keyboard-first preview/diff, no inline
comments `[negative evidence, §6.1]`. Conductor: visual diff
viewer + line comments routed to agent as file+line context
[https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md](https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md),
Checks tab for CI/todos (parent review
`docs/review/full-competitive-ux-review.md:620-623`). Juggler:
no diff UI (zero hits, §6.3); review happens in the embedded
terminal or externally. **Notifications.** Neither competitor
documents a quota/waiting notification system: Squad relies on
the TUI list + autoyes daemon polling
[https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md](https://github.com/mkb23/overcode/blob/HEAD/docs/design/bakeoffs/overcode-vs-claude-squad.md);
Conductor changelog emphasises workspace status/navigation,
not alerts [https://www.conductor.build/changelog](https://www.conductor.build/changelog).
`[negative evidence]` for both. Juggler specs tray alerts
(`docs/SPEC.md:352`) but ships none (`grep` tray/notification
in `ui/src/` + `crates/` = zero hits, verified in-session) —
an open lane: quiet digest + needs-attention badge (parent
review WOW-g, `docs/review/full-competitive-ux-review.md:788-790`)
beats both by existing. **Failure modes.** Squad: resumed
sessions fail when worktree creation changes CWD — "The tmux
session starts in the worktree path instead of the original
project path… the agent starts in the wrong directory, can't
find the session to resume, exits immediately"
[https://github.com/rakesh97/claude-squad/issues/28](https://github.com/rakesh97/claude-squad/issues/28);
reboot loses tmux sessions (history survives, sessions do
not) [https://github.com/3jins/claude-squad-restore](https://github.com/3jins/claude-squad-restore);
multi-instance tmux prefix collisions need manual config
[https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad).
Conductor: closed source = unauditable failure surface;
quota burn at 3–5 parallel agents (§5.3) with no in-app
guardrail `[negative evidence]`. Juggler: handover picks
newest-by-mtime JSONL, no source freeze, `replace('/','-')`
cwd guess (`crates/juggler-app/src/commands.rs:1088-1194`;
parent review F3.3, `docs/review/full-competitive-ux-review.md:264-287`) —
same class of resume fragility as Squad's CWD bug. Fix both
with `transcript_ref` (`crates/juggler-core/src/model.rs:314-321`)
+ freeze-before-copy.

## 9. Handover/continuation — the asymmetric feature

Neither competitor hands work *between accounts*. Squad
continuation is same-account tmux reattach
[https://github.com/3jins/claude-squad-restore](https://github.com/3jins/claude-squad-restore);
Conductor continuation is the PR (humans/CI carry context).
Juggler's transcript transplant + `--resume`
(`crates/juggler-app/src/commands.rs:1088-1194`) with lineage
(`SessionStatus::HandedOver`, `crates/juggler-core/src/model.rs:188-207`)
is category-unique — and currently fragile (F3.3) and
over-claimed (`TRANSPLANT|FORK` advertised at
`crates/juggler-adapters/src/claude_code.rs:111-119` while
`crates/juggler-handover/src/lib.rs:1-13` is doc-only; parent
review F3.1). Protect the uniqueness, fix the honesty first.

## 10. Feature-by-feature verdict (Gap = Juggler gap severity)

| Feature | Squad | Conductor | Juggler (repo) | Gap | Verdict |
|---|---|---|---|---|---|
| Worktree isolation | Yes (§3.1) | Yes (§3.2) | No (`model.rs:113`) | **P0** | **Steal** Squad atomicity |
| Parallel list/triage | List+preview | Workspace dashboard | Rail, capped (`WorkflowRail.tsx:184-195`) | P1 | Beat with inbox (WOW-a) |
| Diff review | Preview diff | Diff+comments→agent | None (§6.3) | P1 | **Steal** Conductor cockpit |
| Accept/merge/PR | `s`/`c`/`r` verbs | PR→merge→archive | None | P1 | Steal verbs, explicit push |
| Multi-harness | Any CLI `-p` | 4 built-in | 1 (`state.rs:91-96`) | P1 | Beat on depth, not count |
| Multi-account | No (§5.1) | No (§5.3) | Yes, first-class (§5.4) | — | **Beat: defend wedge** |
| Quota display | No `[neg. ev.]` | No `[neg. ev.]` | Modelled M3 (`SPEC.md:453`) | **P0** | Ship before adapters |
| Handover lineage | Branch only | PR only | v0 transplant (§9) | P1 | Fix honesty (F3.1–F3.4) |
| Autoyes/auto-accept | `-y` experimental | No | Forbidden by invariant | — | **Avoid** (breaks click-bound rule) |
| Notifications | None `[neg. ev.]` | None `[neg. ev.]` | Specced, unbuilt (`SPEC.md:352`) | P1 | Beat by shipping digest |
| Setup/run scripts | No | Yes (§3.2) | No | P2 | Steal later |
| Cloud/mobile | No | Cloud+iOS `[unresolved]` | Fleet M6+ | P2 | Avoid now |
| Licence/trust | AGPL free | Proprietary free | MIT (`Cargo.toml:17`) | — | Beat: lead with openness |

**Bottom line:** copy isolation and review craft; never copy
automation that acts without a click; win on the one axis
both competitors structurally omit — quota-aware juggling
across named accounts. *End of deep profile 05.*
# 06 — Vibe, Crystal & the Switchers: quota/switching table stakes

> Section 06 of the deep review. Parent: `docs/review/full-competitive-ux-review.md`.
> Question: what do Vibe Kanban, Crystal, cc-switch, ccusage,
> claude-swap/`cswap`, `aisw`, and the native Claude Code / Codex
> usage surfaces actually do — and which of their behaviours are
> day-1 table stakes for Juggler's juggling promise?
> Method: competitor web research (URLs inline) + repo inspection
> (`file:line`). No live competitor install was run; install-time
> claims are labelled `[docs]` vs `[observed-in-repo]`.

## Verdict in five lines

- Orchestrators (Vibe, Crystal) solved parallel worktrees, never quota/rotation; their deaths orphan *parallel-session* users, so winning them needs worktrees + diff review first.
- Switchers (`cc-switch`, `cswap`, `aisw`, `ccusage`) solved quota visibility + one-click switching, but none supervises live sessions (no stuck/lineage/handover).
- Native baselines rose in 2026 (statusline `rate_limits`, Codex `/status`); any Juggler quota UI less truthful than `/usage` will be judged against `/usage`.
- Juggler advertises unimplemented quota/handover: `USAGE_QUOTA_FEED|TRANSPLANT|FORK` in [claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L111) vs `percent={0}` rings + `ComingSoon` Usage in UI.
- The open niche is *quota-aware session supervision*, won on §8's day-1 behaviours — not harness breadth.

## 1. How to read this section

### 1.1 Honesty tiers (Juggler's own vocabulary)

Juggler models trust explicitly: `Confidence::{Measured, Computed,
Estimated}` with `Estimated` as the safe default in
[model.rs](../../crates/juggler-core/src/model.rs#L286).
The spec's honesty rule — "every number in the UI carries its
confidence" — is stated in
[SPEC.md](../SPEC.md#L254).
This section grades every competitor number against the same tiers:

- **Measured** — vendor-reported %/reset (statusline feed, `/usage`,
  OAuth usage API, Codex `token_count` snapshots).
- **Computed** — deterministic local math (JSONL tokens × price).
- **Estimated** — inferred burn vs an assumed plan cap.

Most switcher marketing collapses these three into one number.
Juggler's differentiation is to *not* collapse them — if the UI
keeps the labels honest.

### 1.2 What "switching" means (four different verbs)

| Verb | Meaning | Who does it |
|---|---|---|
| Config switch | Rewrite live config to another provider/key | cc-switch |
| Account switch | Swap the active login credential set | cswap, aisw |
| Session transplant | Move one live conversation to another account | Juggler (specced) |
| Auto-rotation | Switch without asking when quota crosses a line | cswap `auto` |

Confusing these four is the #1 positioning error in this category.
Juggler's spec keeps them separate: switch-profile (one click, new
sessions only), takeover (two clicks), return (one click, suggested)
in [SPEC.md](../SPEC.md#L284).
Competitors mostly ship only the first two verbs.

### 1.3 Evidence key

- `[repo]` — verified in this repository at the cited `file:line`.
- `[docs]` — competitor README/docs/changelog via web research.
- `[inferred]` — synthesis judgement; flagged so the reader can
  discount it. Star counts are snapshots, never rankings.

---
## 2. Native baselines first: `/usage`, statusline, Codex `/status`

Any third-party quota UI now competes with the vendor's own UI.
This is the bar — and it rose during 2026.

### 2.1 Claude Code: `/usage` + statusline `rate_limits`

- `[docs]` Since Claude Code v2.1.80+, the statusLine command
  receives a `rate_limits` object on stdin (`five_hour` +
  `seven_day`, each `used_percentage` + `resets_at`) — official,
  deterministic, zero token cost; reset epochs match the `/usage`
  TUI exactly. Print-mode `/usage` scraping is dead on v2.1.207
  (insights-only). Source: [sergeiwallace/ai-cli-utils research commit](https://github.com/sergeiwallace/ai-cli-utils/commit/54baf74424d2fcaaaa1b2efb99e1b1d6a05cdde8).
- `[docs]` A dependency-free statusline shows model, context %,
  5h/weekly rate limits, tokens and session cost on one line:
  [savinofiore/claude-usage-statusline](https://github.com/savinofiore/claude-usage-statusline).
  The 5-hour window is rolling (resets 5h after first message);
  the 7-day window is weekly rolling: [AndyJae/claude-code-usage](https://github.com/AndyJae/claude-code-usage).
- `[docs]` `cc-limits` shows the same in realtime: context, 5h,
  7d with time-until-reset: [sslinnn/cc-limits](https://github.com/sslinnn/cc-limits).
  A quota shim can persist the statusline snapshot for other tools:
  [ffffhx/open-token-board commit](https://github.com/ffffhx/open-token-board/commit/b62cce6629faa52e99e9d5572eafa81583426a29).

**Implication for Juggler.** The spec already names the right
source: "install a managed statusline script per profile dir; its
stdin JSON carries `rate_limits.five_hour/seven_day`" as **Measured**
in [SPEC.md](../SPEC.md#L191).
The domain type exists — `QuotaWindow { used_percentage,
resets_at }` in
[model.rs](../../crates/juggler-core/src/model.rs#L277) —
and the capability bit exists in
[capabilities.rs](../../crates/juggler-adapter/src/capabilities.rs#L21).
But grep finds no statusline installer and no producer of a
non-default `UsageSnapshot`: sessions are created with
`UsageSnapshot::default()` in
[commands.rs](../../crates/juggler-app/src/commands.rs#L706).
`[repo]` The feed is *declared*, not *wired*.

### 2.2 Codex: `/status`, `/wham/usage`, `token_count` snapshots

- `[docs]` Inside Codex CLI, `/status` prints remaining 5-hour
  and weekly quota % with exact reset timestamps (e.g. CLI 0.153.4,
  Oct 2026): [codexusage.dev guide](https://www.codexusage.dev/).
- `[docs]` There is no `codex usage` subcommand; usage lives in
  the TUI status card/status line, polled from `/wham/usage` every
  ~60s, flagged stale after 15 min: [wez/quotas docs](https://github.com/wez/quotas/blob/HEAD/docs-usage/codex.md).
- `[docs]` Every `token_count` event in Codex session logs carries
  `rate_limits` (`primary.used_percent`, `window_minutes: 300`,
  `resets_at`) — parseable without any API call:
  [Medium quota-app writeup](https://medium.com/@vinayanand2/how-much-ai-quota-do-you-have-left-i-built-an-app-for-easy-access-11a53791a064).
  A macOS menu-bar reader exists: [sangimed/codex-menubar](https://github.com/sangimed/codex-menubar).
- `[docs]` There is **no** public stable "remaining %" API; the
  honest options are passive (parse snapshots, classify limit
  errors) rather than reverse-engineering private routes:
  [jimyungsik/omo-account-router design](https://github.com/jimyungsik/omo-account-router/blob/HEAD/docs/usage-remaining-design.md).

**Implication for Juggler.** The spec's Codex stance — token burn
Measured, plan-credit correlation Estimated and labelled — in
[SPEC.md](../SPEC.md#L198)
is *more* honest than most tools, but it is beatable: parsing the
`token_count` snapshots upgrades Codex quota from Estimated to
Measured for the primary/secondary windows. `[inferred]` That
parse should be scheduled with the Codex adapter, not after it.

### 2.3 Baseline scoreboard (what a user gets with zero extra tools)

| Surface | Claude Code native | Codex native |
|---|---|---|
| Current-window % | `/usage` + statusline | `/status` in TUI |
| Reset time | Epoch + countdown | Exact timestamp |
| Multi-account view | None (one login) | None (one login) |
| Cost in USD | Session cost in statusline | Not subscription % |
| History/trends | None | None |

The gap in the last three rows *is* the third-party market.
Everything else a switcher adds must beat row 1–2 on accuracy.

## 3. Vibe Kanban (Bloop) — the kanban orchestrator that died

### 3.1 What it does

`[docs]` Kanban-over-agents: cards move To Do → In Progress →
Review → Done; moving a card to In Progress auto-creates a git
worktree and starts an agent. Agent-agnostic (Claude Code, Codex,
Gemini CLI, Copilot, Amp, Cursor, OpenCode…), own MCP server,
integrated diff view with inline comments sent back to the agent,
PR creation via `gh`. Repo: [BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban).
Parent review summary is in the full review's Vibe deep-dive.

### 3.2 Multi-account switching mechanics

None native. `[docs]` Vibe is BYO-subscription/keys with no
rotation or per-account dashboard (parent review, Q7). Switching
"accounts" means switching the agent binary on a card — a harness
switch, not a quota switch. There is no equivalent of Juggler's
switch-profile/takeover/return verbs (§1.2).

### 3.3 Quota data source / accuracy / reset handling

None. No 5h/weekly %, no reset countdown, no burn-down. Quota
pain is delegated to the underlying CLIs — exactly the gap the
parent review calls out: orchestrators delegate quota/accounts
to the CLIs (full review Q7). `[inferred]` A Vibe user learns
about the quota wall when a card's agent dies mid-attempt.

### 3.4 Cost analytics

None subscription-oriented. No USD ledger, no per-account cost
rollup. (Cost analytics live in the separate `ccusage` ecosystem,
§6.) Vibe's data model is tasks-in-SQLite, not usage events.

### 3.5 Session features

Strongest-in-class parallel UX: worktree+branch per task-attempt
(per repo in multi-repo), terminal + dev-server + preview browser
per workspace, diff review loop, guarded squash-merge/archive.
Weaknesses (parent review): tasks in SQLite not git; manual
checkouts/external PRs unrecognised; worktree deletion loses the
card link; per-CLI output adapters brittle; no rules/quality gates.

### 3.6 Pricing / OSS / status — the sunset

- Free/self-hosted, Apache-2.0 (some posts wrongly say MIT).
- `[docs]` "Upstream is sunsetting. Bloop, the company behind
  vibe-kanban, shut down on 2026-04-10, and v0.1.44 (2026-04-24)
  is the last release": [docker-vibe-kanban REMOTE_ACCESS](https://github.com/gwwtests/docker-vibe-kanban/blob/HEAD/docs/REMOTE_ACCESS.md).
- `[docs]` Dead-project audit: "Last push 2026-04-24; 0 PRs merged
  in the 90 days since; 151 open PRs stranded": [kayushkin/llm-bridge RELEASE-TARGETS](https://github.com/kayushkin/llm-bridge/blob/HEAD/RELEASE-TARGETS.md).
- `[docs]` Homepage notice "Vibe Kanban is sunsetting… hosted
  offering winding down, maintenance by volunteers": [aq.dev alternatives page](https://aq.dev/alternatives/vibe-kanban/).
  A local-first fork continues it: [mdevel-uy/vibe-kanban](https://github.com/mdevel-uy/vibe-kanban).

### 3.7 Why users love it / leave it

Love: the card=worktree=agent metaphor — the most-copied UX in
the category (parent review). Leave: not by choice — the vendor
died. `[inferred]` The forced migration is the opportunity; see §9
for sizing and what these users actually need (worktrees + diff
review), which is *not* Juggler's current wedge.

### 3.8 vs Juggler (feature-by-feature)

`[repo]` Juggler has no worktree/branch isolation: a session has
only a `cwd` (parent review F1.1). Vibe wins isolation, diff
review, PR flow. Juggler (specced) wins quota + handover lineage:
takeover/return in
[SPEC.md](../SPEC.md#L284).
Neither ships the other's strength today. Do not pitch Juggler
to Vibe orphans on quota alone — pitch it once worktree-on-launch
exists, or explicitly as the *quota companion* beside their next
orchestrator.

## 4. Crystal (Stravu) → Nimbalyst — the worktree GUI that pivoted

### 4.1 What it does

`[docs]` MIT Electron desktop: parallel Claude Code/Codex
sessions, each in a git worktree; session list + diff/compare
panes; rebase/squash tooling. Repo: [stravu/crystal](https://github.com/stravu/crystal).
Crystal is the cleanest reference for "compare two agents'
approaches side by side" (parent research summaries).

### 4.2 Multi-account switching mechanics

None as a switcher. Crystal isolates by *worktree*, not by
*account*: parallel sessions share the user's single CLI login.
There is no named-profile registry, no per-account quota, no
rotation. (Same delegation pattern as Vibe, §3.2.)

### 4.3 Quota / cost / session features

- Quota: none native (no 5h/weekly, no resets). Cost: none.
- Sessions: worktree manager, terminal-session manager,
  git-diff/status managers, pluggable Claude Code + Codex CLI
  registry — `[docs]` file inventory in [vincenthanxiaodu/oh-my-term research](https://github.com/vincenthanxiaodu/oh-my-term/blob/HEAD/docs/research/remote-agent-products.md).
  The durable pattern is "GUI derived from the CLI's own on-disk
  state" — which Juggler's transcript watcher also follows.

### 4.4 Pricing / OSS / deprecation

- MIT, ~3.1k stars at deprecation. `[docs]` "Crystal (formerly a
  multi-session AI code assistant manager) has been deprecated
  and replaced by **Nimbalyst**. Deprecated: February 2026":
  [stravu/crystal README](https://github.com/stravu/crystal).
  Migration points to [nimbalyst.com/crystal](https://nimbalyst.com/crystal/).
- `[docs]` Analysis of the pivot: "session-management-on-top-of-
  terminals was too thin a wedge" — the team moved to a multi-
  editor AI workspace: [dearken10/baton prior-art](https://github.com/dearken10/baton/blob/HEAD/docs/prior-art.md).
  Corroborated as "deprecated 2026-02 and replaced by Nimbalyst,
  an AI-native workspace": [alp82/curia research](https://github.com/alp82/curia/blob/HEAD/docs/research/agent-face-tools.md).

### 4.5 Why users love it / leave it

Love: polished per-agent workspace UX, side-by-side compare.
Leave: forced — deprecated Feb 2026 (v0.3.5), successor is
commercial Nimbalyst. Parent review: "beloved parallel-session
UX, unsustained as a free standalone" (full review).

### 4.6 vs Juggler + the cautionary lesson

`[repo]` Juggler's supervisor + embedded-terminal design is the
same wedge Crystal walked away from. The difference must be
*supervision value* (stuck/quota/lineage), not terminal hosting:
Juggler models `Stuck`/`WaitingUser`/`QuotaBlocked` in
[model.rs](../../crates/juggler-core/src/model.rs#L188)
but — per parent review F2.5 — never emits the last three.
`[inferred]` If those statuses stay unemitted, Juggler repeats
Crystal's mistake with better types. Crystal's MIT code remains
a legitimate reference for diff/compare UX; its users need a
free worktree GUI *now*, which Juggler is not yet (§3.8).

## 5. cc-switch (farion1231) — the config-switching incumbent

### 5.1 What it does

`[docs]` "A cross-platform desktop All-in-One assistant for
Claude Code, Codex, OpenCode, OpenClaw, Grok Build & Hermes
Agent" — switch API providers in one click and manage MCP,
Skills and Prompts in one place, "no more hand-editing JSON /
TOML / YAML config files": [farion1231/cc-switch](https://github.com/farion1231/cc-switch).
Official site: ccswitch.io (same repo README).

### 5.2 Multi-account / provider switching mechanics

- **Config switch, not session transplant** (§1.2, verb 1): one
  click rewrites the live provider config; tray quick-switch;
  Claude Code needs no restart (parent review). Universal
  providers sync one config to Claude Code / Codex / Gemini CLI:
  [zleap-admin/302-cc-switch feature summary](https://github.com/zleap-admin/302-cc-switch).
- **Projects (v3.17.0, 2026-07-13):** save current provider +
  MCP + Skills + prompt files as a named snapshot; switch the
  whole set in one click from title bar or tray; leaving state
  auto-saves back: [cc-switch v3.17.0 changelog](https://github.com/farion1231/cc-switch-website/blob/HEAD/public/docs/changelog/en/3.17.0.md).
  Projects are now global shared entities with per-tool scoped
  switching: [cc-switch profiles refactor commit](https://github.com/farion1231/cc-switch/commit/dbb5999d1ec71fb047cc62e4f092dffaec123325).
- **MCP sync:** upserting an enabled MCP server auto-syncs to
  the app config: [cc-switch MCP auto-sync commit](https://github.com/farion1231/cc-switch/commit/e77eab211605442328fc2b3e04f8662682af2742);
  sync is per-server fault-isolated: [cc-switch MCP sync fix](https://github.com/farion1231/cc-switch/commit/81c0ff252b830584d086f0e327ab1edf93afb507).
  Provider switches sync MCP for all apps: [cc-switch MCP-on-switch commit](https://github.com/farion1231/cc-switch/commit/a00eb764f7da12ee9dfe13ff920c098fb10d94bd).

### 5.3 Quota data source / accuracy / reset handling

- `[docs]` Inline 5h/weekly quota/balance (incl. Copilot/Codex
  OAuth) in the tray row `name · mode · provider · quota`
  (parent review). Quota is provider-balance oriented: API-key
  and coding-plan balances, not only subscription windows.
- `[docs]` Coding-plan quota adapters exist per provider
  (kimi/zhipu/minimax/volcengine); a missing adapter (iFlytek)
  means "cannot query remaining quota like users of other
  providers": [cc-switch issue #4840](https://github.com/farion1231/cc-switch/issues/4840).
  Accuracy is therefore *per-provider uneven* — a structural
  weakness Juggler's Measured/Estimated labelling can exploit.
- `[docs]` Statusline companions read quota however you switch
  ("Works however you switch providers — CC Switch, manual edits,
  or any other tool"): [baigong-ai/cc-planbar](https://github.com/baigong-ai/cc-planbar).

### 5.4 Cost analytics

- `[docs]` Usage statistics: spending/requests/tokens dashboards
  per model; **models.dev automatic pricing sync** (opt-in) plus a
  human-editable `~/.cc-switch/model-pricing.json` (v3.19.0):
  [cc-switch v3.19.0 release notes](https://github.com/farion1231/cc-switch/blob/HEAD/docs/release-notes/v3.19.0-en.md).
  Grok OAuth-mode usage import closes a previously invisible gap
  (same notes). Storage is SQLite SSOT at
  `~/.cc-switch/cc-switch.db` with auto-rotating backups:
  [theresanaiforthat cc-switch profile](https://theresanaiforthat.com/company/farion1231/repository/cc-switch/).

### 5.5 Session features / pricing / OSS

- Sessions: none supervised. cc-switch configures *future* CLI
  runs; it does not watch live sessions, detect stuck, or keep
  lineage. That absence is Juggler's opening (§8).
- Free, MIT (fork READMEs confirm "fork of cc-switch (MIT)"):
  [wanjiaruan/302-cc-switch](https://github.com/wanjiaruan/302-cc-switch).
  Very large star count in search snapshots (~140k string in repo
  metadata snippets — treat as unverified magnitude, Appendix).

### 5.6 Why users love it / leave it

Love: one-click switching without restart, tray-at-a-glance,
no more JSON editing, broad tool coverage (7–10 CLIs).
Leave (`[inferred]` from the adapter-gap evidence): uneven
quota accuracy per provider, config-switch surprises (state
auto-save on project switch), and no help once sessions are
*running* — the tool's job ends at launch.

### 5.7 vs Juggler

`[repo]` Juggler's tray is specced quota-only
([SPEC.md](../SPEC.md#L352))
and absent in `ui/src` (parent review F7.2); the Usage screen is
`ComingSoon` in
[App.tsx](../../ui/src/App.tsx#L339).
cc-switch therefore beats Juggler today on every switcher
expectation: tray switch, quota glance, MCP/skills sync, pricing
sync. Juggler's counter is supervision + handover + honesty
labels — none of which cc-switch attempts. See §8, S1–S5.

## 6. ccusage (ryoppippi) — the cost-accounting standard

### 6.1 What it does

`[docs]` CLI reading `~/.claude/projects` JSONL (+ Codex/
OpenCode/Amp sources), producing daily/monthly/session/5h-block
token + API-pricing cost reports (parent review). Repo:
[ryoppippi/ccusage](https://github.com/ryoppippi/ccusage).
It answers "what did I burn?" — never "what do I have left?".

### 6.2 Switching mechanics

None. ccusage is read-only accounting; it does not switch,
rotate, or launch. Its role in the stack is the *ledger* that
switchers lack — `cswap` users pair it for spend (parent review).

### 6.3 Quota data source / accuracy / reset handling

- Source: local transcripts only. Cost is **Computed** (§1.1):
  tokens × price table. `[docs]` Pricing can run offline with an
  embedded LiteLLM snapshot; Nix builds embed the locked pricing
  file so sandboxed builds fetch nothing: [alexejboiko/ccusage](https://github.com/alexejboiko/ccusage).
  Forks add `--share` runs with offline pricing by default:
  [leoisadev1/ccusage-plus](https://github.com/leoisadev1/ccusage-plus).
- Known accuracy trap: `[docs]` "Claude Code's `.jsonl`
  transcripts contain no `costUSD` field for subscription usage,
  so cost is *always* computed from a price table fetched over
  the network" — plus two more silent-error modes documented in
  [vibetec-eu/ccdash](https://github.com/vibetec-eu/ccdash).
  Per-prompt (not just daily) breakdowns exist downstream:
  [haruaki-nayuta/ccmeter](https://github.com/haruaki-nayuta/ccmeter).
- Resets: the 5h-block report aligns to Claude's rolling window,
  but it reports *burn inside blocks*, not vendor remaining %.
  No reset countdown, no headroom — by design.

### 6.4 Cost analytics (its whole product)

Daily/monthly/blocks/session reports, per-model splits,
multi-source (Claude/Codex/OpenCode/Amp), statusline mode.
`[docs]` Complementary-tools framing: ccusage does "macro usage
aggregates… plus a statusline" while deep per-session audit
lives elsewhere: [axisrow/claude-devtools README](https://github.com/axisrow/claude-devtools/blob/HEAD/README.md).

### 6.5 Sessions / pricing / OSS / love-leave

- Sessions: historical only (which session burned what). No live
  supervision, no resume, no lineage.
- Free OSS CLI (~13–17k stars, parent review snapshot).
- Love: trustworthy-enough money numbers from data already on
  disk; scriptable; offline mode. Leave: numbers silently wrong
  when pricing fetch fails or transcripts lack cost fields
  (ccdash evidence above) — users discover this only by auditing.

### 6.6 vs Juggler

`[repo]` Juggler already parses the same JSONL and already has
`TokenCounts` per session in
[model.rs](../../crates/juggler-core/src/model.rs#L261),
but models tokens/credits only — no pricing table, no USD
(parent review F1.2). The spec's usage engine promises
per-session/account/window aggregates + burn-down in
[SPEC.md](../SPEC.md#L254).
`[inferred]` Shipping ccusage-parity (daily/monthly/blocks +
offline price snapshot + CSV) is the cheapest credibility win
available: the input pipeline exists; only pricing + rollups
are missing. Label every USD figure Computed, never Measured.

## 7. claude-swap (`cswap`) + `aisw` — true account juggling

### 7.1 claude-swap: what it does

`[docs]` "Switch between multiple Claude Code accounts, with
automatic rate-limit rotation, usage dashboard, and parallel
sessions": [realiti4/claude-swap](https://github.com/realiti4/claude-swap).
CLI (`cswap`) + TUI + optional menubar. Claude-only. This is
Juggler's closest incumbent: the only tool whose *headline* is
juggling (§1.2, verbs 2 + 4).

### 7.2 Switching mechanics (the day-1 bar)

- `[docs]` `cswap switch` rotates to the next account;
  `cswap switch 2` / `switch user@example.com` / alias targets a
  specific one; leaving-account token is stored (same README:
  [realiti4/claude-swap](https://github.com/realiti4/claude-swap)).
- `[docs]` **Auto-rotation:** "When the active account's 5-hour
  or 7-day window reaches the threshold (default 90%), it
  switches to the account with the most quota left — before you
  hit the limit, and safe to run while Claude Code is working":
  `cswap auto` (polls every 60s), `--threshold 80`, `--once`,
  `--json`, `--dry-run`: [cswap v0.16.0 release](https://github.com/realiti4/claude-swap/releases/tag/v0.16.0)
  and [PR #81](https://github.com/realiti4/claude-swap/pull/81).
- `[docs]` Usage-aware manual switch: `--switch --best` picks
  most remaining 5h/7d headroom and "never moves you onto an
  account worse than your current one": [PR #59](https://github.com/realiti4/claude-swap/pull/59).
  Menubar shows every account's 5h/7d/spend and switches by
  click (specific/rotate/best/next-available), sharing the
  `autoswitch.*` settings with the CLI (same README).
- `[docs]` Directory binding: `cswap map` binds a directory to
  an account so bare `cswap run` there launches that account:
  [hainnt/c-hottag comparison](https://github.com/hainnt/c-hottag/blob/HEAD/docs/comparison.md).
  Desktop wrapper adds aliases, disable/enable (hold out of
  rotation), threshold/strategy editing, switch notifications:
  [theskinnyrat/cswap-desktop](https://github.com/theskinnyrat/cswap-desktop).

### 7.3 Quota source / accuracy / reset handling

- `[docs]` `cswap list` is the dashboard: "every account's
  5-hour and 7-day usage and reset times at a glance" (README,
  [realiti4/claude-swap](https://github.com/realiti4/claude-swap)).
  Source is per-account credential-backed usage fetch (Measured-
  class, §1.1) — which creates its known failure mode:
- `[docs]` Token-family desync: once `cswap run` bootstraps a
  profile, Claude rotates the token family *inside* the profile;
  the backup's refresh token 401s forever and "usage for that
  account then freezes silently at the last pre-session
  measurement": [PR #97](https://github.com/realiti4/claude-swap/pull/97).
  Active-account refresh has the mirror bug: [PR #70](https://github.com/realiti4/claude-swap/pull/70).
  `[inferred]` Silent staleness is the switcher category's
  original sin — Juggler's freshness/stale-flag design must beat
  this explicitly (Codex's 15-min stale flag, §2.2, is the model).
- Resets: shown per account; auto-switch acts *before* the wall.
  No burn-down forecast ("exhausts in ~40m") found in docs.

### 7.4 Cost / sessions / pricing / love-leave

- Cost: spend column in menubar; deep cost via `ccusage` pairing
  (§6.2). No built-in USD ledger comparable to cc-switch (§5.4).
- Sessions: parallel sessions supported; maintained-fork activity
  (`aswap` = upstream + unmerged fixes) signals both love and
  upstream-fix latency: [blackhole1/aswap](https://github.com/blackhole1/aswap/blob/HEAD/README.md).
- Free OSS. Love: set-and-forget rotation, best-account picking,
  per-directory accounts. Leave: Claude-only, silent-stale usage
  bugs, rotation is *automatic* (see §7.6 compliance contrast).

### 7.5 `aisw` (burakdede): named profiles across CLIs

`[docs]` "Switch between multiple Claude Code, Codex CLI,
Antigravity and Gemini CLI accounts in one command. Named
profile manager for AI coding agents": [burakdede/aisw](https://github.com/burakdede/aisw).
Mechanics: `aisw use claude acct`, `aisw use --all --profile
acct`; "After `aisw use …`, start a fresh agent process… you
should not rely on reusing an already running or resumable
session from the previous account" (same README). Quota: none
shown in README evidence (negative evidence only). Cost: none.
Sessions: fresh-process switching only — no transplant, no
lineage. Love: one-command, multi-CLI, explicit. Leave: restart-
to-switch friction that cc-switch (no-restart) already solved.

### 7.6 vs Juggler (feature-by-feature)

`[repo]` Juggler's invariant forbids `cswap auto`'s headline:
automation is passive (monitor/alert/suggest); every state-
changing action is an explicit click, spec §7 in
[SPEC.md](../SPEC.md#L261).
The compliant answer is *suggested* rotation ("A at 92% — switch
new sessions to B? [Switch]"), never auto-rotation. But the
*information* `cswap auto` acts on — per-account 5h/7d + resets +
best-account ranking — is table stakes (§8, S2/S4). Juggler's
handover entry point exists in
[commands.rs](../../crates/juggler-app/src/commands.rs#L1070)
(`session_handover`), yet advertised `TRANSPLANT` in
[claude_code.rs](../../crates/juggler-adapters/src/claude_code.rs#L111)
outruns the proven single-JSONL copy (parent review F3.3).
Niche map: choose **cswap** for one-active-account auto-rotation,
**aisw** for multi-CLI named profiles, per [nemo-illusionist comparison](https://github.com/nemo-illusionist/claude-code-account-switcher/blob/HEAD/docs/comparison.md) —
Juggler must win the third cell: *parallel supervised* juggling.

## 8. Day-1 table stakes switcher users expect (vs Juggler today)

| # | Expectation (source) | Juggler today `[repo]` |
|---|---|---|
| S1 | One-click switch, no restart (cc-switch §5.2) | Profile launch only; no tray switch |
| S2 | Every account 5h/7d % + reset at a glance (`cswap list` §7.3) | `percent={0}` rings — missing |
| S3 | Quota as truthful as `/usage` (§2.1) | Feed declared, not wired (§2.1) |
| S4 | Best-account ranking + threshold warning (`--best`, 90% §7.2) | None; suggest-only version specced |
| S5 | Named profiles + per-dir affinity (`aisw use`, `cswap map`) | Profiles exist; no dir affinity |
| S6 | USD ledger, offline pricing, CSV (ccusage §6, cc-switch §5.4) | Tokens only, no USD (§6.6) |
| S7 | MCP/skills/prompt sync across accounts (cc-switch §5.2) | Not built (parent F1.9) |
| S8 | Freshness you can see; stale never silent (§7.3, §2.2) | `refresh()` swallows errors (parent F2.1) |
| S9 | Worktree isolation + diff review (Vibe/Crystal §§3–4) | No worktree (parent F1.1) |
| S10 | Live-session help after launch (nobody ships this) | Modelled, unemitted statuses (§4.6) |

S1–S6 are *switcher* table stakes: without them a `cswap`/
cc-switch user churns in day 1. S9 is the *orchestrator* stake
for Vibe/Crystal orphans. S10 is the wedge: nobody ships it, and
it is the only row where Juggler can be first. Sequence: S2+S3
(wire the statusline feed into `QuotaWindow`), S8 (visible
freshness), S4-as-suggestion, S6, then S1/S5/S7 tray parity;
S9 before marketing to orphans; S10 as the headline.

## 9. Sunset / migration opportunity — sized honestly

- Vibe: ~26.9k stars pre-sunset `[docs]` [codecoradev issue #312](https://github.com/codecoradev/uteke/issues/312);
  151 stranded PRs (§3.6). Crystal: ~3.1k stars (§4.4). These are
  *audience* proxies, not movable users — no download/active-user
  figure was verified (Appendix). `[inferred]` Realistic capture
  needs (a) import of projects/worktrees, (b) S9, (c) a comparison
  page — in that order. Quota messaging alone captures ~none.
- The switcher incumbents are *not* sunsetting: cc-switch and
  `cswap` are actively releasing (v3.19.0, v0.16.0 above). There
  is no orphan pool there — only head-to-head wins on S8/S10.
- Timing: Vibe's CDN-dependent `npx` install "will break when
  the CDN goes away" (§3.6) — the migration window closes when
  orphans land on Conductor/T3/Superset-style successors, e.g.
  [Superset vs Vibe comparison](https://github.com/kogan/superset/blob/HEAD/apps/marketing/content/compare/superset-vs-vibe-kanban.mdx).

## 10. Bottom line for the roadmap

1. Stop advertising `USAGE_QUOTA_FEED|TRANSPLANT` until wired/
   proven (§2.1, §7.6) — switcher users audit this first.
2. Ship S2/S3/S8 as one slice: statusline feed → `QuotaWindow`
   → labelled rings with reset countdowns + stale flags.
3. Answer `cswap auto` with suggested rotation + reset-queue
   (one click), preserving the passive invariant (§7.6).
4. Add ccusage-parity cost (Computed, offline prices, CSV).
5. Earn Vibe/Crystal orphans with worktrees + import, not copy.

## Appendix — unresolved evidence (do not quote as fact)

- No live install/run of any competitor; CLI behaviours are
  `[docs]` from README/changelog/PR evidence, not execution.
- Star counts are search snapshots (cc-switch ~140k string
  unverified; Vibe 26.9k via secondary issue; Crystal 3.1k).
- `aisw` quota absence and Vibe/Crystal quota absence are
  negative evidence (no feature found), not vendor denials.
- Vibe active-user/download counts, Nimbalyst pricing, and
  `cswap` usage-API internals were not primary-verified.
- Juggler line refs verified 2026-10-13 on `main`; UI absence
  claims rely on parent-review grep + §5.7 spot checks.







# 07 — Personas & Journeys (Deep Dive)
> Parent: [full review](full-competitive-ux-review.md#L1) · Inputs: [PROPOSAL.md](../PROPOSAL.md#L49), [SPEC.md](../SPEC.md#L86), prior map [review:729](full-competitive-ux-review.md#L729). Quotes/diaries are `[composite]` verbatim-style, not interviews — validate §9. Repo claims cite `file:line`; competitors link out.
## 0. Reading guide & cross-persona truths
Marks: `[repo]` grounded · `[composite]` invented voice · `[inferred]` needs dogfood · `[competitor]` URL.
Prior 4 seeds ([PROPOSAL.md:51](../PROPOSAL.md#L51)) split into 6: solo maximiser, salaried operator, and billable consultant optimise for momentum, throughput, and client separation — opposite at the quota wall.
1. Quota-blindness is #1 for 5/6; roadmap inverts it (M3 after adapters) [review:15](full-competitive-ux-review.md#L15). 2. `Stuck/WaitingUser/QuotaBlocked` modelled, never emitted [review:30](full-competitive-ux-review.md#L30) — "which needs me?" unanswerable. 3. Handover v0 copies one JSONL, siblings left [review:20](full-competitive-ux-review.md#L20); trust won/lost there. 4. No worktree; session=only `cwd` [review:63](full-competitive-ux-review.md#L63). 5. No USD view [review:76](full-competitive-ux-review.md#L76). 6. Onboarding best-built, protect it [review:53](full-competitive-ux-review.md#L53). 7. Suggest-only, never auto-rotate ([PROPOSAL.md:62](../PROPOSAL.md#L62)). 8. Instances/Sessions conflated [review:33](full-competitive-ux-review.md#L33).
| # | Persona | Subs/mo* | Sessions/day | Pays how |
|---|---|---|---|---|
| 1 | Maya, solo indie Multi-Maxer | ~$450–620 | 4–8 | Self, reluctantly |
| 2 | Devon, staff eng 10+ parallel | $0 (employer ~$500+) | 10–20 | Employer; OSS time |
| 3 | Priya, agency multi-client | $600–1,200 pass-thru | 6–14 | Client-billed |
| 4 | Jonas, OSS maintainer | $20–100 | 3–10 bursty | Free; contributes code |
| 5 | Sara, enterprise-locked | $0 visible | 2–6 approved | Procurement; cannot pay |
| 6 | Alex, student optimiser | $20–38 | 1–4 | Free only; churns fast |
\* Estimates from tiers in [research/zcode.md:15](../research/zcode.md#L15) (GLM Lite $18/Pro $80/Max $168 via [codingplan.org](https://codingplan.org/Coding%20Plans)); order-of-magnitude only.
---
## 1. Maya — Solo Indie Multi-Maxer (seed A, [PROPOSAL.md:51](../PROPOSAL.md#L51))
**Snapshot:** 34, Lisbon; solo SaaS founder ~$4k MRR `[inferred]`; M3 Pro 36GB; 2×Claude Max+Codex Plus+GLM Lite; 4–8 sessions/day, 2–3 repos; expert CLI, aliases+direnv today ([PROPOSAL.md:25](../PROPOSAL.md#L25)).
**JTBD:** Functional "keep shipping when one account dies mid-feature" · Emotional "stop feeling stupid paying twice and still stalling" · Social "look prolific without hiring".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:30 | Tray: A 78%, B 22%, reset 17:00 — calm *if* quota exists; today blind |
| 09:00 | Launch feature+tests — delight 1-click; fear wrong account |
| 11:20 | Session waited 25m on permission, unseen — status unemitted `[repo]` |
| 13:00 | Quota wall A mid-refactor — panic: re-explain or wait 4h? |
| 13:05 | Handover A→B — distrust: "what did it drop?" (siblings, F3.4) |
| 15:30 | B finishes — momentum kept *iff* handover honest |
| 17:05 | Reset, return to A — aha if 1-click; churn if manual copy |
| 21:00 | Cost check — anxiety: no USD ledger (F1.2) |
### Top-10 pains (F×S, 1–5 each)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Quota wall, no warning | 25 | 6 | Same-branch collision | 12 |
| 2 | Waiting invisible | 16 | 7 | Alias fragility on CLI update | 9 |
| 3 | Handover drops todos/subagents | 15 | 8 | Reset, no resume nudge | 8 |
| 4 | Wrong-account billing/tier | 15 | 9 | No archive/search pile-up | 8 |
| 5 | No USD for Max#2 renewal | 12 | 10 | Notifications none-or-spam | 6 |
**Workarounds:** `CLAUDE_CONFIG_DIR` aliases+direnv ([research/claude-code.md:34](../research/claude-code.md#L34)), ccusage cron, sticky-note reset times, manual JSONL `cp`.
> `[composite]` "I pay for two Max accounts so I never stop. Then I stop — I can't see the wall coming and the spare needs the whole story re-told."
> `[composite]` "Don't auto-switch me. Tap my shoulder at 80%, have the other account *ready*. The click is mine."
**Diary Tue 13:04:** *"A died mid billing-migration. Todos were in that session. B re-asked about Stripe webhooks I'd answered at 10am."* — sibling loss [review:289](full-competitive-ux-review.md#L289).
**Aha:** forecast "A exhausts ~40m; B 80%" (WOW-b [review:769](full-competitive-ux-review.md#L769)) + fidelity label pre-click. **Churn:** one silent handover loss or wrong-account bill. **WTP/OSS:** $0 core; sponsor $5–15/mo, $49 one-off Pro pack; bug reports, no Rust. **A11y/neuro:** ADHD context-switch cost — inbox by waiting-time, colour+text never colour-only [review:840](full-competitive-ux-review.md#L840), reduced motion.
---
## 2. Devon — Staff Engineer, 10+ Parallel (seeds D+C, [PROPOSAL.md:54](../PROPOSAL.md#L54), [PROPOSAL.md:53](../PROPOSAL.md#L53))
**Snapshot:** 41, Austin; staff eng 80-person startup; M4 Max 64GB + Linux desktop (fleet); employer Claude Max×2/Codex/Cursor expensed; 10–20 sessions/day, 4 services; judges Juggler as infra.
**JTBD:** Functional "run agents like CI: triage, not babysit" · Emotional "control at 15 sessions, not haunted by tabs" · Social "the person whose agents ship".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:00 | Overnight: 3 done, 2 stuck, 1 quota-blocked — needs inbox; statuses unemitted |
| 09:00 | Launch 6 parallel — admission OK/Tight/No matters (SPEC §5.2) |
| 10:30 | RAM/swap thrash — wants per-session footprint, not Activity Monitor |
| 12:00 | 5 finished, diffs unseen — review bottleneck `[inferred]` (WOW-h) |
| 14:00 | Quota wall shared Max — forecast would have routed at launch |
| 16:00 | Jump sessions — rail caps 15/8, no search (F2.5); wants ⌘K |
| 18:30 | Handover A→Codex (T3/T4) — advertised, not built (F3.1): trust hit |
| 20:00 | 4 overnight left — digest not pings (WOW-g) or he mutes all |
Expectations set by Conductor worktree+diff [concepts](https://www.conductor.build/docs/concepts/workflow), Vibe inline-comment-to-agent [review:639](full-competitive-ux-review.md#L639), Squad 1-task=1-worktree [review:579](full-competitive-ux-review.md#L579). Juggler wins only on juggling.
### Top-10 pains (F×S)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | "Which of 15 needs me?" | 25 | 6 | Rail truncation, no search | 15 |
| 2 | Review bottleneck `[inferred]` | 20 | 7 | Cross-harness handover absent | 10 |
| 3 | Worktree/branch collisions | 20 | 8 | Scale jank 20 sess. (F5.1) | 9 |
| 4 | Machine overload 10+ | 16 | 9 | Alert fatigue→mute→miss | 8 |
| 5 | Stuck vs waiting-on-model | 16 | 10 | Parallel quota burn (Pro cap [review:617](full-competitive-ux-review.md#L617)) | 16 |
**Workarounds:** Claude Squad/Conductor trial, tmux+worktrees, [ccusage](https://github.com/ryoppippi/ccusage), `ps`/notify scripts, Slack self-DMs as inbox.
> `[composite]` "I don't have a coding problem. I have air-traffic control with fifteen planes and no tower."
> `[composite]` "If 'handover' is copy-one-file-and-pray, say so on the button. I'll click — I just won't bet a migration on it."
**Diary Thu 08:02:** *"One agent asked a question at 01:14, waited seven hours. A badge '1 needs you' would have saved a sprint day."* — F2.5 lived.
**Aha:** inbox by waiting-time (WOW-a) + admission verdict; first morning it surfaces the 01:14 waiter in <10s. **Churn:** scale jank (watcher full-reads, Mutex — F5.1) or T3/T4 overclaim found live. **WTP/OSS:** employer $20–50/seat/mo plausible (vs Conductor Pro $50 [pricing](https://www.conductor.build/pricing)); personally $0; contributes Rust, load fixtures, conformance tests. **A11y/neuro:** keyboard-only ⌘K ≤3 keystrokes [review:855](full-competitive-ux-review.md#L855), icons+text (dyslexia), batched notifications for focus.
---
## 3. Priya — Agency/Consultant, Multi-Client (B generalised, [PROPOSAL.md:52](../PROPOSAL.md#L52))
**Snapshot:** 38, Toronto; 6-person agency, 4 clients; M3 Max 48GB, client VPNs; agency Claude Max×2 + per-client Codex + GLM Pro, $600–1,200/mo pass-thru; 6–14 sessions/day; she is who clients audit. Wrong-account = wrong *client* billing (Claude #87447, [research/claude-code.md:36](../research/claude-code.md#L36)).
**JTBD:** Functional "prove which account touched which client repo, when" · Emotional "never fear a client seeing another client's name" · Social "look enterprise-grade in security review".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:45 | Client A standup — needs per-client filter; rail is flat today |
| 09:30 | Launch on wrong profile (similar labels) — dread; affinity guard absent (F1.4) |
| 11:00 | Screen-share, other client's session visible — shame risk; needs Present mode `[inferred]` |
| 13:00 | Handover within Client B — needs redaction preview before pack moves (F1.6) |
| 15:00 | Client: "what did AI do Tuesday?" — lineage minimal (F3.3), no CSV (F1.2) |
| 17:30 | Invoice: tokens→USD per client — manual ccusage+spreadsheet, 45m |
| 19:00 | Quota wall shared Max — other client's work must NOT auto-move (guardrail) |
Red line: packs verbatim, redaction only for diagnostics [review:120](full-competitive-ux-review.md#L120) — for Priya a confidentiality incident on a tired Friday. Secret-scan preview is P0 *for her*, P1 globally; put it in the wizard, not Settings.
### Top-10 pains (F×S; * = existential, rank by S first)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Per-client cost/provenance export absent | 20 | 6 | Quota shared, no per-client budget | 12 |
| 2 | Cross-client leakage on share* | 15 | 7 | Handover lineage too thin to audit | 12 |
| 3 | Wrong-client billing* | 15 | 8 | Pack leaks secrets verbatim* | 10 |
| 4 | Credential-copy vs SPEC S8 in audit (F4.1)* | 10 | 9 | Profile clone/MCP sync absent | 9 |
| 5 | No Present/client mode `[inferred]` | 9 | 10 | Client data retained forever (F1.3) | 8 |
**Workarounds:** separate macOS users per big client, [cc-switch](https://github.com/farion1231/cc-switch) tray, ccusage per config dir, manual redaction, Toggl+spreadsheet allocation.
> `[composite]` "Clients don't care which model. They care Client A's roadmap never appears in Client B's status call."
> `[composite]` "Give me one page — this account, this repo, this spend — and your logo goes in my proposal deck."
**Diary Fri 16:58:** *"Exported a pack to work at home. Staging key sat in a tool result. Nothing bad happened. I still feel sick. A preview catches it in four seconds."* — F1.6 embodied.
**Aha:** wizard step-2 redaction preview + usual-account marker (UX.5 [review:843](full-competitive-ux-review.md#L843)), then per-client CSV. **Churn:** any cross-client incident, or auditor hitting `.credentials.json` copy (F4.1) unanswered. **WTP/OSS:** highest — $30–60/seat/mo client-billed *if* audit export+redaction land, else $0 and OS-user separation stays; case studies, redaction patterns, sponsorship. **A11y/neuro:** screen-share anxiety is a feature — Present mode hides other clients/emails; client colour+name, never colour alone; predictable focus for live demos.
---
## 4. Jonas — OSS Maintainer, Bursty & Broke (new; OSS model [review:719](full-competitive-ux-review.md#L719), sunset window [review:507](full-competitive-ux-review.md#L507))
**Snapshot:** 29, Berlin; employed by day, maintainer by night; M2 Air 16GB + CI; Claude Pro + GLM Lite $18, $20–100/mo partly sponsored; 3–10 sessions/day, release weeks 15+; expert, reads Rust, files perfect bugs.
**JTBD:** Functional "triage 40 issues without burning week's quota by Tue" · Emotional "help everyone without resenting the project" · Social "good steward: transparent, responsive, kind".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 07:30 | Launch 3 issue-repro sessions — admission Tight on 16GB: honest, useful |
| 09:00 | Day job; Juggler sessions must survive/adopt (SPEC reliability) |
| 18:00 | Weekly quota 91% Thu on small Pro tank — forecast matters *more*, not less `[inferred]` |
| 19:00 | Handover Pro→GLM Lite to save Claude — cross-harness T3 absent, replay lossy |
| 20:30 | Review contributor PRs — needs diff queue; terminal-only is slow |
| 22:00 | Archive release-week sessions — no lifecycle → 200 stale rows (F1.3) |
### Top-10 pains (F×S)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Small Pro quota, no forecast → mid-week wall | 20 | 6 | No sponsor cost reporting | 9 |
| 2 | 16GB: 3 sessions = swap death | 16 | 7 | Update breakage (weekly harness churn) | 9 |
| 3 | Repro sessions pile up indistinguishable | 15 | 8 | Onboarding imports day-job profiles too | 8 |
| 4 | Claude→GLM handover absent | 12 | 9 | Dense rail on 13" laptop | 8 |
| 5 | Stuck detection absent overnight | 12 | 10 | No adapter SDK to contribute (F1.8) | 4 |
**Workarounds:** [Claude Squad](https://github.com/smtg-ai/claude-squad) TUI, tmux+worktrees, ccusage weekly sponsor report, GitHub labels as session tracker, "quota-budget Sunday" ritual.
> `[composite]` "Pro is not 'less Max'. It's a different sport. Your forecast must work when the tank is tiny."
> `[composite]` "I can't pay you. I can write your adapter for the tool I maintain — give me a conformance test to run."
**Diary Thu 18:12:** *"91% and it's Thursday. Release is Saturday. Review the security PR now or save quota for the release fire? A tool that just shows the cliff edge is already worth installing."*
**Aha:** burn-down on *small* quota + admission respecting 16GB — "built for me, not Max whales" (positioning risk if ignored). **Churn:** Max-only feel (Pro weekly ignored) or nagging RAM advisor. **WTP/OSS:** $0, $5/mo symbolic sponsorship; real currency adapter PRs, fixtures, i18n, dogfood — unlock via conformance kit (F1.8/F5.5). **A11y/neuro:** burnout-aware quiet digest default, no streaks/gamification; screen-reader labels (VoiceOver triage when eyes tire — untested, Appendix A [review:1014](full-competitive-ux-review.md#L1014)); low-vision density options.
---
## 5. Sara — Enterprise-Locked Developer (stresses compliance [PROPOSAL.md:56](../PROPOSAL.md#L56), non-goal [PROPOSAL.md:45](../PROPOSAL.md#L45))
**Snapshot:** 36, Stockholm; bank/fintech regulated; MDM MacBook Pro 32GB, no admin, allowlist; employer Claude SSO + approved Codex only, $0 visible; 2–6 sessions/day approved accounts; strong dev, zero ToS-ambiguity tolerance. User, not buyer — potentially a blocker.
**JTBD:** Functional "use two approved accounts without IT ticket per switch" · Emotional "never worry Legal flags my tooling" · Social "recommend to team without staking my name".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 08:30 | Install needs signed/notarised + MDM — unsigned = non-starter (M5) |
| 09:00 | Add work account via vendor SSO in embedded terminal — good (F4.3 vendor-owned login) |
| 09:10 | Personal account prompt — she must NOT; needs hard separation/opt-out |
| 11:00 | Security: "does it copy credentials?" — F4.1 ambiguity blocks deployment |
| 14:00 | Quota wall, only approved spare — suggest-only is a *feature* (click audit trail) |
| 16:00 | Export pack for colleague? — non-goal sharing ([PROPOSAL.md:45](../PROPOSAL.md#L45)) must refuse clearly |
| 17:00 | Telemetry prompt — opt-in only (SPEC privacy) or uninstall |
### Top-10 pains (F×S; * = existential blocker, any one = zero adoption)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Multi-account optics "circumventing?" | 12 | 6 | Pack redaction absent | 9 |
| 2 | No handover click audit log `[inferred]` | 12 | 7 | Windows/Linux absence, mixed team | 8 |
| 3 | F4.1 vs SPEC S8 unanswered* | 10 | 8 | DB backup/restore story (F1.7) | 8 |
| 4 | Unsigned build cannot install* | 10 | 9 | Onboarding scan finds personal profiles | 6 |
| 5 | Fleet mDNS/gRPC triggers netsec review* | 5 | 10 | Vendor ToS change, no comms plan* | 5 |
**Workarounds:** vendor CLIs only, IT-approved VS Code, logout/login account switch (30m), Confluence "which account for which project". Envies Maya, cannot copy her.
> `[composite]` "Your guardrails page is why I'm here. Your credential copy is why Legal said 'not yet'. Both are you — pick."
> `[composite]` "I don't need team features. I need a signed binary, a one-page data-flow diagram, and proof you never proxy."
**Diary Wed 11:20:** *"Security review, minute 14: 'Does it copy credentials?' I said spec says never. Reviewer found the import code. Meeting over. The tool didn't fail — the story did."* — F4.1/F6.2 truth gap.
**Aha:** an artefact, not a feature — decided F4.1 + `guardrails.rs` exists (F4.2) + signed build; then inbox. **Churn:** never adopts or forced uninstall; organisational, not experiential. **WTP/OSS:** cannot pay; procurement $50–100/seat/yr only with signed/MDM/support — do NOT promise in OSS v1; contributes security reviews, threat-model PRs. **A11y/neuro:** WCAG AA as procurement checkbox (UX.8 P1s are *sales* features); predictable UI, no `window.alert` surprise modals (F2.1 fails enterprise review instantly); full keyboard + focus-visible.
---
## 6. Alex — Student/Hobbyist Cost-Optimiser (segment proven by GLM credits [research/zcode.md:14](../research/zcode.md#L14))
**Snapshot:** 22, Nairobi, remote MSc + hackathons; M1 Air 8GB or lab Mac; Claude Pro *or* GLM Lite $18 + $5 API top-ups, $20–38/mo max; 1–4 sessions/day (thesis + side app); fast learner, price-first, skims docs.
**JTBD:** Functional "stretch $20 across thesis + demo" · Emotional "don't feel poor next to Max screenshots" · Social "portfolio project that looks senior-built".
### Day-in-life with Juggler
| Time | Moment (emotion/friction) |
|---|---|
| 09:00 | Lab Mac fresh install, scan finds nothing — onboarding must shine empty (protect F2.6) |
| 09:20 | GLM Lite credits 2,000/5h as *budget* — needs Computed label + "≈N sessions" |
| 10:00 | 2 sessions on 8GB — admission No + lighter suggestion, not shame |
| 13:00 | Quota 80% before demo rehearsal — forecast saves demo; wall kills it |
| 15:00 | Handover Claude→GLM (cheaper) — T3 replay quality decides trust in cheap tier |
| 19:00 | Hackathon sprint, burn visible live — delight if glanceable |
| 23:00 | $5 top-up decision — USD ledger (F1.2) vs vibes |
### Top-10 pains (F×S)
| # | Pain | F×S | # | Pain | F×S |
|---|---|---|---|---|---|
| 1 | Wall before deadline/demo, no forecast | 20 | 6 | Budget cap/alert absent `[inferred]` | 12 |
| 2 | 8GB: heavy session swaps, all slow | 20 | 7 | Jargon learning curve (profile/transplant) | 8 |
| 3 | Credits not translated to money/time | 15 | 8 | Search absent at 60 thesis sessions | 6 |
| 4 | Built-for-multi-Max feel in onboarding | 12 | 9 | Flaky network stalls login flows | 6 |
| 5 | Handover to cheaper tool absent | 12 | 10 | Paywall/enshittification anxiety | 6 |
**Workarounds:** one terminal, `/usage` checks, [ccusage](https://github.com/ryoppippi/ccusage) screenshots to himself, manual free-tier hopping, deleting sessions to "save quota" (myth — UI should educate honestly).
> `[composite]` "Everyone's screenshot has three Max rings. Mine has one Pro ring at 91% and a demo at nine. Build the cliff alarm for me, I'm your loudest campus user."
> `[composite]` "Don't show me credits. Show me 'about six sessions left this week at how you're working'. Then I can plan."
**Diary demo-eve 13:20:** *"Rehearsal used the last window. Demo at four runs on the free tier with a model that formats code wrong. I practised the talk. I didn't practise begging the quota gods."*
**Aha:** "≈6 sessions left" + 80% alarm with cheap-tier suggestion (suggest-only WOW-d); first saved demo = advocate. **Churn:** paywall on quota/forecast (core promise), Max-centric onboarding, 8GB jank blamed on him. **WTP/OSS:** $0 core forever, maybe $12/yr student sync; tutorials, campus talks, walkthrough videos = top-of-funnel gold; docs/i18n/first-run bugs (only persona who *is* a new user every semester). **A11y/neuro:** colour-blind-safe + text labels, plain-language glosses, single-session focus view, no dense multi-ring overwhelm.
---
## 7. Cross-persona deep cuts (what the one-line pains hide)
### 7.1 The quota wall is three different walls
Maya hits a *momentum* wall (context in flight), Devon a
*throughput* wall (parallel burn multiplies it), Alex/Jonas
a *budget* wall (small tank, deadline-shaped). One ring UI
serves all three only if it shows: time-to-exhaust at current
burn (Maya), per-session burn attribution (Devon), and
sessions-remaining translation (Alex/Jonas). A percentage
alone — the hardcoded `percent={0}` placeholder today
([review F2.4](full-competitive-ux-review.md#L193))
— serves none. Honesty tiers (Measured/Computed/Estimated,
SPEC.md:254-259 via review) must be visible per wall type.
### 7.2 Handover trust is persona-asymmetric
For Maya/Devon a lossy handover wastes an afternoon; for
Priya it can leak a client secret; for Sara it fails an
audit; for Alex it decides whether cheap tiers are usable
at all. Hence fidelity labelling cannot be a tooltip: it is
the wizard's step 2 (UX.5), with carry-list in plain words
("transcript + subagents carried; MCP state does not
transfer"). T3/T4 advertised-but-absent (F3.1) hurts Devon
most because he will *discover it mid-incident*.
### 7.3 "Which needs me?" scales non-linearly
At 2 sessions (Sara/Alex) the rail suffices; at 8 (Maya/
Priya) badges matter; at 15+ (Devon) only a waiting-time-
sorted inbox works, and rail truncation (15/8 caps, F2.5)
actively hides the oldest waiter. Design the inbox data
model fleet-ready now (WOW-j) even though Fleet ships late.
### 7.4 Cost is justification, not analytics
Maya: renew Max #2? Priya: bill the client. Jonas: report
to sponsors. Alex: spend $5 or not? Devon: show his manager.
Nobody wants a chart for its own sake — every cost view
must end in a decision sentence or export. CSV/PDF export
is therefore P1, prettier charts P2 (inverts typical Usage
dashboard instinct; cf. ccusage decision-shaped reports,
[ccusage](https://github.com/ryoppippi/ccusage)).
### 7.5 Accessibility is not one persona's row
Colour-only status fails Maya (ADHD scanning), Priya (live
demo), Sara (procurement WCAG), Alex (colour-blindness) —
four different reasons, one fix (text+icon+colour, UX.8).
Keyboard flow is Devon's speed, Jonas's VoiceOver triage,
and Sara's audit demo. Treat UX.8 P1s as cross-persona P1.
---
## 8. Anti-personas — who NOT to build for
| Anti-persona | Why not | Response instead |
|---|---|---|
| Account sharers/poolers (5 friends, 1 Max) | Violates user-owned guardrail ([PROPOSAL.md:62](../PROPOSAL.md#L62)) | Refuse in copy; point to team plans |
| Auto-rotation seekers ("rotate at 90%") | claude-swap auto-rotates [review:674](full-competitive-ux-review.md#L674); Juggler is suggest-only | Suggested switch + 1 click; never auto |
| Proxy/gateway seekers | Prohibition zone ([PROPOSAL.md:125](../PROPOSAL.md#L125)); non-goal ([PROPOSAL.md:42](../PROPOSAL.md#L42)) | Document why; hard no |
| Team-pool managers (RBAC/SSO admin) | Non-goal v1 ([PROPOSAL.md:45](../PROPOSAL.md#L45)); Fleet ≠ team | Fleet = my machines only |
| Chat-UI seekers | Non-goal ([PROPOSAL.md:46](../PROPOSAL.md#L46)) | Embed terminal; harnesses own UX |
| Autonomous assigners | Non-goal ([PROPOSAL.md:44](../PROPOSAL.md#L44)) | Inbox + suggestions; human assigns |
| Non-dev AI users | No harness/quota job | Out of scope; don't dilute onboarding |
| Windows-only teams (today) | macOS-first (SPEC platform) | Say so plainly; ports later |
**Anti-persona test per feature:** "Does this require
sharing, proxying, auto-rotating, or auto-assigning?"
Yes → decline citing the guardrail, never "roadmap it".
This also protects Sara: visible refusals are audit assets.
---
## 9. Journey maps (Discover→Onboard→Daily→Wall→Handover→Ship→Advocate)
Legend: 😀 delight · 😐 neutral · 😖 friction · 💥 trust-break. Opportunity = smallest flipping change.
### 9.1 Maya
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😐 | "Another orchestrator?" → lead with juggling wedge (F7.1) |
| Onboard | 😀 | Scan finds t3 profiles; state F4.1 decision in summary |
| Daily | 😐 | Waiting invisible → emit WaitingUser + inbox badge (WOW-a) |
| Quota wall | 💥 | Blind + re-explain → forecast at launch (WOW-b) pre-empts |
| Handover | 😖 | Fidelity unknown → wizard tier + carry-list (UX.5) |
| Review/ship | 😐 | No diff queue `[inferred]` → read-only diff on finished (F7.3) |
| Pay/advocate | 😐 | Nothing to buy → sponsor + Pro pack, core free |
### 9.2 Devon
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😀 | Vibe sunset migrant (F7.4); import path absent → Squad/Vibe import |
| Onboard | 😖 | Instances≠Sessions (F2.2) → merge to Projects/Sessions now |
| Daily | 😖 | Rail caps, no search → ⌘K + filters (WOW-c) |
| Quota wall | 😖 | No per-session burn → burn attribution in inbox row |
| Handover | 💥 | T3/T4 advertised, absent (F3.1) → downgrade flags to RESUME-only |
| Review/ship | 😖 | 5 finished pile up `[inferred]` → review queue: diff stat + pack preview |
| Pay/advocate | 😀 | Employer pays if ROI shown → cost CSV for manager (F1.2) |
### 9.3 Priya
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😐 | "Auditable?" → security/lineage one-pager (F4.1 decided) |
| Onboard | 😖 | Flat labels, leakage fear → client grouping + Present mode `[inferred]` |
| Daily | 😖 | Wrong-profile near-miss → usual-account marker + confirm (F1.4) |
| Quota wall | 😖 | Cross-client temptation → per-client budgets; suggest, never move |
| Handover | 💥→😀 | Secret in pack (F1.6) → redaction preview flips it |
| Review/ship | 😀 | Lineage thin for client Q&A → jump-links both ways (UX.5) |
| Pay/advocate | 😀 | Needs invoice-grade export → per-client PDF/CSV (F1.2) |
### 9.4 Jonas
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😀 | Vibe/Crystal sunset (F7.4), "free really?" → MIT clarity + import |
| Onboard | 😖 | Scan mixes day-job/OSS → selective import checkboxes |
| Daily | 😐 | 16GB Tight verdicts → lighter-profile suggestion, not block (SPEC §5.2) |
| Quota wall | 💥 | Small-tank forecast absent → Pro weekly burn-down, not Max-only 5h |
| Handover | 😖 | Claude→GLM T3 absent → honest "replay, lossy" label + pack preview |
| Review/ship | 😐 | Archive pile-up → lifecycle archive-after-release (F1.3) |
| Pay/advocate | 😀 | $0 but contributes → conformance kit invites adapter PR (F1.8) |
### 9.5 Sara
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😐 | Multi-account optics → guardrails page as landing (PROPOSAL §5) |
| Onboard | 💥 | Unsigned + F4.1 unanswered → signed build + decided S8 = gate pass |
| Daily | 😐 | `window.alert`, dead nav (F2.1/F2.4) → banner errors; hide ComingSoon (UX.1) |
| Quota wall | 😐 | Only approved spare → click-audited suggestion (lineage log) |
| Handover | 😖 | Redaction absent → sanitised pack for internal repro (F1.6) |
| Review/ship | 😖 | No audit trail `[inferred]` → lineage + click-log export |
| Pay/advocate | 😐 | Procurement >> product → earn internal champion; don't sell yet |
### 9.6 Alex
| Step | Emotion | Friction → Opportunity |
|---|---|---|
| Discover | 😀 | Max screenshots alienate → show Pro/GLM-Lite hero too |
| Onboard | 😀 | Jargon profile/transplant → plain-language glosses; protect wizard (F2.6) |
| Daily | 😖 | Swap jank blamed on user → admission + "1 session is fine" framing |
| Quota wall | 💥 | No cliff alarm → 80% alert + cheap-tier suggestion (WOW-d) |
| Handover | 😖 | Replay quality unknown → fidelity label "summary risk: medium" |
| Review/ship | 😀 | Search absent at 60 sessions → ⌘K (WOW-c) |
| Pay/advocate | 😀 | Paywall fear → "core free forever" pledge; campus advocacy |
### 9.7 Journey synthesis
Across 6×7=42 steps 💥 occurs only at **quota wall,
handover, and Sara's onboard** — never discover or ship.
Spend P0 on forecast + honest handover + truth artefacts,
not landing pages or review polish. Review friction (😖×5)
is survivable; one silent handover loss is not. This
corroborates Phase 1 ordering from the demand side
([review:914](full-competitive-ux-review.md#L914)).
Emotion curves also diverge at *advocate*: Devon/Priya/Jonas
advocate with artefacts (CSV, case study, adapter PR),
Maya/Alex with stories (saved demo, never-stopped day),
Sara only inside her org after an audit passes. Marketing
must produce artefact-shaped proof, not testimonials alone.
---
## 10. Persona × Feature Priority Matrix
P0 churns/blocks without it · P1 before v0.1-beta *for that
persona* · P2 delight/later · — n/a. Global = max severity.
| Feature | Maya | Devon | Priya | Jonas | Sara | Alex | Global |
|---|---|---|---|---|---|---|---|
| Quota + forecast (WOW-b) | P0 | P0 | P1 | P0 | P1 | P0 | **P0** |
| Needs-Attention inbox (WOW-a) | P1 | P0 | P1 | P1 | P1 | P2 | **P0** |
| Honest handover wizard (UX.5) | P0 | P0 | P0 | P1 | P1 | P1 | **P0** |
| Worktree-on-launch (F1.1) | P1 | P0 | P1 | P1 | P2 | P2 | **P0/P1** |
| ⌘K + search (WOW-c) | P1 | P0 | P1 | P1 | P2 | P1 | **P1** |
| Cost ledger USD/CSV (F1.2) | P1 | P1 | P0 | P1 | P2 | P0 | **P1** |
| Redaction preview (F1.6) | P2 | P2 | P0 | P2 | P1 | P2 | **P1*** |
| Account health/affinity (F1.4) | P1 | P1 | P0 | P2 | P1 | P2 | **P1** |
| Reset-queue (F1.5) | P1 | P1 | P2 | P1 | P2 | P1 | **P1** |
| Lifecycle archive/search (F1.3) | P1 | P1 | P1 | P1 | P1 | P2 | **P1** |
| Admission advisor (SPEC §5.2) | P2 | P0 | P2 | P0 | P2 | P0 | **P1** |
| Tray quick-switch (F1.9) | P1 | P1 | P1 | P2 | P2 | P2 | **P1** |
| Guardrails.rs + F4.1 decision | P2 | P2 | P1 | P2 | P0 | P2 | **P0†** |
| Signed/notarised (M5) | P2 | P2 | P1 | P2 | P0 | P2 | **P1†** |
| Quiet notifications (WOW-g) | P1 | P1 | P2 | P1 | P2 | P2 | **P1** |
| Review queue/diff (WOW-h) | P2 | P1 | P1 | P1 | P2 | P2 | **P2‡** |
| Profile clone/MCP sync (F1.9) | P2 | P1 | P1 | P2 | P2 | P2 | **P2** |
| Fleet roll-up (WOW-j) | P2 | P1 | P2 | P2 | P2 | P2 | **P2** |
| Conformance kit (F1.8) | — | P1 | — | P0 | P2 | — | **P1** |
| Present/client mode `[inferred]` | — | — | P1 | — | P1 | — | **P2** |
| Budget cap alert `[inferred]` | P2 | — | P1 | P1 | — | P0 | **P2** |
\* P0 in Priya/Sara contexts — ship inside wizard.
† Blocker-class: gates trust/adoption, low daily visibility.
‡ `[inferred]` — dogfood-validate ([review:1007](full-competitive-ux-review.md#L1007)).
**Reading:** forecast+handover+inbox are each P0 for ≥2
personas, P1 for the rest — Phase 1 trio survives persona
scrutiny. Fleet/review queue are never P0: keep post-1.0 /
dogfood-gated despite vocal Devon demand. Worktree is the
only near-P0 the prior Phase 1 under-weights for Devon.
---
## 11. Validation plan (falsify before building on this)
1. **5 dogfood diaries** (Maya+Devon, 1 week): does the
   01:14-waiter recur? Are these diaries representative?
2. **Handover trust A/B:** fidelity label vs none; measure
   click-through + post-handover "lost work?" — tests §9.7.
3. **Pro-quota check:** Jonas/Alex need weekly- or 5h-shaped
   forecast? Statusline covers both (SPEC §4.1) — verify UI.
4. **Priya interviews (n=3 agencies):** is Present mode and
   per-client PDF real, or `[inferred]` invention?
5. **Sara gate test:** F4.1 options paper to a real security
   reviewer *before* building; if neither passes, remove
   enterprise positioning rather than selling around it.
6. **Alex onboarding test (n=5 students, fresh Mac):** time
   install→first session; where does jargon stop them?
   Success metric mirrors PROPOSAL: install→2 accounts
   parallel <5m ([PROPOSAL.md:142](../PROPOSAL.md#L142)) — Alex proves the 1-account variant.
## 12. Opportunity backlog by persona (sized, sequenced)
Sizing: S ≤1wk slice · M milestone item · L post-1.0.
Each row names the journey step it repairs (§9).
### Maya — keep momentum, keep trust
| Opportunity | Step | Size |
|---|---|---|
| 80% shoulder-tap + staged spare profile (no auto-switch) | Wall | S |
| Return-on-reset 1-click with delta summary ("while you were away", PROPOSAL G3) | Handover | M |
| Renewal report: "Max #2 saved N stalls ≈ $X vs API" | Advocate | M |
| Sticky ReadyHome skip per healthy profile (F2.7) | Daily | S |
### Devon — tower view or nothing
| Opportunity | Step | Size |
|---|---|---|
| Inbox default home; rail badge = unsnoozed count (UX.3) | Daily | M |
| Per-session burn + footprint in every inbox row | Wall/Daily | M |
| 20-session load test in CI before adapter #2 (F5.1) | Daily | M |
| Worktree-on-launch + same-branch warn (F1.1) | Daily | M |
| Fleet-ready inbox schema now, Fleet UI later (WOW-j) | Advocate | S |
### Priya — separation is the product
| Opportunity | Step | Size |
|---|---|---|
| Client grouping + Present mode (hide others/emails) `[inferred]` | Daily | M |
| Launch confirm when profile ≠ project's usual account (F1.4) | Daily | S |
| Redaction preview as wizard step 2, default-on export (F1.6) | Handover | M |
| Per-client budget bars on shared quota (no cross-move) | Wall | M |
| One-page provenance PDF (account/repo/spend/lineage) | Advocate | M |
### Jonas — small tank, big heart
| Opportunity | Step | Size |
|---|---|---|
| Weekly-window forecast equal to 5h in UI (not Max-only) | Wall | S |
| "Sessions remaining" translation for Pro/Lite tiers | Daily | S |
| Release-mode: archive-all-finished + sponsor cost summary | Ship | S |
| Conformance kit invites his adapter PR (F1.8) | Advocate | M |
| Quiet digest default; no streaks, no shame metrics | Daily | S |
### Sara — artefacts before features
| Opportunity | Step | Size |
|---|---|---|
| F4.1 decision doc + guardrails.rs (F4.2) | Onboard | M |
| Signed/notarised build + MDM note (M5) | Onboard | M |
| One-page data-flow: no proxy, vendor login, local-only | Discover | S |
| Click-audit export (who handed over, when, fidelity tier) `[inferred]` | Ship | M |
| Fleet off-by-default with netsec explainer | Onboard | S |
### Alex — translate, don't display
| Opportunity | Step | Size |
|---|---|---|
| "≈N sessions left" beside every credits/tokens number | Daily | S |
| Budget cap alert $5/$10 with cheap-tier suggestion (WOW-d) | Wall | M |
| Single-session focus view; advanced panels opt-in | Daily | M |
| Plain-language glossary inline (profile, handover, pack) | Onboard | S |
| Pro/GLM-Lite hero screenshots in README/store | Discover | S |
### Sequencing note
S-rows above are Phase 0/1 fillers with outsized persona
value (shoulder-tap, renewal report, weekly forecast,
glossary). None requires a new adapter. If capacity forces
choice, cut Fleet/review polish before any S-row: §9.7
shows trust breaks at wall/handover/onboard, and every
S-row repairs exactly one of those three steps.
## 13. What this changes vs the prior review
The prior persona map ([review:729](full-competitive-ux-review.md#L729))
named five jobs and eight ranked pains for an undifferentiated
"5–15 sessions/day" user. This file changes three conclusions:
1. **Pain ranking is persona-conditional.** Redaction is
   rank-8 globally but existential for Priya/Sara; admission
   is invisible to Maya but P0 for Devon/Jonas/Alex on
   constrained machines. A single ranked list (prior §743)
   mis-sequences both. Use §10's max-severity column.
2. **Willingness-to-pay is artefact-conditional.** Nobody
   pays for "quota charts". Priya pays for provenance PDFs,
   Devon via employer for ROI CSV, Maya for a renewal
   report. Monetisation follows exports and trust artefacts,
   not dashboard tiers — keep core quota/forecast free or
   Alex/Jonas (the OSS contributor funnel) never arrive.
3. **Advocacy has three shapes** (story, artefact, internal
   audit pass — §9.7), so launch proof must ship in all
   three: a saved-demo story, a downloadable provenance
   sample, and the F4.1/guardrails decision doc. Testimonials
   alone persuade only Maya and Alex.
Net: Phase 1 (forecast + inbox + honest handover) is
confirmed, but its *acceptance tests* should be written as
persona scenarios — Maya's 13:00 wall, Devon's 01:14 waiter,
Priya's Friday pack, Sara's minute-14 review, Alex's demo-eve
— not as feature checklists. A feature that passes its unit
tests but fails Priya's Friday has not shipped.
## 14. Sources
Repo: [PROPOSAL.md](../PROPOSAL.md#L49), [SPEC.md](../SPEC.md#L86), findings F1.1–F7.4 + WOW/UX in [full review](full-competitive-ux-review.md#L59), quota/pricing in [research/zcode.md](../research/zcode.md#L13), [research/claude-code.md](../research/claude-code.md#L51). Competitors: [Conductor workflow](https://www.conductor.build/docs/concepts/workflow) · [pricing](https://www.conductor.build/pricing) · [cc-switch](https://github.com/farion1231/cc-switch) · [ccusage](https://github.com/ryoppippi/ccusage) · [Claude Squad](https://github.com/smtg-ai/claude-squad).
*End — composites are design tools; never quote externally as research.*
# 08 — Architecture, Data, Supervisor & Security (Deep Review)

> Deep architecture review of AI Juggler at `main` (2026-10-12 baseline).
> Extends `docs/review/full-competitive-ux-review.md` findings F3–F5
> with crate-level evidence. Every factual claim cites `file:line`
> or a specification reference. No code was modified for this review.
> Method: full read of `crates/juggler-core`, `juggler-adapter`,
> `juggler-adapters`, `juggler-supervisor`, `juggler-app` sources.

## Verdict in one paragraph

The layering is right and the discipline is real, but the system is
a single-harness, single-connection, single-process design wearing
multi-harness, multi-session types. The three load-bearing gaps are:
(1) the adapter trait at `crates/juggler-adapter/src/adapter.rs:26-124`
cannot express SPEC §4's `usage`/`export`/`prime` contract, so M2
adapters will fork the trait or bypass it; (2) one `Mutex<Storage>`
at `crates/juggler-app/src/state.rs:17` serializes all SQLite access
while N+1 list queries and full-file transcript rescans grow with
session count; (3) the supervisor's crash/orphan story depends on a
single append-only JSONL journal with PID-only identity, which is
not safe against PID reuse. None of these require a rewrite. All
three require decisions before adapter #2, not after it.

## 1. Current architecture map

Crate graph (from `Cargo.toml:1-11` workspace members and each
crate's `src/lib.rs`):

```text
                    ┌─────────────────────────────────┐
                    │  ui/ React + TS (webview)        │
                    │  invoke() / Tauri events         │
                    └──────────────┬──────────────────┘
                                   │ commands + events
                    ┌──────────────▼──────────────────┐
                    │  juggler-app (Tauri 2 shell)     │
                    │  commands.rs:1313  lib.rs:119    │
                    │  state.rs:108  terminal.rs:213   │
                    │  AppState { Mutex<Storage>,      │
                    │    AdapterRegistry, Supervisor,  │
                    │    terminal_taps }               │
                    └──┬────────┬────────┬───────┬─────┘
                       │        │        │       │
              ┌────────▼──┐ ┌───▼─────┐ ┌▼─────┐ ┌▼──────────┐
              │ juggler-  │ │juggler- │ │jugg- │ │ juggler-  │
              │ supervisor│ │adapters │ │ler-  │ │ handover  │
              │ PTY/piped │ │ Claude  │ │core  │ │ STUB only │
              │ journal   │ │ + fake  │ │model │ │ lib.rs:13 │
              │ ring buf  │ │         │ │SQLite│ │           │
              └─────┬─────┘ └──┬──────┘ └──┬───┘ └───────────┘
                    │          │          │
              ┌─────▼──────────▼──────────▼─────┐
              │  juggler-adapter (trait contract) │
              │  HarnessAdapter, Capabilities,    │
              │  LaunchSpec, SessionWatcher       │
              │          │ imports model from     │
              │          └────► juggler-core      │
              └──────────────────────────────────┘
                    ┌─────────────────────────────────┐
                    │  juggler-link STUB only lib.rs:12│
                    │  (fleet §11, no implementation)  │
                    └─────────────────────────────────┘
```

### 1.1 What actually flows at runtime

- **Launch:** `launch_profile` at
  `crates/juggler-app/src/commands.rs:657-714` looks up profile +
  account, optionally writes a settings overlay to the managed
  state dir (`commands.rs:666-673`), calls
  `adapter.launch_spec` (`commands.rs:686`), spawns via the
  supervisor (`commands.rs:688`), then opens the `Mutex<Storage>`
  lock to touch the project and insert the session row
  (`commands.rs:692-709`). Note the ordering: the process is
  spawned *before* the session row exists, so a crash between
  `spawn` and `insert` leaves a journaled process with no row.
  The startup reconciler at `crates/juggler-app/src/state.rs:58-72`
  cannot see it because it iterates stored rows only.
- **Lifecycle:** one forwarder thread at
  `crates/juggler-app/src/lib.rs:55-70` polls
  `supervisor.recv_timeout(500ms)` and calls
  `apply_lifecycle_event` at `commands.rs:782-810`, which maps
  `Started → Running` and `Exited → Finished/Stopped/Failed`
  only. `WaitingUser`, `Stuck`, `QuotaBlocked` (model at
  `crates/juggler-core/src/model.rs:194-198`) have no producer.
- **Terminal stream:** supervisor ring (`crates/juggler-supervisor/src/ring.rs:11-17`)
  fans out to `terminal.rs` forwarders (`crates/juggler-app/src/terminal.rs:100-145`)
  that batch at 16 ms (`terminal.rs:29`) and base64-encode into
  `terminal://output` Tauri events.
- **Discovery/registry:** `default_registry` at
  `crates/juggler-app/src/state.rs:91-97` registers exactly one
  adapter (`ClaudeCodeAdapter`). `AdapterRegistry` itself
  (`crates/juggler-adapter/src/registry.rs`) is a `Vec` with
  last-wins replacement; there is no manifest loader hook in
  code despite SPEC §4 promising post-v1 TOML manifests.
- **Watching:** `ClaudeTranscriptWatcher`
  (`crates/juggler-adapters/src/claude_code.rs:410-483`) uses
  `notify` for wake-up and an unconditional full-tree rescan as
  the source of truth (`claude_code.rs:438-450`). See §5.

### 1.2 Layering strengths worth protecting

- The trait crate genuinely is the only place harness specifics
  belong: `crates/juggler-adapter/src/adapter.rs:14-22` states the
  read-only-except-`prepare_account` rule and `juggler-core`
  explicitly knows nothing about harnesses
  (`crates/juggler-core/src/lib.rs:1-12`). Keep this boundary.
- `juggler-core::fs::atomic_write` (`crates/juggler-core/src/fs.rs:18-30`)
  is temp-file + `sync_all` + rename, the correct crash-safe
  primitive; `write_settings_overlay`
  (`crates/juggler-adapters/src/claude_code.rs:598-616`) uses it.
- Error types are typed and semver-conscious:
  `CoreError` at `crates/juggler-core/src/error.rs:10-44`
  (`#[non_exhaustive]`, `SchemaTooNew`/`NotADatabase` degrade
  rather than crash), matching QUALITY.md §1 as cited in
  `crates/juggler-core/src/storage/mod.rs:3-8`.

## 2. Data model: ER diagram and field-by-field critique

### 2.1 Entity-relationship diagram (as built, v4)

```text
accounts ──< profiles ──< sessions >── projects (nullable FK, no constraint)
  │             │             │
  │             │             ├─ kind, machine (bare TEXT, no FK)
  │             │             ├─ usage_json (TEXT blob)
  │             │             └─ transcript_ref_json (TEXT blob)
  └─────────────┴── state_dir on disk (managed dirs, 0700)

projects: root UNIQUE, history_json TEXT ('{}' default)
NO TABLES FOR: usage_events, lineage, footprint_stats,
  quota windows, context packs — all promised by SPEC §6/§12
  (docs/SPEC.md:399) but absent from storage/mod.rs:82.
```

### 2.2 Schema definition (verbatim provenance)

Schema lives as string constants in
`crates/juggler-core/src/storage/mod.rs:28-82`: `V1` (accounts,
profiles, sessions), `V2` (`profiles.name`), `V3` (sessions
`kind`/`project`, `projects` table), `V4` (`projects.history_json`).
`MIGRATIONS: &[&str] = &[V1, V2, V3, V4]` at
`crates/juggler-core/src/storage/mod.rs:82`. Pragmas set in
`initialize` at `storage/mod.rs:118-133`: `journal_mode=WAL`,
`synchronous=NORMAL`, `foreign_keys=ON`.

### 2.3 Every table, every field

**`accounts`** (`storage/mod.rs:29-36`, repo `storage/accounts.rs:27-110`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | App-generated string; no format guard. Fine, but inserts accept any id — collision handling is a raw SQLite error surfaced as `Storage` (`accounts.rs:27-41`). |
| `harness` | TEXT NOT NULL | No CHECK or FK to a harness registry. `login_state_of` at `commands.rs:867-872` silently returns `LoggedOut` for any non-Claude harness, so a bad `harness` value degrades invisibly. |
| `label` | TEXT NOT NULL | No uniqueness. Two accounts both labelled "Work" are indistinguishable in the rail; `session_infos` uses label as display key (`commands.rs:947`). Add partial unique index on `(harness, label)` or document duplicates as allowed. |
| `plan` | TEXT NULL | `PlanTier` is deliberately opaque (`model.rs:115-140`) — correct for vendor churn, but there is no `plan_verified_at`, so F1.4 expiry/tier drift has nowhere to live. |
| `state_dir` | TEXT NOT NULL | Stored via `to_string_lossy` (`accounts.rs:36,58`). On non-UTF-8 paths this silently mangles the isolation root. Acceptable on macOS; must be documented, and `Account.state_dir: PathBuf` (`model.rs:154-156`) round-trips lossily through storage. |
| `created_at` | INTEGER NOT NULL | Written as `Timestamp::now().millis()` at insert (`accounts.rs:37`) but absent from the `Account` struct (`model.rs:145-157`) — the model cannot answer "when was this account added?" A v5 column already exists in spirit; expose it or drop it. |

**`profiles`** (`storage/mod.rs:38-44`, repo `storage/profiles.rs:32-116`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | Same note as accounts. |
| `account_id` | TEXT FK CASCADE | Correct: deleting an account cascades to profiles (`profiles` v1 at `storage/mod.rs:40`) and transitively to sessions. But cascade also destroys session history on account delete — there is no archive path (prior F1.3). Prefer soft-delete for accounts with sessions. |
| `name` | TEXT (v2) | Added by `storage/mod.rs:61` with `DEFAULT ''`. Legacy rows get empty names; `profile_infos` at `commands.rs:895` will render blank. Backfill to label-derived default in v5. |
| `settings_json` | TEXT NOT NULL | `SettingsOverlay{model, env}` (`model.rs:159-168`) serialized wholesale (`profiles.rs:40`). No schema version inside the blob; adding a field is backward-compatible via `#[serde(default)]`, removing one is not. Env values are stored in plaintext SQLite — see §6. |
| `default_cwd` | TEXT NULL | Lossy path conversion again (`profiles.rs:41,134-137`). Also duplicated by `Profile.default_cwd` (`model.rs:182-184`) and launch-time `cwd` argument — `launch_profile` precedence at `commands.rs:678-685` (explicit > profile default > temp dir) is reasonable but undocumented in the type. |
| `created_at` | INTEGER NOT NULL | Same orphan as accounts: stored (`profiles.rs:42`) but not in `Profile` (`model.rs:172-184`). |

**`sessions`** (`storage/mod.rs:46-57` + `66-67`, repo `storage/sessions.rs:38-145`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | Generated as `sess-<suffix>` at `commands.rs:687`. Login terminals use `login-<suffix>` (`terminal.rs:67`) but are never inserted as rows — an intentional split that must stay explicit, because `alive_adopted_sessions` only reconciles journaled spawns. |
| `native_id` | TEXT NULL | Dual source of truth with `transcript_ref.native_id` (`model.rs:311-321`). Handover writes `native_id: Some(..)` on the row but `TranscriptRef{native_id: None}` (`commands.rs:1173,1183-1186`) — the two disagree by construction today. Collapse to one: keep `native_id` on the row, drop it from `TranscriptRef`, or make `TranscriptRef` authoritative and drop the column. |
| `profile_id` | TEXT FK CASCADE | Deleting a profile deletes its sessions. Combined with account cascade, one profile delete can erase a project's entire history. Needs an archive/retention state before delete is exposed broadly. |
| `machine` | TEXT NOT NULL | `MachineId` (`model.rs:63-66`) hard-coded `"this-mac"` at `commands.rs:699,1175`. Harmless now; fleet §11 will need a real `machines` table, and a bare TEXT with no FK will accept garbage until then. Add CHECK or accept the debt explicitly. |
| `kind` | TEXT (v3) | `InstanceKind::{Terminal,Desktop}` (`model.rs:323-355`), parse failures are hard errors in `map_session` (`sessions.rs:159-167`), while `SessionStatus` parse failures are also hard errors (`sessions.rs:172-180`). This contradicts the "degrade, never panic" intent at `model.rs:242-243` — one unknown enum value makes `list()` fail for *every* session. Map unknown → sentinel + warning instead. |
| `project` | TEXT NULL | **No foreign key.** v3 adds the column bare (`storage/mod.rs:67`) with no `REFERENCES projects(id)`. Orphaned project ids are possible today; `projects_list_impl` joins in memory (`commands.rs:1212+`), so orphans silently render as "no project". v5 must add the FK (table rebuild required — see §3). |
| `cwd` | TEXT NOT NULL | Lossy (`sessions.rs:54`). Also the handover encoder does `replace('/','-')` on this string (`commands.rs:1121`) — two lossy steps composed. |
| `status` | TEXT NOT NULL | Nine-value enum (`model.rs:186-207`). Correct size; missing `Archived` for lifecycle (F1.3) and no `status_changed_at`, so "waiting how long?" (the Needs-Attention sort key in F2.5) cannot be answered. |
| `started_at` / `last_activity` | INTEGER | Epoch millis via `Timestamp` (`model.rs:74-113`). `last_activity` is set only on lifecycle transitions (`commands.rs:807`); transcript-only activity (watcher events) does not update it, so `last_activity` under-reports truly active sessions. |
| `usage_json` | TEXT NOT NULL | `UsageSnapshot{tokens, window, confidence}` (`model.rs:299-309`). Sound for a snapshot, but SPEC §6 requires an append-only `usage_events` table (`docs/SPEC.md` Storage layout) — snapshots alone cannot produce burn-down or per-day rollups. This is the single largest data-model gap vs SPEC. |
| `transcript_ref_json` | TEXT NULL | Path + native id as JSON. Path is harness-owned and read-only by contract (`model.rs:311-321`) — correct. But no `transcript_mtime`/`bytes` cache, so freshness checks must stat the file every time. |

**`projects`** (`storage/mod.rs:68-74` + `80`, repo `storage/projects.rs`):

| Field | Type | Critique |
|---|---|---|
| `id` | TEXT PK | `proj-<suffix>` generated in `touch` (`projects.rs` touch impl). Fine. |
| `root` | TEXT NOT NULL UNIQUE | Canonical root is the juggling anchor (ADR-0007, `model.rs:367-384`). UNIQUE is correct. But `touch` does read-then-insert/update without a transaction (`projects.rs` touch impl) — two concurrent launches in a new directory can race to the UNIQUE constraint. Wrap in `INSERT … ON CONFLICT(root) DO UPDATE`. |
| `name` | TEXT NOT NULL | Defaults to final path component. No rename support in the model despite SPEC §9 calling projects user-facing. Add when the rail gains rename. |
| `created_at` / `last_activity` | INTEGER | Present in both table and `ProjectRecord` (`model.rs:378-381`) — the one table where storage and model agree. |
| `history_json` | TEXT (v4) | `ProjectHistory{accounts, native_sessions}` (`model.rs:357-365`). `merge_history` overwrites the blob (`projects.rs` merge); concurrent backfill + launch `touch` can clobber. Parse-merge-write must hold the storage lock — it does today only because everything holds the one Mutex. |

### 2.4 Model types with no storage (and vice versa)

- `UsageSnapshot.window: Option<QuotaWindow>` (`model.rs:303-306`)
  exists in the model but no quota feed writes it — M3 storage
  (windows per account, not per session) is undesigned. Quota is
  an *account* property; attaching `window` to a session snapshot
  will duplicate it N times and invite inconsistency.
- `InstanceKind::Desktop` (`model.rs:330-332`) has a storage
  value and an `as_str`/`parse` pair but no producer: every
  insert writes `Terminal` (`commands.rs:700,1176`). The type is
  ahead of the supervisor, which has no desktop launch path (§4).
- `accounts.created_at`/`profiles.created_at` are stored but
  unmodelled (above). Pick one direction in v5; do not carry
  both halves.

## 3. Migration strategy: sound skeleton, three missing muscles

### 3.1 What the current strategy gets right

- Forward-only migrations indexed by `user_version`
  (`storage/mod.rs:82,135-155`), each in its own transaction with
  `pragma_update(user_version)` inside the same `tx`
  (`storage/mod.rs:149-152`). A crash mid-migration rolls back.
- Newer-database rejection via `CoreError::SchemaTooNew`
  (`storage/mod.rs:138-143`, `error.rs:19-27`) instead of silent
  downgrade — the correct call for a local-first app.
- Tampered negative `user_version` degrades to re-run-and-fail
  (`storage/mod.rs:144-146`) rather than panicking.

### 3.2 Gaps that will bite at v5+

1. **No table-rebuild migrations.** Adding the missing FK on
   `sessions.project` (§2.3) or a CHECK on `accounts.harness`
   cannot be done with `ALTER TABLE ADD COLUMN`. SQLite requires
   create-new/copy/drop/rename. The current `&[&str]` of bare SQL
   (`storage/mod.rs:82`) can express it, but there is no helper,
   no test that a v3 database upgrades cleanly, and no guidance
   that rebuilds must disable FK enforcement during copy. Decide
   the rebuild pattern *before* the first rebuild is needed.
2. **No migration tests against historical fixtures.** Tests open
   fresh in-memory databases (`open_in_memory` at
   `storage/mod.rs:112-115`). A regression that breaks upgrade
   from a real v1 file would ship unnoticed. Commit one binary
   fixture per released version and upgrade each in CI.
3. **No backup before migration.** SPEC §12 promises one
   `juggler.sqlite` (`docs/SPEC.md:399`) holding lineage and
   usage history, yet `open` migrates in place
   (`storage/mod.rs:99-105`) with no pre-migration copy. F1.7
   (rolling backups + corrupt-DB recovery) is still open. At
   minimum: copy to `juggler.sqlite.bak-v<N>` before any
   migration that rebuilds a table, and keep the last 3.
4. **JSON blobs migrate invisibly.** `settings_json`,
   `usage_json`, `transcript_ref_json`, `history_json` have no
   version field. A serde-incompatible change to
   `SettingsOverlay` (`model.rs:159-168`) would fail at row-map
   time for every profile (`profiles.rs:126`) with no migration
   hook. Embed `"v":1` or migrate blobs lazily on read; choose
   one and write it down.
5. **WAL housekeeping is absent.** WAL is enabled
   (`storage/mod.rs:122`) with `synchronous=NORMAL`, but there
   is no `wal_checkpoint`, no `busy_timeout`, and no second
   connection exists to contend — yet. The moment a read replica
   or the fleet link (§11) opens the file, default `busy_timeout=0`
   turns contention into `SQLITE_BUSY` errors. Set
   `busy_timeout=5000` now, while it is a one-line change.

### 3.3 Proposed v5 (the next migration, concretely)

```sql
-- v5: fix the known debts in one rebuild pass
ALTER TABLE sessions ADD COLUMN status_changed_at INTEGER;
ALTER TABLE sessions ADD COLUMN archived_at INTEGER;
CREATE TABLE usage_events (
  id INTEGER PRIMARY KEY,
  session_id TEXT NOT NULL REFERENCES sessions(id),
  account_id TEXT NOT NULL REFERENCES accounts(id),
  observed_at INTEGER NOT NULL,
  kind TEXT NOT NULL,          -- tokens | quota_window
  payload_json TEXT NOT NULL,
  confidence TEXT NOT NULL     -- measured|computed|estimated
);
CREATE INDEX idx_usage_session ON usage_events(session_id, observed_at);
CREATE TABLE lineage (
  id TEXT PRIMARY KEY,
  source_session TEXT NOT NULL REFERENCES sessions(id),
  target_session TEXT REFERENCES sessions(id),
  kind TEXT NOT NULL,          -- t1|t2|t3|t4
  created_at INTEGER NOT NULL,
  detail_json TEXT NOT NULL DEFAULT '{}'
);
-- sessions PROJECT FK via rebuild (create/copy/drop/rename)
CREATE TABLE machines (
  id TEXT PRIMARY KEY, label TEXT NOT NULL, is_local INTEGER NOT NULL DEFAULT 0
);
INSERT INTO machines VALUES ('this-mac','This Mac',1);
```

This is deliberately boring: it adds what SPEC §6/§7/§12
already promise (`usage_events`, lineage) and nothing more.

## 4. Adapter contract critique: what M2 will break

### 4.1 The trait as built vs SPEC §4 as written

SPEC §4 sketches (`docs/SPEC.md` adapter section) an async trait
with `discover`, `prepare_account`, `launch_spec`, `watcher`,
`usage`, `export`, `prime`. The built trait at
`crates/juggler-adapter/src/adapter.rs:26-124` is synchronous and
implements: `id`, `capabilities`, `verified_against`, `discover`,
`prepare_account`, `launch_spec`, `watcher`, plus defaulted
`scan_environment`, `import_account`, `login_url`, `login_spec`.
The three omissions are the whole handover/usage story:

- **No `usage()`** — `UsageSnapshot` exists in core
  (`model.rs:299-309`) and `USAGE_TOKENS`/`USAGE_QUOTA_FEED` bits
  exist (`capabilities.rs:20-22`), but nothing produces them.
- **No `export()`/`prime()`** — `juggler-handover/src/lib.rs:1-13`
  is doc-only; T3/T4 have no trait home. The Claude adapter
  advertises `TRANSPLANT|FORK` (`claude_code.rs:111-120`) and the
  fake advertises the same (`fake.rs:49-56`), yet the only
  transplant code is the app-level byte-copy in
  `commands.rs:1088-1194`. Advertising a capability with no
  implementing method is the F3.1 honesty bug.
- **No session-binding type.** `LaunchMode::Resume{session_id}`
  (`launch.rs:23-26`) carries a bare `String`; each adapter
  interprets it (Claude: `--resume <id>` at
  `claude_code.rs:294-296`). Codex `exec resume` and ZCode
  `--resume` differ in whether the id is a file stem, a thread
  id, or a DB row id — a stringly contract will leak.

The synchronous design is, however, defensible and should be
kept: adapters do bounded filesystem/process probes, the app
calls them from Tauri commands, and an async trait would drag
`async-trait` boxing into a semver-protected API. SPEC's
`#[async_trait]` sketch should be corrected to match the code,
not the reverse.

### 4.2 Per-harness gaps

**Codex (SPEC §4.2).** Needs `CODEX_HOME` isolation (parallel
to `CLAUDE_CONFIG_DIR` at `claude_code.rs:38`), rollout JSONL
under `sessions/**/rollout-*.jsonl`, `token_usage_record`
parsing, and `codex exec resume <id>`. Blockers in the current
trait: `watcher(transcript_root: &Path)`
(`adapter.rs:68-73`) takes a single root path, which fits
Codex, but `SessionEvent` (`watcher.rs:10-30`) has only
`Discovered`/`MessageAppended`/`Ended` — Codex's app-server
(SPEC §4.2 phase 2, `PROTOCOL_SERVER` bit at
`capabilities.rs:24`) emits approvals and turn lifecycle that
have no variant. Also `verified_against: &[&str]`
(`adapter.rs:35`) is a static list; Codex's fast churn needs
range expressions. Minimum viable Codex work is PTY + exec
mode; do not design the trait around app-server yet.

**ZCode (SPEC §4.3).** The hardest fit. Sessions live in a
vendor SQLite database (`<home>/.zcode/cli/db/db.sqlite`),
isolation is full `HOME` redirect, launch is `node zcode.cjs`,
and transplant is schema-coupled row cloning (SPEC T2 notes) —
explicitly unverified. The trait's `watcher(&Path)` assumption
(a directory of transcript files) does not hold: ZCode needs a
`DbWatcher` that opens the vendor DB read-only. Forcing ZCode
through `transcript_root: &Path` will produce a lying adapter.
ZCode should ship T1/T3 only, with `TRANSPLANT` unset, until
the row-clone is fixture-proven.

**Desktop-class harnesses (SPEC §9, ADR-0007).**
`InstanceKind::Desktop` (`model.rs:330-332`) promises windowed
apps with "no terminal surface", but `LaunchSpec`
(`launch.rs:32-43`) has only `program/args/env/cwd/needs_pty`
and the supervisor has exactly two spawn paths (PTY, piped) at
`supervisor.rs:107-111`. A desktop harness needs focus/raise,
window identity, and "is the app already running?" semantics.
None exist. Do not add `Desktop` producers until the supervisor
gains a `LaunchKind`.

### 4.3 Proposed trait changes (exact signatures)

Keep the trait synchronous and object-safe. Add four methods
with default implementations so existing adapters keep
compiling, and downgrade `TRANSPLANT` advertising until
`transplant` exists and is fixture-tested:

```rust
// crates/juggler-adapter/src/adapter.rs — proposed additions
pub trait HarnessAdapter: Send + Sync {
    // ... existing methods unchanged ...

    /// Current usage/quota for one account (read-only, no network).
    fn usage(&self, account: &Account)
        -> Result<UsageReport, AdapterError> {
        Err(AdapterError::Unsupported("no usage feed".into()))
    }

    /// Exports a native session to the neutral Transcript IR.
    fn export_session(&self, transcript: &TranscriptRef)
        -> Result<Transcript, AdapterError> {
        Err(AdapterError::Unsupported("no transcript export".into()))
    }

    /// Prepares a transplant of `source` into `target` state dir.
    /// Returns the native id to resume under the target account.
    /// Implementations must be copy-only (source never mutated).
    fn transplant_session(
        &self,
        source: &TranscriptRef,
        target_state_dir: &Path,
        target_cwd: &Path,
    ) -> Result<TransplantOutcome, AdapterError> {
        Err(AdapterError::Unsupported("no transplant".into()))
    }

    /// Builds the launch spec that primes a new session from a
    /// context pack / exported transcript (T3/T4 handover-in).
    fn prime_spec(
        &self,
        account: &Account,
        profile: &Profile,
        cwd: &Path,
        pack: &ContextPackRef,
    ) -> Result<LaunchSpec, AdapterError> {
        Err(AdapterError::Unsupported("no priming".into()))
    }
}
```

Supporting types to add:

```rust
// crates/juggler-adapter/src/usage.rs (new)
pub struct UsageReport {
    pub account: AccountId,
    pub windows: Vec<QuotaWindowState>, // label, used_pct, resets_at
    pub confidence: Confidence,         // from juggler-core
    pub observed_at: Timestamp,
}
pub struct QuotaWindowState {
    pub label: String,                  // "5h" | "weekly" | "credits"
    pub window: QuotaWindow,
}

// crates/juggler-adapter/src/handover.rs (new)
pub struct TransplantOutcome {
    pub native_id: String,              // id valid under target
    pub files_copied: u64,
    pub bytes_copied: u64,
    pub forked: bool,                   // collision resolved by fork
}
pub struct ContextPackRef {
    pub pack_dir: PathBuf,              // managed packs/ dir
    pub source_native_id: Option<String>,
}
```

And in `launch.rs`, replace the stringly resume id with a
typed binding without breaking current implementors:

```rust
pub enum LaunchMode {
    Interactive,
    Headless { prompt: String },
    Resume { binding: SessionBinding },
}
pub enum SessionBinding {
    NativeId(String),                   // today's behaviour
    TranscriptPath(PathBuf),            // Claude transcript-path resume
}
```

Migration path: Claude implements `transplant_session` by
moving `commands.rs:1120-1146` logic into the adapter (using
`TranscriptRef.path`, not newest-by-mtime), implements
`export_session` via the existing parser
(`claude_transcript.rs` `parse_transcript`/`dedupe_messages`),
and only then re-advertises `TRANSPLANT`. The fake harness
implements all four first — it is the conformance kit (F1.8) —
so every future adapter is tested against the same fixtures.
ZCode leaves `transplant_session` defaulted (Unsupported) and
does not set `TRANSPLANT`; Codex implements `NativeId` resume
plus `TranscriptPath` if `exec resume` accepts it, verified by
fixture, not assumed.

## 5. Supervisor: failure modes (FMEA)

Supervisor core: `crates/juggler-supervisor/src/supervisor.rs:52-61`
(`children: Mutex<HashMap>`, `user_stopped: Mutex<Vec>`, one
`mpsc` event channel), journal `journal.rs:26-30`, ring
`ring.rs:11-17` (512 KiB default, `supervisor.rs:19-20`).
Stop ladder: `supervisor.rs:296-318` (SIGTERM, 10 ms poll,
force-kill). Spawn paths: piped `supervisor.rs:122-178`
(2 readers + 1 waiter), PTY `supervisor.rs:180-266`
(1 reader + 1 waiter). Dropping the supervisor intentionally
does not signal children (`supervisor.rs:47-51`).

| # | Failure | Current behaviour (evidence) | Effect | Fix |
|---|---|---|---|---|
| F-1 | App crash, children survive | Journal `Spawned` without `Exited` → adopted (`journal.rs:151-196`); reconciler marks non-adopted Started/Running `Stopped` (`state.rs:56-72`) | Correct by design; adopted sessions have no PTY/ring (see F-3) | Keep; surface "adopted, terminal detached" state |
| F-2 | PID reuse after crash | Adoption checks only PID liveness via `sysinfo` (`journal.rs:188-196,199-216`); journal stores `program` (`journal.rs:35-40`) but never compares it | A recycled PID owned by an unrelated process is adopted as a live session; `stop` then SIGTERMs a stranger (`supervisor.rs:309`) | Verify `/proc`/`ps` exe path + start-time vs journal; store start-time in journal |
| F-3 | Adopted orphan control | `alive_adopted_sessions` returns ids only (`supervisor.rs:432-438`); no `ChildHandle` is rebuilt | UI shows Running with no output/input/stop path — `stop` returns `UnknownSession` (`supervisor.rs:296-303`) | Rebuild handle in adopt mode (output unavailable, stop = kill PID) or mark `Detached` |
| F-4 | PTY reader EOF vs process exit | Reader breaks on `Ok(0)/Err` (`supervisor.rs:492-504`); exit comes only from waiter thread (`supervisor.rs:242-259`) | If waiter `wait()` fails, `finished` set silently (`supervisor.rs:256-258`), no `Exited` event, session stuck Running until restart | Emit synthetic `Exited{Code(-1)}` on waiter failure |
| F-5 | Zombie accumulation | Children are waited (`supervisor.rs:158,243`); adopted orphans are *not* our children, `alive_pids` excludes Zombie/Dead (`journal.rs:208-213`) | Our zombies: none. Orphan zombies correctly counted dead. Residual: piped grandchildren reparented to init — out of scope, document it | None; keep zombie filter |
| F-6 | PTY/handle leak on stop | `stop` kills but never removes the `ChildHandle` (`supervisor.rs:285-287` insert-only; no `remove` anywhere) | `children` map + 512 KiB rings grow for process lifetime; 50 stopped sessions retain ~25 MiB (see §6) | Remove handle (or shrink ring) on `Exited` after snapshot retention window |
| F-7 | Disk full | Journal `append` opens+writes per record (`journal.rs:82-98`); spawn ignores journal error only at `record_exited` (`supervisor.rs:517`, `let _ =`), `spawn` propagates (`supervisor.rs:112-114`); SQLite WAL grows unchecked | Full disk → spawn fails *after* process started (spawn precedes journal at `supervisor.rs:107-114`), orphan with no record | Journal-before-spawn intent record; check free disk (SPEC §5.2) at admission |
| F-8 | DB lock / poisoned Mutex | `AppState.storage: Mutex<Storage>` (`state.rs:17`); poison → `AppError::StatePoisoned` (`commands.rs:830-838`); lifecycle forwarder holds lock per event (`lib.rs:60-68`) | One panic while holding the lock poisons *all* storage access until restart; no recovery path | `lock().unwrap_or_else(into_inner)` + log; longer term, split read pool (§7) |
| F-9 | Descendant leak on kill | Force-kill kills only the direct child (`supervisor.rs:149-151,234-236`); `descendants` (`supervisor.rs:347-356`) is query-only, never used by `stop` | Claude's spawned subagents/tools survive a "stopped" session and keep burning quota | Kill the process group/tree in the ladder (TERM tree → grace → KILL tree) |
| F-10 | Event channel stall | Single consumer polls 500 ms (`lib.rs:57-62`); `events_tx.send` ignores failure (`supervisor.rs:115-118,519-523`) | Bursty exits queue unboundedly in `mpsc`; dashboard lags but does not crash. Acceptable; bound or document | Document; add channel-depth metric at 20+ sessions |

Two invariants hold and must not regress: stop intent is
tracked explicitly (`user_stopped`, `supervisor.rs:55-58,305-308`)
because PTY kills surface as non-zero codes, and exit
classification (`lifecycle.rs:9-18`, `commands.rs:793-804`)
maps user-stop → `Stopped`, never `Failed`. The `Signaled(_) →
Stopped` mapping at `commands.rs:801` is, however, too broad:
an OOM-killed (SIGKILL) session also becomes `Stopped`,
hiding crashes. Distinguish SIGKILL+memory-pressure (SPEC
§5.3) from user-initiated SIGTERM.

## 6. Scale model: 10 accounts / 50 sessions

### 6.1 Thread and memory accounting (per live session)

| Resource | PTY session | Piped session | Evidence |
|---|---|---|---|
| Reader threads | 1 | 2 (stdout+stderr) | `supervisor.rs:222-224` vs `143-144` |
| Waiter threads | 1 | 1 | `supervisor.rs:242`, `157` |
| Terminal forwarder | 1 (when attached) | 0 | `terminal.rs:100-102` |
| Ring memory | 512 KiB retained | 512 KiB | `supervisor.rs:20`, `ring.rs:22-30` |
| Watcher | shared per account (notify + rescan) | same | `claude_code.rs:418-436` |

At 50 live PTY sessions: 100 supervisor threads + up to 50
forwarder threads + Tauri/tokio runtime. Thread count is not
the binding constraint (each blocks on I/O); ring memory is
50 × 512 KiB = **25 MiB retained**, plus per-push byte-by-byte
`VecDeque` churn in `ring.rs:42-47` (push iterates per byte —
a 1 MiB burst is 1M deque ops; rewrite `push` with extend/
drain before 50-session dogfood). At 10 accounts: 10 watchers,
each holding an `notify` backend handle — fine on macOS
(FSEvents coalesces), but each `poll` timeout triggers a full
rescan (below).

### 6.2 The three superlinear costs

1. **Transcript rescan.** `scan()` at `claude_code.rs:453-473`
   walks the whole `projects/` tree, `read`s **every** `.jsonl`
   fully (`claude_code.rs:458`) and counts lines, on every
   poll cycle. Cost is O(total transcript bytes) per watcher
   per poll. With 10 accounts × ~200 MB of transcripts this is
   gigabytes of re-reads per minute. Fix (F5.1): persist
   `(path, offset, mtime)` per file; parse appends only.
2. **SQLite serialization + N+1.** Every command locks the
   one Mutex (`state.rs:17`). `session_infos`
   (`commands.rs:911-961`) runs accounts list + per-account
   profile lists + sessions list + two more account lists
   (accounts listed **three times**: `commands.rs:912-917`,
   `931-936`, plus profiles loop) — O(accounts + profiles +
   sessions) queries per board refresh, and `emit_sessions`
   (`commands.rs:817-825`) re-runs it on every lifecycle event.
   At 50 sessions this is still milliseconds, but it holds the
   global lock during all of it, stalling launches. Fix: one
   JOIN query + cache account/profile maps; measure before
   adding a connection pool (WAL allows concurrent readers).
3. **`sysinfo` full refreshes.** `descendants_of`
   (`supervisor.rs:557-579`) and `alive_pids`
   (`journal.rs:200-216`) each construct `System::new()` and
   refresh **all** processes per call. Called per adoption
   check / per descendant query, this is O(processes) with
   allocation churn. Fix: one shared `System` refreshed on a
   2 s interval (SPEC §5.2 staggered refresh), reused by both.

### 6.3 Projected behaviour at 10/50 (estimates, labelled)

Assuming the fixes above are *not* made: launch latency stays
flat (spawn is per-session), board refresh degrades linearly
with stored sessions (no pagination in `sessions list` at
`sessions.rs:125-145` — every refresh materializes all rows),
and watcher CPU becomes the dominant background cost. With
the fixes: 50 sessions is comfortably within one process;
the next wall is the webview (50 xterm instances), which is
a UI virtualization question, not a Rust one. Add the
20-session fake-harness load test (F5.1) before claiming
either way — today there is no load evidence at all.

## 7. Security & compliance audit (invariant by invariant)

Invariants: proposal §5 / SPEC §8 (`docs/SPEC.md` Security:
credentials never read/copied/stored; managed dirs 0700; local only).

| Invariant | Verdict | Evidence |
|---|---|---|
| No credential reads | **Holds** | `login_state` metadata-only (`claude_code.rs:332-344`); keychain probe omits `-w` (`claude_code.rs:579-589`) |
| No credential copies | **Violated (F4.1)** | Import byte-copies `.credentials.json` (`claude_environment.rs:209-232`, perms tightened `228-230`) vs SPEC §8. Adopt prior option (b): amend SPEC to "copied only into 0700 managed dir on user-initiated import", plus guardrail test |
| Writes confined to managed dirs | **Holds** | `prepare_managed_dir` 0700+marker (`claude_code.rs:389-403,538-555`); `atomic_write` (`core/fs.rs:18-30`); import dest-must-not-exist (`claude_environment.rs:180-185`), source read-only |
| Passive / click-bound | **Holds, unenforced** | Mutations behind Tauri commands (`lib.rs` handler list); no proxy/`BASE_URL` code. But `tests/guardrails.rs` (SPEC §7) does not exist — write it before M2 (F4.2) |
| Vendor login untouched | **Holds** | Guided terminal only (`claude_code.rs:354-372`, `terminal.rs:56-73`) |
| Env secrets at rest | **New finding** | `SettingsOverlay.env` (`model.rs:165-167`) persists plaintext in `settings_json` (`profiles.rs:40`) inside `juggler.sqlite`; launching also merges full parent env into PTY (`supervisor.rs:201-206`). Redact env values in diagnostics/logs; document that profile env is secret-bearing |
| Spawn integrity | **Watch** | Binary resolved via PATH + fallbacks (`claude_code.rs:64-81`) plus env override `AI_JUGGLER_CLAUDE_BIN` (`claude_code.rs:41,68-73`) with only `is_file` checked. A writable PATH dir hijacks launches. Prefer absolute resolution + version probe (already in `discover`, `claude_code.rs:126-194`) reused at launch |

## 8. Target architecture & ADRs to write

**Target (evolution, not rewrite):** keep the 7-crate layout; move transplant/export into `juggler-handover` behind the new trait methods (§4.3); add `usage_events`+`lineage` (v5, §3.3); WAL read pool only if measured; incremental transcript tailing with persisted offsets; shared `sysinfo` refresher; supervisor gains adopt-handles, tree-kill, journal start-time; `juggler-link` stays stubbed until M6.

**ADRs to write (in order):** ADR-0008 adapter trait v2 (usage/export/transplant/prime, sync kept); ADR-0009 usage & lineage storage (v5 schema + blob versioning); ADR-0010 supervisor adoption identity (PID+exe+start-time, tree-kill); ADR-0011 credential-import carve-out (SPEC §8 amendment); ADR-0012 watcher tailing & resource-manager refresh budget; ADR-0013 desktop-harness `LaunchKind` (deferred until needed).

**Unresolved evidence:** no load test at 10/50 exists; ZCode DB schema and Codex `exec resume` path semantics unverified against real binaries; desktop-harness requirements are SPEC-only.
# 09 — Handover & Quota Engine Design (Deep Design)

> Scope: the two signature engines that make AI Juggler a *juggler* rather
> than a launcher — session handover (SPEC §7) and the usage/quota engine
> (SPEC §6). This file *designs* what the prior review only *flagged*:
> exact schemas, state machines, maths, and acceptance criteria.
> Method: repo inspection only (no live vendor calls). Every factual
> claim carries `repo/file:line` or a competitor/vendor URL.
> Verdict up front: **handover v0 must be relabelled and rebuilt before
> it is marketed; quota must ship Claude-Measured first, before any
> second adapter.** The rest of this file is the argument and the design.

## How to read this file

| Reader | Read |
|---|---|
| Solo maintainer, 10 min | §0 verdicts + §H7 + §Q9 checklists |
| Implementer, handover | §H1–H9 in order |
| Implementer, quota | §Q1–Q9 in order |
| Reviewer challenging this design | §H2/§H5 and §Q3 are the load-bearing claims |

Conventions: `H-*` = handover requirement, `Q-*` = quota requirement,
`F-H-*` / `F-Q-*` = failure. “Must” blocks v0.1-beta; “should” follows.
All timestamps are epoch-ms (`crates/juggler-core/src/model.rs:88-113`).

## 0. Verdicts (decision table — read first)

| # | Decision | Rationale (one line) | Gate |
|---|---|---|---|
| D1 | Relabel v0 to `F1 Transcript-only (experimental)` today | v0 copies 1 JSONL, no siblings (`crates/juggler-app/src/commands.rs:1120-1146`) | UI copy PR |
| D2 | Ship fidelity levels F0–F4 (§H6), keep T1–T4 as engine internals | Users need “what carries” not tier codes (SPEC `docs/SPEC.md:271-274`) | Wizard |
| D3 | Move transplant out of `commands.rs` into `juggler-handover` | Crate is doc-only (`crates/juggler-handover/src/lib.rs:1-13`) | F5.3 |
| D4 | Downgrade `TRANSPLANT` to false until §H8 fixtures pass | Capability lies today (`crates/juggler-adapters/src/claude_code.rs:111-120`) | Phase 0 |
| D5 | Quota order: Claude-Measured → ZCode-Computed → Codex-Estimated | Only Claude has a sanctioned feed (§Q1) | Roadmap |
| D6 | Never auto-rotate; recommend + one click only | ADR guardrail (`docs/adr/0003-automation-policy-compliance-guardrails.md:13-16`) | Guardrail test |
| D7 | USD ledger is *estimated* by default, never “Measured” | Token ≠ price without a dated pricing table (§Q7) | Honesty rule |

Prior review context (do not re-litigate): handover v0 is
single-file/mtime-based and T3/T4 do not exist
(`docs/review/full-competitive-ux-review.md:243-282`); quota sits at M3
after M2 breadth (`docs/review/full-competitive-ux-review.md:427-439`);
the daily loop’s #1 pain is the quota wall
(`docs/review/full-competitive-ux-review.md:743-750`). This file assumes
those findings and designs the fix — it does not restate them.

---

# PART H — HANDOVER ENGINE

## H1. Where the code actually is (reality map)

| Concern | Real location today | SPEC promise | Gap |
|---|---|---|---|
| Tier logic | None — inline in Tauri command (`crates/juggler-app/src/commands.rs:1088-1194`) | Engine crate (§7) | No engine |
| Handover crate | Doc-only, 13 lines (`crates/juggler-handover/src/lib.rs:1-13`) | “context packs, rules sync, takeover” (`crates/juggler-handover/Cargo.toml:3`) | Shell |
| Adapter contract | No export/prime/transplant methods (`crates/juggler-adapter/src/adapter.rs:26-124`) | `export`/`prime` sketched (`docs/SPEC.md:173-177`) | Trait gap |
| Capability flags | `TRANSPLANT|FORK|RESUME` advertised (`crates/juggler-adapters/src/claude_code.rs:111-120`) | Gate on verified transplant (`docs/SPEC.md:280`) | Over-claim |
| Transcript parser | Tolerant JSONL reader (`crates/juggler-adapters/src/claude_transcript.rs:63-88`) | Shared IR (`docs/SPEC.md:185`) | No IR type |
| Lineage | `HandedOver` status only (`crates/juggler-core/src/model.rs:205-207`) | A→B→A chains (`docs/SPEC.md:302`) | No lineage table |
| Storage | v1–v4, no handover/usage tables (`crates/juggler-core/src/storage/mod.rs:27-82`) | Lineage + usage store (`docs/SPEC.md:399`) | Schema gap |
| Resume | `--resume <id>` (`crates/juggler-adapters/src/claude_code.rs:294-296`) | T1 native resume (`docs/SPEC.md:271`) | Works, unsurfaced |

**H-1 (must):** the first handover PR adds trait methods or deletes the
flags. A capability bit with no implementation is a lie the UI will
render. Minimal honest trait (mirrors SPEC sketch, sync-first to match
the current non-async trait at `crates/juggler-adapter/src/adapter.rs:26`):

```rust
// crates/juggler-adapter/src/handover.rs (new)
pub struct SessionBundle { pub manifest: BundleManifest, pub files: Vec<BundleFile> }
pub struct BundleManifest {
    pub schema: u32, pub harness: HarnessId, pub native_id: String,
    pub cwd: PathBuf, pub harness_version: String, pub created_at: Timestamp,
    pub fidelity: Fidelity, pub redacted: bool,
}
pub trait HandoverSupport { // optional sub-trait; None = T3/T4 only
    fn bundle_session(&self, r: &TranscriptRef) -> Result<SessionBundle, AdapterError>;
    fn install_bundle(&self, b: &SessionBundle, t: &Account) -> Result<TranscriptRef, AdapterError>;
    fn prime_prompt(&self, ir: &TranscriptIr) -> Result<String, AdapterError>;
}
```

Keeping this a *separate* sub-trait (not new required methods) preserves
the semver-protected trait promise (`docs/adr/0002-harness-adapter-architecture.md:13-25`)
and lets the fake adapter (`crates/juggler-adapters/src/fake.rs:49-57`)
opt in incrementally. `juggler-handover` then owns orchestration;
`commands.rs` shrinks to parse-args → engine → emit.

## H2. What a Claude JSONL actually contains — and what it misses

Evidence base: committed fixtures (`crates/juggler-adapters/tests/fixtures/claude-code/`),
the tolerant parser (`crates/juggler-adapters/src/claude_transcript.rs:30-155`),
and the vendor directory map (`docs/research/claude-code.md:24,40-45`).
Format is internal/unofficial and version-churning — pin per version.

| Content | In main `.jsonl`? | Evidence | Handover consequence |
|---|---|---|---|
| User/assistant turns (`uuid`/`parentUuid` tree) | Yes | Fixture `session-basic.jsonl:1-5`; parser fields (`crates/juggler-adapters/src/claude_transcript.rs:99-110`) | Carries in F2+ |
| Token `usage` per assistant msg | Yes | `session-basic.jsonl:2-3,5`; `extract_usage` (`crates/juggler-adapters/src/claude_transcript.rs:140-155`) | Feeds quota (§Q1) |
| Inline sidechains (`isSidechain:true`) | Yes, inline | `session-subagent.jsonl:3-4`; flag (`crates/juggler-adapters/src/claude_transcript.rs:45-46,106-109`) | Byte-copy *does* carry these |
| `summary` leaf lines | Yes | `session-basic.jsonl:6`; kind counted (`crates/juggler-adapters/tests/claude_transcript.rs:48-53`) | Preserve verbatim; do not render as a turn |
| `system/compact_boundary` | Yes | `session-malformed.jsonl:6` (`subtype:compact_boundary`) | Marks context loss — see §H2.1 |
| Tool calls/results as *text* | Partially — parser drops them | `extract_text` keeps only `type:text` (`crates/juggler-adapters/src/claude_transcript.rs:120-137`); fixture asserts empty text (`crates/juggler-adapters/tests/claude_transcript.rs:59-63`) | F3 replay must distil, not “replay fully” |
| Separate subagent transcripts | **No — sibling files** | Vendor map: dir holds “subagent transcripts, spilled tool results, file-history” (`docs/research/claude-code.md:40`) | v0 drops them (F-H-04) |
| File-history snapshots / checkpoints | **No — sibling state** | 100 checkpoints, ~30d (`docs/research/claude-code.md:44`) | `/rewind` history does not transplant |
| In-session todos | **Unverified in JSONL** | SPEC claims lossless todos (`docs/SPEC.md:272`); no todo code in parser/handover path | Treat as *not carried* until §H8 proves it |
| MCP server state / live processes | No (by construction) | MCP config is per-dir (`docs/research/claude-code.md:62`); processes are runtime | Never promise transfer |
| Model KV / working context | No (for anyone) | Honest boundary (`docs/SPEC.md:276`) | F3 rebuilds context from text |
| Credentials / keychain | Must never copy | Metadata-only login probe (`crates/juggler-adapters/src/claude_code.rs:323-344`) | Excluded from every bundle |

### H2.1 Context compaction — the silent fidelity killer

`/compact` + auto-compaction replace history with a summary in-context
(`docs/research/claude-code.md:45`); the fixture’s `compact_boundary`
line is the on-disk scar. After compaction, the JSONL still *contains*
pre-compact lines, but the *model’s* working context no longer does.
Transplanting the raw file therefore over-promises: the resumed model
re-reads more than it “remembers”, yet may still miss
post-compact-only state. **H-2 (must):** fidelity computation scans for
`compact_boundary`; if present, the wizard says “session was compacted
— early detail survives as text, not as live context” and F2 drops to
the F2− label in §H6. No other tool labels this; it is cheap trust.

### H2.2 `cwd` mangling — why `replace('/','-')` is wrong

v0 encodes cwd as `source.cwd.replace('/', "-")`
(`crates/juggler-app/src/commands.rs:1121-1122`). The codebase already
knows decode is ambiguous: `transcript_cwd` reads the true `"cwd"` from
the first 25 lines because dir-name decode fails on dots/long paths
(`crates/juggler-app/src/commands.rs:1046-1060`). Encode has the mirror
problem: `/a/b.c` and `/a/b-c` collide under naive replacement, and
vendor mangling is undocumented/versioned
(`docs/research/claude-code.md:40-41`).

**H-3 (must):** never *compute* the target dir from cwd. Resolve the
source file from `SessionRecord.transcript_ref`
(`crates/juggler-core/src/model.rs:314-321,415`) — the record already
knows its file — and derive the target dir by *discovering* the target
account’s existing project dir for that cwd (scan its `projects/*`,
match by embedded `"cwd"` via `transcript_cwd`), falling back to the
vendor-documented mangling only when no dir exists, with fallback
flagged in the manifest. Golden test: cwd with dots + dashes + spaces
and a >200-char path must round-trip (fixture H-F6, §H8).

## H3. T1–T4 reality check (per harness)

SPEC tiers (`docs/SPEC.md:269-274`): T1 native resume, T2 transplant,
T3 replay, T4 pack. Reality against code + research:

| Tier | Claude | Codex | ZCode |
|---|---|---|---|
| T1 | Real: `--resume <id\|path>`, `--fork-session` (`docs/research/claude-code.md:42`; `crates/juggler-adapters/src/claude_code.rs:294-296`) | Real: `codex resume` / `exec resume <id>` (`docs/research/codex.md:44`) | Real: `--resume sess_*` / `-c` (`docs/research/zcode.md:65`) |
| T2 | Plausible, unproven: copy JSONL + resume-by-path (`docs/SPEC.md:280`); binary shared so format matches | Plausible: copy `rollout-*.jsonl` (`docs/SPEC.md:281`); `session_meta.cwd` is absolute (`docs/research/codex.md:46`) | **Blocked**: SQLite clone, schema-coupled (`docs/SPEC.md:282`); tables at `docs/research/zcode.md:63` |
| T3 | Needs IR + prime; parser drops tool text today (§H2) | Vendor precedent: Codex `/import` of Claude chats ([changelog via `docs/research/codex.md:63`](https://learn.chatgpt.com/docs/changelog)) | Source feeds T3 via SQLite reads (`docs/research/zcode.md:76-77`) |
| T4 | Pack format specced (`docs/SPEC.md:296-304`), zero code | Same pack | Same pack; redact rollout headers (`docs/research/zcode.md:64`) |

**H-4 (must):** T2-Claude ships first and alone. T2-Codex waits for the
Codex adapter; T2-ZCode stays T3 until a schema-versioned clone is
proven against two ZCode versions — SPEC already concedes this
(`docs/SPEC.md:282`), so the wizard must show ZCode targets as
“Replay” and never grey out into a fake “Transplant”.

## H4. Failure taxonomy (each needs a test + a user sentence)

| ID | Failure | Trigger in v0 | Engine behaviour (required) | User sees |
|---|---|---|---|---|
| F-H-01 | Wrong session copied | Newest-by-mtime (`crates/juggler-app/src/commands.rs:1026-1033`) with 2 live sessions | Resolve via `transcript_ref`; refuse if ambiguous | “Couldn’t tell which file is this session — pick it” |
| F-H-02 | Torn last line | Source appending during `std::fs::copy` (`crates/juggler-app/src/commands.rs:1143-1146`) | Freeze: stop source or snapshot-copy + parse-verify tail | “Source is still writing — stop it first?” |
| F-H-03 | ID collision on target | Same `sessionId` already present; no fork (`docs/SPEC.md:280` unimplemented) | Fork: new UUID + rewrite `sessionId` chain, keep `parentUuid` tree | “Already on B — opened as a fork” |
| F-H-04 | Sibling state lost | Only 1 file copied (§H2) | Copy `<session>*` set + manifest inventory | Carry-list shows ✓/✗ per item (§H6) |
| F-H-05 | Mangled-dir collision | §H2.2 encode guess | Discover-don’t-compute (H-3) | No collision possible; fallback flagged |
| F-H-06 | Version drift | Parser pinned to 2.1.296 (`crates/juggler-adapters/src/claude_code.rs:29-32`) | Pre-flight: parse target file, count malformed; abort if >0 unknown-critical | “B runs Claude X, file is Y — proceed as Replay?” |
| F-H-07 | Compacted history | `compact_boundary` present (§H2.1) | Downgrade label, keep raw copy | F2− wording |
| F-H-08 | Secrets in pack | Verbatim export (`docs/SPEC.md:296-304`) | Redact pass + preview (§H5) | Diff of redactions, approve to export |
| F-H-09 | Target quota-full | No headroom check in v0 | Quota gate (§Q6 score; block/warn) | “B at 94% — A or C recommended” |
| F-H-10 | Half-copied target | Crash mid-copy | Stage to temp dir, verify, atomic rename | Either old or new — never partial |
| F-H-11 | Return divergence | A resumed while B continued | Delta priming (§H7.3), never silent merge | “While you were away” summary shown |
| F-H-12 | Worktree/branch clash | Two accounts, same checkout (`docs/SPEC.md:113` cwd-only) | Warn same-branch live sessions; offer worktree | Branch chip + warning (§UX prior F1.1) |

Invariants for all rows: source is *copied, never moved*
(`docs/SPEC.md:287`); every failure leaves source resumable; no
state change without the confirm click (ADR-0003:13-16).

## H5. Preview, diff & redaction (the trust surface)

The pack *is* the preview artifact (SPEC §7.3,
`docs/SPEC.md:292-306`): `PACK.md` + `conversation.md` + `state.json`
+ `diffs/`. Design the wizard around rendering it *before* execution:

1. **Carry-list (always).** Generated from the bundle manifest: turns
   count, sidechains (inline ✓ / sibling files n, bytes), todos
   (carried / not found / not carried), file-history (not carried),
   MCP (reconnect on target / not carried), compaction flag. Each row
   is ✓/✗/⚠ — never prose alone. This replaces tier codes in the UI.
2. **Conversation preview.** First/last 3 turns + full-text search over
   `conversation.md`; T3 shows the *exact* priming text (tool outcomes
   distilled — labelled “distilled”, since the parser drops raw tool
   text today, §H2). Byte count + estimated target tokens shown.
3. **Diff preview (project sync + return).** Rules files
   (AGENTS.md ↔ CLAUDE.md ↔ `.zcode/AGENTS.md`, `docs/SPEC.md:289`),
   uncommitted diff stat, and the “while you were away” delta render
   as unified diffs; nothing applies without the apply click.
4. **Redaction (export-only by default).** Pattern pass over pack +
   conversation: `sk-*`/`Bearer …`/`.env` values/private keys/ZCode
   rollout headers (`docs/research/zcode.md:64`). Local same-machine
   handover defaults *off* (fidelity), export defaults *on*; the
   preview lists each redaction as a diff hunk the user can un-redact
   per item. Redacted values never touch disk outside the pack.

**H-5 (must):** no handover executes from a preview the user has not
seen. The confirm click is on the preview screen, not before it.

## H6. Proposed fidelity levels with user-facing labels

Keep T1–T4 inside the engine; show users *fidelity* — what survives.
Levels are computed per (source, target) pair at wizard-open, never
stored as a static harness property.

| Level | User label (wizard badge) | Engine tier | Carries | Explicitly does not carry |
|---|---|---|---|---|
| F4 | **Continues exactly** | T1 same-account resume | Everything native | — (same account; no copy) |
| F3 | **Same conversation, new account** | T2 transplant | Turns, inline + sibling sidechains, usage history | File-history/rewind, MCP runtime, running tools |
| F2 | **Same conversation, new account (compacted)** | T2 + `compact_boundary` | As F3, raw text complete | Live pre-compact context (model re-reads) |
| F1 | **Full story, fresh start** | T3 replay | Every turn as text, distilled tool outcomes | Native todos/file-history; target rebuilds context |
| F0 | **Briefing pack** | T4 pack | Task/decisions/TODOs/diffs summary (`docs/SPEC.md:296-304`) | Verbatim history beyond last N turns |

Rules: (a) the badge always pairs with the §H5 carry-list — a badge
alone is marketing; (b) F-level can only *drop* after pre-flight
(parse check, compaction scan, quota gate), never rise; (c) ZCode
targets cap at F1 until H-4 lifts; (d) cross-machine T2 additionally
requires same binary version + same checkout path
(`docs/SPEC.md:388`) else F1. Competitor anchor: no orchestrator
labels fidelity at all — Claude Squad/Conductor keep sessions
agent-bound (`docs/research/landscape-and-stack.md:11,60-62`); this
label is the differentiator, so it must be conservative.

## H7. Rollback & lineage data model

Today lineage is a status flip (`crates/juggler-app/src/commands.rs:1164-1188`).
Required: an append-only lineage graph that survives A→B→A and forks.

```sql
-- migration V5 (append to crates/juggler-core/src/storage/mod.rs MIGRATIONS)
CREATE TABLE handovers (
  id TEXT PRIMARY KEY, created_at INTEGER NOT NULL, actor TEXT NOT NULL DEFAULT 'user',
  source_session TEXT NOT NULL REFERENCES sessions(id),
  target_session TEXT REFERENCES sessions(id),          -- NULL until launch succeeds
  source_account TEXT NOT NULL, target_account TEXT NOT NULL,
  tier TEXT NOT NULL, fidelity TEXT NOT NULL,           -- T2/F3 etc. as shown to user
  bundle_sha256 TEXT, bundle_bytes INTEGER, manifest_json TEXT NOT NULL,
  state TEXT NOT NULL,                                  -- §H7 state machine
  failure TEXT, prev_handover TEXT REFERENCES handovers(id)  -- chain link
);
CREATE TABLE session_files (  -- bundle inventory, one row per copied file
  handover TEXT NOT NULL REFERENCES handovers(id), rel_path TEXT NOT NULL,
  sha256 TEXT NOT NULL, bytes INTEGER NOT NULL, kind TEXT NOT NULL, -- main|sidechain|spill|pack
  PRIMARY KEY (handover, rel_path));
CREATE INDEX idx_handovers_source ON handovers(source_session);
```

State machine (handover): `Staged → Verified → Launched → Confirmed`,
any → `Failed{stage,reason}`; `Confirmed → Returned` when the return
flow (§H7.3 analogue of SPEC `docs/SPEC.md:288`) completes. Transitions
are storage transactions; file install is stage-verify-rename (F-H-10)
*inside* `Staged→Verified`, so a crash never yields a half-installed
target. Rollback = delete staged/target copy + mark `Failed`; source
needs no rollback because it is never mutated. “Undo handover” in UI
means: stop target, resume source (T1) — offered for 24h, implemented
as a new lineage edge, not history rewriting.

### H7.1 Dedupe & fork identity

Resumed files re-emit history; `dedupe_messages` already implements
first-wins by `uuid`, uuid-less always kept
(`crates/juggler-adapters/src/claude_transcript.rs:167-186`, proven by
`crates/juggler-adapters/tests/claude_transcript.rs:91-101`). Bundle
install must call it pre-verify: if target already holds ≥90% of
source uuids under a different `sessionId`, treat as F-H-03 fork
(new sessionId, rewritten chain) rather than a duplicate copy.

## H8. Verification test plan (fixtures + live dogfood)

Fixture corpus lives beside existing goldens
(`crates/juggler-adapters/tests/fixtures/claude-code/`). Each new
fixture is anonymised, version-tagged, and asserted on *counts*
(lines/kinds/messages/usage) like the current suite
(`crates/juggler-adapters/tests/claude_transcript.rs:42-89`).

| ID | Fixture / test | Asserts | Blocks |
|---|---|---|---|
| H-F1 | Sibling set: main + 2 subagent files + spill | Bundle copies all; inventory complete | F-H-04 |
| H-F2 | Concurrent-append: writer thread during bundle | Freeze/snapshot; tail parses; no torn line | F-H-02 |
| H-F3 | Collision: target holds same sessionId | Fork rewrite; both resumable (fake) | F-H-03 |
| H-F4 | Compacted session (`compact_boundary`) | Label F2; raw bytes identical | F-H-07 |
| H-F5 | Malformed tail (reuse `session-malformed.jsonl`) | Degrade-count, never panic (`crates/juggler-adapters/tests/claude_transcript.rs:103-121`) | F-H-06 |
| H-F6 | Pathological cwd (dots/dashes/spaces/long) | Discover-don’t-compute round-trip (H-3) | F-H-05 |
| H-F7 | Golden path, PTY-independent (piped fake) | Handover e2e runs in CI (today PTY-bound: `docs/review/full-competitive-ux-review.md:401-412`) | Ship gate |
| H-F8 | A→B→A chain (fake harness) | Lineage graph, delta prime, no dup uuids | Return flow |

Live dogfood matrix (manual, `#[ignore]` like existing dogfood):
Claude {last verified 2.1.296 (`crates/juggler-adapters/src/claude_code.rs:32`), current latest}
× project shapes {tiny single-file, monorepo, worktree, compacted
long session, subagent-heavy} × {F3, F2, F1-to-Codex when adapter
exists}. Pass = target resumes, next turn answers a question only
answerable from pre-handover context, carry-list matched reality.
Any cell failing drops that pair’s advertised fidelity one level
until fixed — the matrix *is* the capability flag’s source of truth.

### H9. Handover acceptance criteria (ship gate)

- [ ] `TRANSPLANT` advertised only for pairs green in §H8 matrix.
- [ ] H-F1–H-F8 pass in CI (H-F7 without PTY); source bytes unchanged.
- [ ] Every wizard execution passed through §H5 preview; lineage row
  written for success *and* failure; A→B→A dogfood cell green.
- [ ] Copy-paste check: UI never says “lossless” except F4.

---

# PART Q — QUOTA ENGINE (design from scratch)

No quota code exists: `UsageSnapshot` is modelled
(`crates/juggler-core/src/model.rs:299-309`) but handover inserts it
empty (`crates/juggler-app/src/commands.rs:1182`) and SPEC §6
(`docs/SPEC.md:254-259`) is prose. This part designs the engine to
build. Goals G2/G3 (`docs/PROPOSAL.md:32-33`) depend on it.

## Q1. Data sources per harness (sanctioned only)

Rule: local files + vendor-fed local JSON only. No dashboard scraping,
no undocumented quota endpoints — OpenAI research explicitly places
scraping near the prohibited line (`docs/research/codex.md:70-72`)
and ZCode has no public quota API (`docs/research/zcode.md:77`).

| Harness | Primary source | Fallback | Confidence ceiling |
|---|---|---|---|
| Claude | Statusline stdin JSON `rate_limits.five_hour/seven_day.{used_percentage,resets_at}` — Pro/Max only ([statusline docs](https://code.claude.com/docs/en/statusline); `docs/research/claude-code.md:54`) | Transcript `usage` sums (`crates/juggler-adapters/src/claude_transcript.rs:140-155`) | **Measured** |
| Claude exhaustion | `system/api_retry` rate_limit (headless) + `StopFailure` hook (`docs/research/claude-code.md:54`) | Transcript “hit your … limit” pattern (SPEC `docs/SPEC.md:194`) | Measured event |
| Codex | Rollout `token_usage_record` (`docs/research/codex.md:43`) + `exec --json` `turn.completed.usage` (`docs/research/codex.md:59`) | `/statusline` widgets `five-hour-limit`/`weekly-limit` (`docs/research/codex.md:51`) | **Estimated** (no public API, `docs/research/codex.md:52`) |
| ZCode | SQLite `model_usage` tokens × published credit formula (`docs/research/zcode.md:76`) | Plan caps as configured ceilings (`docs/SPEC.md:210`) | **Computed** (pool total still configured) |

Claude statusline capture: install a managed statusline script per
profile dir (SPEC `docs/SPEC.md:192`) that appends the stdin JSON to
an append-only spool the engine tails — the script must be a
pass-through (chain any user statusline) or adoption will break
users’ existing setups. `/usage`/`stats-cache.json` are local-machine
only (`docs/research/claude-code.md:50`) — use as reconciliation, never
as truth for multi-device accounts. ccusage proves local-JSONL
aggregation at scale ([ccusage](https://github.com/ccusage/ccusage);
`docs/research/claude-code.md:55`); cc-switch shows tray-level
5h/weekly display is the expected shape
([cc-switch](https://github.com/farion1231/cc-switch);
`docs/review/full-competitive-ux-review.md:665-669`).

## Q2. Confidence levels (honesty contract)

Reuse `Confidence::{Measured, Computed, Estimated}` — default
`Estimated`, “never overstate trust”
(`crates/juggler-core/src/model.rs:286-297`; SPEC honesty rule
`docs/SPEC.md:259`). Engine rules:

- **Q-1:** confidence is per-(account, window, instant), stored with
  every sample — a Claude account is Measured only while statusline
  events are fresh (<10 min); stale Measured decays to Estimated in
  UI (“last measured 2h ago”), never silently stays green.
- **Q-2:** forecasts inherit the *lowest* confidence of their inputs.
- **Q-3:** ZCode credits are Computed; its 5h/weekly *pool* % stays
  Estimated until the user enters plan caps (tiers at
  `docs/research/zcode.md:14`: Lite 2k/10k, Pro 12k/60k, Max 28k/140k).
- **Q-4:** USD is never Measured (§Q7). UI badge text is exactly
  “Measured / Computed / Estimate” — no stars, no hidden footnotes.

## Q3. Window maths (5h / weekly)

Vendor facts: Claude = rolling 5h + weekly, shared across models
(`docs/research/claude-code.md:51-53`); Codex = 5h rolling + weekly,
plan-dependent, Pro has no 5h limit (`docs/research/codex.md:50`);
ZCode = 5h rolling + 7d-from-purchase weekly with peak/off-peak
credit rates (`docs/research/zcode.md:14-16`).

Definitions (all UTC epoch-ms; windows are *sets of events*, not
buckets — bucketing breaks rolling windows):

- Event: `UsageEvent{account, session?, ts, model, tokens(TokenCounts
  at crates/juggler-core/src/model.rs:261-275), credits?, source, confidence}`.
- 5h usage at `t`: sum events in `(t−5h, t]`. Weekly: sum in
  `(t−7d, t]` (ZCode: `(anchor+k·7d]` phases from purchase anchor).
- % used (Measured): trust vendor `used_percentage` verbatim; only
  *interpolate* between samples by adding locally-seen token deltas
  scaled by the last observed %-per-token slope, clamped [0,100].
- % used (Computed/Estimated): `100 × window_sum / cap`, where `cap`
  is user-confirmed plan cap; if cap unknown, show absolute burn +
  “set your plan to see %” — never invent a denominator.
- Reset: Measured → vendor `resets_at`. Estimated → earliest event
  expiry `min(ts)+5h` (rolling) — display as “≈” with confidence.

Property tests (SPEC M3-2, `docs/plan/EPICS.md:91`): window sum is
monotone under event insertion; events exactly on the boundary follow
the `(t−W, t]` rule on both windows identically; clock-skew (±5 min)
never yields negative headroom; dedupe by message `uuid` before
summing (resumed files double-count otherwise — the parser’s dedupe
contract, `crates/juggler-adapters/src/claude_transcript.rs:167-169`).

## Q4. Forecast, burn-rate & reset-queue

- **Burn-rate:** EWMA over 5/15/60-min token (or credit) rates per
  account; headline uses 15-min, cone shows 5↔60 spread. Cold start
  (<10 min data): “learning — no forecast yet”, never a fake number.
- **Time-to-exhaust:** `TTE = headroom / rate15` when rate > 0;
  display only when TTE < window remaining (else “won’t exhaust this
  window at current pace”). Forecast confidence = Q-2 inheritance;
  Estimated forecasts render as ranges (“40–70 min”), Measured as
  point + range on hover.
- **Launch/handover gate:** projected cost of the *intended* session
  (median session burn for this project, from history) vs headroom →
  OK / Tight / No, mirroring the admission advisor’s verdict language
  (`docs/SPEC.md:237`). “No” warns, never blocks launch (SPEC:237).
- **Reset-queue:** on `QuotaBlocked`, offer “Queue resume at ≈reset”.
  Implementation is a passive timer + notification; the resume itself
  fires only from the notification click (ADR-0003) — Claude already
  self-waits on v2.1.234+ (`docs/research/claude-code.md:53`), so the
  queue’s value is *cross-account* return (F-H-11 delta), not babysitting.

## Q5. Alert policy (anti-fatigue by construction)

Pain evidence: quota walls kill in-flight work (Claude #98299 via
`docs/PROPOSAL.md:23`) yet notification fatigue is itself a ranked
pain (`docs/review/full-competitive-ux-review.md:751`). Policy:

| Trigger | Channel | Cooldown / dedupe |
|---|---|---|
| TTE < 45 min (any window) | Tray + ring amber, no sound | Once per window per account |
| TTE < 15 min or ≥90% | Notification (actionable: Hand over / Queue) | Once; snooze 30 min |
| Exhausted / `QuotaBlocked` | Notification + inbox row | Once per exhaustion |
| Reset reached (queued item) | Notification (click = resume) | Once |
| Forecast was wrong by >2× | Silent log + calibration note | Never interrupts |

Digest by default: non-actionable crossings batch into an hourly
digest; per-project mute; quiet hours honoured. Every alert carries
its confidence badge — an Estimated 90% alert reads differently from
Measured, and the copy says so. Alert state machine per (account,
window): `Armed → Warned → Critical → Exhausted → (reset) → Armed`;
no backward transitions without a reset or manual re-arm, which is
what guarantees at-most-once semantics above.

## Q6. Smart target-account recommendation (suggest, never switch)

Used at NewSession, handover-target, and reset-return. It *ranks*; the
user clicks (ADR-0003; prior WOW-d at
`docs/review/full-competitive-ux-review.md:778-781`). Hard filters
first (eliminate, don’t score): wrong harness for a T2-only request,
logged-out (`crates/juggler-adapters/src/claude_code.rs:332-344`),
quota-exhausted with no reset before need-by. Then score survivors:

`score = 0.45·headroom + 0.20·affinity + 0.15·fidelity + 0.10·costFit + 0.10·calm`

- *headroom*: min(5h, weekly) headroom % (confidence-discounted:
  Estimated × 0.85 so Measured truth wins ties).
- *affinity*: this project’s historical account share (from
  `ProjectHistory.accounts`, `crates/juggler-core/src/model.rs:357-365`)
  — also the wrong-billing guard (Claude #87447,
  `docs/research/claude-code.md:36`).
- *fidelity*: F-level numeric (F3 > F1) for handover; launch-neutral.
- *costFit*: inverse projected USD (§Q7) normalised across candidates.
- *calm*: 1 − (active sessions on account / soft cap) — spreads load.

Explanation UI (required, not optional): ranked rows each with the
top-2 contributing factors in words (“80% headroom · usual account
for this repo”) + confidence badge + TTE. One “Why this order?”
expander shows all five inputs. No score without an explanation ships.

## Q7. Cost ledger in USD

Tokens already parse per message (§H2); pricing is the missing table.
Design: versioned `pricing(model, effective_from, in, out, cache_r,
cache_w, source_url)` rows, bundled + user-overridable, modelled on
ccusage’s per-model tables ([ccusage](https://github.com/ccusage/ccusage))
and cc-switch’s models.dev sync
(`docs/review/full-competitive-ux-review.md:665-669`). Ledger entry =
`UsageEvent` priced at event-time rates; rollups per session/account/
project/day (prior F1.2). Labels: token counts keep their §Q2
confidence; USD is always “Estimate” (Q-4) except statusline
`cost.total_cost_usd` passthrough, shown as “vendor-reported”.
Subscription accounts show *both* views — “≈$X at API prices (you pay
flat Max)” — so Persona A’s Max-vs-API question
(`docs/review/full-competitive-ux-review.md:782-784`) is answerable
without pretending the flat fee is metered. CSV export reuses the
rollup query; ZCode credits show alongside USD, never converted at an
invented rate (formula at `docs/research/zcode.md:14`).

## Q8. Exact schemas & engine state machines

```sql
-- migration V6 (quotas; V5 is §H7 — order by build sequence)
CREATE TABLE usage_events (
  id INTEGER PRIMARY KEY, account TEXT NOT NULL, session TEXT,
  ts INTEGER NOT NULL, model TEXT, input INTEGER NOT NULL DEFAULT 0,
  output INTEGER NOT NULL DEFAULT 0, cache_r INTEGER NOT NULL DEFAULT 0,
  cache_w INTEGER NOT NULL DEFAULT 0, reasoning INTEGER NOT NULL DEFAULT 0,
  credits REAL, usd_est REAL, source TEXT NOT NULL,      -- statusline|transcript|rollout|sqlite|api_retry
  confidence TEXT NOT NULL, msg_uuid TEXT, UNIQUE(account, msg_uuid));
CREATE TABLE quota_samples (
  account TEXT NOT NULL, window TEXT NOT NULL, ts INTEGER NOT NULL,
  used_pct REAL NOT NULL, resets_at INTEGER, confidence TEXT NOT NULL,
  PRIMARY KEY (account, window, ts));
CREATE TABLE reset_queue (
  id TEXT PRIMARY KEY, session TEXT NOT NULL, account TEXT NOT NULL,
  resume_at INTEGER NOT NULL, state TEXT NOT NULL DEFAULT 'queued', -- queued|notified|resumed|dismissed
  created_at INTEGER NOT NULL);
CREATE INDEX idx_usage_acct_ts ON usage_events(account, ts);
```

Engine pipeline (per account, single writer like storage today):
`Ingest(source event) → Dedupe(uuid) → Window fold → Forecast →
Alert FSM (§Q5) → Recommend (§Q6 on demand)`. Account quota state
machine: `Unknown → Learning → Tracking ⇄ Stale(>10m, Measured only)
→ Exhausted → (reset) → Tracking`. `Unknown/Learning` render empty
states, never 0% rings (prior F2.4). All maths is pure functions over
`usage_events` so §Q3 property tests need no harness.

## Q9. Quota acceptance criteria & build order

- [ ] Claude-Measured within ±5% of `/usage` over a full window on 2
  live accounts (SPEC M3 gate, `docs/plan/EPICS.md:95`); statusline
  chaining preserves a pre-existing user statusline.
- [ ] §Q3 properties + §Q4 cold-start + §Q5 at-most-once alerts pass
  on fixture timelines (no live account needed).
- [ ] Every rendered number carries confidence; Estimated forecasts
  show ranges; unknown caps show no % (Q-1–Q-4 audited in UI pass).
- [ ] Recommendation explains itself (§Q6) and never executes.
- [ ] Build order: spool+ingest → windows/forecast → rings/alerts →
  recommendation → USD ledger → Codex/ZCode estimators. Each step
  ships value alone; none waits for a second harness.

*End of design. Unresolved evidence is listed in the return schema,
not buried here: live-format truth (§H8 matrix) and ZCode/Codex
quota behaviour remain unproven until dogfood runs.*






# 10 — Product Proposal: Missing Features (Mini-PRDs)

> Parent: [full-competitive-ux-review.md](full-competitive-ux-review.md#L1) ·
> Inputs: [PROPOSAL.md](../PROPOSAL.md#L1), [SPEC.md](../SPEC.md#L1), [EPICS.md](../plan/EPICS.md#L1), [07-personas-and-journeys.md](deep/07-personas-and-journeys.md#L1)
> Method: repo claims cite `file:line`; competitor claims link out. Invariant per feature: prepare/passive, execute on click ([SPEC.md:26](../SPEC.md#L26), [PROPOSAL.md:62](../PROPOSAL.md#L62)).

## 0. Personas (frozen for all PRDs)

| ID | Persona | One-liner | Sessions/day | Wallet |
|---|---|---|---|---|
| P1 | Maya, Solo Multi-Maxer | 2×Max+Codex+GLM, bootstrapped SaaS | 4–8 | Self-funded |
| P2 | Devon, Staff Eng | 10–20 parallel, 4 services | 10–20 | Employer |
| P3 | Priya, Agency | 4 clients, per-client billing/audit | 6–14 | Client pass-thru |
| P4 | Jonas, OSS Maintainer | Pro+Lite, 16 GB, bursty | 3–10 | Free; contributes |
| P5 | Sara, Enterprise-Locked | SSO-approved accounts only | 2–6 | Procurement |
| P6 | Alex, Student Optimiser | 1×Pro/Lite, 8 GB, deadline-driven | 1–4 | Free only |

Source: [07-personas:56](deep/07-personas-and-journeys.md#L56) splits [PROPOSAL.md:51](../PROPOSAL.md#L51) A–D; solo, salaried, and billable users optimise for opposite things, so one UI must not conflate them. Truths: quota-blindness is pain #1 for 5/6 yet sits at M3 after adapters ([EPICS.md:26](../plan/EPICS.md#L26)); `Stuck/WaitingUser/QuotaBlocked` modelled ([model.rs:188](../../crates/juggler-core/src/model.rs#L188)) but never emitted ([commands.rs:782](../../crates/juggler-app/src/commands.rs#L782)).

**PRD shape:** Problem → Persona → Story → UX flow → Controls/states → Backend/data → Acceptance → Effort (S≤3d, M≈1–2w, L≈3–4w) → Priority → Metric. **Non-negotiables:** no auto-rotation (suggest + click only); every number carries Measured/Computed/Estimated ([SPEC.md:259](../SPEC.md#L259)); no proxy/credential reads beyond the F4.1 decision; local-first, fleet carries telemetry never credentials ([SPEC.md:377](../SPEC.md#L377)); honest empty states, never fabricate (`percent={0}`, [full review:193](full-competitive-ux-review.md#L193)).

---

## 1. Quota Engine + Forecast — P0 · L

**Problem.** Walls kill in-flight work ([PROPOSAL.md:23](../PROPOSAL.md#L23)); engine is SPEC-only ([SPEC.md:254](../SPEC.md#L254)), rings hardcoded `percent={0}`. This is G2, the noun in "juggler". **Persona:** P1 (primary), P4, P6 — small tanks first.
**Story.** As Maya launching a 3h refactor, I see "A exhausts in ~40m; B has 80% headroom" *before* Start and route correctly once.
**UX flow.** (1) NewSession rows: 5h/weekly ring + `%` text (never colour-only), burn arrow, reset countdown, forecast pill. (2) Admission=No-quota → "exhausts ~13:40; route to B?" [Use B] [Launch anyway]. (3) Handover wizard reuses ranked rows. (4) Overview rings → Usage detail. (5) 80%/90% → quiet notification (§7), never modal.
**Controls/states.** `no-data` ("start one session to measure"), `measured`, `estimated-stale` (>15m), `blocked` (+reset CTA); per-account threshold sliders; honesty-tier badge on every figure.
**Backend/data.** Statusline `rate_limits` feed ([SPEC.md:192](../SPEC.md#L192)); append-only `usage_events` + rollups ([SPEC.md:258](../SPEC.md#L258)); burn = EWMA tokens/min (30m); forecast = remaining ÷ burn with widening confidence band. Pull M3-1/M3-2 forward ([full review:917](full-competitive-ux-review.md#L917)).
**Acceptance.** Claude ±5% vs `/usage` over a full window; forecast MAPE ≤30% after 20m burn; Pro weekly renders (not Max-only). **Metric:** walls with <10m warning → <10%. **P0 · L.**

## 2. Needs-Attention Inbox — P0 · M

**Problem.** "Which of 15 needs me?" is unanswerable — statuses never emitted ([full review:201](full-competitive-ux-review.md#L201)); P2's 01:14 waiter sat 7h ([07-personas:252](deep/07-personas-and-journeys.md#L252)). **Persona:** P2, P1.
**Story.** As Devon at 08:00, I clear three overnight blockers from one queue in two minutes.
**UX flow.** (1) Rail first item `Inbox (3)` + tray count. (2) Rows: session · project · account · why-chip (WaitingUser/Stuck?/QuotaBlocked/Crashed/Review-ready) · ticking wait · primary action (Reply / Hand over / Resume-at-reset / View diff). (3) Row → session panel on that action. (4) Snooze 1h/1d, mute per session. (5) `g i`, `j/k`, `Enter`.
**Controls/states.** Row `fresh / snoozed` (excluded from badge) / `escalated` (>30m); global `all-clear` ("All clear — N running") as designed success state; `degraded` banner, never silent (cf. F2.1).
**Backend/data.** Emit statuses at choke point `apply_lifecycle_event` ([commands.rs:782](../../crates/juggler-app/src/commands.rs#L782)); heuristics per [SPEC.md:220](../SPEC.md#L220); inbox = `WHERE status IN (...) ORDER BY waiting_since`; `machine_id` column from day one (fleet-ready).
**Acceptance.** Fake-harness corpus ≥90% precision/recall ([EPICS.md:79](../plan/EPICS.md#L79)); badge = unsnoozed rows (UI test). **Metric:** median waiting-time; overnight waiters. **P0 · M** (WOW-a; ship with §1).

## 3. Smart Routing Suggestion — P1 · M (needs §1)

**Problem.** cc-switch switches without restart ([full review:665](full-competitive-ux-review.md#L665)); claude-swap auto-rotates at ~90%. Answer with a guardrail-compliant suggestion, never rotation. **Persona:** P1, P6 ("GLM for grunt work").
**Story.** As Maya, B ranks first with reasons ("82% headroom · light footprint · usual for this repo"); one click accepts; live sessions untouched.
**UX flow.** (1) NewSession pre-sorted; top row badged "Suggested". (2) "Why?" expands factors. (3) [Launch suggested] / [Choose another]. (4) At 90% on a *running* account: inbox row "New sessions → B?" affects only the next-launch default. (5) "Don't suggest GLM here" persists per project.
**Controls/states.** Score = 0.5 headroom + 0.3 footprint-fit + 0.2 project history; `suggested / dismissed-project / no-headroom` (all >85% → show reset countdowns).
**Backend/data.** §1 headroom + `footprint_stats` ([SPEC.md:235](../SPEC.md#L235)) + per-project history; pure SQLite function, no ML.
**Acceptance.** Never fires for a live-session switch (guardrail asserts no such API, [SPEC.md:136](../SPEC.md#L136)); explanation matches stored factors. **Metric:** acceptance >40%; wrong-account launches → ~0. **P1 · M.**

## 4. Worktree Isolation — P0/P1 · M

**Problem.** Session = only `cwd` ([SPEC.md:113](../SPEC.md#L113)) despite parallel-on-one-repo promise ([PROPOSAL.md:75](../PROPOSAL.md#L75)). Claude Squad (1 task = 1 worktree, [full review:579](full-competitive-ux-review.md#L579)), Conductor ([workflow docs](https://www.conductor.build/docs/concepts/workflow)), Vibe all isolate. **Persona:** P2, P1.
**Story.** As Devon with 6 parallels on one service, each session owns a branch; no clobbering.
**UX flow.** (1) "Isolate in worktree" default ON when repo has a live session. (2) Branch preview `juggler/<profile>/<short-id>` editable. (3) Row shows branch + dirty dot; shared branch → amber warn (never block). (4) Finish → [Open diff] [Clean up] (refuses listing uncommitted files). (5) Non-git cwd: hidden + tooltip.
**Controls/states.** `shared-cwd / worktree-clean / worktree-dirty / worktree-orphaned`; base-ref picker (HEAD default).
**Backend/data.** `git worktree add` under `<managed>/worktrees/<id>`; `worktree_path, branch, base_ref` on SessionRecord; `git status --porcelain` throttled 15s; managed-dir-only writes.
**Acceptance.** Concurrent-edit fixture: no conflicts; shared-branch warning fires; dirty cleanup blocked with file list. **Metric:** collision warnings; worktree adoption %. **P0 (P2) / P1 · M.**

## 5. Session Search / Replay / Archive — P1 · L

**Problem.** No archive/delete/search/retention ([full review:86](full-competitive-ux-review.md#L86)); ~4 MB/day growth ([SPEC.md:233](../SPEC.md#L233)); Codex users demand delete ([research/codex.md:80](../research/codex.md#L80)). **Persona:** P4 (200 stale rows), P2, P6 at 60 sessions.
**Story.** As Jonas, I bulk-archive 40 repro sessions, then find "the Stripe session" by content.
**UX flow.** (1) Filter chips Active / Archived / All. (2) Bulk → Archive / Export pack / Delete (typed confirm; managed files only). (3) Search: title/project/account/transcript (FTS) with snippets. (4) Replay: read-only scrubber (messages + burn over time), [Fork from here] primes a new session — Opcode checkpoints ([opcode](https://github.com/winfunc/opcode)) adapted to lineage. (5) Retention "archive after N days" (default 30, never auto-delete) + storage readout.
**Controls/states.** Lifecycle `active → archived → deleted` (tombstone kept for lineage); replay `readonly`; fork adds `forked_from` edge.
**Backend/data.** SQLite FTS5 over Transcript IR (message text; tool payloads excluded); `archived_at` migration; replay reads stored JSONL, never re-invokes harness; GC touches managed dirs only, logs counts.
**Acceptance.** 1k-session search <300 ms; archive keeps `transcript_ref` resolvable; fork primes with exact prefix. **Metric:** sessions >90d still Active ↓80%. **P1 · L** (search M + replay M + lifecycle S, in that order).

## 6. Cost Ledger — P1 · M (needs §1)

**Problem.** Tokens only, no USD ([full review:76](full-competitive-ux-review.md#L76)); ccusage proves JSONL→cost ([ccusage](https://github.com/ryoppippi/ccusage)). P3 cannot invoice; P1 cannot judge Max #2. **Persona:** P3, P6 ($5 top-ups), P1.
**Story.** As Priya, I export per-client CSV (account × project × USD, tier-labelled) in one click, not 45 min of spreadsheets.
**UX flow.** (1) Usage matrix: tokens → USD with badge "Estimated — list pricing, not your subscription fee" (API-equivalent value ≠ cash paid, permanently labelled). (2) Daily budget bar per account (P6 $5 cap alert). (3) [Export CSV] [Per-client pack] via project→client tags (§15). (4) Pricing editor + "last synced". (5) Manual subscription fees per account → "did this sub earn its $200?"
**Controls/states.** `priced / unpriced-model` (USD `—`, never `$0.00`) / `stale-pricing` (>30d); budget ok/warn/over.
**Backend/data.** Versioned pricing (model → $/MTok), models.dev sync + offline edit; USD joined at query time (never baked into `usage_events`, so repricing is retroactive); streaming CSV.
**Acceptance.** Fixture totals match ccusage on same corpus; export reconciles to screen to the cent. **Metric:** Priya invoice prep <5 min. **P1 · M.** Label discipline *is* the feature.

## 7. Notifications Centre — P1 · M

**Problem.** No quiet middle: silence today, spam if naive. Threshold alerts promised ([SPEC.md:257](../SPEC.md#L257)) with no centre/digest/mute; P2 mutes-all within a week ([full review:751](full-competitive-ux-review.md#L751)). **Persona:** P2, P4.
**Story.** As Devon, one hourly digest with three items; interrupt only if a session waits >15 min.
**UX flow.** (1) Header bell (unsnoozed count) → panel: Today / Earlier by project. (2) Items deep-link to the clearing inbox row. (3) Per-event toggles (WaitingUser, Stuck?, Quota 80/90/100%, Crash, Review-ready, Reset-arrived) × Interrupt / Digest / Off. (4) Quiet hours 22:00–08:00 digest-only. (5) Per-project mute, per-session snooze (shared store with §2).
**Controls/states.** Item `unread / read / snoozed / muted`; `empty` = "Nothing needs you" (designed success); OS-permission-denied → in-app-only banner.
**Backend/data.** `notifications(event, session_id, created_at, read_at, snoozed_until, channel)`; OS notify on Interrupt only; digest assembled on schedule.
**Acceptance.** Muted project never interrupts (fixtures); digest = exact unsnoozed actionable set; 21:59/22:01 boundary test. **Metric:** <8 interrupts/day; mute-all <5%. **P1 · M.**

## 8. Account Health — P1 · S–M

**Problem.** Presence-only login probe ([claude_code.rs:332](../../crates/juggler-adapters/src/claude_code.rs#L332)); no expiry/tier/wrong-account detection despite wrong-billing reports ([research/claude-code.md:36](../research/claude-code.md#L36)). **Persona:** P3, P5, P1.
**Story.** As Priya, "Client A usually uses Work Max — this is Personal Max. Launch anyway?" stops a mis-billing.
**UX flow.** (1) Health card per account: tier read-back where exposed, login state, expiry countdown, last-verified. (2) Degraded → cause + vendor-login CTA in embedded terminal ([claude_code.rs:354](../../crates/juggler-adapters/src/claude_code.rs#L354)). (3) Affinity warning when profile ≠ usual (plurality of last 20). (4) ReadyHome skip sticky per *healthy* profile only (fixes [full review:227](full-competitive-ux-review.md#L227)).
**Controls/states.** `healthy / expiring-soon / expired / tier-mismatch / unknown` (probe failure never shown as healthy); affinity `usual / unusual-confirmed` (remembered per project+profile).
**Backend/data.** Extend metadata-only `login_state` (length + keychain existence, never contents); hourly + on-launch refresh; affinity = derived query. F4.1 decision gates any "import keeps you logged in" claim — print it on the card.
**Acceptance.** Expired fixture cannot pass ReadyHome silently; warning fires exactly on plurality mismatch; no credential bytes in logs. **Metric:** wrong-account launches → 0. **P1 · S–M.**

## 9. Reset Queue / Scheduled Continue — P1 · M (needs §1+§2)

**Problem.** Reset "return" is a suggestion only ([SPEC.md:288](../SPEC.md#L288)); Codex users demand auto-wait parity ([#21073](https://github.com/openai/codex/issues/21073)); Claude already auto-waits ([research/claude-code.md:53](../research/claude-code.md#L53)). **Persona:** P1 (17:00 return), P4.
**Story.** As Maya blocked at 13:00, I queue "resume at reset ≈17:00"; one notification at reset, one click resumes — no clock-babysitting, no unapproved action.
**UX flow.** (1) QuotaBlocked row → [Queue at reset] showing reset time + target (return-to-source vs continue-here). (2) Queue section in inbox with countdowns, [Cancel] [Resume now]. (3) At reset: interrupt + row flips to `ready`; resume still one click ([SPEC.md:26](../SPEC.md#L26)). (4) Optional "Block launch when Admission=No" (default off).
**Controls/states.** `queued / ready / resumed / cancelled / stale` (lineage broken → explained); FIFO per account, one click each.
**Backend/data.** `reset_queue(session_id, account_id, resume_at, kind)`; `resume_at` from feed `resets_at` ([SPEC.md:192](../SPEC.md#L192)), Estimated-labelled when inferred; local timer only; relaunch reconciles past-due to `ready`, never executes.
**Acceptance.** Fake-clock guardrail: never launches unclicked; countdown error ≤60s on fixtures. **Metric:** resume latency after reset <15m. **P1 · M.**

## 10. Command Palette (⌘K) — P1 · M

**Problem.** Rail caps 15/8 without search ([WorkflowRail.tsx:184](../../ui/src/components/WorkflowRail.tsx#L184)); Recognition/Flexibility fail ([full review:759](full-competitive-ux-review.md#L759)). **Persona:** P2; everyone past 15 sessions.
**Story.** As Devon: ⌘K → "stripe" → Enter → session with [Resume] focused, <3s, mouse untouched.
**UX flow.** (1) ⌘K anywhere (terminal yields the chord, §18). (2) Fuzzy over projects/sessions (FTS snippets via §5)/accounts/*actions*. (3) Recents when empty; `↑↓` Enter; jump/resume inline, handover opens wizard. (4) Esc closes, focus returns. (5) `?` opens shortcuts.
**Controls/states.** `open / querying / no-results` (path-query alone may offer path creation); full `role=dialog` + trap (fixes [full review:872](full-competitive-ux-review.md#L872)).
**Backend/data.** Thin client over §5 FTS + static action registry; cap 50, recents outrank fuzzy score.
**Acceptance.** All daily actions ≤3 keystrokes (keyboard walk); opens <100 ms at 1k sessions. **Metric:** palette uses/day. **P1 · M.**

## 11. Review / Diff Queue — P2 · L (dogfood-gated)

**Problem.** Finished sessions pile up (P2 pain #2) yet demand is `[inferred]` ([full review:503](full-competitive-ux-review.md#L503)). Vibe's inline-comment loop ([full review:639](full-competitive-ux-review.md#L639)) and Conductor's diff+PR set expectations. **Persona:** P2, P4.
**Story.** As Devon, five finished sessions wait with diff stats; I approve three, return one with a comment, archive one — no five terminals.
**UX flow.** (1) Inbox "Ready for review": project, branch (§4), diff stat, summary line. (2) Split view: file list + read-only diff, PACK.md tab. (3) [Approve → archive] [Continue] [Comment to agent…] (sent only on [Send], never auto). (4) [Create PR] via `gh` when a worktree branch exists. (5) Empty: "No finished work waiting."
**Controls/states.** `awaiting-review / approved / changes-requested / pr-created`; diff `clean / dirty-uncommitted` (shown) / `no-changes` (say so).
**Backend/data.** `git diff <base>...HEAD` + `status` on the session worktree/cwd; comment = resume primed with the comment; decisions logged (§17).
**Acceptance.** Diff byte-matches `git diff` on fixtures; no comment reaches harness without Send. **Gate:** read-only v1 only after 5 dogfood diaries confirm demand, else defer. **Metric:** unreviewed age. **P2 · L** — most over-buildable item here; stage it.

## 12. Backup / Restore / Export — P1 · S–M

**Problem.** One WAL sqlite holds lineage + usage ([SPEC.md:399](../SPEC.md#L399)), atomic writes only ([SPEC.md:22](../SPEC.md#L22)) — no backup or corrupt-DB UX. Blocker for P5. **Persona:** P5, P3.
**Story.** As Sara after a disk error: "Database damaged — Restore from backup (yesterday 18:02)?" not a blank start.
**UX flow.** (1) Rolling backups (last 7) on clean exit + daily; Settings lists sizes/dates. (2) [Export backup…]: sqlite snapshot + packs + config (no credentials, no harness state). (3) [Restore…] previews, then destructive-confirmed [Restore and relaunch]. (4) Corrupt-boot screen: newest-good restore or "Start fresh" (renames, never deletes). (5) Transcript retention picker per §5.
**Controls/states.** `backed-up / backup-failed` (banner, never silent) / `restore-preview / restored`; fresh start → `juggler.sqlite.corrupt-<ts>`.
**Backend/data.** `VACUUM INTO` snapshots; manifest (schema + app version); restore migrates forward, refuses newer schema explicitly; atomic writes ([core/fs.rs:18](../../crates/juggler-core/src/fs.rs#L18)).
**Acceptance.** Kill -9 mid-write → restore or fresh, never half-migrated; export→wipe→import preserves lineage; bundle has zero credential-path files. **Metric:** unrecoverable-DB incidents → 0. **P1 · S–M.**

## 13. Updates: App + Harness Tracking — P1 · M

**Problem.** Updater + harness tracking specified ([SPEC.md:355](../SPEC.md#L355)) but unsigned builds block P5 and weekly harness churn ([PROPOSAL.md:132](../PROPOSAL.md#L132)) breaks adapters silently. **Persona:** P5, P4.
**Story.** As Jonas: "Claude 2.1.301 available (you run 2.1.296) — adapter verified to 2.1.298" so I update deliberately.
**UX flow.** (1) Settings → Updates: channel picker (Beta until 1.0, [SPEC.md:359](../SPEC.md#L359)), [Check now], Install-on-quit / Relaunch now. (2) Harness cards: installed vs latest vs verified range; [Open vendor updater] — never auto-updates ([EPICS.md:119](../plan/EPICS.md#L119)). (3) Verify-fail → hard block, no bypass. (4) "What changed" → CHANGELOG.
**Controls/states.** App `up-to-date / available / downloading / verified-ready / verify-failed`; harness `verified / beyond-verified-range` (probe results shown) / `check-failed-offline` (neutral).
**Backend/data.** `tauri-plugin-updater` + Releases `latest.json` ([SPEC.md:357](../SPEC.md#L357)), signature non-disableable; daily harness checks are the only sanctioned background network ([SPEC.md:23](../SPEC.md#L23)); drift from `discover()` ranges.
**Acceptance.** Bad-signature artifact rejected (negative test); channel switch resolves correct feed; never claims verified beyond declared range. **Metric:** % installs on verified range. **P1 · M** (signing CI is M5; this covers UX/tracking).

## 14. MCP / Settings Sync — P2 · M (needs §8)

**Problem.** Only project sync exists (M4-6, [EPICS.md:108](../plan/EPICS.md#L108)); no MCP/rules portability ([full review:148](full-competitive-ux-review.md#L148)). Opcode's MCP registry ([opcode](https://github.com/winfunc/opcode)) makes it table stakes. **Persona:** P3, P2.
**Story.** As Priya onboarding Client D, I clone A's profile, tick 5 of 6 MCP servers, untick the secret-bearing one — nothing moves silently.
**UX flow.** (1) [Sync from…] → checklist: MCP servers (env masked), rules files, model defaults, permission mode. (2) Per-item diff (add / overwrite / identical-greyed). (3) [Apply selected]; summary of exactly what changed. (4) Rules conversion AGENTS.md ↔ CLAUDE.md ↔ `.zcode/AGENTS.md` as a sync item type ([SPEC.md:289](../SPEC.md#L289)).
**Controls/states.** Item `new / changed / identical / conflict` (default Keep target); secrets-adjacent items default **unticked** + warning icon.
**Backend/data.** Adapter `settings_overlay` read/write on managed dirs only ([model.rs:101](../../crates/juggler-core/src/model.rs#L101)); vendor formats parsed inside adapters only; applies via `atomic_write`, logged (§17), env values never logged (key names only).
**Acceptance.** Unchecked items byte-identical after sync (hash test); conflict never overwrites by default. **Metric:** new-account time-to-productive <5 min. **P2 · M.**

## 15. Multi-Repo & Multi-Client Grouping — P2 · M

**Problem.** Flat rail ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L1)); P3's clients and P2's services (one product, many repos) lack grouping and screen-share safety. Vibe does per-repo worktrees ([full review:641](full-competitive-ux-review.md#L641)). **Persona:** P3, P2.
**Story.** As Priya on a Client B call, "Present mode: Client B" hides all other clients' names, sessions, notifications.
**UX flow.** (1) Optional Group tag (client/service) + collapse + filter chips (also relieves the 15-row cap; §10 backs it). (2) Group repo-set launch: primary repo = cwd, siblings listed in the priming pack; §4 isolation per-repo. (3) Present mode (`⌘⇧P`): only that group visible, emails masked, persistent banner while active. (4) Per-group budget (§6) + quota attribution.
**Controls/states.** Group `expanded / collapsed / active-filter`; Present `off / on(group)`. Group delete untags, never deletes projects.
**Backend/data.** `groups(id, name, colour)` + nullable `projects.group_id` (ungrouped is first-class); Present is a pure UI filter (no data movement).
**Acceptance.** Present-mode DOM contains zero other-group strings (screenshot-diff test); launch pack lists siblings exactly. **Metric:** Present-mode adoption. **P2 · M** (Present alone is S, may pull forward).

## 16. Templates & Prompts Library — P2 · S–M

**Problem.** Every session starts blank; P4 re-types repro scaffolds, P6 lacks exemplars. No template concept in SPEC §9. Precedent: Opcode agent library ([opcode](https://github.com/winfunc/opcode)). **Persona:** P4, P6.
**Story.** As Jonas, Template "Issue repro" pre-fills the opener with repo slots resolved; I edit, then launch.
**UX flow.** (1) NewSession Template picker (None default; built-ins: Issue repro, PR review, Refactor-with-tests, Cheap grunt pass). (2) Slots `{{project}} {{branch}} {{issue}}` become inline fields when unresolved — never sent literally. (3) [Save prompt as template]; templates are local markdown, CRUD in Settings. (4) In-terminal `/slug` snippets deferred — v1 is launch-time only and says so (terminal passthrough).
**Controls/states.** `builtin` (read-only, duplicable) / `custom / invalid` (bad slot blocks launch until fixed or removed). No sharing/sync in v1 (§22).
**Backend/data.** Markdown under `<app-data>/templates/` (editable outside the app — deliberate); slots resolved from session context; use logged by id only (§17).
**Acceptance.** Unresolved `{{slot}}` never reaches a harness (launch-path test); built-ins render on two fixture projects. **Metric:** template launch share; P6 time-to-first-prompt. **P2 · S–M.**

## 17. Audit Log — P1 (P5/P3) · S–M

**Problem.** Lineage stops at `HandedOver` ([full review:282](full-competitive-ux-review.md#L282)); no record of who clicked what — the artefact Sara's review and Priya's Q&A need. Fleet commands already require attribution ([EPICS.md:140](../plan/EPICS.md#L140)); build locally now. **Persona:** P5, P3.
**Story.** As Priya asked "what did AI do Tuesday?", I export an audit CSV and answer from record, not memory.
**UX flow.** (1) Settings → Audit: reverse-chron, filter by project/account/type/date. (2) Session panel shows its slice (lineage + timestamps + actor). (3) [Export CSV/JSON] with redaction (paths shortened, emails masked). (4) Append-only UI, no edit/delete; retention follows §5. (5) Confirm steps preview their audit line ("Records: Handover sess-42 A→B, T2, you clicked Confirm").
**Controls/states.** Events: `launch, stop, handover, return, resume, review-decision, sync-apply, pack-export, settings-change, backup, restore`; actor `local-user` → `console:<machine>` (column ships now).
**Backend/data.** `audit_log(id, ts, actor, event, subject_type, subject_id, detail_json)` written in the action's transaction; `detail_json` allowlisted only (never prompts, env values, credential paths).
**Acceptance.** Every state-changing command emits exactly one row (coverage test); redacted export has no absolute home paths (regex). **Metric:** exports/month. **P1 · S–M** — cheap now, costly to retrofit.

## 18. Keyboard System — P1 · M (pairs §10)

**Problem.** Only `⌘B/⌘N` exist ([App.tsx:205](../../ui/src/App.tsx#L205)); modals lack trap/return-focus, tabs lack arrows, splitter not keyboard-operable ([full review:869](full-competitive-ux-review.md#L869)). Product for P2, procurement gate for P5. **Persona:** P2, P5.
**Story.** As Devon, a full morning — triage, jump, resume, handover — without the mouse, focus never lost.
**System.** L1 global: `⌘K ⌘N ⌘B ⌘⇧P`, `g i/p/s`, `?` sheet. L2 list: `j/k Enter x s m`. L3 session: `r h ⌥⌘T`. Terminal focused → single-key layer suspends (Esc exits; ⌘-chords live). Cheat sheet generated from the registry (cannot drift). All modals `role=dialog` + trap + Esc + return-focus; `prefers-reduced-motion` + AA subtext (≥4.5:1) ship together — sales features for P5.
**Controls/states.** Status-bar indicator `⌨ App / ⌨ Terminal — Esc to exit`; OS-reserved chords win; remapping UI is P2.
**Backend/data.** Single frontend keymap registry feeding menus, tooltips, sheet, palette. No storage.
**Acceptance.** 12-action mouse-disabled walk passes; trap test per modal; sheet diff-matches registry in CI. **Metric:** keyboard task-completion rate. **P1 · M.**

## 19. Tray + Global Hotkey — P1 · S–M

**Problem.** Tray quota + hotkey promised ([SPEC.md:352](../SPEC.md#L352)), absent in `ui/src`. cc-switch's tray row is the incumbent's home screen ([full review:665](full-competitive-ux-review.md#L665)). **Persona:** P1 (08:30 glance), P6.
**Story.** As Maya, the menu bar reads "2 need you · A 78% · reset 17:00"; one click lists accounts + quick-launch.
**UX flow.** (1) Tray title = worst signal (inbox > quota > all-clear). (2) Menu: inbox lines (→ session), account rows % + reset (→ Accounts), top-5 recent quick-launch (suggested profile, still a click), [Open] [Quit]. (3) Hotkey `⌥⌘J` (remappable) → floating instances list (palette-lite, keyboard-first). (4) Badge dot only when unsnoozed inbox >0. (5) Quit with live sessions warns with count.
**Controls/states.** `all-clear / needs-attention(n) / quota-low (≥90%) / blocked`; colour always paired with text.
**Backend/data.** One pre-aggregated core query (inbox + quota + recents) pushed on change, never webview-polled; Tauri global-shortcut plugin; tray is a view, never a state owner.
**Acceptance.** Summary updates <1s after fixture change; quick-launch logs the click (§17); hotkey list keyboard-operable. **Metric:** tray-open → action rate. **P1 · S–M.**

## 20. Fleet Teaser — P2 · S now, L later

**Problem.** Fleet (M6/M7, [SPEC.md:455](../SPEC.md#L455); gRPC/mTLS [SPEC.md:373](../SPEC.md#L373)) is correctly late but must not be invisible or data-unready. **Persona:** P2, P1.
**Story.** As Devon, Settings previews v0.2 fleet and my inbox already has a machine column — upgrade feels like a filter opening.
**UX flow (v0.1 only).** (1) Settings → Fleet card: honest status, pairing explainer (code/QR, LAN/VPN, no relay, credentials never leave, [SPEC.md:383](../SPEC.md#L383)), [Notify me] (local flag, no telemetry). (2) Local machine renders as disabled "This Mac" filter chip wherever machines will appear — IA rehearsed, zero fake data. (3) No ComingSoon nav item; the card is the whole surface.
**Controls/states.** Only `local-only` in v0.1; schema ships now: `sessions.machine_id`, `audit.actor`, inbox grouping.
**Backend/data.** Now: three columns + M5 transport spike only ([EPICS.md:121](../plan/EPICS.md#L121)). Later: read-only remote board → remote control → cross-machine handover last (T2 needs same binary + checkout path, [SPEC.md:388](../SPEC.md#L388)).
**Acceptance.** No fleet dead-end (IA test); local sessions round-trip with `machine_id`; copy claims nothing the spike hasn't proven. **Metric:** [Notify me] flags (M6 demand signal). **P2 · S.**

## 21. Cut / Defer List

| Item | Verdict | Why |
|---|---|---|
| ZCode adapter before Claude loop | Defer Phase 3 | Bundle-only, undocumented ([SPEC.md:282](../SPEC.md#L282)); fragility before §1/§2 prove value |
| Codex quota polish | Ship labelled, defer tuning | No API ([SPEC.md:203](../SPEC.md#L203)); honest Estimated beats fiction |
| T3/T4 marketing | Cut until proven | Crate doc-only ([full review:243](full-competitive-ux-review.md#L243)); advertise RESUME-only (F3.1) |
| Kanban clone | Cut | Sunset category; import users, not the metaphor ([full review:512](full-competitive-ux-review.md#L512)) |
| Full comment-to-agent | Behind §11 gate | `[inferred]` demand; read-only diff first |
| Public adapter SDK | Post-1.0 | Publish in-tree conformance suite in M2 (F1.8) instead |
| Overview L3 analytics | Defer | Fix L1→L2 inversion first ([full review:861](full-competitive-ux-review.md#L861)) |
| Replay fork (full) | Stretch | Ship read-only scrub; fork only if replay is used |
| Win/Linux packaging | Post-1.0 | macOS-first decided ([SPEC.md:18](../SPEC.md#L18)); CI cross-builds keep core honest |
| Fleet remote control | v0.3 | Read-only board first; commands widen audit surface |

## 22. Anti-Features (never build)

1. **Auto-rotation / auto-switch at threshold** (claude-swap pattern) — violates guardrail 2 ([PROPOSAL.md:62](../PROPOSAL.md#L62)). Offer §3 suggestion + click instead.
2. **Traffic proxy / gateway / request routing** — prohibition zone ([PROPOSAL.md:42](../PROPOSAL.md#L42), [PROPOSAL.md:125](../PROPOSAL.md#L125)).
3. **Login automation, captcha/OTP handling, credential storage** ([PROPOSAL.md:43](../PROPOSAL.md#L43)).
4. **Account pooling / sharing between people** and team RBAC/SSO admin in v1 ([PROPOSAL.md:45](../PROPOSAL.md#L45)); Fleet = *my* machines only.
5. **A replacement chat UI** ([PROPOSAL.md:46](../PROPOSAL.md#L46)) — embed terminals; harnesses own UX.
6. **Autonomous task assignment** ("which account deserves this") ([PROPOSAL.md:44](../PROPOSAL.md#L44)) — inbox + suggestions, human assigns.
7. **Auto-accept permissions** (Claude Squad `-y` daemon pattern) — crosses the click-bound invariant; do not copy.
8. **Fabricated metrics** (hardcoded rings, "Machine OK") — honest empty states only; a supervision tool that lies about state is worse than none.
9. **Cloud relay / mandatory account / default telemetry** — local-first is the trust claim ([SPEC.md:23](../SPEC.md#L23)).
10. **Gamification** (streaks, burn leaderboards) — wrong incentive at the quota wall; P4 explicitly rejects it.
Test for any proposal: *does it require sharing, proxying, auto-rotating, or auto-assigning?* Yes → decline citing the guardrail.

## 23. Naming & Positioning Proposal

**Keep the name; fix the sentence.** "AI Juggler" already encodes the wedge — the risk is sounding like another orchestrator. Current README undersells state ("spec phase", contradicted by CHANGELOG per [full review:25](full-competitive-ux-review.md#L25)); positioning must lead with juggling, not sessions.
- **Tagline (primary):** "Run every AI coding account you pay for — quota-aware, without the terminal sprawl."
- **Category line:** "Orchestrators run your agents. Switchers swap your account. Juggler does both, quota-aware." ([full review:483](full-competitive-ux-review.md#L483) — make it true via §1/§2 before printing it.)
- **Hero screenshot (only one matters):** NewSession with forecast — "A exhausts in ~40m; B has 80% headroom" — plus a Pro/Lite variant so P6 doesn't self-exclude. Never lead with a board screenshot; boards are commodities.
- **Vocabulary:** retire "transplant" from user copy (keep in SPEC); user words: *Hand over*, *Continue on…*, *Return when reset*. Persona gloss at first use for P6; no "instance" in user copy until §2's merge decision (Instances/Sessions conflation, F2.2) is resolved — say *Sessions* and *Accounts*.
- **Trust page (one page, linkable):** guardrails §5 of PROPOSAL + decided F4.1 + "we never proxy, never rotate, never read credentials" with `guardrails.rs` as the evidence link. This page *is* Sara's aha and Priya's sales artefact.
- **Sunset window:** one honest comparison/migration page for Vibe Kanban + Crystal users ([full review:507](full-competitive-ux-review.md#L507)) + config import (§21) — import users, not the kanban metaphor.

## 24. OSS Monetisation Options (core stays free)

Constraint: quota/forecast/inbox are the core promise — paywalling them destroys the wedge and P6 trust. Monetise *around* supervision, never *inside* the juggle. MIT licence decided ([SPEC.md:466](../SPEC.md#L466)); all options below respect it (no relicensing retroactively).

| Option | What sells | Persona fit | Risk |
|---|---|---|---|
| GitHub Sponsors + Open Collective | Goodwill | P4, P1 ($5–15/mo) | Voluntary; not a business |
| Pro Pack (one-off $49) | §6 per-client PDF, §11 PR polish, §20 fleet-early | P1, P3 | Must not gate §1/§2/§3 |
| Agency seats ($30–60/seat/mo) | §15 Present mode, §17 audit export, support | P3 | Support load; keep optional |
| Enterprise support contract | Signed builds, security reviews, MDM help | P5 (procurement) | Never promise SSO/RBAC (non-goal) |
| Paid adapter maintenance | Sponsor-a-harness (e.g. ZCode upkeep) | P4 ecosystem | Governance: no pay-to-prioritise core |
| Hosted update/signing (none) | — | — | Do not cloud-ify; relay = trust + ToS risk |

**Recommendation:** Sponsors now; Pro Pack at v0.1-beta *only if* §6/§15/§17 ship polished; defer any seat pricing until Priya-class dogfood proves invoice value. Publish a "Core free forever" pledge — it is a distribution asset for P6's campus funnel, not charity.

## 25. MoSCoW + RICE

RICE = Reach (dogfood users, 10= all 6 personas) × Impact (0.25–3) × Confidence % ÷ Effort (person-weeks). Scores order *within* MoSCoW bands; guardrail/blocker items (†) jump bands regardless of score.

| # | Feature | MoSCoW | R | I | C | E(wk) | RICE | Phase |
|---|---|---|---|---|---|---|---|---|
| 1 | Quota + forecast | **Must** | 10 | 3 | 80 | 4 | 600 | 1 |
| 2 | Needs-attention inbox | **Must** | 9 | 3 | 90 | 2 | 1215 | 1 |
| 8 | Account health | **Must**† | 7 | 2 | 80 | 1 | 1120 | 1 |
| 12 | Backup/restore | **Must**† | 6 | 2 | 90 | 1 | 1080 | 1 |
| 17 | Audit log | **Must**† | 5 | 1 | 90 | 1 | 450 | 1 |
| 3 | Smart routing | Should | 8 | 2 | 70 | 2 | 560 | 1–2 |
| 4 | Worktree isolation | Should | 8 | 2 | 80 | 2 | 640 | 1 |
| 5 | Search/replay/archive | Should | 8 | 2 | 80 | 4 | 320 | 1–2 |
| 6 | Cost ledger | Should | 7 | 2 | 80 | 2 | 560 | 2 |
| 7 | Notifications centre | Should | 8 | 1 | 80 | 2 | 320 | 1–2 |
| 9 | Reset queue | Should | 7 | 2 | 70 | 2 | 490 | 2 |
| 10 | Command palette | Should | 8 | 2 | 90 | 2 | 720 | 1 |
| 13 | Updates (UX/tracking) | Should | 9 | 1 | 80 | 2 | 360 | 1–2 |
| 18 | Keyboard system | Should | 6 | 2 | 90 | 2 | 540 | 1 |
| 19 | Tray + hotkey | Should | 8 | 1 | 90 | 1 | 720 | 1 |
| 11 | Review/diff queue | Could | 6 | 2 | 40‡ | 4 | 120 | 2–3 gated |
| 14 | MCP/settings sync | Could | 6 | 1 | 70 | 2 | 210 | 2–3 |
| 15 | Multi-repo grouping | Could | 5 | 2 | 60 | 2 | 300 | 2 |
| 16 | Templates/prompts | Could | 6 | 1 | 70 | 1.5 | 280 | 2 |
| 20 | Fleet teaser (schema) | Could | 5 | 1 | 90 | 1 | 450 | 1 (S only) |
| — | Fleet control, SDK, kanban | **Won't (v1)** | — | — | — | — | — | post-1.0 |

† Blocker-class: gates P5/P3 trust. ‡ `[inferred]` demand — dogfood-gates §11.

**Sequencing (one line each).** Phase 0 (days): truth pass, F4.1 decision, `guardrails.rs`, honest empty states. Phase 1 (v0.1-beta): §1 + §2 + handover-honesty + §4 + §8/§12/§17/§10/§18/§19 + §20-schema. Phase 2: Codex + §3/§6/§9/§14 + scale fixes (incremental tailing, shared sysinfo, joined queries — F5.1). Phase 3: ZCode + §11-gated/§15/§16 + signed 1.0; Fleet M6/M7 after 1.0 on the Phase-1 schema.

## 26. Risks & Falsification

| Risk | Kill-test (do first) |
|---|---|
| Forecast wrong on Pro/weekly windows | Dogfood P4/P6 cohort 1 week; MAPE per window type, not blended |
| Inbox becomes second spam channel | Count interrupts in dogfood; >8/day → tighten defaults before beta |
| Worktree confuses non-git users | First-run test with P6: can they say where files went? |
| USD misread as cash owed | Show ledger to 3 users; ask "what did you pay?" — any confusion = copy bug, fix before ship |
| Priya Present mode is invented | 3 agency interviews before §15 beyond the S slice (§20 validation in 07-file) |
| Review queue demand is Devon's alone | §11 gate: 5 diaries or it stays read-only |

**What would change this proposal:** a vendor shipping sanctioned multi-account + quota APIs (absorbs §1/§3 into adapters), or dogfood showing handover — not quota — is the daily pain (re-promote handover UX above §6/§9). Until then: **forecast + inbox + honest handover is the product; everything else is retention.**

## 27. Dependency Graph & Critical Path

Nothing here is independent. The graph below is the build order;
violating it re-creates the parent review's inverted roadmap.

```
§8 Account health ──► §3 Routing ──► §9 Reset queue
§1 Quota/forecast ──► §3, §6, §9, §19 (tray numbers)
§2 Status emission ─► §7 Notifications ─► §19 badge
§2 ─► §11 Review-ready ─► §11 Diff queue
§5 FTS index ───────► §10 Palette (snippets)
§4 Worktree ────────► §11 Diff base_ref
§17 Audit log ──────► §14 Sync, §11 Decisions, Fleet actors
§15 Groups ─────────► §6 Per-client export
§18 Keymap registry ► §10 Palette, §15 Present, §19 Hotkey list
§20 Schema (machine_id) ► Fleet M6 (no migration later)
```

**Critical path to v0.1-beta:** §8 (S) → §1 (L) → §2 (M) →
handover-honesty (outside this file, F3.3–F3.5) → §10+§18 (M).
Everything else parallelises around that spine. If the spine
slips, cut §5-replay and §11 first — they are the only Must-adjacent
items with a read-only fallback (search-only, no fork).

**Data migrations in order:**
1. `sessions.machine_id, waiting_since` (§2/§20) — additive, safe.
2. `usage_events` burn-rate materialisation (§1) — backfill from JSONL.
3. `sessions.archived_at, worktree_path, branch, base_ref` (§4/§5).
4. `audit_log` + `notifications` + `reset_queue` (§17/§7/§9).
5. `groups` + `projects.group_id` (§15) — last; pure metadata.

Each migration ships with a down-test from an empty schema, per
[EPICS.md:44](../plan/EPICS.md#L44) round-trip discipline.
No migration may read harness state dirs; all read `juggler.sqlite`
plus managed-dir JSONL only.

## 28. Expanded UX Flows (three make-or-break journeys)

### 28.1 Maya: quota wall at 13:00 (validates §1+§9+handover)

| Step | Surface | System does | Maya does |
|---|---|---|---|
| 11:20 | Tray | Account A crosses 80%; digest item queued | Nothing (no interrupt) |
| 12:40 | Session header | Forecast pill: "A exhausts ~13:10" | Glances, keeps working |
| 13:02 | Inbox | QuotaBlocked row appears, badge 0→1 | Clicks row |
| 13:03 | Handover wizard | Targets ranked; B 82% headroom; fidelity listed | [Hand over to B] |
| 13:04 | Session B | Primed with transcript + siblings; lineage recorded | Continues task |
| 16:58 | Notification | Interrupt: "A reset — return?" | [Resume on A] |
| 17:00 | Session A | Resumed with "while you were away" delta | Reviews, ships |

Failure branch: if B also <10% headroom, wizard shows reset
countdowns instead of a ranking and offers [Queue at reset] (§9)
as primary. The user is never offered a dead-end handover.

### 28.2 Devon: Monday 08:00 triage (validates §2+§18+§11)

| Step | Keys/action | Result |
|---|---|---|
| Open app | `g i` | Inbox: 2 waiting, 1 stuck?, 1 quota, 3 review-ready |
| Oldest waiter | `j j Enter` | Session panel, permission prompt focused |
| Answer | Type + Enter in terminal | Row clears; badge decrements live |
| Stuck? row | `Enter` → [View output] | Last-output pane shows model-wait vs hang evidence |
| Review batch | `x x x` select → [Approve → archive] | Three archived; §17 rows written |
| Jump to spike | `⌘K` "spike" Enter | Fourth repo session focused |

Target: 7 inbox items cleared in <4 minutes, mouse untouched.
Measure this exact script in dogfood; >6 minutes fails §2.

### 28.3 Priya: Friday invoice (validates §6+§15+§17)

| Step | Surface | Outcome |
|---|---|---|
| Filter | Group chip "Client B" | Rail + Usage scoped; banner confirms filter |
| Verify | Usage matrix | "Estimated" badges visible; unpriced model shows `—` |
| Cross-check | Session slice (§17) | 14 sessions, 2 handovers, all timestamped |
| Export | [Per-client pack] | CSV + audit JSON in one bundle |
| Share | Present mode for call | No other client string in DOM |

Failure branch: any unpriced model in range → export still runs,
USD column shows `n/a (model unpriced)` and the cover note says so.
Never silently sum a partial total as if complete.

## 29. Persona × Feature Coverage Check

P0/P1 cells must have a Phase-1 home; "—" means deliberately unserved.

| Feature | P1 | P2 | P3 | P4 | P5 | P6 | Owner phase |
|---|---|---|---|---|---|---|---|
| §1 Quota/forecast | P0 | P0 | P1 | P0 | P1 | P0 | 1 |
| §2 Inbox | P1 | P0 | P1 | P1 | P1 | P2 | 1 |
| §3 Routing | P1 | P1 | P1 | P1 | P1 | P0 | 1–2 |
| §4 Worktree | P1 | P0 | P1 | P1 | P2 | P2 | 1 |
| §5 Lifecycle | P1 | P1 | P1 | P1 | P1 | P1 | 1–2 |
| §6 Cost ledger | P1 | P1 | P0 | P1 | P2 | P0 | 2 |
| §7 Notifications | P1 | P1 | P2 | P1 | P2 | P2 | 1–2 |
| §8 Health | P1 | P1 | P0 | P2 | P0 | P2 | 1 |
| §9 Reset queue | P1 | P1 | P2 | P1 | P2 | P1 | 2 |
| §10 Palette | P1 | P0 | P1 | P1 | P2 | P1 | 1 |
| §11 Review queue | P2 | P1 | P1 | P1 | P2 | P2 | gated |
| §12 Backup | P2 | P2 | P1 | P2 | P0 | P2 | 1 |
| §13 Updates | P2 | P2 | P1 | P1 | P0 | P2 | 1–2 |
| §14 MCP sync | P2 | P1 | P1 | P2 | P2 | P2 | 2–3 |
| §15 Grouping | — | P1 | P0 | — | P1 | — | 2 |
| §16 Templates | P2 | P2 | P2 | P1 | P2 | P1 | 2 |
| §17 Audit | P2 | P2 | P0 | P2 | P0 | — | 1 |
| §18 Keyboard | P1 | P0 | P2 | P1 | P1 | P2 | 1 |
| §19 Tray/hotkey | P1 | P1 | P1 | P2 | P2 | P1 | 1 |
| §20 Fleet teaser | P2 | P1 | P2 | P2 | P2 | P2 | 1 (S) |

Reading: no persona is P0-orphaned in Phase 1. P3's audit/health
pair and P5's backup/signed-build pair are the trust gates;
P6 is served by §1+§3+§6-translation, not by any P6-only feature —
deliberate: small-tank correctness beats student-specific UI.

## 30. Validation Plan (before any Phase-1 code freezes)

1. **Five dogfood diaries** (P1 + P2 archetypes, 1 week): does the
   01:14-waiter and 13:00-wall recur as modelled? Re-score RICE
   Impact from diaries, not opinion.
2. **Fidelity-label A/B:** handover with vs without carry-list;
   measure click-through and post-handover "lost work?" reports.
3. **Forecast calibration:** log predicted vs actual exhaustion
   per window type from day one (even before UI ships) so Phase 1
   launches with error bars, not hopes.
4. **Present-mode falsification:** 3 agency owners react to a
   clickable mock; if leakage fear doesn't move, cut §15 to tags-only.
5. **Sara gate paper:** F4.1 options + §12/§17/§13 artefacts to a
   security reviewer *before* building; a "no" removes enterprise
   language from positioning (§23) rather than blocking the roadmap.
6. **Small-tank check:** verify statusline covers Pro weekly and
   GLM credit windows; if the feed is Max-5h-only, §1 copy must say
   so per-account — never imply uniform coverage.

## 31. Settings Inventory (single home for every new knob)

New features die when their settings scatter. Every control above
lives in exactly one place; the table below is the contract.

| Setting | Home | Default | PRD |
|---|---|---|---|
| Quota warn thresholds (80/90%) | Account card | 80 / 90 | §1 |
| Forecast low-confidence hide | Usage → Display | Show with band | §1 |
| Inbox snooze durations | Inbox header menu | 1h / 1d | §2 |
| Routing suggestion per project | Project panel | On | §3 |
| Worktree default per project | Project panel | Auto (live session) | §4 |
| Retention: archive after days | Settings → Storage | 30, never auto-delete | §5 |
| Budget cap per account ($) | Account card | Off | §6 |
| Notification modes + quiet hours | Settings → Notifications | Digest hourly | §7 |
| Affinity warning | Settings → Launch | On | §8 |
| Block launch at Admission=No | Settings → Launch | Off | §9 |
| Palette recents count | Settings → Keyboard | 8 | §10 |
| Review auto-surface | Inbox menu | On (read-only) | §11 |
| Backup count + schedule | Settings → Backup | 7, daily + on exit | §12 |
| Update channel + auto-check | Settings → Updates | Beta, on | §13 |
| Sync conflict default | Sync wizard | Keep target | §14 |
| Present mode group | Rail group menu | Off | §15 |
| Template directory | Settings → Templates | app-data | §16 |
| Audit retention | Settings → Storage | Follow §5 | §17 |
| Global hotkey chord | Settings → Keyboard | `⌥⌘J` | §19 |
| Fleet notify-me flag | Settings → Fleet | Off | §20 |

Rules: no setting defaults to a state-changing automation;
every default above is passive. Import/export of settings
travels in the §12 bundle (minus machine identity and fleet keys).

## 32. Empty, Loading & Error States (no silent surfaces)

The parent review's F2.1/F2.4 failures generalise: every new
surface ships all five states or it does not ship.

| Surface | Empty (designed) | Loading | Error (recoverable) |
|---|---|---|---|
| Quota rings | "Start a session to measure" | Skeleton ring | "Feed stale 22m — Retry" banner |
| Inbox | "All clear — N running" | Row skeletons | "Watcher paused — Resume" |
| Search | "No matches — try project name" | Progress >50ms | "Index rebuilding (n%)" |
| Diff queue | "No finished work waiting" | Diff skeleton | "Not a git repo" explained |
| Backup list | "First backup at exit" | — | "Backup failed — Open logs" |
| Fleet card | "This Mac only (v0.1)" | — | Spike status, never fake peers |

`window.alert` is banned for all of the above; one banner/toast
system owns errors app-wide, with [Retry] where a retry exists
and a log link where it does not. First render never shows an
empty state before the first fetch resolves.

## 33. Open Questions for the Maintainer

1. F4.1 credential-copy: amend SPEC S8 (option b + guardrail test)
   or change import behaviour? Blocks §8 copy and §23 trust page.
2. Instances vs Sessions: merge now (this file assumes Sessions +
   Accounts in user copy) or fund a real Instances entity?
3. Pricing source: models.dev sync vs bundled table + manual edit —
   who owns freshness when offline for a month?
4. Worktree root: inside managed dir (clean invariant) vs sibling
   of the repo (friendlier to external tooling)? Pick one; §4 and
   §11 both depend on the answer.
5. Pro Pack boundary: which §6/§15/§17 artefacts are paid without
   gating the core juggle? Decide before v0.1-beta, publish it.

*End. Companion deep-dives: [07-personas](deep/07-personas-and-journeys.md#L1) for journey evidence; parent [full review Roadmap](full-competitive-ux-review.md#L901) for milestone mapping.*




# 11 — UX Redesign: Build-Ready Screen Spec (Deep)
> Parent: [full review](full-competitive-ux-review.md#L1) · Prompt: [agent-review-prompt.md](../internal/agent-review-prompt.md#L30) · Contract: [SPEC.md](../SPEC.md#L316) §9 + [ADR-0007](../adr/0007-instance-centric-information-architecture.md#L1). Audience: designer + frontend engineer; every screen is implementable without a follow-up question. Evidence: `file:line` = repo truth, URL = competitor bar. **Keep/Fix/New/Remove** + P0/P1/P2 as in [full review](full-competitive-ux-review.md#L10).

## 0. How to build from this doc
- Order: §3 shell/state → §4.1 Inbox → §4.17 palette → §4.9 Handover → rest. Quota feed (M3) gates §4.10–4.11 data only; layouts + honest empty states ship first.
- **Never render a number the backend did not send.** Overview hardcodes rings `percent={0}` + `Machine OK/Fleet 1` today ([App.tsx](../../ui/src/App.tsx#L461)); redesign replaces each with an explicit empty state.
- Passive invariant on every screen: app prepares/suggests/stages, only an explicit click mutates ([SPEC.md](../SPEC.md#L26)); sole exception is opt-in per-profile crash auto-restart, off by default. No auto-switch/rotate/resume control may be added.

## 1. Design principles
### 1.1 Calm supervision (tower, not cockpit)
1. **Silence is a feature.** Default surface answers “does anything need me?” and otherwise stays quiet — inverting today's count-led Overview ([App.tsx](../../ui/src/App.tsx#L431)). 2. **One queue, one badge, everywhere:** needs-attention count is identical in rail, tray, title. Statuses `WaitingUser/Stuck/QuotaBlocked` exist in domain ([model.rs](../../crates/juggler-core/src/model.rs#L188)) but are never emitted ([full review](full-competitive-ux-review.md#L201)); emitting them is a design dependency. 3. **Every alert carries its clearing action.** 4. **Honesty is visual:** every quota/cost number shows Measured/Computed/Estimated as text ([SPEC.md](../SPEC.md#L254)). 5. **Source always survives:** handover copies, never moves ([SPEC.md](../SPEC.md#L287)); destructive controls state what remains.
### 1.2 Glanceable (3-second rule)
Status readable in ≤3 s: icon + word + colour, never colour alone. Today status is a colour dot ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L419)) and `faint #5b6474` at 10.5 px ([styles.css](../../ui/src/styles.css#L24)) — both fail (see §2.4). Numbers tabular (`tabular-nums`, [primitives.tsx](../../ui/src/components/primitives.tsx#L156)); times relative + absolute on hover/focus (`resets 17:00 · in 2h 14m`). Max 2 lines/rail row, 5 columns/table at default width; overflow → L3.
### 1.3 Keyboard-first
Every daily action ≤3 keystrokes from anywhere ([full review](full-competitive-ux-review.md#L855)); today only `⌘B/⌘N` exist ([App.tsx](../../ui/src/App.tsx#L205)). Focus always visible/logical/returned; terminal focus explicit (`⌘.` enters, `Esc` returns to chrome). Shortcuts never fire while typing except `Esc/⌘K/⌘Enter/⌘.`. No traps except modal dialogs (which must trap — today they do not, §2.5).
### 1.4 Progressive disclosure
L1 glance (tray/rail) → L2 triage (inbox row/panel header: why + forecast + one action) → L3 detail (history, pricing, lineage). Daily loop never requires L3; fixes today's inversion where L1 hides behind `ComingSoon` ([App.tsx](../../ui/src/App.tsx#L338)). Full rules in §3.4.

## 2. Design-system gaps (build before screens)
Seed: `@theme` tokens ([styles.css](../../ui/src/styles.css#L12)), `Button/Card/Chip/StatCard/QuotaRing` ([primitives.tsx](../../ui/src/components/primitives.tsx#L31)), 8 icons ([icons.tsx](../../ui/src/components/icons.tsx#L22)), form/modal ([form.tsx](../../ui/src/components/form.tsx#L54)). A good seed, not a system.
### 2.1 Tokens (keep names, add)
| Group | Today | Add |
|---|---|---|
| Colour | Dark only ([styles.css:14](../../ui/src/styles.css#L14)) | `[data-theme=light]` mirror; soft-pair text ≥4.5:1 both themes |
| Status | Dots only | `--color-status-{waiting,stuck,quota,review}` + icon+label (§2.4) |
| Type | Sans/mono ([styles.css:42](../../ui/src/styles.css#L42)) | 11/12/13/15/18/24 px; `faint` banned <11.5 px |
| Space | `--radius-card` ([styles.css:49](../../ui/src/styles.css#L49)) | 4-pt scale; radii 6/10/14/full; row heights 32/40 |
| Motion | Ease + breathe ([styles.css:52](../../ui/src/styles.css#L52)) | 100/150/250 ms; reduced-motion kills breathe/glow |
| Elevation | Card + glows ([styles.css:55](../../ui/src/styles.css#L55)) | Popover/dialog shadows; glow = live/primary only |
### 2.2 Density / themes / motion / icons
**Density:** Cozy default / Compact (rail 32 px, table 36 px, terminal stays 12.5 px [TerminalView.tsx](../../ui/src/components/TerminalView.tsx#L41)); hit targets ≥28 px. **Themes:** dark default, light first-class (not inverted); `prefers-color-scheme` on first run, override in Settings. **Motion:** 100–250 ms ease-out, 100 ms panel cross-fade; live-dot breathe static under reduced motion. **Icons:** keep 24-grid/1.75-stroke ([icons.tsx](../../ui/src/components/icons.tsx#L8)); add `inbox, search, bell, gauge, coins, branch, diff, archive, play, handover, queue, shield, keyboard, sun/moon`; icon-only banned in primary flows; per-harness glyphs replace single Claude spark ([Profiles.tsx](../../ui/src/screens/Profiles.tsx#L264)) before adapter #2.
### 2.3 New primitives; deletions
Add `Banner, Toast (6s), Skeleton, EmptyState, StatusChip (all 9 statuses, [model.rs:188](../../crates/juggler-core/src/model.rs#L188)), QuotaMeter (bar+%+tier+reset; replaces ring-only [primitives.tsx:165](../../ui/src/components/primitives.tsx#L165)), ConfirmDialog, WizardShell, DataTable, Kbd`. Delete `window.alert` paths ([App.tsx:371](../../ui/src/App.tsx#L371), [Profiles.tsx:367](../../ui/src/screens/Profiles.tsx#L367)) + dead duplicate modal ([WorkPanels.tsx:354](../../ui/src/screens/WorkPanels.tsx#L354)).
### 2.4 Status encoding + contrast (normative)
Text ≥4.5:1, large/icons ≥3:1, both themes; retire `faint` for body text. Map (icon+word always): Running `●`, WaitingUser `◐ Needs you`, Stuck `▲ May be stuck?` (always `?` — heuristic, [SPEC.md:220](../SPEC.md#L220)), QuotaBlocked `⛔ Quota`, Review `◆ Ready to review`, Finished `✓ Done`, Failed `✕ Failed`, Stopped `■ Stopped`, HandedOver `↗ Handed over`.
### 2.5 Dialog / focus / live-region contract
Every modal/wizard: `role=dialog aria-modal`, labelled close (today bare `✕`, [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L130)), trapped focus, `Esc` closes (except mid-handover-copy), focus returns to invoker. One polite `aria-live` for results, `role=alert` + Banner for errors. Tabs = full ARIA pattern (arrows + `tabpanel`); today rail lacks it ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L119)). Splitter keyboard-operable (`←/→ ±8px, Home/End`). Keep global `:focus-visible` ring ([styles.css:76](../../ui/src/styles.css#L76)).

## 3. New IA & navigation model
ADR-0007's two-mode split is right ([ADR-0007](../adr/0007-instance-centric-information-architecture.md#L1)); contents are not — Instances = Sessions filtered through same rows ([full review](full-competitive-ux-review.md#L171)).
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
- **Rail (new order):** `Needs attention (badge) → Projects → Sessions → Accounts`. **Remove** Instances tab until resource manager gives it distinct content (CPU/RAM/tokens per [SPEC.md:331](../SPEC.md#L331)); reintroduce as resource view, not filtered list.
- **Header:** `Overview · Accounts · Usage · Cost · ⚙ Settings`. **Remove** Fleet until M6; `ComingSoon` nav ([App.tsx:338](../../ui/src/App.tsx#L338)) trains users to stop exploring. `+ New ▾`: Session / Account / Import machine… (disabled with reason pre-M6).
- **Selection:** single selection (project|session|account|inbox) + global override, extending today's model ([App.tsx:53](../../ui/src/App.tsx#L53)); Search/palette/tray all land here. Below 1100 px rail auto-collapses to icons+badges (window min 1024×680, [tauri.conf.json](../../crates/juggler-app/tauri.conf.json#L1)); terminal stacks first, min-height 240 px.
- **Global state:** `loading|ready|degraded(reason)|error(retry)`; `refresh()` must surface failure, not swallow it ([App.tsx:101](../../ui/src/App.tsx#L101)). Skeletons until first fetch (never flash empty); degraded = Banner `Showing last known state from 14:32 — Retry`; `Stale · 12m` chips; offline disables launch/handover with reason, never hides data. Every screen inherits `empty/loading/error/offline/stale`.
- **Keys (also Settings → Shortcuts):** `⌘K` palette · `⌘N` new · `⌘B` rail · `⌘1–4` rail sections · `⌘⇧H` handover · `⌘,` settings · `⌘.` terminal focus · `g then i/p/s/a` go-to · `?` help · `Esc` close/back. Bench: Claude Squad single-key triage ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)) — match it, never copy its auto-accept daemon.
- **Disclosure rules:** L1 count+worst status only; L2 why+forecast+one action; L3 history/pricing/lineage/raw paths; no modal except wizards + destructive confirms; advanced fields behind `▸ Details`, closed by default, remembered.

## 4. Screen-by-screen spec
Per screen: **Purpose · Entry→Exit · Elements** (`→ cmd` via [bridge.ts](../../ui/src/lib/bridge.ts#L1)) · **States · Keys · A11y · Copy** (verbatim). Wireframes = layout truth at 1360×860 ([tauri.conf.json](../../crates/juggler-app/tauri.conf.json#L1)).

### 4.1 Home / Needs-attention (Inbox) — New, P0, default landing
- **Purpose:** “which session needs me now?” in one glance; replaces `WorkEmpty`/`ReadyHome` as resting state (sign-in gate folds in as rows, §4.16). Sources: WaitingUser, Stuck?, QuotaBlocked, crashed, review-ready, signed-out ([full review:765](full-competitive-ux-review.md#L765)). **Entry→Exit:** launch (badge>0) · rail · tray · `⌘1`/`g i` → row/action jumps to Session/Project/Account; clearing returns here.
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
- **Purpose:** juggling anchor — every directory, who is on it, which account ([ADR-0007](../adr/0007-instance-centric-information-architecture.md#L1)); today rows show name+counts only ([WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L238)). **Entry→Exit:** rail/`⌘2`/`g p` → row §4.3; `⌘N` with project preselected → §4.8.
- **Elements:** `Filter projects…` (`/` focuses) · sort `Recent ▾` (Recent/Name/Most sessions) · rows: icon, name, `⎇ main · ●2 running|idle`, account dots (≤3+`+2`), `last active 12m` · focus actions `▶ New session`, `Open in Finder` · Pinned group (`⌘⇧P`) · footer counts. **States:** empty keeps “Launch a session in any directory — it becomes a project here.” + `Choose folder…` · skeletons · §3 states · no-results `No projects match “xyz”. [Clear]`. **Keys:** type-ahead, `↑↓ Enter`. **A11y:** full row `aria-label` (“payments-api, main, 2 running, active 12m ago”); state in text. **Copy:** `On it now`, `Free to juggle`, `2 running · 1 needs you`.

### 4.3 Project detail — Fix, P0
- **Purpose:** who is/was here, launch-with, hand over, history; evolves [WorkPanels.tsx:196](../../ui/src/screens/WorkPanels.tsx#L196). **Entry→Exit:** Projects/palette/inbox → session §4.7, account §4.5, `Hand over…` §4.9; `Esc` back.
- **Elements:** header name (inline rename), `⎇ branch · dirty`, root (mono, copy, `▸ Details`: canonical path/first seen) · per-account `QuotaMeter` + forecast · `On it now` rows (StatusChip, age, tokens, `Open/Hand over…/Stop`) · `Launch with…`: ranked profiles, `★ usual`, admission `OK/Tight/No` ([SPEC.md:237](../SPEC.md#L237)), signed-out reason · `History` timeline (Today/Yesterday/Earlier, lineage links) · L3 `Forget project…` (confirm).
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
- **Elements:** rows: glyph, label, plan chip, login chip (`Logged in/Needs login/Expired?`), mini 5h meter, `n live`, last used · `Add account`, `Import…` · group by harness if >1 · row `⋯`: New profile, Clone…, Sign in/Re-check. **States:** empty [Profiles.tsx:157](../../ui/src/screens/Profiles.tsx#L157) + `Import existing setup` · skeletons · quota-unknown `Quota not available yet` (no fake ring) · signed-out sorts last. **Keys:** `↑↓ Enter, a` add, `/` filter. **A11y/Copy:** never colour-only; `Needs login — Claude's sign-in, ~30 seconds`, `Expired? — re-check`.

### 4.5 Account detail — Fix, P1 (today cards [Profiles.tsx](../../ui/src/screens/Profiles.tsx#L174))
- **Purpose:** one login's health/quota/profiles/history; wrong-tier/expiry surprises die here (F1.4). **Entry→Exit:** Accounts/Project/palette → Launch §4.7, Sign-in terminal, Usage §4.10.
- **Elements:** header glyph, label (rename), harness, plan chip + `Change plan…`, login chip + `Sign in/Re-check` (→ `terminal_spawn_login` [bridge.ts:378](../../ui/src/lib/bridge.ts#L378)) · quota card: 5h+weekly meters, tier, reset countdowns, `exhausts ~15:40 at current burn` · health card with honest unknowns (`Tier: Max 20× (you set) · Vendor reports: — · Expires: unknown`; extends metadata-only `login_state`, never credential contents) · profiles table (name/model/default dir/live, `Launch/⋯ Edit·Clone·Delete`; Delete = ConfirmDialog, today one-click [Profiles.tsx:239](../../ui/src/screens/Profiles.tsx#L239)) · recent sessions (10, `View all →`) · `▸ Details`: state dir, created, storage.
```
┌─ ◈ Work Max [Max 20×▾] ● Logged in ─── [Re-check][⋯] ────┐
│ Claude Code · 3 profiles · 2 live · used 4m ago          │
│ 5h ▓▓▓▓▓░ 78% Measured·resets 17:00  7d ▓▓░░ 31%         │
│ PROFILES Deep work·opus [Launch]  SESSIONS ●/⛔ [All →]   │
└──────────────────────────────────────────────────────────┘
```
- **States:** signed-out quota → `Sign in to see quota — Juggler never sees your credentials. [Sign in]` · skeletons · stale timestamped · delete blocked live (`2 live sessions use this profile — stop them first.`). **Keys:** `l` launch, `⇧L` picker, `u` Usage. **A11y:** meter text twins; plan `listbox`; copy never implies vendor-verified tier: `Plan (you set this — vendor doesn't report it yet)`.

### 4.6 Sessions (list) — Fix, P0
- **Purpose:** every conversation findable/resumable; fixes 15-row cap, no search ([WorkflowRail.tsx:377](../../ui/src/components/WorkflowRail.tsx#L377)). **Entry→Exit:** rail/`⌘3`/`g s` → row §4.7; filters persist; archived behind toggle.
- **Elements:** search `Search sessions, projects, transcripts…` (§4.12) · chips `All/Live/Needs you/Done/Archived` · project+account filters · sort (Recent/Waiting/Tokens) · `DataTable`: StatusChip, Label (editable), Project, Account, Age/Waiting, Tokens, `⎇ Branch`, `⋯` (Open, Resume/Relaunch, Hand over…, Archive, Export pack…) · bulk Archive/Export · `Live now` pinned group.
```
┌─ Sessions [Search…] [All][Live][Needs you][Done] Sort▾ ──┐
│ ● billing-migration payments-api Work Max 24m 182k [⋯]  │
│ ✓ api-cleanup       payments-api Personal  2h   91k [⋯]  │
│ LIVE NOW pinned · ARCHIVED behind toggle                 │
└──────────────────────────────────────────────────────────┘
```
- **States:** empty `No sessions yet — start one below. [New session]` · no-results + clear · skeleton table · ended always `Resume`+`Relaunch` (missing today F2.3; Resume = T1 `claude --resume` [SPEC.md:271](../SPEC.md#L271)) · archived empty `Nothing archived. Sessions archive after 30 days (Settings).` **Keys:** `/ ↑↓ Enter, r` resume, `h` handover, `e` archive, `x` select. **A11y:** grid roles, Tab-reachable actions, status text first column. **Copy:** `Resume` (same account, lossless) vs `Relaunch` (fresh, same profile+folder), one-line tooltips, never conflated.

### 4.7 Session + terminal — Fix, P0 ([WorkPanels.tsx:44](../../ui/src/screens/WorkPanels.tsx#L44) + [TerminalView.tsx](../../ui/src/components/TerminalView.tsx#L29))
- **Purpose:** watch/answer/stop/hand over without leaving; terminal *is* panel (terminal-kind), desktop-kind shows status + Focus-window, no fake terminal. **Entry→Exit:** Sessions/Inbox/Project/Search → rail/`Esc` back; scrollback preserved (5 000 lines [TerminalView.tsx:14](../../ui/src/components/TerminalView.tsx#L14)).
- **Elements:** header StatusChip, label (rename), kind, project link, account, age, tokens, quota-impact · actions `Stop` (live; ConfirmDialog if running: `Stop “billing-migration”? Transcript is kept — you can Resume later. [Keep running][Stop]`), `Hand over…`, `Resume/Relaunch` (ended), `⋯` (Open project, Copy ID, Export pack…, Archive) · xterm (`⌘F` find, `⋯` Copy/Clear view, `⌘. to type` hint) · below-strip L2: lineage (`Handed over from sess-42 · Work Max ↗`), branch/worktree dirty warning, `▸ Details` (native ID, transcript path, PID, RSS/CPU when available) · login terminals reuse shell + guidance card ([WorkPanels.tsx:128](../../ui/src/screens/WorkPanels.tsx#L128)).
```
┌─ ◐ Needs you billing-migration·payments-api·Work Max ────┐
│ 24m·182k tok·5h ~9%        [Hand over…][Stop][⋯]         │
│ ┌ terminal ──────────────────────────────── ⌘F find ─┐   │
│ │ ❯ Allow edit to src/billing.ts? (y/n)              │   │
│ └────────────────────────────────────────────────────┘   │
│ ↗ from sess-42·Personal  ⎇ juggler/billing               │
└──────────────────────────────────────────────────────────┘
```
- **States:** starting (`Starting…`) · live · waiting (pulse once, then static amber) · ended (`Finished · scrollback replayed`) · failed (`Failed (exit 1) — [Resume][View log]`) · detached read-only + `Reconnecting…` · desktop empty honest placeholder. **Keys:** `⌘.` focus, `⌘F`, `⌘⇧H`, `⌘⌫` stop+confirm, `Esc`. **A11y:** focus order header→terminal→strip; offscreen live summary on status change only (`…needs your reply: Allow edit…`; canvas never read raw). **Copy:** handover promise moves to wizard with tier (§4.9), never bare (today [WorkPanels.tsx:181](../../ui/src/screens/WorkPanels.tsx#L181)).

### 4.8 New Session wizard — Fix, P0 (today [NewSession.tsx](../../ui/src/screens/NewSession.tsx#L18))
- **Purpose:** folder + account with forecast *before* committing; zero typed paths kept (recents+picker+drop [NewSession.tsx:49](../../ui/src/screens/NewSession.tsx#L49)). **Entry→Exit:** `⌘N`/New/drop/Project Start → success §4.7; `Esc` returns.
- **Elements:** 1 *Where:* recent cards (name/⎇/last active/prior count), `Choose folder…` (→ [bridge.ts:468](../../ui/src/lib/bridge.ts#L468)), drop zone, worktree toggle + `juggler/<profile>/<short>` preview + same-branch warning (F1.1). 2 *Who:* ranked profiles (meter, `~40m left`, `★ usual`, admission, signed-out + inline Sign in) + suggest-only banner `Personal has 80% headroom and usually runs this project. [Use Personal][Keep Work Max]`. 3 *Review:* folder/account/model/worktree/admission + `Start session` (→ [bridge.ts:282](../../ui/src/lib/bridge.ts#L282)).
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
- **Purpose:** continue at quota wall without re-explaining, fidelity stated *before* click (F3.5); today bare buttons ([WorkPanels.tsx:139](../../ui/src/screens/WorkPanels.tsx#L139)) → `session_handover` ([bridge.ts:460](../../ui/src/lib/bridge.ts#L460)). **Entry→Exit:** `Hand over…`/Inbox/`⌘⇧H` → opens target §4.7 with lineage; source stays resumable; return offered inline.
- **Elements:** 1 *Target:* ranked accounts (headroom/forecast/`★ usual`/signed-out disabled), tier per target T1–T4 ([SPEC.md:269](../SPEC.md#L269)). 2 *Fidelity:* tier badge + carry list + redaction preview for pack/export (F1.6); honest v0 label until sibling-file test passes: `Transcript-only (experimental)`. 3 *Confirm:* source→target, freeze note, single `Hand over to Personal`; progress Freeze→Copy→Resume; success + `Queue return at reset (17:00)` (passive timer; resume still one click, F1.5).
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
- **Purpose:** burn-down/resets/forecasts; the juggling screenshot (WOW-b); engine + honesty ([SPEC.md:254](../SPEC.md#L254)). **Entry→Exit:** header/Account/tray → cards §4.5, rows §4.7.
- **Elements:** account cards (5h+weekly meters, tier, countdowns, sparkline) · timeline (uPlot [SPEC.md:349](../SPEC.md#L349); `5h/7d/30d`, series toggles) · thresholds link (Settings) · `Export CSV` · forecast `At current burn: Work Max exhausts ~15:40 · Personal lasts the window`.
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
- **Purpose:** finished sessions awaiting a look (diff stat + pack preview). Bar: Vibe inline-comment-to-agent + Conductor Checks ([github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban), [conductor.build/docs](https://www.conductor.build/docs/concepts/workflow)). Demand inferred ([full review:791](full-competitive-ux-review.md#L791)): ship read-only v1; comment-to-agent only after dogfood. **Entry→Exit:** Inbox Review / Sessions Done → Continue/Hand over/Archive.
- **Elements:** rows (session, project, `⎇`, `+n/−m files k`, age, account) · detail: read-only diff + file list, `PACK.md` preview ([SPEC.md:296](../SPEC.md#L296)), transcript tail · `Continue session, Hand over…, Archive, Export pack…` · bulk-archive >7d.
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

### 4.15 Settings (all tabs) — New, P1 (specced [SPEC.md:353](../SPEC.md#L353), absent in UI)
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
  *General:* Appearance, Density, Reduce motion, Startup view, update channel Stable/Beta + `Check for updates…` ([SPEC.md:357](../SPEC.md#L357)). *Notifications:* per-event interrupts, waiting threshold (5 min), quota 80%/95%, digest time, per-project mutes, `Send test`. *Supervisor:* stuck thresholds ([SPEC.md:220](../SPEC.md#L220)), per-profile crash auto-restart (off, same-profile only), RAM/swap/disk + admission margin. *Usage & Cost:* plan caps (→§4.5), pricing sync, soft budget, CSV defaults, retention `Archive 30d · Delete never/after…` (typed confirm, F1.3). *Data & Privacy:* sizes (DB/accounts/packs/logs), `Back up/Restore…/Export diagnostics (redacted)` (F1.7), telemetry Off ([SPEC.md:23](../SPEC.md#L23)), paths, Reveal. *Shortcuts:* §3 table, rebind + conflict detection, reset, hotkey toggle. *Advanced:* harness versions/verified ranges ([SPEC.md:184](../SPEC.md#L184), read-only), harness-update notify-only, logs, plain-language guardrails.
- **States/A11y:** skeleton pane · save-error Banner (value reverts, never silent) · switches with state in name. **Copy:** plain units (`Warn when a session waits longer than 5 minutes`).

### 4.16 Onboarding v2 — Fix, P1 (today wizard [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L40), best-built surface — protect it)
- **Purpose:** zero-knowledge first run: scan → import/fresh → vendor login → summary ([SPEC.md:409](../SPEC.md#L409), [ADR-0006](../adr/0006-smart-onboarding-scan-and-import.md#L1)). Auto-opens at zero accounts ([App.tsx:92](../../ui/src/App.tsx#L92)). Keep interaction model; v2 fixes a11y + funnel leaks (§5), adds harness step for adapter #2.
- **Entry→Exit:** first run / Accounts `Add account` / Inbox sign-in row → Done lands on §4.1 (or first-project prompt if none); dismiss = `I'll set up later` (not bare ✕) with resumable `Finish setup (1 left)` inbox row. ReadyHome gate becomes sticky per profile (F2.7): skip persists unless login unknown/expired.
- **Elements:** `WizardShell` steps `Detect → Identity → Login → Summary` (+ `Harness` first when >1 harness): Detect auto-scan (metadata-only) with found-setup cards (path/origin/signed-in/bytes), `None of these — add fresh`; Identity plain-words name + plan (kept [OnboardingWizard.tsx:20](../../ui/src/screens/OnboardingWizard.tsx#L20)); Login vendor flow (embedded terminal primary, Open web, Copy command, Re-check); Summary check-list (created/imported counts, `original not modified`, next action `Start first session`).
```
┌─ Add a Claude account ●─●─○─○ ───────── [I'll set up later]┐
│ First, let's look at what you already have.              │
│ (•) ~/.claude · signed in · 48 MB  [Import this →]       │
│ ( ) None of these — fresh account                        │
│ Scanning is metadata-only; originals are never touched.  │
└──────────────────────────────────────────────────────────┘
```
- **States:** scanning skeleton (kept spinner row) · none-found fresh path · import >2 GiB ceiling explained pre-click · login pending `Not signed in yet — finish in the terminal, then Re-check.` · error inline per step (never blocks Back) · preview-mode labelled. **Keys:** full §2.5 wizard (labelled close, trap, return-focus). **A11y:** progress `Step 1 of 4: Detect`; scan results as radio group. **Copy:** keep `AI Juggler never sees your credentials.`, `Your original … was not modified`; add time hint `Takes about 2 minutes`.

### 4.17 Tray + global hotkey + command palette — New, P1 (tray specced [SPEC.md:352](../SPEC.md#L352), absent in UI)
- **Tray (menu-bar):** first line = inbox summary `3 need you · Work Max 78% · reset 17:00` (L1, §1.4); sections: Needs-attention items (click → window + owner), Accounts (label · 5h% · `New session here`), `New session…`, `Open AI Juggler`, `Settings…, Quit`. Icon badge = unsnoozed count; icon state also in shape (dot/bang), not colour alone. Bench: cc-switch tray switcher ([github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)) — match glanceability, keep suggest-only (no one-click account switch of running work). **States:** no accounts `Set up an account…` · backend down `Reopen AI Juggler` · quota unknown `Quota —` (dash, never 0%).
```
Tray ▾  3 need you · Work Max 78% · reset 17:00
  ◐ billing-migration (24m)…  ⛔ api-refactor…
  Accounts: Work Max 78% [New] · Personal 22% [New]
  [New session…] [Open] [Settings…] [Quit]
```
- **Global hotkey:** default `⌥Space` (rebindable, Settings → Shortcuts; off if conflict) summons palette over any app (Tauri global-shortcut; capability must be added — today only core+dialog [default.json](../../crates/juggler-app/capabilities/default.json#L1)). Single press = palette; with window focused = `⌘K` equivalent. Never steals focus from a terminal mid-typing without visual summon animation (suppressed under reduced motion).
- **Palette (`⌘K`):** input + grouped results (Actions / Sessions / Projects / Accounts), recents first, fuzzy over titles + transcript titles; inline actions (`Enter` open, `⌘Enter` primary action, `→` preview). Actions include `New session, Hand over…, Resume, Go to…, Settings…`. Footer: `↑↓ navigate · Enter open · ⌘Enter act · Esc close` + `See all results →` (§4.12). **States:** empty = recents + top actions; no-results `No matches — [Search transcripts for “…”]`; offline = local titles only + note. **A11y:** combobox pattern (`aria-expanded/activedescendant`), labelled icon actions (T3/Opcode bar: per-thread surfaces, [t3.codes](https://t3.codes/), [github.com/winfunc/opcode](https://github.com/winfunc/opcode) — palette must feel as instant).
```
┌─ ⌘K ─────────────────────────────────────────────────────┐
│ > bill_______________________________________________  │
│ ACT  Hand over billing-migration…  SESSION billing-… ◐   │
│ Enter open · ⌘Enter hand over · See all results →        │
└──────────────────────────────────────────────────────────┘
```

## 5. Onboarding funnel redesign (targets are hypotheses — instrument, don't assume)
Funnel stages (event names for analytics; local-only counts, telemetry Off by default [SPEC.md:23](../SPEC.md#L23) — show counts in Settings → Data, never auto-send):
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
\* Targets = first-beta goals to validate with 20-user dogfood, not benchmarks; miss → interview, don't add steps. Instrument drop-off per step + time-in-step; any step < target for 2 cohorts gets a usability fix before new features. Protect what works: freeze wizard interaction model + add first UI regression tests ([full review](full-competitive-ux-review.md#L213)).
- **Empty-machine path:** no Claude found → Detect explains install check + `Show me how to get Claude Code` (vendor link, never bundled installer), fresh path still completable later.
- **Abandon/resume:** dismissed wizard leaves `Finish setup` inbox row + Accounts banner; state (chosen setup, name) persists locally so resume lands on the same step.
- **Add-account (returning user):** same wizard, Detect pre-collapsed (`2 accounts already set up`), Identity prefills next-free label suggestion; success returns to Accounts, not Inbox.

## 6. Cross-screen flows (build these as tests)
1. **Morning triage:** tray `3 need you` → Inbox (oldest first) → `Reply…` focuses terminal (`⌘.` state) → reply clears row live → badge decrements in rail+tray+title together. Fail state: stale badge = P0 bug.
2. **Quota wall:** session hits limit → QuotaBlocked row + system alert (threshold) → Handover wizard (target ranked, fidelity shown) → confirm → target session opens, source shows lineage + `Queue return 17:00` → reset notification → one-click return. No step may auto-execute.
3. **Launch with forecast:** Project `Start` → New Session step 2 shows Tight + workaround → `Start anyway` launches; admission never hard-blocks except backend-down.
4. **Find anything:** `⌘K` “webhook” → transcript hit → `⌘Enter` Resumes owner session at context (scrollback top + find pre-filled). Offline degrades to titles with note.
5. **Review loop (P2):** Finished → Review queue → diff stat → `Continue session` resumes with terminal focused; Archive removes from queue, never deletes transcript.
Each flow gets a Playwright/webview test per [SPEC.md:442](../SPEC.md#L442) before the screen is “done”; keyboard-only run of flows 1–4 is the a11y gate (no mouse, screen-reader labels on).

## 7. Build order, acceptance, and open decisions
- **Phase A (P0 shell):** state model + Banner/Toast/Skeleton, rail reorder, Inbox v1 (needs status emission), palette, Session Resume/Relaunch. Exit: flows 1+4 keyboard-only pass; no `window.alert` remains (grep gate).
- **Phase B (P0 loop):** New Session + Handover wizards, Project/Account detail quota strips. Exit: flow 2 dogfood on real Claude (also F3.4 fidelity proof); honest tier label until then.
- **Phase C (P1):** Usage/Cost, Notifications, Settings (all tabs), Tray/hotkey, Onboarding v2 + funnel events. Exit: §5 targets instrumented; light theme + Compact pass contrast audit (both themes, automated axe + manual VoiceOver pass — code-read alone is insufficient, [full review](full-competitive-ux-review.md#L1014)).
- **Phase D (P2):** Review queue read-only, Instances resource view (only with real CPU/RAM data), Search transcript index. Exit: dogfood validates review demand before comment-to-agent.
- **Per-screen done =** all five states rendered, shortcuts work with terminal focused/unfocused, focus returns correctly, copy verbatim, skeleton (not spinner text — today `Loading…` [Profiles.tsx:75](../../ui/src/screens/Profiles.tsx#L75)), and L1/L2/L3 disclosure checked. Truncation always has tooltip + full text in `aria-label` (rail today truncates silently [WorkflowRail.tsx:272](../../ui/src/components/WorkflowRail.tsx#L272)).
- **Open decisions (need owner call, defaults recommended):** (1) Instances timing — default: removed until resource data (above). (2) Global hotkey default `⌥Space` vs `⌘⇧K` — default `⌥Space`, rebindable. (3) Review comment-to-agent — default: deferred post-dogfood. (4) Light theme in beta — default: ship, it's a Settings toggle + token mirror, not new screens. (5) Fleet in nav — default: hidden until M6 pairing exists ([SPEC.md:387](../SPEC.md#L387)).
- **What not to build:** kanban board (import Vibe/Squad users, not the metaphor, [full review](full-competitive-ux-review.md#L507)), auto-rotation (claude-swap does it, [github.com/realiti4/claude-swap](https://github.com/realiti4/claude-swap) — Juggler stays suggest-only), checkpoint timeline (Opcode's strength, [github.com/winfunc/opcode](https://github.com/winfunc/opcode) — lineage covers Juggler's job), mobile companion (T3's, [t3.codes](https://t3.codes/) — out of scope macOS-first [SPEC.md:18](../SPEC.md#L18)).

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
### 11.1 Overview (global) — Fix, P1 (today [App.tsx](../../ui/src/App.tsx#L431))
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
- Ship only when per-instance RSS/CPU/token attribution is real ([SPEC.md:331](../SPEC.md#L331), supervisor footprints [SPEC.md:231](../SPEC.md#L231)). Until then the rail slot stays Accounts (§4.4) — do not ship a filtered Sessions clone.
- When shipped: table (Instance, Kind Terminal/Desktop, Account, Project, RSS, CPU%, Tokens, Age) + `Focus window/Open terminal, Stop` + overload banners (swap thrash, RSS anomaly [SPEC.md:239](../SPEC.md#L239)). Desktop-kind rows have no terminal action.
```
┌─ Instances (future) ─────────────────────────────────────┐
│ billing-migration Term·Work Max·payments 2.9GB 12% [Open]│
│ (slot hidden until RSS/CPU attribution is real)          │
└──────────────────────────────────────────────────────────┘
```
- **States/Copy:** empty `Nothing running.`; overload `Memory pressure — sessions may stall. [See largest]`.

### 11.3 Fleet placeholders & empty-nav policy — Remove, P0 honesty
- No `ComingSoon` screens. Pre-M6, Fleet appears only as disabled `+ New ▾ → Import machine…` with tooltip `Fleet arrives in M6 — see Roadmap`, and in Settings → Advanced as roadmap text. When M6 lands: machines as filters (not a separate board), pairing wizard (code/QR [SPEC.md:387](../SPEC.md#L387)), cross-machine handover reuses §4.9 with T2 path warning (same binary version + same repo path required, stated pre-confirm).
- Policy for all future nav: a destination ships only with (a) real data or honest empty, (b) one primary action, (c) all five states. Otherwise it does not enter nav.

## 12. Implementation notes for the frontend engineer
- **Data needs (new/changed bridge calls; names follow [bridge.ts](../../ui/src/lib/bridge.ts#L282) style):** `inboxList()` (waiting-sorted, snooze-aware), `quotaList()` (per account/window + tier + forecast), `costSummary(range)`, `searchQuery(q, scope)`, `notificationsList()`, `settingsGet/Patch`, `handoverPreview(session, target)` (tier + carry list before execute). Existing `sessionsList/projectsList/accountsList/profilesList` stay; add `last_activity/quota` fields to generated types via ts-rs, never hand-edit types.
- **Events:** extend `session://updated` payload with inbox delta (or add `inbox://updated`); badge counts derive from one selector so rail/tray/title cannot diverge (flow 1 gate).
- **Performance:** virtualize Sessions/Projects lists >100 rows; terminal mount must not block panel paint (lazy xterm chunk already separate [TerminalView.tsx](../../ui/src/components/TerminalView.tsx#L1)); quota timers tick locally (1/min render, no refetch storm).
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
Parent [full review](full-competitive-ux-review.md#L802) §UX named direction (inbox, forecast, palette, disclosure, a11y list). This file turns each into: named controls + backend calls, five rendered states per screen (§8), focus order + live-region behaviour (§9), verbatim copy (§10), funnel targets with instrumentation (§5), flow-level acceptance tests (§6), and explicit non-goals (§7). Anything still marked `future/deferred/inferred` is a deliberate omission with a trigger to build it, not an oversight.

## Appendix — traceability (claim → source)
Repo: shell/state [App.tsx](../../ui/src/App.tsx#L1), rail [WorkflowRail.tsx](../../ui/src/components/WorkflowRail.tsx#L1), panels [WorkPanels.tsx](../../ui/src/screens/WorkPanels.tsx#L1), new-session [NewSession.tsx](../../ui/src/screens/NewSession.tsx#L1), accounts [Profiles.tsx](../../ui/src/screens/Profiles.tsx#L1), onboarding [OnboardingWizard.tsx](../../ui/src/screens/OnboardingWizard.tsx#L1), ready-home [ReadyHome.tsx](../../ui/src/screens/ReadyHome.tsx#L1), terminal [TerminalView.tsx](../../ui/src/components/TerminalView.tsx#L1), primitives/icons/form/styles above, bridge [bridge.ts](../../ui/src/lib/bridge.ts#L1), domain [model.rs](../../crates/juggler-core/src/model.rs#L1), spec/ADR above. Competitors: [t3.codes](https://t3.codes/), [opcode](https://github.com/winfunc/opcode), [claude-squad](https://github.com/smtg-ai/claude-squad), [conductor](https://www.conductor.build/docs/concepts/workflow), [vibe-kanban](https://github.com/BloopAI/vibe-kanban), [cc-switch](https://github.com/farion1231/cc-switch), [ccusage](https://github.com/ryoppippi/ccusage), [claude-swap](https://github.com/realiti4/claude-swap). Limits: no runtime/screen-reader test was run for this spec; contrast/focus claims need Phase C empirical pass. Unresolved backend gaps (quota feed, status emission, transcript search index, global-shortcut capability) are named per screen — layouts do not depend on inventing their data.
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



# 12 — Roadmap, Metrics, Risks & GTM: A Sequencing Plan That Sells the Juggle

> **Scope:** Revised epics, quality gates, beta programme, measurement,
> pricing/sustainability, go-to-market, risk register, decision log,
> and a 90-day week-by-week plan.
> **Companions:** [full-competitive-ux-review.md](full-competitive-ux-review.md) ·
> [EPICS.md](../plan/EPICS.md) · [QUALITY.md](../plan/QUALITY.md) ·
> [RUNBOOK.md](../plan/RUNBOOK.md) · [gate-M1.md](../qa/gate-M1.md)
> **Date:** 2026-10-13 · **Status:** Proposal — no scope is authorised
> until the maintainer closes the decision log in §10.
> **Citation rule:** Every factual claim about this repo cites
> `file:line`; every competitor claim cites a primary URL. Forward-
> looking targets are labelled **[target]** and are not evidence.

---

## TL;DR — the decision this document asks for

1. **Stop sequencing by harness count; sequence by loop closure.**
   The founding promise is G2/G3 — know before the quota wall and
   never lose momentum ([PROPOSAL.md:32-33](../PROPOSAL.md#L32)) —
   yet quota sits at M3 after M2 ships two adapters
   ([EPICS.md:26-28](../plan/EPICS.md#L26),
   [SPEC.md:452-454](../SPEC.md#L452)). The full review calls this
   inverted ([full-competitive-ux-review.md:15-19](full-competitive-ux-review.md#L15)).
   This plan pulls Claude-Measured quota, stuck/waiting emission,
   and handover-honesty ahead of adapter #2.
2. **v0.1-beta is a Claude-only juggling loop, not a three-harness
   board.** One harness with quota + honest handover + return-on-
   reset beats three harnesses that are quota-blind. ZCode — bundle-
   only and undocumented ([full-competitive-ux-review.md:416-418](full-competitive-ux-review.md#L416))
   — is the last adapter, not the second.
3. **The sunset window is the GTM window.** Vibe Kanban is
   sunsetting after Bloop's 2026-04-10 shutdown and Crystal was
   deprecated Feb 2026 ([full-competitive-ux-review.md:50-52](full-competitive-ux-review.md#L50)).
   A migration path for those users is cheaper than any paid channel
   and expires; it must ship with v0.1-beta, not after 1.0.
4. **No metrics without an event taxonomy the code can actually
   emit.** §7 defines events, properties, and the exact funnel from
   install → 2 accounts → quota wall → handover → return. If an
   event cannot be produced by a Tauri command that exists or is
   scoped in §3, it is marked **[not yet emittable]**.
5. **Stay MIT, stay free, monetise nothing until 1.0 proves
   retention.** The licence is MIT by ADR
   ([docs/adr/0005-mit-license.md](../adr/0005-mit-license.md#L1));
   sustainability options in §8 are sequenced so none of them
   requires relicensing or a cloud account — both would break the
   local-first invariant ([PROPOSAL.md:67](../PROPOSAL.md#L67)).

---

## 0. Baseline: what is actually true today

Do not plan from the milestone table; plan from the gate file and
the changelog. The two disagree, and that disagreement is itself
the first roadmap item.

| Claim | Evidence | Implication |
|---|---|---|
| README says "spec phase / no releases yet" | [README.md:7](../../README.md#L7) | Public face understates the product |
| CHANGELOG records shipped handover v0 + Claude adapter | [CHANGELOG.md:335-341](../../CHANGELOG.md#L335) | Handover exists but is transcript-only |
| Gate-M1 is mechanics-verified, awaiting user confirmation, all boxes unchecked | [gate-M1.md:3-4](../qa/gate-M1.md#L3), [gate-M1.md:54-67](../qa/gate-M1.md#L54) | M2 work starting now violates EPICS process |
| EPICS bars the next milestone without gate sign-off | [EPICS.md:16](../plan/EPICS.md#L16) | Phase 0 is a process fix, not code |
| QUALITY requires the repo to tell the truth | [QUALITY.md:9](../plan/QUALITY.md#L9) | README/CHANGELOG drift is a quality defect |
| Only Claude is registered; Codex/ZCode have zero adapters | [full-competitive-ux-review.md:369-370](full-competitive-ux-review.md#L369) | "Multi-harness" is a type-level claim today |
| `Stuck/WaitingUser/QuotaBlocked` modelled, never emitted | [full-competitive-ux-review.md:30-32](full-competitive-ux-review.md#L30) | The #1 daily question is unanswerable |
| Quota is M3, after M2 adapters | [SPEC.md:452-454](../SPEC.md#L452) | Roadmap inverted vs. G2/G3 |

**Consequence:** Any revised epic that starts with "build adapter
#2" inherits three lies — that M1 is closed, that quota is a
dashboard feature rather than the product, and that handover is
done. Phase 0 exists to retire those lies before new scope lands.

---

## 1. The central sequencing decision: quota vs. adapters

### 1.1 Why this is the decision, not a detail

PROPOSAL G2 is "know before you go" and G3 is "never lose
momentum" ([PROPOSAL.md:32-33](../PROPOSAL.md#L32)). Both require
quota state: the return-on-reset flow needs a reset time
([SPEC.md:288](../SPEC.md#L288) via full review), and the
handover wizard needs target headroom
([EPICS.md:109](../plan/EPICS.md#L109)). Without quota, handover
is a copy operation with no destination advice, the tray has
nothing to show, and the admission advisor (SPEC §5.2) has no
quota input. Adapters, by contrast, multiply surface: each one
imports its vendor's format fragility, and Codex quota is
Estimated with no API while ZCode is bundle-only
([full-competitive-ux-review.md:416-418](full-competitive-ux-review.md#L416)).

### 1.2 Options considered

| Option | What ships first | Loop closed? | Risk |
|---|---|---|---|
| A. Adapters first (current EPICS M2→M3) | Codex + ZCode, then quota | No — 3 blind harnesses | Highest: fragility ×3 before the differentiator exists |
| B. Quota first, Claude only (recommended) | M3-1/M3-2 + stuck + handover-honest, then Codex | Yes, on one harness | Lowest: proves G2/G3 where the data is Measured |
| C. Quota + Codex in parallel | Statusline feed + Codex adapter concurrently | Partially — Codex quota stays Estimated | Splits a tiny team; conformance kit not ready |
| D. All three quotas, no new adapter | Claude Measured + ZCode Computed + Codex Estimated on paper | No — ZCode/Codex have no adapter to feed it | Quota without sessions is a chart, not a juggle |

Option D fails on inspection: M3-3/M3-4 (ZCode credit computation,
Codex estimation, [EPICS.md:92-94](../plan/EPICS.md#L92)) presuppose
adapters that do not exist. Option C fails on staffing (§4): the
conformance kit must land *with* adapter #2, not after #3
([full-competitive-ux-review.md:421-423](full-competitive-ux-review.md#L421)),
and building it concurrently with the quota engine doubles the
review surface per PR against the ≤400-line guideline
([QUALITY.md:26](../plan/QUALITY.md#L26)).

### 1.3 Decision: Option B, with two carve-outs

- **Pull forward:** M3-1 Claude statusline quota feed
  ([EPICS.md:90](../plan/EPICS.md#L90); mechanism at
  [SPEC.md:192](../SPEC.md#L192)), M3-2 usage engine
  ([EPICS.md:91](../plan/EPICS.md#L91)), M2-5 stuck detection
  ([EPICS.md:79](../plan/EPICS.md#L79)), M4-2 lineage
  ([EPICS.md:104](../plan/EPICS.md#L104)).
- **Defer:** M3-3/M3-4 until their adapters exist; ZCode adapter
  to the last pre-1.0 phase (documented surface beats bundle
  archaeology, [full-competitive-ux-review.md:466-468](full-competitive-ux-review.md#L466)).
- **Keep early regardless:** the adapter conformance kit ships
  *with* Codex (Phase 2), because every later adapter — including
  community ones — inherits its fixture corpus.

### 1.4 What "quota first" must not become

Quota-first is not dashboard-first. The Usage screen is a
`ComingSoon` placeholder today
([full-competitive-ux-review.md:193-194](full-competitive-ux-review.md#L193)),
and charting is L3 detail in the redesigned IA. The Phase 1 quota
deliverable is three glanceable surfaces — ring + burn-down +
reset countdown in NewSession, the handover wizard, and the tray
— each carrying its Measured/Computed/Estimated honesty label
([SPEC.md:254-259](../SPEC.md#L254)). A beautiful Usage page
with no launch-time forecast would repeat the original inversion
in miniature.

---

## 2. Revised epics: Phase 0 → 1.0

Each phase lists entry criteria (all must be true before work
starts), scope, exit criteria (all must be evidenced in the gate
report per [QUALITY.md:86-109](../plan/QUALITY.md#L86)), and
explicit non-goals. Phases replace M2–M8 numbering for planning;
EPICS task IDs are retained for traceability.

### Phase 0 — Truth & gates (days, not weeks)

**Entry:** Maintainer available for two decisions (D1, D2 in §10).
**Scope:**
- Fix README status + milestone table ([README.md:7](../../README.md#L7),
  [README.md:61-71](../../README.md#L61)); add a docs-consistency
  check (README status vs. latest CHANGELOG entry) to the gate
  checklist.
- Label handover v0 "transcript-only (experimental)" in UI and
  CHANGELOG ([CHANGELOG.md:335-341](../../CHANGELOG.md#L335));
  downgrade advertised capabilities to `RESUME` only — `TRANSPLANT|
  FORK` are advertised but unimplemented
  ([full-competitive-ux-review.md:243-249](full-competitive-ux-review.md#L243)).
- Decide the credential byte-copy contradiction: import copies
  `.credentials.json` into the managed dir
  ([full-competitive-ux-review.md:320-326](full-competitive-ux-review.md#L320))
  against SPEC S8's "never copied" wording. Recommended: amend
  S8 to the 0700-managed-dir carve-out + guardrail test (§10, D1).
- Close or explicitly waive gate-M1 in the gate file itself,
  naming waived criteria ([gate-M1.md:54-67](../qa/gate-M1.md#L54)).
- Replace fabricated Overview numbers (`percent={0}` rings,
  hardcoded "Machine OK") with honest empty states
  ([full-competitive-ux-review.md:193-199](full-competitive-ux-review.md#L193)).
- App-level `loading|ready|degraded|error` model; delete every
  `window.alert` path ([full-competitive-ux-review.md:160-169](full-competitive-ux-review.md#L160)).

**Exit:** `just verify` green; gate-M1 signed or waived in writing;
no numeric UI element renders invented data; README, CHANGELOG,
and EPICS agree on current state. **Non-goal:** any new feature.

### Phase 1 — v0.1-beta: the Claude juggling loop

**Entry:** Phase 0 exit + gate-M1 disposition recorded.
**Scope (the loop, nothing else):**
- Quota: M3-1 statusline feed + M3-2 usage engine; rings,
  burn-down forecast, reset countdown, honesty labels.
- Needs-attention slice: emit `Stuck/WaitingUser/QuotaBlocked`
  end-to-end (model exists at
  [full-competitive-ux-review.md:201-207](full-competitive-ux-review.md#L201);
  emission choke point is `apply_lifecycle_event`), rail chips,
  inbox v1 sorted by waiting time.
- Handover-honest: use `transcript_ref` not newest-by-mtime,
  freeze source before copy, copy the whole session file set
  (subagents, file-history), verify the copy parses, record
  lineage; move logic into `juggler-handover` (currently a
  doc-only stub, [full-competitive-ux-review.md:391-395](full-competitive-ux-review.md#L391));
  PTY-independent golden-path test so CI exercises it.
- Safety: `guardrails.rs` suite (named in SPEC but missing,
  [full-competitive-ux-review.md:350-357](full-competitive-ux-review.md#L350));
  worktree-on-launch option + same-branch warning; DB
  backup/restore (single `juggler.sqlite` holds lineage + usage).
- Daily UX: ⌘K palette + session search; Resume/Relaunch on
  ended sessions; session archive/search v1; accessibility P1s
  (dialog semantics, ARIA tabs, text status chips, contrast).

**Exit (the dogfood criterion):** On real Claude, with 2 accounts:
launch → quota wall visible *before* exhaustion → handover with
fidelity label → return on reset. Plus: 20-session fake-harness
load test in CI; coverage targets met
([QUALITY.md:63](../plan/QUALITY.md#L63)); signed + notarized
build installs cleanly on a fresh VM (existing M5-2 scope,
[EPICS.md:118](../plan/EPICS.md#L118)). **Non-goals:** Codex,
ZCode, Fleet, review queue, cost ledger UI beyond per-session
tokens.

### Phase 2 — v0.2: second harness + cost (breadth with proof)

**Entry:** v0.1-beta shipped; ≥10 external users completed the dogfood loop (§5); no open P0/P1.
**Scope:** Codex adapter (M2-1/M2-2, [EPICS.md:75-76](../plan/EPICS.md#L75)) **with** the conformance kit (same fixture corpus per adapter — launch/resume/parse/quota-tier; Codex quota = Estimated); cost ledger (tokens→USD via versioned pricing table mirroring cc-switch's models.dev sync, rollups + CSV, every figure tier-labelled; no USD view exists today, [full-competitive-ux-review.md:76-80](full-competitive-ux-review.md#L76)); account health (tier/expiry/affinity warning, extending metadata-only `login_state`); reset-queue (passive timer + notification, resume still one click per ADR-0003); pack redaction; tray quick-switch; scale fixes (byte-offset tailing, shared `sysinfo`, joined queries — bounds at [full-competitive-ux-review.md:368-380](full-competitive-ux-review.md#L368)).
**Exit:** Claude+Codex side-by-side; honesty tiers on every number; 20-session load test on both adapters; a contributor can add a fake adapter from the kit docs alone. **Non-goals:** ZCode, Fleet, auto-rotation.

### Phase 3 — v0.5: third harness + the review surface

**Entry:** v0.2 stable ≥4 weeks; conformance kit proven by Codex.
**Scope:** ZCode adapter (M2-3) behind the kit, with ZCode Computed quota (M3-3) + Codex calibration (M3-4) landing where their adapters exist; profile clone + selective MCP/rules sync; review queue **only if validated** — demand is **[inferred]** ([full-competitive-ux-review.md:502-505](full-competitive-ux-review.md#L502)), so read-only diff first, inline-to-agent after ≥5 unprompted requests; quiet notifications; session replay (cf. [opcode.sh](https://opcode.sh/)); Vibe/Squad config import (§8.3); self-update hardening (bad-signature rejected, [EPICS.md:117](../plan/EPICS.md#L117)).
**Exit:** Three harnesses, one board/inbox; ≥3 real migrants import unaided; update channel proven. Labelled **v0.5** = complete single-machine product; 1.0 reserved for the stability promise. **Non-goals:** Fleet control, Win/Linux packaging.

### Phase 4 — v1.0: fleet-readiness + stability, then fleet

**Entry:** v0.5 retention met (§6.3); security review scheduled.
**Scope:** Fleet link (M6: identity/mDNS/pairing/telemetry/read-only board, [EPICS.md:130-134](../plan/EPICS.md#L130)) on the Phase-1 fleet-ready inbox model; fleet control (M7, originally v0.3 at [EPICS.md:136-145](../plan/EPICS.md#L136)) is **post-1.0** — premature for a tiny team (§3). Stabilisation, perf pass, security review, docs completeness; stable channel opens at 1.0 ([PROPOSAL.md:150](../PROPOSAL.md#L150)).

**Exit:** Full regression matrix green; no open P0/P1; stable
channel live; gate report signed. **Non-goal:** team/org
features — explicitly out of scope by design
([PROPOSAL.md:45](../PROPOSAL.md#L45)).

### 2.1 Dependency graph (what blocks what)

```
Phase 0 ──► gate-M1 disposition ──► Phase 1 (all of it)
Phase 1 quota ──► handover headroom check ──► return-on-reset
Phase 1 status emission ──► inbox ──► tray badge ──► (v1.0) fleet roll-up
Phase 1 handover crate ──► Phase 3 cross-harness replay
Conformance kit (Ph.2) ──► ZCode (Ph.3) ──► any community adapter
Cost ledger (Ph.2) ──► budget alerts ──► Persona-A retention (§7)
Fleet identity (Ph.4) ──► fleet control (post-1.0 only)
```

The critical path is **quota → handover-honesty → return-on-
reset**. Everything else either feeds it or waits for it. If a
week slips, cut from Phase 1's Daily-UX list first — never from
the critical path, and never by skipping the fidelity test.

---

## 3. Staffing & effort realism for a tiny team

**Assumption [target]:** 1 maintainer + occasional contributors +
agent runs under [RUNBOOK.md](../plan/RUNBOOK.md#L1). The
runbook's own loop — lowest open task, ≤400-line PRs, gate stops
for human confirmation ([RUNBOOK.md:23-30](../plan/RUNBOOK.md#L23)) —
is the capacity model; this section prices phases against it.

| Phase | Engineering weeks [target] | Bottleneck | De-scope lever |
|---|---|---|---|
| Phase 0 | 0.5–1 | Maintainer decisions D1/D2 | Waive gate-M1 explicitly instead of closing it |
| Phase 1 | 6–9 | Fidelity test + status emission are serial | Cut ⌘K to session-search only; defer archive UI |
| Phase 2 | 5–7 | Codex format drift; pricing-table upkeep | Ship cost ledger Claude-only, Codex tokens-only |
| Phase 3 | 5–8 | ZCode bundle archaeology | ZCode read-only (sessions+quota, no handover) |
| Phase 4 | 6–10 | Fleet transport + security review | Ship 1.0 single-machine; fleet link as v1.1 |

Rules: one critical-path item in flight (parallel quota+handover PRs collide at `apply_lifecycle_event`); each phase pays its own test debt (Phase 1: `guardrails.rs` + PTY-independent golden path); budget 10–15% **[target]** vendor-drift tax — Claude ships weekly, Codex broke profiles in 0.134.0 ([PROPOSAL.md:132](../PROPOSAL.md#L132)); book UI design checkpoints ([EPICS.md:55](../plan/EPICS.md#L55)) on the critical path or the gate stalls.

## 4. Quality gates per phase

All phases inherit the QUALITY Definition of Done
([QUALITY.md:13-26](../plan/QUALITY.md#L13)) and the gate
definition ([EPICS.md:158-164](../plan/EPICS.md#L158)). The table
adds only phase-specific, falsifiable checks.

| Gate | Automated | Functional (evidenced) | Kill criterion |
|---|---|---|---|
| P0 | `just verify`; docs-consistency check | Gate-M1 signed/waived; no fabricated UI numbers (screenshot audit) | Any UI number without a data source |
| P1 (v0.1-beta) | Guardrails suite; PTY-independent handover test; 20-session load test; coverage ≥85% core ([QUALITY.md:63](../plan/QUALITY.md#L63)) | Real-Claude dogfood: wall→handover→return; quota within tolerance vs. `/usage` across a full window (M3-6 criterion, [EPICS.md:95](../plan/EPICS.md#L95)) | Handover loses subagent/todo state silently |
| P2 (v0.2) | Conformance kit passes both adapters; cost-ledger property tests on window math | Claude+Codex parallel day; CSV totals match fixture sums exactly | Any quota/cost figure without an honesty tier |
| P3 (v0.5) | Bad-signature update rejected; import fixtures for Vibe/Squad configs | ≥3 external migrants complete import unaided | Update path installs an unsigned build |
| P4 (1.0) | Full 3-OS matrix; fuzz with no new crashes since last gate ([QUALITY.md:95](../plan/QUALITY.md#L95)) | Fresh-machine install → 2 parallel accounts < 5 min ([PROPOSAL.md:142](../PROPOSAL.md#L142)) | Any open P0/P1; any guardrail regression |

A gate that fails its kill criterion does not get waived — it
gets a smaller phase. That is the lesson of gate-M1 sitting
unsigned while M2 scope accumulated
([full-competitive-ux-review.md:452-460](full-competitive-ux-review.md#L452)).

---

## 5. Beta & dogfood programme

### 5.1 Who to recruit, in order

1. **Vibe Kanban refugees (highest intent, expiring).** Bloop shut
   down 2026-04-10; Vibe is community-maintained, last push
   2026-04-24 ([full-competitive-ux-review.md:646-653](full-competitive-ux-review.md#L646));
   upstream: [github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban).
   They already run parallel agents in worktrees; Juggler's pitch
   is supervision + quota, not another board.
2. **Crystal / Nimbalyst switchers.** Crystal deprecated Feb 2026
   for commercial Nimbalyst
   ([full-competitive-ux-review.md:655-660](full-competitive-ux-review.md#L655));
   upstream: [github.com/stravu/crystal](https://github.com/stravu/crystal).
   Users who chose an MIT tool once will choose one again.
3. **Claude Squad power users** who hit the quota wall the TUI
   cannot see ([github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad));
   **cc-switch / claude-swap users** who have accounts but no
   session supervision ([github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch),
   [github.com/realiti4/claude-swap](https://github.com/realiti4/claude-swap)).
4. **The maintainer's own 5-account setup** (gate-M1 dogfood
   environment, [gate-M1.md:33-43](../qa/gate-M1.md#L33)) — the
   permanent canary, not a substitute for external users.

### 5.2 Programme mechanics
Cohorts of 5 **[target]** (named build, 30-min pairing call, one task: the dogfood loop) — small cohorts keep the tiny team (§3) from drowning. Recruit in Vibe/Crystal issues/Discussions, r/ClaudeAI, Claude Discord; offer the migration guide (§8.3) first. **Dogfood script:** install → import → 2 accounts parallel → run one to its 5h wall → handover (labelled) → return on reset; any step needing a terminal outside the app is a P1. Label every beta issue with the funnel stage it blocked (§6.3).

### 5.3 What beta must prove before Phase 2 entry

≥10 external users complete the dogfood loop; ≥3 complete a
return-on-reset unprompted; zero reports of writes outside
managed dirs (the PROPOSAL success criterion,
[PROPOSAL.md:145](../PROPOSAL.md#L145)); handover continuity
judged successful in ≥80% of tries
([PROPOSAL.md:144](../PROPOSAL.md#L144)).

---

## 6. Metrics: north star, funnel, retention

### 6.1 North star

**Weekly Juggling Loops Completed (WJLC):** a loop = one user
moves a live session across accounts (handover executed) *and*
the destination session produces output afterwards. It is the
only metric that requires G1+G2+G3 to all work at once. Stars,
downloads, and sessions launched are vanity inputs to it.

- **[target] v0.1-beta exit:** 10 external users × ≥1 loop/week.
- **[target] v0.2:** 50 WJLC/week; **v0.5:** 200; **1.0:** 500.
- Guardrail metric paired with it: **Handover Fidelity Complaint
  Rate** (loops followed within 7d by a fidelity issue) must stay
  <5% **[target]** — WJLC growing while fidelity complaints grow
  faster means the label, not the feature, is working.

### 6.2 Exact event taxonomy

Local-first means telemetry is opt-in only
([PROPOSAL.md:67](../PROPOSAL.md#L67)); every event below is
emitted to the local `usage_events`-style store first and leaves
the machine only if the user opts in. Names are `snake_case`,
past tense, `object_action`. **[not yet emittable]** marks events
whose emitting code is scoped but unbuilt.

| Event | Emitted by | Key properties |
|---|---|---|
| `app_first_launch` | Tauri startup | `os, arch, app_version` |
| `account_imported` | import command | `harness, source_kind (t3/vibe/squad/manual), size_mb_bucket` |
| `account_login_confirmed` | `login_state` probe | `harness, method (vendor_terminal)` — never token data |
| `session_launched` | launch command | `harness, account_id_hash, worktree_bool, admission_verdict` |
| `quota_snapshot` | statusline feed | `harness, tier (measured/computed/estimated), pct_5h, pct_7d, resets_at` |
| `quota_wall_hit` **[not yet emittable]** | `QuotaBlocked` emission (Ph.1) | `harness, pct_at_hit, session_age_min` |
| `attention_raised` **[not yet emittable]** | status emission (Ph.1) | `kind (stuck/waiting/quota/crash), session_id_hash` |
| `attention_cleared` **[not yet emittable]** | inbox action | `kind, wait_min, action (reply/handover/resume)` |
| `handover_prepared` | handover wizard open | `from_tier, to_tier, fidelity_tier, headroom_pct` |
| `handover_executed` | handover command | `fidelity_tier, files_copied, lineage_depth` |
| `handover_returned` | resume-on-reset click | `away_duration_min, delta_sessions` |
| `session_resumed` | resume command | `harness, age_hours, via (board/palette/tray)` |
| `cost_exported` (Ph.2) | CSV export | `rows, range_days` |

Identity rule: `account_id_hash` is a salted local hash; no
email, path, transcript text, or credential-adjacent field is a
property of any event. This follows the metadata-only precedent
of `login_state` ([full-competitive-ux-review.md:323-326](full-competitive-ux-review.md#L323)).

### 6.3 Funnel & retention

**Activation funnel (per install cohort):**
`app_first_launch → account_imported (≥1) → account_login_confirmed
(≥2 accounts) → session_launched (2 parallel) → quota_wall_hit →
handover_executed → handover_returned`. The 2-account step is
the activation cliff: PROPOSAL's own success bar is install →
2 parallel accounts < 5 min ([PROPOSAL.md:142](../PROPOSAL.md#L142)).
**[target]** ≥40% of installs reach 2 parallel accounts in the
first session; <20% means onboarding — currently the best-built
surface ([full-competitive-ux-review.md:53-55](full-competitive-ux-review.md#L53)) —
has regressed or the import path is broken.

**Retention:** W1/W4 cohort retention of users with ≥1 WJLC.
**[target]** W4 ≥35% for the juggler cohort; single-account users
retained mainly by quota visibility are a separate cohort and
must not be averaged in — they validate G2 alone. Churn signal
to instrument from v0.2: accounts imported but never launched in
14d (import succeeded, value never demonstrated).

## 7. Pricing & OSS sustainability

The code is MIT ([docs/adr/0005-mit-license.md](../adr/0005-mit-license.md#L1));
signing is via Vuxtra's Apple Developer account with notarisation
budgeted ([PROPOSAL.md:135](../PROPOSAL.md#L135)). Options:

| Option | Revenue | Invariant risk | Verdict |
|---|---|---|---|
| Pure OSS + GitHub Sponsors / Open Collective | Low, spiky | None | **Adopt now** — covers signing/domain costs |
| Paid signed builds, free source (source-available convenience) | Modest | Low; must keep `cargo build` path documented | **[target] adopt at v0.5** if maintainer time is the constraint |
| Pro tier: fleet control + advanced cost analytics | Real | Medium — feature-gating can fork the community | Defer to post-1.0; never gate quota/handover (the loop is the OSS promise) |
| Hosted relay / cloud sync | Real | **High** — breaks local-first, adds credential-adjacent surface | Reject (Fleet is LAN/VPN by design, [PROPOSAL.md:111](../PROPOSAL.md#L111)) |
| Support contracts for teams | Niche | Low | Opportunistic only; team features are a non-goal ([PROPOSAL.md:45](../PROPOSAL.md#L45)) |

Crystal's fate is the cautionary data point: a beloved MIT
parallel-session tool deprecated for a commercial successor
([full-competitive-ux-review.md:655-660](full-competitive-ux-review.md#L655)).
Any monetisation that reads as "the free tool is being closed"
will spend the refugee goodwill §5 recruits. Rule: whatever is
in the OSS build at 1.0 stays in the OSS build.

## 8. GTM & community

### 8.1 Positioning (one sentence, testable)

"Orchestrators run your agents. Switchers swap your account.
Juggler does both, quota-aware." — grounded in the finding that
no orchestrator does account juggling and the juggling layer
(cc-switch, claude-swap, ccusage) is a separate, cruder toolset
([full-competitive-ux-review.md:45-49](full-competitive-ux-review.md#L45)).
Competitor anchors: [t3.codes](https://t3.codes/) ·
[github.com/winfunc/opcode](https://github.com/winfunc/opcode) ·
[conductor.build](https://www.conductor.build/) ·
[github.com/ryoppippi/ccusage](https://github.com/ryoppippi/ccusage).

### 8.2 Launch sequence

1. **v0.1-beta:** Show HN ("Show HN: AI Juggler — quota-aware
   juggling for Claude Code accounts") leading with the WOW-b
   screenshot — launch-time quota forecast — plus the honest
   fidelity label as the trust hook. Same week: r/ClaudeAI,
   Claude Code Discord, and a post in Vibe/Crystal migration
   threads (§5.1). Submit to awesome-claude-code, awesome-tauri,
   and awesome-rust lists (each is a PR, not an ad).
2. **v0.2:** "Claude + Codex, one quota picture" — target Codex
   community + ccusage users ([github.com/ryoppippi/ccusage](https://github.com/ryoppippi/ccusage))
   with the cost-ledger CSV story.
3. **v0.5/1.0:** migration-guide refresh + a comparison page
   (Juggler vs. Squad/Vibe/T3 on the juggling layer only — never
   claim board/orchestration parity that was not built).

### 8.3 Migration guides (ship with the phases noted)

| From | Guide contents | Ships |
|---|---|---|
| Vibe Kanban | Export projects from SQLite-backed board → import as Juggler projects; map task-attempts to sessions; what does not transfer (card history) stated plainly | v0.1-beta |
| Crystal | Point Juggler at existing worktrees; session list appears via transcript scan; no re-login beyond per-account vendor flow | v0.1-beta |
| Claude Squad | Import `~/.claude-squad/config.json` profiles (`default_program`, named profiles per full review) → Juggler profiles; tmux sessions finish naturally, new ones launch in Juggler | v0.5 (Ph.3 import) |
| cc-switch / aisw | Import named profiles; tray habits mapped ("your one-click switch is now per-session launch") | v0.2 |
| T3 Code | Import t3-managed `ClaudeCodeProfiles` — already the gate-M1 import path ([gate-M1.md:33-35](../qa/gate-M1.md#L33)) | v0.1-beta (exists) |

Each guide ends with an honest "stay where you are if…" box.
Refugee trust is the asset; a guide that oversells spends it.

### 8.4 Community infrastructure (before Show HN, not after)

GitHub Discussions on; `good-first-adapter` issue label backed
by the Phase 2 conformance kit; a public roadmap mirroring §2;
CHANGELOG discipline per [QUALITY.md:79](../plan/QUALITY.md#L79).
The adapter guide is the growth engine long-term: T3 ships 6+
harnesses ([full-competitive-ux-review.md:140-143](full-competitive-ux-review.md#L140));
Juggler will not out-build that, so it must out-enable it.

---

## 9. Risk register

Likelihood/impact are **[judgement]**, reviewed at each gate.
Every risk names an early-warning signal so it is detected, not
discovered.

| # | Risk | L×I | Mitigation (owner: maintainer unless noted) | Early warning |
|---|---|---|---|---|
| R1 | Vendor ToS tightens against multi-account supervision | M×H | Stay inside ADR-0003 guardrails: passive-only, click-executed, no proxy/rotation ([docs/adr/0003-automation-policy-compliance-guardrails.md](../adr/0003-automation-policy-compliance-guardrails.md#L1)); every feature degrades to single-account value ([PROPOSAL.md:69](../PROPOSAL.md#L69)); surface vendor terms in-product at account-add | Vendor changelog/ToS diff; adapter `verified_against` drift warnings spike |
| R2 | Harness API/format change breaks an adapter (Claude weekly; Codex 0.134.0 precedent, [PROPOSAL.md:132](../PROPOSAL.md#L132)) | H×H | Versioned parsers + committed fixture corpus + golden-file tests that fail loudly on drift (M4-1 pattern, [EPICS.md:103](../plan/EPICS.md#L103)); capability probing at runtime, never assume (SPEC design rule); standing 10–15% drift tax (§3) | Fixture CI failures on a harness update; `verified_against` mismatch banner in dogfood |
| R3 | Credential trust incident (real or perceived) | L×H | Metadata-only login detection precedent ([full-competitive-ux-review.md:323-326](full-competitive-ux-review.md#L323)); Phase 0 decision D1 makes the import copy explicit, tested, and documented; `guardrails.rs` asserts no credential reads outside the carve-out; SECURITY.md reporting path exercised before beta | Any issue/PR mentioning credentials; guardrail test added *before* the feature it guards |
| R4 | Scope creep into "yet another orchestrator" | H×M | Non-goals enforced in design review ([PROPOSAL.md:136](../PROPOSAL.md#L136)); review queue gated on unprompted demand (§2, Phase 3); every new screen must name the funnel stage it serves (§6.3) or it is cut | Phase scope growing after entry criteria met; PRs >400 lines ([QUALITY.md:26](../plan/QUALITY.md#L26)) becoming normal |
| R5 | Handover fidelity failure burns first-user trust | M×H | Fidelity tier displayed *before* confirm; copy the whole session file set and verify parse (Phase 1); never market above the proven tier; fidelity complaint rate is the north-star guardrail (§6.1) | Golden-path fixture with subagents/todos fails; beta fidelity issues >5% of loops |
| R6 | Tiny-team bus factor / maintainer burnout | M×H | Cohorts of 5, not open flood (§5.2); agent runs bounded by RUNBOOK stop conditions ([RUNBOOK.md:32-43](../plan/RUNBOOK.md#L32)); de-scope levers pre-agreed per phase (§3 table) so cutting is a decision already made | Gate slips twice consecutively; review queue >1 week |
| R7 | Competitor closes the juggling gap first (cc-switch adds sessions; T3 adds named accounts) | M×M | Speed on the loop, not breadth (§1); migration guides + import paths make Juggler the destination even if a rival ships first; conformance kit invites the rival's users to bring their harness with them | Competitor release notes mentioning quota-aware handover (anchors: [github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch), [t3.codes](https://t3.codes/)) |
| R8 | Signing/notarisation or updater failure at release | L×H | M5-1/M5-2 negative tests in Phase 1/3 gates (§4); fresh-VM install is an exit criterion, not a hope; beta channel before stable ([PROPOSAL.md:150](../PROPOSAL.md#L150)) | Nightly signed-build job red; Gatekeeper warnings in dogfood |

## 10. Decision log — open questions for the maintainer

Each item has a recommended default. Silence adopts the default
at the Phase 0 gate; none of these block Phase 0 starting.

| # | Question | Recommended default | If rejected |
|---|---|---|---|
| D1 | Credential byte-copy on import vs. SPEC S8 "never copied" ([full-competitive-ux-review.md:331-338](full-competitive-ux-review.md#L331)) | Amend S8 to the 0700-managed-dir carve-out + guardrail test (spec follows honest code) | Remove the copy; users re-login per imported account (worse onboarding, purer spec) |
| D2 | Gate-M1: close, or waive with named criteria? ([gate-M1.md:54-67](../qa/gate-M1.md#L54)) | Waive the full-working-session items into the Phase 1 dogfood criterion; record the waiver in the gate file | Phase 1 entry waits on a full manual M1 pass (adds ~1 week, §3) |
| D3 | Fleet control (M7) before or after 1.0? | After 1.0; fleet *link* (read-only) is the 1.0 fleet scope | Pull M7 forward only with a named second maintainer |
| D4 | Review queue: build on inferred demand? | No — read-only diff at v0.5, full queue only after ≥5 unprompted requests (§5/§2 Ph.3) | Build it and accept the R4 scope risk explicitly |
| D5 | Paid signed builds at v0.5? | Yes, convenience-only; source build stays first-class (§7) | Stay sponsors-only; accept slower maintainer sustainability |
| D6 | ZCode adapter depth | Sessions + quota first; handover only after conformance proof | Full adapter — budget the bundle-archaeology tax (§3) |
| D7 | Telemetry: any network event export before 1.0? | No — local store only, opt-in export designed but off until 1.0 | Ship opt-in export at v0.2 with the §6.2 identity rule as a guardrail test |

---

## 11. The 90-day plan (week by week)

Starts the Monday after D1/D2 are answered. Assumes §3 staffing;
each week ends with `main` green ([RUNBOOK.md:21](../plan/RUNBOOK.md#L21)).

| Wk | Critical path | Ships / proves | Gate |
|---|---|---|---|
| 1 | Phase 0: README/CHANGELOG truth, capability downgrade, honest empty states | Docs-consistency check live | D1/D2 recorded |
| 2 | Gate-M1 disposition; `guardrails.rs` skeleton (writes confinement) | Gate file signed/waived | **P0 gate** |
| 3 | Statusline quota feed (M3-1) behind a feature flag | First Measured quota snapshot in dogfood | — |
| 4 | Usage engine windows + burn-down (M3-2) | Forecast shown in NewSession (internal) | Design checkpoint |
| 5 | Status emission: `QuotaBlocked` + `WaitingUser` end-to-end | `quota_wall_hit` event emittable | — |
| 6 | Stuck heuristics v1 (M2-5 fixtures) | Inbox v1 with real rows | — |
| 7 | Handover: `transcript_ref` fix + source freeze + sibling-file copy | Fidelity fixture (subagents+todos) green | — |
| 8 | Handover moved to `juggler-handover`; PTY-independent golden path | Handover test runs in CI | Design checkpoint (wizard) |
| 9 | Handover wizard: headroom ranking + fidelity label + lineage view | Dogfood loop closed internally | — |
| 10 | Worktree-on-launch + same-branch warning; DB backup/restore | Safety nets complete | — |
| 11 | ⌘K session search; Resume/Relaunch; a11y P1 pass | Daily loop complete | **Internal dogfood week** |
| 12 | Signed/notarised build; fresh-VM install test; migration guides (Vibe/Crystal/T3) | Beta candidate | — |
| 13 | **v0.1-beta to cohort 1 (5 users, §5)**; Show HN draft staged, not posted | First external WJLC counted | **P1 gate review** |

Weeks 14+ (post-90-day, for orientation): beta cohorts 2–3,
Show HN on the first week with ≥10 external WJLC, then Phase 2
entry review against §5.3. If week 13's gate fails, repeat weeks
11–12 with the failure as the scope — do not start Codex on a
loop that has not closed for strangers.

---

## Appendix — deliberate omissions
No re-audit of UI/handover/competitors (deep 01–11 + full review are the evidence); no dates beyond 90 days (§3 ranges are **[target]**, re-estimated per gate); no cloud/account/team tier without amending [PROPOSAL.md:40-47](../PROPOSAL.md#L40) via the decision log. *Next artefact: the Phase 0 PR, not another plan.*









# 13 — Critic: Gaps, Contradictions & Cross-Checks (Deep)

> Role: completeness critic over parts 01–12. This file does not
> re-review the product; it audits the review. Every claim below
> was checked against the part files on disk (`wc -l`, heading
> scan, spot-reads), not against the parts' self-reported
> summaries. Where this file and a part disagree, the part's
> own text (quoted section) wins until a maintainer rules.

## 0. Verdict in one paragraph

The set is complete in coverage and honest in labelling, but
not yet safe to build from without a deduplication pass: the
same five features (quota/forecast, handover wizard, inbox,
⌘K palette, `usage_events` schema) are specified two to four
times with materially different details, and at least three
direct contradictions (Stop confirmation, Conductor pricing,
migration version ownership) would produce divergent
implementations depending on which part an engineer opens
first. No part is missing or stub-length; the risk is
over-specification, not under-specification.

## 1. Existence & length verification (measured)

Command run by critic: `wc -l docs/review/deep/*.md` plus
`grep -c '^#'` per file. Measured 2026-10-13 session.

| Part | File | Lines (wc) | Headings | >300? |
|---|---|---|---|---|
| 01 | `01-ui-shell-and-rail-button-audit.md` | 413 | 52 | pass |
| 02 | `02-ui-panels-sessions-terminal-button-audit.md` | 426 | 22 | pass |
| 03 | `03-ui-onboarding-handover-settings-audit.md` | 492 | 24 | pass |
| 04 | `04-competitor-t3-and-opcode.md` | 585 | 35 | pass |
| 05 | `05-competitor-claude-squad-and-conductor.md` | 599 | 34 | pass |
| 06 | `06-competitor-vibe-crystal-switchers.md` | 597 | 52 | pass |
| 07 | `07-personas-and-journeys.md` | 452 | 47 | pass |
| 08 | `08-architecture-data-supervisor-security.md` | 600 | 25 | pass* |
| 09 | `09-handover-and-quota-engine-design.md` | 542 | 26 | pass |
| 10 | `10-product-proposal-missing-features.md` | 505 | 38 | pass |
| 11 | `11-ux-redesign-screen-spec.md` | 454 | 48 | pass |
| 12 | `12-roadmap-metrics-risks-gtm.md` | 548 | 36 | pass |

Total: 6,213 lines across 12 parts. `*08` sits exactly on
its 600 ceiling — any future edit must delete as much as it
adds or the part breaches its own target band. Parts 01/02
sit below the 450 floor used by competitor parts; that is
consistent with their narrower UI-audit scope and is not
counted as a gap, but a reader comparing raw lengths should
not mistake 413 lines of control tables for thinner evidence:
01 carries 33 `### B-` headings (see §2).

Self-report cross-check: every part's submitted line count
matches `wc -l` within ±1 (trailing-newline counting). No
part inflated its count. Heading density varies 22–52;
02's low count reflects table-driven controls (SP-/PP- IDs)
rather than missing structure — verified in §2.

## 2. Spot-reads (two parts read in depth, rest sampled)

### 2.1 Part 01 — button-by-button claim: TRUE

`grep '^### B-'` returns 33 headings spanning B-01…B-38
(numbering groups some controls, e.g. B-02 covers four nav
items). Spot-read B-22 (Stop, `WorkPanels.tsx:100`), B-30
(NewSession live), B-32 (Profiles Delete, no confirm/undo),
B-36–B-38 (primitives, TerminalView, dead code). Each entry
carries location, backend command, success/error behaviour,
accessibility note, verdict and redesign — the enumeration
is real, not a heading shell. The dead-code finding
(`NewSessionModal` duplicate, unused `stopSession` export)
is independently repeated by 02, which corroborates it.

### 2.2 Part 02 — same surface, different ID scheme: TRUE

02 audits the same `WorkPanels.tsx` surface with SP-/PP-/NS-
IDs in tables (SP-01…SP-11 spot-read). Coverage is genuine
(9-state lifecycle vs 5 tones, FitAddon-only xterm, ended-
session action vacuum), but the ID scheme and several
redesigns diverge from 01 for identical controls — see
contradiction C1 and dedupe rows D6–D7. A builder cannot
implement both parts literally.

### 2.3 Competitor parts — URL citation claim: TRUE

Measured URL counts: 04 = 85, 05 = 151, 06 = 54 `https://`
links. Spot-reads confirm official GitHub/docs/pricing URLs
attached to competitor claims and `file:line` for Juggler
claims. Caveat: URL presence proves citation, not freshness;
star/funding figures remain snapshot-dated (see §4).

### 2.4 Part 07 — first-person personas claim: TRUE, labelled

Each of Maya/Devon/Priya/Jonas/Sara/Alex carries two
`[composite]` blockquote lines in first person (e.g. Maya:
"I pay for two Max accounts so I never stop…") plus an
italic first-person diary entry. The `[composite]` label and
the header disclaimer ("not interviews — validate §9") are
present on the page, so the verbatim style does not deceive
a careful reader. It will deceive anyone who copy-pastes a
quote without its label — see caveat K4.

### 2.5 Part 10 — acceptance criteria claim: PARTIAL

Every mini-PRD §1…§20 contains an `**Acceptance.**` sentence
(24 matches). Spot-read §1: "Claude ±5% vs `/usage` over a
full window; forecast MAPE ≤30% after 20m burn" — measurable
and falsifiable, good. But no PRD uses Given/When/Then, none
lists negative/edge acceptance (empty, stale, denied), and
several acceptances depend on backends that 09/11 declare
unbuilt. Verdict: acceptance *present*, acceptance *suite*
not yet testable. Roadmap part 12 inherits this gap.

## 3. Direct contradictions between parts

### C1 — Stop: confirm sheet vs explicitly no confirm

01 (B-22) prescribes a confirm sheet: `Stop "{label}"?
Running work will end.` with danger button. 02 (SP-04)
argues the opposite in so many words: "Confirm only if…?
No — Stop is recoverable via Resume, so: no confirm, but
toast with Undo (Resume) for 10s". Both cannot ship.
**Ruling needed:** critic favours 02 (recoverable action +
undo beats modal friction, and 01 itself demands Undo
everywhere else), but the maintainer must strike one text.

### C2 — Conductor pricing: debunked in 05, reused in 07

05 (§7.2) establishes that the parent review's $50/$60
Conductor figures are stale and that exact paid tiers are
unresolved (official pricing page gated). 07 nevertheless
grounds Devon's willingness-to-pay in "vs Conductor Pro
$50 [pricing]" as a factual anchor. 12 largely avoids the
figure. **Fix:** 07's anchor must be reworded to
"employer seat budget" without the debunked price, or
footnoted `[unresolved per 05 §7.2]`.

### C3 — Migration ownership: V5 defined twice

08 (§3.3) proposes v5 creating `usage_events` + `lineage`.
09 (§H7) assigns V5 to lineage/handover and (§Q) V6 to
`usage_events` + `quota_samples`, with different columns
and index names (`idx_usage_session` vs `idx_usage_acct_ts`).
10 (§§1/6) consumes `usage_events` without owning a schema.
An engineer applying both migrations corrupts `user_version`
sequencing. **Canonical:** 09 (it sequences by build order
and says so); 08's §3.3 must be marked superseded.

### C4 — Handover wizard: 3 steps vs 7 steps

01 (B-23) and 02 (SP-08/§11.3) specify a 3-step wizard
(target → fidelity/carry-list → confirm). 03 (§6.3) proposes
a 7-step wizard for the same flow. 11 re-specs the wizard
again at screen level. Step count is not cosmetic: it drives
component structure, telemetry events and test fixtures.
**Canonical:** 11 for screens, 09 for fidelity semantics;
01/02/03 wizard texts should point there, not re-specify.

### C5 — Quota forecast maths stated twice, differently

09 (Part Q) designs EWMA time-to-exhaust with staleness
decay and confidence tiers (Measured/Computed/Estimated).
10 (§1) restates burn as "EWMA tokens/min (30m); forecast =
remaining ÷ burn with widening confidence band" — compatible
in spirit, different in constants and silent on decay.
12 then schedules forecast accuracy targets atop 10's
version. **Canonical:** 09; 10 §1 should cite 09's formulae
instead of paraphrasing them.

### C6 — Minor: status vocabulary drift

02 demands a 9-state chip set with a `Stuck?` honesty suffix;
11 (§2.4) re-specifies status encoding normatively; 01 uses
the same state names with different tone language. Not a
logical contradiction, but three normative tables for one
enum guarantees drift. Canonical: 11 §2.4.

## 4. Claims needing a caveat before external use

- K1. Contrast/focus claims (01, 02, 03, 11): all rest on
  code-read of `styles.css:24` (faint 10.5px) and modal code.
  No browser, VoiceOver or screen-reader pass exists. Any
  WCAG AA assertion must read "code-inferred, unverified".
- K2. Quota percentages in every wireframe (01–03, 10, 11)
  are illustrative until the M3/statusline Measured feed
  exists (SPEC:192). Screenshots of these docs must not be
  shown to users as product capability.
- K3. Competitor magnitudes: T3 funding (no primary source),
  Melty $22M vs $24M (05 flags disagreement), cc-switch
  ~140k stars (06 flags unverified string), Vibe 26.9k /
  Crystal 3.1k snapshots. Safe externally only as
  "order-of-magnitude, snapshot-dated".
- K4. Persona quotes/diaries (07, echoed in 10/12): composite
  verbatim-style, not field interviews. Validation plan
  (dogfood diaries, Priya n=3, Sara security gate, Alex n=5)
  must run before any quote leaves the building.
- K5. Handover fidelity: TRANSPLANT is advertised in code
  (`claude_code.rs:111-120`) while the handover crate is a
  doc-only stub; sibling-file (subagent/file-history) loss,
  dotted/long-cwd mangling and compaction loss are
  fixture-evidenced only. No live Claude 2.1.296 dogfood
  matrix (09 §H8) has run. Do not promise F3+ fidelity.
- K6. Scale numbers (08: 10 accounts / 50 sessions) are
  reasoned estimates from Mutex/rescan/sysinfo code paths;
  no load test exists. Present as hypotheses with the
  measurement plan, not as limits.
- K7. ZCode/Codex semantics: ZCode SQLite session-clone
  schema coupling and Codex exec-resume behaviour are
  unverified against real binaries; Codex quota has no
  sanctioned API, so its percentages stay Estimated until
  user-confirmed denominators calibrate them.
- K8. Inferred demand: present/client mode, budget-cap
  alerts, review/diff queue and smart routing are marked
  `[inferred]` in 07/10/12. They are hypotheses gated on
  dogfood/interviews, not roadmap commitments.
- K9. Negative evidence: "no multi-account/quota in
  Conductor/Squad/Vibe/Crystal/aisw" means docs-and-search
  found none and in-app settings were not inspectable for
  the closed-source tools. Phrase as absence-of-evidence.
- K10. Terminal behaviour: attach-failure and xterm remount
  scroll-loss claims (01, 02) were never run against a live
  Tauri backend. Treat as code-read inferences.

## 5. Gaps (missing, not merely duplicated)

- G1. No part owns the toast/confirm/undo system as a
  buildable spec: 01 §8 and 02 both demand it, 11 defines a
  dialog/focus contract, but props, queueing, timing (10s
  undo appears in both 01 and 02 — at least consistent) and
  telemetry names are nowhere centralised. Needs one owner.
- G2. No accessibility verification plan with dates/owners:
  four parts defer to a "Phase C empirical pass" that 12
  does not schedule as a gate. Deferral without a gate is
  how K1 ships.
- G3. No single settings inventory: 03 audits absence vs
  SPEC §9.2, 10 lists settings per PRD, 11 specs seven tabs.
  Three inventories will diverge on first implementation.
- G4. Error-state copy: parts specify empty/loading states
  well (11 §8 matrix) but backend error strings (handover
  failure taxonomy in 09 H4 excepted) lack verbatim copy in
  01–03 redesigns beyond "toast Retry".
- G5. Fleet/Usage remain ComingSoon in product and are
  correspondingly thin in every part; 10/12 correctly scope
  fleet to schema+teaser, but no part specs the teaser
  screen itself. Acceptable if deliberate — flagged so the
  omission is a decision, not an accident.
- G6. No part reconciles persona set naming across files
  beyond 10's explicit freeze (P1 Maya…P6 Alex matching
  07). 12's recruiting cohorts (Vibe/Crystal refugees) are
  not mapped to personas; GTM-to-persona traceability gap.

## 6. Cross-part feature deduplication map

Read: "Canonical home" is the one place a builder should
implement from; other parts should shrink to a pointer.

| # | Feature (proposed 2–4×) | Appears in | Canonical home |
|---|---|---|---|
| D1 | Quota engine + forecast | 06 S1–S10, 09 Q, 10 §1, 11 Usage | 09 (maths) + 10 §1 (product) |
| D2 | Handover wizard + fidelity tiers | 01 B-23, 02 §11.3, 03 §6.3, 09 H, 11 wizard | 09 H (semantics) + 11 (screens) |
| D3 | Needs-attention inbox | 02, 07 journeys, 10 §2, 11 §4.1 | 11 §4.1 (screen) + 10 §2 (PRD) |
| D4 | ⌘K command palette | 01 §9, 04 steal, 10 §10, 11 palette | 10 §10 + 11 (interaction) |
| D5 | `usage_events` / quota schema | 08 §3.3, 09 H7/Q V5–V6, 10 §§1/6 | 09 V5–V6 only (see C3) |
| D6 | Stop/Delete confirm + undo | 01 B-22/B-32, 02 SP-04 | Ruling per C1, then 11 contract |
| D7 | Toast/banner system | 01 §8, 02 §10, 11 §2.5 | 11 §2.5 (needs G1 props) |
| D8 | Worktree isolation | 05 §3, 06 S9, 10 §4, 12 Phase 1 | 10 §4 |
| D9 | Review / diff queue | 05 steal #1, 10 §11, 12 v0.5 | 10 §11 (dogfood-gated) |
| D10 | Status/state chip encoding | 01, 02 §7, 11 §2.4 | 11 §2.4 (see C6) |
| D11 | Notifications centre/digest | 07, 10 §7, 11 tray/hotkey | 10 §7 |
| D12 | Settings (all tabs) | 03 audit, 10 inventory, 11 §tabs | 11 (screens) + 10 (inventory) |
| D13 | Session search/replay/archive | 02 SP-11, 10 §5, 11 | 10 §5 |
| D14 | Cost ledger / USD estimates | 06 ccusage, 09 Q, 10 §6, 11 Usage | 09 (honesty tiers) + 10 §6 |
| D15 | Onboarding funnel fixes | 03 §§1–5, 11 §5 | 03 (audit) + 11 §5 (targets) |

Net effect if un-deduped: an estimated 25–30% of the
proposal surface exists in two normative versions. The fix
is editorial (replace duplicate specs with pointers), not
new writing — roughly one pass per row above.

## 7. Consolidated unresolved (inherited, de-duplicated)

Runtime verification (browser/screen-reader/Tauri live),
Measured quota backend (M3), status emission for
WaitingUser/Stuck/QuotaBlocked, supervisor input-buffer
signal for unsent-input detection, desktop Focus/Attach
adapter (SPEC:336), live handover dogfood matrix
(Claude 2.1.296 × latest × 5 project shapes), ZCode SQLite
schema proof, Codex denominator calibration, Conductor
exact paid tiers, primary T3 funding source, live GitHub
star counts via API, in-app settings inspection for
closed-source competitors, persona validation interviews,
10/50 load test, and maintainer decisions D1/D2 that start
12's 90-day clock. Each item already has an owner part;
this list exists so none is silently dropped between parts.

## 8. Ship-gate checklist for the review set itself

1. Maintainer rules on C1 (Stop) and strikes the loser text.
2. 07's Conductor $50 anchor fixed per C2.
3. 08 §3.3 marked superseded by 09 V5–V6 per C3.
4. Wizard step-count unified (C4) in 01/02/03 pointers.
5. Dedupe pass per §6 table: duplicates become pointers.
6. K1–K10 caveats copied into any external-facing summary.
7. G1 toast/undo spec and G2 a11y gate get named owners.
8. Only then: 12's Phase 0 exit criteria may be declared.

Until items 1–5 land, the correct citation practice is:
engineers cite the canonical home in §6, and treat every
other part's version of that feature as commentary.

— End of critic part 13. Measured files: 12 parts,
6,213 lines, all >300 lines; spot-reads: 01 (B-22/B-30/
B-32/B-36–38), 02 (SP-01…SP-11), 07 (Maya/Devon/Priya),
10 (§1–§2 acceptance); URL counts measured for 04–06.

---

# Appendix — Unresolved Items (preserved verbatim from parts)

- No runtime/browser/screen-reader test run — contrast (styles.css:24 faint 10.5px), focus-trap and focus-steal claims are code-read only
- Quota headroom values in redesigns await M3 Measured quota backend; wireframe percentages are illustrative
- TerminalView attach-failure behaviour unverified without live Tauri backend
- No runtime/browser/screen-reader verification - focus-trap, AA contrast (styles.css:24), xterm remount scroll loss are code-read inferences
- Stop-confirm for unsent input needs supervisor input-buffer signal absent from bridge.ts
- Quota forecast wireframes blocked on M3 statusline feed (SPEC:192); desktop Focus/Attach blocked on desktop adapter (SPEC:336)
- Live handover fidelity on real Claude 2.1.296 (cross-CLAUDE_CONFIG_DIR --resume, dotted/long cwds, MCP state) — needs dogfood, code-read only
- Contrast/focus-trap behaviour unverified — no browser or screen-reader pass performed
- T3 funding: no primary raise source found (marked NEG-EV)
- Live star counts from snapshot mirrors only, no GitHub API fetch
- In-app non-doc settings not run for either tool; absence claims docs-level NEG-EV
- Conductor exact paid tiers unresolved — official pricing page gated; treated as freeware + team/cloud upsell, $50/$60 not repeated as fact
- Conductor in-app multi-account/quota absence remains negative evidence (docs/search only, in-app settings not inspectable); iOS status conflicting (Mac-only+iOS drivable vs Coming soon)
- Melty total raised $22M vs $24M disagreement flagged unresolved; live GitHub star counts are snapshot magnitudes without API fetch
- No live install/run of Vibe/Crystal/cc-switch/ccusage/cswap/aisw — CLI behaviours from README/changelog/PR evidence only
- Star counts are snapshots: cc-switch ~140k string unverified, Vibe 26.9k secondary, Crystal 3.1k; no active-user/download sizing verified
- aisw/Vibe/Crystal quota absence is negative evidence, not vendor denial; Nimbalyst pricing and cswap usage-API internals not primary-verified
- All quotes/diaries are composite verbatim-style, not field interviews — validation plan §11 (dogfood diaries, Priya n=3, Sara security gate, Alex n=5) required before external use
- Present/client mode, budget-cap alert, review-queue demand marked [inferred], need dogfood/interview validation
- Subscription spend figures are order-of-magnitude estimates from research tiers, not live vendor pricing verification
- No 10/50 load test exists — scale numbers are reasoned estimates, not measurements
- ZCode vendor SQLite schema and Codex exec-resume semantics unverified against real binaries
- Desktop-harness requirements are SPEC-only; no implementation evidence
- No live Claude/Codex/ZCode binary run: JSONL sibling-file layout, todo-in-JSONL, and statusline freshness are fixture/research-evidenced only; Sec.H8 dogfood matrix (Claude 2.1.296 x latest x 5 project shapes) still required before TRANSPLANT ships
- ZCode T2 blocked: SQLite session-clone schema-coupling unproven across versions
- F4.1 credential-copy decision (SPEC S8 vs import code) blocks proposal sections — needs maintainer security call
- Maintainer rulings still needed: C1 Stop confirm-vs-undo, C3 canonical migration (critic recommends part 09), C4 wizard step count
