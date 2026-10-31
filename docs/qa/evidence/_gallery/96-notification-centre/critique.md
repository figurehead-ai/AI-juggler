<!-- captures: f89c8d098169 -->
# 96-notification-centre: notifications, banners, toasts and your menu

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good.

## Checklist

| Check | Finding |
|---|---|
| Grid | The centre's rows use the row anatomy (mark, words, time, ⋯); your menu's lines are the row menus' grid. |
| Hierarchy | What needs you first, then what happened; banners above the page, toasts at the bottom. |
| Spacing | The row menus' rhythm. |
| Icons | Every line of your menu and of a need's ⋯ has its icon. |
| Shared controls | No system controls. |
| Disabled reasons | A muted centre says until when. |
| Plain words | Banners say what is wrong and what to do; toasts offer Undo and Retry. |
| Type sizes | Body 14 px, times 12 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | Captured at 1280. |

## Good

- The centre, banners and your menu are unchanged apart from the header; your menu holds Settings, This Mac and Processes.

## Weak

- centre-row-menu-1280x800: the open ⋯ menu's bottom edge cuts across the Work Max row's "window left · resets at 3:48 AM · 2" line, so it reads like strikethrough.
- user-menu-account-1280x800: your menu's bottom edge slices through the top of the Runway card's "Not measured yet · Measure its limits" line the same way.
- banner-quota-1280x800: the rail's Fake Desk row shows its hover buttons (grip, pin, open) over its "Working" status, leaving the word half covered.
- banner-offline-1280x800: the header's New session fades while actions are paused, but Needs you's Open its page (solid violet) and the Continue / See what happened buttons look as live as ever.

## Fixed

- Nothing open from the review.

## Captures

| Capture | What is on it | What changed in 0.3 |
|---|---|---|
| header-badge-1280x800 | The header's bell with its one count. | The 0.3 header (AJ-E2803). |
| centre-populated-1280x800 | The centre with needs and what happened. | The 0.3 header. |
| centre-row-menu-1280x800 | A need's ⋯ menu. | The 0.3 header. |
| centre-empty-1280x800, centre-muted-1280x800 | Nothing to show; muted for the quiet hours. | The 0.3 header. |
| toast-undo-1280x800, toast-error-1280x800 | A result with Undo; an error with Retry that stays until closed. | The 0.3 header. |
| banner-quota-1280x800, banner-offline-1280x800 | An account almost out of room; AI Juggler's own service not answering. | The 0.3 header. |
| user-menu-1280x800, user-menu-account-1280x800 | Your menu without and with a Figurehead account: Settings, This Mac, Processes. | Processes moved here (AJ-E2507). |
| tray-menu-1280x800 | The menu-bar item's menu, from the Rust builder's output. | Unchanged. |

## 0.3.2

The darker canvas lifts the Notifications popover, the row menu, the toasts and your menu off the page in centre-populated, centre-row-menu, toast-undo and user-menu, and the grey light canvas does the same for their white surfaces; each need's Open, Continue on another account… and Continue elsewhere is one solid violet, so the row's main action is obvious and the secondary Snooze 1 h and Remind me at 3:48 AM sit visibly lower. In dark the lighter accent shows on those buttons, the header's New session, the avatar, the unread dots, Mark all read and Notification settings, and the toast's Undo (5s) outline and countdown bar; the light captures change only by a pixel in the This Mac line. The top bar stays one row, with "This Mac · Plenty of room" whole; in user-menu-account, where the Fleet tab joins the row, the This Mac line is left out instead, as it was before. banner-quota shows the slim banner with "Remind me at 3:48 AM", which says what will happen; banner-offline fades the header's New session while actions are paused, though the page's own buttons below stay live-looking. Two old blemishes remain: in centre-row-menu the open menu's bottom edge cuts across the Work Max row's second line so it reads like strikethrough, and in user-menu-account your menu's bottom edge slices through the Runway card's "Not measured yet · Measure its limits" line the same way. In banner-quota the rail's Fake Desk row still shows its hover buttons over its "Working" status. tray-menu is the plain menu-bar menu and reads cleanly in both themes; it did not change this round.
