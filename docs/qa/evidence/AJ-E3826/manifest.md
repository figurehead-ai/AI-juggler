# Evidence manifest: AJ-E3826 (Settings › AI › Models says what Bot uses and whether it runs)

Built 2026-11-26 on release/0.5.2 (Opus). One source: the page reads the engine status and today's counts AJ-E3825 built (`ai_engine_status`, `AiEngineStatusInfo`) through the same store as Bot's header (`useEngineStatus` in `ui/src/lib/bot-engine.ts`; a read, it starts nothing); a new choice goes through the chat job's existing pick (`ai_set_pick`). The DTO change is one field's meaning: a job whose flag is off by default answers `why: not_built` (`features/ai/service.rs`).

| Req | Verdict | Where |
|---|---|---|
| R1 Bot's AI card | PASS | `models-bot-card.test.tsx` R1; captures `inuse`, `failed`, `installed`, `offline` |
| R2 Recommended only while nothing is in use | PASS | `models-bot-card.test.tsx` R2; captures `models`, `inuse` |
| R3 each job's state and why | PASS | `models-bot-card.test.tsx` R3; `service_tests.rs`; captures every state |

R1: PASS — vitest `models-bot-card.test.tsx` "opens the page once a model is in use…" (first section, chosen in `bg-accent-soft`, "Running · answered 2 min ago", "Today · 12 answers · 8.1k tokens in · ≈ 2.3k est. out"), "failed: why and the fix in one line…", "several models: Automatic first…" (choosing writes the chat pick, Automatic writes none), "the state and totals in words…" (idle, starting, answering, off; none today; estimates labelled); `models-pane.test.tsx` "offline…" (Automatic chosen, its line names the model in use). Commits a3eac93dd, 24c953b1b.
R2: PASS — vitest `models-bot-card.test.tsx` "nothing in use: Recommended leads…" and "inuse / failed / approval / offline / installed: a model in use, so no Recommended and no In use beside it"; `models-pane.test.tsx` "one installed and in use: Bot's AI leads…". Commit a3eac93dd.
R3: PASS — vitest `models-bot-card.test.tsx` "in use: on with its model, Beta, or off and why…" (the six lines as D-149 has them, the chips lit only for Bot's AI, attention hints and the digest, no picker on Bot's row, no switch on the unbuilt review), "nothing on yet: each Beta off says what the tests found…", "a job's why is the notes of the model it would run on…", "never a bare Not yet; a job stopped by an update says so…", "a job this version doesn't have is on no model card…", "the second-model approval names only the jobs this version has"; Rust `features::ai::service::tests::on_a_mac_with_apple_intelligence_bot_waits_for_its_beta` (review: off, `not_built`) and `a_remote_kill_and_a_narrowed_verdict_each_hide_a_job` (a killed job: off, no why). builtin.json notes in step with D-149 (feed: "The plain line reads better for now."; watches: "Drafts are being made exact before Bot fills the form."; digest 64 %). Commits cd590b614, a3eac93dd.

Audit M1–M6 (`docs/qa/0.5.2-rich-mode-audit.md` §M): M1 the picker's options and field carry each model's icon (vitest "a job's picker … each with its icon (M1)"; `ModelsJobs.tsx` left `scripts/rich_mode.allow`); M2 `RowChip on` while the job is on; M3 `text-meta`, `text-label`, `type-section` (no `text-[12.5px]`, `[11px]`, `[11.5px]`, `[17px]` left in the pane); M4 the fit marks the glyph in its hue on `surface-2` / `WARNING_CHIP`; M5 no `divide-y` (jobs, installed), no `inset-ring-hairline` (cards, chips, facts, the Mac strip), no colour-mixed accent edge; M6 Recommended `bg-accent-soft` over `bg-surface`, no `shadow-card`, no `color-mix`. Seen in the gallery family `docs/qa/evidence/_gallery/99-ai-models/` and its critique (line 19: 4), commit 2c0cd4096.

## Usability answers (PAT-USE), as built

1. Someone checking why Bot answers as it does wants which AI and whether it works: match — "Bot's AI" leads the page with the model's tile, name and live state.
2. The choice, its state and today's use; each job's state and why: match — the chosen row's fill and "Running · answered 2 min ago"; "Today · … ≈ … est. out"; six job lines, never a bare "Not yet"; icons on every option.
3. One live answer, no duplicate: match — Recommended hides once a model is in use; Bot's model is chosen in the card only (its row keeps its switch); header and card read one store; model cards and Use it name only this version's jobs.
4. Nothing set up: match — Recommended (or the engine-setup guide) leads with one primary; no Bot's AI card; a failure is the reason and fix behind the warning's mark, no fill.
5. No system prompt: match — `ai_engine_status` only; nothing starts from Settings.
6. Watch-fors: match — no "In use" beside a running model, estimates labelled, no bare "Not yet", no monochrome row (rich_mode_lint OK, line 19 scored 4).

## Checks (2026-11-26)

vitest (settings, ai, setup, tour, bot: 24 files, 235 tests) pass; `tsc -b --noEmit` clean; `ui_lint.py` OK; `rich_mode_lint.py` OK (ModelsJobs left the list); `native_controls_lint.py` OK; `plain_words_lint.py` OK; `critique_check.py` OK; `debt_baseline.py` OK; `cargo test -p juggler-app --lib features::ai::` 166 pass; `cargo clippy -p juggler-app --no-deps --all-targets -D warnings` clean; `cargo fmt --check` clean; models-gallery 52 states pass axe (re-run at 650b18ae7: the radio states render the same); the gate's 16 Settings states compare unchanged. `just keep-going verify-static`: 117 of 120 on the first run; its three failures were this item's (the index row, a raw radio, a copied fixture), each fixed and re-run alone since (commits 650b18ae7 and this manifest's).
