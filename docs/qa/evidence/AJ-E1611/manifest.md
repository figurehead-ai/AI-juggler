# Evidence manifest — AJ-E1611

Task: `docs/tasks/E16-more-tools.md` AJ-E1611 (tool update checks).
Status: **DONE-EVIDENCED** on fixtures and a recording transport; the
first live request is on the maintainer's checklist
(`docs/internal/MAINTAINER-STEPS.md` step 16, "New tool versions").
2026-10-24, reviewer claude-opus.

## How it works

- Each adapter's spec names where its versions are published
  (`ToolSpec::release_feed`: npm's `latest` for `@anthropic-ai/claude-code`,
  `@openai/codex`, `opencode-ai`, `@moonshot-ai/kimi-code`) and the command
  the user runs to update it (`ToolSpec::update_command`: `claude update`,
  `npm install -g @openai/codex@latest`, `opencode upgrade`,
  `kimi upgrade`). No tool id appears outside the adapters.
- `features::tool_updates`: ten minutes after start, then hourly, for each
  tool the user has an account on, at most one request a day to
  `https://registry.npmjs.org/<package>/latest` (no credentials, no body),
  through `juggler-account`'s transport. What it found (the version and
  when) is kept in AI Juggler's settings. Off with one tick on any tool's
  page (`tool_update_check`).
- The tool's page (`ToolUpdateLine`): "<Tool> <version> is out; not
  tested with AI Juggler yet. To update, run `<command>` yourself." Calm:
  a line, never a popup, and no button runs anything.

## Requirements

| Claim / Req ID | Command / fixture / live step (exact) | Threshold | Observed | Verdict | Commit SHA | Date | Reviewer |
|---|---|---|---|---|---|---|---|
| Req.1 each tool's version is recorded and its release channel checked once a day: one request per tool a day, only tools with an account here, none with the check off; a failed answer keeps what was known | `cargo test -p juggler-app --test tool_updates` (`one_request_per_tool_a_day_only_for_tools_in_use_and_none_when_off`) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.1 the result on the tool's view: the newer version, whether AI Juggler was tested with it, the update command; nothing when current; the check can be turned off there | vitest `tool-update-line.test.tsx` (4); `ui/tests/visual/more-tools-gallery.spec.ts` (OpenCode's page, run locally, capture checked by eye) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Fixtures per tool: every registered tool names its npm package and update command; an answer per tool reads as its version; versions compare by their numbers (a pre-release before its release); `2.1.x` matches | `tool_updates::every_registered_tool_lists_its_versions_on_npm_and_says_how_to_update`; `juggler-app` lib `tool_updates::tests` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Req.2 never update a tool (guardrail): the module starts no process; the command is text only | `juggler-app` lib `tool_updates::tests::no_update_is_ever_run`; vitest "says a newer version is out…" (no button) | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Privacy page: the new "tool-updates" line, and npm's registry the only new address the app's code may name | `trust::every_trust_claim_names_tests_that_exist`; `trust::the_app_reaches_only_its_own_addresses` | pass | pass | PASS | branch head | 2026-10-24 | claude-opus |
| Performance: one request per tool per day; nothing before the first window (the first pass waits ten minutes) | `tool_updates::one_request_per_tool_a_day_only_for_tools_in_use_and_none_when_off`; `FIRST_PASS_AFTER` | ≤ 1/tool/day | 1/tool/day | PASS | branch head | 2026-10-24 | claude-opus |
| Live: the registry answers the `%2F` form for a scoped package, and turning the check off makes no request | the maintainer's checklist | — | — | UNPROVEN | branch head | 2026-10-24 | claude-opus |

## For the maintainer

- The check is **on by default**, as the app's own update check is. It
  reaches a third party (npm's registry) the app didn't reach before; the
  Privacy page and README say so. Say if it should start off instead.
- OpenCode 2.x isn't published on npm (Homebrew only), so a 2.x install
  hears of 1.x releases only, and none is newer: it shows nothing.

## Redaction checklist

- [x] No credentials, tokens or keys
- [x] Paths generalised (temporary folders only)
- [x] No transcript content
