# E29 — Trust and the finished basics: what the spec promised for the daily loop

> Source: the maintainer's lock-in of 0.4, 2026-10-30 (D-59, recorded in
> [`docs/plan/ROADMAP.md`](../plan/ROADMAP.md) "0.4 — Trust and the
> finished basics"): "0.4 makes AI Juggler trustworthy and complete for
> daily use: secure enough to recommend, the spec's promises for the core
> workflows actually there, and proven on real tools."
> Security is E20 (AJ-E2001…AJ-E2019); the live checks are AJ-E1305,
> AJ-E2301, AJ-E808 and AJ-E710. This epic is the rest: the spec's
> promises the code still lacks, from the spec-gap audit
> ([`docs/review/spec-gap-audit.md`](../review/spec-gap-audit.md),
> 2026-10-27, with its "Status at 0.2.0" table), each re-read against
> `release/0.4` on 2026-10-30. Anything fixed since the audit is left out
> (see "Already there" below).
> Read before any task here: `docs/review/deep/11-ux-redesign-screen-spec.md`
> (cited as "deep/11 §") §1.4, §2.5, §3, §4.1, §4.5, §4.7, §4.11, §4.14,
> §4.15, §4.16, §4.17, §8, §9, §10; `docs/wireframes/index.html` screens
> 02, 07, 16, 17, 20, 26, 27 (cited "wfNN"); `docs/SPEC.md` §9 and §12.1;
> D-07, D-10, D-25, D-36, D-38, D-40, D-44, D-45, D-46, D-49, D-50, D-57,
> D-58; E28's shared controls (`ui/src/components/controls/`).
>
> **Splits, and why.** The lock-in named six areas. Two are split here
> because their parts live on different screens, read different data and
> can't share one status honestly:
> - "Cost and quality" is two tasks. Cost is the Usage screen and the cost
>   ledger (wf20, deep/11 §4.11); quality is the account's page and the
>   session states and transcripts (wf07, D-07). AJ-E2903 reads AJ-E2902's
>   "finished task".
> - "Safe edges" is four tasks, one per surface: launch settings (Accounts),
>   an ended session's page, the rail's splitter, and Home. Each has its own
>   files, tests and gallery; one of them (the delete) is a data-loss fix
>   and must not wait on the others.
>
> **Order.** All ten can start at once. AJ-E2907 (the delete) and AJ-E2909
> (the splitter) are small and close a data-loss risk and a WCAG level A
> failure: do them first. AJ-E2903 follows AJ-E2902's ruling on "finished
> task". AJ-E2910's new keys go through the shortcut registry, so
> AJ-E2905 makes them rebindable without extra work.
>
> **Law for every task here:**
> - **Design quality is part of done (D-57).** One grid, a clear
>   hierarchy, the app's own icons (`ui/src/components/icons*.tsx`), the
>   shared controls (`ui/src/components/controls/`: ChoiceLine, Picker,
>   Segmented, Toggle, Disclosure; never a system select, radio or
>   checkbox), disabled states that say why, body 14 px and nothing under
>   12 px, both themes, and a critique beside each gallery family
>   (AJ-E2814).
> - **Plain words (D-25).** No internal ids, codes or raw errors on screen;
>   a tool by its display name; inferred states say "probably" (D-36);
>   launch settings, never "profile" or "recipe" (D-10, D-35).
> - **Progressive disclosure.** The default shows what the daily loop
>   needs; depth is one step away (▸ Details, ⋯, a sheet), never on by
>   default.
> - **Instant and paged.** Cached or skeleton data at once; every growing
>   list paged by keyset cursor or windowed; heavy parts loaded lazily
>   (`ui/src/lazyParts.ts`, `lib/lazy.tsx`) and prefetched after first
>   paint; nothing re-reads transcripts on open when a summary exists.
> - **Honest numbers.** A number the tool doesn't record isn't shown;
>   estimates carry "~" and say they are estimates (deep/11 §4.11, §10);
>   unknown is never 0 % or $0.
> - **Nothing acts by itself** (deep/11 §0): the app prepares and
>   suggests; only a click changes anything.
> - **Cross-platform seams (D-52).** Tool specifics in adapters
>   (ADR-0002), OS specifics behind `juggler-platform`; no new macOS-only
>   assumption in shared code (⌘ reads as Ctrl on Windows and Linux).
> - **Every change** gets vitest, gallery captures (both themes, 1024×700
>   and 1280×800 at least), the a11y gate (axe 0 critical/serious), a
>   manifest under `docs/qa/evidence/<task-id>/`, and a walk by hand in the
>   real app before it is called done.

## Where each task comes from

| Task | Spec (deep/11) | Wireframe | Decision | Audit (`spec-gap-audit.md`) | Today's code |
|---|---|---|---|---|---|
| AJ-E2901 ⌘K palette | §4.17, §2.5, §8 row 4.17, §9 | wf27 | D-36, D-38, D-58 | top 15 #10; §1.5 rows "Groups", "Recents…", "Global commands", "Dialog contract" | `ui/src/components/GoToPalette.tsx`, `PaletteActions.tsx`, `ui/src/lib/palette-commands.ts` |
| AJ-E2902 Cost you can act on | §4.11, §8 row 4.11 | wf20 | D-07, D-12 | top 15 #12; §11 rows "Cost per finished task", "Wasted spend", "Per-account table", "Weekly cost review", "Pricing version" | `ui/src/screens/Usage.tsx`, `ui/src/components/usage/`, `ui/src/components/CostMatrix.tsx`, `crates/juggler-app/src/commands/costs.rs` |
| AJ-E2903 Quality per account | §1.4 (L3) | wf07 | D-07 | "Next after these" first bullet; §5 row "Quality signals (D-07)" | `ui/src/components/RightInspector.tsx:966-1011`, `crates/juggler-core/src/quality/signals.rs` |
| AJ-E2904 Notification thresholds and Send test | §4.14, §4.15 | wf26 | D-49 | §2 rows "waiting longer than a threshold", "quota below a threshold", "Send test", "waiting threshold 5 min; quota 80 % / 95 %" | `ui/src/components/NotificationSettings.tsx`, `crates/juggler-app/src/features/notifications/`, `crates/juggler-core/src/alerts.rs` |
| AJ-E2905 Settings search and rebindable shortcuts | §4.15, §3 keys, §9 | wf26 | D-36, D-50 | §1.4 row "Settings › Shortcuts"; §14 rows "`/` filter", "Shortcuts table / rebind / reset" | `ui/src/screens/Settings.tsx`, `ui/src/screens/settings/panes.ts`, `ui/src/lib/shortcuts.ts`, `ui/src/lib/app-keys.ts` |
| AJ-E2906 First run shows its confidence | §4.16 | wf16, wf17 | D-18, D-25, D-46 | top 15 #14; §9 rows "Found / Likely / Guessed", "Rescan" | `ui/src/screens/FirstRun.tsx`, `ui/src/screens/firstRunScan.ts`, `ui/src/types/generated/DiscoveredInfo.ts` |
| AJ-E2907 Confirm before deleting launch settings | §4.5, §10 "Delete profile blocked" | — | D-10, D-35 | "Next after these"; §10 row "Delete launch settings asks first"; §4.2 row "{n} live sessions use this profile" | `ui/src/screens/accounts/ProfileRow.tsx:208`, `crates/juggler-app/src/commands.rs:2690-2707` |
| AJ-E2908 An ended session's page | §4.7, §10 "Resume vs Relaunch" | wf09 | D-05, D-40, D-45 | "Next after these"; §5 rows "Archive / Replay / Delete…", "Copy ID", "Resume vs Relaunch" | `ui/src/pages/ItemHeader.tsx:184-291`, `ui/src/lib/action-specs.ts:121-143` |
| AJ-E2909 A keyboard-operable rail splitter | §2.5, §9 | wf01 | — | §1.2 row "Splitter is keyboard-operable" | `ui/src/components/WorkflowRail.tsx:269-289` |
| AJ-E2910 Home: filter chips, keys, Mark all reviewed | §4.1 | wf02 | D-25, D-36, D-49 | §1.3 rows "Chips All / Needs reply / …", "Keys r/h/s/e", "Mark all finished as reviewed" | `ui/src/screens/Home.tsx`, `ui/src/components/HomeNeeds.tsx`, `ui/src/components/HomeRecent.tsx` |

## Already there (left out, re-read 2026-10-30)

| Promised | Where it is now |
|---|---|
| The backend-failure banner (top 15 #5) | `ui/src/components/OfflineBanner.tsx` (E25, AJ-E2504) |
| ⌘K's global commands, partly: every screen, New session…, New project…, Notifications, Keyboard shortcuts, Show or hide the list, and "Search History for …" | `ui/src/lib/palette-commands.ts:80-117`; before typing only New session shows (`top: true`, `:83`, `:110-112`) |
| A failed settings save puts the value back | `ui/src/lib/optimistic.ts:1-40` (`saveOptimistic`), used at `ui/src/components/NotificationSettings.tsx:44-52,84-89` and in `SettingsSections.tsx` |
| Per-project mute | the centre's row menu (`ui/src/components/NoticeRow.tsx:128`, "Mute this project"), listed and undone in Settings › Notifications › Muted (`ui/src/screens/settings/NotificationsPane.tsx:24-27`, `ui/src/components/NoticeMutes.tsx:13-18`) |
| Digest and quiet hours | `ui/src/components/NotificationSettings.tsx:91-139`; the digest is hourly (no time setting: see AJ-E2904's open question) |
| The cost tile never says "$0" | `ui/src/components/usage/Dashboard.tsx:130-149` ("No list price" / "No usage yet") |
| The Figurehead step in first run (D-46) | `ui/src/screens/FirstRun.tsx:1287` (`FigureheadStep`); not re-added |
| The session History tab, redacted History snippets | fixed in 0.1.9 (the audit's status table, rows 1–2) |

## Step 1 — the palette

### AJ-E2901 The command palette: commands with nothing selected, accounts and apps, recents, fuzzy match, a preview, a real dialog | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2801
**Goal**: ⌘K does things, not only goes places: it opens as a proper dialog, shows recents and the top actions before you type, finds sessions, apps, projects and accounts by loose typing, previews a result with →, and hands the full search to History.
**User story**: As someone with twenty sessions over four accounts, I press ⌘K, type "wrk bil", see the billing session on Work first, press → to check it is the one waiting, and ↩ to open it; with nothing typed I see what I opened last and New session.
**Requirements**:
1. The palette SHALL meet the dialog contract (deep/11 §2.5, §9): `role="dialog"`, `aria-modal="true"`, an accessible name; Tab and ⇧Tab stay inside (the shared `keepTabInside`, `ui/src/components/form.tsx:39`); Esc closes; focus returns to the element that had it before ⌘K (§9: "store + restore"). The combobox pattern stays (`aria-expanded`, `aria-activedescendant`).
2. With nothing typed it SHALL show, in this order: the selected item's actions (today's behaviour, `PaletteActions.tsx:59-75`); **Recent** (up to 8 things chosen from the palette: sessions, apps, projects, accounts, screens, commands); **Actions**: New session…, Settings, and — only when it applies — Processes with what is left running ("Processes — 2 left running", wf27's "Open Cleanup Centre (2 leftovers)") and Notifications with the unread count (deep/11 §8 row 4.17: "empty = recents + top actions").
3. Typing SHALL list groups in this order, each with its section label: Selected · Actions · Sessions · Apps · Projects · Accounts · Search. Each group shows at most 5; more say "4 more" and are reached through See all results.
4. **Accounts** (deep/11 §4.17; audit §1.5): each account by its label, its tool's display name and its room in words ("31 % of the 5-hour window left", "not measured yet" — never 0 % when unknown); ↩ opens its page; ⌘↩ starts New session on it, through the same sheet as everywhere (D-45's suggestion rules unchanged).
5. **Apps** (AJ-E2106 Req.4, whose apps part this task takes over; D-36 places, D-38 icons): each app open now with its own icon read at run time and its launch settings' badge; ↩ opens its place; ⌘↩ brings it forward through `juggler-platform` (no macOS call in shared code).
6. Matching SHALL be fuzzy: the typed letters in order within a title, label, folder or the item's other words, ranked exact prefix, then word start, then the rest, then by recency; the matched letters are shown bold (not by colour alone, WCAG 1.4.1). The rail's search words (`is:`, `app:`, `project:`, `account:`; `parseQuery`, `ui/src/lib/rail-view.ts:175`) SHALL keep working.
7. **→** on a session, app, project or account SHALL open a preview beside the list (a `complementary` region the combobox controls): its state word, where it runs, its account and last activity, and its primary actions (Open, Resume or Continue on another account… where they apply); ← returns to the list. The preview reads only cached state; any message text comes only through the existing last-messages reader (AJ-E2108), never a new transcript read.
8. The footer SHALL read "↑↓ move · ↩ open · ⌘↩ go to its app · → preview · Esc close" and carry **See all results →**, which opens History searched for the words (D-25: History is the search screen; the same as today's "Search History for …" row, `palette-commands.ts:115`).
9. No results: "No matches for “…”." with Search History for “…” focused. Offline (the backend not answering): titles from the cache only, with "AI Juggler's service isn't answering — showing what was open last." (deep/11 §8 row 4.17).
10. Recents SHALL be kept per Mac as ids only (no titles or message text), at most 8, dropped when the item is gone, and cleared by Settings › Advanced › local cache's Clear (D-58).
11. The palette SHALL stay a shortcut, never the only path (wf27): every action it lists exists as a visible control somewhere. The registry's label for ⌘K SHALL change from "Search and go to a session" (`ui/src/lib/shortcuts.ts:99`) to "Search, act or go".
**UI/UX reasoning**: deep/11 §4.17 and wf27; Nielsen #7 (flexibility: accelerators for experts) and #6 (recognition: recents and top actions before typing); WAI-ARIA APG combobox and modal-dialog patterns; WCAG 2.4.3 (focus order) and 2.1.2 (no trap except the modal's). States: empty with recents; empty on a first run (no recents: top actions only); typing with each group; one group capped with "n more"; preview open; no results; offline; a selected item's greyed action with its reason; the hotkey capture (kept). Keys: ⌘K opens and closes; ↑↓; ↩; ⌘↩; → and ←; Esc.
**Performance budget**: open to first paint ≤ 50 ms with 500 sessions, 200 projects, 20 accounts and 30 apps (cached data, no IPC on open); each keystroke re-ranks in ≤ 16 ms on the same fixture; the preview paints in ≤ 50 ms; the palette code stays out of the first bundle (lazy, prefetched after first paint).
**Quality gates**: vitest `src/components/goto-palette.test.tsx`, `palette-commands.test.tsx`, `palette-actions.test.tsx` (extended) and a new `src/lib/palette-match.test.ts` (ranking, rail words, highlight ranges); Playwright `keyboard-flows.spec.ts` (focus returns to the invoker; Tab stays inside); `just ui`; the a11y gate; `scripts/size_budget.py`.
**Evidence/validation proof**: `cd ui && npx vitest run src/components/goto-palette.test.tsx src/lib/palette-match.test.ts` all pass; a timing test on the 500-session fixture recorded in `docs/qa/evidence/AJ-E2901/timing.json`; the gallery family `palette`; manifest `docs/qa/evidence/AJ-E2901/manifest.md`; pass = 0 failures, 0 axe critical/serious, p95 keystroke ≤ 16 ms.
**Visual testing**: `ui/tests/visual/menus-gallery.spec.ts` extended (or a new `palette-gallery.spec.ts`): every state above at 1024×700, 1280×800 and 1440×900, dark and light; axe on each.
**Files to touch**: `ui/src/components/GoToPalette.tsx` (split: new `ui/src/components/palette/` for groups, preview and footer, within the size budget), `PaletteActions.tsx`; `ui/src/lib/palette-commands.ts`; new `ui/src/lib/palette-match.ts`, `ui/src/lib/palette-recents.ts`; `ui/src/lib/rail-view.ts` (`paletteMatches`), `ui/src/lib/shortcuts.ts`; `ui/src/App.tsx` (the invoker's focus); `ui/src/components/form.tsx` (export `keepTabInside` if needed); the local-cache clear in `ui/src/screens/settings/LocalCacheSection.tsx`.
**Research basis**: deep/11 §4.17 ("recents first, fuzzy over titles", "`→` preview", "See all results →"), §2.5, §8 row 4.17, §9 ("`⌘K` from anywhere returns focus to invoker"); wf27; audit top 15 #10 and §1.5; `ui/src/components/GoToPalette.tsx:237-247` (a plain overlay and `Card`, no `role="dialog"`), `:147-155` (focus on open, never restored), `:104-109` (substring `includes`); `ui/src/lib/palette-commands.ts:100-117` (substring match; only `top` commands before typing); `ui/src/components/form.tsx:39,86-112` (the trap and focus return to reuse); AJ-E2106 Req.4; https://www.w3.org/WAI/ARIA/apg/patterns/combobox/; https://www.w3.org/WAI/ARIA/apg/patterns/dialog-modal/.
**Handover note**: Don't add a second matcher for the rail: extend `paletteMatches` so the rail's search and ⌘K agree. A modal palette must still let the global hotkey summon it over another app (`lib/outside.ts` → `App.tsx`); restore focus only when the invoker is still in the document. Open questions: (1) wf27 also lists "Park until reset" in ⌘K; the action registry has no park action today (`ui/src/lib/action-specs.ts`), so it isn't added here — say if it should be. (2) Should ⌘K also find individual settings once AJ-E2905's index exists? The spec lists "Settings…" only.

## Step 2 — cost and quality

### AJ-E2902 Cost you can act on: per finished task, wasted spend, a weekly review, top project per account, dated prices | Status: TODO | Priority P0 | Effort L | Depends: AJ-E2801; the ruling in the Handover note (Q1) for Req.1
**Goal**: Usage answers "am I getting value from these subscriptions?": what a finished task costs, what went on sessions that crashed or stuck, which project each account went on, a review of last week, and which price list the estimates use.
**User story**: As someone paying for three subscriptions, I open Usage and see that a finished task costs about $1.90, that $6.30 went on two crashed sessions I can open, that Work went mostly on AIJuggler, and on Monday I read last week's review in one sheet.
**Requirements**:
1. **Per finished task** (wf20 "Per finished task $1.92 avg · 44 tasks"; D-07 "cost per completed task where known"): a tile SHALL show the range's average estimated cost and tokens per finished task with the count ("~$1.92 a finished task · 44 finished"); with none finished: "No finished tasks in this range". What a "finished task" is waits on Q1; it SHALL be one definition used here and by AJ-E2903.
2. **Wasted spend** (wf20 "Wasted (crashed/stuck) · Links to those sessions"): a tile SHALL show the estimated cost of sessions that ended in a crash or were probably stuck when they ended ("~$6.30 on 2 sessions that crashed or probably stuck"); its link opens a sheet listing those sessions (paged), each opening its page. Stuck is inferred, so the words say "probably" (D-36).
3. **Cost by account** (wf20's table; deep/11 §4.11 breakdown): a section SHALL list each account with tokens, ~cost, its top project by cost (a link to the project) and how sure the price is ("list price" / "no list price for one model"); sortable by cost; subscription fees stay out of these figures (wf20: "never mixed into it").
4. **Weekly cost review** (wf20): a button on Usage SHALL open a sheet for the last full week (the locale's first day of week) against the week before: total ~cost and tokens, per account with its top project, per finished task, wasted spend, and the five costliest sessions; with Export CSV of that week (the existing export, `ui/src/components/CostMatrix.tsx:232`, scoped to the week).
5. **Prices** (deep/11 §4.11 "pricing version … (never silent)"): the cost tile's note and Prices and details SHALL name the price list and its date ("List prices of 12 Oct 2026") from the bundled table and any later sync (`crates/juggler-app/src/commands/costs.rs:141-160`); the stale warning (`CostMatrix.tsx:236-240`) stays. **Update** SHALL NOT be shown until Q2 is ruled (D-25: an action that isn't built isn't shown).
6. Every figure SHALL come from the usage history's summaries (AJ-E2203, `crates/juggler-app/src/features/usage_history/`) and the cost ledger, never a sum of rows on screen or a transcript read on open; every estimate carries "~" (deep/11 §10); unknown is words, never $0.
**UI/UX reasoning**: wf20's purpose ("answered with measured numbers, exportable for expenses"); deep/11 §1.4 (the tiles are L2, the review sheet and the table L3); Nielsen #1 and #8; the readable-and-visual rule (tiles in the grid, a table not a strip). States: populated; a range with nothing finished; nothing wasted ("Nothing went on crashed or stuck sessions"); a model with no list price; prices stale; the review sheet for a week with no usage; loading (skeleton tiles); offline (last figures from the cache with the shared banner). Copy: "API-equivalent estimate — your subscription price doesn't change per token." (deep/11 §4.11).
**Performance budget**: the new tiles and the account table from summaries in ≤ 50 ms in the backend (90 days, 10 accounts, 2,000 sessions; debug build, local SQLite); the review sheet ≤ 100 ms; the sheet's code loads only when opened.
**Quality gates**: `cargo test -p juggler-app --test cost_cuts` (new: finished, wasted, top project, week boundaries in two locales); vitest `src/components/usage/cost-tile.test.tsx` (extended), new `src/components/usage/cost-review.test.tsx`; the IPC audit (`scripts/ipc_audit.py`: new reads registered as Read, paged where they list); `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; `just load-gate` row "Usage cost cuts, 2,000 sessions" within budget; the gallery family `usage`; manifest `docs/qa/evidence/AJ-E2902/manifest.md`; pass = 0 failures, 0 axe critical/serious, figures reconciling with Prices and details' total for the same range to the cent.
**Visual testing**: `ui/tests/visual/usage-gallery.spec.ts` extended: the new tiles, the account table, the wasted sheet, the review sheet, each empty state, 1024×700, 1280×800 and 1440×900, dark and light; axe on each.
**Files to touch**: `ui/src/screens/Usage.tsx`, `ui/src/components/usage/Dashboard.tsx`, `TopLists.tsx`, new `ui/src/components/usage/CostByAccount.tsx`, `CostReview.tsx`, `WastedSheet.tsx`; `ui/src/components/CostMatrix.tsx`; `ui/src/lib/ipc/costs.ts`; `crates/juggler-app/src/commands/costs.rs`, `crates/juggler-app/src/features/usage_history/` (`fold.rs`, `dto.rs`, `commands.rs`); `crates/juggler-core/src/cost_ledger.rs`; new `crates/juggler-app/tests/cost_cuts.rs`; `crates/juggler-app/src/lib.rs`.
**Research basis**: wf20 (`docs/wireframes/index.html:468-480`); deep/11 §4.11; D-07, D-12; audit top 15 #12 and §11 rows; `ui/src/screens/Usage.tsx:57-62,87-91` (Prices and details); `crates/juggler-app/src/commands/costs.rs:17-36` (`pricing_stale`, `pricing_last_synced_ms` sent), `:141-160` (nothing writes `pricing_synced_at`); `crates/juggler-core/src/quality/pricing.rs:20-38` (version, source, captured date); `ui/src/types/generated/UsageHistoryInfo.ts` (`by_profile`, `by_project`, `top_sessions`); https://github.com/ryoppippi/ccusage (the per-block cost cut deep/11 benches).
**Handover note**: The cost of a session that moved accounts belongs to each account for its own part; don't double count across a switch. Open questions: **Q1** — what is a "finished task"? wf20 (2026-10-13) predates pool tasks; today it could be (a) a session that ended on its own as finished (not stopped, crashed or handed over), or (b) a pool task that reached Done (D-44), costed by its holder sessions while they held it. (a) works for every user; (b) only with pools. **Q2** — where would Update fetch prices from? deep/11 §4.11 names a models.dev sync; no ADR covers that network call, and E20's network rules apply. Until ruled, prices change only with an app update. **Q3** — should the weekly review also arrive as a notice on Monday, or only open on demand (this task: on demand)? Related but out of scope: the user's own prices have a backend (`costs.rs:325-390`, `pricing_override_set`/`_clear`) and no screen.

### AJ-E2903 Quality per account: errors and retries, stuck episodes, context at finish, tokens per finished task | Status: TODO | Priority P1 | Effort M | Depends: AJ-E2902
**Goal**: D-07's measurable signals, per account and over time, so the user can tell which account is actually performing — each signal labelled with its source, and no composite score.
**User story**: As someone choosing between Work and Personal for a long task, I open each account and see that Work's sessions hit 2 stuck episodes this week and finish at 71 % context on average, and that Personal's had 9 errors in 300 turns.
**Requirements**:
1. An account's page SHALL show a "How its sessions went" section (last 7 days, wf07) under its quota card, with:
   - **Errors and retries**: "9 errors or retries / 300 turns", from the same counter as a session's (`crates/juggler-core/src/quality/signals.rs:111-131`), never a bare percentage; a tool with no error feed says why ("Codex doesn't record errors in its files").
   - **Stuck episodes**: "2 times probably stuck", counted from AI Juggler's recorded state changes (`instance_transitions`, `crates/juggler-core/src/storage/sessions.rs:154,232-255`); sessions AI Juggler didn't watch are not counted and the line says so.
   - **Context at finish**: the average share of the context window in use when sessions ended, only where the tool reports context (`signals.rs:132-140`: estimating context from cumulative tokens is prohibited); otherwise "Not reported by <tool>".
   - **Tokens per finished task**: from AJ-E2902's definition.
2. Each line SHALL carry its source in words, as the session's signals do (`TIER_WORDS`, `ui/src/components/RightInspector.tsx:970-975`: "read from the session", "worked out from what's measured").
3. An ended session's Usage tab SHALL add its context at finish and its stuck episodes (its error line exists).
4. There SHALL be no composite score anywhere (D-07; the existing grep gate stays green).
5. A new section with no data in the window SHALL wait until it has some (D-25: "a row with nothing to show waits until it has data").
**UI/UX reasoning**: wf07 ("quality signals tell you which account is actually performing"); D-07; deep/11 §1.4 (L3 depth on the account's page, not the rail). Nielsen #1, #2 (match the real world: "9 errors / 300 turns" beats "3 %"). States: populated; a tool without an error feed; a tool without context reports; no sessions in 7 days (section hidden); loading; offline.
**Performance budget**: the section's figures in one backend read ≤ 30 ms (10 accounts, 2,000 sessions, 7 days) from stored summaries; context at finish is stored when a session ends, never recomputed from transcripts on open.
**Quality gates**: `cargo test -p juggler-core --test quality_rollup` (new) and `cargo test -p juggler-app --test account_quality` (new); vitest `src/pages/account-quality.test.tsx` (new); the composite-score grep gate; the IPC audit; `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `account-page` with the section; manifest `docs/qa/evidence/AJ-E2903/manifest.md`; pass = 0 failures, 0 axe critical/serious, and each figure checked against a hand count on the replica fixture (`just replica`).
**Visual testing**: `ui/tests/visual/account-settings-gallery.spec.ts` (or `accounts-gallery.spec.ts`) extended: the section populated, each "not reported" line, 1024×700 and 1280×800, dark and light; `session-pages-gallery.spec.ts` for an ended session's Usage tab.
**Files to touch**: `ui/src/pages/AccountPage.tsx`, new `ui/src/pages/account/Quality.tsx`, `ui/src/pages/account-quality.test.tsx`; `ui/src/pages/session/UsageTab.tsx`; `ui/src/lib/ipc/` (a quality read); `crates/juggler-core/src/quality/signals.rs`, new `crates/juggler-core/src/quality/rollup.rs`; `crates/juggler-core/src/storage/sessions.rs` (context at finish, at the next free schema version if a column is needed); `crates/juggler-app/src/commands.rs` (`instance_quality` neighbours) or a new `crates/juggler-app/src/features/quality/`; `crates/juggler-app/src/lib.rs`.
**Research basis**: D-07 ("Error/retry rate, stuck/waiting state, context-window use, tokens and cost per completed task where known … No composite quality score"); wf07 (`docs/wireframes/index.html:304-313`: "Error / retry rate (7d)", "Stuck episodes 2 (7d) supervisor", "Context at finish avg 71 %", "Tokens / completed task"); audit §5 row "Quality signals (D-07)" and "Next after these"; `ui/src/types/generated/QualitySignalsInfo.ts` (per live session only).
**Handover note**: wf07 writes "3 % (7d)"; `signals.rs` says never a bare percentage — keep the numerator and denominator. Open questions: (1) wf07 has a row "Composite score: Unavailable — by design"; D-25 says a row with nothing to show waits. This task leaves it out; say if you want it. (2) Should Usage also get a table comparing accounts on these signals? The sources put them on the account only.

## Step 3 — notifications

### AJ-E2904 Notifications tuned to you: a waiting threshold, quota warnings at 80 and 95 %, Send test | Status: TODO | Priority P1 | Effort M | Depends: AJ-E2801
**Goal**: The interrupts arrive when the user would want them: a wait only after it has lasted, quota warnings at the user's two levels, and a test that shows the system lets them through.
**User story**: As someone who answers quick questions from the rail, I'm only notified when a session has waited five minutes; I'm warned at 80 % and again at 95 % of a window; and after setting this up I press Send test and see one arrive.
**Requirements**:
1. **Waiting threshold** (deep/11 §4.15 "waiting threshold (5 min)"; §4.14 "Waiting>threshold"): a Picker "Tell me when a session waits longer than" with Right away, 2, 5, 10, 15, 30 minutes, default 5; the system notification for a wait fires once the wait has lasted that long and once per waiting spell (the engine's rearm rule stays, `crates/juggler-app/src/features/notifications/engine.rs:46-53`). Needs you, the bell and the count are unchanged: they show a wait at once (D-49, one count).
2. **Quota warnings** (§4.15 "quota 80%/95%"): two Pickers, "First warning at" (default 80 %) and "Last warning at" (default 95 %), the last above the first; each fires once per window for an account whose window is measured or computed; an estimated reading says "about"; the time-to-empty warnings stay (`crates/juggler-core/src/alerts.rs:61-68`), and the fixed 90 % bound (`PCT_CRITICAL`) gives way to the user's levels.
3. **Send test** (§4.15): a button SHALL send one system notification now ("AI Juggler test — notifications reach you."), regardless of quiet hours and of AI Juggler being in front, then say "Sent. If nothing appeared, allow AI Juggler in <the system's notification settings>", the words for each OS from the platform seam.
4. Each new setting SHALL save through `saveOptimistic` (`ui/src/lib/optimistic.ts`), so a failed save puts the value back and says why (§4.15, §8).
5. The settings SHALL be stored with the other notification preferences (`NotificationPrefsInfo`), with serde defaults so an existing store reads the defaults above; the generated type updates.
6. The words SHALL be plain units (§4.15 "Warn when a session waits longer than 5 minutes"); no "threshold", no window codes.
**UI/UX reasoning**: deep/11 §1.1 (calm supervision: "silence is a feature") and §4.14 ("interrupt only when actionable"); Nielsen #3 (user control) and #9 (help users recognise and recover: Send test makes a blocked permission visible). States: defaults; Right away; last level not above first (the Picker offers only higher levels); Send test sent; Send test failed (the reason in words); notifications all off (Send test still works, to check the permission); a failed save reverted.
**Performance budget**: no new timer: the wait threshold is checked on the existing notification tick; the setting change applies on the next tick (≤ 1 s); the pane paints from cached prefs in ≤ 50 ms.
**Quality gates**: `cargo test -p juggler-app` for `features/notifications` (new: a wait under and over the threshold, once per spell; 80/95 once per window; a test notification bypasses quiet hours and the in-front rule and nothing else); `cargo test -p juggler-core` for `alerts` (the levels from settings); vitest `src/screens/system-notifications.test.tsx`, `src/components/settings-revert.test.tsx` (extended); `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; a live check on this Mac: Send test shows one system notification (a capture, checked by eye, in `docs/qa/evidence/AJ-E2904/`); the gallery family `settings-notifications`; manifest `docs/qa/evidence/AJ-E2904/manifest.md`; pass = 0 failures, 0 axe critical/serious, the live notification seen.
**Visual testing**: `ui/tests/visual/notifications.spec.ts` (or `notification-centre-gallery.spec.ts`) extended: the Notifications pane with each Picker open, Send test's result line, a reverted save, 1024×700 and 1280×800, dark and light.
**Files to touch**: `ui/src/components/NotificationSettings.tsx`, `ui/src/screens/settings/NotificationsPane.tsx`, `ui/src/lib/bridge.ts` or `ui/src/lib/ipc/notifications.ts` (a test command), `ui/src/types/generated/NotificationPrefsInfo.ts` (generated); `crates/juggler-app/src/features/notifications/prefs.rs`, `engine.rs`, `mod.rs`, `commands.rs`, `sources.rs`; `crates/juggler-core/src/alerts.rs`; `crates/juggler-platform` (the settings words per OS); `crates/juggler-app/src/lib.rs` (the test command as Write).
**Research basis**: deep/11 §4.14, §4.15 ("waiting threshold (5 min), quota 80%/95%, digest time, per-project mutes, `Send test`"); audit §2 rows; `ui/src/types/generated/NotificationPrefsInfo.ts` (only `quiet_start`, `quiet_end`, `digest_muted_accounts`); `crates/juggler-app/src/features/notifications/engine.rs:34-43` (a wait notifies on entering the state); `crates/juggler-app/src/features/notifications/sources.rs:19-23` (the low kinds); `crates/juggler-core/src/alerts.rs:61-68` (`TTE_TRAY_MIN` 45, `TTE_CRITICAL_MIN` 15, `PCT_CRITICAL` 90); `crates/juggler-app/src/features/notifications/mod.rs:564-578` (the sink: Tauri's notification plugin, cross-platform).
**Handover note**: The test must go through the same sink as real notifications (or it proves nothing), yet skip the in-front and quiet-hours rules: the user presses it while AI Juggler is in front. The alert machine never goes backwards without a reset (`alerts.rs:1-9`); adding a level must keep that. Open questions: (1) Default wait: §4.15 says 5 min; today it is "right away", so existing users get a change unless you want Right away kept for them. (2) §4.15 also lists a digest time; the digest is hourly today. Not in this task unless you want it.

## Step 4 — Settings

### AJ-E2905 Settings you can search, and shortcuts you can rebind with a conflict check and reset | Status: TODO | Priority P1 | Effort L | Depends: AJ-E2801
**Goal**: Any setting is a few letters away, and every shortcut can be changed to fit the user's hands, refused with a reason when it would clash, and reset.
**User story**: As someone who remembers "quiet hours" but not which pane holds it, I press `/` in Settings, type "quiet", and land on it; and as someone whose window manager owns ⌘⇧P, I change Projects to ⌘⇧J and see the new key on the ? sheet and in ⌘K.
**Requirements**:
1. **Search** (deep/11 §4.15 "`/` filters settings"): a "Search settings" field heads Settings' sidebar; `/` (outside a text field) focuses it; Esc clears it, then leaves it.
2. It SHALL match pane names, section titles, setting labels and hints, and a few other words per setting ("mute" finds Notifications › Muted), from a static index kept beside `PANES` (`ui/src/screens/settings/panes.ts`), so no lazy pane is rendered to search.
3. Results SHALL list "Pane › Setting" lines (↑↓, ↩); choosing one opens the pane at the setting's anchor (the existing anchor mechanism, `panes.ts:60-86`, `ui/src/screens/settings/parts.tsx:16-40`) and focuses it; panes without a match dim in the sidebar. None: "No setting matches “…”."
4. A test SHALL render every pane and fail when an index anchor is missing or a section has no index entry, so the index can't drift.
5. **Shortcuts** (§4.15 "Shortcuts: §3 table, rebind + conflict detection, reset, hotkey toggle"): a new pane, **Keyboard**, SHALL list every shortcut from the registry by area (`SHORTCUT_AREAS`), each with its keys, Change… and, once changed, Reset; "Reset all" at the end, with Undo.
6. Shortcuts the registry matches (`match` present) SHALL be rebindable; the ones the platform or a control owns (Esc, ↩, ⇧F10, ⌃Tab) SHALL show as fixed with the reason.
7. Changing SHALL capture the next key combination (Esc cancels) and refuse, with the reason in words: one AI Juggler already uses where both can fire ("⌘⇧P is Projects. Change that one first."); one the system or a text field keeps (a per-OS list behind the platform seam: ⌘Q, ⌘W, ⌘H, ⌘M, ⌘Tab, ⌘Space, the edit keys on macOS; their equivalents elsewhere); a plain key for an Anywhere shortcut (plain keys belong to typing, `ui/src/lib/shortcuts.ts:205-224`). Home's row keys may stay plain letters.
8. Every place that shows or matches a key SHALL read the effective binding from the registry: `matches`, the ? sheet, ⌘K's key hints (`keysOf`, `ui/src/lib/palette-commands.ts:75-77`), tooltips and `aria-keyshortcuts`; no key text hard-coded in JSX.
9. Bindings SHALL be stored per Mac in AI Juggler's settings; a stored binding that a newer version makes clash falls back to the default and says so once.
10. The go-to key from any app SHALL show in Keyboard with the same capture and refusals as Settings › Desktop apps (`ui/src/screens/settings/DesktopAppsPane.tsx:135-202`, `ui/src/lib/hotkey.ts:56-67`), one shared capture component for both.
**UI/UX reasoning**: deep/11 §4.15 and §9 ("Settings tab list `tablist` vertical"); Apple's System Settings (a search field over the sidebar; Keyboard › Keyboard Shortcuts with per-item restore); Nielsen #7 (flexibility) and #5 (error prevention: conflicts refused before they bite); WCAG 2.1.4 (character key shortcuts can be turned off or remapped). States: search empty, results, none found, a result opened and focused; Keyboard default; a changed shortcut with Reset; capturing; refused (each reason); Reset all with its Undo toast; Windows and Linux key words (Ctrl for ⌘).
**Performance budget**: search over the whole index per keystroke ≤ 5 ms (≈ 200 entries); the Keyboard pane paints in ≤ 50 ms and loads lazily like the other panes; a changed binding applies at once, with no reload.
**Quality gates**: vitest `src/components/settings-screen.test.tsx` (extended), new `src/screens/settings/settings-search.test.tsx` (the drift test), new `src/lib/shortcut-bindings.test.ts` (conflicts per OS, fallback), `src/app-keys.test.tsx` (a rebound key fires, the old one doesn't); Playwright `keyboard-flows.spec.ts` (rebind, use, reset); `just ui`; the a11y gate; `scripts/size_budget.py`.
**Evidence/validation proof**: the tests above pass; the gallery families `settings-search` and `settings-keyboard`; manifest `docs/qa/evidence/AJ-E2905/manifest.md`; pass = 0 failures, 0 axe critical/serious, and the Playwright flow shows the new key on the ? sheet and in ⌘K after a rebind.
**Visual testing**: a new `ui/tests/visual/settings-gallery.spec.ts` (or the existing settings captures extended): search with results and with none, Keyboard default, capturing, each refusal, a changed row with Reset; 1024×700 and 1280×800, dark and light; axe on each.
**Files to touch**: `ui/src/screens/Settings.tsx`, `ui/src/screens/settings/panes.ts` (the index and the Keyboard pane), new `ui/src/screens/settings/KeyboardPane.tsx`, `SettingsSearch.tsx`, `settings-search.test.tsx`; `ui/src/screens/settings/parts.tsx` (anchors on rows), `DesktopAppsPane.tsx`, `pane-icons.tsx`; `ui/src/lib/shortcuts.ts`, new `ui/src/lib/shortcut-bindings.ts`, `ui/src/lib/app-keys.ts`, `ui/src/lib/hotkey.ts`, `ui/src/lib/palette-commands.ts`; `ui/src/components/ShortcutSheet.tsx`; the components that hard-code key text (found by grep for `aria-keyshortcuts` and "⌘" in JSX); the backend settings key and its reader in `crates/juggler-app/src/features/app/`.
**Research basis**: deep/11 §4.15, §3 ("Keys (also Settings → Shortcuts)"); audit §1.4 row "Settings › Shortcuts", §14 rows; `ui/src/screens/settings/panes.ts:36-50` (no Keyboard pane), `ui/src/screens/Settings.tsx:74-105` (no search); `ui/src/lib/shortcuts.ts:96-197` (the registry: one list for keys and the sheet), `:214-224` (`matches`); `ui/src/components/ShortcutSheet.tsx:18-21`; https://support.apple.com/guide/mac-help/keyboard-shortcuts-mchlp2262/mac; https://www.w3.org/TR/WCAG22/#character-key-shortcuts.
**Handover note**: The registry's `match` functions are code; a rebinding needs a data form of a key (modifiers plus the key's place, `code`, as `cmdAltCode` does, because ⌥ changes `key` on a Mac). Keep ⌘ and Ctrl as one "primary" modifier so a binding moves between OSes. A rebinding is never a hidden setting: the ? sheet must say "changed" beside it.

## Step 5 — first run

### AJ-E2906 First run shows its confidence: Found, Likely, Guessed, where each came from, and Look again at any time | Status: TODO | Priority P0 | Effort M | Depends: AJ-E2801
**Goal**: The scan's results say how sure AI Juggler is about each setup and why, so a guess is never presented as a fact, and the user can scan again whenever they like.
**User story**: As a new user with three Claude folders, I see "Work · Likely — from the folder's name ~/.claude-work" and "Signed in · Found — its sign-in is present", fix the name I disagree with after bringing it in, and press Look again after installing Codex.
**Requirements**:
1. Each found setup SHALL show one confidence word, Found, Likely or Guessed, for what the row claims (its name and its sign-in: the weaker of the two), and one line of where that came from in plain words (wf16's Source column): "the tool's usual folder", "set in your shell's settings", "a folder named like a Claude setup", "named by <the other app>".
2. ▸ Details on a row SHALL list each inferred field (name, sign-in, last used) with its own confidence and source, and the scan's notes (`DiscoveredInfo.notes`, shown verbatim today by contract).
3. The backend SHALL send which rule decided each field, as a closed set mapped to words in the UI, never an internal id on screen: `InferenceInfo` today sends only the confidence per field (`ui/src/types/generated/InferenceInfo.ts`), and `signals` is the folder's origin alone (`crates/juggler-app/src/commands.rs:14990`).
4. The words are words, never scores (AJ-E303 Req.3, `crates/juggler-adapters/src/inference.rs:10-41`); Guessed rows are not preselected unless today's rule picks them (signed in or used in 30 days, `ui/src/screens/FirstRun.tsx:136-140`), and say "You can rename it after".
5. **Look again** (wf16 "Rescan") SHALL show on the found step at all times, beside Where we looked (`FirstRun.tsx:590-600`), in first run and in Accounts › Add account (the same step, `ui/src/App.tsx:1023`); a rescan keeps the user's ticks on setups still found and adds new ones unticked unless preselected.
6. The Figurehead step stays as it is (D-46); nothing else in the three steps (D-25) changes.
**UI/UX reasoning**: wf16 ("inference is never silent"); deep/11 §4.16 ("scan results as radio group", protect the best-built surface); D-25 (calm first run, depth on request: one word and one line by default, Details for the rest); Nielsen #1 (visibility) and #2 (the user's words). States: all Found; a mix with a Guessed name; a Likely sign-in ("may have expired"); Details open; rescanning (the scanning skeleton); a rescan that finds one more; a failed scan (unchanged, "Try again").
**Performance budget**: no extra disk read for the sources (the rules run on the scan's own metadata, `inference.rs:144-160`); the found step's first paint unchanged (within 10 % of the load gate's first-run row).
**Quality gates**: `cargo test -p juggler-adapters --test inference` (extended: the rule per field); `cargo test -p juggler-app` for the scan DTO; vitest `src/screens/first-run.test.tsx` (extended: words, Details, Look again keeps ticks); the terminology lint (`just terminology`) for the new words; `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; a walk of first run on the replica (`just replica-walk`) with a Guessed and a Likely setup, captured; the gallery family `first-run`; manifest `docs/qa/evidence/AJ-E2906/manifest.md`; pass = 0 failures, 0 axe critical/serious, every row's source line in words.
**Visual testing**: `ui/tests/visual/journey-first-run.spec.ts` extended: found step with each confidence, Details open, after Look again; 1024×700 and 1280×800, dark and light.
**Files to touch**: `ui/src/screens/FirstRun.tsx` (split the found step if the size budget asks), `ui/src/screens/firstRunScan.ts`, new `ui/src/screens/first-run-confidence.ts` (words); `ui/src/types/generated/InferenceInfo.ts` (generated); `crates/juggler-adapters/src/inference.rs`, `crates/juggler-adapters/tests/inference.rs`; `crates/juggler-app/src/commands.rs` (`inference_info_of`, `:15005-15030`).
**Research basis**: wf16 (`docs/wireframes/index.html:419-434`: "Confidence is shown per item", "Rescan"); deep/11 §4.16; SPEC §12.1; D-25, D-46; audit top 15 #14 and §9 rows; `ui/src/types/generated/DiscoveredInfo.ts:44` (sent, never shown); `ui/src/screens/FirstRun.tsx:376-388` ("Try again" only after a failed scan).
**Handover note**: The origin values (`vendor-default`, `env-override`, `home-pattern`) and the rule tags are internal: map them in one place and let the terminology lint guard it. Open question: wf17 and SPEC §12.1 also promise optional renewal dates in first run (D-12); D-25's three steps dropped them without a recorded decision. Not in this task; say whether to record the drop or bring the step back.

## Step 6 — safe edges

### AJ-E2907 A confirm before deleting launch settings, and no delete while they're in use | Status: IN-PROGRESS (Req.1–3 shipped in 0.3.1 as a bug fix, D-60: the confirm, sessions kept in History instead of the cascade, refused while one is open; Req.4–5, the bin icon and the toast, remain) | Priority P0 | Effort S | Depends: —
**Goal**: Deleting launch settings asks first, says what goes with them, and is refused while a session uses them.
**User story**: As someone tidying an account, I click the bin beside "Deep work" and am told it has 14 past sessions and one running, instead of losing them in one click.
**Requirements**:
1. The delete button SHALL open a confirm (the shared `Modal`, `ui/src/components/form.tsx:63`): "Delete the launch settings “Deep work”?", what goes with them, Cancel (focused) and a red Delete; Esc cancels; focus returns to the button.
2. The confirm SHALL say what happens to the sessions started with them. Today deleting cascades to their session records (`profile_delete`, "cascades to its session records", `crates/juggler-app/src/commands.rs:2690-2707`; the foreign key `ON DELETE CASCADE`, `crates/juggler-core/src/storage/mod.rs:120-123`, kept through the renames at `:271-274`, foreign keys on at `:849`): until Q1 is ruled, it SHALL say "Its 14 past sessions leave History too."
3. While a session started with them is live, Delete SHALL be refused, in the UI and in the backend (no race): "1 live session uses these launch settings — stop it first." (deep/11 §10 "Delete profile blocked", in D-10's words).
4. The bin SHALL be the app's own delete icon (`ActionIcon` "delete"), not an inline SVG (`ui/src/screens/accounts/ProfileRow.tsx:209-211`), with its accessible name kept.
5. After a delete, a toast says "Deleted “Deep work”." (no Undo: the confirm is the guard, deep/11 §4.5 "Delete = ConfirmDialog").
**UI/UX reasoning**: deep/11 §4.5 ("Delete = ConfirmDialog, today one-click") and §1.1.5 ("destructive controls state what remains"); Nielsen #5 (error prevention); Apple HIG "Alerts" (name the action, the destructive button red, Cancel the default). States: confirm with no past sessions; with past sessions; refused while live; the backend's refusal if a session started meanwhile; the toast.
**Performance budget**: the confirm opens in one frame; counting its sessions is one indexed query ≤ 5 ms.
**Quality gates**: `cargo test -p juggler-app` (new: delete refused while live; the count); vitest `src/screens/accounts/accounts-grouped.test.tsx` or a new `src/screens/accounts/delete-launch-settings.test.tsx`; `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `accounts` with the confirm and the refusal; manifest `docs/qa/evidence/AJ-E2907/manifest.md`; pass = 0 failures, 0 axe critical/serious, a live session's launch settings undeletable from the UI and the IPC.
**Visual testing**: `ui/tests/visual/accounts-gallery.spec.ts` extended: the confirm (with and without past sessions), the refusal; 1024×700 and 1280×800, dark and light.
**Files to touch**: `ui/src/screens/accounts/ProfileRow.tsx`, `ui/src/screens/Profiles.tsx:212-215`, `ui/src/screens/accounts/AccountCard.tsx`; `ui/src/lib/bridge.ts` (`deleteProfile`); `crates/juggler-app/src/commands.rs` (`profile_delete`), `crates/juggler-core/src/storage/profiles.rs:111-117`.
**Research basis**: audit "Next after these" ("Deleting launch settings is one click with no confirm"), §10 row "Delete launch settings asks first", §4.2 row; deep/11 §4.5, §10; D-10, D-35.
**Handover note**: Check what happens when the deleted ones are the account's default launch settings (the row offers "default", `ProfileRow.tsx:5`) and say it in the confirm. Open question **Q1**: should deleting launch settings keep their past sessions in History (a schema change: the sessions keep a name instead of the foreign key) rather than delete them? deep/11 §1.1.5 ("source always survives") leans that way; it is a data-retention choice, so it is yours.

### AJ-E2908 An ended session's page: Resume, Archive, Replay and Copy ID | Status: TODO | Priority P1 | Effort M | Depends: —
**Goal**: The page of a session that ended offers what History's row offers and the spec promised: Resume the same conversation on the same account, Relaunch fresh, Replay, Archive, and copy its id.
**User story**: As someone who opened yesterday's session from Home, I press Resume and carry on the same conversation on the same account, or Replay it to read what happened, without going back to History.
**Requirements**:
1. The header of an ended session SHALL offer **Resume** (primary) and **Relaunch** (secondary, today's, `ui/src/pages/ItemHeader.tsx:184-188,279-291`), with the §10 tooltips: "Resume — same session, same account." and "Relaunch — fresh session, same launch settings and folder."
2. Resume SHALL continue the tool's own conversation on the same account and launch settings, in AI Juggler (its terminal, or its runtime per D-40), through the path a Switch Graph epoch already resumes by (`epoch_resume_impl`, `crates/juggler-app/src/commands.rs:13817-13860`); whether a tool can resume is the adapter's (ToolSpec), never a per-tool branch in the UI.
3. Resume SHALL be disabled with its reason when it can't run: no conversation ("It has no conversation to resume yet", the words of `copy-resume`, `ui/src/lib/action-specs.ts:96`); signed out ("Sign in to Work first"); out of room ("Work is out of room until 17:00", with Continue on another account… beside it, D-45); open in another app now (refused as Continue in AI Juggler is).
4. Its ⋯ menu SHALL add **Replay**, **Archive / Unarchive** (with Undo) and **Copy the conversation's id**, taken from History's actions (`ui/src/lib/action-specs.ts:121-143`), not copied: one registry (AJ-E1537).
5. Copy the conversation's id SHALL copy the tool's own id (shown today as plain text, `ui/src/components/RightInspector.tsx:476-481`) and say "Copied."
6. A live session's page SHALL NOT change.
**UI/UX reasoning**: deep/11 §4.7 ("Copy ID, Export pack…, Archive"), §10's Resume/Relaunch copy; D-05 (resume, never copy back); Nielsen #3 (user control) and #6 (the actions where the session is). States: ended with a conversation (Resume on); without (Resume off, with the reason); signed out; out of room; archived (Unarchive); a desktop app's ended session (what its tool supports); the menu open.
**Performance budget**: the header's actions resolve from cached state in ≤ 16 ms; Resume shows its starting state within 100 ms of the click.
**Quality gates**: `cargo test -p juggler-app` (a session-level resume over `epoch_resume_impl`, refusals); vitest `src/pages/session/header-doors.test.tsx` (extended), `src/components/palette-actions.test.tsx` (the registry); `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; a live check on the real setup with a spare Claude Code session in AI Juggler Dev (read-only accounts rule: no sign-in, no Allow), Resume continuing the same conversation; the gallery family `session-pages`; manifest `docs/qa/evidence/AJ-E2908/manifest.md`; pass = 0 failures, 0 axe critical/serious, the resumed session's conversation id equal to the ended one's.
**Visual testing**: `ui/tests/visual/session-pages-gallery.spec.ts` extended: each state above, the ⋯ menu open, 1024×700 and 1280×800, dark and light.
**Files to touch**: `ui/src/pages/ItemHeader.tsx`, `ui/src/components/InspectorHeaderMenu.tsx`, `ui/src/lib/action-specs.ts`, `ui/src/lib/actions.ts`, `ui/src/components/ManageWork.tsx` (the host's archive and replay), `ui/src/components/ReplayDialog.tsx`; `ui/src/lib/bridge.ts`; `crates/juggler-app/src/commands.rs` (a session resume beside `epoch_resume`), `crates/juggler-app/src/lib.rs`.
**Research basis**: audit "Next after these" ("An ended session's own page has no Resume, Archive, Replay or Copy ID"), §5 rows "Archive / Replay / Delete…", "Copy ID", "Resume vs Relaunch"; deep/11 §4.7, §10; `ui/src/pages/session/DrivenPanel.tsx:105-108` (Resume exists only for driven sessions); D-05, D-40, D-45.
**Handover note**: An ended terminal session is listed under the `session` kind, whose actions (`action-specs.ts`, SESSION) have no Archive or Replay; add History's actions for ended sessions rather than a second list. Delete… is History's too; it isn't in the lock-in's list, so it isn't added here.

### AJ-E2909 A keyboard-operable rail splitter | Status: TODO | Priority P0 | Effort S | Depends: —
**Goal**: The rail's width can be changed from the keyboard and read by a screen reader, as the drag does.
**User story**: As someone who doesn't use a pointer, I Tab to the edge of the list, press → four times to widen it, and End to make it as wide as it goes.
**Requirements**:
1. The handle SHALL be focusable (`tabIndex=0`, after the rail's list in the focus order, deep/11 §9: "Rail splitter after list"), `role="separator"`, `aria-orientation="vertical"`, `aria-valuenow` (the width), `aria-valuemin` 208, `aria-valuemax` 360 (`ui/src/components/WorkflowRail.tsx:73-75`), `aria-valuetext` ("264 pixels wide") and `aria-controls` naming the rail.
2. ← and → SHALL change the width by 8 px (deep/11 §2.5), ⇧← and ⇧→ by 32 px (as the pool canvas moves nodes, AJ-E2808 Req.7), Home to the narrowest and End to the widest.
3. A keyboard change SHALL be stored as a drag's is (`STORE_WIDTH`) and survive a restart.
4. The handle SHALL show the app's focus ring when focused and its accent line, as on hover.
5. The accessible name SHALL be "Resize the list" (the list's word, as ⌘B's "Show or hide the list"), not "Resize navigator".
6. The double-click density cycle stays; it has a keyboard twin already (⌃D).
**UI/UX reasoning**: deep/11 §2.5 ("Splitter keyboard-operable (←/→ ±8px, Home/End)"); WAI-ARIA APG window splitter pattern; WCAG 2.1.1 (keyboard, level A) and 2.5.7 (dragging movements have a non-drag alternative). States: resting, hover, focused, at the minimum, at the maximum, the rail folded (no handle, as today).
**Performance budget**: each key step repaints the rail in ≤ 16 ms with 50 sessions; one storage write per change, debounced at 200 ms.
**Quality gates**: vitest `src/components/rail-keyboard.test.tsx` (extended: keys, bounds, ARIA values, storage); `just ui`; the a11y gate on the rail.
**Evidence/validation proof**: the tests above pass; the gallery family `rail` with the handle focused; manifest `docs/qa/evidence/AJ-E2909/manifest.md`; pass = 0 failures, 0 axe critical/serious, a width set by keys restored after reload in the Playwright run.
**Visual testing**: `ui/tests/visual/rail-narrow.spec.ts` (or `places-gallery.spec.ts`) extended: the handle focused at the default, the minimum and the maximum; dark and light; 1024×700 and 1280×800.
**Files to touch**: `ui/src/components/WorkflowRail.tsx` (`:269-289`, `beginDrag` at `:155`), `ui/src/components/rail-keyboard.test.tsx`.
**Research basis**: audit §1.2 row "Splitter is keyboard-operable" (`role=separator` with no tabIndex or keys); deep/11 §2.5, §9; `ui/src/components/WorkflowRail.tsx:269-289`; https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/; https://www.w3.org/TR/WCAG22/#dragging-movements.
**Handover note**: A focusable separator needs `aria-valuenow`, or screen readers treat it as a static divider. The handle is 6 px wide; don't widen the visible line, but check its pointer target against WCAG 2.5.8 in the a11y gate.

### AJ-E2910 Home: filter chips, the r / h / s / e keys, and Mark all finished as reviewed | Status: TODO | Priority P1 | Effort M | Depends: AJ-E2801
**Goal**: On a busy morning, Home's Needs you can be narrowed to one kind, its rows can be handled from the keyboard, and the finished list cleared in one step.
**User story**: As someone with eight things waiting, I pick "Probably stuck · 2", deal with both with the keys, go back to All, and clear the seven finished sessions I've already read with one click.
**Requirements**:
1. Above Needs you, chips SHALL filter it: **All**, **Asks you n**, **Probably stuck n**, **Limits n**, **To review n**, **Sign-in n** (deep/11 §4.1's All · Needs reply · May be stuck · Quota · Review · Sign-in, in the app's words: `needWord`, `ui/src/lib/rail-items.ts:95-107`; D-36 "probably"). A chip shows only with a count above zero; the row of chips shows only when two or more kinds are present.
2. Mapping (`ui/src/lib/home-needs.ts:12`): Asks you = sessions that ask and pool asks; Probably stuck = stuck; Limits = limit and quota; To review = sessions done to review and tasks waiting on your review (D-44); Sign-in = signin. Failed, This Mac, heavy and left-running items show under All only.
3. The choice SHALL NOT be remembered across visits (a stale filter would hide a new need), and a filtered list SHALL say what it hides ("3 more under All"). The count in the rail, the bell and the badge never changes with a chip (D-49, one count).
4. Keys on the focused row (deep/11 §4.1 "`r` reply, `h` handover, `s` snooze, `e` archive"), registered in the shortcut registry (area Home) and listed on the ? sheet and Home's key line (`ui/src/screens/Home.tsx:506`): **r** opens its page at the answer (an ask) or its reply; **h** opens Continue on another account… for it (as ⌘⇧H, only with a conversation); **s** opens its Snooze choices (the row menu's, `ui/src/lib/action-specs.ts:57-85`); **e** archives an ended row with Undo. A key that doesn't apply to the row does nothing and the row's ⋯ says why.
5. "Pick up where you left off" SHALL offer **Mark all reviewed** with the number ("Mark all 7 reviewed", wf02's "Mark all finished as reviewed"), covering the whole list, not only the three shown (`ui/src/components/HomeRecent.tsx:96-106`), in one backend call and one Undo toast.
6. Chips are a single-choice group (the shared Segmented, `ui/src/components/controls/Segmented.tsx`), one tab stop, arrows between them.
**UI/UX reasoning**: deep/11 §4.1 (chips, keys); wf02 (morning triage in two minutes); D-25 (Home leads with "what needs me?"; Triage folded into Home, AJ-E1021); Nielsen #7 (accelerators) and #3 (Undo); WCAG 2.1.4 (single-key shortcuts only on the focused list, remappable through AJ-E2905). States: one kind (no chips); several kinds; a chip chosen with hidden items noted; the chosen chip's last item handled (back to All); Mark all with Undo; each key on a row it applies to and one it doesn't.
**Performance budget**: a chip change re-filters with 100 needs in ≤ 16 ms (no backend call); Mark all writes ≤ 200 sessions in one transaction ≤ 50 ms.
**Quality gates**: vitest `src/screens/home.test.tsx` (extended: chips, mapping, keys, Mark all and Undo), `src/app-keys.test.tsx` (the keys never fire while typing); `cargo test -p juggler-app` (the bulk mark and its undo); `just ui`; the a11y gate.
**Evidence/validation proof**: the tests above pass; the gallery family `home` with the chips and Mark all; manifest `docs/qa/evidence/AJ-E2910/manifest.md`; pass = 0 failures, 0 axe critical/serious, the rail's count unchanged by any chip in the Playwright run.
**Visual testing**: `ui/tests/visual/home-gallery.spec.ts` extended: chips (each chosen), the hidden-items line, Mark all and its toast; 1024×700, 1280×800 and 1440×900, dark and light.
**Files to touch**: `ui/src/screens/Home.tsx` (`:236-274` keys), `ui/src/components/HomeNeeds.tsx`, `HomeRecent.tsx`, new `ui/src/components/HomeChips.tsx`; `ui/src/lib/home-needs.ts`, `ui/src/lib/shortcuts.ts`; `ui/src/lib/bridge.ts` (`instanceMarkReviewed` gains a many-session form); `crates/juggler-app/src/commands.rs` (`instance_mark_reviewed`), `crates/juggler-app/src/lib.rs`.
**Research basis**: deep/11 §4.1; wf02 (`docs/wireframes/index.html:232-245`: "Mark all finished as reviewed"); audit §1.3 rows (chips "No decision drops them", keys r/h/s/e, Mark all); `ui/src/lib/shortcuts.ts:173-191` (Home's keys today: J/K/↑↓, Enter, A); D-25, D-36, D-44, D-49.
**Handover note**: The sources disagree on letters: wf02 says "S switch · P park · A archive", deep/11 §4.1 says r / h / s / e, and AJ-E1005 made A run the row's action. This task follows §4.1 and keeps A; say if you want wf02's letters instead. Plain-letter keys must stay on Home's list only and never fire in a field (`matches`, `ui/src/lib/shortcuts.ts:214-224`).

## Rulings (2026-10-30)

Design calls, made by the goal (the best daily experience, usable and
quick) as the maintainer asked; each can be overruled.

1. **AJ-E2902 Q1, a "finished task":** a session that ended on its own
   as finished. Every user has sessions; a pool task that reached Done
   (D-44) also shows its own cost where pools are used.
2. **AJ-E2902 Q2, where prices come from:** with app updates, shown with
   their date ("Prices as of …"). No Update button and no fetch in 0.4: a
   new outbound call needs its own ADR and a Privacy line first.
3. **AJ-E2902 Q3, the weekly review:** on demand, plus a Monday notice
   the user can turn on in Notifications (off by default).
4. **AJ-E2907 Q1, past sessions:** kept in History. Shipped in 0.3.1
   (D-60).
5. **AJ-E2904, the wait before a notification:** 5 minutes for new
   installs (deep/11 §4.15); an existing install keeps "Right away", as
   it chose by default. The hourly digest stays; no digest time.
6. **AJ-E2903, wf07's "Composite score: Unavailable" row:** left out
   (D-25: no row that says nothing). No table comparing accounts in 0.4;
   each account's page carries its own quality.
7. **AJ-E2906, renewal dates in first run:** the drop is recorded here:
   first run stays three steps (D-25); renewal dates are set on the
   account's page, where they already are.
8. **AJ-E2910, Home's letters:** deep/11 §4.1's r / h / s / e, under one
   rule: no key changes what it does today (A keeps running the row's
   action); a clash found while building is resolved toward today's key.
9. **AJ-E2901, Park until reset and settings in ⌘K:** settings yes,
   from AJ-E2905's search; Park until reset no (no park action exists;
   a 0.5 candidate).
