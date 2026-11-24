# Evidence manifest — AJ-E2207

Task: `docs/tasks/E22-visual-usage.md` AJ-E2207 — a session's usage.
Audited 2026-11-23 by zcode-primary against HEAD `92ada8c32`; the
task's own wait condition ("the session page's Usage tab mounts it")
is met and was landed with the 0.1.5 usage charts.

| Claim | Requirement | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| `ui/src/pages/session-usage/SessionUsage.tsx` with `profile` and `conversation` | Req.1 | The component exists with exactly those props (plus `profileLabel`); `UsagePanel` mounts it on the session page's Usage tab whenever the usage history has turns for the conversation (`known`), with its account's Runway beside it — and falls back to the tool's own counts (`SessionUsageTab`) when there are none | PASS | 2026-11-23 | zcode-primary |
| Turn by turn, by kind and model, and its share of its windows | Goal | `session-usage-visual` + `session-usage-turns` testids render the charts; the share-of-limits piece is `share-of-limits.test.tsx` (2/2) plus the panel's `SubagentsShareLine` and the Runway note ("this session's share is above") | PASS | 2026-11-23 | zcode-primary |
| One read of at most 2,000 turns | perf budget | The panel's read is one cached key (`session-usage:<profile>:<conversation>`, `useCached`, 30 s) over `usageSession` — the same single read `SessionUsage` makes (the panel comments pin this); the turn cap lives in the backend read | PASS | 2026-11-23 | zcode-primary |
| A gallery state once mounted | quality gate / visual | `session-pages-gallery.spec.ts` "a session page's Usage" shoots the mounted tab and asserts the three testids visible, both themes; the empty-history fallback has its own state ("with no counts of its own") | PASS | 2026-11-23 | zcode-primary |

## Test counts at audit

- `pnpm --dir ui test src/pages/session-usage/`: 2/2.
- The mounting panel and its gallery run in the full suites (vitest
  3778/3778; the visual gate 344/344 at AJ-E2023's audit).
