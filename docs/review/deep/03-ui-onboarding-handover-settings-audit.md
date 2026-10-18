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
  ([OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L108)).
  Dismissal is persisted as `aij.onboarding.dismissed`
  ([App.tsx](../../../ui/src/App.tsx#L94)).
  There is no "resume where I left off": closing mid-import discards
  `label`/`plan` state because the component unmounts.
- Auto-open fires **once per dismissal**, not once per zero-account
  state: if the user closes with zero accounts and relaunches, the
  wizard does not reappear (`App.tsx:94-99`). The only re-entry is
  Accounts → Add account ([Profiles.tsx](../../../ui/src/screens/Profiles.tsx#L89)).
  For a zero-knowledge user this is a silent dead end.
- `HARNESS` is hard-coded to `"claude-code"`
  ([OnboardingWizard.tsx](../../../ui/src/screens/OnboardingWizard.tsx#L18)).
  SPEC §12.1 and ADR-0006 describe a harness-generic scan
  ([docs/SPEC.md](../../SPEC.md#L414));
  the wizard cannot onboard Codex/ZCode even when adapters land.
- Every backend call in the funnel returns a **bare string error**
  (`AppError` serializes via `to_string()`,
  [error.rs](../../../crates/juggler-app/src/error.rs#L52)).
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
([commands.rs](../../../crates/juggler-app/src/commands.rs#L347)).
The Claude implementation assembles candidates from four origins —
vendor default `~/.claude`, `$CLAUDE_CONFIG_DIR` override, t3.codes
`ClaudeCodeProfiles` children, and home `​.claude*` pattern matches —
then canonicalizes, dedupes, filters by `is_claude_config_dir`
(any of `.credentials.json`, `settings.json`, `projects`, …),
and sorts default → logged-in → most-recent
([claude_environment.rs](../../../crates/juggler-adapters/src/claude_environment.rs#L50)).
Three audit findings the prior review did not surface:

1. **The `managed` flag is computed and then ignored by the UI.**
   `DiscoveredInfo.managed` is populated
   ([commands.rs](../../../crates/juggler-app/src/commands.rs#L200))
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
   ([claude_code.rs](../../../crates/juggler-adapters/src/claude_code.rs#L230)).
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
([commands.rs](../../../crates/juggler-app/src/commands.rs#L392));
fresh → `createAccount` → `create_account` (`commands.rs:280-302`).
Both create the account row **and** a default `Main` profile inside
the same storage lock (`commands.rs:413-425`), fulfilling SPEC
§12.1 step 3 ("immediately launchable",
[docs/SPEC.md](../../SPEC.md#L423)).
Findings beyond the prior review:

- **Duplicate import of the same source succeeds silently.** The
  onboarding test asserts re-import works and only managed-dir
  import is refused
  ([onboarding.rs](../../../crates/juggler-app/tests/onboarding.rs#L128)).
  There is no source-path dedupe: importing `~/.claude` twice yields
  two managed accounts with identical history and (by default) the
  same basename-derived label. Downstream, handover chips and
  NewSession rows become ambiguous. The wizard needs a Detect-level
  "already imported as «Work»" badge, which requires the backend to
  remember import provenance — today `Account` stores no source path
  ([model.rs](../../../crates/juggler-core/src/model.rs#L93)).
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
  ([docs/SPEC.md](../../SPEC.md#L422))
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
   stored" ([docs/SPEC.md](../../SPEC.md#L310)),
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

Entry is up to six profile chips under "Continue on another account" ([WorkPanels.tsx](../../../ui/src/screens/WorkPanels.tsx#L139)); logged-out targets disabled ("· sign in first", `:149-172`), ready targets fire immediately — no confirmation or preview — via `sessionHandover` ([bridge.ts](../../../ui/src/lib/bridge.ts#L460)) → `session_handover_impl` ([commands.rs](../../../crates/juggler-app/src/commands.rs#L1088)). Footer claims "transcript copied, then resumed" (`WorkPanels.tsx:184`) with no tier/size/loss disclosure; failure is only `window.alert` ([App.tsx](../../../ui/src/App.tsx#L371)).

### 6.2 Backend truth vs. SPEC §7 promise

SPEC requires: targets listed **with fidelity tier + quota headroom**,
freeze source, lineage, source remains resumable
([docs/SPEC.md](../../SPEC.md#L287)).
Code does: newest-JSONL-by-mtime lookup ignoring the session's own
`transcript_ref` (`commands.rs:1120-1133` vs.
[model.rs](../../../crates/juggler-core/src/model.rs#L415)),
cwd mangled by `replace('/','-')` (`commands.rs:1121`) instead of the
transcript's recorded `cwd` (the unambiguous decode the same file
already implements at `commands.rs:1048-1060`), single-file
`std::fs::copy` (`commands.rs:1142-1146`) while Claude is potentially
still appending (no freeze/stop, no torn-line check), then
`--resume <id>` (`claude_code.rs:294-296`) with no collision handling
despite SPEC's `--fork-session`/rewrite requirement
([docs/SPEC.md](../../SPEC.md#L280)).
Lineage is one status flip to `HandedOver` (`commands.rs:1166-1169`)
plus a continuation row (`:1171-1187`) — no pack, no A→B→A chain
([docs/SPEC.md](../../SPEC.md#L302)),
no return path. Capabilities already over-advertise:
`TRANSPLANT|FORK` are declared (`claude_code.rs:111-120`) while
`crates/juggler-handover/src/lib.rs:1-13` contains zero functions
and the adapter trait has no export/prime methods
([adapter.rs](../../../crates/juggler-adapter/src/adapter.rs#L26)).
Concrete failure modes: wrong-session pick under parallel activity,
torn final JSONL line, dotted/long-cwd mangle miss, target-ID
collision, silent sibling loss (separate subagent files, file-history,
todos — inline `isSidechain` lines do survive inside the main file,
prior F3.4). The golden-path test proves only the stub happy path
([golden_path.rs](../../../crates/juggler-app/tests/golden_path.rs#L93)).

### 6.3 Proposed multi-step wizard (replaces chips; SPEC §7.2 takeover)

Steps: **Source → Target → Fidelity preview → Redaction → Diff/confirm
→ Progress → Done (lineage + rollback)**. Every step is reversible
until Confirm; execution stays one explicit click (automation policy,
[docs/SPEC.md](../../SPEC.md#L263)).

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
([docs/SPEC.md](../../SPEC.md#L292)).
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
([App.tsx](../../../ui/src/App.tsx#L44));
**Settings does not exist at all** — no nav entry, screen, or route.

| Surface | Today (code) | SPEC target (§9.2) | Gap per control |
|---|---|---|---|
| Usage | `ComingSoon` card, zero controls (`App.tsx:338-341,525-539`) | Per-account timelines, 5h/weekly/credit windows, per-session breakdown, CSV export (`docs/SPEC.md:349`) | No chart, window selector, account filter, honesty-tier label, export, or empty-state guidance; nav click is a dead end that trains users to stop exploring (prior UX.7) |
| Fleet | Same `ComingSoon` (`App.tsx:340-341`) | Machine list (state/resources/permissions), filters across board/overview/usage, pairing wizard, cross-machine handover (`docs/SPEC.md:350,387-389`) | No machine card, pairing code/QR, permission toggle, or filter; Overview hardcodes Fleet=1 (`App.tsx:468-473`) |
| Overview quota | `QuotaRing percent={0}` ×2 (`App.tsx:513-514`) + copy admitting M2 deferral (`:516-518`) | Rings with reset countdowns per account (`docs/SPEC.md:345`) | Fabricated 0% reads as "no usage"; needs explicit "No data yet" state (prior F2.4). Ring itself is sound (`primitives.tsx:165-218`) but has no % text, tier label, or countdown |
| Overview machine | Hardcoded "OK / nominal (live gauges: M3)" (`App.tsx:461-467`); rail footer "this-mac · healthy" (`App.tsx:282-290`) | CPU/RAM/swap/disk gauges (`docs/SPEC.md:345`) | Static success claim with no sensor behind it — worst placeholder pattern in the app |
| Settings | **Absent** (grep: no Settings screen in `ui/src`) | Stuck thresholds, alert levels, terminal prefs, update channel, telemetry off (`docs/SPEC.md:353`) | Every specced control missing: threshold sliders, alert toggles, font/scrollback, channel picker, telemetry switch, plus unspecced-but-needed: storage size, retention, backup/restore (prior F1.3/F1.7) |
| Tray / hotkey | Absent in `ui/src` | Aggregate quota, alerts, quick-launch, global hotkey (`docs/SPEC.md:352`) | No glance surface; daily loop requires full window (prior F7.2) |

Recommendation: remove Usage/Fleet from nav until each has one real control, ship Settings (thresholds + terminal + updates + backup) before M3, and never render a numeric ring/gauge without data + confidence label ([docs/SPEC.md](../../SPEC.md#L259)).

---

## 8. Modal, toast & error system

There is **no toast/banner system**. Channels today: inline `text-danger` strings (wizard `OnboardingWizard.tsx:266`, NewSession `NewSession.tsx:211`), card banners (Profiles `Profiles.tsx:95-99`), and `window.alert` ×3 (`App.tsx:371,384`; `Profiles.tsx:367`) — modal, unstyled, unrecoverable. `refresh()` swallows errors and keeps stale data (`App.tsx:101-116`, prior F2.1). Modals (`form.tsx:54-97`, wizard, `NewSession.tsx:77-80`) handle Escape inconsistently and lack `role="dialog"`/focus-trap/return-focus (prior UX.8). Required: one status model (`loading|ready|degraded|error` + Retry), one non-blocking toast, deletion of every `window.alert`, and coded errors beyond `to_string()` (`error.rs:52-60`).

## 9. Fix list (priority; acceptance in italics)

**P0:** (1) Auto re-probe login on PTY exit + poll; wire wizard to `onTerminalExit` (*login completes without finding Re-check*). (2) Step-0 harness/install gate (*no-binary machine never reaches dead Login*). (3) Handover: `transcript_ref`, freeze, sibling copy, verify, lineage (*golden-path gains subagent fixtures*). (4) Remove fabricated Overview numbers. **P1:** Detect badges + full notes; partial import; label guard; Summary Launch/Add-another; ReadyHome sticky skip; handover wizard §6.3 (T1 first); nav gating; Settings v1; toast system. **P2:** Max 5×, offline copy, wizard resume, redaction, fleet-ready lineage.

## 10. Competitor check (bounded)

Orchestrators delegate accounts/quota to CLIs — [T3 Code](https://t3.codes/), [Opcode](https://github.com/winfunc/opcode), [Claude Squad](https://github.com/smtg-ai/claude-squad), [Conductor](https://www.conductor.build/docs/concepts/workflow), [Vibe Kanban](https://github.com/BloopAI/vibe-kanban) — while switchers own quota without sessions: [cc-switch](https://github.com/farion1231/cc-switch) (tray quota, one-click switch), [claude-swap](https://github.com/realiti4/claude-swap) (rotation/dashboard). Onboarding already beats orchestrator first-run; the gap is switcher table-stakes (tray glance, headroom at decision time) — hence §6.3/§7 outrank Usage charts. *Unresolved: live fidelity on real Claude 2.1.296 (cross-dir `--resume`, dotted cwds, MCP) needs dogfood; contrast/focus unverified without browser/screen-reader pass.*
