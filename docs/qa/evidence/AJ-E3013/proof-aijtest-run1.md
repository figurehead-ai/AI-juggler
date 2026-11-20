# AJ-E3013 lab proof on aij-test (research §7.2), run 1 (2026-11-16)

The real Claude app, the sanctioned test profile **aij-test**, and AI Juggler Dev (built from 0557a46c6) on the real HOME (condition 7b). A Sonnet driver used `scripts/qa/floor_drive` by Dev's pid and `scripts/qa/app_drive` (AIJ_APP_PROFILE=aij-test) for the Claude app. Each verdict was checked by Opus; the clean-up was checked by Opus afterwards.

| Step | Verdict | Evidence |
|---|---|---|
| A0: aij-test is an App | PASS | already "Claude app · aij-test" |
| A1: the snapshot before | PASS | settings.json and .claude.json sha256 recorded; no `skills/` |
| A2: the consent preview | PASS | exactly 2 files: `skills/ai-juggler/.claude-plugin/plugin.json`, `skills/ai-juggler/skills/pool-tasks/SKILL.md`; no settings.json line |
| A3: only its own files written | PASS | the 2 files appeared; both sha256 unchanged |
| A4: a Code session with the connector | PASS | `sessions/<pid>.json`: entrypoint `claude-desktop`, status idle; `juggler-agent --mcp --plugin` was that pid's child |
| A5: whoami attested | **FAIL, then PASS** | the first call, seconds after the start, was refused ("the caller wasn't started by a Code-tab session of this Claude app profile"); 40 s later it named the pool, member role, joined as App, level Work together. No permission prompt at any point |
| A6: messages, a task, a Blocked rule | PARTIAL | inbox read and "received" replied (Messages showed it). The session wasn't listed as its own recipient or drawn on the diagram. The task and the Blocked rule were not run: the brief allowed no prompt that makes a task or sends from the Lab member |
| A7: the idle member's line | PASS (wording) | "Its sessions: reads messages on its next turn"; never "can't reply" |
| A9: woken at Full | **FAIL** | message at 23:40:42; the registry stayed idle for 150 s ("Waiting in its inbox"). The Lab CLI member woke in ~3 s |
| A10: exact Remove | PASS | `skills/` gone; settings.json byte-identical (checked by Opus: `ca3d163b…`); .claude.json differed only by Claude's own `cachedFeatureFlags` / `cachedFeatureFlagsAt`; a new session had no AI Juggler tools. The card's words stayed stale |
| A11: refusals | PARTIAL | a changed SKILL.md was named and left alone, but Remove deleted plugin.json (a half-removal); an unrelated `skills/aij-unrelated/` was untouched twice |

**Route notes:**
- The native Open panel and popover items aren't reachable through the Claude app's own pid, so the session ran in the app's remembered `~/lab/web`. Scoping the pool to web swept about 33 old ended web sessions in (their inboxes got copies of two broadcast messages).
- `type` and `key` worked without `front`.

**Left in AI Juggler Dev's store:** the pool "aij46 scratch" (scope web + aj46) and its messages. It is to be deleted by the next run (the delete confirmation was clipped, so it couldn't be reached).

**Fixes routed:**
- The start race: a bounded retry of the first attestation.
- A9's root cause (likely the waker backing off after a refusal at start).
- An all-or-nothing Remove, with an explicit "Remove anyway".
- Connected desktop sessions in the member and recipient lists and on the diagram.
- The card's words following the real state.
- The delete confirmation kept in view.

**Next run:**
- A6 with harmless pool-action prompts (the Lab member makes a task and sends a message).
- A9 again.
- A11 again.
- Delete "aij46 scratch".
