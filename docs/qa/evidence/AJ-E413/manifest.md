# Evidence manifest — AJ-E413 (Desktop adapter contract)

Task: `docs/tasks/E4-indicators-and-inspector.md` AJ-E413.
Branch: `stage2/e4-desktop`. Local runs (macOS arm64) 2026-10-14,
reviewer zcode-primary. Rust suites run in CI (verify job); component tests
in the ui job; the real-launch record is a local-only run by design.

| Claim | Command | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|---|
| Req.1 desktop launch path | `cargo test -p juggler-app --test desktop` (4/4) | program/bundle id, args, env overlay, account state dir → PID + bundle id; per-Provider isolation stated | `HarnessAdapter::desktop_launch` (default `Unsupported`) + `desktop_isolation_label`; `DesktopLaunch { program, bundle_id, args, env, cwd, isolation_label }`; capability bits `DESKTOP_APP` \| `DESKTOP_SINGLE_ACCOUNT`; wire `DesktopLaunchInfo { session_id, pid, bundle_id, isolation_label }` — fake-desktop returns bundle `dev.aijuggler.fake-desktop` and the copy-exact label `Single-account app — Juggler tracks, cannot isolate` (asserted verbatim) | PASS | 2026-10-14 | zcode-primary |
| Req.1 label as data, never guessed | `cargo test -p juggler-app --lib session_info_desktop_isolation_label_maps_per_provider` | desktop rows carry the per-Provider claim; terminal rows and unregistered providers `null` | `InstanceInfo.isolation_label` resolved from the adapter registry (`desktop_isolation_labels`) at read time; unit test asserts label verbatim on desktop rows, `None` on terminal rows and on desktop rows whose provider has no adapter — the panel never invents a limit | PASS | 2026-10-14 | zcode-primary |
| Req.2 same ledger, no PTY | `desktop_launch_tracks_in_the_same_ledger_without_a_pty` | tracked incl. children/exit/crash; no PTY allocated | `Supervisor::spawn_desktop`: null stdio, `ChildHandle` in the same spawn ledger, journal `spawned`/`exited` lines, kernel wait thread → `LifecycleEvent::Exited`; `write_input` refuses (`NotATty`, asserted); stop through the shared TERM→KILL ladder → `Stopped`; crash fixture exits `ExitKind::Code(7)` (asserted) | PASS | 2026-10-14 | zcode-primary |
| Req.2 restart reconciliation | `restart_reconciler_closes_dead_desktop_rows_never_fake_live` | dead rows closed after restart, never fake-live | fresh `AppState` over the same data dir reconciles the desktop row out of `Starting` (kind-blind E410 reconciler) | PASS | 2026-10-14 | zcode-primary |
| Req.3 desktop surface, no terminal card | `pnpm --dir ui test` (desktop-surface, 8 cases) | status/account/project/resources + `Open / Focus window` primary; terminal card NOT rendered | `InstanceOverview` desktop branch: `desktop-surface` card (status chip, `app` chip, provider, account · project, cpu/ram/children, tokens where readable), primary `Open / Focus window` (`desktop-focus`), amber `desktop-isolation` callout (copy-exact), focus outcome tier text after the attempt (`desktop-focus-outcome`); terminal card absent (asserted), hand-over section absent (asserted); exited/crashed state lines copy-exact (`app exited · relaunch from Actions` / `app crashed · exit details on the Processes tab`); terminal instances keep card + hand-over (regression case) | PASS | 2026-10-14 | zcode-primary |
| Req.3 focus tier honesty | `desktop_focus` command + bridge | outcome states which tier ran, never overstated | `App not running — Relaunch?` when terminal; opener app-activation tier returns `App opened — exact-window focus lands with AJ-E414` — window-level raise (AXUIElement) explicitly NOT claimed until AJ-E414 | PASS | 2026-10-14 | zcode-primary |
| Req.4 CLI-engine preference | `providers_without_a_desktop_app_are_refused_by_name` + fake-desktop `launch_spec` | choice at Add time; CLI preferred where it exists | contract is capability-gated BOTH ways: `desktop_launch` refuses `provider claude-code ships no desktop app` (asserted); fake-desktop `launch_spec` refuses `the fake desktop app has no CLI engine` (the either/or resolved per provider). No dual-capability provider ships in the default registry yet, so the Add-time dual offer has nothing to render for — the capability bits + both launch paths are the seam (ZCode, SPEC.md:134, lands with its adapter) | PASS | 2026-10-14 | zcode-primary |
| Budgets (real app) | `cargo test -p juggler-app --test desktop real_desktop -- --ignored --nocapture` → `textedit-real-launch.log` | launch-to-tracked <3 s; exit propagation <1 s; <0.5% CPU | TextEdit (sacrificial; fresh untitled window, no documents touched): tracked_in=2.3 ms; stop→exit-observed 30 ms with journal `spawned`/`exited` at the same ms timestamp; no PTY/readers/polling — one passive kernel wait thread, stats ride the shared tick | PASS | 2026-10-14 | zcode-primary |
| Quality gates | `just verify` | fixture app launches/tracked/exits/reconciled; no credential or login automation | verify green (fmt, clippy `-D warnings`, workspace tests, docs, evidence, terminology); desktop.rs 4/4 + the ignored real-launch record PASS locally; `desktop_focus` uses only `opener.open_path` — no credentials, no login automation, no AX permission requests | PASS | 2026-10-14 | zcode-primary |
| Visual | `pnpm --dir ui visual:capture` | desktop panel + rail `app` chip, 1280x800/1024x680 light/dark; states running, exited, crashed, single-account warning | 96/96 incl. new `13-desktop/` (6 captures): running dark 1280x800, running light 1024x680, exited, crashed, single-account warning, rail chip — machine-vision check on the running capture confirms: Running chip, `cpu 2% · ram 84 MB · children 1`, amber isolation callout verbatim, `Open / Focus window` primary, NO terminal card | PASS | 2026-10-14 | zcode-primary |
| Regression | full suites | prior gates intact | 133/133 component tests (8 new); `pnpm --dir ui build` green; existing galleries byte-stable (determinism test green) | PASS | 2026-10-14 | zcode-primary |

## Notes for the next agent

- E414–E415 remain in E4 after this flip: AJ-E414 (Focus Window —
  AXUIElement exact-window raise, window identity at launch, permission
  explainer, palette/hotkey) and AJ-E415 (desktop live preview).
- The desktop surface lives in `RightInspector.tsx` `InstanceOverview`
  (NOT WorkPanels — that `SessionPanel` is the login-terminal panel
  only; AJ-E406 moved instance detail to the inspector). The focus
  source is the injected `focusBridge` seam (E410 pattern): static
  bridge imports inside RightInspector break vi.mock hoisting for
  every inspector test — inject, don't import.
- `desktop_focus` records the launch cwd at spawn (the account state
  dir); AJ-E414 must persist the window identity (owning PID, bundle
  id, window title, CGWindow id) per Req.2 — extend `DesktopLaunchInfo`
  / the InstanceRecord, don't re-derive.
- The rail sorts attention rows first — visual captures must target
  rows by status text (`hasText`), never `nth()`; the running capture
  initially grabbed the failed row and was caught by vision-check +
  a `toContainText("Running")` assert (now in the spec).
- The isolation label is registry-resolved per read, NOT persisted on
  the row: a provider changing limits updates every desktop Instance
  at once. Keep `session_join_indexes`/`desktop_isolation_labels` in
  mind when adding per-provider desktop metadata.
- Real-launch record is `#[ignore]`d (no GUI in CI): rerun with
  `cargo test -p juggler-app --test desktop real_desktop -- --ignored
  --nocapture` when the contract changes.
