# Evidence manifest — AJ-E2404

Task: `docs/tasks/E24-processes.md` AJ-E2404, The Processes page. Branch `e24-processes`. Status: **IN-PROGRESS**.

## What exists now

`ui/src/screens/processes/` (the page, the view, the tree table, cells, row actions, toolbar, the stop and group sheets, history, the rules panel, the automatic cleanup's banners), `ui/src/lib/ipc/processes.ts`, `ui/src/preview/processes*.ts` (the made-up Mac), on 0.1.5's Page, AppIcon, type scale and KillSheet. Cleanup's screen, its Ports view and its rows are retired into it; the nav, the top bar's button, ⌘⇧C, This Mac's Clean up and a session's Processes tab lead here.

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 the views: Needs attention first, Everything running, Ended, Ports, Cleanup history | `pnpm exec vitest run src/screens/processes-page.test.tsx` | each view shows its rows | pass (8) | PASS | — | 2026-10-26 | agent |
| Req.2 the tree: folding, keyboard tree navigation, windowed rows | `pnpm exec vitest run src/lib/process-states.test.ts src/screens/processes-page.test.tsx`; `processes-gallery.spec.ts` "the large Mac" | a healthy session is one line; ↑ ↓ ← → Home End; ≤ 60 rows in the DOM at 2,000 processes | pass; large Mac ≤ 60 rows | PASS | — | 2026-10-26 | agent |
| Req.2 keyboard: Enter on a row opens Stop…, Esc returns focus to the row | `pnpm exec playwright test tests/visual/keyboard-flows.spec.ts -g "processes"`; `capture.spec.ts -g "keyboard-only pass"` | focus inside the sheet, back on the row | pass | PASS | — | 2026-10-26 | agent |
| Req.3 filters off by default, search `port:5173`, Include other apps (names and memory only) | `pnpm exec vitest run src/screens/processes-page.test.tsx`; `cargo test -p juggler-app --test processes_graph the_rest_of_the_mac_shows_only_when_asked_for_and_only_names_and_memory` | as asserted | pass | PASS | — | 2026-10-26 | agent |
| Req.4 row actions: Stop… (the verified sheet), Keep, Remind me, Go to session, Reveal folder, Copy PID; Clean up (N)… with a line per process; another app's: its own sheet, no wider scope | `processes-gallery.spec.ts` (menu, stop, stop-theirs, group-cleanup); `capture.spec.ts -g "kill-sheet"` (16 states) | each opens; outcome per process | pass | PASS | — | 2026-10-26 | agent |
| Req.5 Cleanup is this page; nav, top bar, ⌘⇧C, This Mac and a session's tab lead here | `pnpm exec vitest run` (app, processes-tab, this-mac-page, system-notifications); `home-gallery.spec.ts`, `this-mac-gallery.spec.ts -g "Clean up leads"` | each leads here | pass | PASS | — | 2026-10-26 | agent |
| Req.6 "zombie" in plain words where one shows | `pnpm exec vitest run src/screens/processes-page.test.tsx src/components/processes-tab.test.tsx` | the explanation shows with a zombie row, not without | pass | PASS | — | 2026-10-26 | agent |
| Budget: a page ≤ 50 ms | `cargo test --release -p juggler-app --test processes_graph the_graph_and_a_page_fit_the_budget_at_two_thousand_processes -- --nocapture` | ≤ 50 ms | 0.16–0.52 ms | PASS | — | 2026-10-25 | agent |
| Budget: first paint ≤ 300 ms | `AIJ_PW_PORT=4291 pnpm exec playwright test tests/visual/load.spec.ts -g "Processes page paints"` | median ≤ 300 ms | median 42 ms over 5 opens (the first, cold, 357 ms: the page's code and the preview's made-up Mac loading on first use; the app fetches both, and now the first rows, in idle time after its first paint) | PASS | — | 2026-10-26 | agent |
| Quality: the a11y gate, 0 serious | `AIJ_PW_PORT=4291 ./scripts/a11y_gate.sh --workers=4` (run locally: Docker's gate times out on this Mac) | 0 critical/serious | 840 gallery states scanned in 665 tests: critical 0, serious 0, moderate 0, minor 0; 678 of 682 tests passed, the 4 failures (48 remote cleanup still choosing the retired machine filter) fixed and re-run: pass | PASS | — | 2026-10-26 | agent |
| Visual: 1280, 1680 and 2000 px, dark and light, every capture opened | `AIJ_PW_PORT=4291 pnpm exec playwright test tests/visual/processes-gallery.spec.ts` → `docs/qa/evidence/_gallery/90-processes/` (48 captures, each scanned with axe) | looked at; axe 0 serious | looked at (all 48); axe 0 | PASS | — | 2026-10-26 | agent |
| Visual gate: the pinned container's baselines | `scripts/visual_gate.sh --update` in the pinned container | baselines rendered | not run here (Docker times out on this Mac): `gallery/90-processes/attention-1280x800-dark`, `18-ports`, `20-kill`, `23-alerts`, `24-fleet` and the regression `processes-*` shots need rendering; the retired `10-processes`, `19-cleanup` and `cleanup-*` baselines are removed | UNPROVEN | — | 2026-10-26 | agent |
| Live: the page on a real Mac | `docs/qa/processes-lab.md` steps 1–9 | as written | not run yet | UNPROVEN | — | 2026-10-26 | agent |

## Redaction checklist (tick before commit)

- [x] No credentials, tokens, or keys in any file here (secret scan ran)
- [x] Paths hashed or generalised where they reveal private layout
- [x] Transcript excerpts scrubbed of personal content (none: fixtures only)
