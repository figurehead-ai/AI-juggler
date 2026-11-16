# Feed summaries and an orchestrator bot: options (research, 2026-11-16)

This is research for a discussion, not a decision.

The research was gathered by a Sonnet research agent reading the web only, and checked by the coordinating session. It cites vendor pages wherever possible. **UNVERIFIED** marks a claim we couldn't confirm.

## The question

The maintainer's words:
- **The feed:** Home's feed should read like a Twitter feed: "small excerpts about what is happening with each agent, along with alerts and asks", so the user can "scroll through quickly and see how they are all progressing".
- **The long-term goal:** "our own AI Bot agent that is overseeing progress, alerting and deciding when and what to bring up to user, eventually even making decisions for user. Like a true orchestrator". A paid cloud tier may host it.

## Option A — excerpts from what the agents already write (no AI model)

**What the tools give us:**
- **Claude Code**
  - Its hooks fire when a turn ends (`Stop`, `SubagentStop`) and carry `last_assistant_message`, the agent's own closing text.
  - Other hooks report permission prompts, idle, task completed and failures.
  - Anthropic says the transcript file format "is internal… and changes between versions", so hooks are the supported route. https://code.claude.com/docs/en/hooks
  - Task lists (TaskCreate/Update, formerly TodoWrite) are **not on by default for the newest models**. They're on for Opus 4.7, Sonnet 4.6, Haiku 4.5 and earlier, and in background and cloud sessions. So "3 of 7 tasks done" isn't always available. https://code.claude.com/docs/en/tools-reference
- **Codex:** `codex exec --json` streams turn events: agent messages, commands, file changes and plan updates. https://learn.chatgpt.com/docs/non-interactive-mode.md
- **Git:** files changed, diff size, commits and test exit codes are local and exact.

**A precedent:** Claude Code's own agent view does exactly this mid-turn. Its row text "updates at most once every 15 seconds from the session's own recent output without sending a model request". At the end of each turn, a Haiku-class model writes a summary, billed to the user's own plan. https://code.claude.com/docs/en/agent-view

**What it can produce:**
- state (working, needs you, done, failed);
- a trimmed excerpt of the agent's own last message;
- files, tests and commits;
- task progress when a list exists;
- alerts.

**What it can't do:** rephrase a long or rambling message into one good sentence.

**Cost:** none. **Privacy:** fully local. **Platforms:** all, through each tool's adapter. **Terms risk:** low.

## Option B — an AI model running on the Mac

**B1. Apple's built-in model (the Foundation Models framework)**
- **Requirements:** macOS 26 or later, an M1 or newer Mac, Apple Intelligence switched on, and a supported language and region (not mainland China). https://support.apple.com/121115
- **Size and context:** about 3B parameters (Apple tech report). It reads about 4,096 tokens at a time on macOS 26, and Apple's WWDC26 material says 8,192 on macOS 27 for newer devices. https://developer.apple.com/documentation/foundationmodels/managing-the-context-window.md
- **Cost:** free, on-device, works offline, with no request limits (WWDC26 session 319). There's no download for us to ship.
- **For our app:** it's Swift-only, so our Rust app needs a small Swift bridge. Community crates exist (`fm-rs`, `foundation-models`); their maturity is UNVERIFIED.
- **Quality:** Apple claims parity with similarly sized models. Independent audits are critical:
  - one reports made-up answers on 69% of false-premise questions, and refusals of 18% of harmless summaries (preprint, not peer-reviewed);
  - another found invented associations in 15% of summaries.

  So it should only **shorten the agent's own text**, never add facts.
- **Terms:** Apple's acceptable-use list bans, among others, unsupervised high-risk decisions (employment, medical, legal, finance). Nothing in it forbids summarising. Use by apps sold outside the App Store is UNVERIFIED.
- **Also on macOS 27:** Apple's Private Cloud Compute model, with a 32K context, no token cost to the developer, a per-user daily quota and an entitlement to apply for, for apps under 2M downloads. Background use is UNVERIFIED.

**B2. Small open models we'd run ourselves** (optional download)

| Model | Licence |
|---|---|
| Qwen3.5-4B | Apache-2.0 |
| SmolLM3-3B | Apache-2.0 |
| Phi-4-mini | MIT |
| Llama 3.2 3B | community licence with conditions |
| Gemma (newer versions) | licence to confirm |

- **Size:** about 2–3 GB at 4-bit (UNVERIFIED).
- **Engines:** llama.cpp (Rust crate `llama-cpp-2`) or mistral.rs, both of which also run on Windows and Linux. Apple's MLX is mostly Mac-only.
- **Speed and power:** older benchmarks put a 7B model at roughly 14 tokens a second on an M1 and 24 on an M4, and a small study measured about 1 joule per token on an M1 Pro. A one-line summary costs about one to two seconds of power. Triggered only when a turn ends, that's small.

**B3. The user's own Ollama or LM Studio, if installed:** an easy optional hook-up, with no download from us.

## Option C — the user's own Claude, Codex or Gemini doing the summarising

- **Anthropic:** subscription sign-in is "intended exclusively for… ordinary use of Claude Code and other native Anthropic applications". Third-party developers may not "route requests through Free, Pro, Max, or Ultra plan credentials on behalf of their users". https://code.claude.com/docs/en/legal-and-compliance
  - `claude -p` draws from the user's plan limits (Anthropic help centre, 2026-10-07).
  - Whether an app may start the user's own `claude -p` for background work **isn't addressed anywhere**.
  - Anthropic blocked third-party access in 2026 (press coverage).
- **OpenAI Codex:** API keys are "the recommended default for automation". Nothing found on third-party apps using a ChatGPT sign-in. https://learn.chatgpt.com/docs/auth
- **Google Gemini CLI:** third-party access is "a violation… grounds for suspension". Account suspensions have been reported. https://www.geminicli.com/docs/resources/tos-privacy

**Takeaway:** it would cost the user part of their plan, and puts their accounts at real risk.

## Option D — a cloud service with our own API keys (a premium tier)

Assuming 200 summaries a day, each reading about 2,000 tokens and writing about 150 (our arithmetic):

| Model | Cost per active user per month |
|---|---|
| Fireworks Nemotron 3.5 Lightning | $0.78 |
| OpenAI gpt-5-nano | $0.96 |
| Claude Haiku 5.5 | $1.65 ($0.83 in batch) |
| OpenAI gpt-6-luna | $1.65 |
| Gemini 3.1 Flash-Lite | $4.35 |
| Claude Haiku 4.5 | $16.50 |

**Data retention:**
- **Anthropic:** not kept by default, except some models need 30 days; zero retention via the sales team.
- **OpenAI:** abuse logs kept up to 30 days; zero retention by approval.
- **Fireworks:** zero retention by default.
- **Groq:** no retention by default.

**What it needs:** opt-in, redaction of secrets and paths, per-project exclusion, and a clear statement of what is sent.

## Option E — the orchestrator bot

**Comparable products:**
- **Claude Code agent teams** (experimental): a lead with a shared task list and hooks. They use many more tokens.
- **Claude Code auto mode:** a separate checker reviews risky actions and pauses back to the user after 3 blocks in a row or 20 in total.
- **GitHub Copilot coding agent:** one branch and one pull request per task, with session logs.
- **Cursor cloud agents:** separate virtual machines, plus a dashboard.

**Autonomy frameworks:** MIT Media Lab's levels A1–A5 run from Directed (approve every action) to Independent. The Cloud Security Alliance's levels 0–5 range from per-action approval up to monitoring only. Gartner warns about "approval fatigue".

**Risks:**
- **Destructive actions:** an unchecked agent deleted a production database (Replit, July 2025).
- **Prompt injection.**
- **Token cost.**
- **Account suspension** under the providers' terms.

**How it fits AI Juggler:** our pool levels (Read / Work together / Full) and the calm attention levels map naturally onto a staged bot:
1. an advisor that triages and drafts;
2. acting within pool rules, with approval gates, an audit log and cost caps;
3. more autonomy, earned over time.

## Comparison

| | Quality | Privacy | Cost to us | Cost to the user | Platforms | Effort | Terms risk |
|---|---|---|---|---|---|---|---|
| A. Agents' own text | exact, can't rephrase | local | none | none | all | per adapter | low |
| B1. Apple model | small; may invent | local | none | battery | Mac 26+, Apple Intelligence | Swift bridge | low (Apple's list) |
| B2. Open small model | 3–4B class | local | hosting the download | disk, RAM, battery | all | runtime and model management | low (licences) |
| C. User's CLI | frontier | goes to the vendor | none | the user's plan | where installed | low | **medium–high** |
| D. Our cloud | frontier or open | leaves the Mac (opt-in) | ~$1–2 per user per month | the subscription | all | backend, billing, redaction | low–medium |

## Open questions

- Whether an app may start the user's own `claude -p`.
- Apple's context size and background-use rules.
- Real summary quality of small models on agent transcripts.
- Cloud latency.
- Gemini API zero-retention terms.
