<!-- captures: a6a552922bf3 -->
# 99-pool-selection: the canvas's selection and the inspector's title (AJ-E3012)

Design critique (D-57, AJ-E2814) against 97-pool-canvas (the canvas before) and the calm UI round 2 rules. The canvas with each kind selected — nothing (the pool), a session, an App, an Account, a Tool, everyone in scope, a line, the pool's own setup opened from the checklist, and the lead's place waiting for an App's next session — dark and light, 1280×800. Fixture names only.

## Checklist

| Check | Finding |
|---|---|
| Grid | Canvas, the members' column (224 px) and the inspector (340 px) share the page card's edges; the inspector's title block aligns with its sections' 16 px gutter. |
| Hierarchy | The inspector leads with the selection's own title (15 px semibold), its kind and App as a subtitle (muted), then its status pill; the setup line and the sections follow. |
| Spacing | 12 px between member cards in the column, the stack's two edges inside it; the inspector keeps its 8 px rhythm. |
| Icons | The App's icon beside a session's title; the lead's crown on the App whose next session leads. |
| Shared controls | The role and own-choice Pickers, the setup's LeadPicker, JoinAs, RoleCards, TalkTable and OwnLevelChoice in the pool's own setup. |
| Disabled reasons | An App's role picker greys Lead with its reason. |
| Plain words | "Session · Claude Code · Work Max · Lead", "App · Reviewer", "App · Leads next", "Line · what passes between these two"; "No session open yet · new ones in any project join"; "Its sessions read messages on their next turn"; Diagram, not Canvas. |
| Type sizes | 12 px floor in the cards' wake lines and the pill. |
| Both themes | Checked dark and light: the accent ring and its canvas-coloured offset read in both; the raised surface is 0.5's `--color-selected`. |
| 1280×800 | With four broad members the column scrolls inside itself; the canvas fits beside it. |

## Good

- One selection treatment everywhere on the canvas: a solid 2 px accent ring with a 2 px offset on a raised surface; no dashes. Keyboard focus is a separate ink outline outside it.
- The selected card and the inspector are linked by the same accent on the inspector's title edge.
- Broad members read as many (a stacked card), not as a dashed outline; their names take two lines before an ellipsis.
- The setup checklist is the pool's: in full on the pool, one line over its settings, and never over a member, a line or the lead's card.

## Weak

- Each broad card's wake line adds a third line; with four cards the column scrolls at 800 px.
- The inspector's back control reads "Pool" (to the pool) and "Done" (from its settings): two words for one way back.

## Fixed

- (The coordinator's check) Compact names broke mid-word at the 38 % fit ("Plann/er", "Checkou/t copy…"); names now break only between words, and a word wider than its room makes the name one line with an ellipsis.
- (The coordinator's check) A member's inspector showed the pool's "Setup 2 of 6" line; it shows only on the pool.
- (The coordinator's check) The pool's talk list showed an Observer's sends as Allowed though every one asks you; it reads Ask first now, and a new pool blocks them.

- (This branch) Opening a setup step scrolled the whole window (scrollIntoView); it scrolls the inspector's own list now.
- (This branch) The talk table's five columns didn't fit the inspector's 340 px; there it is a line per role, its pairs wrapping.

## Round 2 (AJ-E3011, after the two newcomer walks)

Checked by eye, dark and light, against the calm rules and the checklist above.

- The pool's view is **Diagram | List** (the walks read "Canvas" as a drawing area).
- An App whose next session leads is one card, not a Lead placeholder beside it: "App", the crown and "Leads next", its name, "Its next session leads · its others join as Member", its open-now line, and **Start one now** under it (the accent tint, not a filled primary: the page keeps one). Selected, the inspector's subtitle reads "App · Leads next", with a Who leads section (the lead line, Start one now, Change who leads) and "Its other sessions join as" over its role picker. Pass, both themes.
- Broad cards say "No session open yet · new ones in any project join" (was "none open now · new ones join").
- The pool's own inspector keeps the team tools card above the checklist while they're off for it (see 92-agent-tools), and still once every step is done.
- Weak, seen (not from this round): at the 38 % fit a line from "Fix the cart…" crosses the "Checko…" and "Review…" names when the Account is selected, and the "Review…" badge's line meets its edge; both from the canvas's edge routing.
- Moved by round 2 (all 18 captures): the header's Diagram label, the broad cards' open-now words, and selection-lead-next (the one App card and its inspector).

## Round 3 (AJ-E3011, the diagram path)

The walk set a project's own pool up on the diagram and found it said less than New pool. Checked by eye, dark and light:

- selection-pool (nothing selected): the inspector opens on the summary in the Review's own sentences (who's in, who leads, on their own, team tools, each with its Change), then what waits for you in **Needs you** (said with what it is, a link to it), the team tools card while they're off, then the checklist. The facts block ("Messages · Wait for you", "Pause after 5 in a row") is gone. Pass.
- selection-pool-setup: the pool's settings keep Who's in, Who leads (the setup's LeadPicker and its read-back) and Everyone else open; Who may message whom and What they may do on their own are folds with the setup's own why-lines, open when the checklist opens them; the rest of its settings (Allowed without asking, Messages into sessions, Used this week, Notifications, Your other Macs, Pools its agents may write to) are folds closed at first, each with one plain line. Pass.
- selection-app-role (new): an App's role menu offers "Lead: its next session leads" with its line, enabled; an Account's Lead stays greyed with "Lead is one session; pick one of this Account's sessions to lead." in the menu and beside the picker. Pass.
- The header reads "On their own (everyone in this pool): Work together", the setup's own control as a menu.
- Everyone's card reads "Every new session in any project joins here, now and later".
- Weak: the summary's Who's in names three and "and N more"; at 1280×800 the checklist starts below the fold when the team tools card shows.

## Round 4 (AJ-E3011): ready with what happens next, "takes charge", More settings

Checked by eye, dark and light:

- selection-pool: the pool is ready (who's in set, defaults count as done, the team tools on), so the checklist gives way to "This pool is ready" and "What happens next: Planner leads; Claude Code CLI · Personal checks its work; their messages show here." (the same sentence New pool's Review ends with). Pass.
- selection-app-role: the App's role menu reads "Lead, once one of its sessions starts" with "It takes charge as soon as one of its sessions starts in the pool's projects. Its other sessions keep their role." While it's open the page under it is inert (menus-a11y.spec checks a press on Reviewer can't reach Take out of the pool). Pass.
- selection-lead-next: the App card's badge "Lead when a session starts", its line "Takes charge as soon as one of its sessions starts · its others join as Member"; the inspector's Who leads sentence and role picker say the same. Pass.
- selection-pool-setup: the rarer settings (Kept apart, Your other Macs, Pools its agents may write to) sit under one closed "More settings", so Who may message whom opens higher. Pass.
- Take out of the pool is held like Stop (a toast with Undo); nothing changes on these captures.
- Weak (unchanged): at the 38 % fit the line from "Fix the cart…" crosses the "Checko…" and "Review…" names.

## Sam's round-4 follow-ups (AJ-E3011)

- header-custom (new): Custom chosen in the header opens its list in place, under the control: "Custom: what they may do on their own", "On their own they may do the 11 things you picked, under Custom actions.", and the pool's own Custom list (the inspector's and Settings' one list), Done at its foot; the page under it is inert. The inspector's summary reads the same count ("On their own they may do the 11 things you picked"). Checked dark and light. Pass.
- Weak: at 1280×800 the list's Done sits below the popover's fold (it scrolls inside); Escape and a press outside close it too.
- The ready line names which to open first when an App's next session leads ("Open Claude Code CLI · Work Max in web first: it leads; …"); none of these captures has that state in view.

## Round 5 (AJ-E3011, under D-55)

- Every capture: the inspector is 380 px at first (was 340), resizable from its left edge 360 to 420 px, its width kept.
- opened-lead (new): Who leads opened from the summary's Change sits at the panel's top with the chosen radio focused (its ring visible), the whole list in view at 1280×800. A browser test checks the same at 1024×768 and that the role menu and the join menu open inside the window. Pass.
- The talk table's cells look like buttons ("Click a cell to change it." above it). Pass.
- Weak: at 1280×800 the opened Who leads list pushes its read-back to the panel's foot.

## Round 6 (AJ-E3011)

Checked by eye, dark and light:

- selection-pool and header-custom: the ready box ends with the first prompt to give, with Copy ("Then tell Planner: “Plan the work in every project, hand out tasks, and ask Claude Code CLI to review.”" on selection-pool). Pass.
- Weak: at 1280×800 selection-pool's prompt line sits at the panel's foot, its second line below the fold (the panel scrolls).
- selection-app-role re-rendered with 9 px of noise and was left as committed.

## Section titles fixed (2026-11-16)

Section titles re-rendered (6db085536): a colour token named like a type size (`--color-section` beside `--text-section`) had Tailwind build `.text-section` as a colour only, so every section title in the captures from 56f98a514 on was about 13 px regular grey. Re-captured and checked by eye against the earlier captures:

- Every capture: the inspector's title (15 px semibold, as Hierarchy says) and its section titles ("Who leads", "Waiting on you", "The conversation") are ink and semibold again; what's under them sits 1-2 px higher. Wording, controls and states are unchanged. Pass.
