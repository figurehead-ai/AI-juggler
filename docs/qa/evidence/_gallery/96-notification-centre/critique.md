<!-- captures: 9e98682821bd -->
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

The darker canvas lifts the Notifications popover, the row menu, the toasts and your menu off the page in centre-populated, centre-row-menu, toast-undo and user-menu, and the grey light canvas does the same for their white surfaces; each need's Open, Continue on another account… and Continue elsewhere is one solid violet, so the row's main action is obvious and the secondary Snooze 1 h and Remind me at 3:48 AM sit visibly lower. In dark the lighter accent shows on those buttons, the header's New session, the avatar, the unread dots, Mark all read and Notification settings, and the toast's Undo (5s) outline and countdown bar; the light captures change only by a pixel in the This Mac line. The top bar stays one row, with "This Mac · Plenty of room" whole; in user-menu-account, where the Fleet tab joins the row, the This Mac line is left out instead, as it was before. banner-quota shows the slim banner with "Remind me at 3:48 AM", which says what will happen; banner-offline fades the header's New session while actions are paused, though the page's own buttons below stay live-looking. Two old blemishes remain: in centre-row-menu the open menu's bottom edge cuts across the Work Max row's second line so it reads like strikethrough, and in user-menu-account your menu's bottom edge slices through the Runway card's "Not measured yet · Measure its limits" line the same way. In banner-quota the rail's Fake Desk row still shows its hover buttons over its "Working" status. tray-menu is the plain menu-bar menu and reads cleanly in both themes; it did not change this round. The second pass shows only in toast-undo's rail, where the Open now feat/read-model row now reads "ai-j… · 3 h ago" with its time whole instead of "ai-jugg… · 3 …".

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: centre-populated (dark), centre-row-menu (light), user-menu-account (dark) and banner-offline (light) from the 0.5.1 shoot, then centre-populated (dark) and toast-undo (light) after the fix; beside the calm round-2 mock (`calm-ui/mock-r2-*`: one filled button per surface, the other needs' "Open its page" ghost). What 0.5.1 changed: AJ-E1548 #9 and #10, the bell holds "Needs you · 6", the latest few under Latest, then "See all on Home ›" (the 30-day history lives once, in Home's feed); "Remind me at 3:48 AM · a watch" says a watch is made (#14); calm surfaces for the popover, menus and toasts; the rail's rows.

- **Before the fix:** every need in the centre had a filled violet button (Open, Open, Switch account…, Continue elsewhere: four filled in one popover), louder than the mock and against BTN-1. **Fixed** (7e858e860): the first need's main action is the centre's one filled button, the others' are secondary, as Home's Needs you does. Re-rendered (gate: centre-*, toast-*) and checked by eye: centre-populated has one filled Open; toast-undo's centre (5 needs after the snooze) one filled Open.
- **Weak, kept:** the quota row's "Switch account…" and "Remind me at 3:48 AM · a watch" fill the row, so its ⋯ wraps alone onto a third line; the open ⋯ menu's edge still crosses the quota row's "window left · resets at 3:48 AM" line (centre-row-menu), and your menu's edge the Runway card's red bar (user-menu-account); banner-offline pauses actions but the page's Open its page stays filled (it needs no service, so that is honest).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | What needs you, acted on in place; history one click to Home. |
| 2 | Prominence | 4 | Needs first, latest after, digest in the foot. |
| 3 | Default | 4 | Each need's main action first in its row. |
| 4 | Mode | 4 | Calm popover and menus. |
| 5 | Mock-up | 4 | After the fix as the round-2 mock: one filled button per surface. |
| 6 | Areas | 4 | Header, needs, latest, foot. |
| 7 | Colour | 4 | Status marks by kind (speech, warning, error); amber/red only on needs; one accent fill. |
| 8 | Type | 4 | Title semibold, why line ink, place line muted. |
| 9 | Actions | 4 | One filled per view after the fix; the ⋯ wrap above. |
| 10 | Serious | 4 | Disconnect Figurehead… in danger red with its ellipsis; Snooze and Mute undoable (toast-undo). |
| 11 | Doors | 5 | Watches ›, See all on Home ›, Notification settings, Edit watch. |
| 12 | Words | 4 | "Needs you · 6"; "· a watch" reads tacked on but says what Remind does. |
| 13 | States | 4 | Empty, muted, populated, toasts, banners each captured. |
| 14 | Consistency | 4 | Row menus with icons on every line. |
| 15 | Accessibility | 4 | sr-only "— title" on each action; not re-walked this round. |
| 16 | Hand walk | 4 | PL1 walk (bell = rail count, F34); not re-walked here. |
| 17 | System prompts | 5 | None from the centre; the first watch alert's notification permission is primed at Watch (AJ-E1548 R6). |
| 18 | Usability | 4 | AJ-E1548 (below). |

**AJ-E1548 match:** (1) one meaning per name: "Needs you" in the bell, rail and Home. Match. (2) Each "mark all" says what it clears (Mark all read). Match. (3) The bell is no second history: Latest few, then See all on Home. Match. (4) n/a here. (5) Notifications primed at Watch, not here. Match. (6) The bell's count equals the rail's (6 and 6). Match.

Lowest: 4 (after the fix; line 9 was 2 before it).
