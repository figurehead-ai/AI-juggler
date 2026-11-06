# Evidence manifest — AJ-E2504

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2504, §2 Banner,
the alert banners, and the offline banner. Built 2026-10-27; the band was
reworked in 0.3.2 (one banner, the rest behind "N more"). Checked
requirement by requirement on 2026-11-06 (release/0.4.1). Two
requirements have gaps, so the status stays DONE-PENDING-EVIDENCE.

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | **The component:** `ui/src/components/notifications.test.tsx` "says who and what, shows its meter, and runs only the action clicked" (title, detail, a `meter` with `aria-valuenow`, only the clicked action runs, the close). **Every band banner is one** (a structural fact, proven by the source): `components/NotificationRegion.tsx` renders the account banner (`quota-banner`, line 239), a queued resume coming due (`reminder-ready`, 266), a parked session's reset (`parked-offer`, 285), this computer's alerts (`machine-alert`, 302), another computer's alerts (`remote-alert-*`, 308), and the update found and ready (`update-notice`, `update-ready`, 322 and 340) through `<Banner>`; `OfflineBanner.tsx` too (line 31). Tests that render them through it: `banner-band.test.tsx` (quota, reminder), `this-mac.test.tsx` (machine alert). |
| R2 | GAP | **Tested:** the words, "Mia is almost out of room", "7% of its 5-hour window left · resets at … · 1 session uses it" (`plain-words-on-screen.test.tsx`; `notifications.test.tsx` "never says an internal id…"); the meter (above); the primary "Continue “shop” in another app…" (Hand over…, renamed by R2's amendment and F16); "Remind me at <time>" (amended); Snooze 1 h hides it at once (`banner-band.test.tsx` "snooze hides it at once too"); × hides it at once and a refused close comes back ("a close hides it in the same frame…", "a close the backend refuses comes back"); at most three, one per account, and a dismissed one stays in the centre (`centre::tests::banners_are_due_quota_needs_at_most_three_and_dismissed_ones_stay_in_the_centre`; since 0.3.2 the band shows one and "2 more notices", `banner-band.test.tsx` "shows one banner…"). **Missing:** no test that on Home the account banner is left out because Home lists it in Needs you. `this-mac.test.tsx` checks that only for this computer's alert; the same `hideMachineAlerts` flag also hides the account banners. |
| R3 | GAP | **Tested:** the banner: `notifications.test.tsx` "offline: the last state stays, it says since when, and Retry is a click" (`role=alert`, the two sentences word for word, Retry runs once on the click, nothing shown without the app's service). **Missing:** no test that the window's refresh reports a failure (`shell/use-app-data.ts` calls `backendFailed()` / `backendAnswered()`), that the shown data stays, or that New session waits and says why (`AppHeader.tsx`: disabled, "Waits until AI Juggler's service answers again"). |

Run on 2026-11-06: `pnpm exec vitest run src/components/notifications.test.tsx
src/components/banner-band.test.tsx src/plain-words-on-screen.test.tsx
src/components/this-mac.test.tsx` (passed); `cargo test -p juggler-app --lib
features::notifications` (48 passed). The gallery (banner-quota,
banner-offline) was not run here.
