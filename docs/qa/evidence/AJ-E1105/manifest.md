# Evidence manifest — AJ-E1105

Task: `docs/tasks/E11-usage-statistics.md` AJ-E1105 (consent in
onboarding and Settings ▸ Privacy). 2026-10-18, reviewer claude-opus.
Captures: [`_gallery/62-privacy`](../_gallery/62-privacy) and
[`_gallery/63-consent`](../_gallery/63-consent), 1280×800 and 1024×680,
light and dark.

| Claim | Threshold | Observed | Verdict | Date | Reviewer |
|---|---|---|---|---|---|
| The first-run policy step shows the three options with equal weight and nothing pre-selected; leaving without choosing keeps `off` | Req.1 | `UsageStatsChooser` beside the pack-policy question on both first-run paths. `first-run-policy.test.tsx` › AJ-E1105: three options sharing one class list (equal weight), none checked; an answer is saved on the Accept click (`["local"]`); no answer saves nothing, so stats stay off. Capture `63-consent/first-run-*` | PASS | 2026-10-18 | claude-opus |
| A consent-version bump asks again with the current mode pre-selected, sending nothing until answered | Req.1 | `UsageStatsReconsent` at launch when the stored consent version is older than the current one (focus on the current choice; Escape = ask next time). `privacy-card.test.tsx`: shown after a bump with `share` pre-selected, saved on Save; never shown before a first choice. Nothing is sent meanwhile: `cycle::tests::ci_debug_and_unanswered_consent_never_send` (AJ-E1104). Capture `63-consent/reask-*` | PASS | 2026-10-18 | claude-opus |
| Settings ▸ Privacy shows the mode (with the override reason), this week's counters by category, the pending/sent/skipped reports with week, status and reason (each opening its exact JSON read-only, with Copy), the next upload time, "Delete local stats", "Open metrics documentation", and the crash-report setting | Req.2 | `PrivacyCard` over `usage_stats_overview` (reads only, creates nothing — `the_privacy_overview_shows_what_was_counted_and_kept`), `usage_stats_report` (refuses any name but a week under pending/sent/skipped), `usage_stats_delete_local` (keeps the mode), `usage_stats_open_docs` (a fixed address; the interface passes none). The viewer shows the exact report spaced out to read; Copy copies it byte for byte. Crash reports read "never" — they don't exist until AJ-E1107 | PASS | 2026-10-18 | claude-opus |
| States: off, local, share, overridden, not included in this build, an upload error in plain words with the retry time | Req.3 | `privacy-card.test.tsx`: one test per state (DO_NOT_TRACK disables the controls; "This build doesn't include usage statistics."; "Couldn't reach Figurehead. Will retry …"); choosing Off says "Usage stats are off. Local stats deleted."; Delete local stats says "Local stats deleted." Captures `62-privacy/{off,local,share,overridden,not-included,upload-error,report-json}-*` | PASS | 2026-10-18 | claude-opus |
| Renders < 200 ms with 12 weeks of reports | perf budget | `privacy card: 12 reports + 150 rows rendered in 16 ms` (jsdom) | PASS | 2026-10-18 | claude-opus |
| vitest for each state; axe 0 in both themes | quality gate | `privacy-card.test.tsx` (9) and the AJ-E1105 tests in `first-run-policy.test.tsx` (3); `a11y.spec.ts` › "settings — privacy, populated, report open — axe 0" in dark and light, plus the existing Settings axe test, which now includes the card | PASS | 2026-10-18 | claude-opus |

Note for the maintainer: the website's usage-stats spec (§3.2) says the
user has to pick an option to continue. Flow (a)'s budget (AJ-E305: at
most 5 clicks on the primary path, and the pack-policy answer already
counts as one) leaves no room for a second required answer. So the
consent is shown on the same step with equal weight and nothing
pre-selected, but doesn't block Continue; not answering keeps stats off,
as AJ-E1105 Req.1 says. If a required answer is wanted, flow (a)'s
budget has to grow to 6, which is the maintainer's call.
