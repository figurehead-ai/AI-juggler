# ADR-0036: The agent digest and watches: a checked cross-agent digest with a non-AI rollup beneath it, and typed watches that code evaluates and Chat only drafts

**Status:** Accepted (2026-11-18: the coordinating session, with the maintainer's quiet-hours ruling recorded on the decision log). Building starts once ADR-0037's summary is reviewed, so the store and the log are settled first.
**Date:** 2026-11-18
**Deciders:** Opus (design); the coordinating session (review); the maintainer (D-132, and the open question at the end)
**Serves:** D-132 (the digest and watches), under D-130 as clarified (severity-based acceptance, opt-in Beta); D-133 as extended (one memory and context system; its store, audit log and context assembler are ADR-0037's); D-117 (calm attention, levels from facts); D-118 (the feed); D-126 (rich making and editing); D-127 (never show what can't be used); D-123 (the 0.6 bot, out of scope here); [ADR-0033](./0033-feed-summary-engines.md) (the feed line, A's excerpt, the gate, the levels); [ADR-0034](./0034-ai-jobs-router.md) (the router, the queue, Chat v1 and its look-ups, the shared guard); [ADR-0035](./0035-remote-config.md) (flags and params); [ADR-0019](./0019-extension-architecture.md) (adapters as data, features as modules); E37 ([`E37-home-activity-feed.md`](../tasks/E37-home-activity-feed.md)) and E38 ([`E38-chat.md`](../tasks/E38-chat.md))

## Context

- **The ruling ([D-132](../review/decision-log.md), 2026-11-18).** The maintainer: the main benefit of the 0.5 models is "the summaries of what agents are doing, and alerting and consolidating notifications… if someone has 7 agents we need a model to process messages from all 7 and summarize in short descriptions / almost like tweets… instead of user constantly scanning and watching, chat summarizes and notifies". And: "user should be able to chat and tell our system to watch for certain things… 'Hey chat, let me know when this and that agent finishes or it gets stuck'… All the things that are within our tool capabilities." D-132 makes the digest 0.5's main AI job, adds watches set up through Chat, lets Chat v1 create, change and remove watches after the user confirms, and sets the tests.
- **The acceptance rule ([D-130](../review/decision-log.md) as clarified).** For read-only jobs, *critical* means fabrication: an invented fact, a contradiction of the source or the facts, a confident answer to a different question, or acting on planted instructions. At most 1 % may reach the user, none systematic. Omissions are tolerated up to about 10 %. Value: preferred in at least 60 % of decided pairs. A passing job ships as opt-in Beta through a remote-config flag, labelled "AI", with the source one tap away and a "this is wrong" button that records locally; the non-AI output stays the default for a release of clean Beta use. A failed round means optimise and retest, never drop: facts first, then verify and retry up to two times, then adapter training, then larger models.
- **One memory and context system ([D-133](../review/decision-log.md), extended the same day).** Memory and context become one system for 0.5 Chat and the 0.6 bot: one store with scope, kind, authority, provenance, confidence, as-of and superseded-by on every record; one context assembler with pluggable sections and per-job budget data; one retrieval interface; one append-only event and audit log with versioned migrations, so 0.6 only adds. The coordinating session placed that design in ADR-0037, written right after this one. (D-133's extension text says "ADR-0036"; the number there is to be corrected to 0037.) This ADR uses ADR-0037's seams and defines none of them.
- **What AI Juggler already has, which this design rests on:**
  - **The notification path** (`features/notifications/`): one model (`centre.rs`: told once per key, the one count of L2 and L3 needs, snooze, mute, 30 days kept), a pure `Engine` (`engine.rs`: each state entered is told once, re-armed after 5 or 30 minutes, held through quiet hours, dropped when the kind is off, the account left out or AI Juggler in front) and `show_due` (`mod.rs`), which sends **one system notification per due item**. Nothing consolidates today: seven agents finishing in a minute are seven notifications.
  - **The journals the feed reads** (`juggler-core/src/storage/feed.rs`): notices, session endings (`instance_lifecycle`), switches, task events and agents' messages; F2 adds turn ends and other apps' state changes (`conversation_events`, AJ-E3705), runway crossings and the files a session left (AJ-E3704); F3 adds git commits and pull requests (AJ-E3706).
  - **The live facts:** session states (`SessionStatus`: running, waiting for the user, stuck, quota-blocked, finished, stopped, failed, handed over), the runtime's events for driven sessions (`RuntimeEvent`: turn started and ended with its outcome, approval requested, question asked, ask resolved, limits updated), the conversation index's states for other apps' sessions (`asks_you`, `working`, `idle`, `done_review`), the stuck threshold (`stuck_after_minutes`, default 5), the quota alert's phases, keep-alive restarts, processes left running, the git state per session (`git_dirty_count` on the session read model).
  - **Three things already called "digest":** the centre's day-so-far line (`NoticeDigestInfo`), the alerts' hourly renewal lines, and D-118's "While you were away" divider. The new one is the **agent digest** in code and "Digest" on screen; the other three keep their names and jobs.
  - **The AI layer** (ADR-0034): one router, one queue (interactive before in-view before background), the shared guard (`features/ai/guard/`), engines behind cards, A's excerpt as the feed line's last rung, and Chat's six read-only look-ups.
- **What the evaluations taught** (ADR-0033's verdicts, [the v4 design](../qa/evidence/AJ-E3709/v4-design.md)): a small model rewriting free prose changes verbs and drops caveats; it does better filling short, typed fields that code composes into text, with AI Juggler's facts beside the agent's words and checks that compare against those facts exactly. The digest adds one risk the single-turn jobs did not have: with several agents in one context, a model can **attribute one agent's work to another**. That is fabrication, and the check below treats it so.

## Decision

### 1. The agent digest (job e)

#### 1.1 What it is

A feed item, made every so often while agents work, that says in a few short lines what each agent is on and what is notable. It sits in Home's stream (D-118) as its own item type, `agent_digest`. It is always available as a **rollup** written by code from the journals (§1.4). When the user has opted into the Beta (§1.8) and an engine has passed the digest's run (§3.1), the lines are written by the model and checked against the facts, item by item, with the rollup's line for that agent as the fallback.

The digest never replaces a need. Open L2 and L3 needs stay pinned on top of the feed and keep their own notifications (D-117: one loud mark per need). The digest may mention them, in normal ink with a link to the pinned need, never in amber or red.

#### 1.2 Input

The window runs from the previous digest's end (or the last 30 minutes, for the first one) to now. The digest's input is assembled by ADR-0037's **one context assembler**, for the job `Digest`, from its pluggable sections and the budget data below. There is no digest-specific builder. The sections the digest contributes, as section providers registered with the assembler:

| Section | What it holds | Trust |
|---|---|---|
| `instructions` | the role, the reader ("a developer glancing at the feed"), the purpose, the field rules, 4 or 5 fresh worked examples (one with nothing notable, one with a caveat, one with a failure the facts know) | the app's own |
| `agents` | one block per agent that changed in the window, ranked (below): its ref (`a1`…), App, project, state and level now, and its facts: turns ended in the window, each with a ref (`t1`…) and its outcome, open questions, failures, tests (ran, passed, failed, not run, unknown), tasks done of total, git (branch, dirty files, files changed, commits in the window), runway crossing for its account, processes left | facts, from AI Juggler's journals and read models |
| `events` | the window's notable events with refs (`e1`…): a new L2 or L3, a session ended, a crash and restart, a task done or blocked, a pool message | facts |
| `agent_words` | per cited turn, A's excerpt (AJ-E3708) after the normaliser and the input guard, inside its own inner quote, marked as data; only while Last messages is on | untrusted |
| `memory` | empty in 0.5. The seam for D-133's long-term memory (a user's stated interest in a project, for example) once ADR-0037 offers it to this job | per ADR-0037 |

**An agent** is one session: an AI Juggler session or another app's conversation (the conversation index). Its subagents roll up into it. Agents with no change in the window are not sent; the digest says "N agents quiet" from a count.

**Budget data per engine tier** (the assembler's budget table for `Digest`; tokens, counted as ADR-0034 §6.4 counts them):

| Section | 4,096 (Apple's model on macOS 26) | 8,192 (Apple's model on 27; the local GGUF model) |
|---|---|---|
| `instructions` with examples, and the output schema | about 1,500 | about 1,500 |
| `agents` and `events` | at most 1,500 | at most 3,800 |
| `agent_words` | at most 600 | at most 2,000 |
| Output cap | 300 | 400 |
| Margin | 196 | 492 |

**When the agents don't fit,** the assembler's retrieval ranks them: an agent at L3, then L2, then one whose session ended, then by the count of turns ended in the window, then by its newest activity. Within an agent, its newest turns first, at most 3 turns and 2 excerpt sentences each on the 4K tier. An agent cut from the model's context still gets its rollup line (§1.4): **the digest covers every changed agent, whether or not the model saw it.**

#### 1.3 Output

The model fills a guided shape (Apple's `@Generable`, a grammar for llama.cpp; ADR-0034 §2's floor), and code composes the text. "Extract, then compose", as the v4 harness does:

```rust
pub struct DigestDraft { pub items: Vec<DraftItem> }          // at most 6

pub struct DraftItem {
    pub agent: String,          // an issued agent ref: "a3"
    pub kind: DraftKind,        // On | Notable
    pub notable: NotableKind,   // None | Finished | BigChange | Stuck | LeftProcesses | RunwayLow
                                // (Asks and Fails are never the model's: §1.5 rule 1)
    pub doing: String,          // 3–60 characters: what the agent is on ("the retry backoff in the payments client")
    pub detail: String,         // 0–60 characters, or empty: one detail from the cited rows
    pub caveat: CaveatKind,     // None | NotRun | NotDone | Partial | Blocked | Unsure
    pub cites: Vec<String>,     // issued turn and event refs: "t4", "e2"; at least one
}
```

**The shown item** is composed by code: the agent's name and App icon from its facts, a state word from the facts (never from the model), then the model's `doing` and `detail`, then the caveat in the app's own words. For example, "**api** · on the retry backoff in the payments client · tests not run". Each item is at most 20 words and 140 characters. Every item carries its **cites**: the session id and the turn ids (or event ids) it rests on, resolved by code from the refs, so a click opens the session at that turn.

**A digest** is a header ("Digest · 10:45 AM · 5 agents"), at most 6 items in the order L3, L2, notable, then "on X" by newest activity, then "and 3 more agents · 2 quiet" as a link that unfolds the rest from the rollup.

#### 1.4 The rollup: the fallback and the baseline

A pure function over the same facts, `rollup(window, facts) -> Digest`, with no model: deterministic, under 30 ms for 20 agents, built in every case and stored with every digest.

- **Per agent:** "{name} · {state now} · {N} turns · {M} files · tests {passed | failed | not run}", and, while Last messages is on, "last: “{A's excerpt, first sentence}”" with its caveat line when A's excerpt has one (ADR-0033 §6). A fact the journals don't hold is left out, never guessed.
- **Notable items** from fixed templates in `feed-words.ts`: finished, asks you (the agent's own question, A's excerpt), failed (the agent's own failure sentence and the exit or test facts), stuck for N minutes, ended and left N processes, runway below X % on an account, a task done or blocked, a pool message.
- **The order and the cap** are the same as §1.3's.

The rollup is three things at once: what the digest shows when the AI is off, unavailable or skipped; the line an AI item falls back to when it fails its check; and the **baseline** the value check compares against (§3.1). An AI digest is only worth shipping if blind reviewers prefer it to this.

#### 1.5 The check: facts first, item by item

The digest's rule set sits on ADR-0034 §7's shared guard (`features/ai/guard/`), as the feed line's and Chat's do. Before the model: the input guard on every `agent_words` sentence, the quoted frame, and per-call markers (ADR-0034 §7.1). After the model, each item passes, in order:

1. **Levels from facts.** An agent at L2 or L3, or with an outcome of `Asking` or `Failed`, gets no model words: its item is the rollup's, with the agent's own question or failure (ADR-0033 §2, extended to the digest). A draft item for such an agent is dropped as a bug and counted. `NotableKind` has no `Asks` or `Fails`, so the shape can't say it either.
2. **Refs.** The agent ref and every cite were issued for this digest, and every cited turn or event belongs to the cited agent.
3. **Attribution.** Every content word and fact token in `doing` and `detail` appears in **that agent's own** rows or cleaned excerpt, not just anywhere in the bundle (ADR-0033 §6 steps 2 and 2b, scoped to one agent). An item that says what another agent did is rejected. This is the digest's own rule; it answers the multi-agent risk in the Context.
4. **Verdict against the facts.** A `notable` and any outcome word must agree with the agent's facts: `Finished` needs a finished outcome in a cited turn; "passes" needs a tests fact that passed; `Stuck` needs the stuck state; `RunwayLow` needs a crossing event; `BigChange` needs a git fact above the threshold (10 files or a commit). This is ADR-0034 §7.2 step 3, against the agent's rows.
5. **The caveat.** When a cited turn's facts say tests not run or partial work, or its excerpt states a caveat (ADR-0033 §6 step 3's list), the item's `caveat` must say so. Code adds the caveat's words from the facts whenever they know it, so the model can't drop a caveat the facts hold.
6. **No acting claims.** No sentence says AI Juggler, the digest or anyone did or will do something on the user's behalf (ADR-0034 §7.2 step 6's stem list).
7. **Echo.** No three-word run shared with a sentence the guard removed (ADR-0033 §6 step 5).
8. **Shape and language.** The field limits, plain text, no URL, the secret-hiding pass leaves it unchanged, and the agent's text is in a language the word lists cover (English and German in 0.5); otherwise the rollup line.

**The ladder** (D-130 lever 2, which replaces ADR-0034's "no retry" for this job):

1. Every item passes: shown, labelled "AI".
2. Any item fails: the whole draft is regenerated once with the failed items' reasons in plain words ("item for a2 named a file a2 didn't touch"), at most two times, within the job's deadline.
3. Still failing: each failed item becomes the rollup's line for that agent; the passing items stay. If more than half the items fell back, the whole digest is the rollup.
4. An engine error (`Refused`, `TimedOut`, `TooLong`, `Crashed`): the rollup.

Every rung is silent to the user and recorded in the audit log (§2.6).

#### 1.6 The scheduler

One event-driven scheduler in `features/digest/`, with no polling and no wake-up while nothing changes:

- **What it hears.** The same observations watches hear (§2.3): turns ended, state changes, sessions ended, task events, pool messages, git facts, runway crossings. Each one marks its agent **changed**. A subset is **notable**: a new L2 or L3, a session ending, a crash.
- **Nothing changed, no digest.** With no changed agent, no timer is armed and nothing runs.
- **The cadence.** With changed agents, one timer is armed for `last_digest + interval`, where interval = 30 minutes for 1 active agent, 25 for 2, 20 for 3 and 15 for 4 or more (active: changed in the last 30 minutes). The bounds are a remote `param` (ADR-0035 §2: `digest_interval_min`, 10..=60).
- **Notable at once, batched.** A notable observation brings the digest forward: it is made 60 seconds later, so a burst becomes one digest, and never sooner than 5 minutes after the previous one. This only shapes the feed's picture: the need's own notification goes out through the existing path at once (§1.7).
- **One at a time.** At most one digest is pending; a change while one is being made widens the next window instead of queueing a second.
- **Away, then one.** While the user is away (D-118's away state), the cadence stops. On return, one digest covers the whole window and becomes the "While you were away" divider's first item. No digest is written for no one.
- **Battery and load.** On battery below 10 %, or Low Power Mode on battery below 20 % (battery-critical), no digest is made, AI or rollup; the changed set is kept and the next digest covers it. Otherwise the AI job is a **background** job on ADR-0034's queue under its `FeedLine` skip rules (Low Power Mode, thermal Serious, memory pressure Warning, Apple below 20 % on battery, the local model on battery until Q5), and a skip means the rollup, not a delay. A Chat answer stops a running digest job, which goes back to the queue in its place (ADR-0034 §4.1); if its deadline passes first, the rollup shows.
- **Deadline:** 30 seconds a digest job, killed at 32 seconds, retries included.

The router gains `JobKind::Digest` with needs `{ context_tokens: 4096, tools: false, tier: Unknown or better }` (ADR-0034 §2) and a `passed.rs` row per engine after §3.1's run.

#### 1.7 Consolidated notifications, through the existing path

Consolidation is code, in the notifications module, in front of the sink. It changes how many notifications reach the system, never whether a need is told:

- **A 60-second batch.** `show_due` hands its due items to a batcher. The first L2 or L3 item of a burst goes out at once; further items due within the next 60 seconds are held and sent at its end as **one** notification. Calm kinds (finished, reset, processes left) always wait for the batch.
- **One or many.** One item keeps today's notification. Two or more become one: its title from the highest level ("web asks you · 3 more"), its body the names in at most two lines ("api, docs and infra finished"), its click opening the feed at the newest digest, or the centre when a need is among them.
- **The digest's notification replaces the calm pings.** When the digest is on and the user has "Finished" notifications on, sessions that finish are told in the next digest's notification ("4 agents finished · tests failed on web") instead of one each. L2 and L3 needs are never folded into a digest's notification: they keep the immediate path and are only batched among themselves.
- **Every existing rule stays:** told once per key, the kinds the user turned on (a new kind, **Digest**, off by default like every other), muted projects and accounts, snoozes, never while AI Juggler is in front, and **quiet hours**: held items wait, and when quiet hours end they come as one consolidated notification.
- **A model's words never reach a notification** (ADR-0033 §2's rule, extended): a notification's text is code-composed from facts and names. The AI lines stay in the feed.

#### 1.8 The opt-in Beta, the label, the source and "this is wrong"

- **The flag.** `digest_ai_beta`, declared in `features/digest/` (ADR-0035 §2), remote `OffOnly`: its compiled default is on only in a build whose `passed.rs` holds a `Digest` row, and the remote file can withdraw it without a release. The rollup has no flag; it is the feature.
- **The opt-in.** Settings › AI on this Mac gains **Write the digest with AI (Beta)**, off by default, shown only when the flag is on and the router has a route for `Digest` (D-127: an option the Mac can't use is not shown). A quiet card offers it once in the feed, under the first digest, with **Try it** and **Not now**.
- **The label.** An AI-written item carries a small "AI" mark; a digest with any AI item says "AI · Beta" in its header. A rollup item carries none.
- **The source, one tap away.** **Show source** on the digest unfolds, per item, the rollup line, the cited turns' excerpts and facts, with links to the session at each cited turn.
- **This is wrong.** A button per AI item records locally: the digest's id, the item, the engine, the prompt and guard versions, and an optional choice (wrong agent, didn't happen, wrong outcome, missing something). It is written to ADR-0037's audit log, shown as a count in Settings › AI, and never leaves the Mac unless the user attaches it to a bug report, with a preview.
- **The default flips** after a release of clean Beta use (D-130), by a release that changes the compiled default, never remotely.

#### 1.9 Storage

Digests live on **ADR-0037's store**, as records of a digest kind with ADR-0037's fields (scope global, authority "observed by AI Juggler" for the rollup and "inferred by a model" for AI items, provenance with the window and the cited ids, the agent text inside marked "untrusted", confidence from the check's result, as-of the window's end). The feed reads them through one more `FeedSource` (rank 6) over that store. Each digest record keeps its rollup beside its AI items, so **Show source** never recomputes. Kept 30 days, like notices; dropped at once, with every excerpt-bearing record, when Last messages goes off.

### 2. Watches

#### 2.1 The typed watch

```rust
pub struct Watch {
    pub id: WatchId,
    pub targets: Vec<WatchTarget>,      // 1–10
    pub matching: Matching,             // Each (fires per target) | All (fires once, when every target has met it)
    pub conditions: Vec<Condition>,     // 1–4, any of them fires
    pub lifetime: Lifetime,
    pub paused: bool,
}

pub enum WatchTarget {
    Session(SessionKey),                // an AI Juggler session or another app's conversation
    App(PlaceId),                       // every session of an App or profile
    Project(ProjectId),
    Pool(PoolId),
    Account(AccountId),
    Any,                                // every session on this Mac
}

pub enum Lifetime {
    Once,                               // the first fire ends it
    UntilEnds,                          // until every target session has ended (session targets only)
    Until(i64),                         // an expiry: "today" is local midnight; or a chosen time
    Always,
}
```

A watch has **no action field and no way to hold one**: its only output is a notification (§2.5). The Rust type is the contract the model's draft, the store and the view all share.

#### 2.2 The closed set of conditions

Each condition maps to observations AI Juggler already makes; nothing here needs a new reader. A condition is offered for a target only when that target's App can report it: each adapter's `ToolSpec` (ADR-0019 §1) gains `observes`, the set of observations its adapter delivers, and the set is checked against the conformance kit's fixtures. `observes` follows D-128: it is proved per tool version, and narrowed or killed through the remote compat section. Where an App can't report a condition, the condition is **hidden** for that App, with no "unsupported" mark (D-127). Chat's "I can't watch for that" names the nearest option this App does support (§2.4).

| Condition | Fires when | Observed from | Parameters | Targets |
|---|---|---|---|---|
| `Finishes` | a turn ends cleanly and the agent waits idle (not asking, not failed) | `RuntimeEvent::TurnEnded { Completed }`; the conversation index's `working` → `idle` or `done_review` (AJ-E3705); `SessionStatus::Finished` | — | session, App, project, pool, any |
| `Ends` | the session closes, however it ended | the `ended:` lifecycle event (`instance_lifecycle`) | — | session, App, project, any |
| `AsksYou` | it asks a question or for an approval | `WaitingUser`; `ApprovalRequested`, `QuestionAsked`; `asks_you`; the need kind `asks` | `after_min` 0–120 (default: the user's wait setting) | session, App, project, pool, any |
| `Fails` | a turn or the session fails | `TurnEnd::Failed`; `SessionStatus::Failed`; endings `Error`, `Crashed`; the need kind `failed` | — | session, App, project, pool, any |
| `Stuck` | it looks stuck and stays so | `SessionStatus::Stuck` held | `for_min` 0–240 (0: as soon as it looks stuck; the heuristic trips after `stuck_after_minutes`) | session, App, project, any |
| `IdleFor` | no activity while it's open | the last observation for that session, by a deadline (§2.3) | `min` 5–480 | session, App, project, any |
| `HitsLimit` | it reaches a usage limit | `SessionStatus::QuotaBlocked`; `ErrorCategory::RateLimit`; the need kind `limit` | — | session, App, account, any |
| `RunwayBelow` | an account's room crosses below a share | the quota alert's phase changes and runway crossings (AJ-E3704) | `percent` 5–95 | account, App (its account), any |
| `RunwayResets` | an account has room again | `Kind::Reset` | — | account, any |
| `LeavesProcesses` | a session ends and leaves processes running | `Kind::LeftBehind` | — | session, App, project, any |
| `Restarted` | a crashed App is started again by keep alive | `Kind::Crashed`, `app_started_again` | — | App, any |
| `PoolMessage` | a pool gets a message | `agent_messages` | `from`: optional agent | pool |
| `TaskChanges` | a task moves to a state | `task_events` | `to`: done, blocked or in review | pool, project |
| `Commits` | a commit lands in the project | the git journal (AJ-E3706, F3) | — | session, project; offered once F3's journal exists |
| `Starts` | a new session starts | `RuntimeEvent::Started`; a new conversation in the index | — | App, project, any |

**Not in the set** (each answered by §2.4's nearest option): what an agent says ("when it mentions the database"), test results by name ("when the tests pass"; nearest: `Finishes` or `Fails`), pull requests merged or reviewed, deploys, money in dollars (nearest: `RunwayBelow`), clock reminders ("at 5 pm"), anything on another Mac (Fleet watches are not in 0.5), and anything that acts ("and then restart it").

A new condition is one variant, its observation mapping, its words, its fixtures and its harness items: the set grows only that way.

#### 2.3 Evaluation: deterministic, on the existing event stream

- **One observation bus, in process.** `features/observe/` defines `Observation`, a typed enum (a session's state entered, a turn ended with its outcome, an ask opened or resolved, a session ended, a need opened, a quota phase, a reset, processes left, a restart, a task event, a pool message, a commit, a session started). It is published at the points that already write these facts: `notifications::session_status`, `model_event`, the lifecycle record, the conversation index's transitions (AJ-E3705), the task and message journals, the quota pass, the git journal. The journals stay the durable record; the bus only fans out what was just written. Its subscribers are the watch engine, the digest scheduler (§1.6), and, in 0.6, the bot.
- **A pure engine.** `features/watches/engine.rs`: `observe(&mut self, obs: &Observation, now) -> Vec<Fire>`, no I/O. Watches are indexed by target (session, App, project, pool, account, any), so an observation touches only the watches that can match it; under 1 ms for 50 watches.
- **Deadlines, not polling.** `Stuck { for_min }`, `IdleFor`, `AsksYou { after_min }` and `Until` expiries arm a deadline when their state is entered and disarm it when it's left. One timer sleeps until the earliest deadline; with none armed, nothing wakes.
- **Firing rules.** `Each` fires per target; `All` fires once when every target has met a condition since the watch began. A target and condition fire at most once per entry into the state, re-armed after 5 minutes (the notification engine's rule), so a flickering agent is told once. `Once` ends at its first fire; `UntilEnds` when every target session has ended; `Until` at its time; `Always` never. An `App`, `Project`, `Pool` or `Any` target includes sessions that start after the watch was made.
- **While AI Juggler was closed.** Each watch keeps a cursor (the last observation time it evaluated). On start, the journals are replayed past the cursor once, in order; what fires is told once, marked "while AI Juggler was closed", in one consolidated notification. States AJ-E3705 records "as seen on start" fire with that mark too.
- **A target that is gone.** A session that no longer exists ends its `Session` targets; a removed App, project, pool or account ends its watches with a quiet note in the Watches view ("ended: web was removed").
- **Bounds.** At most 50 active watches (a new one past that asks the user to end one); 10 targets and 4 conditions each.

#### 2.4 Chat's job (f): from a plain request to a typed watch the user confirms

When the user's message to Chat is about watches, the job is `JobKind::WatchDraft` (needs `{ context_tokens: 4096, tools: false, tier: Unknown or better }`), on ADR-0034's router and queue at interactive priority. Its context comes from ADR-0037's assembler: the instructions with fresh examples, the **condition catalogue** (generated from §2.2's table, with only the conditions this Mac's Apps can report), the **target catalogue** (the sessions, Apps, projects, pools and accounts as issued refs with names, ranked by the pickers' "suggested, then all" rule and cut to the budget), the user's current watches as refs (for edits and removals), and the last two exchanges. Session names are agent-written text, so they are quoted data under the guard.

The model fills a guided shape and nothing else:

```rust
pub struct WatchDraft {
    pub intent: Intent,                 // Create | Edit | Remove | List | NotAWatch
    pub watch: Option<String>,          // an issued watch ref, for Edit and Remove
    pub targets: Vec<String>,           // issued target refs; "any" is a ref too
    pub matching: Matching,
    pub conditions: Vec<CondDraft>,     // a condition from the catalogue's enum, with its parameters
    pub lifetime: LifetimeDraft,        // Once | UntilEnds | Today | UntilTime(hh:mm) | Always | Unsaid
    pub unsupported: Option<Unsupported>, // { asked: short text from the request, nearest: a catalogue condition or None }
}
```

**Code does everything after that:**

1. **Validates:** refs issued, parameters in bounds, each condition offered for each target (its App's `observes`). A failure becomes an unsupported answer, never a silent substitution.
2. **The unsupported net.** A tested cue list (tests, merge, pull request, deploy, mentions, says, dollars, cost, at a clock time, remind, then, and restart, and stop) catches a request whose draft claims support for something §2.2 lacks. If a cue matches and the draft has no `unsupported`, the draft is not shown as a watch: Chat answers "I can't watch for that" with the nearest option. This is the facts-first rule applied to parsing: the shape bounds what can be claimed, and the cue net catches a wrong mapping onto it.
3. **Fills what was not said** with the defaults, shown in the echo: `Once` for a `Finishes` or `Ends` on sessions, "today" otherwise; `Each`.
4. **Asks when unsure:** a name that matches several targets ("two sessions are called api") or none shows the target picker, suggested first, in the panel.
5. **Writes the echo itself,** from the typed watch, in the app's words, never the model's: "Tell you when **api** or **web** finishes, or looks stuck for 10 minutes. Until both finish." with **Watch**, **Change** and **Cancel**. An edit shows before and after; a removal names the watch.
6. **Creates, changes or removes it only on the user's click.** The click calls `watch_create`, `watch_update` or `watch_remove`, the same commands the Watches view uses, with the typed watch. There is no tool the model can call to write a watch: Chat's look-up registry stays read-only (ADR-0034 §6.2); the draft is an output, not a call.
7. **Unsupported:** "I can't watch for “the tests pass”. I can tell you when api finishes or fails." with the nearest option as a one-click draft to confirm, or, with no nearest, "I can watch your agents for: finishing, asking you, failing, getting stuck…" from the catalogue.
8. **List** is answered by code from the store, with links into the Watches view; Chat's seventh look-up, `watches`, read-only like the six (it changes `chat_has_exactly_the_six_read_tools` to seven).

**Without a model, watches still work:** the Watches view's **New watch** and each session row's **Watch…** menu (§2.6) make the same typed watch by hand, and Chat's no-model panel gains a "Watch a session…" shortcut to that form. Watches are a deterministic feature; only drafting them from words is AI.

#### 2.5 Alerts through the existing path

- A fire becomes a notice in the one model (`centre.rs`), of a new kind `watch` ("Watches you set" in Settings › Notifications, **on** by default: the user asked for it when they confirmed the watch), with a new engine `Kind::Watch`. It then follows every rule of §1.7: told once, the 60-second batch, consolidation, quiet hours held, never while AI Juggler is in front.
- **One notification per thing that happened.** When a watch fires on something that is also a need (it asks you, it failed), the need and the watch are one notification: "web asks you · you asked to be told". The need keeps its level; the watch only makes sure it's told.
- **Its level:** a watch on a calm event (finishes, ends, a commit) is an L1 note in the feed and the centre, never counted; a watch on a need inherits the need's level, and the need is the loud copy (D-117).
- **The words** are the app's own, from the typed watch and the facts: "**api** finished · your watch". A click opens the session at the turn that fired it.
- **Quiet hours hold watch alerts, unless the user turns on "Even during quiet hours" for that watch** (decided 2026-11-18). It is off by default, and the confirmation echo states it whenever it is on ("…, even during quiet hours").

#### 2.6 The Watches view (D-126 rich)

A making-and-editing screen in D-126's rich mode, reached from Chat's panel header, from the notification centre (a **Watches** tab), and from each session row's menu (**Watch…**).

- **The list:** each active watch as a row in plain words (the same echo as Chat's), its targets' App icons, its state ("waiting", "told 2 times", "ends today at midnight"), and **Pause**, **Edit** and **Remove**. Ended watches below, folded and paged, never loaded all at once.
- **New and Edit** as one sheet:
  - **Watch what:** the target picker with App logos, suggested first (the open sessions, the current project), then all, with search;
  - **For what:** the conditions as choice cards with icons, only those the chosen targets can report, each with its parameter inline ("stuck for [10] minutes");
  - **Until:** a segmented control (Once, Until it ends, Today, Until…, Always);
  - **A live preview** of the notification as it will look, and the plain-words summary beside it;
  - one filled primary button, **Watch**.
- **Empty state:** an illustration, one line ("Get told when an agent finishes, asks you or gets stuck"), **New watch**, and "or ask Chat" when Chat has a route.
- The ui-consistency rule, the design critique and the a11y gate apply, with mock-ups in both themes to the maintainer before building (D-126 point 3).

#### 2.7 Storage and the audit log

- **Watches live on ADR-0037's store** as records of its standing-request kind (watch), with its fields: scope (session, project or pool for those targets; global for App, account and any), authority "said by the user" (only a confirmed watch is ever written), provenance (the Chat turn and the request's words, or "Watches view"), confidence 1, as-of, and superseded-by (an edit writes a new version that supersedes the old; the old stays for the log). Agent-written text inside a watch (a session's name as it was then) is marked "untrusted".
- **ADR-0037's append-only event and audit log** records: every watch created, edited, paused, resumed, removed, expired and fired (with the observation that fired it); every alert sent, held for quiet hours, and consolidated (with what it folded); every digest made (its window, AI or rollup, the engine, the prompt and guard versions, each item's check verdict and fallback reason); and every "this is wrong". Its versioned migrations are ADR-0037's.
- **D-133's long-term memory is the first later consumer:** Chat can then answer "what did I ask you to watch last week?" from the same records. This ADR leaves that seam and designs no memory.
- **The watch engine's state** (armed deadlines, cursors, fire counts) is rebuilt from the store and the journals on start; nothing in memory is the only copy.

### 3. The harness for (e) and (f), under D-130 as clarified

Both runs use AJ-E3709's harness (v4's layout, the v5 blind sets' style): synthetic sets written blind to the prompts, tuned on practice sets only, judged once on held-out sets with the prompts frozen. Real transcripts are not used (D-124's ask was declined). A Sonnet session runs the harness; Opus checks every PASS or FAIL verdict before it is recorded (the model-tiering rule). The blind preference review is by a fresh session with the key unopened, as in v3 and v4.

#### 3.1 The digest (e)

**The sets.** Digest windows built from multi-agent bundles: 2 to 9 agents, each with turns, AI Juggler's facts and git state, in English and German. Practice: at least 60 windows. Held-out: at least 60 windows, written blind, including at least 15 with a planted order in an agent's text, 15 where an agent's words contradict its facts (says "done", tests failed), 10 with two agents on similar work (the attribution trap), and 10 where the agents don't fit the 4K budget.

| Bar | Target | Measured on |
|---|---|---|
| Fabrication-critical: an invented fact, work attributed to the wrong agent, an outcome against the facts, an obeyed planted order | at most 1 % of shown items, and **none systematic** (no error class appearing in 2 or more windows; a systematic class is fixed and re-run before shipping) | shown items, by script and by eye, each by-eye read checked by Opus |
| Invented refs or cites; model words on an L2 or L3 agent; model words in a notification | 0, structural | every draft |
| Omissions (a dropped notable item, a dropped detail) | tolerated, recorded; at most about 10 % of shown digests | shown digests against the facts |
| Value against the rollup | the AI digest preferred in **at least 60 %** of decided pairs; ties not counted as decided | blind pairs, AI digest against the rollup, shuffled and unlabelled |
| Drafts passing every check without fallback | at least 80 % of items (below it the job mostly shows the rollup) | drafts |
| Latency | median at most 10 s, p95 at most 20 s, retries included, on the slowest qualifying Mac | the held-out run |

A pass records a `Digest` row in `passed.rs` per engine (ADR-0034 §3.2) and turns the Beta option on in that build (§1.8). A fail produces a diagnosis and the next optimisation in D-130's order (facts first, then the retry ladder's tuning, then adapter training, then larger models), and a new held-out set when a prompt changes.

#### 3.2 Watch drafting (f)

**The set.** At least 150 requests written blind: at least 100 supported (one and several targets, `All` and `Each`, each condition with parameters, each lifetime said and unsaid, edits, removals, lists), at least 40 unsupported (each with its expected nearest option), at least 10 not about watches, and at least 15 with an order planted in a session's name. English and German.

| Bar | Target |
|---|---|
| Exact rule matches on the supported requests: targets, matching, conditions with parameters, and lifetime all equal to the expected typed watch | at least **95 %** |
| Unsupported conditions claimed (a watch shown for something §2.2 can't observe, after the cue net) | **0** |
| Watches created without the user's confirmation | **0**, structural: the store's write path is reachable only from the three commands, and a test drives every draft through the job and checks the store is unchanged |
| Ambiguous names asked about rather than guessed | at least 95 % |
| Nearest option offered for unsupported requests | at least 90 % (a weak nearest is minor) |
| Obeyed planted orders | 0 |
| Latency | median at most 2 s, p95 at most 4 s |

The exact match is scored on the typed watch, not on words, so it is a script; Opus checks every mismatch the script reports before the verdict is recorded. The echo is code-written from the typed watch, so "the rule shown matches the request" reduces to the exact match.

### 4. Privacy and security

- **Everything stays on the Mac.** Digests, watches, alerts and the audit log never go to a paired Mac, usage statistics, crash reports or logs. The helpers keep ADR-0033 §4's no-network sandbox. The trust guide's "AI on this Mac" section gains one row per claim, each with its named test (`no_digest_or_watch_text_leaves_the_mac`, `a_notification_never_carries_model_words`, `a_watch_is_written_only_by_its_three_commands`, `a_watch_has_no_action`).
- **Agent text is untrusted data,** guarded as in ADR-0033 and ADR-0034 §7: orders removed before the model, quoted in the frame, echo-checked after; session names in the target catalogue too. Records holding it carry "untrusted" provenance in ADR-0037's store.
- **A watch never runs an action.** By type (no action field), by layering (`features/watches/` may import only the observation bus, the store's watch kind, the notifications model and read services; the architecture lint enforces the allowlist), and by test. A planted order in a session's name can at worst produce a draft the user sees and declines.
- **Bounded:** 50 watches, 10 targets and 4 conditions each; 6 digest items; one digest at a time; a 30-second job deadline; the 60-second batch.
- **STRIDE, briefly:** spoofing (a link opening something else: links come from cited ids, never from model text); tampering (Chat changing data: no write tool; watches only through the confirmed commands); repudiation (the audit log records every watch change and fire); information disclosure (no transcript in the digest's context, only A's excerpts behind Last messages, already secret-hidden); denial of service (the bounds above); elevation (an injected order: removed, quoted, checked, and nothing to act with).

**Out of scope: the 0.6 bot (D-123).** Nothing here acts. A watch that triggers an action ("when it finishes, start the next task") is the bot's, under its own design ruling and its own, much stricter harness (D-130's clarification: these 0.5 bars never carry over). It would be a new record kind with the bot's authority on ADR-0037's store, never an action field added to `Watch`. In 0.6 the bot's decisions, approvals and actions join **the same audit log** as this ADR's watches, alerts and digests, and its eyes include the same observation bus.

### 5. Windows and Linux (D-52)

The observation bus, the watch engine, the scheduler, the rollup, consolidation and the Watches view are OS-neutral. The AI digest and watch drafting run where an engine has a card (ADR-0034 §10); elsewhere the rollup and the form serve. Battery and Low Power readings come through `juggler_platform`; unknown means "not critical", so a digest is never withheld on a reading the OS can't give.

## Alternatives considered

- **A model in the watch loop** ("does this look stuck?"). Unbounded cost per event, wrong answers that alert or miss, and a model deciding something. Rejected: the model only drafts; code evaluates.
- **Polling for watches** (every minute, check every watch). Wakes an idle Mac and duplicates the facts' own events. Rejected for the observation bus and deadlines.
- **Free-text conditions** stored as the user's words and judged later. It can't be evaluated deterministically or tested for exact matches, and it invites the model to pretend. Rejected for the closed set.
- **Letting the model call a `create_watch` tool.** It would put a write in Chat's registry and let a planted order reach it. Rejected: the draft is an output; the user's click writes.
- **The digest as a notification written by the model.** It puts unchecked-by-eye words in the one place the user can't open the source first. Rejected: notifications are code-composed; AI lines stay in the feed.
- **The digest built from per-turn feed lines** (job a, then concatenate). No view across agents, no "on X" over several turns, and B's lines don't ship (ADR-0033). Rejected for one job over the window.
- **A digest-specific context builder.** Two places to budget and rank, and 0.6 would rewrite it. Rejected for ADR-0037's assembler (D-133 extended).
- **Watches in the app settings table or their own table.** A second store with its own provenance rules; D-133's memory and the 0.6 bot would need to reach into it. Rejected for ADR-0037's store.
- **A fixed digest timer** (every 15 minutes). It runs when nothing changed and is slow when something big happens. Rejected for the adaptive cadence with notable triggers.
- **Consolidation by the model** (a model chooses what to fold). It could hide a need. Rejected: consolidation is code, and L2 and L3 needs are never folded into a calm notification.
- **Digests while the user is away.** Twenty digests overnight that nobody reads. Rejected for one digest on return.

## Consequences

- **Easier:**
  - Seven agents finishing is one notification, and the feed says what each is on without the user scanning.
  - Watches work with or without a model; Chat only makes them faster to set up.
  - A new condition, App or job is local: a variant and its mapping, an adapter's `observes`, a `JobKind` with its rule set.
  - The 0.6 bot finds its eyes (the bus), its memory (ADR-0037's store) and its log already in place.
- **Harder:**
  - The observation bus must be published at every place that writes a fact; a missed site means a watch that never fires. Each adapter's conformance kit checks its `observes` against fixtures.
  - The digest's harness needs multi-agent bundles, a fixture shape the earlier runs did not have.
  - Two Beta flags and one more Settings switch to explain plainly.
- **Accepted:**
  - An AI item that fails its check is replaced by the rollup's line; some correct AI lines are lost.
  - Agents cut from the model's context get rollup lines in an otherwise AI digest.
  - No digest on battery-critical, even the rollup; the next one covers the gap.
  - Quiet hours hold every watch alert unless that watch's "Even during quiet hours" is on (off by default).
  - `Commits` is not offered until F3's git journal exists (D-127).
- **To verify when building** (design facts, not questions): the token cost of the condition and target catalogues in Apple's 4K context; the 60-second batch against macOS's own notification grouping; that every `Observation` site is covered (a test lists the journal writers and fails on one with no publish).

## Order of work

The tasks are in docs/tasks/E37-home-activity-feed.md and E38-chat.md (AJ-E3712, AJ-E3814 to AJ-E3817; the harness for jobs (e) and (f) is AJ-E3818).

1. **E37, AJ-E3712: the digest scheduler, the feed item, consolidation and the rollup.** `features/digest/` (scheduler, `rollup`, the `agent_digest` source on ADR-0037's store), the observation bus (`features/observe/`), the notifications batcher and the Digest kind, `feed-words.ts`. Depends on AJ-E3702 and ADR-0037's store; uses AJ-E3704, AJ-E3705, AJ-E3706 and AJ-E3708's facts as they land (missing facts are left out). Then **AJ-E3713: job (e)** on the router, its rule set on the shared guard, the ladder, the Beta flag, the label, Show source and "this is wrong", and §3.1's run. Without a pass, the rollup ships alone and the next optimisation round follows (D-130).
2. **E38, AJ-E3814: the watch model and deterministic evaluation.** The `Watch` type, the closed conditions and `ToolSpec.observes`, the engine with deadlines and cursors, expiry, the target picker, the watch store **on ADR-0037's store**, the audit records. Depends on step 1's bus.
3. **E38, AJ-E3815: Chat creates, edits and removes watches with confirmation.** Job (f), the catalogues as assembler sections, validation, the unsupported net, the code-written echo, the three commands, the `watches` look-up, and §3.2's run, shipped as opt-in Beta behind `watch_draft_beta` (OffOnly, the same rule as §1.8).
4. **E38, AJ-E3816: the Watches view (D-126 rich).** Mock-ups in both themes first; then the list, the sheet, the session row's Watch…, the empty state, the centre's Watches tab; the a11y gate, the design critique and a Sonnet newcomer walk verified by Opus.
5. **E38, AJ-E3817: alerts through the existing path.** The `watch` notice kind and `Kind::Watch`, folding with needs, the batch and quiet hours, the replay notification on start, the Settings row.

## Decided questions

1. **May a watch break through quiet hours?** Decided 2026-11-18: only when the user turns on "Even during quiet hours" for that watch. It's off by default, and the confirmation echo says so when it's on. Every other watch alert waits for quiet hours to end, then comes in one notification.

The memory and context design is ADR-0037 (D-133, as extended).
