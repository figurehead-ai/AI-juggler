# Where the app breaks the design system today (2026-11-21)

Read on `release/0.5` at `d88090841`. Counts come from greps of `ui/src` (tests, preview and generated code left out). The screen-level readings lean on the 0.5 screen audit (`docs/research/screen-audit.md`), rechecked against today's code: since that audit, the 0.4.6 kit has landed (`FlowLayout`, `RadioCard`, `ControlRow`, `Section`, `Alert`, `ToolLogo`), but only the pool wizard and the Watches form use most of it.

**Status.** 0.5.0 fixes A1, A2, the Move or Copy dialog (A16 and `DESIGN-SYSTEM.md` §8) and B1's token, with check N1 (D-142). The rest of Part A and checks N2–N20 follow after 0.5.0 (AJ-E3618, AJ-E3619).

Part A ranks the gaps, worst first. Part B lists conflicts between existing rules. Each is put as a choice with my recommendation; some need the maintainer and some are design calls I've made and recorded.

---

## A. Gaps, worst first

### A1. The shared `Modal` overflows the window: the reported bug's root cause

**Breaks:** DLG-1, DLG-2, DLG-3, DLG-4, BTN-4.

**Where:** `components/form.tsx`, `Modal`.
- The overlay is `fixed inset-0 flex items-center justify-center p-6`. The dialog has no `max-height` (only the `table` variant has `max-h-full`), and its body (`data-modal-body`) doesn't scroll.
- A body taller than the window is centred, so it spills past both edges. The top part ends up above the window, where nothing can scroll to it. The buttons, written by each caller inside the body, end up below the bottom.
- This is exactly what was reported: "choices scrolled up under the top edge while the buttons could scroll away". At the 1024×680 minimum, any sheet over about 630 px does it.
- There is no fixed header or footer: the title row scrolls with everything else, and footers are hand-made per caller.
- The title is `text-sm font-semibold` (14 px) and the subtitle `text-xs`, both off the type roles.

**Reach:** 30 callers. These can grow tall: `components/AccountSettingsSheet.tsx` (Copy settings, `wide`, a wall of steps and a code block; the audit saw it "clip its own button"), `screens/accounts/ServiceAccountSheet.tsx`, `components/BudgetCaps.tsx`, `components/ReportBugSheet.tsx`, `components/ProjectSyncSheet.tsx`, `components/PoolSheet.tsx`, `components/HandoverView.tsx`, `screens/accounts/LoginModal.tsx`, `components/FolderTrustAsk.tsx`, `components/QuitSheet.tsx`, `screens/processes/GroupCleanupSheet.tsx`, `pages/pool/ScopeSheet.tsx`, `components/usage/WastedSheet.tsx`.

**Fix:** build `components/DialogFrame.tsx` from `switch/SwitchDialogFrame.tsx`'s structure, which is right: `max-h-[min(92vh,760px)]`, `flex-col overflow-hidden`, a `shrink-0` header, a `min-h-0 flex-1 overflow-y-auto` body and a `shrink-0` footer. Add header, footer and body slots, the size classes and `.scroll-edge`. Then make `Modal` a thin wrapper over it, with a `footer` prop, so all 30 callers get the fixed footer. N1 and N2 hold it.

### A2. 35 hand-made dialog overlays, each with its own frame

**Breaks:** PR-11, DLG-1, DLG-2, DLG-7, BTN-7.

**Where:** files with their own `fixed inset-0` overlay.
- **No height cap at all** (overflow risk grows with content): `screens/FleetScreen.tsx`, `screens/settings/ai/ModelsApproval.tsx`, `components/RecoverySummaryDialog.tsx`, `components/UsageStatsReconsent.tsx`, `components/RemoteCleanup.tsx`, `components/IdentityMismatchDialog.tsx`, `components/FigureheadCloudParts.tsx`, `components/DeleteInstanceDialog.tsx`, `components/IsolationOfferDialog.tsx`, `components/AccountMacsCard.tsx`, `components/RestartApp.tsx`, `components/WorktreeGit.tsx`, `components/FleetPromptDialog.tsx`, `components/RecoveryCodeSheet.tsx`, `components/bot/BotSheets.tsx`, `components/ReplayDialog.tsx`, `components/TerminalView.tsx`, `screens/FirstRun.tsx`.
- **A cap but their own structure:** `screens/AddProviderSheet.tsx`, `screens/NewSession.tsx`, `screens/NewProjectSheet.tsx`, `screens/PairingWizard.tsx`, `screens/WorkPanels.tsx`, `screens/pools/NewPoolFrame.tsx`, `components/LimitDialog.tsx`, `components/KillSheet.tsx`, `components/BackupRestoreCard.tsx`, `components/CrashReports.tsx`, `components/GoToPalette.tsx`, `components/CrossMachineSwitchBack.tsx`, `components/RemoteLaunch.tsx`, `components/SwitchBackFlow.tsx`, `components/ContinueDialog.tsx`, `components/StorageRetentionCard.tsx`.

**Each re-implements** its sizes, focus trap (`keepTabInside`, `useDialogFocus` or `useDialogKeys`), close button and button classes. `switch/SwitchDialogFrame.tsx` exports its own `BUTTON`, `SECONDARY` and `PRIMARY` classes. `primariesIn()` can't see them, so the one-primary check misses every switch dialog.

**Fix:** move each to `DialogFrame` (A1), the window-sheet ones to `FlowLayout`. N2's allowlist starts with these 35 and shrinks.

### A3. Most making and editing screens are still bare

**Breaks:** D-126 §1, MD dials, TGL-2, RC-1, RC-2, PR-3.

**Kit adoption** (files using each, outside `controls/`):

| Component | Files |
|---|---|
| `ToggleRow` | 3 |
| `CheckRow` | 0 |
| `RadioCard` | 4 |
| `CheckCard` | 3 |
| `FlowLayout` | 5 |

All of them are the pool wizard's files plus `watches/WatchForm.tsx`. By the audit, 37 of 43 rich or mixed screens use none of the kit. Worst first (AUDIT §2, still true in code):
1. **Settings › Notifications** (`components/NotificationSettings.tsx`): plain `Checkbox` lists and five `Picker`s in one flat list; no icons; no "needs you" vs "just so you know" grouping.
2. **Add an app** (`screens/AddProviderSheet.tsx`): letter tiles where logos belong; the same button on every row.
3. **Add a fresh app** (`screens/accounts/*`, `G/71-add-account`): a name box and one checkbox in a narrow column.
4. **The switch wizard** (`components/SwitchFlow.tsx`, `components/switch/*`): paragraphs, grey chips, amber boxes for plain facts. `MoveBody` is bold-label bullets over muted 12 px lists. See the worked example, `DESIGN-SYSTEM.md` §8.
5. **First run: computer access and found apps** (`screens/FirstRun.tsx`, `FirstRunSteps.tsx`): one small card on a blank screen; bare checkboxes with dots instead of logos; "Recommended" in muted 12 px.
6. **Copy settings into another account** (`components/AccountSettingsSheet.tsx`).
7. **Account settings and renewal**.
8. **Monthly caps sheet** (`components/BudgetCaps.tsx`).
9. **Privacy and stats** (`components/PrivacyCard.tsx`, `UsageStatsChooser.tsx`).
10. **Settings › Advanced and General** (`screens/settings/*`).

**Fix:** follow the audit's order. The type scale and `Section` are already done; next come `RadioCard` and `CheckRow` everywhere, then `FlowLayout` and summary panels. Re-shoot each screen at 1024×680 and 1280×800 in both themes, with N6 on.

### A4. Body text still in `muted`: the "monochrome" complaint, unenforced

**Breaks:** COL-2, COL-3, PR-3.

**Counts:** `text-muted` has 1,333 uses and `text-ink-2` has 583. PRINCIPLES §7 measured 889 against 809 for `text-ink` on 2026-10-30. The token fix landed, but sentences keep being written in `muted`.

**Examples:**
- `switch/MoveChoice.tsx`: the notes, members and outputs lists use `text-label text-muted`.
- `components/form.tsx`: `Field`'s hint is `text-label text-muted`, and its placeholder is `muted` where it should be `faint`.
- The audit's "five-line grey paragraph" on Account settings.

**Fix:** N5.2 (paragraphs longer than two lines in muted fail) with a shrinking list, plus a codemod pass per screen as it is redone.

### A5. Text off the type scale, and under 12 px

**Breaks:** TYP-1, TYP-2.

**Under 12 px (7 places):**
- `screens/settings/ai/ModelParts.tsx` lines 26 and 66 (`text-[11px]`, `text-[11.5px]`);
- `screens/settings/ai/ModelsJobs.tsx` line 108 (`text-[11px]`);
- `pages/pool/setup/TalkTable.tsx` lines 28 and 38 (`text-[11px]`, aria-hidden glyphs, so allowed);
- `components/HomeFeedCard.tsx` line 52 and `components/NeedCard.tsx` line 41 (`text-[10px]` initials in a 20 px aria-hidden disc: decorative, but the letters are what a sighted user reads, so make them 12 px or use the App icon).

**Off the roles:**
- 27 arbitrary `text-[…]` sizes;
- 326 `text-xs` and 120 `text-sm`, which bypass the roles. `Button` sizes use them, and so do `Picker` sizes and `Modal`'s title.

**Fix:** N3. Map `text-xs` to `text-label` and `text-sm` to `text-body` in `primitives.tsx` and `Picker.tsx` first; that alone changes hundreds of call sites. The new Models screen is the newest offender.

### A6. Tags drawn three ways; "Recommended" drawn three ways

**Breaks:** TAG-5, RC-3, PR-11.

- **Two tag components:** `Chip` (`components/primitives.tsx`, 33 files) and `Pill` (`components/Pill.tsx`, 12 files) do the same job with different tone maps and paddings.
- **Recommended, three ways:**
  - `switch/MoveChoice.tsx` line 224: a success-toned pill. Success is L1 "done", so the tone means the wrong thing.
  - `screens/FirstRun.tsx` line 466: muted `text-xs` words.
  - `screens/settings/ai/ModelsRecommended.tsx`: a card.
- **Step marks:** `switch/SwitchDialogFrame.tsx` `StepMarks` draws done steps in the success tint. The kit draws done steps as accent-soft.

**Fix:** one `Tag` with tones neutral, accent, L1, L2 and L3, and "Recommended" as the accent tag. N12 holds it.

### A7. `Card` draws every card bordered, shadowed and sheened

**Breaks:** CRD-3, ELV-2, D-117 round 2.

**Where:** `components/primitives.tsx` `Card`: `sheen rounded-card border border-hairline bg-surface shadow-card … hover:border-muted/25`. 63 files use it.

**Why it matters:** round 2 removed card borders on calm screens ("the surface tone separates"), and a hover border on a non-interactive card suggests it can be clicked. Other decoration still in `styles.css`:
- `.numeral`, a violet gradient on numbers with a raw `#9d9dfb`;
- `--shadow-glow-live`, still the old cyan `#22d3ee59`.

**Fix:** a mode prop, or two components: `Area` (borderless, for calm screens) and `Card` (rich, raised). Drop the hover border except on clickable cards. Retire `.numeral` and `.sheen` from new work.

### A8. Menus without icons; `MenuItem` has no icon slot

**Breaks:** ICN-4, MNU-1, D-57.

**Where:** `components/RowMenu.tsx` `MenuItem` takes `children` only. 34 of 82 menu items have no icon:

| File | Items without an icon |
|---|---|
| `screens/processes/RowActions.tsx` | 12 |
| `components/PlaceHeader.tsx` | 7 |
| `components/RailIdleApp.tsx` | 5 |
| `screens/accounts/AppsSection.tsx` | 3 |
| `components/HomeFeedFilters.tsx` | 3 |
| `components/ViewPopover.tsx` | 2 |

Some are choice lines with ✓, which is allowed.

**Fix:** an `icon` prop on `MenuItem`, required by type (N8).

### A9. Missing tokens: radii, shadows, motion, z-index, dialog sizes, icon sizes

**Breaks:** RAD-1, ELV-1, MOT-1, COL-1.

- **Radius:** one token (`--radius-card`, 14 px), but in use: `rounded-lg` 398, `rounded-full` 218, `rounded-xl` 156, `rounded-md` 133, `rounded-card` 67, `rounded-[10px]` 28, `rounded-2xl` 24, `rounded-[14px]` 17, `rounded-[9px]` 8, `rounded-sm` 7.
- **Shadows:** the dialog shadow is a raw value in `form.tsx`, and there is no popover shadow token.
- **Motion:** durations are hand-typed (120, 150, 600, 700 ms).
- **Layers:** dialogs and toasts are both `z-50`.
- **Sizes:** no dialog size classes (`max-w-md`, `2xl`, `3xl`, `5xl` and `680px` all appear).
- **Raw hex in components:** 14 places. Most are the terminal palette (allowed); the others are `ReplayDialog.tsx`, `RightInspector.tsx` and `WorkPanels.tsx`.

**Fix:** add `--radius-control`, `--radius-row`, `--shadow-popover`, `--shadow-dialog`, `--duration-fast/base/slow` and z-index classes. Then run N3.

### A10. Serious choices explained only in the switch

**Breaks:** PAT-SER-1, PAT-SER-6.

**Where:** only `switch/Explain.tsx` gives "What does this mean?" plus "Learn more in the guide". "Learn more" appears 3 times in the whole UI. The other serious choices have no consequence line, example or guide link:
- Privacy and stats;
- Processes' automatic cleanup tiers;
- Settings › Agent access (access levels);
- the keys question;
- Delete with its folder;
- Copy settings.

**Fix:** promote `Explain` to `components/Explain.tsx` and add the serious-choices registry (N11).

### A11. Disabled without a reason; PRINCIPLES still says "show greyed"

**Breaks:** BTN-5, PAT-HIDE-2, D-127.

- `switch/SwitchDialogFrame.tsx` `ContinueButton` is `disabled={!ready}` with no reason. `FlowFoot`'s `reason` line is the model to follow.
- `controls/Picker.tsx` and `ChoiceOption` support `disabled` with an optional `reason`. Nothing forces the reason, or forces the option to be one the user can clear.
- `docs/design/PRINCIPLES.md` §10 still reads "Unavailable options stay visible, greyed, with their reason", which contradicts D-127 (B2).

**Fix:** N9 (types plus lint) and amend PRINCIPLES §10.

### A12. ✕ means both "close" and "broken"

**Breaks:** ICN-1, ICN-2.

**Where:** `components/Alert.tsx` gives level `broken` the `IconClose` icon. The same × closes the Alert, the dialogs and the toasts. The L3 level mark (`styles.css` `.level-mark[data-level="3"]`) is also an ✕.

**Fix:** a distinct L3 glyph (⊘, as CALM §b allows). Keep × for close in neutral chrome only. Keep the small red ✕ inside the level mark only beside its word. N16 holds it.

### A13. No shared empty state

**Breaks:** EMP-1, PR-11, DEEP §2.3.

**Where:** each screen makes its own: `related/RelatedEmpty.tsx`, `watches/WatchesEmpty.tsx`, `bot/BotEmpty.tsx`, `FleetPeersEmpty.tsx`, and the Home first-session card. Inviting empties (rich) and calm one-liners are not distinguished.

**Fix:** `components/EmptyState.tsx` with `rich` and `calm` forms, marked `data-empty-state` for N14.

### A14. One-primary checked in only 6 views

**Breaks:** BTN-1.

**Where:** `attention-budget.spec.ts` checks the header, the rail and Home. `primariesIn()` is asserted in 6 component tests. Primaries drawn with `PRIMARY_FILL` (the switch dialogs) aren't marked `data-primary` at all.

**Fix:** N4.

### A15. Stale doc comments that teach the old rules

**Breaks:** D-87, D-117 (by example).

- `components/Banner.tsx` header: "everything the window says above a screen: an account almost out of room …". D-87 removed page-top banners for account and session notices.
- `docs/review/deep/11-ux-redesign-screen-spec.md` §2.1: proposes 11 px text and "faint banned < 11.5 px". The readability scale replaced this with "nothing under 12 px".

**Fix:** a docs pass. An agent reading these builds the wrong thing (M:read-specs-and-wireframes cuts both ways).

### A16. The words on the switch's Move choice

**Breaks:** CNT-2.

**Where:** `switch/MoveChoice.tsx` labels the choice "Move to account". The destination is an App (a Claude app profile) that belongs to an Account; D-35 says a profile is not an account.

**Fix:** "Move" with the destination named: the title "Continue on Claude app · Personal" and the button "Move to Personal", as in the worked example.

---

## B. Conflicts between existing rules

**Decided 2026-11-21 in D-142**, each as recommended: B1 choice A, B2 (D-127 wins, PRINCIPLES §10 amended), B3 choice A, B4, B5. The C calls stand.

### Asked of the maintainer (now decided, D-142)

**B1. "Muted text ≥ 7:1" against the light theme's `muted`.**

- **The rules:**
  - The decided scale (M:readability-and-visuals, 2026-10-25) says muted text must reach 7:1.
  - The tokens adopted from PRINCIPLES §7 give light `muted` `#4f5767`, which is 5.87:1 on canvas-hi, 6.26:1 on canvas, 5.97:1 on surface-3 and 7.26:1 on surface. Dark `muted` passes, at 7.28:1 or more everywhere.
- **Choice A:** darken light `muted` to about `#424958`. That gives 7.29:1 on canvas-hi, 7.78:1 on canvas, 7.42:1 on surface-3 and 9.03:1 on surface. `ink-2` stays a clear step above (10.2–12.7:1).
- **Choice B:** amend the rule to "AA everywhere, APCA Lc 60 or more", which today's tokens meet.
- **Recommendation: A.** Readability is the maintainer's most repeated complaint. The change is one token, and `ink-2` stays distinct. Raise `contrast.spec.ts` to 7:1 for `muted`.
- **What it decides:** whether light-theme labels get darker on every screen.

**B2. Unavailable options: "show greyed with a reason" (PRINCIPLES §10) against "hide what the setup can't use" (D-127).**

- **Recommendation:** D-127 wins. It is later and a direct ruling. Amend PRINCIPLES §10 to say: greyed only when the user can clear it, saying how; otherwise hidden.
- **What it decides:** whether the written principles agents are critiqued against stop teaching the old rule.
- Precedence answers this one already. It is listed so that the amendment is recorded as a D-number.

**B3. Group labels in tracked capitals (D-117) against "no uppercase grey section titles" (M:readability-and-visuals).**

- **The rules:**
  - D-117 approved 12 px tracked capitals in `--color-section-label` for group labels (Needs you, Your apps).
  - The readability rule, written after "11 px uppercase grey section titles" were found hard to read, asks for 15 px sentence-case section titles in ink.
- **Choice A:** both, with clear roles. Capitals only label a group of rows or steps, and only at 12 px in `section-label`, which is brighter than `muted`. Every content section that holds choices, on every screen, has a sentence-case `.type-section` heading. Capitals are never the only heading of a form section.
- **Choice B:** sentence case everywhere, dropping the capitals.
- **Recommendation: A.** It is what the approved calm and kit mock-ups draw. It is written as TYP-7.
- **What it decides:** whether the rail, Home and wizard group labels keep their approved look.

**B4. Are decision dialogs "rich"?**

- D-126 lists wizards, create and edit forms, setup, onboarding, inviting empty states and settings. It doesn't name decision dialogs (Switch, Hit its limit, identity mismatch) or destructive confirms. The audit classed them rich.
- **Recommendation:**
  - Decision dialogs are rich: icons, logos, choice cards, a summary panel.
  - Destructive confirms are "rich-compact": the icon, the consequence panel and tags, at confirm size, with no illustration.
- **What it decides:** whether about 20 dialogs get the rich treatment in 0.5.

**B5. Which window sizes count.**

- **The rules:**
  - D-57 and QUAL's critique line: 1280×800 and 1024×700.
  - The visual gate (AJ-E803) and the real window minimum (`tauri.conf.json`): 1024×680.
  - M:readability-and-visuals: judge at 1280, 1680 and 2000 px wide.
- **Recommendation:**
  - Gates and critiques: 1024×680 and 1280×800, both themes. The 1024×700 in D-57 becomes 680, the real minimum.
  - Monitoring dashboards also get the Opus review at 1680 and 2000 px wide.
- **What it decides:** whether every dialog is proven at the smallest window people can actually make.

### Design calls I've made (recorded here, not asked)

These follow "decide by the goal". The maintainer can overrule any of them.

- **C1. Section heading size stays 15 px.** PRINCIPLES §7 and §8 proposed 16 px. The tokens, the D-126 kit and the readability scale all use 15. PRINCIPLES should be amended to match.
- **C2. Secondary buttons by mode.**
  - Ghost on calm screens (CALM round 2).
  - Bordered secondary beside the primary on rich screens (the kit's "Add, sign in later", `FlowFoot`).
  - Cancel is ghost everywhere.
- **C3. Selection by mode.**
  - Neutral `SELECTED` for segments, tabs, rows and chips everywhere (D-117).
  - The accent ring `CHOSEN_CARD` only on rich choice cards (D-126 "accent for selection" on making screens).
  - Both already exist in `controls/selected.ts`; this writes down which to use where.
- **C4. Disabled Next.**
  - Primer and Atlassian say never disable. Apple and Carbon allow it.
  - The app keeps a waiting primary on choice steps, but always with its reason visible in the footer (`FlowFoot`).
  - Steps with typed input keep Next enabled and explain the problem on press (WIZ-7, BTN-5).
- **C5. Button order and case.**
  - Order: Cancel, then the primary, at the trailing edge (macOS), in every dialog, sheet and flow.
  - Case: sentence case for buttons, not Apple's title case, because the app already uses it everywhere (CNT-3).
- **C6. Icons stay in-house, not SF Symbols.** The UI is a web view and must ship on Windows and Linux (D-52). The set is drawn to sit beside SF Symbols' regular weight (ICN).
- **C7. Body text stays 14 px, not macOS's 13 pt.** This is the maintainer's decided scale; Apple's 13 pt is for native controls.
- **C8. Type-to-confirm only for bulk, unrecoverable deletes.** No external system mandates it; undo is preferred everywhere else (PAT-DES-5).
- **C9. Mode per region on mixed screens.** The editing panel is rich, and the monitoring surface around it stays calm (MD, AUDIT "mixed").
