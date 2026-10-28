# E25 — Notifications and your menu: one model, one count, every view

> Source: the maintainer, 2026-10-27 (verbatim): "Approved: build the
> notification centre, banners, toasts, user menu and menu-bar item as
> designed, into 0.2."
> The approved design (the approval record):
> [`docs/design/notifications-and-user-menu-2026-10-27.html`](../design/notifications-and-user-menu-2026-10-27.html)
> — six parts: the header (a bell with one count, your avatar), the
> notification centre, the alert banners, the toasts, your menu, and the
> menu-bar item with the badge.
> Spec: `docs/review/deep/11-ux-redesign-screen-spec.md` §1.1 (one queue,
> one badge), §2.3 (Banner, Toast 6 s, Skeleton, EmptyState), §2.5 (the
> dialog contract), §3 (`refresh()` must surface failure; degraded =
> Banner + Retry), §4.14 (the centre), §4.17 (the menu-bar item), §8 (the
> state matrices: offline, empty), §10 (the copy bank). Wireframe 27
> (`docs/wireframes/index.html`). The spec-gap audit (branch
> `spec-gap-audit`, `docs/review/spec-gap-audit.md` §2, §3, top gaps 3–5)
> lists what was missing: no centre, no badge, no menu-bar item, no
> Banner primitive, silent backend failures, raw internal words in the
> quota alerts.
>
> **Law for every task here:**
> - **One notification model, in the backend.** Every source writes
>   through it: Needs you (worked out in the backend on its own tick, D-49:
>   the list the rail and Home show), an account almost out of room and its reset (the alert pass),
>   a session that finished, a crash and its restart, an update, a plan
>   renewing soon. The centre, the banners, the bell, the app badge, the
>   menu-bar item and the system notifications are views of it.
> - **One count.** The bell, the badge, the menu-bar item, the rail and
>   Home show the Needs you count. A snoozed need leaves the count until
>   its snooze ends; a muted one is still counted and only stops
>   interrupting (D-49, a change from the approved note).
> - **Nothing acts by itself.** A notice says what happened and offers its
>   one action (and the design's second); only the user's click runs it.
> - **The system-notification rules stay (AJ-E1510):** off until chosen,
>   once per state, never while AI Juggler is in front, quiet hours,
>   accounts left out. Snoozes and mutes now apply to them too.
> - **Plain words.** No internal ids, window codes (`five_hour`),
>   confidence badges ("(Measured)") or raw errors; a tool by its display
>   name; a window as "5-hour" or "weekly".
> - **Platforms:** the menu-bar item through Tauri's cross-platform tray
>   API; the app badge behind the platform seam
>   (`juggler_platform::system::app_badge`: the macOS Dock; Windows'
>   taskbar overlay and Linux say "not yet"); this computer's name and the
>   user's name from the same seam. No macOS-only code in shared code.
> - **Kept 30 days, read a page at a time** (the instant-and-paged rule).

## Step 1 — the model

### AJ-E2501 The one notification model | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P0 | Effort L | Depends: AJ-E1510, AJ-E611
**Goal**: One backend model of what needs the user and what happened, which every source writes through and every view reads.
**User story**: As someone juggling many sessions and accounts, I find everything that needed me or happened today in one place, told once, in plain words, and the app never acts for me.
**Requirements**:
1. Schema v37 SHALL add `notices` (key, kind, origin, needs, title, detail, project, data JSON — what a click opens and the facts a line is drawn from —, action, meter, created/updated/read/resolved/snoozed/dismissed times) and `notice_mutes` (project or kind); the upgrade keeps a `before-v36` copy.
2. A need SHALL be open once per key (a partial unique index); a happening SHALL be told once per key while kept.
3. Needs you SHALL be worked out in the backend (`features/notifications/needs.rs`, D-49) on the 5 s resources tick, with no window: AI Juggler's sessions that wait, may be stuck, hit their limit or failed (not parked, archived or in a forgotten project); other apps' sessions that ask or finished something not looked at since (the seen marks, the app's own last view; apps turned off in Settings left out); sign-ins; This Mac (its alerts, a session working it hard, processes left running); monthly caps; agents' messages and tasks. It is written into the model only when it changed: new items written (unread), changed ones updated (their marks kept), gone ones ended. Bounded sizes and counts.
4. Sources: an account almost out of room (the alert pass's emissions; ended, and "has room again" told, once the window resets), a session that finished, a crash restarted (try n of 3), an update found or ready, the digest's renewal lines.
5. Marks: read (one or all), snooze a subject (`s:<session>`, `c:<account>/<conversation>`, `signin:<account>`…: until a time ahead, or until woken), dismiss (a banner; the notice stays), mute a project or a kind (Settings lists and unmutes them). D-49: a snoozed subject leaves the count until its snooze ends — the backend's tick ends it, and every view follows with no window; the rail's own snoozes are these. Mutes and dismissals don't change the count.
6. Kept 30 days (an open need always); lists paged by keyset cursor.
7. System notifications SHALL be a view: their kinds on in Settings (now also an account almost out of room, a crash restarted, an account with room again), a snoozed need or muted kind/project never interrupting, the model's own words.
**UI/UX reasoning**: Nielsen #1 (visibility), #3 (user control: snooze, mute, every action a click); calm supervision (§1.1).
**Performance budget**: a sync of 50 needs ≤ 10 ms; a summary ≤ 5 ms; a page ≤ 20 ms (debug build, local SQLite).
**Quality gates**: `cargo test -p juggler-core --test notices_storage`; `cargo test -p juggler-app --lib features::notifications`; `--test notification_model --test system_notifications`; clippy `-D warnings`; the IPC audit (paged lists, thin async commands).
**Evidence/validation proof**: the tests above; the gallery family `96-notification-centre`.
**Visual testing**: AJ-E2503–E2507.
**Files to touch**: `crates/juggler-core/src/storage/notices.rs`, `storage/mod.rs`; `crates/juggler-app/src/features/notifications/` (centre, sources, commands, views, engine, prefs, mod); the hooks in `commands.rs` (the alert pass), `features/sessions/restart.rs`, `updates.rs`.
**Research basis**: spec §4.14; the audit §2; AJ-E611 (the alert pass, the digest), AJ-E1510 (the engine).
**Handover note**: The quota alert copy in `commands.rs` (`step_account_alerts`) and the `AppError` words are being fixed on main for 0.1.9; this model writes its own words from the quota display, so the two don't collide. The alert pass still runs on the window's 30 s poll (as before).

## Step 2 — one count, everywhere

### AJ-E2502 One count: the bell, the app badge, Needs you | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P0 | Effort M | Depends: AJ-E2501
**Goal**: The bell in the header, the app badge and the menu-bar item show the same number as Needs you on Home.
**Requirements**:
1. Needs you SHALL be the backend's one list (D-49): the rail (by place and the Project lens, the collapsed rail and folded headers), Home, the bell, the badge, the menu-bar item and the system notifications read it and its one count; the window works nothing out (`lib/needs-you.ts` reads it). The rail's Needs you also lists the needs that aren't a session's, so its number is the bell's. In the preview a lazy stand-in (`preview/needs.tsx`) runs the same rules over the preview's own data.
2. Home SHALL list the model's own needs (an account almost out of room) with "Hand over…".
3. The bell: the count (none at zero), "Notifications: 3 things need you, 2 unread" for screen readers, ⌘⇧N, opens the centre.
4. The app badge SHALL be set through the platform seam on every change.
**Quality gates**: `src/count-agreement.test.tsx` (the bell, Home and the rail); `tests/notification_model.rs` (the same count on the rail's list, the bell, the badge and the menu-bar item, worked out by the backend's tick with no window, a snooze ending bringing its need back); the gallery's header capture.
**Files to touch**: `crates/juggler-app/src/features/notifications/needs.rs`, `ui/src/lib/needs-you.ts`, `components/NoticeOpeners.tsx`, `preview/needs.tsx`, `components/NotificationBell.tsx`, `components/AppHeader.tsx`, `components/PlaceBoard.tsx`, `components/RailNoticeRow.tsx`, `screens/Home.tsx`, `crates/juggler-platform/src/system.rs`, `features/notifications/views.rs`.

## Step 3 — the views

### AJ-E2503 The notification centre | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P0 | Effort L | Depends: AJ-E2501
**Goal**: §4.14's centre, as designed: Needs action, Earlier, the marks, the digest.
**Requirements**:
1. A labelled dialog from the bell or ⌘⇧N: focus moves in, Tab stays inside, Esc closes and focus goes back.
2. Needs action mirrors Needs you (snoozed ones folded under "Snoozed: n"); each row: an unread dot, its mark, its title, its line in the user's time ("waiting 24 min", "7% of its 5-hour window left · resets at 9:09 PM"), its primary (Open, Review, Hand over…, Sign in…, Continue elsewhere…), the design's second (Snooze 1 h, Queue for the reset, Mark reviewed) and ⋯ (Snooze 1 h, Snooze until tomorrow, Mute this project, Mute this kind — the shared row menu, icons on every line).
3. Earlier: time · detail · its link (See why, What's new), 20 at a time, "Show earlier".
4. Mark all read; the digest ("Today: 4 finished · 1 restarted · Work Max resets 9:09 PM"); Notification settings.
5. Empty: "You're all caught up."; in the quiet hours: "Muted until 09:00 — Needs you still tracks 2."
6. Settings › Notifications › Muted lists the mutes (paged) with Unmute.
**Quality gates**: `src/components/notifications.test.tsx`; the gallery (populated, ⋯, empty, muted).
**Files to touch**: `ui/src/components/NotificationCentre.tsx`, `NoticeRow.tsx`, `NoticeMutes.tsx`, `lib/notice-words.ts`, `lib/notice-actions.ts`, `lib/notices-store.ts`, `lib/ipc/notices.ts`, `screens/settings/NotificationsPane.tsx`, `lib/shortcuts.ts`, `lib/app-keys.ts`.

### AJ-E2504 §2 Banner, the alert banners, and the offline banner | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P0 | Effort M | Depends: AJ-E2501
**Goal**: One shared Banner; the account banners in plain words with their meter and real actions; the window says when AI Juggler's own service stops answering.
**Requirements**:
1. `Banner` (tone, title, detail, meter, actions, close): every band banner uses it — an account almost out of room, an update, a parked session's reset, a queued resume, this computer's alerts, another computer's alerts.
2. The account banner: "Work Max is almost out of room · 7% of its 5-hour window left · resets at 9:09 PM · 2 sessions use it", the meter, Hand over…, Queue for the reset, Snooze 1 h, ×; at most three (one per account); a dismissed one stays in the centre; on Home the item is in Needs you instead.
3. `refresh()` SHALL no longer swallow a failure: with the app's service live, "Showing the last state AI Juggler knew — It couldn't reach its own service 20 s ago. Actions are paused until it's back." with Retry (`role=alert`); data stays; New session says why it waits.
**Quality gates**: `src/components/notifications.test.tsx`; the gallery (banner-quota, banner-offline).
**Files to touch**: `ui/src/components/Banner.tsx`, `NotificationRegion.tsx`, `OfflineBanner.tsx`, `lib/backend-health.ts`, `App.tsx`.

### AJ-E2505 §2 Toast | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P0 | Effort S | Depends: AJ-E1503
**Goal**: The one toast system as designed: bottom right, six seconds, a time bar, hover pauses, Undo or Retry, errors stay.
**Requirements**:
1. Six seconds unless an undo window says otherwise (a held Stop keeps its 5 s).
2. A bar shows the time left; hovering or focusing (not focus placed for the user) pauses it.
3. `action` (Retry, Sign in…) runs on the click and closes it; `tone: "error"` stays until closed.
4. Three at most; one polite live region.
**Quality gates**: `src/components/notifications.test.tsx`, `feedback.test.tsx`, `switch-flow.test.tsx`; the gallery (toast-undo, toast-error).
**Files to touch**: `ui/src/components/Toast.tsx`, `ToastStack.tsx`, `styles.css`.

### AJ-E2506 The menu-bar item (§4.17) | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P1 | Effort M | Depends: AJ-E2501
**Goal**: The count beside AI Juggler's icon in the menu bar, and a menu of what needs you and what is open.
**Requirements**:
1. Tauri's tray (`tray-icon`), made at start, redrawn on every change: the count as its title, "AI Juggler · 3 things need you" as its tooltip.
2. The menu: "Needs you · n" and five lines (with "and n more…"), "Open now · 10 · 4 working" and the apps, Open AI Juggler, New session…. A line brings AI Juggler forward on that notice; nothing else happens.
3. What is open comes from the window (Home's own count of open sessions by app).
**Quality gates**: `features::notifications::tray` tests (the builder, the ids, the gallery's fixture `ui/tests/visual/fixtures/tray-menu.json`); the gallery's tray-menu capture (drawn from that fixture; the system draws the real one).
**Files to touch**: `crates/juggler-app/src/features/notifications/tray.rs`, `views.rs`, `Cargo.toml`, `lib.rs`.

### AJ-E2507 Your menu (the avatar) | Status: DONE-PENDING-EVIDENCE (agent, 2026-10-27) | Priority P0 | Effort M | Depends: AJ-E1203
**Goal**: The header's Settings and More move into your menu, with who you are and your optional Figurehead account.
**Requirements**:
1. Navigation keeps Home · Accounts · Usage · History (D-25); ⌘, still opens Settings.
2. Without an account: your name and initial (the system's), "On this Mac · no Figurehead account", one quiet "Connect a Figurehead account (optional)" (D-18, D-46: no reminders). With one: the address shortened, the plan, Cloud backup (when last), My Macs (how many), and Disconnect Figurehead… last.
3. Settings ⌘,, This Mac, Processes, My Macs (without an account), Privacy, Appearance (Light / Dark / Auto inline), Keyboard shortcuts ?, Help and guides, Report a problem…, About and updates (version · channel).
4. The menu rule (AJ-E2106): icons on every line, three groups, `role=menu`, the arrows, Esc.
**Quality gates**: `src/components/notifications.test.tsx`; the gallery (user-menu, user-menu-account); the visual helpers reach the old More's screens through it.
**Files to touch**: `ui/src/components/UserMenu.tsx`, `icons-menu.tsx`, `RowMenu.tsx`, `MainNav.tsx`, `lib/nav.ts`, `lib/ipc/app.ts`, `crates/juggler-app/src/features/app/user.rs`.
**Handover note**: This Mac, Processes and My Macs were under More; the approved menu didn't list them, but removing More without a home for them would have hidden them, so they are in the "getting there" group. Say so in the gallery review.
