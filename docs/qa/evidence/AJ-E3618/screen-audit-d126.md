# AI Juggler screen audit (D-126), release/0.4.6 gallery

Method: opened the main dark capture of every one of the 142 gallery folders (1280x800 or nearest) plus the light variants of Home, first-run and Notifications settings, and the two maintainer captures (58.png, 59.png). Read-only. Scores are 1 (bad) to 5 (good): R = readability, P = polish, M = mode fit.
No real-looking person names were seen in any capture (accounts are "Work Max", "Personal", "Fake Desk", example.com mail; Mac names like "Mac mini" are device labels).

## 1. Inventory (merged: folders that show one screen are one row)

| # | Screen / area | Gallery folders | Type | App area |
|---|---|---|---|---|
| 1 | Home (needs-you card, Runway, Pick up, Your apps) | 01-rail, 01-rail-density, 54-summary (stale), 55-triage, 67-home, 93-messages, 94-places, 95-tasks, 99-home-chips, 99-inactive-apps, 99-keep-alive, 97-nav, 96-notification-centre | MONITOR | Home |
| 2 | Home empty / all-clear / first-session state | 55-triage, 67-home, 74-overload (Home part) | MIXED: the feed is MONITOR; the "Start your first session" card is MAKE (inviting) | Home |
| 3 | Rail (left list, App/Project/Account/Pool, folded, splitter) | 01-rail, 01-rail-density, 01-rail-sort, 69-rail-narrow, 74-rail-projects, 99-rail-splitter, 75-feedback (skeleton) | MONITOR | rail |
| 4 | Notifications popover | 27-ranking | MONITOR | header |
| 5 | Command palette | 14-focus, 99-palette | MONITOR (navigation overlay) | overlay |
| 6 | Session page: Overview (terminal + inspector) | 05-session-panel, 06-inspector, 12-model, 51-identity, 52-worktree, 53-parked, 76-keys, 82-terminal-tools, 92-session-pages, 99-resume, 99-conversation-feed, 82-project-sync | MONITOR | Sessions |
| 7 | Session page: Usage tab | 09-quality, 26-quota | MONITOR | Sessions |
| 8 | Session page: Processes tab | 22-recovery, 10-processes (stale) | MONITOR | Sessions |
| 9 | Session page: Subagents tab | 88-subagents | MONITOR | Sessions |
| 10 | Desktop-app session: window preview | 13-desktop, 15-preview, 74-overload | MONITOR | Sessions |
| 11 | Account page (Overview, Apps, Sessions tabs) | 06-inspector (account), 08-inspector-aggregate, 79-pages, 99-account-page, 99-account-quality | MONITOR | Accounts |
| 12 | Account Settings card (plan, renewal, tidy-up) | 07-inspector-content, 11-renewal | MAKE/EDIT | Accounts |
| 13 | Accounts page (cards with usage, apps, launch settings) | 68-accounts, 78-outside, 93-accounts-profiles, 94-service-accounts, 94-usage, 83-keep-going, 75-account-manage, 77-menus | MIXED: usage rings and sparklines are MONITOR; the launch-settings editor, "Add an account" and row menus are MAKE/EDIT | Accounts |
| 14 | App page | 99-app-page, 99-app-profiles, 99-related-list | MONITOR | Apps |
| 15 | Usage page | 87-burn-down, 99-usage-cost, 25-costs | MONITOR | Usage |
| 16 | Monthly cap sheet | 89-budget-caps | MAKE/EDIT | Usage |
| 17 | Processes page (table, ports, cleanup rules) | 18-ports, 19-cleanup (stale), 21-tiers, 23-alerts, 90-processes | MIXED: the table and counts are MONITOR; the "Automatic cleanup" toggles are MAKE/EDIT | Processes |
| 18 | Stop / clean confirmations | 20-kill, 48-remote-cleanup | MAKE/EDIT (decision dialogs) | Processes |
| 19 | This Mac | 72-this-mac, 81-this-mac | MONITOR | This Mac |
| 20 | History | 56-history, 80-marks, 90-outside-search, 95-app-icons | MONITOR | History |
| 21 | Fleet page | 42-fleet, 24-fleet, 49-fleet-sync, 45-remote, 47-remote-focus | MIXED: machine/session views are MONITOR; "Link", pair and permission controls are MAKE/EDIT | Fleet |
| 22 | Fleet: Add a machine wizard, pair dialog, access sheet, remote launch | 43-pairing, 61-account-macs, 46-fleet-board, 88-fleet-launch | MAKE/EDIT | Fleet |
| 23 | Settings: General | 97-controls, 99-settings-search, 58-updates | MAKE/EDIT | Settings |
| 24 | Settings: Notifications | 84-notifications, 99-when-to-tell | MAKE/EDIT | Settings |
| 25 | Settings: Privacy | 62-privacy, 63-consent | MAKE/EDIT | Settings |
| 26 | Settings: Accounts, Keyboard | 65-settings-sections, 99-settings-keyboard | MAKE/EDIT | Settings |
| 27 | Settings: Advanced (backup, cache, diagnostics, debug, Cloud) | 44-settings, 57-backup, 60-cloud-backup, 85-diagnostics, 98-local-cache, 99-debug-mode | MAKE/EDIT | Settings |
| 28 | Settings: The Floor | 99-floor-settings | MAKE/EDIT | Settings |
| 29 | The Floor list and the 3D office | 99-floor-list, 99-floor-menu, 99-floor-looks | MONITOR | Floor |
| 30 | Pool wizard ("Set up ..." and "New pool", 9 steps) | 98-pool-setup, 58.png | MAKE/EDIT | Pools |
| 31 | Pool canvas (running pool) | 97-pool-canvas, 92-agent-tools, 99-pool-selection, 99-desktop-connector | MIXED: diagram and Needs-you are MONITOR; "Who's in / Who leads / On their own" inspector is MAKE/EDIT | Pools |
| 32 | New pool's canvas (setup state) | 59.png | MIXED, mostly MAKE/EDIT (a pool still being set up) | Pools |
| 33 | Move-to-pool dialog | 91-pools | MAKE/EDIT | Pools |
| 34 | Switch / Continue-elsewhere wizard (3 steps, handover, switch back) | 28-switch, 29-door, 30-switchback, 31-redaction, 32-graph, 33-beta, 50-cross-switch, 99-carry-choice | MAKE/EDIT | dialogs |
| 35 | "Hit its limit" decision dialog, switch preflight bits | 89-limit-ready, 96-switch-setup, 97-handover-told | MAKE/EDIT | dialogs |
| 36 | New session dialog | 73-launch-advisor, 86-open-pack, 99-new-session | MAKE/EDIT | dialogs |
| 37 | Add an account chooser | 38-flows-cde, 91-more-tools | MAKE/EDIT | dialogs |
| 38 | Add a fresh app (form) | 71-add-account | MAKE/EDIT | first run / Accounts |
| 39 | Bring in your accounts / import folder (first-run steps 1 and 2) | 37-flows, 39-import, 84-privacy, 99-first-run-confidence | MAKE/EDIT | first run |
| 40 | First-run: computer access and later steps | 66-first-run | MAKE/EDIT | first run |
| 41 | Log in dialog | 40-login | MAKE/EDIT | dialogs |
| 42 | Copy settings into another account sheet | 83-account-settings | MAKE/EDIT | dialogs |
| 43 | Delete / forget confirmations | 41-policy, 76-project-manage | MAKE/EDIT | dialogs |
| 44 | Crash report and usage-stats re-ask | 63-consent, 64-crash | MAKE/EDIT | dialogs |
| 45 | Cloud recovery code | 59-vault, 60-cloud-backup | MAKE/EDIT | dialogs |
| 46 | Report a bug sheet | 99-report-bug | MAKE/EDIT | dialogs |
| 47 | Old per-tool page ("Tool" with More menu) | 72-tool-view (stale) | MONITOR | retired area |

Counts: 47 merged screens. MONITOR 18, MAKE/EDIT 22, MIXED 7 (rows 2, 13, 17, 21, 31, 32 and the Accounts/Fleet/Processes splits). Counting each MIXED once.

## 2. Per-screen scores

Format: R / P / M. Evidence, then up to three problems.

**1 Home (needs-you card, Runway, Pick up, Your apps)** R4 P4 M4
- Big headline, Needs-you card with one filled primary and outlined secondaries, Runway bars: the best-crafted screen. Amber appears only on the headline numbers that need action.
- Problems: (a) "Runway by account" repeats "Profiles: ..." and "Resets at ..." grey lines for every account so the 58% / 0% values are the only thing that stands out and the three-line rows are noisy; (b) "Pick up where you left off" titles truncate with an ellipsis ("Keep the old shortcuts wor...") while the Mark reviewed text link has room; (c) in 99-home-chips/99-keep-alive the "Your apps" tiles are tall and mostly empty (Open buttons floating at the bottom of 230px cards).

**2 Home empty / all-clear / first-session** R3 P2 M2
- 67-home: "All clear" heading, then a huge empty left column with the Runway card pushed right; 55-triage: the "Start your first session" card is plain text plus one button with no illustration or logos; the permission banner sits under it.
- Problems: (a) a half-empty canvas (left 55% blank) in all-clear and in light; (b) the first-session invitation is two lines of grey copy, not an inviting empty state; (c) Runway for 3 accounts all show "Not measured yet" dashes, a wall of identical rows.

**3 Rail** R3 P3 M2
- Rows are two lines with the status coloured text: cyan "Working", amber "Stopped with an error", amber "Probably stuck", amber "Limit reached" all at once; app icons at 16px are identical blue glyphs; the 01-rail variants (Project grouped, 4 coloured dots per group header, F badges) look busy.
- Problems: (a) status colour on every row (cyan or amber text) so real alerts do not stand out, and "Stopped with an error" and "Probably stuck" share the same amber (D-117 says 4 attention levels); (b) the Needs you group repeats app, project and tool as 3 grey fragments truncated with "..."; (c) in 69-rail-narrow the folded rail is a column of letter monograms ("C", "F") with tiny coloured dots, unreadable without hover.

**4 Notifications popover** R4 P4 M3
- Clear section label "NEEDS ACTION - 6", tinted icon tiles, primary Open buttons.
- Problems: (a) every row has a filled primary button, so six calls to action compete; (b) bell badge and 6 "unread" dots add three more signals; (c) panel overlaps and hides part of Home's cards with no scrim.

**5 Command palette** R4 P4 M4
- Good: grouped, icons, shortcut hints. In 14-focus the input is a bare bordered box; the key-choice link "Choose a key to open from anywhere..." is faint. No major problem.

**6 Session page Overview** R3 P3 M3
- Strong header (title, state pill, account pill), terminal at left, inspector cards. But the terminal pane is the big black rectangle with 4 lines of placeholder (that is preview-only), and the "Where it runs" key/value list uses the same grey for labels and values.
- Problems: (a) labels and values in "Where it runs" are nearly the same weight and grey; (b) the inspector "Runway" card ends in a cut-off card and "More below" link: a cramped right column with clipped content; (c) header meta line "Claude Code - Last active 7 h ago - started 8h ago - Pool" is one long 12px grey line with 5 facts.

**7 Session Usage tab** R3 P3 M3
- Cards "Tokens / min", "CPU and memory now", "Errors", "Context" show the same pattern: grey title, one sentence of copy, tiny footnote "read from the session". A two-line empty banner "No token counts for it yet" leads.
- Problems: (a) every metric card is a sentence rather than a number; no big values, no hierarchy (the type scale collapses to one size); (b) "read from the session" footnote repeated in 6 cards; (c) amber "Used up until 5:13 AM" red text is fine, but the progress bars are thin 4px lines on dark.

**8 Session Processes tab** R4 P4 M3
- Table with state icons, sparklines and per-row Stop buttons; "Left behind" rows are amber with a warning triangle every row.
- Problems: (a) amber on all 4 left-behind rows plus the legend row, so no single row stands out; (b) the "Waiting for its parent means ..." paragraph explains a term above the table in 13px grey; (c) blank right gutter beside tab header.

**9 Subagents tab** R4 P3 M4
- Chat bubbles read well; the left column shows the sparse "Back" bar. Fine. Problem: a lot of empty space below a 3-line conversation; transcript bubbles are small relative to canvas.

**10 Desktop-app window preview** R3 P2 M1
- Stacked amber boxes: a permanent amber "isolation: Single-account app - Juggler tracks, cannot isolate" bar, an amber Screen Recording permission box, plus pills "Working / app / F fake-desktop" and a primary "Open / Focus window" button inside the panel while the header also has "Go to it".
- Problems: (a) amber alarm boxes for a normal state (a tool limitation notice is permanent amber); (b) two primary buttons for the same action (header Go to it and in-panel Open / Focus window); (c) four paragraphs of explanatory copy inside bordered boxes ("The window is the app's own; ...").

**11 Account page** R4 P4 M4
- Header with initial avatar, tabs with counts, Limits card with ring, sparkline, "Its apps" card with logos. Good and consistent.
- Problems: (a) "Limits" with "Not measured yet" shows a hollow grey ring with a dash, the biggest graphic carries no information; (b) the Apps tab (99-account-page) is a two-row card and a blank screen below; (c) "Last 7 days, new tokens 50M tokens" sparkline label is 11px grey.

**12 Account Settings card (renewal, tidy-up)** R2 P2 M1
- Native checkboxes, native date input "10/19/2024" with a tiny "Clear" button, "Flat fee $/mo [e. g. 200] save" as a text link, two columns of 12px grey paragraphs.
- Problems: (a) bare native checkboxes and a native date picker, no icons; (b) the explanatory paragraph under "Tidying up" is five lines of 12px grey with the safety facts buried in it; (c) section labels ("Plan and renewal", "Tidying up", "Desktop sessions") are bold 13px, same size as the rows beneath.

**13 Accounts page** R4 P4 M3
- Account cards with logo tile, rings, sparklines per account colour. Readable. 93-accounts-profiles has an Accounts / Apps pill tab.
- Problems: (a) tool logo is the monogram "C" in a grey box on every card, not the tool's logo (the rail uses real icons); (b) the launch-settings editor (83-keep-going) is a dense in-card form: "Options" links, "If a session crashes: Restart it" select, "Add variable", greyed "Save overrides" button, two blue Launch buttons per row; (c) row menu (77-menus) has a destructive red Remove and a muted grey "Start a session / Add launch settings first" item, fine, but the menu's first label "acme-max" is the same grey as hint text.

**14 App page** R4 P4 M4
- Matches Account page; "Open now" rows show coloured state words. "Find this app's other profiles" prompt card is plain. Problem: in 99-app-profiles (1024 wide) the tab row and header buttons get tight; fine otherwise.

**15 Usage page** R3 P4 M3
- Real charts (bars, heat map "When you work", burn-down), good. Problems: (a) olive bars at about 25% brightness on a dark card, low contrast against the grid; (b) right half of the top card is an empty panel in 87-burn-down; (c) "Cost by account" table shows ~$ amounts in plain white with no emphasis on the biggest number, and 99-usage-cost "Crashed or probably stuck ~$490" is the same weight as "Per finished task ~$28".

**16 Monthly cap sheet** R3 P2 M1
- Plain modal form: bare name box, amount box, bare radios, bare checkbox list.
- Problems: (a) no icon, no preview of what the cap will do ("you'll hear at 80% and 100%" is a one-line subtitle); (b) the project list is two bare checkboxes with no project icons; (c) input boxes are dark-on-dark with very faint borders.

**17 Processes page** R3 P3 M2
- KPI tiles (7 / 1 / 1 / 24) with coloured numbers, tabs with counts, big table.
- Problems: (a) amber triangle "Left behind" in 5 rows plus an amber "Automatic cleanup is paused" banner plus an amber 7: alarm-coloured without a clear top priority; (b) "Automatic cleanup" toggles are labelled "Preview" with 3 near-identical cards of 2 toggles each: a MAKE/EDIT panel with bare toggles and no icons; (c) the legend sentence above the table is a full paragraph.

**18 Stop / clean confirmations** R4 P3 M3
- Plain small modals with a red outlined confirm button; "Won't stop it" red outlined notice. Fine for a dialog. Problem: the monospace "program ? / ports -" fragment reads as debug output; no icon.

**19 This Mac** R3 P3 M3
- Memory/CPU/Swap cards. Problems: (a) the memory bar is grey with a hatched segment, so "22 GB in use of 32 GB" (the key fact) has the same visual weight as the paragraph below it; (b) cards on the left are staggered masonry with an empty slot beside "AI Juggler 204 MB"; (c) a 4-line explanatory paragraph inside the Memory card ("Memory macOS keeps compressed is shown on its own ...") is longer than the data.

**20 History** R4 P4 M4
- Clean list with day headers, logos, project and profile chips and a good search box with filter pills. Problem: the Archived/Ended segmented control and the filter pills are three different pill styles; row metadata ("Ended - main - in Claude Code") is 12px grey.

**21 Fleet page** R3 P3 M2
- Cards "Link on", "Paired machines", "This Mac" with tinted badges ("Degraded v1.1", "Synced", "View only"). 
- Problems: (a) five status pills per machine in different colours (amber, green, outline, outline) on one row: noisy for a monitor view; (b) the machine fingerprint is a 3-line mono block in the same grey as labels; (c) Link-off toggle card and the two cards beneath share one surface tone so there is no hierarchy ("Paired machines" nested inside a card).

**22 Fleet wizards / sheets** R3 P3 M2
- 43-pairing "Add a machine": big code 482 913 is good, but the stepper "1 Find -> 2 Verify -> 3 Permissions" is 12px text, content sits in a small modal over a blurred page. 46 access sheet: four radio cards with title + description, good, but no icons or illustration for No access / View only / View + control.
- Problems: (a) permission levels are bare radios with no icon or colour gradation; (b) stepper as inline text arrows; (c) 61-account-macs: two fingerprint boxes with 2 lines of mono text, the key comparison, in 12px.

**23 Settings: General** R4 P3 M2
- Theme, Session list, Order cards use radio-card tiles (selected tile tinted). Better than most, and the left nav has coloured icon tiles.
- Problems: (a) Theme is three bare radio rows with no sun/moon/system preview; (b) Session list (Comfortable/Compact/Minimal) has no preview of what a row looks like, though that is exactly what needs one; (c) the page is a narrow 740px column next to a 260px nav on a 1280 canvas, rest empty on the right.

**24 Settings: Notifications** R2 P2 M1
- Nine native checkboxes with grey 12px captions, then Quiet hours, then a second checkbox list per account, then "When to tell me" selects.
- Problems: (a) a long list of bare checkboxes with no icons and no grouping; titles and captions have little contrast (title 13px white, caption 12px grey); (b) section labels ("Notify me outside AI Juggler", "Quiet hours", "In the digest and notifications") have the same size as list items, so the page has no scannable structure; (c) the intro paragraph is 3 lines of copy before the first control.

**25 Settings: Privacy** R3 P2 M2
- Three radio cards for stats, then a code-style table of event keys ("accounts.configured", "perf.cold_start_ms") with counts.
- Problems: (a) a raw telemetry table in monospace on the page's main area reads like a log; (b) the three choices have no icons (cloud-send, device, off), despite being the main decision; (c) "See exactly what's collected" is a small underlined link under the choices.

**26 Settings: Accounts / Keyboard** R4 P3 M3
- Keyboard page is a tidy table with keycap chips (good). Accounts page: three cards, native "mm/dd/yyyy" date inputs for renewal, three identical rows.
- Problems: (a) native date inputs on 3 rows, "Not set: no renewal reminders" repeated 3 times; (b) "Tools on this computer" card has a monogram "C" tile; (c) header "Accounts" then "Accounts and profiles" repeat the same word.

**27 Settings: Advanced** R3 P3 M2
- Many cards (Retention, Backup, Local cache, Diagnostics, Debug, Figurehead & Cloud) with buttons and dense captions; toggles and a "What the zip holds" tree with JSON. 
- Problems: (a) 6 sections stacked in one scrolling page, each headed by a 14px bold line, so scanning is hard; (b) JSON snippet and file tree inside Diagnostics; (c) button rows are all the same small outlined style (Back up now, Restore, Export, Report a bug) with no primary.

**28 Settings: The Floor** R3 P3 M3
- Rows with caption + control; the segmented controls are tiny (Follow the system / Full / Calm / Off). Problem: captions are 3-line explanations with CPU percentages ("about 2.7% of a core ...") for a decorative feature; no preview of Pixel art vs Full resolution.

**29 The Floor list and 3D office** R4 P4 M3
- The 3D (99-floor-looks) is a distinct pixel-art style with room labels in uppercase pixel font: strong, but it is a different design language from the rest of the app (pixel font, purple background). List view is a tidy table with state glyphs. Problem: the list has Floor / State / Session / Project / Subagents / Tasks columns where Subagents and Tasks are mostly empty.

**30 Pool wizard** R2 P2 M1 (already being redesigned; classified only)
- Dark empty canvas with a 620px content column, 9 numbered steps in a left rail, bare chips ("Apps 0 / Sessions 0 / More"), bare checkboxes, a small "In this pool" summary card, no icons, no previews.
- Problems: (a) the form is a narrow column in the middle of a 1690px empty canvas; (b) the choices (This Mac, mini) are bare checkbox rows, no app logos; (c) the live summary "In this pool" is two lines of text, not a picture of the result. 98-pool-setup (the template step) is better: 9 tiles with title, description and meta line, but tiles are text-only (no icon or diagram per template) and the last row of tiles is clipped by the footer.

**31 Pool canvas (running)** R3 P3 M3
- The diagram with nodes, dashed handoff lines with amber chips ("1 waiting", "Paused") and an inspector. Good idea; the inspector has 5 paragraphs.
- Problems: (a) a 4-paragraph consent prompt card (92-agent-tools: "Let the sessions in your pools message each other ...") sits on top of the canvas and pushes the diagram down; (b) the right inspector "WHO'S IN / WHO LEADS / ON THEIR OWN / TEAM TOOLS" is four 13px paragraphs, headings in small caps grey; (c) the Custom actions menu (99-pool-selection) is a list of 12 bare checkboxes.

**32 New pool's canvas** R2 P2 M1 (already being redesigned; classified only)
- Mostly empty 1690x1340 canvas with a dot grid; a long consent prompt across the top; the "Add a session" panel at left and role cards stacked at right; a footnote "Its sessions show here as they start" at the bottom.
- Problems: (a) a vast empty dot grid with no illustration or example of what a pool looks like; (b) the consent prompt is 4 paragraphs of grey copy before the user sees anything; (c) role cards ("Lead - No lead for now", "Everyone Member") are outlined text boxes without icons.

**33 Move-to-pool dialog** R4 P3 M3
- Simple list with an "It's here" marker. Problem: bare text rows, no pool icons or member count; fine as a small picker.

**34 Switch wizard** R3 P2 M1
- Step pills "Target / What's carried / Confirm" (small outlined chips with a check), long bullet copy, amber boxed warnings, tag chips "The whole conversation / Project rules / Your uncommitted changes" (outlined grey), radio options with italic "recommended" text.
- Problems: (a) a MAKE flow that is mostly paragraphs: step 1 (28-switch step 1) is three sentences plus two amber boxes; step 2 lists "Goes along" as bare grey chips; step 3 has 4 stacked sections; (b) the "what carries over" information is chips and text, not a visual (before/after, source -> target with the two app logos); (c) amber used for "Beta - Not proven yet" striped badge, two warning boxes, and "Stays behind" at the same time. 30-switchback is the worst: three radio groups with a "todos.json" conflict and a long amber note. 29-door's Desktop/CLI cards are the best piece (radio cards) but have a dashed empty circle for disabled and no logos.

**35 Hit-its-limit decision dialog** R4 P3 M3
- Two option cards with titles and one primary button each; good structure. Problems: copy-heavy ("Takes the conversation, the branch and the project's context with it. Switching is in beta ...") and the second card's "Park until 4:13 AM" is a grey full-width button that looks disabled.

**36 New session dialog** R3 P3 M2
- 73-launch-advisor: folder cards with a folder glyph, account card with "Suggested: the most room" chip, then a red square and four lines on memory pressure. 99-new-session is a good compact sentence "Start in web-shop on Claude Code - Work Max".
- Problems: (a) account choice is an outlined card with a ">_" glyph, not the tool logo and account colour; (b) folders are text cards without project colour or last-used time visual; (c) the red "Not now - this Mac is short of room" box is alarm-coloured inside the main choice.

**37 Add an account chooser** R2 P2 M1
- A list of tools: "Claude Code Installed [Claude Code ->]" and one blue button per row (4 identical blue buttons: Claude Code, OpenCode, Kimi Code, Grok Build); monogram tiles "C / O / K / G".
- Problems: (a) letter monograms instead of tool logos; (b) the button repeats the tool name and is the same colour on every row so nothing is the primary; (c) a grey "More" disclosure hides the rest, and the copy "Which tool is it for? Installed tools are ready; others link to their install page." is the only guidance.

**38 Add a fresh app form** R3 P2 M1
- Narrow 770px column on an empty dark canvas, with a bold-grey paragraph, a focused input, a bare checkbox "Start with the same settings as [Work Max]", and a legal footnote.
- Problems: (a) bare form on an empty canvas, no illustration or preview of the app being created; (b) no tool logo despite saying "Claude Code"; (c) the bold intro paragraph is the same size and weight as the headline's supporting copy, followed by three small grey notes.

**39 Bring in your accounts / import folder** R3 P3 M2
- Better than most: stepper with numbered circles, a table-like card of setups with checkboxes, two radio cards ("Use them where they are - Recommended" / "Make separate copies"), a glowing primary. 99-first-run-confidence: the "Default / Found", "claude-work / Likely" rows add confidence chips. 
- Problems: (a) the table has bare checkboxes and 6px coloured dots rather than app logos; (b) a bold 4-line intro paragraph and 4 terms/legal footnotes with "C / O" monograms at the bottom take half the screen, and one is cut off at the bottom edge (37-flows); (c) lots of empty canvas on both sides of a 770px column.

**40 First-run: computer access** R3 P2 M1
- "One optional permission makes it smoother." One card with Allow and "Continue"; the rest of the canvas is empty. The light version (66 access-reopen) with two cards is the same bare style.
- Problems: (a) a blank 1280x800 screen with one 90px card, no illustration of what the permission does; (b) the heading, the bold grey subtitle and the card all use near-identical dark/white tones; (c) "Skip for now" and "Continue" separated across the width with no emphasis on what the permission gains.

**41 Log in dialog** R3 P2 M2
- Small modal showing "API key account" pill twice (once as text, once as a pill), a cyan dot and a ring. Problem: duplicate label ("API key account" twice in 3 words), no logo, no action beyond text.

**42 Copy-settings sheet** R2 P2 M1
- Checklist of MCP servers/skills with numbered 4-step terminal instructions and a code block, in a plain card with no title bar styling; the code block overflows (two long commands wrap).
- Problems: (a) a wall of instructions in 12-13px text (steps 1 to 4 inside one checkbox row); (b) bare checkboxes with 12px "MCP server" tag text; (c) the sheet is clipped at the bottom with a cut-off button.

**43 Delete / forget confirmations** R4 P3 M3
- Short title, one paragraph, red outlined button. Fine; no icon.

**44 Crash report / stats re-ask** R3 P3 M3
- The crash sheet shows the raw JSON report, which is honest but heavy (monospace, 14 lines). Stats re-ask (63-consent) reuses the radio cards: fine.

**45 Cloud recovery code** R4 P3 M3
- Good code box, amber warning, a checkbox gate. Problem: three small outline buttons of different weights (Later, Start backups); "Start backups" is outlined rather than filled when the box is ticked.

**46 Report a bug sheet** R3 P2 M2
- Plain form with a title box and textarea, collapsed "Goes with it" and "The whole report" disclosures; footer note about public issues. No icons; low-contrast placeholder.

**47 Old "Tool" page** (stale) R3 P2 M2
- Uppercase "TOOL" label, tiny "RUNWAY, LEAST LEFT FIRST" caption, red "0% left", three header links; the Add account button overlapped by a menu. This is the previous design (see section 5).

## 3. Ranked worst-first (25)

| Rank | Screen | Type | Why (one line) |
|---|---|---|---|
| 1 | Pool wizard "Who's in" (58.png) and template step (98-pool-setup) | MAKE/EDIT | Already in redesign. Narrow column on a 1690px empty canvas, bare checkboxes and chips, text-only tiles |
| 2 | New pool's canvas (59.png) | MIXED, mostly MAKE | Already in redesign. Empty dot-grid canvas, 4-paragraph prompt, text-box role cards |
| 3 | Settings: Notifications (84-notifications, 99-when-to-tell) | MAKE/EDIT | Nine+ bare native checkboxes, no icons, headings same size as rows |
| 4 | Switch wizard (28-33, 50, 99-carry-choice) | MAKE/EDIT | Paragraphs and grey chips, amber everywhere, no source-to-target visual; step pills tiny |
| 5 | Add a fresh app form (71-add-account) | MAKE/EDIT | Bare form in a narrow column on an empty canvas, no logo or preview |
| 6 | Add an account chooser (38-flows-cde, 91-more-tools) | MAKE/EDIT | Monogram letters instead of logos, four identical blue buttons |
| 7 | First-run computer access and permission steps (66-first-run) | MAKE/EDIT | One small card on a blank screen, no illustration |
| 8 | Bring in your accounts / import (37, 39, 84-privacy, 99-first-run-confidence) | MAKE/EDIT | Bare checkboxes, dot-only identity, half the screen is legal footnotes |
| 9 | Copy settings into another account sheet (83-account-settings) | MAKE/EDIT | Wall of terminal instructions and code in a plain sheet, clipped |
| 10 | Account Settings card and renewal (07-inspector-content, 11-renewal, 65 Accounts) | MAKE/EDIT | Native checkboxes and native mm/dd/yyyy inputs, 12px grey paragraphs |
| 11 | Desktop-app window preview (13-desktop, 15-preview, 74-overload) | MONITOR | Permanent amber boxes for a normal state, two primary buttons, 4 paragraphs |
| 12 | Settings: Privacy (62-privacy) | MAKE/EDIT | Raw monospace telemetry table on the main area; choices have no icons |
| 13 | Monthly cap sheet (89-budget-caps) | MAKE/EDIT | Plain form with bare radios and checkboxes, no preview |
| 14 | Processes page (18, 21, 23, 90-processes) | MONITOR (+ cleanup toggles MAKE) | Amber on every left-behind row, triple warning, bare "Preview" toggles |
| 15 | Settings: Advanced (44, 57, 60, 85, 99-debug-mode) | MAKE/EDIT | Six stacked card sections, JSON and file tree, all-equal outlined buttons |
| 16 | Settings: General (97-controls, 99-settings-search) | MAKE/EDIT | Theme and session-list choices have no preview; narrow column |
| 17 | Fleet page (42-fleet, 49-fleet-sync, 24-fleet) | MIXED | Five status pills per machine, nested cards, mono fingerprint blocks |
| 18 | Fleet wizards and sheets (43-pairing, 61-account-macs, 88-fleet-launch) | MAKE/EDIT | Inline-text stepper, bare chip rows, permission levels with no icons |
| 19 | Home empty / all-clear / first session (55-triage, 67-home) | MIXED | Half-empty canvas, plain invitation card |
| 20 | Pool canvas, running (97-pool-canvas, 92-agent-tools, 99-pool-selection) | MIXED | Prompt eats the canvas, inspector is four paragraphs, 12 bare checkboxes in the menu |
| 21 | New session dialog (73-launch-advisor, 86-open-pack) | MAKE/EDIT | Glyph instead of logo, red box inside the main choice |
| 22 | Rail (01-rail, 01-rail-density, 69-rail-narrow) | MONITOR | Status colour on every row, two ambers identical, monogram column in folded view |
| 23 | Session Usage tab (09-quality, 26-quota) | MONITOR | Metric cards are sentences, no big values |
| 24 | Settings: The Floor (99-floor-settings) | MAKE/EDIT | 3-line explanations with CPU percent, tiny segmented controls, no preview |
| 25 | This Mac (72-this-mac, 81-this-mac) | MONITOR | Key number same weight as copy, staggered cards, long explainer paragraph |

Worth fixing just below the cut: Accounts launch-settings editor (83-keep-going), Usage burn-down chart contrast (87), Login dialog (40), Report a bug (99-report-bug), Account page Apps tab emptiness (99-account-page).

## 4. Patterns that recur (fix once, at token or component level)

1. **Section headers and labels are the same size and weight as rows.** Seen in Settings Notifications ("Notify me outside AI Juggler", "Quiet hours"), Account settings ("Plan and renewal", "Tidying up"), Advanced (every card), the Pool inspector (small caps grey), Processes legend, This Mac. Fix: a real type scale with three steps (page title 22-24, section heading 15-16 semibold, label 12 uppercase or medium), and a Section component that everything uses.
2. **Bare native controls on MAKE/EDIT screens.** Native checkboxes (Notifications x9, Privacy, Account settings, Cap sheet, Custom actions menu, import table), native radios without cards (Theme, Cap sheet, Switch "stop original"), native `mm/dd/yyyy` date inputs (Accounts settings, Renewal). Fix: one styled Checkbox, one RadioCard (icon + title + one-line + optional preview), one DateField, and make them the only exports.
3. **Letter monograms and glyphs where tool/app logos belong.** "C / F / O / K / G" in outlined squares in the Add-account chooser, Accounts cards, Settings "Tools on this computer", import footnotes, the folded rail, and ">_" in the New session account card; the rail and Home use real icons. Fix: one ToolLogo component (logo, tint, fallback monogram only for unknown tools) in every choice row.
4. **Narrow centred column on an empty canvas** for every MAKE flow: pool wizard (620px on 1690), add app, import, first-run steps, Settings (740px content next to a 260px nav), plus no illustration or preview. Fix: a FlowLayout with a two-pane structure (form at left, live preview/summary at right) and a max width that uses 1100+ px; the steps rail gets icons.
5. **Amber used as a general "notice" colour, not an alert.** Persistent amber boxes ("Single-account app", "isolation"), "Beta - Not proven yet" striped badge, "Stays behind", "Renews in 6 days", "Left behind" on every process row, status text for Stopped and Probably stuck in the rail, memory-pressure red inside the New-session card. Fix: define an Alert/Notice component with 3 levels (info neutral, attention amber only when user action is needed, critical red) and remove amber from informational facts; Processes rows use one amber marker, not five.
6. **Explanatory copy as grey 12px paragraphs.** Switch wizard (3 to 5 sentences per step), Pool prompt (4 paragraphs), Settings captions, Fleet header, Accounts header, Account settings, Floor settings, This Mac. Fix: a HelpText component with max two lines and "Learn more"; move the rest behind disclosure; raise secondary text contrast (grey is about #8b93a1 on #12161e).
7. **One surface tone for everything (card in card).** Fleet, Settings Advanced, Processes, Pool inspector: bordered dark card inside a bordered dark card inside the page, so hierarchy comes from borders only. Fix: a surface ladder (page, card, raised, inset) with distinct fill tints and a rule that cards do not nest in the same tone.
8. **Dead space on monitoring pages.** Home all-clear (left column empty), Usage top card (right panel empty), Account Apps tab (one card then blank), Pool canvas below diagram. Fix: layout rule that columns collapse when empty, and empty states with a purpose (an action or an illustration).
9. **Inconsistent segmented controls and pills.** Rail App/Project/Account (filled dark pill), History Ended/Archived (bordered box), Diagram/List (pill), Floor settings (tiny chips), Fleet badge pills (outlined plus filled), tab rows with counts. Fix: one SegmentedControl, one Tab component, one Badge with fixed variants.
10. **Primary action competes.** Notifications popover (six filled Open buttons), Add-account chooser (four identical blue buttons), session window preview (two primaries for one action), Home needs-you rows each with a button. Fix: one filled primary per surface; the rest as outline or text.

## 5. Captures that look stale (old layout)

Certain (clearly the old design: top nav "Home / Accounts / History / Settings / More", rail chips "Needs you / Running / All", pixel-grey avatars with initials "FD / WM", uppercase "SESSION / TOOL / APP" labels, red "Failed"/"Hit its limit"):
- 10-processes (all captures), 19-cleanup (all), 72-tool-view (all). Also the clean-up rows of 20-kill and 21-tiers show the same old row table only in the blurred background; the dialog front is current.
- 54-summary: old Home ("Since you last looked" card with boxed groups), predates the Needs you card, and it is rendered with a different font stack.

Probable (captured with the earlier Project-grouped rail, no "Needs you" or "Open now" group, no Pool tab, older Home): 01-rail, 01-rail-density, 01-rail-sort (its Home is the older one), 05-session-panel, 06-inspector, 07-inspector-content, 08-inspector-aggregate, 09-quality, 11-renewal, 12-model, 13-desktop, 14-focus, 15-preview, 17-ledger, 18-ports, 22 to 33, 37 to 41. These were captured against the rail before the App-grouped rail (cf. 94-places and 99-* captures which show the current rail with "Needs you" and "Open now" groups). They show the right component, but their surrounding chrome is older: re-capture before using them as design references.

Different font rendering (Linux Playwright stack, Arial-like, not the app's Inter): 53-parked, 54-summary, 66-first-run (some), 71-add-account, 37-flows. Not an app problem but they look cruder than the app, so they understate the real design.

Current (match the latest layout, with App rail, Pool tab and back/forward arrows): 92-agent-tools, 93-messages, 94-places, 94-usage, 95-tasks, 97-*, 98-*, 99-* (all), 87-burn-down, 88-subagents.

## 6. Summary

- Types: MONITOR 18, MAKE/EDIT 22, MIXED 7 (47 merged screens from 142 folders).
- Biggest gap is on MAKE/EDIT: 16 of the 25 worst are making or editing screens (wizards, forms, settings, onboarding), where bare native controls, monograms, narrow columns, and paragraphs of copy replace the accent-colour choice cards, logos and previews D-126 asks for. The monitoring screens are mostly well-crafted; their problems are over-use of amber and dead space.
- Light mode matches dark (checked Home, first run, Notifications): the same problems, slightly lower contrast on grey captions.
