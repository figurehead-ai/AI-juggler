<!-- captures: 7c9eeaeb2910 -->
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
