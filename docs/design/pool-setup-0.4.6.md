# Pool setup a newcomer understands (0.4.6: AJ-E3011, AJ-E3012)

The design for 0.4.6's pool setup and canvas selection. Sources: the maintainer's findings on 0.4.5's setup (2026-11-14, via the coordinating session), the newcomer walks before the change (`docs/qa/evidence/AJ-E3011/walks/before-run*.md`), the calm UI round 2 proposal (its levels and §c/§c2 tokens, names agreed with the 0.5 session), D-28, D-62, D-92, D-116 and D-119.

## Who we design for

Someone who has never heard of pools, agents, roles, leads, reviewers, observers or permission levels. They use two or three coding assistants and want them to "work as a team: one in charge, one checking, and I decide what they may do without asking me". Every screen is judged by one question, for every control on it: **"Do I know what this does and who it affects?"** Any "no" is a design bug.

## What the walks found (before)

All runs failed (F): ease 3/7, value 3/7, feeling −1. The same five walls each time:

1. **The lead is a dead end.** With Apps in the pool and no session open, "Lead is one session; pick one of this App's sessions to lead" offers nothing to pick (D-92 forbids an App leading).
2. **What members may do on their own is unexplained.** "The pool's (Work together) / Read / Work together / Full" isn't tied to "asking me", and it's unclear whose setting it is.
3. **Roles are unexplained.** Member / Reviewer / Observer is a bare segmented control.
4. **The agent-tools banner** ("Let agents … see each other and work together? … Asked once, for every pool") is a wall of text above the work, read as a global switch, and left unanswered.
5. **The order disappears.** The teaching card's three steps vanish once members are added (or sit behind a card), and talk rules were never found.

Also: the manual path is a small grey link; "Everyone in this pool's projects" doesn't say who that is; a project's own pool ("web · all sessions") isn't told apart from one you made; the "Add a project" list's first row repeats its label; the inspector is headed "Member rule", not by what is selected; selection is a dashed border.

## The design

### 1. One flow, in order, from either door

"New pool" and a project's pool open the same guided setup. The Template step keeps the templates, and **"Start from scratch"** becomes a full card beside them (it replaces "Blank" and the small "Set up by hand on the canvas" link). The steps, each with a heading and one line on why:

1. **Where:** which projects. *Why: sessions in these projects can join.*
2. **Who's in:** Apps (every session they start here joins) and sessions open now. *Why: only these work together.*
3. **Who leads:** an inline picker (below). *Why: the lead plans and hands out the work.*
4. **Everyone else:** "New sessions in these projects join as: [Member ▾]" (the old "Everyone in this pool's projects" row), and each member's role on a role card.
5. **Who may message whom:** the talk table (below).
6. **What they may do on their own** (below), then seal, if wanted.
7. **Review:** the whole pool read back in sentences, then **Make pool**.

**Defaults make a working pool from a name and its projects.** From step 2 on, **Make pool** is enabled; everything later is prefilled (members join as Member; Work together; everyone may message everyone) and says so. The canvas keeps the same order: its checklist stays as a side card with ticks until every step is done (it never vanishes when members are added and never sits under a card), and each step opens the same control as the setup.

**A short intro** at the top of the first pool's setup, in 2–3 lines with a small diagram (a lead handing work to members, a reviewer checking it): what a pool is ("a team of your sessions on the same projects"), why ("they share messages, notes and tasks, and you approve what matters"), and what happens after you make it. It is full on the first pool and collapsed to one line afterwards (a chevron opens it).

### 2. Every control says what it sets and for whom

A control never stands alone. It has a visible label first, then the choice reads back as a sentence: "New sessions in web join as **Member** and may **Work together**." A control that can't be explained in one line is the wrong control. Every term has plain words, or an info icon whose popover gives one line and an example, plus a link to `docs/guide/pools.md`. Serious choices (seal, Full, the agent tools) give the consequence and an example, per the serious-choices rule.

### 3. Who leads: never a dead end

D-92 stands: one session leads. The picker offers, in this order:
- sessions open now in the pool's projects (most recent first);
- **"[App]'s next session"**, one row per App in the pool: "Claude Code · Work: its next session here leads". This is a new pool setting, `lead_next_from` (an App). When a session of that App joins and the pool has no Lead, it joins as a named session member with Lead, and the setting clears. It shows on the canvas as a placeholder card: "Lead: Claude Code · Work's next session";
- "No lead for now": the pool works without one, and the canvas says what a lead would add.

### 4. Roles as cards

Each role is a small card with an icon, one line on what it may do, and who holds it now:
- **Lead:** plans the work and hands out tasks; one session.
- **Member:** works on the tasks it's given, and messages the team.
- **Reviewer:** reads everything, checks work in review and gives its verdict; doesn't build.
- **Observer:** watches and reads; changes nothing.

A member's role picker shows the same lines.

### 5. What sessions may do on their own (the old "level")

The word "Level" goes. The label is **"What these sessions may do on their own"**, a labelled segmented control with a one-line consequence and an example under the chosen option:
- **Look only** (was Read): "They see the team, its cards, tasks and their own messages. Anything more waits for your OK in Needs you."
- **Work together** (was Work together): "They message each other and share notes and tasks. Anything more waits for your OK in Needs you."
- **Work together and wake each other** (was Full): "As Work together, and a message wakes an idle session so the work keeps going. Each wake starts a turn on your subscription."

The maintainer's example word "Anything" isn't used for the third option: Full doesn't allow anything, its difference is waking (D-28, D-119), and the label must say what it does. A member's own choice offers **"Same as the pool"** first. Per member, the canvas says "wakes on new messages" or "reads messages on its next turn" (D-119); a Codex app thread says "can't be woken".

### 6. Who may message whom

A table with plain row and column labels ("Lead can message…", no "From ↓ · To →"). **"Allowed" is calm**: muted text, no fill. Colour appears only for **"Ask first"** (L2: the amber word and dot) and **"Blocked"** (L3: red with a glyph), so an exception stands out at once. One line above it says what it does; one line below says how to set it for one pair (the canvas line).

### 7. The agent-tools consent where it applies

The banner becomes a compact card in the flow, shown the first time a pool is made. It reads: "Let AI Juggler give these sessions the pool's tools: messages, shared notes and tasks." Below that sits its consequence and an example ("Claude can see that Codex is reviewing and send it a note"), and that it is one switch for every pool, with Settings › Agent access named. **Turn on** and **Not now** follow; "Not now" says what won't work until it's on.

### 8. Calm, per the round 2 rules

The pool screens are the first built calm. The levels are L3 Broken (red, a glyph, one tinted block), L2 Needs you (an amber word and dot, no fill), L1 Note (a quiet glyph) and L0 Calm (the grey hierarchy). Section labels are small tracked caps. Surface tone replaces card borders, and there are no row dividers. Controls are filled, secondary buttons are ghosts, and app icons are muted at rest and in colour when they need you, are selected or are hovered. Every collapsible stays, with its chevron restyled calm. The tokens use 0.5's agreed names (`--color-section`, `--color-fill`, `--tone-saturate`, `--tone-opacity`, `--color-divider`, `--color-count`/`-ink`, `--color-count-alarm`/`-ink`, `--level-3`, `--level-2`, `--level-1-done`, `--level-1-live`) with the proposal's values; `attentionLevel()` goes in `ui/src/lib/attention.ts`. No other token is added without the 0.5 session. One spacing scale throughout.

### 9. The canvas's selection and inspector (AJ-E3012)

- The inspector always leads with the selected object's own title, its kind and app as a subtitle ("Jive CLI" / "Session · Claude Code · Member"). That covers a session, an App, an Account, everyone in scope, a line, and the pool itself when nothing is selected. Status is a pill under the title, never the heading.
- One selection treatment: a solid 2 px accent ring with an offset, and a raised surface. Dashes never mean selection. A broad member (an App, everyone in scope) is drawn as a stacked card, not a dashed one. The inspector's title edge carries the same accent. Keyboard focus is a separate, visible ring.
- Cards don't truncate the App's name where room allows (two lines before an ellipsis); the full name is in the inspector's title.

### 10. Smaller fixes

- A project's pool says what it is on the project's Pools tab: "web's own pool: every session in web, as one team".
- The "Add a project" list loses its placeholder row.

## Done means

- Two newcomer walks after the build, scored the same way. Every screen answers the key question "yes"; any "no" is redesigned and walked again.
- The design-quality critique; vitest per control; a11y (labels, aria-selected and aria-current on cards, a heading naming the selection); gallery captures of every step, empty and filled, light and dark.
- Before and after screenshots with notes saying what changed, for the review page.
- P10 re-walked; the AJ-E3304 pool scenarios that cross these screens re-walked.

## Round 2 (after the first two walks after the build)

Both after-walks reached S with no detour (ease 5/7, value 5/7, feeling +1; before: F, F, F at 3–4/7 and −1). The bar for done is 6/7 or better on ease and value from two walks, and no "no" on the key question. Every friction point either run logged, with its fix:

1. **The team can't talk until "Turn on", and nobody turned it on with confidence** (run 2 left it off: "one switch for every pool", who it affects and what it costs unclear). The card becomes "Let your assistants work as a team": what it gives (messages to each other, shared notes and tasks), who it covers (the sessions AI Juggler starts or connects in any of your pools, never a session outside a pool), what it costs (nothing extra: a session uses its own turns only when it acts), what happens without it ("the pool is set up, but nobody in it can message"), an example, and where to switch it off (Settings › Agent access). The button says "Turn on team tools". Review shows it as a line ("Team tools: off — they can't message yet · Turn on") so it is decided before Make pool.
2. **Claude shown twice** (a Lead placeholder card and the App as Member), and its row in Everyone else said Member. An App whose next session leads is one card: its role line says "Its next session leads · its others join as Member"; its row in Everyone else says the same with the picker for "its others".
3. **"Next session" read as "not in charge now".** The lead line says "Claude Code · Work leads from its next session in web (none is open now)", with **Start one now**: it starts that App's session in the pool's project (the usual New session, prefilled), which then leads.
4. **"On their own" doesn't say what it leaves out.** One line under it: "This is how they work together through AI Juggler. What each may change in your files stays with its own app's settings."
5. **"Wake each other" half understood.** Its example shows at once under the option (not behind "What does this mean?"): "Codex finishes a review; its note wakes Claude's idle session to carry on, without you. Each wake uses a turn on your subscription."
6. **Who's in counted "3" for two assistants** (it counted "Everyone in web" silently). The panel says "2 apps, and every new session in web", with "Everyone in web" as its own line saying what it is.
7. **"Can act in the pool: 0 full, 0 shell only, 0 watched" is jargon.** Not shown in the setup. Where it is kept (the canvas list), in words: "open now: none — sessions join as they start".
8. **The kinds tabs (Apps / Sessions / Accounts / Tools / Macs / Everyone) are unexplained.** One line above them ("Add whole apps — every session they start here joins — or single sessions open now"); Apps and Sessions first, the rarer kinds under "More".
9. **The role cards look like buttons and aren't.** They become a plain reference ("What each role does"), styled as text with an icon, no hover or button edge.
10. **A member's second picker "Same as the pool (Work together)" has no label.** Its label is "On its own:".
11. **The template cards' chips use the old words** ("talk: allowed", "level: Read", "member↔member: ask first", "sealed"). In plain words: "everyone may message", "look only", "members ask before messaging each other", "kept apart".
12. **"Pool"** isn't known: the Pools page says "Teams of your AI sessions" under its title, and the intro's first line defines it.
13. **"Canvas"** read as "a drawing area?": the tab is named **Diagram** (Diagram | List).
14. **The project page's "agents: Work together"** reads "On their own: Work together", as everywhere else.
15. **"none open now · new ones join"** reads "No session open yet · new ones in web join".
16. **The "Subagents" tab and "Settings › Agent access"** were not understood: say what each is where it appears (a one-line hint or tooltip with words, not a bare name).
17. **The role menu clipped at 1024×768** (seen once; at 1280×800 it fits): menus near the window edge flip or shift inside the window at every size from 1024×680.
18. **"No lead for now" preselected** while Apps are in: the step's why-line says "Pick one to be in charge, or leave it for now", so leaving it is a visible choice.

## Round 5 (after Sam on the bundled build, SD 4/7), consistent with D-55

D-55 (the maintainer's ruling: "A pool is one live canvas you watch and edit in place") and the view-and-edit-in-place rule stay in force. Round 5 keeps them:

1. **An empty pool (no members) opens the guided setup.** It is the same component as New pool's steps, prefilled with the pool's projects (and its name, for a project's own pool), with "Show the diagram". This is onboarding, the one flow from both doors. While the pool is still empty, a checklist item may open its step full width.
2. **Once the pool has members, everything is edited in place on the diagram.** The checklist and the settings never send the user away from it. The side panel is fixed instead:
   - the inspector is wider and resizable, about 360–420 px (default about 380), its width remembered per viewer;
   - the control being edited is scrolled into view and focused (a checklist item or a header link opens the panel at that section);
   - the panel's sections are in setup order: who's in, who leads, everyone else, who may message whom, on their own, then more settings;
   - "Who leads" and the role menu are never off-screen: at 1024×768 and 1280×800 the opened section's control is fully visible and its menu opens inside the window (tested).
3. **No layout shift on Add**, measured by a test.
4. **A project's own pool draws no "Everyone in web" card.** Its scope is said in the header ("Every session in web is part of this team"), and its join role in Everyone else.
5. **The question card's cost line:** "It costs nothing extra. Your assistants use their own turns only when they act."
