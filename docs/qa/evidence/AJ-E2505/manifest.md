# Evidence manifest — AJ-E2505

Task: `docs/tasks/E25-notifications-and-user-menu.md` AJ-E2505, §2 Toast.
Built 2026-10-27. Checked requirement by requirement on 2026-11-06
(release/0.4.1); its one gap was closed the same day. Status:
DONE-EVIDENCED (2026-11-06, release/0.4.1).

| Req | Result | Evidence |
|---|---|---|
| R1 | PASS | `ui/src/components/notifications.test.tsx` "stays six seconds, and hovering pauses its time": `TOAST_SECONDS` is 6 and the toast goes at 6 s. The held Stop keeps its 5 s: `components/feedback.test.tsx` "sends the stop only once the 5 s window has passed" (nothing sent at 5 s − 1 ms, the stop sent and the toast gone at 5 s). |
| R2 | PASS | **Tested:** the bar is there (`toast-time` on each of three toasts: "sits at the bottom right, three at most, with the time left as a bar"); hovering stops the time and leaving resumes it ("stays six seconds, and hovering pauses its time": still there after 20 s of hover, gone 3 s after leaving). **Closed (2026-11-06):** `ui/src/components/notification-views.test.tsx` "the user's focus on its Undo holds the time; leaving it lets it run" (focus moved to Undo: still there after 60 s; focus moved away: gone) and "focus put on Undo for them (nothing else had it) lets the time run" (`focusWhenFree` puts focus on Undo, and the toast still goes). |
| R3 | PASS | `notifications.test.tsx` "an error stays until it's closed, and its Retry runs once, on the click": after 120 s the error toast is still there with no time bar, Retry hasn't run; one click runs it once and closes the toast. "a warning that something didn't work stays until it's closed too". |
| R4 | PASS | Three at most: "sits at the bottom right, three at most…" shows four, keeps the newest three. One polite live region: `feedback.test.tsx` "announces through the polite region that is always in the page" (`role=status`, `aria-live=polite`, the announcement text, the countdown never in the region). |

Run on 2026-11-06: `pnpm exec vitest run src/components/notifications.test.tsx
src/components/feedback.test.tsx` (passed). The gallery (toast-undo,
toast-error) was not run here.

Gaps closed on 2026-11-06: `pnpm exec vitest run src/components/notification-views.test.tsx src/components/user-menu-facts.test.tsx src/screens/system-notifications.test.tsx` (26 passed); the whole suite passed too (281 files, 2135 tests).
