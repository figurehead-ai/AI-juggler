# When an open session hits its limit: the signals AI Juggler gets

Research date: 2026-10-29
Author: agent (read-only on the repo and the Mac: the maintainer's Claude Code transcripts were read once, read-only, for the **structure** of limit lines only — key paths, value types and the enum-like values below; no message text, path or id was kept or copied; no account was pushed to a limit, nothing was signed in, AI Juggler's own data folder was not opened)
Evidence rules: `docs/research/EVIDENCE-STANDARD.md`. Labels: **VP** = Verified-primary, **I** = Inferred, **U** = could not verify.
Feeds: 0.3's open-session limit (`crates/juggler-app/src/features/sessions/limits.rs`, `HarnessAdapter::limit_hit`, `Capabilities::LIMIT_HITS`).

## The problem

Until 0.3 a session was "hit its limit" (`SessionStatus::QuotaBlocked`)
only on `LifecycleEvent::Exited` with a non-zero exit and a rate-limit
signature in its output (AJ-E405, `heuristics::classify_exit`). An
interactive Claude Code session shows its limit and **stays open**, so it
was never marked: Needs you's limit row, the limit dialog, Park and the
session page's auto-opened switch list were dead paths for it. The
account-level banner did fire (from the hit line below, through the usage
pipeline), with Hand over… / Queue / Snooze.

## The signals, per kind of session

| Session | Signal it already gave | Used now | Label |
|---|---|---|---|
| AI Juggler's terminal session, Claude Code (hooks on, AJ-E2306) | 1. The **transcript line** Claude Code writes when a turn stops at a limit (below). 2. The **`StopFailure` hook**, `error: "rate_limit"`, reported over the app's socket (`Observed.error`) but never turned into a status. 3. PTY output (the limit message on screen). 4. The status line's `rate_limits` (account-level %, only on render). | 1, read through the adapter; 2 as the trigger to read at once | 1: VP (structure observed); 2: VP (docs) |
| AI Juggler's terminal session, Claude Code without hooks | 1, 3 | 1, on the heuristic tick and when output moves | VP |
| Driven Claude session (stream-json, Beta) | `rate_limit_event` (`rate_limit_info.status: "rejected"` → `LimitsSnapshot.reached`), the turn's `result` with `is_error` (codec → `RuntimeEvent::Error { category: RateLimit }`), and the transcript line (it keeps a transcript) | `Error { RateLimit, will_retry: false }`, neutral across tools | I (codec mapping of a real limit not seen live) |
| Driven Codex session (app-server) | `error` with `codexErrorInfo: usageLimitExceeded` → `Error { RateLimit }`; `account/rateLimits/updated` | the same `Error` | I (fixture only) |
| Codex in a terminal | its rollout; no limit record read | not supported (`limit_hit` default: none) | U |
| Another app's session (the Claude app's Code sessions, a terminal AI Juggler didn't start) | the same transcript line (the hit lines observed come from these: entrypoints `claude-desktop`, `sdk-ts`) | the account banner only; they have no AI Juggler session row to mark | VP (line) |

### 1. The transcript line (VP, structure observed 2026-10-29)

Observed read-only in the maintainer's transcripts: 711 lines with
`"error":"rate_limit"`, every one `type: "assistant"`,
`isApiErrorMessage: true`, `apiErrorStatus: 429`,
`message.model: "<synthetic>"`, with
`quotaLimits {status: "rejected", rateLimitType: "five_hour" | "seven_day",
resetsAt: <epoch s>, isUsingOverage, overageStatus, …}` and (in all but
28) the same under `apiErrorParams.rate_limit_info` plus
`apiError: "usage_limit_reached"`. 436 were sub-agents' (`isSidechain:
true`, in `subagents/` files), 275 the sessions' own. No line with a
`quotaLimits.status` other than `rejected` was found (a warning is not
written as an error line). The lines after a hit are the user's next
prompt and bookkeeping (`attachment`, `queue-operation`, `last-prompt`),
then the model's reply.

`claude_limits::limit_hit_sample` already read this for the account; the
new `limit_hit_in_tail` reads it for the session: the newest of the
session's own assistant lines decides — a hit (`rejected`, 5-hour or
week) counts until the model replies (an assistant line with no `error`
and a real model). Warnings, a model's own week (`seven_day_opus`, …) and
other API errors are no hit (the tests from
`a_per_model_week_a_warning_or_another_error_is_no_hit` hold).

### 2. `StopFailure` (VP, https://code.claude.com/docs/en/hooks, accessed 2026-10-29)

"Runs instead of Stop when the turn ends due to an API error." Input:
`error` (one of `rate_limit`, `overloaded`, `authentication_failed`,
`oauth_org_not_allowed`, `account_on_hold`, `billing_error`,
`invalid_request`, `model_not_found`, `server_error`,
`max_output_tokens`, `cloud_credential_error`, `unknown`),
`error_details`, `last_assistant_message`. `rate_limit` covers a plain 429
too (the docs' example is "API Error: Rate limit reached"), so it is a
**trigger to read the transcript**, never the mark by itself.

### 3. Claude Code waits at the limit and may continue by itself (VP, same page)

Since v2.1.234 the `Notification` hook has `quota_auto_resume_fired`
(Claude Code "continues your task after a claude.ai usage limit paused
it: at the reset, or sooner…"), `quota_auto_resume_stale` (after a long
sleep it waits for Enter) and `quota_auto_resume_disabled`. So an open
session at its limit is the normal case, and it may carry on with no
prompt from the user: the model's reply clears the mark (and so does the
reset time passing).

## What could not be verified

- **The CLI's own line (U).** Every observed hit came from the
  `claude-desktop` or `sdk-ts` entrypoint; the corpus has `cli` sessions
  but none that hit a limit. Inferred to be the same (one code path writes
  the synthetic error message), not seen.
- **Ordering of the hook and the line (U).** Whether the transcript line
  is written before `StopFailure` runs. The watch reads at the hook, again
  when the terminal's output moves (at most once a second) and on the 15 s
  tick, so either order marks within about a second.
- **Driven runtimes at a real limit (I).** The stream-json `result` and
  Codex's `usageLimitExceeded` mapping are fixture-tested only.
- **The `quota_auto_resume_*` notifications** are documented but not yet
  taken by AI Juggler's hooks (the hook keeps the type word; a follow-up
  could show "continues at 3 PM by itself").
- **Live end to end.** No account was taken to its limit (the
  maintainer's rule); the path is proven with the fake tool's open-limit
  fixture (`fake-harness run --limit-hit-ms`) and recorded-shape lines.
