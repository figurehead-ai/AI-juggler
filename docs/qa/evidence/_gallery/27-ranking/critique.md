<!-- captures: 62706e625f99 -->
# 27-ranking: how accounts are ranked for new work: the Home queue, the bell's notifications, and the New session dialog's Ranked for you list with its Why this order? explanation

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In notification the bell popover is a clear raised panel in both themes, but every one of its five items still carries an equally solid violet button, so nothing leads, and the same action is labelled two ways ("Continue elsewhere" and "Continue on another account…"); at 1280x800 the last item's "Continue on another account…" button still runs flush into the popover footer's top border, its bottom edge sliced. In ranked and why the New session dialog groups well (folders as cards, a ranked list with a Suggested chip, then Which account? and a solid violet "Launch on Work Max"); in dark the Suggested chips, the Which account? ring and the "Why this order?" focus ring now carry the lighter violet. The layout problems are unchanged: at 1280x800 the Which account? card is cut at the bottom of the scroll area in ranked and pushed out of sight in why, in why-1440x900 its second line is sliced, the suggested account reads "82% headroom" in the ranked list but "58% left, from its latest session" in Which account?, and "82% left this week ·" ends on a stray separator. The queue captures are now re-rendered on the 0.3.2 Home: a violet "Work Max has room again" banner with a solid "Open feat/read-model" button, a Needs you card that leads with Personal and its solid "Open its page", Runway by account with bars and reset times, and Pick up where you left off. The banner button and "Open its page" are two solid primaries one above the other, and the banner button carries a focus ring in every queue capture, so it reads as already pressed.

## Weak

- notification-1280x800-dark.png, notification-1440x900-light.png: five equal solid violet buttons in one popover; two different labels for the same continue action.
- notification-1280x800-dark.png, notification-1280x800-light.png: the last notification's "Continue on another account…" button sits flush on the popover footer's border, its bottom edge cut.
- ranked-1280x800-dark.png, ranked-1280x800-light.png: the Which account? card is cut at the bottom of the dialog's scroll area.
- why-1440x900-dark.png, why-1440x900-light.png: the Which account? card is sliced mid-line; it shows 58% left while the ranked list above says 82% headroom.
- ranked-1440x900-light.png (and dark): "82% left this week ·" ends on an orphan separator.
- queue-*.png: two solid primaries stacked (the banner's "Open feat/read-model" and Personal's "Open its page"), and the banner button shows a focus ring on arrival.
- queue-*.png: Needs you labels the same action "Continue on another account…" on Personal and "Continue elsewhere" on the Personal row below it.
- queue-1280x800-*.png: Pick up where you left off titles are cut ("Keep the old shortcuts wor…", "ai-juggler · feat/read-model · …"); Fake Desk's runway shows a bare "—" on the right.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: notification-1280x800 dark (the 0.5.1 shoot and after 7e858e860), queue-1280x800 light, and the eight ranked/why captures. Home is D-118's two columns in every capture: Needs you now as one card per need (the first card's primary the one filled button), the feed with its chips and Showing:, then Your apps and Runway by account on the right.

- **notification:** the bell's centre over Home: Needs you · 6, each with its icon, title, where, "waiting 2 h", and its actions. Since 7e858e860 only the first item's Open is filled; the others are tinted (the 0.3.2 weak point, five equal violet buttons, is gone); "Continue elsewhere" and "Switch account…" both still appear (one is a limit's hand-over, the other a quota warning's). The footer "Today: Claude Max 20x resets at 3:48 AM · Notification settings" sits clear of the last item.
- **queue:** a session's own page with its due reminder: "This session's account has room again: it can carry on / The reminder you set. Nothing restarted by itself." and a close; nothing resumes by itself (D-87).
- **Removed** (F62): ranked-* and why-*, eight captures of New session's old "Ranked for you" list and "Which account?" card behind a 0.3 Home, gone from the product since 9735d44c8 and written by no test since; they misled. The ranked list lives on in the switch dialog (SwitchTargetRows), which its own family shows.
- Re-captured: notification and queue, both sizes and themes (local, after F53).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | What needs you and what's due, in the bell and on the session's page. |
| 2 | Prominence | 4 | Needs you first in the centre, Earlier below; the reminder at the session's head. |
| 3 | Default | 4 | The first item's Open is the default (↵). |
| 4 | Mode | 4 | Calm: the centre on the overlay tone, the reminder a notice by tone. |
| 5 | Mock-up | 4 | Matches the notification centre's own family (96). |
| 6 | Areas | 4 | Items by rule and tone; the footer separate. |
| 7 | Colour | 4 | Amber and red only as item marks. |
| 8 | Type | 4 | Titles 13 px semibold, where and when 12 px muted. |
| 9 | Actions | 4 | One filled button (since 7e858e860); the rest tinted. |
| 10 | Serious | n/a | Nothing destructive; Snooze and ⋯ are reversible. |
| 11 | Doors | 4 | Each item opens its thing; Watches › and Notification settings link out. |
| 12 | Words | 4 | "Continue elsewhere" and "Switch account…" name two different hand-overs, a little alike. |
| 13 | States | 4 | Read state and "Mark all read"; the reminder says nothing restarted. |
| 14 | Consistency | 4 | The notice and row patterns. |
| 15 | Accessibility | 4 | Not re-walked by hand this round; keyboard names and focus as the a11y gate checks them. |
| 16 | Hand walk | 4 | The PL1 walk (AJ-E2323) drove Home and the rail in Dev; not re-walked here. |
| 17 | System prompts | 5 | None (notifications to the system are opt-in, 84-notifications). |
| 18 | Usability | 4 | What to look at next, and that nothing moved by itself. |

Lowest: 4.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Needs you's cards are tone fills with their level's mark (a stopped one in the broken tone), the most urgent first (F53, F66, F80). Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
