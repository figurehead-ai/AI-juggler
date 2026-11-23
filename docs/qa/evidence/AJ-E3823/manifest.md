# Evidence manifest: AJ-E3823 (one engine-setup guide for Bot; D-144)

Built 2026-11-23 on release/0.5.1 (Opus). Preview build with the mocked backend
(`ui/src/preview/bot.ts`, `ui/src/preview/ai-models.ts`), Playwright captures of the
Bot and Models galleries, both themes; axe clean (0 critical, 0 serious) on every
state at 1280. Generic fixtures only. Live walk on the real Mac: still to do (the
persona walk, Sonnet; Opus verifies). The Apple helper was never run here.

| Req | State | Observed | Verdict | Captures |
|---|---|---|---|---|
| 1 | Probe blocked (helper killed by the system) | `Seen::Blocked` → `AppleAi::Blocked` → "AI Juggler's helper was blocked by macOS", what to do, Check again; recommendation `apple_ai_unknown`, never a download | PASS (unit tests) | captures/bot-setupblocked-dark.png, captures/models-blocked-dark.png |
| 1 | Probe unanswered (hello timed out) | "Couldn't check: no answer", answer the system's message then Check again | PASS (unit tests) | captures/bot-setuppage-dark.png |
| 2 | Checking first | "Checking what this Mac can do…" as a status until the first read | PASS (vitest) | |
| 2, 4 | Apple Intelligence off | "Recommended: Turn on Apple Intelligence (free, nothing to download).", the three steps, Open Apple Intelligence settings (opens Apple Intelligence & Siri, no prompt), Check again | PASS (vitest, gallery) | captures/bot-setupoff-light.png, captures/bot-setupoff-dark.png, captures/models-appleoff-dark.png, captures/models-appleoff-light-900.png |
| 2, 4 | This Mac can't run Apple's | one line says so; Qwen3 1.7B recommended, "Download Qwen3 1.7B · 1.8 GB" primary, free disk shown | PASS (vitest, gallery) | captures/bot-nomodel-dark.png |
| 2, 4 | Apple's ready, Bot's AI off | Apple's recommended, "Use Apple's AI" (turns on Bot's answers only) | PASS (vitest, gallery) | captures/bot-setupready-light.png |
| 5 | What is this? | what it is, where kept, where from (the Qwen team, Hugging Face), nothing asked leaves this Mac, how to remove | PASS (vitest) | |
| 6 | Router order | Apple's ready card first over an installed, running Qwen with no pick (Recommended), and with no recommendation (No download) | PASS (Rust test) | |
| 7 | D-144 no AI | Bot's panel and page show only "Set up Bot": no box, no results, no shortcuts; Ask Bot hidden | PASS (vitest) | captures/bot-setupoff-light.png |
| 7 | D-144 AI ready | a conversation with cited answers; an answer that fails the check is "I don't know" with "What I checked" | PASS (vitest) | captures/bot-beta-light.png |

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
