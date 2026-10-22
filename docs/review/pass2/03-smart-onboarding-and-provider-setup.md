# 03 — Smart Onboarding & Provider Setup (Second Pass)

> Second pass over the canonical maintainer workflow in
> [expected-workflow.md](../../internal/expected-workflow.md#L11)
> ("fully smart and automated onboarding wizard … scanning directories,
> existing providers … making recommendations"). This file does **not**
> restate the prior report; it cites it, then extends or corrects it.
> Prior sources: Maximum-Depth Report (cited as `MDR §…`),
> [full-competitive-ux-review.md](../full-competitive-ux-review.md)
> (cited as `FCUR F…`), and
> [03-ui-onboarding-handover-settings-audit.md](../deep/03-ui-onboarding-handover-settings-audit.md)
> (cited as `Deep-03 §…`). Every claim carries a repo `file:line`,
> a prior-report section, or an external URL.

## 0. Second-pass charter — what was under-weighted, wrong, or missed

The prior research audited the wizard that exists. The maintainer's
ask in [expected-workflow.md](../../internal/expected-workflow.md#L11)
is broader: onboarding for *first install, new project, add provider,
add account, add instance* as one near-zero-config intelligence.
Six gaps follow; the rest of this file designs the fix.

| # | Verdict | Prior position | Second-pass correction |
|---|---|---|---|
| P2-1 | Under-weighted | "Protect onboarding" (`FCUR F2.6`; `MDR` Executive #8) | Protect the *interaction*, not the *scope*: scope is Claude-only and must grow (§1) |
| P2-2 | Wrong by omission | Scan = 4 Claude origins (`Deep-03 §1.2`) | Scan misses shell aliases, installed CLIs, cc-switch DB, Codex/ZCode homes (§3) |
| P2-3 | Missed | No inference/confidence model anywhere | Found configs need labelled inference, not raw cards (§4) |
| P2-4 | Missed | Flows (b)–(f) undesigned | New-project / add-Provider / add-Account / add-Instance / import are distinct flows (§6) |
| P2-5 | Wrong framing | "Account" wizard, singular (`OnboardingWizard.tsx:79`) | Maintainer language is Providers / Accounts / Instances; domain says Harness/Profile/Session (§2) |
| P2-6 | Under-weighted | Login leak noted (`Deep-03 §3`) | Login *state taxonomy* (partial/expired/SSO/API-key) is the real design surface (§7) |

**P2-1 evidence.** The shipped wizard hard-codes one harness:
`const HARNESS = "claude-code"` at
[OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L18),
so the "extend, never fork" advice in `FCUR F2.6` is right about the
shell and wrong if read as "scope is done". SPEC already promises a
harness-generic scan at
[SPEC.md](../../SPEC.md#L414).

**P2-2 evidence.** `scan_claude_environment` assembles only vendor
default, `$CLAUDE_CONFIG_DIR`, manager roots, and `.claude*` home
matches at
[claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L56).
Nothing reads shell rc aliases — yet aliases are the dominant
community multi-account pattern per
[claude-code.md](../../research/claude-code.md#L33)
(canonical gist: https://gist.github.com/KMJ-007/0979814968722051620461ab2aa01bf2).

**P2-5 evidence.** The maintainer names the trio explicitly at
[expected-workflow.md](../../internal/expected-workflow.md#L11)
("we should call them providers, accounts, and instances of the
tool"). No `provider` symbol exists in `crates/` or `ui/src/`
(grep-verified this pass); §2 maps the vocabularies instead of
renaming storage blindly.

## 1. Terminology — adopt maintainer language, map, flag conflicts

### 1.1 Canonical mapping (UI language ←→ domain/storage)

| Maintainer term (UI) | Domain term today | Code anchor | Meaning |
|---|---|---|---|
| Provider | Harness / adapter | [model.rs](../../../crates/juggler-core/src/model.rs#L49) `HarnessId` | The tool vendor+CLI: Claude Code, Codex, ZCode |
| Account | Account | [model.rs](../../../crates/juggler-core/src/model.rs#L145) `Account` | One login you own under a Provider, isolated state dir |
| Instance (tool instance) | SessionRecord + InstanceKind | [model.rs](../../../crates/juggler-core/src/model.rs#L389), `:323` | One running/tracked occurrence (Terminal/Desktop) |
| *(no maintainer word)* | Profile | [model.rs](../../../crates/juggler-core/src/model.rs#L173) `Profile` | Account + settings overlay (model/MCP/env) |
| *(no maintainer word)* | Session (conversation) | ADR-0007 Decision 1 | The resumable work conversation inside an Instance |

ADR-0007 already moves product language toward Instances at
[0007-instance-centric-information-architecture.md](../../adr/0007-instance-centric-information-architecture.md#L38);
this pass completes the trio by promoting **Provider** to the UI
name for what code calls *harness*.

### 1.2 Naming conflicts — flagged, with rulings

1. **Provider ≠ Codex `model_provider`.** Codex config uses
   `model_providers.<id>` for API endpoints per
   [codex.md](../../research/codex.md#L18).
   Ruling: UI "Provider" always means the harness vendor; Codex
   endpoint config stays "model provider (advanced)" inside Profile
   settings, never shortened to Provider.
2. **Instance ≠ Fleet instance.** SPEC §11 uses "instance" for an
   AI Juggler node at
   [SPEC.md](../../SPEC.md#L368).
   Ruling: UI says **Tool Instance** vs **Machine**; code keeps
   `InstanceKind` for tools and `MachineId` at
   [model.rs](../../../crates/juggler-core/src/model.rs#L66)
   for fleet nodes. Never bare "instance" in fleet copy.
3. **Profile is invisible to the maintainer's trio.** The wizard
   silently creates a default `Main` profile (SPEC §12.1 step 3,
   [SPEC.md](../../SPEC.md#L423)).
   Ruling: keep Profile as an *advanced* concept — every Account
   gets `Main`; Profiles surface only under "Customize" (§6).
4. **Session vs Instance in storage.** `SessionRecord` *is* the
   instance record today (comment at
   [model.rs](../../../crates/juggler-core/src/model.rs#L386)).
   Ruling: rename in UI copy only this milestone; storage rename is
   a separate migration, explicitly out of onboarding scope.

## 2. Design principle — near-zero-config intelligence

The maintainer's bar is "it can't have thousands of options, it
needs to do most of the figuring out on its own" at
[expected-workflow.md](../../internal/expected-workflow.md#L11).
Operationalised as four rules the prior report never stated:

- **R1 Scan before ask.** No question the filesystem, env, or a
  metadata probe can answer is ever typed by the user. Today's
  wizard asks label+plan even on import
  ([OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L233));
  both become *confirmations* of inferred values (§4).
- **R2 One recommendation, visibly reasoned.** The wizard proposes
  a complete setup ("3 Claude accounts, 1 Codex — here's your
  juggling set") with per-item confidence; the user accepts,
  edits, or expands. This extends Deep-03's per-card fix
  (`Deep-03 §1.3`) into a set-level proposal.
- **R3 Progressive disclosure.** Default path ≤3 decisions;
  everything else (Profiles, MCP sync, env overlays) lives behind
  "Customize" / "Expert setup". Escape hatches in §6.7.
- **R4 Never credentials.** Scanning stays metadata-only per
  ADR-0006 Decision 1 at
  [0006-smart-onboarding-scan-and-import.md](../../adr/0006-smart-onboarding-scan-and-import.md#L28)
  and the probe contract at
  [environment.rs](../../../crates/juggler-adapter/src/environment.rs#L9).
  The one existing exception — byte-copying `.credentials.json`
  on import at
  [claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L224)
  vs SPEC §8 at
  [SPEC.md](../../SPEC.md#L310) —
  must be resolved per `FCUR F4.1` option (b) *before* the scan
  widens; a wider scan must not widen the credential surface.

## 3. Scan engine — what to probe (never credentials)

Today: one adapter, four origins, Claude-only
([claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L50),
trait default at
[adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L80)).
Target: a per-Provider probe table run in parallel, each row
producing `DiscoveredInstall`-shaped evidence
([environment.rs](../../../crates/juggler-adapter/src/environment.rs#L61))
plus new signal kinds. All probes are existence/size/mtime or
`--version`/`--help` execution — never file *contents* of any
credential file, never keychain *reads* (existence only, as in
[claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L332)).

| # | Signal | Probe (metadata only) | Yields | New? |
|---|---|---|---|---|
| S1 | Config dirs | Existing 4 origins per Provider (`~/.claude`, `~/.codex`, `~/.zcode` + env overrides + home patterns) | Candidate Accounts | Exists (Claude) |
| S2 | Env overrides | `$CLAUDE_CONFIG_DIR`, `$CODEX_HOME` in app env + shell rc export lines (name match only) | Manual/manager setups | Partial (`:58`) |
| S3 | Other managers: t3 | Children of `ClaudeCodeProfiles` ([claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L223)) | ManagerTool installs | Exists |
| S4 | Other managers: cc-switch | `~/.cc-switch/cc-switch.db` existence + provider-row *count* via SQLite metadata (no token columns) | Provider/Account inventory | **New** |
| S5 | Shell aliases/functions | Parse `~/.zshrc`, `~/.bashrc`, `~/.config/fish/*` for `CLAUDE_CONFIG_DIR=`/`CODEX_HOME=` assignments (variable *values* = paths only) | Forgotten manual accounts | **New** |
| S6 | Installed CLIs | `discover()` per adapter: PATH + known bins + `--version` ([claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L126)) | Provider availability + Step-0 gate | Exists, unwired in wizard (`Deep-03 §3.2`) |
| S7 | Login metadata | Per-dir: `.credentials.json` non-empty?, keychain-entry existence, `auth.json` presence for Codex ([codex.md](../../research/codex.md#L17)) | Login state per candidate | Exists (Claude) |
| S8 | Usage recency | Newest transcript mtime + session counts per dir (extends `newest_mtime_secs` at [claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L329)) | "Your active account" ranking | Partial |
| S9 | Projects | Recent cwds from transcript paths / `projects/` children names (dir names only) | New-project flow seeds (§6b) | **New** |
| S10 | Desktop harnesses | `/Applications/*.app` presence for desktop Providers (ADR-0007 Decision 5) | Desktop Instance candidates | **New** |

cc-switch as a scan source is grounded in `MDR` deep-06 §5
(cc-switch stores providers in `~/.cc-switch/cc-switch.db`:
https://github.com/farion1231/cc-switch) and the landscape table at
[landscape-and-stack.md](../../research/landscape-and-stack.md#L12).
Reading its DB is a *new read surface* requiring the same
metadata-only discipline: table/row counts and provider names,
never API keys it stores. If that line cannot be held in code
review, ship S4 as existence+count only.

**Scan contract additions** (trait-level, default-implemented so
adapters adopt incrementally, mirroring ADR-0006 Consequences at
[0006-smart-onboarding-scan-and-import.md](../../adr/0006-smart-onboarding-scan-and-import.md#L71)):

- `scan_shell_aliases() -> Vec<AliasHint>` — path-valued env
  assignments with source file + line; parse is regex-scoped to
  the two known variable names, not general shell evaluation.
- `scan_managers() -> Vec<ManagerHint>` — t3/cc-switch presence.
- Every probe gets a 2 s budget and a `ScanHealth` result
  (`Ok | Partial(reason) | Denied`) — fixing Deep-03 §1.2 finding 2
  ("scan failure is impossible by construction, therefore
  invisible") at the contract level, not just in copy.
- Scan runs off the UI thread with progress (`n of 10 checks`);
  today's unbounded spinner is `Deep-03 §1.1` row 1.

## 4. Inference rules + confidence display

The prior report shows found dirs as neutral cards
(`Deep-03 §1.1`); it never asks *what the app should conclude*.
Each candidate gets an inferred identity + a confidence tier,
rendered inline — never a bare path. Inference is rules-only
(no model call): onboarding must work offline and deterministically.

### 4.1 Rule table (evaluated in order; first match wins per field)

| Field | Rule | Example inference | Tier |
|---|---|---|---|
| Account purpose | Dir/alias name contains `work|corp|acme` → Work; `personal|home|side` → Personal; else basename humanised | `~/.claude-work` → "Work" | High / Med |
| Account purpose | cc-switch provider name or t3 profile folder name | Folder `acct-2` → keep verbatim, tier Low | Low |
| Plan tier | Settings/quota feed presence (Claude statusline history) or user-confirmed last time; **never** guessed from usage volume | No evidence → "Not sure yet" (today's default at [OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L21)) | — |
| Login state | S7 probe: credential file non-empty OR keychain entry exists → Signed in; file exists but empty/stale mtime → "May be expired" | Extends binary probe at [claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L332) | High / Med |
| Active vs dormant | Transcript mtime ≤7 d → Active; ≤90 d → Dormant; else Archive candidate | Uses S8 | High |
| Duplicate | Same canonical dir via 2+ signals (alias + home-pattern) → one candidate, signals merged | Dedupe exists for paths at [claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L83); extend to signal-merge | High |
| Already imported | Managed marker present ([claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L377)) → "Already in AI Juggler" | Fixes `Deep-03 §1.2` finding 1 in the model, not just copy | High |
| Project link | Transcript cwd set (S9) → "Used on: api, web-app" | Dir-name only | Med |

Confidence tiers render as words, not scores: **Found** (direct
probe), **Likely** (name/pattern inference), **Guessed** (weak
signal, always editable). Every inferred field is one-tap editable
inline; editing never re-runs the scan. Plan tier is deliberately
*not* inferred from token volume — quota honesty tiers
(Measured/Computed/Estimated at
[model.rs](../../../crates/juggler-core/src/model.rs#L288))
forbid manufacturing precision the probe cannot support. Note the
shipped plan list also omits Max 5× (`Deep-03 §2.1`,
[OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L20));
add it when plan becomes confirm-not-type (§6).

### 4.2 Confidence display — wireframe (Detect, proposed)

```
┌─ Welcome to AI Juggler ──────────────────────────────┐
│ We scanned your machine (nothing was changed).        │
│ Found 4 setups · 2 need a quick sign-in · Details ▾   │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Claude · ~/.claude            ● Signed in  Found  │ │
│ │ Likely: Work · Active this week · 3 projects      │ │
│ │ [ Import as “Work” ]  [ Edit ]                    │ │
│ └───────────────────────────────────────────────────┘ │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Claude · ~/.claude-side   ○ Not signed in  Found  │ │
│ │ Guessed: Personal · Dormant 40 d                  │ │
│ │ [ Import ]  [ Edit ]  [ Ignore ]                  │ │
│ └───────────────────────────────────────────────────┘ │
│ ┌───────────────────────────────────────────────────┐ │
│ │ Codex · ~/.codex              ● Signed in  Found  │ │
│ │ 1 account · CLI v0.44 installed                   │ │
│ │ [ Import ]  [ Edit ]                              │ │
│ └───────────────────────────────────────────────────┘ │
│ [ Use recommended setup → ]   Add manually · Expert…  │
└───────────────────────────────────────────────────────┘
  signals merged, inference labelled, managed/disabled
  states inline — supersedes Deep-03 §1.3 (per-card only)
```

## 5. Recommendation engine — from inventory to a juggling set

Input: the §4 inventory. Output: one proposed set the user can
accept in a single click. This is the piece the maintainer's
"making recommendations" clause at
[expected-workflow.md](../../internal/expected-workflow.md#L11)
demands and no prior section designs (closest: `MDR` Executive #5
needs-attention inbox — a *daily* surface, not an onboarding one).

**Rules (deterministic, explainable in one line each):**

1. **Import every Found+Signed-in candidate** as its own Account;
   default ON. Rationale shown: "Signed in — import keeps history".
2. **Import Dormant candidates default OFF**, listed under
   "Found but quiet" with one-tap include. Prevents clutter the
   maintainer warned against ("thousands of options", *ibid.*).
3. **One fresh Account per installed-but-unconfigured Provider**
   (S6 positive, S1 empty): proposed as "Add Codex (installed,
   not set up)" — never auto-created without the click.
4. **Label by inferred purpose; suffix collisions** (`Work`,
   `Work 2`) — fixes the duplicate-label hole at `Deep-03 §2.1`
   in the engine, not with a validation error after typing.
5. **First Project suggestion:** most-recent cwd (S9) becomes
   "Start here" on the Summary; the juggling premise needs a
   project, and `Deep-03 §4` notes the funnel ends without one.
6. **No recommendation ever crosses the automation line:**
   recommendations *stage*; creation/login execute only on the
   accept click (SPEC automation invariant at
   [SPEC.md](../../SPEC.md#L26)).

**Worked example** (the brief's case): scan finds 3 Claude
config dirs + 1 Codex home → proposal reads "Your juggling set:
Claude ×3 (Work — signed in; Personal — signed in; Side —
needs sign-in) + Codex ×1 (signed in). Import 3, sign in 1
later?" with `[ Accept setup ] [ Customise ]`. Accept executes
imports sequentially with per-item progress and the §8
verification checklist; failures degrade per-item (§7), never
abort the set. Post-accept, the Summary offers the two exits
Deep-03 §4 found missing: **Launch first Instance** and
**Add another Account** loop-back.

## 6. Flows (a)–(f) — button-by-button wizard specs

Shared shell for all six flows: modal wizard (keep today's
`fixed inset-0` shell at
[OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L109),
per `FCUR F2.6` "protect the interaction"), step dots, Back where
reversible, coded errors (never raw `to_string()` — the blocker
at `Deep-03 §0`), auto re-probe on terminal exit (picker P0-1,
`Deep-03 §9`). Click budgets count primary-path taps from entry
to Done, excluding vendor's own OAuth screens. "Expert" escape:
every flow exposes **Expert setup…** (raw state-dir path, env
overlay, Profile settings) — visible, never in the default path.

### (a) First install — target ≤5 clicks, 0 typed fields required

| Step | Controls (button-by-button) | Clicks |
|---|---|---|
| 0 Install gate | Per-Provider `discover()` rows; [Install Claude Code →] opens vendor install page; [Continue without it] per Provider | 0–1 |
| 1 Scan + proposal | §4.2 screen: [Use recommended setup →] primary; per-card [Import]/[Edit]/[Ignore]; [Add manually]; [Rescan] | 1 |
| 2 Confirm identity | Inferred labels/plans pre-filled; [Looks good →]; inline edit per row | 1 |
| 3 Sign-ins | Queued per Account needing login: [Sign in] (embedded terminal), [Open vendor page], [Copy command], [Skip for now]; auto-advance on probe success | 1 ea. |
| 4 Summary | [Launch first session →] primary; [Add another account]; [Done] | 1 |

Step 0 is new: today a missing `claude` binary fails only at
Login (`Deep-03 §3.2`, `Deep-03` E1). Gate copy uses the searched
paths from `discover()` at
[claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L134).
Zero-found case: proposal collapses to "Create your first
account" with Provider picker (installed first, others greyed
with install links) — never the dead-end dashed box at
[OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L159).

```
┌─ Your juggling set ──────────────── ●●○○ ─┐
│ Claude ×3 + Codex ×1 — ready to import     │
│  ✓ Work (Claude)      signed in            │
│  ✓ Personal (Claude)  signed in            │
│  ○ Side (Claude)      sign in later        │
│  ✓ Main (Codex)       signed in            │
│ Start project: ~/Dev/api (most recent) ▾   │
│ [ Accept setup → ]  [ Customise ]  Expert… │
└────────────────────────────────────────────┘
```

### (b) New project — target ≤3 clicks (the daily juggling entry)

Entry: Projects rail "New project" + Summary "Start here" (§5.5).
This flow is *absent* from prior onboarding scope — the wizard
ends at Account-ready (`Deep-03 §4`) while ADR-0007 makes Project
the juggling anchor at
[0007-instance-centric-information-architecture.md](../../adr/0007-instance-centric-information-architecture.md#L31).

| Step | Controls | Clicks |
|---|---|---|
| 1 Pick folder | Recent dirs (S9) as tiles; [Choose folder…] native picker; drag-drop | 1 |
| 2 Pick starting set | Recommended Account (last-used on this dir, else most headroom) pre-selected; [Change] opens Account list; [Start session →] | 1–2 |
| Done | Instance launches; project pinned with Account history row | — |

No Account yet → inline detour: "No account for Claude yet —
[Add account] (2 min)" launching flow (d) modally, then return.
Never typed paths (parity with NewSession recents at `FCUR F2.7`).

### (c) Add Provider — target ≤4 clicks

Entry: Accounts header [Add Provider]. Distinct from (d): a
Provider is a *tool*, not a login (maintainer trio, §1).

| Step | Controls | Clicks |
|---|---|---|
| 1 Pick Provider | Cards per adapter: Installed vX.Y (Found) / Not installed [Install →]; verified-range badge from `verified_against` ([adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L35)) | 1 |
| 2 First Account | Scan scoped to this Provider pre-run; [Import found] or [Add fresh]; label pre-inferred | 1–2 |
| 3 Sign in + Summary | Same login queue as (a) step 3; Summary with [Launch] / [Add another account on this Provider] | 1 |

Desktop Providers (S10) show kind badge Terminal/Desktop per
ADR-0007; a desktop Provider with single-instance limits is
labelled honestly before selection, not after a failed launch.

### (d) Add Account to an existing Provider — target ≤3 clicks

Entry: Provider card [Add account] + Summary loop-back. This is
today's wizard re-scoped: Provider locked (no harness picker —
fixing the hard-coded `HARNESS` at
[OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L18)
by *parameterising* it, not by adding a parallel wizard).

| Step | Controls | Clicks |
|---|---|---|
| 1 Source | Provider-scoped scan results; [Import this] per card; [Add fresh account] | 1 |
| 2 Identity | Label pre-inferred + collision-suffixed (§5.4); plan confirm (incl. Max 5×); [Create →] | 1 |
| 3 Sign in | Login queue (skip allowed); auto-advance on probe | 1 |
| Summary | [Launch] / [Add another] / [Done] | (1) |

Duplicate-import guard: a source already imported shows
"Already imported as «Work»" with [Open it] instead of [Import]
— requires import provenance on `Account` (field gap noted at
`Deep-03 §2.2`; `Account` today stores no source path at
[model.rs](../../../crates/juggler-core/src/model.rs#L145)).

### (e) Add Instance — target ≤2 clicks (launch, not setup)

Entry: Project panel "Launch with…" + Instances [New instance].
An Instance is a *run*, so this flow must feel like launching,
not onboarding — the prior report's silence here is why "add
instance" kept collapsing into "new session" copy.

| Step | Controls | Clicks |
|---|---|---|
| 1 Launch sheet | Project (pre-filled from context), Account rows with quota headroom + login state, Profile = Main (Customize ▸ reveals Profiles/model/MCP); [Launch →] | 1–2 |
| Gate | Admission advisor verdict OK/Tight/No inline (SPEC §5.2 at [SPEC.md](../../SPEC.md#L237)); Tight/No never blocks, always explains | — |

Logged-out Account rows are disabled with "Sign in first"
(one-tap detour into the login queue, then return). Quota
headroom uses honesty labels; unknown headroom renders
"headroom unknown", never a fabricated ring (`MDR` Executive #8;
today's `percent={0}` fabrication is `Deep-03 §7`).

### (f) Import from competitor / manual `CLAUDE_CONFIG_DIR` — ≤3 clicks

Entry: Detect card action + Expert "Import from folder…". Covers
three sources the current scan half-sees: cc-switch-managed
configs, t3 profiles (S3, exists), and hand-rolled
`CLAUDE_CONFIG_DIR` dirs found via aliases (S5, new).

| Step | Controls | Clicks |
|---|---|---|
| 1 Confirm source | Source path + origin badge (cc-switch / t3 / alias / manual); size + per-folder breakdown; [Import copy] / [Just track it*] | 1 |
| 2 Identity + sign-in | As (d) steps 2–3; keychain re-login note inline *here*, not discovered at Login (fixes `Deep-03 §2.3` caveat 2) | 1–2 |

\* "Just track it" (adopt-in-place, no copy) is **Expert-only and
off by default**: it breaks the managed-dir invariant (writes
stay outside `accounts/` at
[SPEC.md](../../SPEC.md#L400))
and the read-only-source proof (`Deep-03 §2.3`). If offered at
all, it needs its own ADR; default answer is copy-import via
`import_account` at
[adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L94).
Manual-folder import reuses the `is_claude_config_dir` evidence
check at
[claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L294)
(and Codex/ZCode equivalents) — a folder that fails it gets a
plain-language reason + [Pick a different folder], never a raw
OS string (`Deep-03` E6/E8).

### 6.7 Expert escape hatches (every flow)

[Expert setup…] reveals, without leaving the wizard: raw state-dir
path (read-only display + [Reveal in Finder]), env overlay editor
(`SettingsOverlay.env` at
[model.rs](../../../crates/juggler-core/src/model.rs#L161)),
Profile CRUD, "Import from folder…" (flow f), and a copyable
shell snippet reproducing the setup by hand (`CLAUDE_CONFIG_DIR=…
claude` — the community pattern at
[claude-code.md](../../research/claude-code.md#L33)).
Escape hatches are logged as onboarding events (§9) so the team
learns which inferences experts keep overriding.

## 7. Error & edge catalogue (onboarding-specific)

Extends `Deep-03 §5` (E1–E14, incorporated by reference — do not
re-implement differently); rows below are new or re-scoped by
the wider scan. Every row: detection → user copy → recovery
button. No row may surface a raw backend string (blocker,
`Deep-03 §0`).

| # | Case | Detection | Recovery in-flow |
|---|---|---|---|
| N1 | Partial login (OAuth started, abandoned) | PTY exited, probe still LoggedOut | "Sign-in didn't finish" + [Reopen sign-in] [Skip] (extends E11) |
| N2 | Expired login | Credential present, stale/empty (S7 Med tier) | "May be expired — sign in again" queued, Account still created |
| N3 | Duplicate account (same source twice) | Provenance match (§6d) | Block second import; [Open existing] |
| N4 | Corporate SSO | Vendor flow redirects to IdP; probe lags | Copy: "Finish in your browser/IdP window, then return"; poll probe 2 s while step visible |
| N5 | API-key vs subscription | `auth.json`/env-key presence without OAuth markers (Codex, [codex.md](../../research/codex.md#L18)) | Badge "API key account" — quota shows Estimated/none, never subscription rings |
| N6 | Two logins, one identity | Two dirs probe Signed-in with same plan+recency pattern | Warn (non-blocking): "These may be the same login twice — juggling needs two *subscriptions*" (`FCUR` Persona B wrong-account risk) |
| N7 | cc-switch DB locked/corrupt | SQLite open fails (S4) | Degrade S4 to existence-only; note in Scan Details, never fail scan |
| N8 | Alias points at deleted dir | Canonicalize fails (as [claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L86)) | Show greyed "alias → missing folder" with [Ignore]; do not hide silently |
| N9 | Provider installed, unsupported version | `discover()` version ∉ `verified_against` | "Installed (vX, unverified)" — allow Account, badge Instances until verified (SPEC §4 drift rule at [SPEC.md](../../SPEC.md#L184)) |
| N10 | Import > ceiling with breakdown | Per-folder sizes pre-copy (fixes E7) | [Import settings only — skip history] degraded import; [Add fresh] |
| N11 | Offline mid-onboarding | Login web/PTY fails + scan already done | Accounts persist; Summary marks "sign in when online"; no re-scan on reconnect |
| N12 | Scan permission-denied (home subdir) | `ScanHealth::Denied` per probe (§3) | "We couldn't check 2 places" + Details; empty-result copy forbidden (fixes `Deep-03 §1.2` #2) |

N6 deserves emphasis: the maintainer's core case is *multiple
subscriptions from the same lab*
([expected-workflow.md](../../internal/expected-workflow.md#L5));
two dirs sharing one login silently defeats quota juggling, and
no prior section names it. Detection stays heuristic + honest
("may be"), never a block.

## 8. Verification checklist — proving isolation (per Account)

Run automatically after every import/create; results render on
the Summary as pass/fail lines, and any fail blocks "Launch"
(not "Done"). This operationalises the M1 acceptance "env +
keychain isolation verified" in
[EPICS.md](../../plan/EPICS.md#L63)
and the guardrail suite `FCUR F4.2` demands.

- [ ] Managed dir exists, perms 0700, marker present
  (`prepare_managed_dir` contract, ADR-0006 Decision 2).
- [ ] Launch env points at the managed dir only
  (`CLAUDE_CONFIG_DIR`/`CODEX_HOME`/`HOME` per SPEC table at
  [SPEC.md](../../SPEC.md#L130)).
- [ ] Source dir tree hash/mtime snapshot unchanged (import
  only; proof pattern in `Deep-03 §2.3`).
- [ ] Login probe scoped to *this* dir answers independently —
  signing out Account B must not flip Account A's chip.
- [ ] Two Instances (one per Account, same Provider) run
  concurrently; each transcript lands only in its own dir.
- [ ] Zero writes outside managed dirs during the whole flow
  (fixture-HOME guardrail test, `FCUR F4.2`).
- [ ] Credential contents never logged/telemetry-tagged;
  `.credentials.json` handling matches the resolved F4.1 rule.

## 9. Onboarding metrics (instrument from day one)

Events (local-first; telemetry opt-in per SPEC §8): `scan_started/
completed{providers,found,health}`, `proposal_shown/accepted/
customised`, `import_started/completed/failed{reason_code}`,
`login_queued/completed/skipped/expired`, `expert_opened`,
`first_launch{seconds_since_install}`. Targets, tied to the
success metric "install → two accounts in parallel < 5 min" at
[PROPOSAL.md](../../PROPOSAL.md#L142):
time-to-first-Instance (p50 < 3 min on found-set machines),
proposal-accept rate, per-step drop-off (esp. Login queue),
inference-override rate per field (drives §4 rule tuning),
scan-health Denied rate. Report funnel weekly during beta;
a step losing >15% gets a Deep-03-style audit before new features.

## 10. Phasing (smallest shippable slices)

1. **Parameterise + gate:** wizard takes a Provider param (kills
   the constant at
   [OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L18));
   add Step-0 install gate + coded errors. Ships with Claude only.
2. **Infer + propose:** §4 rules + §5 set proposal for Claude;
   Summary gains [Launch] / [Add another]. No new signals yet.
3. **Widen scan:** S5 aliases + S9 projects, then S4 cc-switch
   (existence-only first); Codex/ZCode `scan_environment` impls
   replace trait defaults at
   [adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L80).
4. **Flows (b)/(e) + verification:** new-project and launch-sheet
   entries; §8 checklist automated on Summary.

*Unresolved for implementers:* live cc-switch DB schema version
must be re-verified before S4 reads beyond existence; ZCode
per-account login probe needs the HOME-isolation experiment
re-run per [zcode.md](../../research/zcode.md#L72);
adopt-in-place (flow f \*) requires its own ADR or stays cut.
