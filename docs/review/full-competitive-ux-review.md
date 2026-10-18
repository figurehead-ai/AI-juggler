# AI Juggler — Full Competitive & UX Review
> Independent second-opinion review. Synthesis of eleven parallel
> evidence sweeps: vision/plan audit, built-UI audit, engine/handover
> audit, persona research, and five competitor profiles.
> Date: 2026-10-12. Branch: `main`.
> Method: read-only code/docs inspection + competitor web research.
> No runtime, browser, or screen-reader testing was performed —
> see Appendix A for what that means for confidence.
>
> Priorities: **P0** blocks the core promise or risks user data/trust.
> **P1** must land before v0.1-beta. **P2** later, but plan for it now.
---

## Executive summary
- **The roadmap is inverted against the product's own promise.**
  Juggling (PROPOSAL G2/G3) means curing quota-blindness first, yet
  quota visibility sits at M3 after M2 ships two more adapters
  (SPEC.md:452-454). Every week without quota, the app is a launcher,
  not a juggler.
- **Handover v0 shipped before its fidelity was proven.** It byte-copies
  one newest-by-mtime JSONL and calls `--resume <id>`; sibling subagent
  transcripts, file-history, and todos are silently left behind, and no
  test runs a real Claude binary (see Q3). Calling this "handover"
  without a fidelity label will burn first-user trust.
- **The repo no longer tells the truth about itself.** README.md:7 says
  "spec phase / no releases yet" while CHANGELOG.md:335-341 records a
  shipped, working handover v0 and a Claude adapter. QUALITY.md:9
  demands the repo tell the truth; right now it contradicts itself.
- **The daily loop's most-needed signal does not exist in code.**
  `Stuck`/`WaitingUser`/`QuotaBlocked` are modelled (model.rs:188-207)
  but never emitted (commands.rs:782-810). At 15 parallel sessions the
  user cannot answer "which one needs me?" — the #1 daily question.
- **Instances and Sessions are the same entity wearing two labels**
  (WorkflowRail.tsx:295-326; WorkPanels.tsx:44-189), contradicting
  SPEC.md:324-337. The rail will confuse exactly the power users it
  targets.
- **Failures are invisible.** `refresh()` swallows errors and keeps
  stale data (App.tsx:101-116); other errors use `window.alert`
  (App.tsx:371,384). A supervision tool that fails silently is worse
  than no tool.
- **One guardrail crack needs a security decision, not a docs note.**
  Import byte-copies `.credentials.json` into the managed dir
  (claude_environment.rs:209-232) against SPEC S8's "never copied"
  wording. Either the spec or the code is wrong; pick one, in writing.
- **No orchestrator competitor does account juggling well — and that
  is the opening.** T3, Opcode, Claude Squad, Conductor, and Vibe
  Kanban all delegate quota/accounts to the underlying CLIs. The
  juggling layer (cc-switch, claude-swap, ccusage) is a separate,
  cruder toolset. Owning both in one app is a real, unclaimed niche.
- **Two category leaders just died.** Vibe Kanban is sunsetting after
  Bloop's 2026-04-10 shutdown; Crystal was deprecated Feb 2026. Their
  users are actively looking for a home. The migration window is now.
- **The onboarding flow is genuinely the best-built surface** in the
  app (OnboardingWizard.tsx) and beats most competitors' first-run.
  Protect it; do not let roadmap churn regress it.
---


## Q1 — Missing features vs. the vision
The plans are unusually thorough on the happy path and thin on
everything that happens on day 30: growth, decay, cost, and recovery.

### F1.1 [P0] No git-worktree or branch isolation for parallel accounts
- **Evidence:** SPEC.md:113 gives a session only a `cwd`. PROPOSAL.md:75
  promises parallel accounts on the same repo. Every serious parallel
  tool isolates by worktree: Claude Squad (1 task = 1 worktree+branch),
  Vibe Kanban (worktree per task-attempt), Conductor (worktree per
  workspace). Juggler has no branch concept outside a pack field
  (SPEC.md:301).
- **Recommendation:** Add an optional per-session worktree at launch:
  `git worktree add` into a managed dir, branch named
  `juggler/<profile>/<session-short>`. Show branch + dirty state on
  every session row. Warn (do not block) when two live sessions share
  a branch. This is cheaper than any M2 adapter and protects G1.

### F1.2 [P1] No USD/cost view anywhere
- **Evidence:** SPEC.md:121-125,256-259 model tokens/credits only; no
  pricing table, no per-project or per-account budget. cc-switch syncs
  models.dev pricing; ccusage produces daily/monthly/5h-block cost
  reports from the same JSONL Juggler already parses.
- **Recommendation:** Ship a cost ledger with M3 quota: tokens →
  estimated USD using a versioned pricing table, labelled with the
  same Measured/Computed/Estimated honesty tiers (SPEC.md:254-259).
  Per-session, per-account, per-project rollups + CSV export.

### F1.3 [P1] No session lifecycle: archive, delete, search, retention
- **Evidence:** SPEC.md:395-405 (storage) and SPEC.md:348-349 (Sessions
  UI) contain no archive/delete/search/tag/retention/GC. Transcripts
  plus ZCode logs grow ~4MB/day (SPEC.md:233); Claude's own 30-day
  cleanup (research/claude-code.md:24) is unmanaged; Codex users
  explicitly demand session delete (research/codex.md:80).
- **Recommendation:** Add lifecycle states (active → archived →
  deleted), full-text search over session titles/transcripts, and a
  retention setting (default: archive after 30d, never auto-delete
  without opt-in). Surface storage size in Settings.

### F1.4 [P1] No account-health beyond a presence probe
- **Evidence:** SPEC.md:132 detects login presence only. No OAuth
  expiry, plan-downgrade, or wrong-tier detection — failure modes that
  are documented in the wild (research/claude-code.md:36, issues
  #87447 wrong billing, #94195 wrong tier). No per-project
  account-affinity guard for Persona B (PROPOSAL.md:52).
- **Recommendation:** Extend `login_state` (already metadata-only,
  claude_code.rs:332-344 — good) with plan/tier readback where the
  vendor exposes it, expiry countdown, and a launch-time warning when
  a project's usual account differs from the selected one.

### F1.5 [P1] No reset-queue or scheduled continuation
- **Evidence:** SPEC.md:288 — "return" on reset is only a suggestion.
  No "notify + one-click resume queued task at reset," no pre-flight
  exhaustion block beyond the admission advisor. Claude Code already
  auto-waits (research/claude-code.md:53); Codex users demand the
  equivalent (research/codex.md:78, #21073).
- **Recommendation:** A reset queue: on `QuotaBlocked`, offer "Queue
  resume at reset (≈17:00)" — a passive timer + notification, with
  the resume itself still one explicit click (preserves the
  no-automation invariant). Add a pre-flight hard-stop option when
  admission = No.

### F1.6 [P1] Pack/handover redaction is unspecified
- **Evidence:** SPEC.md:296-304 — packs carry conversation + diffs
  verbatim. Redaction exists only for diagnostics (SPEC.md:314).
  Transcripts routinely contain secrets and auth headers
  (research/zcode.md:64).
- **Recommendation:** A redaction pass on pack export: pattern-based
  secret scanning (keys, bearer tokens, `.env` values) with a preview
  diff the user approves. Default on for export, off for local
  same-machine handover.

### F1.7 [P1] No backup/restore for Juggler's own database
- **Evidence:** SPEC.md:399 — one `juggler.sqlite` (WAL). SPEC.md:22
  promises atomic writes only. No export, backup, or corrupt-DB
  restore UX. That file holds lineage (SPEC.md:302) and usage history.
- **Recommendation:** Automatic rolling backups (last 7, on clean
  exit + daily), one-click export/import, and a "database damaged —
  restore from backup?" recovery screen instead of a blank start.

### F1.8 [P2] Extensibility is deferred too thinly
- **Evidence:** SPEC.md:140 — adapter manifest is post-v1. No adapter
  SDK or conformance kit until EPICS.md:153 (M8-3). Desktop-class
  harnesses are promised (adr/0007:59-63, SPEC.md:336) with no
  milestone or task. T3 already ships 6+ harnesses; cc-switch covers
  10 CLIs.
- **Recommendation:** Publish the fake-harness conformance suite
  (already in-tree) as the adapter contract test in M2, even before a
  public SDK. Document the trait as the de-facto API.

### F1.9 [P2] No settings/MCP/rules portability across accounts
- **Evidence:** Only project sync (EPICS.md:108, M4-6) exists. No
  profile templates/clone, no MCP-server sync, no global-hotkey or
  tray quick-switch parity with aisw/`codex-profiles`
  (research/landscape.md:19-20, research/codex.md:33). SPEC.md:352's
  tray is quota-only.
- **Recommendation:** Profile clone + selective sync (MCP servers,
  rules files, model defaults) with per-item checkboxes. Tray gains
  a quick-switch section mirroring cc-switch's one-row switcher.
---

## Q2 — Workflow-level critique (the daily loop)
### F2.1 [P0] No app-level loading or error state
- **Evidence:** `refresh()` silently swallows errors and keeps stale
  data on screen (App.tsx:101-116). First render flashes `WorkEmpty`
  before data arrives. Elsewhere errors surface as `window.alert`
  (App.tsx:371,384; Profiles.tsx:367) — modal, unrecoverable, unstyled.
- **Recommendation:** Introduce one app-level status model:
  `loading | ready | degraded(reason) | error(retry)`. Degraded shows
  a non-blocking banner with Retry; never render empty-states until
  the first fetch resolves. Replace every `window.alert` with the
  same banner/toast system.

### F2.2 [P1] Instances ≠ Sessions is conflated in the built UI
- **Evidence:** SPEC.md:324-337 defines Instances and Sessions as
  distinct entities. In code, the instance list is sessions filtered
  live/recent, rendered by the same `SessionRow`/panel
  (WorkflowRail.tsx:295-326; WorkPanels.tsx:44-189). There is no
  instance entity with resources or per-instance token stats.
- **Recommendation:** Either implement the SPEC distinction (instance
  = running harness process with resource/token surface) or delete
  the Instances tab and say so in SPEC. The honest smaller move:
  merge to Projects/Sessions now, reintroduce Instances when the
  resource manager (M2) gives it real content.

### F2.3 [P1] Missing SPEC S9.2 session actions
- **Evidence:** No explicit Resume/Relaunch on ended sessions — only
  Stop while live (WorkPanels.tsx:100-112). Handover is inline buttons
  without the fidelity-tier display or quota-headroom check that
  SPEC.md:287,347 require at takeover (WorkPanels.tsx:139-187).
- **Recommendation:** Every ended-session row gets Resume and Relaunch.
  Handover becomes a small wizard (see §UX.5): tier, target headroom,
  redaction preview, then one confirm click.

### F2.4 [P1] Global IA contains dead ends with fabricated data
- **Evidence:** Usage and Fleet are `ComingSoon` placeholders
  (App.tsx:338-341,525-539). Overview quota rings are hardcoded
  `percent={0}` (App.tsx:461-473,513-514); "Machine OK / Fleet 1" are
  hardcoded (App.tsx:510-519). SPEC.md:345,349-350 promise real data.
- **Recommendation:** Until real data lands, rings render an explicit
  "No data yet — quota arrives in M3" empty state, never a numeric
  ring. ComingSoon screens state the milestone and what will appear.

### F2.5 [P1] The needs-attention signal is modelled but never emitted
- **Evidence:** `SessionStatus::{Stuck, WaitingUser, QuotaBlocked}`
  exist (model.rs:188-207) but `apply_lifecycle_event` only ever sets
  Running/Stopped/Finished/Failed (commands.rs:782-810). The rail's
  status tones lack stuck/waiting/quota-blocked chips; the rail caps
  at `slice(0,15)` sessions / `slice(0,8)` recents with no search or
  filter (WorkflowRail.tsx:184-195,328-384).
- **Recommendation:** Ship stuck detection v1 (already M2-5 in
  EPICS.md:79) early and wire the three statuses end-to-end. Add a
  Needs-Attention inbox (§UX.3) sorted by waiting-time, with a count
  badge on the rail and tray.

### F2.6 [P2] Onboarding is the best-realized flow — protect it
- **Evidence:** Auto-opens at zero accounts (App.tsx:92-99); auto-scan,
  read-only import, fresh/plain identity, vendor login, and summary
  all present (OnboardingWizard.tsx:62-71,141-214,270-381) against
  SPEC §12.1. No competitor's first-run is materially better.
- **Recommendation:** Freeze its interaction model; cover it with the
  first UI regression tests the project writes. Any M2 onboarding
  change (Codex/ZCode) extends the same wizard, never a second one.

### F2.7 [P2] Daily launch is strong, with one papercut
- **Evidence:** ReadyHome one-click sign-in gate (ReadyHome.tsx:20-80;
  App.tsx:194-198); NewSession offers recents + native picker +
  drag-drop + sign-in gating with zero typed paths
  (NewSession.tsx:49-57,117-163,170-199; App.tsx:124-130). But
  ReadyHome re-blocks every launch until a session-skip is set.
- **Recommendation:** Make the skip sticky per profile ("don't ask
  again for this account") while keeping the gate for profiles whose
  login state is actually unknown or expired (ties to F1.4).

### F2.8 [P2] Dead code and render side-effects in the panel layer
- **Evidence:** An unused second `NewSessionModal`
  (WorkPanels.tsx:354-443) duplicates `screens/NewSession.tsx`;
  profiles are fetched during render via `if(null) void fetch`
  (WorkPanels.tsx:209-212), risking render loops.
- **Recommendation:** Delete the dead modal; move the fetch into the
  same effect that owns session loading. Small, but this file is the
  daily-loop surface — keep it boring.
---

## Q3 — Handover realism
### F3.1 [P0] T3/T4 are advertised but do not exist
- **Evidence:** `crates/juggler-handover/src/lib.rs:1-13` is doc-only —
  zero functions. The adapter trait (adapter.rs:26-124) has no
  export/transplant/prime methods. Yet the Claude adapter advertises
  `TRANSPLANT|FORK|RESUME` (claude_code.rs:111-120; fake.rs:49-56;
  capabilities.rs:27-28). T3/T4 exist only in SPEC §7. There is no T1
  UI either.
- **Recommendation:** Downgrade advertised capabilities to what is
  implemented today (`RESUME` only), and gate `TRANSPLANT` behind the
  fidelity test in F3.4. Move the real transplant logic into
  `juggler-handover` so the crate stops being documentation.

### F3.2 [P0] Handover v0 shipped pre-M2 with unproven fidelity
- **Evidence:** CHANGELOG.md:335-339 records handover v0 shipped. But
  SPEC.md:280-282's T2 copies JSONL only; the prompt itself flags
  fidelity beyond JSONL as unverified (agent-review-prompt.md:110-111).
  ZCode T2 is unverified and SQLite-coupled.
- **Recommendation:** Label v0 honestly in UI and CHANGELOG as
  "transcript-only handover (experimental)" until F3.4 passes. Do not
  let the M4 wizard copy imply pack-grade fidelity before then.

### F3.3 [P1] T2 is a single fragile byte-copy
- **Evidence:** `session_handover_impl` (commands.rs:1088-1194; also
  1121-1146) picks the newest JSONL by mtime under account A
  (commands.rs:1026-1033), encodes cwd via `replace('/','-')` (:1121),
  copies that one file to account B (:1137-1146), then launches
  `--resume <id>` (claude_code.rs:294-296; launch.rs:23-26). Verified
  independently by two sweeps (priors 9 and 11).
- **Failure modes, concretely:**
  - Newest-by-mtime can pick the *wrong* session when several are
    active; the code ignores `SessionRecord.transcript_ref`
    (model.rs:314-321,415) which already knows the right file.
  - No freeze/stop of the source session: copying a JSONL that Claude
    is actively appending risks a torn final line.
  - `replace('/','-')` is a guess at Claude's project-dir mangling;
    the codebase itself admits dir-name decode is ambiguous for dots
    and long paths (commands.rs:1046-1060).
  - No `--fork-session`/sessionId rewrite and no collision handling
    if the target already has that session ID.
  - Lineage stops at marking the source `HandedOver`
    (model.rs:205-206) — no pack, no diff, no return path yet.
- **Recommendation:** Use `transcript_ref`, stop/freeze the source
  before copying, verify the copy parses (reuse `dedupe_messages`,
  claude_transcript.rs:167-186 — which exists but no handover code
  calls), and record source/target/session IDs as a lineage row.

### F3.4 [P1] Sidechains survive; sibling state does not
- **Evidence:** Inline `isSidechain` lines live *inside* the main
  JSONL (fixture `session-subagent.jsonl:3-4`; parser flags them,
  claude_transcript.rs:45-46,106-109), so the byte-copy does carry
  them — a genuinely reassuring nuance. But separate subagent
  transcripts, spilled tool-results, and file-history snapshots sit
  as sibling files under `projects/<mangled>/`
  (research/claude-code.md:40,44) and are **not** copied. Todos:
  SPEC.md:272 promises in-session todos are lossless, yet there is
  zero todo/file-history/MCP code in the handover path, and the
  parser drops `tool_use`/`tool_result` text
  (claude_transcript.rs:120-137).
- **Recommendation:** Copy the whole `projects/<mangled>/<session>*`
  file set, not the single newest file. Then write the fidelity test
  that proves it: fixture with subagent files + todos + file-history,
  transplant, resume against the fake harness, assert state. Follow
  with one live dogfood on real Claude 2.1.296 (see Appendix A).

### F3.5 [P1] The honest minimum version to ship
- **Recommendation (consolidated):** Ship "T2-honest": transcript +
  sibling session files, source frozen, target headroom checked,
  fidelity tier displayed *before* confirm ("Transcript + subagents
  carried; MCP state and running processes do not transfer"), lineage
  recorded, one-click return on reset (F1.5). Market nothing above
  that until a real-binary test passes. T1 (same-account resume)
  already works mechanically via `--resume <id>`
  (claude_code.rs:294-296) — surface it in UI now (F2.3); it is free.
---

## Q4 — Compliance audit
### F4.1 [P1] Credential byte-copy on import contradicts SPEC S8
- **Evidence:** `import_claude_config` byte-copies `.credentials.json`
  into the managed dir (claude_environment.rs:209-232), confined to a
  0600 managed dir. SPEC S8 (SPEC.md:310) states credentials are
  "never read/copied/stored." Two independent sweeps (priors 9, 11)
  flagged this. Meanwhile `login_state` is correctly metadata-only —
  file length + keychain existence, never contents
  (claude_code.rs:323-344,579-589).
- **Judgment:** The import copy is arguably *necessary* for a copied
  profile to remain logged in, and it never leaves the managed dir.
  But the spec sentence and the code cannot both be right, and
  "credentials never copied" is a claim users will quote back.
- **Recommendation:** Decide explicitly: either (a) re-import copies
  everything *except* `.credentials.json` and the user re-logs-in via
  the vendor flow (spec-pure, worse UX), or (b) amend SPEC S8 to
  "credentials are copied only into the 0700 managed dir during
  user-initiated import, never read, logged, or transmitted" and add
  a guardrail test asserting exactly that. Option (b) with the test
  is the honest engineering call. This needs a named security review,
  not a docs assertion (Appendix A).

### F4.2 [P2] Passive/click-bound behaviour holds — but is unenforced
- **Evidence:** All mutations sit behind Tauri commands
  (commands.rs:633,1070; lib.rs:83-109); no auto-switch, auto-restart,
  proxy, or `BASE_URL` code exists (grep across `crates/*.rs` empty).
  The supervisor spawns the unmodified binary directly
  (supervisor.rs:107-120; claude_code.rs:298-305). Writes are confined
  via `prepare_managed_dir` 0700+marker (claude_code.rs:389-403) and
  `atomic_write` temp+rename (core/fs.rs:18-30); import requires
  dest-must-not-exist with a read-only source
  (claude_environment.rs:167-185).
- **The gap:** SPEC.md:26,136,263 name a `tests/guardrails.rs` suite.
  It does not exist (verified by file search in priors 9/11; only
  onboarding/profile guardrail *comments* exist). The invariants hold
  by discipline, not by test — discipline does not survive refactors.
- **Recommendation:** Write `guardrails.rs` before M2 multiplies the
  write paths: assert no writes outside managed dirs (fixture HOME),
  no credential reads outside the F4.1 carve-out, and no spawn without
  a preceding user-initiated command.

### F4.3 [P2] Login handling is correctly vendor-owned
- **Evidence:** Login runs the vendor's interactive flow in an
  embedded terminal (claude_code.rs:354-372; adapter.rs:108-123).
  No credential handling, no token scraping.
- **Recommendation:** No change. Call this out in the README's trust
  section — it is a genuine differentiator vs. proxy-based switchers.
---

## Q5 — Architecture & quality risks
### F5.1 [P1] Scale is thread/watcher/SQLite-serialization bound
- **Evidence:** Only Claude is registered (state.rs:91-97); Codex/ZCode
  have zero adapters. Each PTY spawn costs 2 reader threads + 1 wait
  thread (supervisor.rs:157,242,492-504) plus a 512KB ring per session
  (supervisor.rs:19-20; ring.rs:11-17). The transcript watcher rescans
  and full-reads every `.jsonl` per poll (claude_code.rs:453-473). A
  single `Mutex<Storage>` (state.rs:13-25) serializes everything, with
  N+1 list queries (commands.rs:874-961). `sysinfo` does a full refresh
  per adopt/descendant query (journal.rs:200-216; supervisor.rs:557-579).
- **Recommendation:** Before M2: incremental transcript tailing (byte
  offset per file, parse appends only), a single shared `sysinfo`
  refresh on an interval, and replacing N+1 lists with joined queries.
  Add a 20-session fake-harness load test to CI.

### F5.2 [P1] Stuck/waiting statuses are never emitted (arch + UX)
- **Evidence:** See F2.5. `apply_lifecycle_event` (commands.rs:782-810)
  is the single choke point and it has no path to Stuck/WaitingUser/
  QuotaBlocked. M2-5 stuck detection (EPICS.md:79) is specced but the
  emission plumbing should land first.
- **Recommendation:** Wire status emission + rail chips + inbox counts
  as one vertical slice before adapter #2. It serves G4 daily and
  de-risks the inbox UX (§UX.3) that everything else hangs off.

### F5.3 [P2] `juggler-handover` is a stub; logic lives in the app crate
- **Evidence:** `juggler-handover/src/lib.rs:1-13` doc-only, while the
  transplant logic lives in `crates/juggler-app` commands
  (commands.rs:1088-1194). Prior 9's verdict stands: "T1/T2 skeleton
  present but naive; T3/T4/handover crate/link crate are stubs."
- **Recommendation:** Move transplant into `juggler-handover` with the
  adapter trait gaining `export_session`/`import_session` methods;
  keep Tauri commands as thin callers. The golden-path test moves
  with it and gains the F3.4 fixtures.

### F5.4 [P2] Test suite is green except the one test that matters most
- **Evidence:** `cargo test --workspace` — all suites pass except
  `golden_path::handover_moves_context_to_another_account_and_resumes_it`,
  which FAILED with `failed to openpty: PermissionDenied` in the
  sandbox (environment artifact, not logic); `dogfood_v1` has 2 ignored
  tests (prior 9). So the handover e2e is unverified in this
  environment, and the ignored dogfood tests are the only real-binary
  coverage.
- **Recommendation:** Make the golden-path test PTY-independent (piped
  mode) so CI actually exercises handover; keep the PTY variant as
  `#[ignore]` dogfood. Never let the signature feature's only e2e be
  the one that cannot run.

### F5.5 [P2] Single-harness reality vs. multi-harness types
- **Evidence:** Domain types are harness-generic but only one adapter
  is registered; Codex quota is Estimated with no API
  (research/codex.md:52; SPEC.md:203), ZCode is undocumented/bundle-only
  (research/zcode.md:92-95), and profile breakage has precedent
  (research/codex.md:19). M2's breadth multiplies exactly these fragile
  surfaces (see Q6).
- **Recommendation:** Land the adapter conformance kit (F1.8) with
  adapter #2, not after #3. Every adapter ships the same fixture
  corpus: launch, resume, transcript parse, quota honesty tier.
---

## Q6 — Roadmap sanity
### F6.1 [P0] The order is inverted against the core promise
- **Evidence:** PROPOSAL.md:14,32-33 (G2/G3) make quota knowledge and
  momentum the product. SPEC.md:288's return flow needs a reset alert;
  EPICS.md:109 (M4-7) wizard needs quota headroom. Yet the milestone
  table puts quota at M3 *after* M2's adapters (EPICS.md:26-28;
  SPEC.md:452-454). Prior 5's verdict, which this review endorses:
  pull M3-1/M3-2 (Claude-Measured quota, SPEC.md:192 statusline feed;
  EPICS.md:90-91) and M4-2 lineage ahead of M2; defer M3-3/M3-4
  (ZCode/Codex estimation) until those adapters exist; keep M2-5
  stuck detection (EPICS.md:79) early because it serves the G4 daily
  loop.
- **Recommendation:** Adopt the phased reorder in §Roadmap below.
  One harness *with* quota + handover beats three harnesses without.

### F6.2 [P0] The docs contradict each other on numbering and state
- **Evidence:** Prompt Q6 calls quota "M4"; the repo consistently puts
  quota at M3 and handover at M4 (README.md:64-68; SPEC.md:453-454) —
  verified in prior 10, correcting prior 5's initial conflation.
  Separately, README.md:7 claims "spec phase / no releases" while
  CHANGELOG.md:335-341 ships handover v0. QUALITY.md:9 requires the
  repo to tell the truth.
- **Recommendation:** Fix README status + milestone table in the same
  PR as any roadmap reorder. Add a docs-consistency check (README
  status line vs. latest CHANGELOG entry) to the M-gate checklist.

### F6.3 [P1] M1 gate is unsigned while M2 scope accumulates
- **Evidence:** `docs/qa/gate-M1.md:54-67` remains unchecked/awaiting
  confirmation; EPICS.md:16 bars the next milestone without sign-off.
  Open work (#22 Session board UI, #23 Guardrail/Gate V1, #16 M1
  tracking) vs. merged PRs (#42 golden-path, #44 walkthrough, #34-36
  rail/onboarding) explains the current state (prior 6).
- **Recommendation:** Close or explicitly waive gate-M1 before any M2
  adapter code. If waived, record *which* criteria are waived and why,
  in the gate file itself.

### F6.4 [P1] M2 breadth multiplies fragile surfaces too early
- **Evidence:** F5.5. Codex quota is Estimated/no-API; ZCode is
  bundle-only/undocumented; the handover crate is a stub; guardrails
  suite does not exist. Each adapter imports its vendor's fragility.
- **Recommendation:** Sequence adapters *after* the Claude quota +
  handover-honest slice proves the loop, and ship Codex before ZCode
  (documented surface beats bundle archaeology). See §Roadmap.
---

## Q7 — Competitive angle
Full deep-dives follow this section; here is the judgment.

### F7.1 [P1] Nobody owns "juggling" — the niche is genuinely open
- **Evidence:** T3 (provider instances only, no named multi-account
  profiles), Opcode (no multi-account feature found in README/docs —
  negative evidence, Appendix A), Claude Squad (no rotation/pooling/
  quota UI upstream), Conductor (no native switcher/quota dashboard
  found), Vibe Kanban (BYO subscriptions, no rotation). The rotation/
  quota tooling lives in a separate ecosystem: cc-switch (tray
  switcher with 5h/weekly quota), claude-swap (auto-rotation at 90%
  thresholds + live dashboard), ccusage (cost reports).
- **Recommendation:** Say it plainly in positioning: "Orchestrators
  run your agents. Switchers swap your account. Juggler does both,
  quota-aware." Then make the claim true with F6.1's reorder.

### F7.2 [P1] Table stakes Juggler lacks that switcher users expect
- **Evidence:** cc-switch: one-click provider/account switch without
  restart, tray quota at a glance. aisw: one-command named profiles
  (`aisw use claude acct`). claude-swap: rotation when 5h/7d usage
  crosses ~90%. Juggler's tray is quota-only on paper (SPEC.md:352)
  and absent in `ui/src` (prior 8).
- **Recommendation:** Tray quick-switch + at-a-glance quota (F1.9) is
  P1, not polish. Suggested-only rotation ("Account A at 92% — switch
  new sessions to B?") fits the passive invariant; never auto-rotate.

### F7.3 [P2] Table stakes Juggler lacks that orchestrator users expect
- **Evidence:** Worktree isolation (F1.1), diff review with inline
  comments sent back to the agent (Vibe Kanban), checkpoints/timeline
  restore (Opcode), per-thread terminal+diff surfaces (T3). Juggler
  has terminal embedding and (specced) packs, but no diff/review loop.
- **Recommendation:** Worktree (P0/P1, F1.1) first; a read-only diff
  view on finished sessions second; inline-comment-to-agent only
  after the review queue (§UX, WOW-h) proves demand. **[inferred]**
  review-queue demand needs dogfood validation (Appendix A).

### F7.4 [P2] The sunset window: Vibe Kanban and Crystal users
- **Evidence:** Bloop shut down 2026-04-10; Vibe Kanban is sunsetting,
  community-maintained, last push 2026-04-24 (prior 7). Crystal
  deprecated Feb 2026 in favour of commercial Nimbalyst. Opcode has
  had no commits since Oct 2025 (prior 4).
- **Recommendation:** An import path ("bring your projects from Vibe
  Kanban/Claude Squad config") plus one honest comparison page is
  cheap, timely marketing. Do not build kanban — import the *users*,
  not the metaphor.
---

## Competitor deep-dives
Star counts are snapshot-dated and were not fetched from the GitHub
API in-session (Appendix A). Treat them as magnitude, not ranking.

### T3 Code (pingdotgg/t3code)
#### What it is
Open-source control plane for harnesses you already pay for —
"orchestrate the agents you already pay for," local-first GUI.
[https://t3.codes/](https://t3.codes/)

#### Harnesses & accounts
Claude Code, Codex, Antigravity, OpenCode, Cursor, Grok, plus custom
via config; BYO already-authenticated providers. Multi-account is
provider *instances* only — auth per provider/env, not first-class
named profiles (mjolnir fork notes). Continuation compatibility
varies by harness.

#### Sessions, quota, cost
Parallel threads/projects, git worktree per task/session, diff viewer
with staging/commit, settle-vs-archive thread lifecycle. A usage
panel shows token + API-cost analytics; whether it reflects official
remaining subscription quota is disputed across sources — a 5h/weekly
probe exists in forks. Juggler must not repeat this ambiguity: label
every number Measured/Computed/Estimated (SPEC.md:254-259).

Pricing: free OSS, zero markup. Traction: ~24k stars (2026-09-24). Strengths: worktrees, mobile. Weaknesses: alpha data-loss ToS, poor CLI sync.

### Opcode / Claudia (winfunc/opcode)
#### What it is
Polished IDE-like GUI wrapper over the Claude Code CLI: spawns the
CLI and parses `~/.claude/projects/*.jsonl`. Tauri 2 + React + TS +
Rust, by Asterisk/Winfunc (YC); formerly Claudia, renamed mid-2025.
[https://github.com/winfunc/opcode](https://github.com/winfunc/opcode)
· [https://opcode.sh/](https://opcode.sh/)

#### Harnesses & accounts
Single-harness (Claude Code) by design. **No multi-account feature
found** in README/docs/targeted search — absence is negative evidence
only (Appendix A). The app auto-detects one `~/.claude` dir and
delegates auth to the CLI, so account swapping means an OS/CLI-level
`~/.claude` swap.

#### Sessions, quota, cost
Visual project/session browser over `~/.claude/projects/`, search,
resume, multiple interactive sessions. Custom agents with system
prompts, per-agent permissions/model, agent library, import/export
`.opcode.json`, sandboxed background agents (seccomp/Seatbelt). MCP
server registry/management UI with Claude Desktop config import.
Usage Dashboard: cost by model/project/date, token charts, export;
alerts listed as "coming soon."

#### Signature feature
Timeline/checkpoints: create a checkpoint anytime, visual branching
timeline, one-click restore, fork sessions from checkpoints, diff
between checkpoints. Nothing in Juggler's SPEC matches this; the
closest is the handover pack (different problem).

Pricing: free AGPL-3.0, ~21-22k stars. Strengths: checkpoints, MCP, agents. Weaknesses: dormant since Oct 2025.

### Claude Squad (smtg-ai/claude-squad)
#### What it is
Go TUI (`cs`) meta-orchestrator — it is not an agent. One task = one
tmux session (process isolation) + one git worktree/branch (FS
isolation), with a central list, preview, background run, and
persisted/restored sessions.
[https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)

#### Harnesses & accounts
Agent-agnostic via `-p/--program`; `default_program` + named
`profiles[{name,program}]` in `~/.claude-squad/config.json`.
Named: Claude Code, Codex, Gemini, Aider, OpenCode, Amp,
Antigravity, any local CLI. **Quota/accounts: absent upstream** —
config holds no rotation/pooling/quota; auth is delegated to each
CLI's login/env. Multi-account quota forks exist (e.g.,
terrene-foundation) and must not be attributed to upstream.

#### UX & review loop
Keyboard-driven terminal-native flow: Tab preview/diff,
review-before-apply; `s` = commit+push branch, `c` = checkout
(commit+pause: keeps branch, removes worktree), `r` = resume.
`-y/--autoyes` [experimental] auto-accepts prompts for Claude Code &
Aider via a polling daemon — note this crosses Juggler's
passive/click-bound invariant; do not copy it.

Pricing: free, `brew install`, ~8.1-8.5k stars. Strengths: fastest parallel setup. Weaknesses: tmux+gh required, weak visual diff.

### Conductor (conductor.build, Melty Labs)
#### What it is
Native macOS app (YC S24) that pioneered the agentic parallel runner:
each agent in an isolated git worktree+branch, one dashboard to
watch/review/merge.
[https://www.conductor.build/docs/concepts/workflow](https://www.conductor.build/docs/concepts/workflow)

#### Harnesses & accounts
Claude Code, Codex, Cursor, OpenCode directly in Conductor or opened
in your IDE; BYO subscriptions/keys; Linear + GitHub integration.
**No Conductor-native multi-account switcher or quota dashboard
found** in docs/searches (negative evidence; not confirmed absent
from in-app settings — Appendix A). Quota pain is real underneath:
a Claude Pro $20 5h cap is easily hit by 3–5 parallel agents.

#### UX & review loop
Start a workspace from a branch/PR/GitHub/Linear issue; Diff Viewer,
terminal/Spotlight tests, Checks tab (git status, CI, deployments,
comments, todos), create PR, merge, archive. Third parties report
checkpoints and multi-model same-prompt (Claude+Codex side-by-side).

Pricing: Free local / Pro $50/mo / Teams $60. Strengths: polished diff+PR flow. Weaknesses: Mac-only, closed-source, quota burn.

### Vibe Kanban (BloopAI) + Crystal/cc-switch sidebar
#### Vibe Kanban — what it is
Kanban-over-agents: plan/prioritise/assign issues as cards
(To Do → In Progress → Review → Done). Moving a card to In Progress
auto-creates a worktree and starts an agent. The card=worktree=agent
metaphor is the most-copied UX in this category.
[https://github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban)

#### Vibe Kanban — harnesses, review, quota
Agent-agnostic, 10+ agents (Claude Code, Codex, Gemini CLI, Copilot,
Amp, Cursor, OpenCode…), switch agents per task, parallel/sequential
orchestration, own MCP server (~40 tools), Rust + React/SQLite, via
`npx vibe-kanban`. Integrated diff view; review diffs and leave
inline comments sent directly back to the agent without leaving the
UI; PR creation via `gh`; guarded squash merge/archive. Worktree+
branch per task-attempt (per repo in multi-repo), terminal + dev
server + preview browser per workspace. **Quota/accounts: none
native** — BYO subscriptions/keys, no rotation/dashboard.

#### Vibe Kanban — pricing, status, weaknesses
Free/self-hosted, Apache-2.0 (some posts wrongly say MIT), ~28.2k
stars. **Bloop shut down 2026-04-10; Vibe is sunsetting** — paid/cloud
wound down with refunds, now community-maintained, last push
2026-04-24, login required since 0.1.9. Tasks live in SQLite, not
git; manual checkouts/external PRs are not recognised; worktree
deletion loses the card link; per-CLI output adapters are brittle;
no rules/quality gates or semantic conflict detection.

#### Crystal (Stravu) — the sidebar cautionary tale
MIT Electron desktop: parallel Claude Code/Codex sessions, each in a
git worktree, session list + diff/compare, rebase/squash tooling.
Deprecated Feb 2026 (v0.3.5), succeeded by commercial Nimbalyst.
Same fate pattern as Vibe: beloved parallel-session UX, unsustained
as a free standalone.

#### cc-switch / ccusage / claude-swap — the juggling layer
This separate ecosystem is Juggler's *real* incumbent competition:

- **cc-switch** (farion1231): cross-platform desktop for Claude
  Code/Codex/OpenCode/Gemini/Grok. Tray row shows
  `name · mode · provider · quota`; one-click provider/account switch
  (Claude needs no restart); inline 5h/weekly quota/balance (incl.
  Copilot/Codex OAuth); usage stats with models.dev pricing sync.
- **ccusage** (ryoppippi, ~13–17k stars): CLI reading
  `~/.claude/projects` JSONL (+ Codex/OpenCode/Amp), producing
  daily/monthly/session/5h-block token + API-pricing cost reports.
  Cost accounting, not subscription-% quota.
- **claude-swap** (`cswap`, realiti4): Claude-only multi-account with
  auto rate-limit rotation and a live usage dashboard; parallel
  sessions. Closest existing thing to "juggling" — and it is a CLI
  switcher, not a session supervisor.
- **aisw** (burakdede): one-command named profiles for Claude/Codex/
  Gemini/Antigravity (`aisw use claude acct`); fresh process after
  switch.

## Competitor comparison (narrow tables)
Cells are short phrases by design; nuance lives in the deep-dives.

### Orchestrators: isolation & review
| Capability | Juggler (today) | T3 | Claude Squad |
|---|---|---|---|
| Worktree per session | No (F1.1) | Yes | Yes |
| Diff review loop | No | Diff + staging | Preview diff |
| Session lifecycle | No (F1.3) | Settle/archive | Pause/resume |
| Handover lineage | v0 transcript | Varies | Branch only |

| Capability | Opcode | Conductor | Vibe Kanban |
|---|---|---|---|
| Worktree per session | No | Yes | Yes |
| Diff review loop | Checkpoint diff | Diff + PR | Inline→agent |
| Session lifecycle | Checkpoints | Archive | Board columns |
| Handover lineage | Fork checkpoint | PR flow | Card history |

### Accounts & quota: the juggling layer
| Capability | Juggler (SPEC) | cc-switch | claude-swap |
|---|---|---|---|
| Named accounts | Yes (G1) | Yes | Yes |
| Quota display | M3 planned | 5h/weekly live | Live dashboard |
| Rotation | Suggest-only | 1-click switch | Auto ~90% |
| Cost (USD) | No (F1.2) | Pricing sync | Via ccusage |

| Capability | T3 | Orchestrators* | Juggler wedge |
|---|---|---|---|
| Named accounts | Instances only | Delegated | First-class |
| Quota display | Cost panel† | None native | Measured M3 |
| Rotation | No | No | Suggest-only |
| Cost (USD) | Yes (analytics) | No | Ledger (F1.2) |

\* Opcode/Conductor/Vibe/Squad grouped: all delegate auth/quota to
the underlying CLIs per deep-dives. † T3 official-vs-cost disputed.

### Platform & licensing snapshot
| Tool | Platform | Licence / price |
|---|---|---|
| Juggler | macOS-first | OSS (this repo) |
| T3 Code | Desktop+web+mobile | MIT, free |
| Opcode | Mac/Linux/Win | AGPL-3.0, free |
| Claude Squad | macOS/Linux TUI | AGPL-3.0, free |
| Conductor | macOS only | Free / $50 Pro |
| Vibe Kanban | Web/desktop | Apache-2.0, free |
---

## Developer-persona pain-point map
Personas distilled from PROPOSAL.md:51-54 and the persona sweep
(prior 8). Jobs-to-be-done assume 5–15 parallel sessions/day.

### The five jobs
1. **Start the right account on the right repo** without wrong-billing
   or wrong-tier surprises (F1.4).
2. **Never lose momentum at the quota wall** — know before it hits,
   move without re-explaining (F1.5, F3.5).
3. **Know which of my 15 sessions needs me *now*** (F2.5).
4. **Resume or hand over without re-explaining context** (F3.4).
5. **Prove cost and provenance** — which account did what, for how
   much (F1.2).

### Pains, ranked by daily cost
| Rank | Pain | Today in Juggler |
|---|---|---|
| 1 | Quota wall mid-task | Blind (M3 pending) |
| 2 | Lost context on handover | Partial (F3.3/F3.4) |
| 3 | Stuck/waiting invisible | Modelled, unemitted |
| 4 | Which account did what | Lineage minimal |
| 5 | Cost burn opaque | Tokens only |
| 6 | Notification fatigue | No quiet digest |
| 7 | Worktree collisions † | Unguarded (F1.1) |
| 8 | Review bottleneck † | No diff queue |

† **[inferred]** Persona-inferred; grounded only in the pack branch
field (SPEC.md:301). Needs dogfood validation (Appendix A).

### Nielsen pass (summary)
Visibility FAIL; Consistency/Flexibility/Recognition WEAK; Match/Control/Minimalism GOOD; Error recovery/Help PARTIAL — detail in F2.1-F2.5, UX.8.

### WOW feature proposals
Ship the P0 trio before any Usage charting: triage and forecast
*prevent* loss; charts only *explain* it.

- **WOW-a [P0] Needs-Attention inbox.** One queue for WaitingUser /
  Stuck? / QuotaBlocked / crashed / review-ready, sorted by waiting
  time, count badge on rail + tray, one-click jump + action,
  snooze/mute per session. Reuses SPEC.md:220-224,239-246 plumbing.
- **WOW-b [P0] Quota forecast at launch and handover.** Inline in
  NewSession/ProjectPanel: "Account A exhausts in ~40m at current
  burn; B has 80% headroom" with admission verdict OK/Tight/No.
  Already promised (SPEC.md:83,257); absent in UI. This is the single
  most persuasive screenshot the product can produce.
- **WOW-c [P0] ⌘K command palette + session search.** Fuzzy search
  across projects/sessions/accounts/transcript text; actions (resume,
  fork, handover, jump) inline; fully keyboard-driven. Fixes the
  Recognition/Flexibility failures in one surface.
- **WOW-d [P1] Smart routing suggestion (never auto).** Rank profiles
  by headroom × footprint × project history, explain the ranking,
  one-click accept. Compliant with PROPOSAL.md:62-63 (prepare/passive,
  execute on click).
- **WOW-e [P1] Cost ledger.** Per-session/account/project tokens →
  estimated USD with confidence label, daily budget bar, CSV export
  (SPEC.md:349,256-259). Answers Persona A's Max-vs-API question.
- **WOW-f [P1] Branch/worktree guard.** Branch + dirty state per
  instance; warn on same-branch parallel sessions; offer
  worktree-on-launch (F1.1).
- **WOW-g [P1] Quiet notifications.** Digest by default; interrupt
  only for actionable events (waiting > N min, quota < X%, stuck);
  per-project mute. Kills pain #6 without hiding pain #3.
- **WOW-h [P2] Review queue.** Finished sessions with diff stat +
  PACK.md preview; approve/continue inline. **[inferred]** demand —
  validate with dogfood before building the full loop (F7.3).
- **WOW-i [P2] Session replay/timeline.** Scrub transcript + token
  burn over time; fork from any point (borrows Opcode's checkpoint
  insight, adapted to lineage).
- **WOW-j [P2] Fleet attention roll-up.** Cross-machine needs-me
  count (SPEC.md:389) once Fleet (M6) exists; design the inbox data
  model fleet-ready now.
---

## UI/UX redesign direction
### UX.1 Information architecture
- Merge Instances into Sessions until the resource manager gives
  Instances real content (F2.2). Rail becomes: **Inbox (badge) ·
  Projects · Sessions · Accounts**.
- Promote the Needs-Attention inbox to the rail's first item and the
  tray's first line. It is the daily home, not Overview.
- Overview keeps quota rings + machine + fleet roll-up, but every
  widget renders an honest empty state until its milestone lands
  (F2.4). No fabricated numbers, ever.
- Usage and Fleet leave the global nav until they have content;
  a nav item that opens "Coming soon" trains users to stop exploring.

### UX.2 The daily loop, redesigned
1. **Glance (tray/rail):** "3 need you · A at 78% · reset 17:00."
2. **Triage (inbox):** oldest-waiting first; each row shows why it
   needs you and the one action that clears it.
3. **Act (session panel):** terminal + the clearing action
   (reply / handover / resume-at-reset) without navigation.
4. **Launch (NewSession):** profile picker shows headroom + forecast
   (WOW-b) and the project's usual account (F1.4) *before* Start.
5. **Close the loop:** finishing a session offers review (WOW-h) or
   archive (F1.3) — never a silent pile-up.

### UX.3 Needs-attention inbox
- Sources: WaitingUser, Stuck?, QuotaBlocked, crash, review-ready.
- Sorted by waiting-desc; each row: session, project, account, why,
  waiting time, primary action, snooze/mute.
- Badge = count of unsnoozed items, on rail and tray. Empty state:
  "All clear — N sessions running" (a *good* empty state worth
  designing, not a blank panel).

### UX.4 Quota UX
- Every quota number carries its honesty tier (Measured/Computed/
  Estimated, SPEC.md:254-259) as a visible label, not a footnote.
- Burn-down forecast ("exhausts in ~40m") beside every ring; reset
  countdown beside every blocked account.
- Colour is never the only channel: pair ring colour with % text and
  a trend arrow (also an accessibility fix, UX.8).
- At QuotaBlocked: inline "Queue resume at reset (17:00)" (F1.5).

### UX.5 Handover UX
- A three-step wizard replacing inline buttons (F2.3):
  1. **Target:** accounts ranked by headroom (WOW-d), usual-account
     marked.
  2. **Fidelity:** tier badge + plain-language carry list
     ("transcript + subagents carried; MCP state does not transfer")
     + redaction preview (F1.6).
  3. **Confirm:** one click; lineage recorded; "Return on reset"
     offered immediately.
- After handover, both sessions show lineage ("Handed over from
  sess-42 · Account A") with jump links both ways.

### UX.6 Command palette
- ⌘K opens fuzzy search over projects/sessions/accounts/transcripts
  (WOW-c). Actions execute without the mouse; recent items first.
- Palette is also the keyboard story's front door: every daily
  action reachable in ≤3 keystrokes from anywhere.

### UX.7 Progressive disclosure
- **L1 glance:** tray line, rail badges, ring + forecast.
- **L2 panel:** why + forecast + suggested action (inbox rows,
  session header).
- **L3 detail:** full Usage/Fleet analytics, pricing tables, lineage
  history. Depth lives in L3; the daily loop never requires it.
  (Fixes the current inversion: L1 data buried in Global placeholders.)

### UX.8 Accessibility & keyboard
Current state (prior 6, code-read only — not empirically verified):

- Modals lack `role=dialog`/`aria-modal`/focus-trap/return-focus;
  Escape works only via wrapper keydown (NewSession.tsx:77-80;
  form.tsx:66-72). Onboarding's close is an unlabelled "✕"
  (OnboardingWizard.tsx:130-136).
- Tablist has `aria-selected` but no `tabpanel`/arrow-key model
  (WorkflowRail.tsx:119-157); the resize separator is not
  keyboard-operable (:218-220).
- Status is conveyed by colour dots alone; no `prefers-reduced-motion`
  despite breathe/glow animations (styles.css:90-104); faint
  `#5b6474` 10.5px subtext likely fails AA (styles.css:24).
- Positives to keep: native buttons, `:focus-visible` styles
  (styles.css:76-79), ⌘B/⌘N shortcuts (App.tsx:205-219).

Required fixes (P1 unless noted):

- All modals: dialog semantics, focus trap, return-focus, labelled
  close buttons. [P1]
- Tabs: full ARIA tabs pattern with arrow keys; separator becomes a
  keyboard-adjustable splitter. [P1]
- Status chips get text labels, never colour-only. [P1]
- Honour `prefers-reduced-motion`; raise subtext to AA contrast
  (≥4.5:1) at 10.5px or bump to 11.5px. [P1]
- Density: rail rows at 12.5px/10.5px with truncation everywhere
  (WorkflowRail.tsx:272-287,429-435) aid scanning but hide status;
  truncation tooltips + the inbox (UX.3) relieve this. [P2]
- Loading states: skeletons instead of bare "Loading…" text
  (Profiles.tsx:75-77; NewSession.tsx:206-208). [P2]
---

## Revised roadmap (reordering proposal)
### Principle
Prove the juggling loop on one harness first (F6.1): quota + handover honesty are the loop; adapters are breadth.

### Phase 0 — Truth & gates (immediate, days)
- Fix README status/milestones (F6.2); label handover v0
  transcript-only/experimental (F3.2).
- Downgrade advertised capabilities to implemented ones (F3.1).
- Close or explicitly waive gate-M1 (F6.3); decide F4.1 (credential
  copy: spec amendment + test, or behaviour change).
- Replace fabricated Overview numbers with honest empty states (F2.4).
- App-level loading/error model; delete `window.alert` paths (F2.1).

### Phase 1 — v0.1-beta: the Claude juggling loop
Scope: everything that makes *one* harness juggle honestly.

- **Quota (pulled forward from M3):** M3-1/M3-2 Claude-Measured via
  statusline feed (SPEC.md:192; EPICS.md:90-91) — rings, burn-down
  forecast, reset countdown, honesty labels (UX.4). This unblocks
  WOW-b, the handover headroom check, and return-on-reset.
- **Needs-attention slice (pulled forward from M2-5):** emit
  Stuck/WaitingUser/QuotaBlocked (F5.2), rail chips, inbox v1 (UX.3).
- **Handover-honest (pulled forward from M4):** `transcript_ref`
  correctness, source freeze, sibling-file copy, fidelity wizard,
  lineage rows (F3.3-F3.5); logic moved into `juggler-handover`
  (F5.3); PTY-independent golden-path test (F5.4).
- **Safety nets:** `guardrails.rs` (F4.2); worktree-on-launch option
  + same-branch warning (F1.1); DB backup/restore (F1.7).
- **Daily-loop UX:** ⌘K palette + session search (WOW-c); Resume/
  Relaunch on ended sessions (F2.3); session lifecycle v1
  (archive/search) (F1.3); accessibility P1 fixes (UX.8).
- **Exit:** dogfood loop (2 accounts, quota wall, honest handover, return on reset) passes on real Claude — also the F3.4 proof.

### Phase 2 — Breadth: second harness + cost
- Codex adapter (before ZCode: documented surface, F6.4) with the
  conformance kit (F1.8/F5.5); Codex quota as Estimated, labelled.
- Cost ledger + pricing table + CSV (F1.2/WOW-e); account-health
  (tier/expiry/affinity) (F1.4); reset-queue (F1.5); pack redaction
  (F1.6); tray quick-switch (F1.9).
- Scale fixes: incremental transcript tailing, shared sysinfo,
  joined queries, 20-session load test (F5.1).
- **Exit:** Claude+Codex side-by-side, honesty tiers visible.

### Phase 3 — Third harness + polish toward 1.0
- ZCode adapter (bundle-only fragility contained behind the
  conformance kit); ZCode Computed quota (M3-3/M3-4 deferred to here).
- Profile clone/MCP sync (F1.9); review queue + diff view (WOW-h,
  dogfood-validated); session replay (WOW-i); quiet notifications
  (WOW-g); Vibe/Squad import path (F7.4).
- Hardening, signed/notarized self-update (existing M5), then 1.0
  stabilization from beta feedback (existing 1.0 definition).
- Fleet (existing M6/M7) unchanged in position — after 1.0 — but the
  inbox data model ships fleet-ready in Phase 1 (WOW-j).

### What moves, at a glance
| Item | From | To |
|---|---|---|
| Claude quota M3-1/2 | M3 (after M2) | Phase 1 |
| Stuck detection M2-5 | M2 | Phase 1 |
| Handover honesty M4-2 | M4 | Phase 1 |
| Codex adapter | M2 | Phase 2 |
| ZCode + M3-3/4 | M2/M3 | Phase 3 |
| Fleet M6/M7 | After beta | After 1.0 |
---

## Features-we-missed checklist
Sorted by priority. Finding references point to the analysis above.

### P0 — blocks the core promise
- [ ] Claude-Measured quota before adapters (F6.1)
- [ ] Honest handover: fidelity label, sibling files (F3.2-F3.4)
- [ ] Capability flags match implementation (F3.1)
- [ ] Needs-attention statuses actually emitted (F2.5/F5.2)
- [ ] App-level error state; no silent staleness (F2.1)
- [ ] README/docs truth pass; gate-M1 closed/waived (F6.2/F6.3)
- [ ] No fabricated quota/machine numbers in UI (F2.4)

### P1 — before v0.1-beta
- [ ] Worktree-on-launch + same-branch warning (F1.1)
- [ ] Cost ledger (USD, labelled, CSV) (F1.2)
- [ ] Session archive/delete/search/retention (F1.3)
- [ ] Account health: tier/expiry/affinity (F1.4)
- [ ] Reset-queue: notify + one-click resume (F1.5)
- [ ] Pack export redaction pass (F1.6)
- [ ] DB backup/restore/export (F1.7)
- [ ] Credential-copy decision + guardrails.rs (F4.1/F4.2)
- [ ] Resume/Relaunch; handover wizard (F2.3/UX.5)
- [ ] ⌘K palette + transcript search (WOW-c)
- [ ] Quota forecast at launch/handover (WOW-b)
- [ ] Tray quick-switch + quota glance (F1.9/F7.2)
- [ ] Instances/Sessions conflation resolved (F2.2)
- [ ] Scale: tailing, sysinfo, N+1, load test (F5.1)
- [ ] Modal/tab/splitter/contrast a11y fixes (UX.8)

### P2 — later, design for now
- [ ] Adapter SDK/conformance kit published (F1.8)
- [ ] Profile clone; MCP/rules sync (F1.9)
- [ ] Review queue + diff view (WOW-h) †
- [ ] Session replay/timeline fork (WOW-i)
- [ ] Quiet notification digests (WOW-g)
- [ ] Smart routing suggestion (WOW-d)
- [ ] Vibe Kanban/Squad import path (F7.4)
- [ ] Fleet attention roll-up (WOW-j)
- [ ] Dead modal removal; fetch cleanup (F2.8)
- [ ] ReadyHome sticky skip; skeletons (F2.7/UX.8)

† Dogfood-validate demand before building (Appendix A).
---

## Appendix A — Unresolved & verification limits
Preserved verbatim from the evidence sweeps. Nothing here was hidden;
each item bounds a claim in this report.

- No runtime/browser or screen-reader test run (read-only code
  review); contrast and focus-trap behaviour not empirically verified.
- SPEC S9 Usage/Fleet/Tray/Settings absence may be intentional M2+
  deferral per milestones SPEC.md:451-455, not yet a v0.1 breach.
- engine-arch-handover: incomplete/spilled/failed evidence (prior 9
  reached this synthesis via a nested handoff file; its grades were
  spot-checked but remain single-origin until re-run).
- Multi-account absence is inferred from absence in official
  README/docs and targeted search returning no feature — no explicit
  "not supported" statement found (Opcode).
- Exact live star count at submission (sources 8,108–8,537 by date);
  no direct GitHub API fetch available via web_search only.
- No official evidence of built-in multi-account rotation or
  per-account quota/reset-time display in Conductor itself (distinct
  from underlying Claude/Codex quotas); not confirmed absent from
  in-app settings.
- Fleet/Usage real UX unreviewable: ComingSoon placeholders
  (App.tsx:338-341); quota confidence UI, tray, global hotkey specced
  (SPEC.md:352) not found in `ui/src`.
- Review bottleneck + worktree collision have no spec/UI grounding
  beyond pack branch field (SPEC.md:301) — P1/P2 proposals are
  persona-inferred, need dogfood validation.
- Live handover fidelity unproven: T2 byte-copy, sidechain/todo/
  file-history transplant, cwd-mangling, cross-`CLAUDE_CONFIG_DIR`
  `--resume` need code+fixture/live Claude test (priors 5+9+11).
- Guardrail compliance code-level: credential byte-copy on import
  (prior 9 P1) vs. SPEC S8 "never copied" — needs security review,
  not docs assertion.
- No runtime/browser/screen-reader test: contrast, focus-trap,
  ComingSoon dead-ends unverified empirically (prior 6).
- Competitor absence claims are negative evidence only: opcode
  multi-account, Conductor multi-account/quota — no explicit vendor
  "not supported" source (priors 3,4).
- Exact live star/fork counts unverified (claude-squad 8,108–8,537
  range; others snapshot-dated) — no GitHub API fetch.
- M1 gate sign-off status (gate-M1.md unchecked per prior 5)
  determines whether M2 breadth/Usage absence is deferral or
  breach — not resolved.
- Review-bottleneck and worktree-collision P1/P2 (priors 5,8) are
  persona-inferred, no dogfood/spec grounding beyond SPEC:301 pack
  branch field.
- Q7 synthesis gap: no head-to-head on multi-account rotation/quota
  (cc-switch/claude-swap vs. orchestrators) tied to Juggler G2/G3
  differentiation claim in the source sweeps; §Q7 of this report is
  the synthesis, built from those profiles.
- Live fidelity still unproven: whether `--resume <id>` (not
  transcript-path form SPEC.md:280) resolves transplant across
  different `CLAUDE_CONFIG_DIR`s on real Claude 2.1.296, and mangling
  for dotted/long cwds — needs live dogfood; stub test cannot prove it.
- MCP state location (`~/.claude.json` vs. config dir) transplant
  behaviour — no code or fixture covers it (prior 9).
- T3 quota depth disputed across sources/versions (official remaining
  quota vs. cost analytics); Conductor pricing and Opcode dormancy
  partly from third-party mirrors, not primary fetches (prior 10).

## Appendix B — Sources
Repo evidence is cited inline as `file:line` throughout (README, SPEC, PROPOSAL, EPICS, research/*, ui/src/*, crates/juggler-*).

Competitor sources (accessed via web research, 2026-10):

- T3 Code — [https://t3.codes/](https://t3.codes/) ·
  [https://github.com/pingdotgg/t3code](https://github.com/pingdotgg/t3code)
- Opcode — [https://github.com/winfunc/opcode](https://github.com/winfunc/opcode) ·
  [https://opcode.sh/](https://opcode.sh/)
- Claude Squad — [https://github.com/smtg-ai/claude-squad](https://github.com/smtg-ai/claude-squad)
- Conductor — [https://www.conductor.build/docs/concepts/workflow](https://www.conductor.build/docs/concepts/workflow) ·
  [https://www.conductor.build/pricing](https://www.conductor.build/pricing)
- Vibe Kanban — [https://github.com/BloopAI/vibe-kanban](https://github.com/BloopAI/vibe-kanban)
- Crystal — [https://github.com/stravu/crystal](https://github.com/stravu/crystal)
- cc-switch — [https://github.com/farion1231/cc-switch](https://github.com/farion1231/cc-switch)
- ccusage — [https://github.com/ryoppippi/ccusage](https://github.com/ryoppippi/ccusage)
- claude-swap — [https://github.com/realiti4/claude-swap](https://github.com/realiti4/claude-swap)
- aisw — [https://github.com/burakdede/aisw](https://github.com/burakdede/aisw)

*End of review. The maintainers' next action is Phase 0 — it is
small, it is honest, and everything else in this report depends on
believing what the repo says about itself.*





