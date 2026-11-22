# Screen audit for the 0.5 tree (AJ-E3618, D-117 and D-126)

A Sonnet survey and draft; Opus reviews it. Read-only: nothing in the app was changed.

**What was done.** Every folder under `ui/tests/visual/__baselines__/linux/gallery/` (125 folders in this tree) was matched to a screen or dialog, and the captures at the top of each ranking were opened by eye. Some captures still show an older rail and top bar; that was ignored on purpose. The earlier 0.4.6 audit (`docs/qa/evidence/AJ-E3618/screen-audit-d126.md`, 142 folders) was used as a cross-check. Where this file repeats one of its readings without a fresh look, the line says so.

**Path shorthand.** `G/` means `ui/tests/visual/__baselines__/linux/gallery/`.

**The two classes (D-117, D-126).**
- **Calm** (monitoring): scarce status colour, nothing competes for attention. Rich styling is not required here.
- **Rich** (making and editing: wizards, forms, setup, settings, decision dialogs): the accent for choice, progress and preview, tinted surfaces, logos on every choice, choice cards with previews, a live summary. Calm never means plain.
- **Mixed** is a monitoring screen with one editing panel inside it. It is counted on its own line.

## 0. Where the kit stands in this tree

The 0.4.6 pool-wizard kit has **not landed in `release/0.5` yet**. A search of `ui/src` finds no FlowSteps, FlowLayout, RadioCard, ToolLogo, CheckRow, ToggleRow or shared Section component. What exists today:

- `ui/src/components/controls/`: Segmented, Checkbox/Radio, ChoiceLine/ChoiceOption, Picker, DateField, Disclosure, and `SELECTED` (the one look for "chosen"). This is the partial base.
- A numbered stepper only in first run (`ui/src/screens/FirstRunSteps.tsx`) and the old switch pills.
- `NewPool*.tsx` is the pre-kit wizard in this tree; it takes the kit only when 0.4.6 merges.

So the column "Kit?" below reads **None** for every screen. "Partial" means it already uses the controls base (`ChoiceOption`, `Segmented`, `SELECTED`, a stepper) but none of the kit's cards, preview or summary. "n/a" is a calm screen.

## 1. Every screen and dialog

Class: C = calm, R = rich, C+R = mixed. Kit?: n/a, None (bare form or bare list of controls), Partial.

| # | Screen or dialog | Gallery folders (G/) | Class | Kit? |
|---|---|---|---|---|
| 1 | Home: Needs you, feed, Your apps, Runway | 67-home, 68-home-feed, 55-triage, 54-summary, 93-messages, 94-places, 99-home-chips | C | n/a |
| 2 | Home empty, all-clear, first session | 55-triage, 67-home | C+R | None |
| 3 | Rail, folded rail, splitter, projects view | 01-rail, 01-rail-density, 01-rail-sort, 69-rail-narrow, 74-rail-projects, 99-rail-splitter, 94-places, 80-marks | C | n/a |
| 4 | Rail view menu (show / only filters) | 97-controls | C+R | Partial |
| 5 | Notification centre, bell, banners | 96-notification-centre, 23-alerts, 17-ledger | C | n/a |
| 6 | Command palette and Go to | 99-palette, 14-focus, 29-door | C | n/a |
| 7 | Session page, Overview and inspector | 05-session-panel, 06-inspector, 07-inspector-content, 12-model, 51-identity, 52-worktree, 53-parked, 79-pages, 82-terminal-tools, 92-session-pages, 99-resume | C | n/a |
| 8 | Session Usage and Quality tabs | 09-quality, 26-quota, 25-costs | C | n/a |
| 9 | Session Processes tab and recovery summary | 22-recovery, 18-ports | C | n/a |
| 10 | Subagents tab | 88-subagents | C | n/a |
| 11 | Desktop-app window preview and focus outcomes | 13-desktop, 15-preview, 47-remote-focus | C | n/a |
| 12 | Account page (Overview, Apps, Sessions) | 08-inspector-aggregate, 99-account-quality, 99-account-page | C | n/a |
| 13 | Account Settings card: plan, renewal, tidy-up | 07-inspector-content, 11-renewal | R | None |
| 14 | Accounts page: cards, usage, apps | 68-accounts, 93-accounts-profiles, 78-outside, 94-usage | C | n/a |
| 15 | Launch settings and keep-alive editor | 83-keep-going | R | None |
| 16 | App page and "find its other profiles" | 72-tool-view, 99-app-page | C | n/a |
| 17 | Usage page and burn-down | 87-burn-down, 99-usage-cost, 94-usage | C | n/a |
| 18 | Monthly cap sheet | 89-budget-caps | R | None |
| 19 | Processes page: table, counts, ports | 90-processes, 18-ports, 21-tiers | C | n/a |
| 20 | Processes: Automatic cleanup toggles | 21-tiers, 23-alerts | R | None |
| 21 | Stop, clean-up and group-clean confirmations | 20-kill, 48-remote-cleanup | R | None |
| 22 | This Mac, memory pressure, keep awake | 72-this-mac, 81-this-mac, 74-overload, 83-keep-going | C | n/a |
| 23 | History, archived, replay, delete | 56-history, 75-feedback | C | n/a |
| 24 | Fleet page: machines, board, sync | 42-fleet, 24-fleet, 46-fleet-board, 45-remote, 49-fleet-sync | C | n/a |
| 25 | Add a machine (pairing wizard) and its failures | 43-pairing, 44-settings | R | None |
| 26 | Remote launch sheet and access levels | 88-fleet-launch, 46-fleet-board | R | None |
| 27 | Compare an account's keys across Macs | 61-account-macs | R | None |
| 28 | Settings: General | 97-controls, 99-settings-search, 58-updates | R | Partial |
| 29 | Settings: Notifications and When to tell me | 84-notifications, 99-when-to-tell | R | None |
| 30 | Settings: Privacy and usage stats choice | 62-privacy, 63-consent, 84-privacy | R | Partial |
| 31 | Settings: Accounts (renewals, tools on this computer) | 65-settings-sections | R | None |
| 32 | Settings: Sessions | 65-settings-sections | R | None |
| 33 | Settings: Switching | 96-switch-setup, 99-carry-choice | R | None |
| 34 | Settings: Desktop apps | 65-settings-sections | R | None |
| 35 | Settings: Agent access | 92-agent-tools | R | None |
| 36 | Settings: Keyboard and key capture | 99-settings-keyboard, 76-keys | R | None |
| 37 | Settings: Advanced (backup, cache, diagnostics, debug) | 57-backup, 98-local-cache, 85-diagnostics, 99-debug-mode, 60-cloud-backup | R | None |
| 38 | Settings: The Floor | 99-floor-settings | R | None |
| 39 | Settings: Models (AJ-E3812, new) | none yet | R | None (to build) |
| 40 | The Floor: list, menu, 3D office | 99-floor-list, 99-floor-menu | C | n/a |
| 41 | Pool wizard and its review | pre-kit code in `ui/src/screens/pools/`; no baseline in this tree | R | None (kit arrives with 0.4.6) |
| 42 | Pool canvas: diagram and Needs you | 97-pool-canvas | C | n/a |
| 43 | Pool canvas inspector: who leads, roles, on their own | 97-pool-canvas, 92-agent-tools | R | None |
| 44 | Pools list, Move to pool, pool sheets | 91-pools | C+R | None |
| 45 | Tasks board | 95-tasks | C | n/a |
| 46 | Switch wizard (target, what is carried, confirm) | 28-switch, 30-switchback, 31-redaction, 32-graph, 33-beta, 50-cross-switch | R | None |
| 47 | Switch set-up offers and handover told | 96-switch-setup, 97-handover-told | R | None |
| 48 | "Hit its limit" decision dialog | 89-limit-ready | R | None |
| 49 | New session dialog and launch advisor | 73-launch-advisor, 86-open-pack | R | None |
| 50 | Add an app chooser | 38-flows-cde, 91-more-tools | R | None |
| 51 | Add a fresh app | 71-add-account | R | Partial (two-step stepper only) |
| 52 | Add an API-key account (service account) | 94-service-accounts | R | None |
| 53 | First run: bring in your apps, found list, where we looked | 37-flows, 39-import, 66-first-run, 99-first-run-confidence | R | Partial (stepper) |
| 54 | First run: computer access | 66-first-run | R | Partial (stepper) |
| 55 | Log in dialog | 40-login | R | None |
| 56 | Copy settings into another account | 83-account-settings | R | None |
| 57 | Delete, forget, pack-secrets confirmations | 41-policy, 76-project-manage | R | None |
| 58 | Crash report and stats re-ask | 63-consent, 64-crash | R | None |
| 59 | Cloud recovery code and vault | 59-vault, 60-cloud-backup | R | None |
| 60 | Report a bug | 99-report-bug | R | None |
| 61 | New project sheet and project sync sheet | 82-project-sync, 76-project-manage | R | None |
| 62 | Identity mismatch, isolation offer, worktree offers | 51-identity, 52-worktree | R | None |
| 63 | Update available and release notes | 58-updates, 33-beta | C+R | None |

**Counts: 63 screens and dialogs. Calm 20. Rich 39 (one of them, Models, is still to build). Mixed 4 (rows 2, 4, 44, 63). Using the full kit: 0. Partial: 6 (rows 4, 28, 30, 51, 53, 54). Bare (None): 37 of the 43 rich or mixed rows.**

## 2. The worst 25 making-and-editing screens

Ranked, worst first. "Opened" means this pass opened the capture; "earlier draft" means the reading comes from the 0.4.6 audit, rechecked against the code (controls, logos and steps in the source), not re-opened.

| Rank | Screen | Capture | Why it ranks here |
|---|---|---|---|
| 1 | Settings: Notifications | `G/84-notifications/settings-chosen-1280x800-dark.png` (opened) | Nine plain checkboxes in one flat list, no icons, no grouping into "needs you" and "just so you know", section titles the same size as the rows, and the second block ("When to tell me") is plain selects that run off the page. |
| 2 | Add an app chooser | `G/38-flows-cde/c-provider-cards-1280x800-dark.png` (opened) | A letter in a box instead of each tool's logo, and the same blue button on every row, so nothing is the clear choice. No preview of what you get. |
| 3 | Add a fresh app | `G/71-add-account/fresh-1280x800-dark.png` (opened) | A name box and one checkbox in a narrow column on an empty page. No tool logo, no preview of the app being created, no summary of what it copies; the bold intro paragraph is as heavy as the headline. |
| 4 | Switch wizard | `G/28-switch/dialog-finish-1280x800-dark.png` (opened) | The most consequential flow is paragraphs, grey chips and amber boxes. It shows no "from here to there" picture of what carries over. Pills for the steps are tiny; radio rows for "when to stop the original" are plain; the Confirm step needs a scroll to see its own stop-the-original choice. |
| 5 | First run: computer access | `G/66-first-run/access-1280x800-dark.png` (opened) | One small card on a blank screen. No picture of what the permission does or what you lose by skipping. "Skip" and "Continue" sit far apart with equal pull. |
| 6 | First run: bring in your apps | `G/66-first-run/found-1280x800-dark.png`, `G/99-first-run-confidence/found-1280x800-dark.png` (earlier draft) | A table with bare checkboxes and small coloured dots where each found app should show its logo; half the screen is legal footnotes. |
| 7 | Copy settings into another account | `G/83-account-settings/collapsed-dark.png` (earlier draft) | A wall of numbered terminal steps and a code block in a plain sheet; bare checkboxes; the sheet clips its own button. |
| 8 | Account Settings card and renewal | `G/11-renewal/editor-1440x900.png` (earlier draft) | Native checkboxes and the system's own date box; the safety facts for "Tidying up" are buried in a five-line grey paragraph. |
| 9 | Pool wizard (pre-kit in this tree) | none in this tree; the 0.4.6 mock-ups are the target | A narrow column of bare checkboxes and chips with a two-line summary. This is the reason the kit exists. It gets rebuilt when 0.4.6 merges, so it is listed once, and the other 24 are the ones to touch. |
| 10 | Monthly cap sheet and Monthly caps cards | `G/89-budget-caps/usage-caps-1280x800-dark.png` (opened) | The cards read fine, but the Set a cap sheet is bare radios and two bare checkboxes with no project icons and no preview of what happens at 80 and 100 percent. Cards show Edit and Remove with the same weight. |
| 11 | Settings: Privacy and stats choice | `G/62-privacy/off-1280x800-dark.png` (earlier draft) | The three choices have no icons, and a raw table of event names in monospace takes the page; "see exactly what is collected" is a small link. |
| 12 | Settings: Advanced | `G/57-backup/progress-1280x800-dark.png`, `G/98-local-cache/on-1280x800-dark.png` (earlier draft) | Six sections stacked in one page, all small outlined buttons, no main action, a file tree and JSON inside Diagnostics. |
| 13 | Settings: General | `G/97-controls/view-open-1280x800-dark.png`, `G/99-settings-search/opened-1280x800-dark.png` (earlier draft) | Theme and list density are choices with no preview, though a preview is exactly what they need. |
| 14 | Add a machine (pairing) | `G/43-pairing/fail-locked-1280x800-dark.png` (earlier draft) | The steps are inline text with arrows; the access levels have no icons or gradation; fingerprints sit in grey mono blocks. |
| 15 | New session dialog and launch advisor | `G/73-launch-advisor/ok-1280x800-dark.png` (earlier draft) | A glyph where the tool's logo belongs; folders are text-only cards; a red box inside the main choice. |
| 16 | Launch settings and keep-alive editor | `G/83-keep-going/settings-keep-going-1280x800-dark.png` (earlier draft) | A dense in-card form: a select, "Add variable", a greyed save and two blue buttons per row. |
| 17 | Settings: The Floor | `G/99-floor-settings/pane-1280x800-dark.png` (earlier draft) | Tiny segmented controls and three-line captions about CPU use; no preview of what each look is. |
| 18 | Switch set-up offers (missing servers and skills) | `G/96-switch-setup/can-set-up-dark.png` (earlier draft) | A checklist of bare checkboxes with text tags; the consequence of each ticked item is not shown. |
| 19 | Settings: Accounts | `G/65-settings-sections/general-1280x800-dark.png` and the accounts pane in code (earlier draft) | Three identical rows with native date inputs and "Not set" repeated; a letter tile for the tool. |
| 20 | Processes: Automatic cleanup toggles | `G/21-tiers/toggles-off-1440x900-dark.png` (earlier draft) | Three near-identical cards of bare toggles with no icons and no preview of what would be cleaned. |
| 21 | Report a bug | `G/99-report-bug/sheet-1280x800-dark.png` (earlier draft) | A plain title and text box, faint placeholders, no sense of what will be sent beyond two collapsed lines. |
| 22 | Log in dialog | `G/40-login/card-logged_out-1280x800-dark.png` (earlier draft) | Says "API key account" twice, no logo, no clear single next step. |
| 23 | Pool canvas inspector | `G/97-pool-canvas/canvas-1280x800-dark.png` (earlier draft) | Four paragraphs for who leads, roles and on their own; roles are text boxes, not role cards with icons. |
| 24 | Settings: Desktop apps, Switching and Agent access | `G/92-agent-tools/agent-tools-question-1280x800-dark.png`, code in `ui/src/screens/settings/` | No styled controls at all in these three panes by the code scan, and Desktop apps and Switching have no baseline of their own. Treated as bare until walked. |
| 25 | New project sheet and project sync sheet | `G/82-project-sync/header-line-dark.png`; code `ui/src/screens/NewProjectSheet.tsx` | Plain sheet forms with no project colour, icon or preview of what is created or synced. |

Just below the cut: Add an API-key account (`G/94-service-accounts/add-account-more-1280x800-dark.png`), the cloud recovery code step, the crash report sheet, the stop and clean confirmations.

Calm screens worth a note, though they are not on this list: the Desktop window preview and the Processes page use amber for a normal, permanent state; the rail uses status colour on every row. Those belong to D-117's attention pass, not this one.

## 3. Recurring problems and the fixes that reach many screens

Each fix is a token or a component, built once under AJ-E3618 before any screen is touched. The kit names are 0.4.6's.

| Fix | What it replaces | Screens it reaches (table rows) |
|---|---|---|
| **One type scale and a real Section (title, one-line help, body)**, with a smaller "learn more" line | Section titles and row labels at the same size and weight; grey 12-pixel paragraphs everywhere | 13, 15, 18, 20, 28 to 38, 41, 43, 46, 47, 56, 60 |
| **RadioCard and CheckRow (icon, title, one line, optional preview)** with `SELECTED` for the chosen look | Bare checkboxes, bare radio rows, italic "recommended" text | 13, 18, 20, 29, 30, 33, 35, 37, 41, 46, 47, 52, 53, 54, 56, 57, 61 |
| **ToolLogo** (the app's own icon read at run time, a letter only for unknown tools) | Letters in boxes and a terminal glyph where logos belong | 15, 31, 41, 49, 50, 51, 53, 55 |
| **FlowSteps and FlowLayout (form on the left, live summary or preview on the right)** | Narrow columns on an empty page, tiny step pills, inline step text | 25, 41, 46, 47, 49, 50, 51, 52, 53, 54, 56, 58, 59, 61 |
| **A tinted summary panel** that shows the consequence ("this will copy X, never Y") | Consequences hidden in grey paragraphs | 13, 18, 20, 46, 47, 48, 51, 56, 57 |
| **One styled DateField and one TimeField** | The system's date box | 13, 31, 29 (quiet hours) |
| **One Alert with three levels (info, needs you, broken)**, one filled primary per view | Amber boxes for plain facts; several equal blue buttons | 26, 46, 48, 49, 50, 55, 59 |
| **One Segmented and Tab look** | Four different pill styles | 4, 28, 38, 23, 24 |
| **Surface ladder (page, card, raised, inset)** and a rule that cards do not nest in one tone | Cards inside cards of one tone | 24, 37, 43, 44 |

If only three things get built first, they pay off most: the type scale with Section, RadioCard with CheckRow, and FlowSteps with FlowLayout and its summary panel.

## 4. Settings > AI > Models (AJ-E3812)

This is a new rich screen and has no capture yet. Design it on the kit from the start: a RadioCard per model with its logo, a plain line on what it is good for, a size and cost line, and a live summary of what it will be used for; progressive disclosure for the rest. It must pass the mock-up in both themes before it is built, like the rest of AJ-E3618.

## 5. What this audit did not do

- It did not re-open every one of the 125 folders. About a dozen captures were opened; the rest rely on the earlier 0.4.6 audit and a source scan of controls and logos. Opus should open the "earlier draft" ones at the top of its review.
- No light-theme check beyond the earlier audit's finding that light shows the same problems.
- Counts treat a folder as evidence for one screen, so a screen with no folder (Desktop apps, Switching, Models, New project, the pool wizard) has no capture to cite.
