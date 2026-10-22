# 04 — Daily Indicators & Right Inspector (Pass 2)

> **Scope:** the daily supervision surface only — what every
> session row/card shows at a glance, and what the right-hand
> inspector shows on selection. Onboarding, handover mechanics,
> and fleet transport are cited, not re-specified here.
> **Canonical input:** [expected-workflow.md](../../internal/expected-workflow.md#L13)
> lines 13–18 — "lots of indicators … which provider it is,
> account, models currently running. Amount of quota left",
> "whole right section … stats, actions to manage, switch",
> "how much resources it is using, cpu, ram, did it spawn any
> sub tasks, sub processes", plus zombie/orphan tracking
> ([expected-workflow.md](../../internal/expected-workflow.md#L17)).
> **Method:** full read of the canonical workflow, then targeted
> windows of the prior reports, `docs/SPEC.md`, `docs/PROPOSAL.md`,
> `docs/plan/EPICS.md`, and the live code they audit. Every claim
> below carries a repo `file:line`, a prior-report section, or
> an external URL. No runtime/browser test was run.

---

## 1. Terminology — adopt maintainer language, map, flag conflicts

The maintainer names three nouns
([expected-workflow.md](../../internal/expected-workflow.md#L11)):
"we should call them providers, accounts, and instances of
the tool". This pass adopts that language in all user-visible
copy. The code/spec use different words for two of the three.

| Maintainer (UI) | Existing domain term | Where defined | Conflict? |
|---|---|---|---|
| Provider | Harness / `HarnessId` | [model.rs](../../../crates/juggler-core/src/model.rs#L91) sketch in [SPEC.md](../../SPEC.md#L91) | **Yes — rename in UI only** |
| Account | `Account` | [model.rs](../../../crates/juggler-core/src/model.rs#L145) | No |
| Instance (tool instance) | `SessionRecord` + `InstanceKind` | [model.rs](../../../crates/juggler-core/src/model.rs#L389), [model.rs](../../../crates/juggler-core/src/model.rs#L325) | **Yes — overloaded** |
| *(no maintainer word)* | `Profile` (Account + overlay) | [model.rs](../../../crates/juggler-core/src/model.rs#L173) | **Yes — invisible layer** |
| *(no maintainer word)* | Session (conversation) | ADR-0007 §1, [SPEC.md](../../SPEC.md#L336) | **Yes — vs Instance** |
| Project | `ProjectRecord` | [model.rs](../../../crates/juggler-core/src/model.rs#L371) | No |

**Conflict 1 — Provider vs Harness.** `HarnessId` values are
`"claude-code" | "codex" | "zcode"`
([SPEC.md](../../SPEC.md#L91));
UI copy today says "Claude Code" as a static chip
(deep 01 §B-06). "Provider" in vendor docs usually means the
model/API provider (Anthropic/OpenAI), not the CLI. Resolution:
UI label = **Provider**, tooltip/subtitle = harness binary
(`claude`, `codex`, `zcode`); code keeps `HarnessId`. Never
show the word "harness" to users. Prior reports use "harness"
throughout (e.g. full review F5.5) — that word must not leak
into row/inspector copy.

**Conflict 2 — Instance vs Session (the load-bearing one).**
ADR-0007 §1–2 already decided: an *instance* is one running
occurrence (Terminal | Desktop); a *session* is the conversation
inside it. The shipped code collapses them: `SessionInfo` is
documented as "An instance/session row"
([SessionInfo.ts](../../../ui/src/types/generated/SessionInfo.ts#L3))
and the Instances rail tab is Sessions filtered live/recent
through the same `SessionRow`
([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L295),
prior deep 01 §0.2, full review F2.2). Resolution for this
spec: **the rail row is the Instance; the inspector's History
tab is the Session.** One instance row may host 0..n past
sessions (resumes/forks). Do not add a second rail list until
that 1:n is real in the read model.

**Conflict 3 — Profile has no maintainer word.** A `Profile`
is Account + `SettingsOverlay{model, env}`
([model.rs](../../../crates/juggler-core/src/model.rs#L161)).
Users think in Accounts; Profiles are launch presets. Resolution:
never a rail/inspector top-level noun. Show as a secondary
chip on the Instance (`Main · opus`) and as a chooser only
inside Launch / Hand over… actions. Prior deep 02 §9.2 (PF-04)
already treats profile fields as launch-form detail — extend
that: demote, don't promote.

**Rule for the rest of this file:** Provider = harness,
Account = login, Instance = running occurrence, Session =
conversation, Profile = launch preset (secondary only).

---

## 2. Second-pass verdict — what prior research missed

Prior work is strong on *actions* (deep 01/02 button audits)
and *states* (deep 02 §7). It under-weights the maintainer's
actual daily question in
[expected-workflow.md](../../internal/expected-workflow.md#L13):
"at a glance, which of my many running things needs me, is
about to die on quota, or is eating the machine?" Five gaps:

**G1 — No per-row quota/cost/resource glance exists in prior
target designs.** Deep 02 §11.2 sketches one header line with
tokens + estimated cost, and full review UX.4 specifies rings
+ forecasts — but both live in the *panel/Overview*, not on
every rail row. The rail row today carries only label + raw
status string + a colour dot
([WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L429)).
A user with 8 instances must click each one to learn quota
or burn. This pass moves quota bar, cost burn, and a CPU/RAM
sparkline *onto the row* (§3). Prior: deep 01 B-11, deep 02
WR-05 — extend, don't restate: they specify the status chip;
neither specifies quota/cost/sparkline on the row.

**G2 — Subprocess/subtask tracking is promised but never
given a UI.** [SPEC.md](../../SPEC.md#L234)
requires the full process tree (subagents, node/electron
helpers, git/ripgrep children) attributed per session via
ppid-map, and the supervisor already exposes `descendants()`
([supervisor.rs](../../../crates/juggler-supervisor/src/supervisor.rs#L341)).
No prior report puts a child-count badge on the row or a
Processes tab in the inspector. The maintainer's zombie
problem ([expected-workflow.md](../../internal/expected-workflow.md#L17))
is therefore invisible until it hurts. This pass adds both
(§3 I-08, §5 Processes tab). Prior: full review F1.1/F5.1
discuss scale, not per-instance child visibility.

**G3 — Git state (branch/dirty) is mentioned once, never
specified as an indicator.** Deep 02 §6 notes "no branch/dirty
indicator exists anywhere" and parks worktree choice in a
launch popover (PP-05). The maintainer workflow is
project-first ([expected-workflow.md](../../internal/expected-workflow.md#L5)):
two instances on the same repo on different branches is the
normal case, and a dirty tree is the top handover risk.
This pass makes branch + dirty count a first-class row and
inspector field (§3 I-09), sourced from `git status` on the
instance `cwd` ([model.rs](../../../crates/juggler-core/src/model.rs#L405)).

**G4 — Prior "needs-attention" is a separate Inbox tab; the
row itself still doesn't rank.** Full review UX.3 and deep 01
§10 both add an Inbox as rail tab 1, sorted waiting-desc.
Correct — but insufficient: when the user is in Projects or
Sessions view, nothing on the row says "this one first".
This pass adds a per-row needs-attention badge with reason
text and makes it the default sort key in every list (§4),
so the Inbox becomes a filter, not a place you must visit.
Prior: full review F2.5 ("modelled but never emitted",
`model.rs:188-207` vs UI) — the emission gap is upstream;
this spec assumes emission (M2-5, [EPICS.md](../../plan/EPICS.md#L79))
and specifies the rendering contract it must satisfy.

**G5 — Quality signals are invoked, never defined.** Prior
reports say "stuck detection ≥90% precision/recall" (SPEC
§9 metrics, [PROPOSAL.md](../../PROPOSAL.md#L146))
and show "Stuck?" chips (deep 02 §7), but no report defines
tokens/min, error rate, or context-% — what is measured vs
inferred vs unknowable per Provider. Showing an undefined
"quality score" would repeat the fabricated-zero sin prior
reports rightly condemn (deep 01 B-14, `percent={0}` at
[App.tsx](../../../ui/src/App.tsx#L513)).
§6 defines each signal with its honesty tier or refuses it.

**What prior got wrong (not just missed):** deep 01 B-02 /
deep 02 SH-02 propose *removing* Usage/Fleet from nav until
real. For the daily surface that is backwards: the *inspector*
Usage tab (§5) is where quota must live per selection, and it
can ship with honest "No data" states long before the global
Usage screen (M3, [EPICS.md](../../plan/EPICS.md#L94)).
Removing the destination delays the per-instance habit.
Keep nav, but make every global number traceable to a row.

---

## 3. The indicator system — every row/card, every field

One row = one Instance. Fields I-01…I-11 in render order.
"Source" names the owning layer today; gaps name the task
that must add it. Refresh = how the *row* updates (inspector
rates in §5 may be faster). Stale rule (global): any value
older than 2× its refresh interval renders dimmed with a
`stale · Nm` caption — never silently as fresh (extends the
degraded-banner contract in deep 01 §8 / deep 02 §10).

| ID | Indicator (row rendering) | Data source today | Refresh | Empty / stale behaviour |
|---|---|---|---|---|
| I-01 | Provider icon + wordmark abbrev. (`◆ Claude`, `⬢ Codex`, `▲ ZCode`) | `SessionInfo.harness` ([SessionInfo.ts](../../../ui/src/types/generated/SessionInfo.ts#L11)); icon set [icons.tsx](../../../ui/src/components/icons.tsx#L22) (8 glyphs — Provider marks must be added) | Static per instance | Unknown harness → generic `◇ Tool` + text; never blank |
| I-02 | Account avatar: initials disc, per-Account hue + label (`WM` / `Work Max`) | `Account.label` ([model.rs](../../../crates/juggler-core/src/model.rs#L151)); denormalised `ProfileInfo.account_label` ([ProfileInfo.ts](../../../ui/src/types/generated/ProfileInfo.ts#L15)) — **not yet on `SessionInfo`; must be added** | Static; login flip live | Logged-out → avatar hatched + `Sign in` text chip (not colour-only) |
| I-03 | Model chip (`opus-4.8`, `gpt-5-codex`) | `ProfileInfo.model` ([ProfileInfo.ts](../../../ui/src/types/generated/ProfileInfo.ts#L23)) via `SettingsOverlay.model` ([model.rs](../../../crates/juggler-core/src/model.rs#L164)); **not on `SessionInfo` today** | On launch; on in-session `/model` change via transcript watch ([SPEC.md](../../SPEC.md#L190)) | No overlay → `default model` (honest, not guessed name) |
| I-04 | State icon + text chip (9 states, §4 legend) | `SessionStatus` 9 variants ([model.rs](../../../crates/juggler-core/src/model.rs#L188)); emission is the gap (full review F5.2) | Event-driven via `sessions_updated` ([bridge.ts](../../../ui/src/lib/bridge.ts#L325)); target <300 ms ([EPICS.md](../../plan/EPICS.md#L66)) | Unknown wire value → `Unknown` chip (parse returns `None`, [model.rs](../../../crates/juggler-core/src/model.rs#L245)); never neutral-dot fallback |
| I-05 | Quota bar (account window) + `% left` + reset countdown (`17:00 · 2h10m`) | `QuotaWindow{used_percentage, resets_at}` ([model.rs](../../../crates/juggler-core/src/model.rs#L279)); Claude statusline feed ([SPEC.md](../../SPEC.md#L192)), ZCode computed ([SPEC.md](../../SPEC.md#L210)), Codex estimated ([SPEC.md](../../SPEC.md#L203)) | 30 s poll + on usage event; countdown ticks 1 min | No feed → dashed bar + `Quota: no data` (replaces fabricated `percent={0}`, deep 01 B-14); tier tag `Meas/Comp/Est` always adjacent ([model.rs](../../../crates/juggler-core/src/model.rs#L288)) |
| I-06 | Cost burn (`$0.41` session, `~$3.20/h` rate) | Tokens `TokenCounts` ([model.rs](../../../crates/juggler-core/src/model.rs#L263)) × pricing table — **pricing table does not exist** (full review F1.2: no USD view anywhere) | Tokens on transcript event; rate recomputed 30 s | No pricing for model → tokens only (`18.2k tok`), never a fabricated `$0.00`; estimates tagged `Est` |
| I-07 | CPU/RAM sparkline (60 s trailing, dual mini-area) + current (`12% · 1.1 GB`) | Supervisor process tree + sysinfo ([SPEC.md](../../SPEC.md#L231): ~2 s trees) — **collector not yet implemented** (no sysinfo in supervisor today) | Sample 2 s, row repaint 5 s (sparkline), text 2 s | Desktop-kind / adopted orphans without tree → `—` + `resources unavailable`; flat-zero only if measured zero |
| I-08 | Subprocess/subtask count badge (`⑂ 4` children, `⧉ 2` subtasks) | `Supervisor::descendants()` ppid walk ([supervisor.rs](../../../crates/juggler-supervisor/src/supervisor.rs#L341)); subtasks = adapter child-session refs (Claude subagent transcripts, [SPEC.md](../../SPEC.md#L190)) — **count not yet surfaced to UI** | 5 s (with I-07 sample) | 0 → badge hidden (not `⑂ 0` noise); orphan-after-exit → badge persists amber with `left behind` (see Processes tab) |
| I-09 | Git branch + dirty (`⎇ main · 3●`) | `git status --porcelain=v2 --branch` on `SessionRecord.cwd` ([model.rs](../../../crates/juggler-core/src/model.rs#L405)) — **not implemented anywhere today** (deep 02 §6) | On fs watch of `cwd/.git` + 15 s poll; after terminal commands (output event debounce 2 s) | Non-git cwd → field omitted entirely; detached HEAD → `⎇ detached@a1b2c3`; stale → branch dimmed |
| I-10 | Needs-attention badge (`! Needs you · 4m`, `⚠ Stuck?`, `⛔ Quota · resets 17:00`) | Derived from I-04 states WaitingUser/Stuck/QuotaBlocked/Failed (full review UX.3 sources) + waiting duration from `last_activity` ([model.rs](../../../crates/juggler-core/src/model.rs#L411)) | Event-driven; waiting timer ticks 15 s | Mutually exclusive: highest-severity reason wins (§4 ranking); snoozed → badge collapses to `💤 12m` text |
| I-11 | Age + last-activity (`42m · active 10s ago`) | `started_at` / `last_activity` ([model.rs](../../../crates/juggler-core/src/model.rs#L409)) | 15 s tick | Just started → `starting…`; no activity yet → `no output yet` |

**Card variant (Overview / project "On it now"):** same
I-01…I-11, two-line layout, sparkline widened to 5 min.
No field exists on card that is absent on row — Overview
must never show a number the rail cannot explain (the
`dashboard_stats` discard at [App.tsx](../../../ui/src/App.tsx#L442)
is the anti-pattern this forbids).

---

## 4. Legend, redundancy, density, sorting & filtering

### 4.1 Legend — state (I-04/I-10), quota, confidence

Colour is **never** the only channel (full review UX.8 P1;
deep 01 §8). Every state = icon + text + colour together.
The legend lives in three places with identical glyphs:
rail footer `?` popover, inspector Overview header, and
the empty-Inbox "All clear" explainer.

| Glyph + text | Meaning (wire) | Colour token | Row action hint |
|---|---|---|---|
| `◌ Starting` | `starting` ([model.rs](../../../crates/juggler-core/src/model.rs#L230)) | accent, pulsing ring | Stop |
| `● Running` | `running` ([:231](../../../crates/juggler-core/src/model.rs#L231)) | live green + filled dot | Focus terminal |
| `! Needs you` | `waiting_user` ([:232](../../../crates/juggler-core/src/model.rs#L232)) | warning amber + `!` badge | Focus terminal |
| `⚠ Stuck?` | `stuck` ([:233](../../../crates/juggler-core/src/model.rs#L233)) — `?` is honesty (heuristic, [SPEC.md](../../SPEC.md#L221)) | warning amber + dashed border | Inspect → Processes |
| `⛔ Quota blocked` | `quota_blocked` ([:234](../../../crates/juggler-core/src/model.rs#L234)) | danger red + barred icon | Hand over… / Queue |
| `✓ Finished` | `finished` ([:235](../../../crates/juggler-core/src/model.rs#L235)) | success + check | Resume |
| `■ Stopped` | `stopped` ([:236](../../../crates/juggler-core/src/model.rs#L236)) | neutral grey + square | Resume |
| `✕ Failed` | `failed` ([:237](../../../crates/juggler-core/src/model.rs#L237)) | danger red + `✕` | Recovery card |
| `⇄ Handed over` | `handed_over` ([:238](../../../crates/juggler-core/src/model.rs#L238)) | accent + swap arrows | Jump to lineage |

Quota bar legend: fill = used %, hatch overlay = estimated
portion, `Meas/Comp/Est` text tag =
`Confidence` ([model.rs](../../../crates/juggler-core/src/model.rs#L288)).
Threshold ticks at 70/90% carry text (`70%`, `90%`) — the
existing ring's colour-only thresholds
([primitives.tsx](../../../ui/src/components/primitives.tsx#L178))
must gain `%` text + `role="img"` label (deep 01 B-14).
Account identity: hue is redundant with initials + label
text (I-02); colour-blind safe because hue is never load-
bearing. Sparkline legend: solid = RAM, outline = CPU%,
axis-free with `now` value in text (I-07).

### 4.2 Density modes (persisted per user, `aij.density`)

| Mode | Row height | Fields shown | For |
|---|---|---|---|
| Comfortable (default) | ~56 px, 3 lines | All I-01…I-11; sparkline 96 px wide | ≤8 instances, learning the tool |
| Compact | ~36 px, 2 lines | I-01 I-02 I-04 I-05(bar only) I-09 I-10; sparkline → single CPU%+RAM text | 9–20 instances (SPEC 20-session target, [SPEC.md](../../SPEC.md#L20)) |
| Minimal / collapsed rail | 36 px icon rail (60 px, [WorkflowRail.tsx](../../../ui/src/components/WorkflowRail.tsx#L110)) | Avatar disc + state glyph + `!` count; quota → 3-segment tick | Monitoring while coding |

Density never hides I-10 (needs-attention) or I-04 text in
Comfortable/Compact; Minimal keeps the glyph + badge count.
Switch: header segmented control + `⌘D` cycle; rows keep
`aria-label` with full field text in all modes.

### 4.3 Sorting & filtering (rail toolbar, above list)

Default sort = **Needs me first**, then quota-risk, then
recent activity. This replaces today's silent caps
(`slice(0,6/8/15)`, deep 02 WR-02/WR-05) — lists virtualise
past 50, search always present.

| Control | Options (in order) | Behaviour |
|---|---|---|
| Sort ▾ | Needs me (default) · Quota risk · Project · Provider · Account · Age · Name | Needs me = severity rank Failed > QuotaBlocked > NeedsYou(wait desc) > Stuck? > Running > rest; Quota risk = lowest `% left` × burn-rate (I-05×I-06), unknown-quota last with `no data` group header |
| Filter chips (multi) | `All` `! Needs me` `Running` `Ended` + Provider ▾ + Account ▾ + Project ▾ | Chips are toggles with counts (`Needs me (3)`); Provider/Account/Project are single-select dropdowns fed by I-01/I-02/project ([ProjectInfo](../../../ui/src/types/generated/ProjectInfo.ts#L3)); combined filters AND across groups |
| Search `⌕` (`⌘K` focuses) | Free text over label, project, branch, model, account | Matches I-03/I-09 too (find "all on `main`"); `Esc` clears; empty result = `No instances match` + `Clear filters` button |
| Group-by ▾ | None · Project · Account · Provider | Group headers carry rollups (`3 running · 1 needs you · lowest quota 22%`); Needs-me sort applies inside groups |

Snooze/mute (from full review UX.3) lives on the I-10 badge
context menu: `Snooze 15m / 1h / Until reset`, `Mute this
instance`. Snoozed items sink below unsnoozed peers but
never disappear; badge on rail tab shows unsnoozed count
only. Fleet-ready: every filter key includes `machine`
([model.rs](../../../crates/juggler-core/src/model.rs#L398))
from day one even though Fleet ships M6
([EPICS.md](../../plan/EPICS.md#L134)).

---

## 5. Right inspector — full specification

Selection model: clicking any rail row (Instance, Project)
or global card (Provider, Account) replaces the inspector.
Header always shows: selection type icon, name, I-04 state
chip, and primary action. Six tabs for every selection type
— **Overview · Usage · History · Performance · Processes ·
Actions** — with per-type content below. Tab state persists
per selection type. Empty selection → inspector shows the
fleet-at-a-glance summary (counts by state, lowest-quota
Account, machine gauges from [SPEC.md](../../SPEC.md#L345)),
never a blank panel. Stale rule from §3 applies per field;
each tab footer shows `Updated 3s ago · source: supervisor`.

Refresh-rate key used in all tables: **E** = event-driven
(<300 ms, [EPICS.md](../../plan/EPICS.md#L66)),
**2s/5s/15s/30s/1m** = poll intervals aligned to
[SPEC.md](../../SPEC.md#L231)
(staggered sysinfo), **M** = manual (button), **—** = static.

### 5.1 Selecting an Instance (the daily case)

Header: I-01 I-02 I-03, label, project chip (→ Project
inspector), `⎇ branch` (I-09), age (I-11). Primary:
terminal-kind → `Focus terminal`; desktop-kind → `Focus
window` ([SPEC.md](../../SPEC.md#L347)).

| Tab | Stats / charts / actions (every element) | Source · refresh | Empty / stale |
|---|---|---|---|
| Overview | State chip + since; Provider/Account/Profile chips; model; project + cwd (copyable); native session id (`TranscriptRef.native_id`, [model.rs](../../../crates/juggler-core/src/model.rs#L320)); quota bar + forecast (I-05); tokens + cost (I-06); CPU/RAM now (I-07); children (I-08); lineage (`⇄ from/to`, [SPEC.md](../../SPEC.md#L302)) | Mixed E/2s/30s as §3 | No quota feed → dashed bar + `No data — arrives with statusline` ; no lineage → section omitted |
| Usage | Token breakdown (in/out/cache/r, [model.rs](../../../crates/juggler-core/src/model.rs#L263)); tokens/min chart (§6); cost chart (uPlot, [SPEC.md](../../SPEC.md#L349)); account-window contribution %; confidence tag per series | Transcript watch E; rollup 30s | Headless/no transcript yet → `No usage recorded yet`; Codex series hatched + `Estimated` |
| History | Session timeline for this instance (resumes, handovers, crashes); transcript viewer link; pack export preview ([SPEC.md](../../SPEC.md#L294)) | Indexer ([SPEC.md](../../SPEC.md#L41)) E on new session | Single session → one row, no timeline chrome; imported history badged `imported` ([model.rs](../../../crates/juggler-core/src/model.rs#L360)) |
| Performance | CPU% + RSS charts (5m/1h); tokens/min; error-rate (§6); context-% gauge (§6, conditional); stuck-score spark (§6); footprint vs learned peak ([SPEC.md](../../SPEC.md#L235)) | 2s sample, chart repaint 5s | <60s old → `Collecting…` skeleton; unavailable signal → named `Not measurable for this Provider` (never hidden) |
| Processes | Live tree table: PID, name, CPU%, RSS, age, `⑂` children; orphan section (below); per-row `Reveal` (no kill — Actions tab owns kills) | `descendants()` ([supervisor.rs](../../../crates/juggler-supervisor/src/supervisor.rs#L341)) 5s | No children → `No subprocesses`; tree read fails → banner + Retry (M) |
| Actions | `Focus/Open terminal`, `Stop` (`⌘.`, deep 02 SP-04 rules), `Resume`/`Relaunch` (ended), `Hand over…` (`⌘⇧H`, wizard deep 02 §11.3), `Queue resume at reset` (QuotaBlocked only), `Export pack…`, `Archive`, `Delete…` (guarded, deep 02 §10) | Commands via bridge; busy spinner per button | Irrelevant actions disabled **with adjacent reason text** (deep 02 §10), never hidden; destructive = confirm popover + Undo toast |

### 5.2 Selecting a Session (conversation, from History/search)

Header: session label, owning Instance chip (→ 5.1), state,
started/ended. Tabs: **Overview** (turn count, tokens,
model(s) used, files touched from transcript IR
[SPEC.md](../../SPEC.md#L185));
**Usage** (per-turn token/cost table, export CSV);
**History** (this session's lineage chain A→B→A,
[SPEC.md](../../SPEC.md#L288));
**Performance** (tokens/min over session life, §6);
**Processes** (read-only: processes this session spawned,
from journal [journal.rs](../../../crates/juggler-supervisor/src/journal.rs#L22) — ended sessions show final tree snapshot);
**Actions** (`Resume` primary `⌘R`, `Hand over…`, `Export
pack…`, `Fork/branch copy`, `Archive`, `Delete…`).
Refresh: E while live, — once ended (immutable + `Ended
HH:MM` stamp). Empty: unparsed transcript → `Transcript not
readable yet` + parser version, never zeroed stats.

### 5.3 Selecting a Provider (e.g. Claude Code)

Header: Provider icon + binary version + verified-range
badge (`discover()`, [SPEC.md](../../SPEC.md#L158):
`verified 2.1.x · you run 2.1.296` or drift warning).
**Overview:** accounts count, running instances, aggregate
quota (worst account first), capabilities flags
([SPEC.md](../../SPEC.md#L146)).
**Usage:** per-Account stacked burn (5h/weekly), all series
tier-tagged. **History:** adapter version history + session
counts by week. **Performance:** aggregate tokens/min,
error rate across accounts (§6 caveats doubled — cross-
account sums mix confidence tiers, label `mixed`).
**Processes:** all instances' trees grouped (read-only rollup
of 5.1 Processes). **Actions:** `Add account…` (wizard,
[SPEC.md](../../SPEC.md#L409)),
`Check for Provider update` (M, never auto-update
[SPEC.md](../../SPEC.md#L358)),
`Open Provider settings`. Refresh E/30s; empty (no accounts)
→ wizard CTA as the whole Overview.

### 5.4 Selecting an Account

Header: avatar (I-02), label, plan tier (`PlanTier`,
[model.rs](../../../crates/juggler-core/src/model.rs#L120)),
login chip. **Overview:** quota rings (5h + weekly/credits,
existing `QuotaRing` [primitives.tsx](../../../ui/src/components/primitives.tsx#L165)
+ `%` text fix), burn-down forecast + reset countdowns
([SPEC.md](../../SPEC.md#L257)),
running instances on this account, usual-projects affinity
(full review F1.4). **Usage:** window timeline (uPlot),
per-session breakdown table, CSV export
([SPEC.md](../../SPEC.md#L349)).
**History:** sessions by this account (grouped Today/
Yesterday/Earlier, deep 02 PP-07), handovers in/out.
**Performance:** tokens/min + cost rate trends; peak
footprint per Profile. **Processes:** instances' trees
(rollup). **Actions:** `Launch session…`, `Sign in /
Re-check login` (deep 02 PF-03 quartet), `New profile…`,
`Rename…`, `Delete account…` (danger, managed-dir-kept
note, deep 02 PF-08). Refresh: quota 30s, rest E/15s.
Empty: logged-out → Overview collapses to sign-in card;
quota unknown → rings dashed `— No data`.

### 5.5 Selecting a Project

Header: name, root (copy + Reveal), `⎇` summary across
instances, dirty total. **Overview:** "On it now" instance
card (full I-01…I-11), accounts-used history
([ProjectInfo.ts](../../../ui/src/types/generated/ProjectInfo.ts#L19)),
instance/session counts. **Usage:** cost/tokens by Account
for this project. **History:** session timeline with lineage
links (deep 02 PP-07). **Performance:** per-instance resource
chart overlaid (who starved whom). **Processes:** all trees
in this project + orphan section. **Actions:** `Launch with
another account…` (rows ranked by headroom + `Usual` star,
deep 02 PP-05 fix), `Hand over project…`, `Open in editor`,
`Pin`, `Rename…`, `Archive project…`. Refresh E/15s.
Empty: missing dir → danger banner + `Relocate…` (deep 02
PP-02); no sessions yet → `Start first session` CTA.

**Processes-tab orphan contract (all selections, maintainer
zombie requirement, [expected-workflow.md](../../internal/expected-workflow.md#L17)):**
when an Instance ends (any state except Stopped-clean), its
descendant set is snapshotted; survivors after 10 s grace
appear in an `Orphaned — left behind (N)` section with PID,
name, RSS, age-since-parent-exit, and parent label. Sources:
`descendants()` ([supervisor.rs](../../../crates/juggler-supervisor/src/supervisor.rs#L341))
+ journal adoption ([supervisor.rs](../../../crates/juggler-supervisor/src/supervisor.rs#L435),
[journal.rs](../../../crates/juggler-supervisor/src/journal.rs#L151)).
Actions: `Verify owner` (re-check ppid chain, M), `Stop
process` (SIGTERM→SIGKILL ladder, [SPEC.md](../../SPEC.md#L225),
confirm popover naming the PID), `Stop all orphans`
(confirm listing all). Auto-cleanup is **opt-in per Profile,
off by default**, grace 60 s, and only ever touches PIDs the
journal recorded as descendants — never a name-based kill
(compliance: "no killing other applications",
[PROPOSAL.md](../../PROPOSAL.md#L47)).
Verification feedback: after any stop, row re-reads at 2 s
and shows `gone ✓` or `still alive — escalated to SIGKILL`;
failure keeps the row with error text. Empty: `No orphaned
processes` (a good empty state, cf. full review UX.3).

---

## 6. Quality & performance signals — honest measurability

No composite "health score". Each signal below stands alone
with its tier. If a Provider cannot supply it, the inspector
says so in words (§5 Performance empty rule).

| Signal | Definition (exact) | Measurable? | Source · refresh | Honesty label & failure mode |
|---|---|---|---|---|
| Tokens/min | Output+input tokens from transcript `usage` events in trailing 5 min ÷ 5 (Claude [SPEC.md](../../SPEC.md#L193), Codex `token_usage_record` [SPEC.md](../../SPEC.md#L202), ZCode `model_usage` [SPEC.md](../../SPEC.md#L210)) | **Yes, all 3 Providers** — but bursty: show as 1-min bars + 5-min average, not a single twitchy number | Transcript watch E, recomputed 15 s | `Measured` (Claude/ZCode), `Measured tokens` for Codex; idle → `0 tok/min` is real, distinguish from `no data` (no transcript yet) |
| Cost/min & session cost | Tokens/min × per-model pricing; session = Σ per-turn | **Partially** — needs a maintained pricing table that does not exist (full review F1.2); subscription plans make "$ cost" a *value-equivalent*, not a bill | Pricing table (new, versioned in-repo) 30 s | Always `Est · value-equiv.`; hide $ entirely where pricing unknown (I-06 rule) |
| Error rate | (failed turns + `api_retry`/tool-error events) ÷ total turns, trailing session or 1 h | **Partially, Provider-dependent:** Claude headless exposes `system/api_retry` ([SPEC.md](../../SPEC.md#L194)); interactive PTY and Codex/ZCode expose only transcript error patterns — parser-version-fragile ([SPEC.md](../../SPEC.md#L184)) | Transcript parse E, rollup 30 s | Show numerator/denominator (`2 errors / 41 turns`), never a bare %; `Not measurable (interactive)` where no structured feed |
| Stuck score | **Not a score.** Three named heuristics from [SPEC.md](../../SPEC.md#L220): (a) no output ∧ CPU < x% ∧ transcript mtime stale for N min; (b) waiting-on-permission > N min; (c) alive-but-no-progress. UI shows which rule tripped + its inputs (`no output 6m · CPU 0.3% · last write 6m ago`) | Heuristic only — thresholds user-tunable ([SPEC.md](../../SPEC.md#L353)); precision/recall target ≥90% on fixtures ([EPICS.md](../../plan/EPICS.md#L79)) is a *corpus* claim, not a per-session probability | Supervisor 15 s evaluation | Label `Stuck?` with `?` permanently (deep 02 §7); **refuse** a 0–100 score — it would fabricate precision the heuristic does not have |
| Context % | Tokens in current context ÷ model context limit | **Provider-dependent, often unknowable:** measurable only where transcript/statusline reports context tokens (Claude statusline JSON, [SPEC.md](../../SPEC.md#L192)); Codex/ZCode compact/summarise internally with no public gauge | Statusline E where present | Where absent: gauge replaced by `Context: not reported by {Provider}` + cumulative session tokens (labelled cumulative, not context). **Never estimate from cumulative tokens** — compaction makes that number wrong, not approximate |
| CPU/RAM (I-07) | Process-tree RSS Σ + CPU% (ppid-attributed, [SPEC.md](../../SPEC.md#L234)) | **Yes** via sysinfo staggered refresh ([SPEC.md](../../SPEC.md#L231)); overhead budget <1% CPU ([SPEC.md](../../SPEC.md#L24)) | 2 s sample | `Measured`; short-lived children may be missed between samples — footnote `2s sampling` on Processes tab |
| Quota forecast | `time-to-empty = % left ÷ current burn rate` ([SPEC.md](../../SPEC.md#L257)) | Yes where I-05+I-06 exist; meaningless at idle (÷0) | 30 s | Show range (`~30–50m at current rate`), tier-tagged; idle → `not burning` instead of ∞ |

External calibration: ccusage proves local-file token
accounting is viable across ~20 tools
([PROPOSAL.md](../../PROPOSAL.md#L123));
Opcode ships the cost-by-model dashboard users will compare
against ([github.com/winfunc/opcode](https://github.com/winfunc/opcode),
cited in deep 01 §8). Both set the bar at *labelled*
numbers — the differentiator is the honesty tier, not the
chart library (uPlot, [SPEC.md](../../SPEC.md#L83)).

---

## 7. ASCII wireframes — rail + inspector

### 7.1 Rail (Comfortable density, Needs-me sort, §4)

```
┌─ AI Juggler ──────────────── ⌘B ┐
│ [Projects][Instances][Sessions] │  tabs (deep 01 B-08 fix)
│ ⌕ search…            ⌘K  [Sort▾]│
│ (All)(! Needs me 2)(Running)▾   │  filter chips + counts §4.3
│─────────────────────────────────│
│ ◆Claude [WM] opus-4.8  ●Running │  I-01 I-02 I-03 I-04
│   fix-auth · ⎇main·3●  42m      │  I-09 I-11
│   Quota ██████░░ 78% left Meas  │  I-05 + tier
│   18.2k tok · $0.41 Est ·⑂4 ⧉2  │  I-06 I-08
│   ▁▃▅▂▅ 12%·1.1GB  active 10s   │  I-07 spark + text
│─────────────────────────────────│
│ ◆Claude [PS] opus-4.8 !Needs you│  I-10 badge (amber, text)
│   review-pr · ⎇feat/x  wait 4m  │
│   Quota ████░░░░ 55% · ▁▂▁ 3%   │
│─────────────────────────────────│
│ ⬢Codex [WM] gpt-5 ⛔Quota 17:00 │  I-10 danger + reset
│   spike · ⎇main  1h05m          │
│   Quota █░░░░░░░ 8% left Est    │
│ [＋ New session ⌘N]             │
│ A 22% · B 81% · !2   ? legend   │  footer: quota glance
└─────────────────────────────────┘  (deep 01 B-05 redesign)
```

Collapsed (Minimal): avatar discs with state glyph +
`!` badge only; tooltip carries the full I-01…I-11 text.

### 7.2 Right inspector — Instance, Overview tab (§5.1)

```
┌─ ● fix-auth [Running][terminal] ─────── [Focus ⌘⏎][⋯]┐
│ ◆Claude [WM]Work Max·Main·opus-4.8  my-app/ ⎇main·3● │
│ Overview│Usage│History│Performance│Processes│Actions │
│───────────────────────────────────────────────────────│
│ State Running 42m · active 10s ago   sess 9f3… ⧉     │
│ Quota (Work Max, 5h) ██████░░ 78% left · Meas        │
│   resets 17:00 (2h10m) · exhausts ~3h at current rate│
│ Tokens 18.2k (in 12k/out 6k) · $0.41 Est             │
│ CPU ▁▃▅▂ 12% · RAM 1.1 GB · ⑂4 children · ⧉2 tasks  │
│ Lineage: ← sess-41 (Personal)                        │
│ [Hand over… ⌘⇧H] [Stop ⌘.]        Updated 3s ago     │
└───────────────────────────────────────────────────────┘
```

### 7.3 Inspector — Processes tab with orphans (§5.1/§5.5)

```
│ Processes (5s)                         [Verify owner] │
│ PID    Process          CPU   RAM    Age   ⑂        │
│ 41201  claude           11%   890MB  42m    4        │
│ 41233  └ node (mcp)      1%    96MB  40m    0        │
│ 41302  └ rg (search)     4%   120MB   12s   0        │
│ ⚠ Orphaned — left behind (2)        since exit 18m  │
│ 39871  node dev-server   0%   210MB  18m  [Stop…]    │
│ 39890  └ vite            0%    88MB  18m  [Stop…]    │
│        [Stop all orphans…]  (confirm lists PIDs)     │
```

Acceptance for both wireframes: keyboard-only path
`⌘K → instance → inspector tab (⌘1…6 within inspector) →
action` without mouse; every number above reachable as
text (screen reader), no colour-only meaning (§4.1);
at Compact density the inspector is unchanged (density
is a rail-only control, §4.2).

*End of pass-2 §04. Siblings in `docs/review/pass2/` cover
onboarding, handover, and fleet against the same canonical
workflow; shared contracts: §1 terminology, §4 legend.*
