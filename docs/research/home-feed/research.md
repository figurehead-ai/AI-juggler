# Home feed research: activity streams that inform without shouting

Written 2026-11-14. It builds on `../calm-attention/research.md`, the calm UI and attention-budget research. Sources were checked on that date. Where a source is secondary or a summary of a search result, it says so.

**The maintainer's ask:** Home in two columns.
- **Left:** a feed "more like a twitter feed", with things that need you and summaries of what agents did and finished, as if "an agent [were] tweeting about what is going on with our system".
- **Right:** the current apps, then runway by account.
- Both columns scroll on their own. The rail's Needs you stays as it is.

## 1. What the research agrees on

1. **Chronological beats ranked in work tools.** GitHub's 2023 merge of "Following" into a ranked "For you" feed drew heavy backlash: users asked for chronological order and for filters to isolate their own repositories. GitHub had already made the feed chronological in July 2022 and re-confirmed that later. A monitoring feed should be chronological, with explicit exceptions (pinned urgent items, a digest at the away boundary), never an opaque ranking.
2. **Rollups are the main defence against noise.** Every mature feed system coalesces by a group key such as verb plus object plus time window, and shows a count, a few sample actors and the newest time. This is what turns "4 finished" into one line instead of four.
3. **Seen is not the same as read.** Opening the feed marks items *seen*; acting on or opening an item marks it *read*. "Since you were away" is a boundary at the last seen time.
4. **Never move the reader's place.** New items arrive behind a "n new" pill instead of being inserted above the scroll position. Twitter/X does this, and Mastodon clients that auto-insert get complaints about jumps.
5. **Feeds need an end.** Infinite scroll lowers interaction cost but hides the end and hurts finding a specific item. Load more, after a few automatic pages, is the compromise.
6. **Accessibility has a pattern.** WAI-ARIA `role="feed"` with `article` children, `aria-busy` while loading, `aria-setsize="-1"` when the total is unknown, and Page Down/Page Up between articles.
7. **The voice is a log line, not a press release.** The useful narrated updates (deploy feeds, GitHub events, Linear updates) follow subject, verb, object, result, link, in one or two lines.

## 2. How work tools do activity feeds

| Tool | Pattern worth taking | Source |
|---|---|---|
| **GitHub dashboard feed** | Event cards (starred, released, opened PR), chronological. In 2023 it added filters by event type. The ranked "For you" merge was rejected loudly by users, who wanted chronological order and their own repos. In 2024 the dashboard moved updates to a separate Feed tab. | [GitHub changelog, Updates to your feed (2023)](https://github.blog/changelog/2023-09-06-updates-to-your-github-feed/); [Feed sorting update (2022)](https://github.blog/changelog/2022-07-28-github-feed-public-beta-sorting-update); [The Register on the backlash](https://www.theregister.com/2023/09/13/github_alienates_customers_by_force/); [New dashboard default (2024)](https://github.blog/changelog/2024-10-02-new-dashboard-experience-now-the-default/) |
| **GitHub notifications** | Grouped by thread. *Done* and *Saved* are separate from read. Filters by reason (participating, mention, review requested, state change), plus custom filters. | [GitHub Docs: managing notifications from your inbox](https://docs.github.com/en/enterprise-server@3.8/account-and-profile/managing-subscriptions-and-notifications-on-github/viewing-and-triaging-notifications/managing-notifications-from-your-inbox) |
| **Linear inbox** | Snooze brings an item back unread, and new activity cancels the snooze. Keyboard: U for read/unread, H for snooze. Up to 2,000 open notifications, then auto-archive. A Priority vs Other split (2026). | [Linear Docs: Inbox](https://linear.app/docs/inbox); [Snooze](https://linear.app/docs/snooze); [Priority inbox changelog](https://linear.app/changelog/2026-09-03-priority-inbox) |
| **Slack** | Activity is the #2 most-used surface. Slack plans to fold Threads, DMs and Recap into Activity as one hub. Recap gives AI summaries of unread channels after time away, on paid plans, with the user picking channels. The unread pill gains a Summarise button at 10+ unread. | [slack.design: evolving the way millions catch up](https://slack.design/articles/no-small-task-evolving-the-way-millions-catch-up-in-slack/); [Slack Help: AI features](https://slack.com/help/articles/25076892548883) |
| **Vercel / Netlify deploy lists** | One row per deploy: status word, branch, commit message, author and relative time. Filter by status (`vercel list --status READY,BUILDING`). A separate `vercel activity` stream of events, filterable by type and date. This is the right model for "what my agents shipped". | [Vercel CLI: list](https://vercel.com/docs/cli/list); [Vercel CLI: activity](https://vercel.com/docs/cli/activity) |
| **Notion** | A per-page Updates list (edits and comments). The inbox collapses notifications **by page**, which is a rollup by object. | [Notion Help: Inbox and page notifications](https://www.notion.com/help/updates-and-notifications) |
| **Datadog events** | Events carry an aggregation key. Aggregated parents are flagged (`is_aggregate`) and can be excluded or expanded. In 2025 monitor events moved the key to per-monitor-and-group, so a key that is too coarse hides detail. | [Datadog Events API v1](https://docs.datadoghq.com/api/v1/events); [Event Management](https://www.datadoghq.com/product/event-management/) |
| **Stream (getstream.io), the feed infrastructure many apps use** | Aggregation by a format key, e.g. `{{verb}}_{{date}}`. Store a group key, a count, a sample of the first actors and the newest timestamp. Seen and read are tracked separately. Changing the grouping later means backfilling. | [Stream docs: Aggregation](https://getstream.io/activity-feeds/docs/react/aggregation/); [Stream blog: scalable activity feed architecture](https://getstream.io/blog/scalable-activity-feed-architecture/) |
| **Twitter/X and Mastodon timelines** | X: a "See new posts" pill appears when you scroll up, instead of inserting posts above you. Mastodon's web UI streams in real time, and clients that insert automatically get "the timeline jumps" bug reports. Users ask for scroll position to be kept. | [TechCrunch 2022 on the timeline update](https://techcrunch.com/2022/03/10/twitters-latest-update-makes-it-easier-to-escape-its-recommendation-algorithm/); [Fedilab issue: timeline jumps](https://codeberg.org/tom79/Fedilab/issues/737); [Tusky issue 2423](https://github.com/tuskyapp/Tusky/issues/2423) |

## 3. Feed problems and the answers we take

| Problem | Answer | Source / precedent |
|---|---|---|
| **Spam and repetition** | (1) Coalesce one session's lifecycle into one item that updates in place: started, working, finished becomes "finished in 42 min". (2) Roll up the same verb per App and project in a window: "finished 4 sessions in web-app". (3) A digest at the away boundary. (4) Working, idle and progress pings are never their own items. | Stream aggregation; Notion collapse by page; the SRE book's "favor a dashboard for subcritical problems" (`../calm-attention/research.md` §3) |
| **Ranked vs chronological** | Chronological. Exceptions: a **pinned** zone for open L2/L3 needs, and the **away digest** at the boundary. No hidden ranking. | GitHub backlash (above) |
| **Read/unread and "since you were away"** | Opening Home marks items *seen*, and acting on one marks it *read*. Unread items get a neutral 6 px dot, never accent. The away boundary is the existing `summary_seen_at_ms` that MorningSummary already keeps, defaulting to 12 h. | Stream's seen vs read split; AI Juggler `overnight.rs` |
| **Pinning urgent items** | Only open L2/L3 needs pin, and they unpin by themselves when resolved. Users can't pin ordinary items, so the zone keeps its meaning. More than 3 pinned collapse into "+n more need you". | Linear Priority inbox; the calm flood rule (`../calm-attention/proposal.md` (a)) |
| **Filtering** | Type chips (All, Needs you, Finished, Switches & limits, Pools & tasks, This Mac), plus one facet picker for Project, App, Pool or Account, suggested first then All (the pickers ruling). Filters are remembered per viewer. | GitHub reason filters; the "Pickers: suggested, then all" ruling |
| **Infinite scroll with paging** | Keyset pages of 30, by `(at_ms, id)`. They load automatically for 3 pages, then show a Load more button. The cached last page renders at once, before the fetch, as a skeleton. `aria-busy` is true while loading. | NN/g on infinite scrolling; Baymard via Smashing; the "Instant and paged" ruling |
| **Timestamps** | Relative under 1 h ("12 min"), then the clock time today ("9:41 PM"), "Yesterday 4:10 PM", then the date. The exact time shows on hover and to screen readers. Day separators ("Today", "Yesterday") replace repeated dates. | [UX Movement: absolute vs relative timestamps](https://uxmovement.com/content/absolute-vs-relative-timestamps-when-to-use-which/); [Close: the ultimate guide to timestamps](https://making.close.com/posts/ultimate-guide-to-timestamps/) |
| **Motion on new items** | No insertion above the reader. At the top, a new item fades in once for 600 ms and nothing loops. When scrolled down, a "3 new updates" pill appears; clicking it scrolls to the top. Reduced motion turns the fade off. | X's pill; the calm budget rule 6 |

## 4. The "agent tweeting" voice

The maintainer wants it to feel like an agent narrating the system. The research and our rulings ("Plain words", "Serious choices explained", "Accuracy means what it claims") point to these rules:

1. **Subject first, then what changed, then the result.** "Claude Code · Work Max finished *Fix the flaky login test* in 42 min. 3 files changed, tests pass." Not "Great news! …".
2. **One fact per sentence, at most two sentences.** The card shows them; more sits behind "Show more".
3. **The author is whoever acted:**
   - the **App** (with its profile) for session events;
   - the **agent's name** for pool messages;
   - the **Account** for limits;
   - **AI Juggler** only for what it did itself: restarts, switches it ran, cleanup, updates.

   That matches Twitter's author-first card and keeps "who did it" accurate.
4. **Numbers are measured or say so.** "about 40k tokens" only when measured. An inferred state keeps "probably".
5. **Every item links to its thing.** A click opens the session, task, thread, account or switch. ↗ goes to the app. No item is a dead end ("Every object has a door").
6. **No emoji, no exclamation marks, no first person for the narrator.** "AI Juggler restarted it", not "I restarted it". This is calmer, and it never pretends to be a model when it isn't one.
7. **Quoting the agent's own words beats paraphrasing them.** If the tool marked a reply as its final answer (`TextBlock.final_answer` in `conversation_feed.rs:202`), the card can quote its first 1–2 lines. These are the agent's own words, behind the existing "show last messages" setting.

## 5. Two ways to write the updates

### (a) Deterministic templates from facts we already have (recommended base)

- **How:** each event type (see `events.md`) has a template with slots: App, profile, title, project, duration, counts, window, reset time and target. Rollups fill a plural template.
- **Cost:** none. Templates are local and instant, work offline, and are testable with fixtures.
- **Privacy:** nothing leaves the Mac. They use only facts AI Juggler already shows elsewhere.
- **Limits:** they can say *that* a session finished and *what it touched* (files changed via `tool_summary::edited_files`, the step list, tokens). They can't say *what it accomplished* in words.

### (a+) Quote the agent's own final answer (recommended, no model call)

- **How:** for a finished turn or session, read the transcript's last `final_answer` block (the bounded `last_messages` read already exists, redacted). Show its first sentence or two, clipped, with "Show more".
- **Cost:** a local file read. It is already done on demand for the conversation view, and the excerpt goes into the local cache (local cache ruling).
- **Privacy:** it stays on the Mac and is gated by the existing **Show last messages** setting (`show_last_messages`, `conversations/last_messages.rs:28`). When that is off, the card shows only template facts.
- **Why:** most coding agents end with a summary of what they did, so this answers "what did it do" in the agent's own words, accurately, for free.

### (b) Optional summaries written by the user's own tool (opt-in, later)

- **How:** for a finished session, run the user's own installed CLI (for example `claude -p` in headless mode, or the equivalent in Codex) on that same account. Input is the redacted final answer plus the step list. The output is two lines, cached locally.
  - [Claude Code headless docs](https://docs.claude.com/en/docs/claude-code/headless)
  - Slack Recap is the product precedent: an AI summary after time away, user-scoped.
- **Cost:** a real model call on the user's plan, a few thousand input tokens per summary. It counts against the same limits the user is trying to stretch. It needs a daily cap and a visible count in Usage.
- **Privacy:** session content goes to that account's own provider, under the user's own account. It needs its **own setting, off by default**, chosen per Account (which tool and account pays), named on the Privacy page, with nothing sent anywhere by default.
- **Terms reading** (enhance, never replace; and a check before shipping):
  - **The precedent:** D-39 already runs Claude Code's own `claude -p /usage` (no model call) on the user's profiles, with their ruling.
  - **The restriction:** in February–April 2026 Anthropic made explicit, then enforced, that Free/Pro/Max subscription OAuth tokens may be used only in Claude Code and Claude.ai, not in third-party tools or harnesses. The guidance for programmatic access is API keys. [The Register, 2026-02-20](https://www.theregister.com/2026/02/20/anthropic_clarifies_ban_third_party_claude_access); [WinBuzzer](https://winbuzzer.com/2026/02/19/anthropic-bans-claude-subscription-oauth-in-third-party-apps-xcxwbn/) (secondary).
  - **Why (b) is different:** it would **never touch tokens**. It would run the user's own official Claude Code binary as the user, which is closer to "enhance the user's running tool".
  - **The caveat:** automated, unattended prompting on a consumer plan is exactly what that policy watches. So (b) needs a written terms reading per provider (Anthropic, OpenAI and the rest) and a maintainer ruling (a D-number) before it ships. With an API key the user supplies, it is clearly allowed but costs money.
- **Value:** small over (a+) when the agent already wrote a summary. It's useful mainly for long sessions with many turns, or tools that don't mark a final answer.

**Recommendation:**
1. Ship **(a) + (a+)**.
2. Measure in the persona walks how often the final-answer excerpt is missing or useless.
3. Build **(b)** only if that rate is high, as an opt-in layer, after the terms reading.

## 6. What this means for AI Juggler

- **The raw material mostly exists.** Durable journals:
  - `notices` (30 days);
  - `instance_transitions` and `instance_lifecycle` (kept forever, unless the user sets retention);
  - `switch_edges`;
  - `task_events`, `agent_messages` and `agent_events` (append-only);
  - `quota_samples` and `usage_events`.

  Plus the conversation index (in memory) and final-answer blocks (read on demand). The details are in `events.md`.
- **The gaps that matter:**
  - other apps' conversation state changes aren't persisted;
  - switch edges have no completion time;
  - alert-phase crossings aren't recorded;
  - nothing records commits or PRs;
  - nothing records a turn ending with its reply;
  - tasks close without a summary;
  - there's no daily usage digest.

  Each is small. `proposal.md` puts them in phases.
- **The calm rules carry over unchanged.** Each need is loud once per area, in the pinned zone on Home and in Needs you in the rail. The feed's own copy of that event is a quiet history line.
