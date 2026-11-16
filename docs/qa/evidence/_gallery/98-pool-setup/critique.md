<!-- captures: 28402ae9aab4 -->
# 98-pool-setup: the pool setup a newcomer understands (AJ-E3011)

Design critique (D-57, AJ-E2814) against the reference screens: Settings' choice lines, the switch dialog's "What does this mean?", and the calm UI round 2 mocks (levels, section labels, surface tone). Every step of New pool, empty (the first pool: the intro full, nothing chosen, the defaults filled in) and filled (a name, two Apps, an App's next session leading, a Reviewer, a pair blocked, waking, the seal), dark and light, 1280×800. Fixture names only.

## Checklist

| Check | Finding |
|---|---|
| Grid | The step list (240 px) and one content column (max 768 px for a form, the Who's in list with its side panel) on the page's 24 px gutter; the foot spans the window. |
| Hierarchy | Step title (15 px semibold) → its why-line (ink-2) → labels (13 px medium, ink) → read-back sentences (ink) → hints (muted). Section labels are the calm small capitals (`section-label`). |
| Spacing | One scale: 20 px between a step's groups, 8 px inside a group, 2 px between list lines (no dividers). |
| Icons | The roles carry their own icons (Lead's crown, Member's list, Reviewer's tick, Observer's eye); Start from scratch its plus. No icon on a section label. |
| Shared controls | Picker (join as, a member's Role: and On its own:, More kinds, Add a project), the calm Segmented (where, Apps / Sessions / More), ChoiceOption (the lead picker, what they may do on their own, the seal), the app's disclosure (the intro, "What does this mean?"), the Button primitive (Review's Turn on). |
| Disabled reasons | Lead on an App's role picker is greyed with its reason ("Lead is one session…"); Make pool shows only once Who's in is reached. |
| Plain words | No "Level", no "From ↓ · To →", no "Blank", no "talk: allowed", no "full / shell only / watched": Look only / Work together / Work together and wake each other; "Lead can message…"; "Start from scratch"; "everyone may message", "kept apart"; "2 apps, and every new session in storefront"; each choice read back as a sentence. |
| Type sizes | 12 px floor (the role cards' holders line, segment words); body 14, meta 13. |
| Both themes | Checked dark and light: the selected segment's edge (0.5's selected-ring) reads in both; the talk table's amber Ask first and red Blocked keep AA. |
| 1280×800 | Every step fits without scrolling except Review filled, which scrolls inside its column (its Change buttons keep it keyboard-reachable). |

## Good

- The order is visible on every step (the side list with ticks) and the foot always says what the rest is filled in with, so "Make pool" from Who's in on is safe.
- Who leads is never empty: sessions open now, each App's next session, or "No lead for now" with what a lead adds.
- Colour only where it means something: the talk table's one exception stands out (amber dot, red ⊘), Allowed is muted text.
- The intro is three lines and a small diagram on the first pool, a one-line fold afterwards.

## Weak

- The Everyone else step reads its join line twice (the labelled picker, then the sentence under it); kept, since the sentence is the read-back the spec asks for, but it is the one repeat on the screens.
- The lead picker's radios sit at the right edge (the shared choice line's layout); a long list of open sessions puts them far from the names on a wide window.
- Who's in's Sessions segment counts every session it could add (8: open and recent), while the App rows count open ones ("5 open now"); both true, but read side by side.
- On their own's legend ("What sessions in this pool may do on their own") repeats the step's title above it.

## Fixed

- (The coordinator's check) Start from scratch let an Observer message everyone, against its role card ("Watches and reads. Changes nothing.") and every template; its sends are blocked by default now, its own row calm, and the review says "an Observer only reads".
- (The coordinator's check, follow-up) The foot under Who's in to On their own still said "Everyone may message everyone." while the table blocked an Observer; it reuses the Review's sentence now ("…; an Observer only reads."), so table, foot and Review agree.

- (This branch) The step heading lacked weight against its why-line; semibold now.
- (This branch) Review's long filled column had nothing to focus in it (axe: a scrolling region); each sentence now has its Change, back to that step.
- (This branch) "Everyone in this pool's projects" named by its scope: "Everyone in storefront".

## Round 2 (AJ-E3011, after the two newcomer walks): every step, both themes

Checked by eye, dark and light, empty and filled, against the calm rules (L0 surroundings, colour only where it means something, surface tone over borders, one selection ring) and the checklist above.

| Step | What changed, and the check |
|---|---|
| How to start | The intro's first line defines a pool ("a team of your AI sessions working on the same projects"); each template's chips in plain words ("everyone may message", "members ask before messaging each other", "look only", "kept apart"), muted 12 px. Cards surface-toned, Start from scratch with the one ring. Pass, both themes. |
| Where | Unchanged. Pass. |
| Who's in | One line above the kinds says what adding brings ("Add whole apps — every session they start here joins — or single sessions open now"); Apps and Sessions are segments, Accounts, Tools, Macs and Everyone under More. The side panel says who's in, in words ("2 apps, and every new session in storefront"), each by name with its kind, "Everyone in storefront" saying what it is; no jargon counters. Pass; one weak point above (the Sessions count). |
| Who leads | The why-line makes leaving it a choice ("Pick one to be in charge, or leave it for now"); an App's row says it leads from its next session in storefront and that Start one now comes on its card once the pool is made; the read-back says "(none is open now)" when that's so. The No lead group sits a group gap (12 px) below the App rows, as a separate choice. Pass. |
| Everyone else | The roles are a plain reference ("What each role does": icon, name, line, who holds it; no surface, edge or hover). Each member's pickers are labelled (Role:, On its own:); the App whose next session leads is one row, "Leads next" with its crown, "Its others: Member", and its sentence ("its next session leads; its others join as Member…"). Pass, both themes. |
| Who may message whom | Unchanged since the Observer fix; the Observer's row calm ⊘ Blocked, the sentence says "an Observer only reads". Pass. |
| On their own | Radios, each with its consequence; a line under the label says what it covers and what stays with each app; waking's example shows at once under it (and the chosen one's). The chosen line has the shared selected fill. Pass; the legend repeats the title (weak). |
| Review | A Team tools line: "Off: they can't message yet." with what they give, the cost, "asked once" and a link to Settings › Agent access, and Turn on in the Change column; "On: …" once on. The leading App reads as one member. Pass. |

- Moved by round 2 (all 32 captures): the foot's and steps' words, the chips, the kinds row and side panel, the role reference, the labelled pickers, the radios on On their own, Review's Team tools line.

## Round 3 (AJ-E3011): one control per setting, both paths

New pool and the diagram now share every control and sentence (the lead control with its read-back, the team tools card, On their own, the summary); `paths-parity.test.tsx` fails if they drift. On these screens, dark and light:

- Who leads: the read-back is the lead control's own; the note "Once the pool is made, Start one now on its App's card starts that session." sits under it (only a made pool can start one). Pass.
- Everyone else: an App's role picker reads "Role: Lead: its next session leads" when its next session leads (chosen here or in Who leads); a wider group says why it can't lead, beside its picker. Pass.
- Who may message whom: the line above the table says what Needs you is ("the list of things waiting for your OK, on Home"). Pass.
- Review: the Team tools sentence, then the full team tools card (the diagram's own) in place of round 2's extra line. Pass.

## Round 4 (AJ-E3011): the team tools asked once, defaults count, what happens next, "takes charge"

Checked by eye, dark and light, empty and filled (the container's baselines and the evidence captures):

- setup-tools-empty (new): D-30's one question is New pool's own step 8, "Team tools", while it's unanswered (the empty path; the filled seed has answered, so it has 8 steps). The card is the pool page's own (one component): the question as its title, what they get, "It costs nothing extra…", "You can switch it off per project or app in Settings." with Settings as a link, then the filled primary "Yes, let them work together" and a secondary "Not now". The step's why-line says it's asked once. Pass, both themes; the card's 768 px column matches the other steps.
- Every empty step: the side list reads 1-9 with Team tools before Review; On their own's foot says "Next: Team tools".
- Who leads (filled) and Everyone else (filled): an App's next session "takes charge as soon as one of its sessions starts in storefront"; its role reads "Lead, once one of its sessions starts" with the badge "Lead when a session starts". Pass.
- Review: Team tools is one line (with Turn on while unanswered; "On: …" once answered), the card having its own step; under the sentences, "What happens next: Open a session in ai-juggler: it joins; their messages show here." from the shared sentences (the diagram's ready box says the same). Pass.
- Weak: Review filled at 1280×800 puts What each member can do and What happens next below the fold (the column scrolls; Make pool stays in the foot).
- Weak: while unanswered, Review's Team tools row offers Turn on right after the step that asked; both are the same action, said once each.

## Sam's round-4 follow-ups

- setup-tools-empty: the question's scope line reads "This is for every pool you make; you can switch it off for a project or an app in Settings." (was "You can switch it off per project or app in Settings."). Checked dark and light. Pass.

## Round 5 (AJ-E3011, Sam's diagram path and Casey's PC1 walk)

Checked by eye, dark and light:

- empty-pool-setup (new): an empty pool's page is New pool's own flow in place: "Set up Docs crew", the same steps and template cards inside the pool page's card, "Show the diagram" quiet at the top right, Next in the foot and no Cancel. Pass.
- setup-talk (empty and filled): "Click a cell to change it." above the table, and each cell is a button that looks like one (surface-2 with a hairline edge). Pass.
- setup-everyone: "New sessions in storefront join as:" with its picker. Pass.
- setup-tools-empty: the card says "It costs nothing extra. Your assistants use their own turns only when they act." and "This is a setting in AI Juggler; it changes nothing in your apps." Pass.
- Weak: the template grid's last row is cut at the empty pool's 800 px height (it scrolls inside the step).
