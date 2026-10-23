# Evidence manifest — AJ-E1510

Task: `docs/tasks/E15-spec-gaps.md` AJ-E1510, System notifications (P1).
Status: **DONE-EVIDENCED** (IN-REVIEW until the release PR). One live
check is the maintainer's: allowing AI Juggler's notifications in macOS
and clicking a real one (tests never grant the permission).

Gallery: `docs/qa/evidence/_gallery/84-notifications/` (Settings ›
Notifications with every kind off, then two chosen; both themes).

## What exists now

1. **Settings › Notifications › Notify me outside AI Juggler**: five
   switches, all off until chosen — a session waits for you, looks stuck,
   hits its usage limit, finishes; This Mac is getting tight. Each says
   what it means in one line; turning one on or off says what changed.
   Stored in app settings (`notify.*`), no migration.
2. **Where they come from** (`features/notifications/`, ADR-0019 §3): the
   state changes are reported where they're made — the heuristic tick
   (waiting, stuck), the lifecycle forwarder (finished, usage limit), the
   recovery drain (working again), This Mac's 5 s tick (its room). The
   pure engine (`engine.rs`) decides what is news: each entry into a state
   is told once; a session flickering back into the same state within 5
   minutes of being told isn't told again (This Mac: 30 minutes, as its
   in-app alerts).
3. **What the user's rules do to them**: a kind turned off, an Account
   left out of the digest, or AI Juggler in front (its main window has
   focus) drops it — it's not told late. Quiet hours (the digest's, now
   "Quiet hours" for both) hold it, and it comes when they end if the
   session is still in that state. The backend keeps no time zone: the
   webview's local hour, already sent with every alerts tick, sets the
   offset; until the first tick an unknown hour counts as quiet, as it
   does for the digest.
4. **What it says**: the session's name as its row shows it (its
   conversation's title, else its folder) as the title; the state, the
   folder when the title isn't it, and the Account below
   ("Waiting for your reply · Work Max"). This Mac: "This Mac: getting
   tight" and its first reason.
5. **A click** (Req.3): the Tauri notification plugin (2.6 is the first
   to report a click on macOS) hands the notification's id back; AI
   Juggler comes forward (show, unminimize, focus) and the UI opens that
   session, or the This Mac page. An id from before a restart brings the
   window forward and opens nothing else.
6. **Nothing more than asked**: the webview has no notification
   permission (no capability grants it); only the backend shows them.
7. **What it costs**: on macOS the plugin waits for each shown
   notification's click on a thread of its own until it is clicked or
   dismissed. Each state is told once, so that's a few threads at most;
   the engine forgets a target once nothing would hold it back.
8. **The same look as its siblings**: the switches are Settings ›
   Sessions' own (now `SettingSwitch.tsx`, shared); the card moved out of
   `SettingsSections.tsx` into `NotificationSettings.tsx` (that file is
   back under its budget and off the allowlist); App's go-to hotkey and
   the click share one hook (`lib/outside.ts`), and App's cap went down
   from 1232 to 1224.

## Proof

| Requirement | Evidence | Result |
|---|---|---|
| Req.1 five kinds, off by default, a choice per kind | `tests/system_notifications.rs` `off_by_default_nothing_is_shown`, `a_waiting_session_is_told_once_with_its_name_folder_and_account` (only the kind turned on is told); Playwright `notifications` (five switches unchecked; turning two on saves exactly those) | PASS |
| Req.1 each kind's words | `each_kind_says_what_happened`, `this_mac_getting_tight_says_why_and_opens_this_mac` | PASS |
| Req.2 quiet hours hold them, released after if the state still holds | `quiet_hours_hold_it_until_they_end`, `an_unknown_hour_holds_it_as_the_digest_does`; engine unit `quiet_hours_hold_it_and_it_comes_after_if_it_still_holds` | PASS |
| Req.2 the digest's Accounts left out apply | `an_account_left_out_of_the_digest_stays_quiet` | PASS |
| Req.2 at most once per session per state | engine units `each_state_is_told_once`, `a_session_flickering_back_to_waiting_asks_once_in_five_minutes`, `this_mac_is_told_again_only_after_half_an_hour`; tight then pressure is one state | PASS |
| Not while AI Juggler is in front | `nothing_while_ai_juggler_is_in_front` | PASS |
| Req.3 a click brings the app forward on that session | `a_click_brings_ai_juggler_forward_on_that_session` (a mock Tauri app: the click's id → `app://open` with the session; an unknown id opens nothing); vitest `system-notifications` (the hook hands the request on, and stops at unmount); Playwright (the open request selects the session, or opens This Mac) | PASS |
| Settings › Notifications, both themes, axe | vitest `system-notifications` (five kinds off; each saved on its own and said; the quiet hours and Accounts name notifications); Playwright `notifications` (axe 0 critical/serious) | PASS |
| Clicking a real macOS notification | needs the permission granted on a real Mac | MAINTAINER |

## Walked

On the preview at 800×600: Settings › Notifications shows Notify me
outside AI Juggler with five unticked switches above Quiet hours and In
the digest and notifications. Ticking "A session waits for you" says
"You'll get a notification when a session waits for you." at the card's
foot. A stand-in click for `sess-live-1` left Settings for that session's
page; one for This Mac opened the This Mac page. The digest's Account
boxes now use the accent colour, as the switches above them do.
