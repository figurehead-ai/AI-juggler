# Evidence manifest: AJ-E3823 (one engine-setup guide for Bot; D-144)

Built 2026-11-23 on release/0.5.1 (Opus). Preview build with the mocked backend
(`ui/src/preview/bot.ts`, `ui/src/preview/ai-models.ts`), Playwright captures of the
Bot and Models galleries, both themes; axe clean (0 critical, 0 serious) on every
state at 1280. Generic fixtures only. Live walk on the real Mac: still to do (the
persona walk, Sonnet; Opus verifies). The Apple helper was never run here.

| Req | State | Observed | Verdict | Captures |
|---|---|---|---|---|
| R1 | Probe blocked (helper killed by the system) | `Seen::Blocked` → `AppleAi::Blocked` → "AI Juggler's helper was blocked by macOS", what to do, Check again; recommendation `apple_ai_unknown`, never a download | PASS (unit tests) | captures/bot-setupblocked-dark.png, captures/models-blocked-dark.png |
| R1 | Probe unanswered (hello timed out) | "Couldn't check: no answer", answer the system's message then Check again | PASS (unit tests) | captures/bot-setuppage-dark.png |
| R2 | Checking first | "Checking what this Mac can do…" as a status until the first read | PASS (vitest) | |
| R2, R4 | Apple Intelligence off | "Recommended: Turn on Apple Intelligence (free, nothing to download).", the three steps, Open Apple Intelligence settings (opens Apple Intelligence & Siri, no prompt), Check again | PASS (vitest, gallery) | captures/bot-setupoff-light.png, captures/bot-setupoff-dark.png, captures/models-appleoff-dark.png, captures/models-appleoff-light-900.png |
| R2, R4 | This Mac can't run Apple's | one line says so; Qwen3 1.7B recommended, "Download Qwen3 1.7B · 1.8 GB" primary, free disk shown | PASS (vitest, gallery) | captures/bot-nomodel-dark.png |
| R2, R4 | Apple's ready, Bot's AI off | Apple's recommended, "Use Apple's AI" (turns on Bot's answers only) | PASS (vitest, gallery) | captures/bot-setupready-light.png |
| R5 | What is this? | what it is, where kept, where from (the Qwen team, Hugging Face), nothing asked leaves this Mac, how to remove | PASS (vitest) | |
| R6 | Router order | Apple's ready card first over an installed, running Qwen with no pick (Recommended), and with no recommendation (No download) | PASS (Rust test) | |
| R7 | D-144 no AI | Bot's panel and page show only "Set up Bot": no box, no results, no shortcuts; Ask Bot hidden | PASS (vitest) | captures/bot-setupoff-light.png |
| R7 | D-144 AI ready | a conversation with cited answers; an answer that fails the check is "I don't know" with "What I checked" | PASS (vitest) | captures/bot-beta-light.png |

## Design critique (Opus, both themes, against models-screen, chat-panel and d126-rich-ui)

- Rich mode, kit parts only: the chosen-card look (accent ring on a tinted surface) marks the
  recommended choice, as the Models screen's Recommended card does; tiles, facts strip, licence
  link and Download are the Models screen's own parts. One filled primary per view in every state.
- The recommendation sits above the choice as one tinted line (sparkle, accent-soft); while
  Apple's state is unknown it turns neutral and says "nothing is recommended", so no state reads
  as a pick on a guess.
- Points carry green checks only for what holds: "Needs Apple Intelligence on" shows a neutral
  dot until it is on (first capture had a check on an unmet need; fixed).
- Two columns from 680 px of container (Models at 1280, the Bot page at 1280 when wide); stacked in
  the 400 px panel and at 900 (the first cut split at 560 and wrapped the primary; fixed).
- Other tested models' Download now names its model; its footer wraps as a row (button, then
  "Stays on this Mac") instead of squeezing the line into three.
- Mock-up drift, by design: the chat-panel mock's no-model panel (look-ups, shortcuts, the bare
  "Get it · 1.8 GB" offer) is replaced by D-144's Set up Bot.

## Close-out (0.5.1, 2026-11-23, Opus)

Each test named below was found by name and run, targeted, on `release/0.5.1` (vitest by file; cargo through `scripts/heavy.sh`, `--lib` with module filters). An OPEN line says where it closes.

R1: PASS — cargo `ai::engines::apple::tests::a_blocked_or_broken_probe_is_its_own_state`, `ai::engines::helper::tests::a_helper_that_doesnt_start_says_why`, juggler-platform `process::tests::a_killed_helper_reads_as_stopped_by_the_system`; vitest `components/ai/engine-setup.test.tsx` "…never "can't run it", never a download recommended" (2169cf6a7).
R2: PASS — `engine-setup.test.tsx` "nothing read yet: checking first", "each Apple state, and the badge…", "ready: … Qwen named, sized, with the free disk", "in the Models screen: …; in the wizard: …"; one component, no copies: `EngineSetup` is used by `setup/MeetBotStep.tsx`, `bot/BotEmpty.tsx` and `settings/ai/ModelsPane.tsx`.
R3: PASS — `engine-setup.test.tsx` "roles and words: a labelled group of two labelled choices, no bare Download, no internal words" ("Choose the small AI that writes Bot's answers"; no bare "model").
R4: PASS — `engine-setup.test.tsx` "each Apple state, and the badge…", "off: the badge says turn it on…", "this Mac can't run Apple's: … Qwen recommended…".
R5: PASS — `engine-setup.test.tsx` "ready: Apple's recommended with Use Apple's AI…", "What is this?: what it is, where it's kept, where it comes from, nothing leaves, how to remove it".
R6: PASS — cargo `ai::router_tests::apple_s_ready_card_comes_first_over_an_installed_download`; the helper's first start goes through AJ-E1547's gate (`engine-setup.test.tsx` "not set up yet: … Check this Mac"; AJ-E1547 R3).
R7: PASS — vitest `components/bot/bot.test.tsx` "…no AI ready (D-144): only Set up Bot, no question box, no results, no shortcuts", "D-144, an answer the AI couldn't check: I don't know, with the records it checked", "Ask Bot shows only while Bot's AI is ready (D-144, D-127)"; cargo `bot::tests::without_an_ai_bot_takes_no_question_and_search_writes_nothing`.

### Usability answers, matched against the built component (`EngineSetup`, `engine-setup.test.tsx`)

1. Bot is an AI that answers, and its setup says which AI writes the answers and what it costs: matches (R3's words, R7).
2. "Checking what this Mac can do…" first, then each AI in plain words, Qwen named and sized with the free disk, a recommended badge, buttons that say what they do, "What is this?": matches (R2, R4, R5 tests).
3. One component in three places; with no AI, Bot shows only this setup: matches (the three call sites; the D-144 test).
4. Apple's off, blocked, couldn't check, no AI possible: each its own words and what to do, never "can't": matches (the per-state tests).
5. The helper's first start primed ("Setting up Bot"); opening System Settings raises no prompt: matches in tests (the "not set up yet" test); the live dialog: PL1 H5, K4.
6. Watch-fors: a nameless "Download · 1.8 GB" (matches: no bare Download); a download recommended while Apple's was only unchecked (matches: none while unknown); an input box with no AI behind it (matches: D-144 test); "model" and "look-ups" unexplained (matches: the words test). How it looks in both themes: the critique above, re-checked in the critique round.
