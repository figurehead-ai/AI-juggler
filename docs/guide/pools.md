# Pools

A pool is a team of your AI sessions working on the same projects: a lead
and two builders on "Checkout redesign", say, and a reviewer checking their
work.
They share messages, notes and tasks, and you approve what matters. You
make pools, name them, and say who leads. Nothing else makes or changes a
pool.

Putting a session in a pool, giving it a role or deleting the pool never
starts, stops or types into it.

## Each project's own pool

Every project has its own pool, named after it: **shop · all sessions**,
say. It holds every session of that project, in any app, that isn't
in another pool of yours, so you can let agents do more in one project
than in another. It is always there and can't be renamed (renaming the
project renames it) or deleted. Sessions with no project are in **No
project · all sessions**.

## Make a pool

**Pools › New pool** opens one guided setup. Its steps are listed at the
left in three groups, each with what it's set to under its name ("Who
leads · Claude's next session"). The form is in the middle. At the right,
a live preview draws the team as its diagram will look, with a row for
each choice (Where, Who's in, Lead, Roles, Messages, On their own);
it lights the part the step you're on sets. In a narrow window the
preview folds into a strip above the form. Every choice reads back as a
sentence ("New sessions in web join as Member and may Work together").

**The basics**

1. **Start**: **Start from scratch**, or a team that's already set up
   (Writer and reviewer, Feature team…) under **Or start from a
   template**, each card with a drawing of its team and what it sets in
   plain words ("everyone may message", "members ask before messaging
   each other", "look only", "kept apart"). The first time, the side says
   what a pool does.
2. **Where**: its name and its projects, each a card that says what's
   open in it now or when it was last worked in, then **Sessions with no
   project** and **Every project**. Only sessions in these projects can
   join.
3. **Who's in**: **Every new session in web** is a switch: on, any
   session you start there joins by itself, now and later. Under it, your
   apps as cards: each app's logo, **Desktop app** or **Terminal**, its
   profile, and what's open of it there now ("1 open in web"). Tick one
   and every session it starts there joins. A desktop app that still needs
   AI Juggler's connector says so in one line under the cards ("Claude ·
   Work needs AI Juggler's connector to message the team. You see the
   exact change before anything is written."), with **Preview and add**:
   one click shows exactly what AI Juggler would write, then **Add to
   Claude · Work** writes it (see [Desktop apps in a
   pool](#desktop-apps-in-a-pool)). Under them, the sessions open now that
   your choice brings in. **Sessions** picks single sessions open now;
   **More** adds a whole account, tool or Mac.

From Who's in on, **Make pool now** is beside Next: the rest is already
filled in (members join as Member, everyone may message everyone, they
may Work together; an Observer only reads), so a name, its projects and
who's in make a working pool.

**How they work** (marked *prefilled* until you reach it)

4. **Who leads**: the lead turns what you ask into tasks and hands them
   out; a drawing shows how, in three steps. Pick a card:
   **[App]'s next session** ("It takes charge as soon as one of its
   sessions starts in web"; until then nobody is in charge), a session
   open now, or **No lead for now**. On the pool's diagram that App is one
   card, "Lead · its next session in web", with **Start one now**: it
   opens New session on that App in the pool's project, and the session
   you start leads. An Account, Tool, Mac or everyone can't lead.
5. **Who builds, who checks**: the four roles, each in a line:
   - **Lead**: plans the work and hands out tasks. One session.
   - **Member**: works on the tasks it's given, and messages the team.
   - **Reviewer**: reads everything, checks work in review and gives its
     verdict. Doesn't build.
   - **Observer**: watches and reads. Changes nothing.

   Then each member is a row with its logo and its role: **Member**,
   **Reviewer** or **Observer** (who leads is the step before's; an App
   whose next session leads sets its other sessions' role here). Every
   new session's role is the same control. **One member works differently
   on its own** (optional) sets what one member may do without asking,
   when it differs from the pool.
6. **Who may message whom**: three ways, as cards: **Everyone talks
   freely**, **Members ask you first** (between members, each message
   waits for your OK) and **Members go through the lead** (members message
   only the lead and the reviewer). Under them, **Each pair**: a table by
   role ("Lead can message…"); click a cell to change it. **Allowed**
   needs nothing from you; **Ask first** waits for your OK in Needs you;
   **Blocked** stops it. A table changed by hand is **Your own mix**. One
   pair of sessions can differ: click the line between them on the
   diagram.
7. **On their own** (see [What agents may do](#what-agents-may-do)):
   **Look only**, **Work together**, or **Work together and wake each
   other**, each a card with a drawing. It's how they work together
   through AI Juggler; what each may change in your files stays with its
   own app's settings. Under **Optional**, **Keep this pool's work
   apart** seals it.

**Finish**

8. **Team tools**, only until you've answered it once: a drawing of two of
   your team's apps talking through AI Juggler, what it gives, what it
   costs (nothing extra) and what it changes, then the two answers as
   cards: **Yes, let them work together** or **Not now** (see [Team
   tools](#team-tools)).
9. **Review**: first whether it can work: if something it needs can't
   (the team tools can't run here, or an app's connector isn't in it),
   **Before it can work** says so. Then the whole pool in sentences, each
   with its **Change**, and **What happens next** in three cards: the pool
   is made, which session to open first ("Open Claude Code · Work in web
   first: it leads"), and what follows ("Codex checks its work; their
   messages show here"). Beside them, what to tell the
   lead first, with **Copy** ("Plan the work in web, hand out tasks, and
   ask Codex to review."). **Make pool** makes it and opens its page. A
   member you chose that the pool doesn't keep is named after, with why.

The new pool opens on its diagram, still being set up (see [A pool's
page](#a-pools-page)): before any session joins, the diagram draws the
team you planned, and the panel at the right is its setup checklist, each
step with the same value as the setup's steps and a **Change**. **Add a
member** at the diagram's top left adds more. **Hide setup** at the
panel's foot switches to the pool's live view; the checklist steps aside
by itself once a session has joined and every step is done.

A project's **Pools** tab opens its own pool ("web's own pool: every
session in web, as one team"), and **Move to pool…** on a session's menu
still puts one session in a pool of yours.

A pool's page shows it as a **Diagram** (who is in it, and who messages
whom) or a **List**. Its header's **On their own (everyone in this
pool)** is the same choice as the setup's. **All of this pool's
settings**, at the foot of its setup checklist, opens a short summary in the
Review's own sentences (who's in, who leads, what they may do on their
own, the team tools) and what waits for you in **Needs you** (the list
of things waiting for your OK, on Home), with the setup's checklist in
one line while it isn't done. A step left at its default counts as done
("default"). Once who's in is set and the team tools are on, it says
**This pool is ready** and what happens next. Each step opens the same
control as the setup. The rest of the pool's settings are folds, closed
at first, each with a line on what it controls; the rarer ones (kept
apart, your other Macs, other pools its agents may write to) are under
**More settings**.
Adding a member or changing a role shows at once, dimmed, until AI
Juggler has saved it; if it can't, it goes back and says why.

Once a pool of yours exists, the list's **View** (⇧V) offers **Group by ›
Pool** (greyed until then): Open now shows your sessions under their
pools, and a pool's **Open** goes to its page. The list keeps the grouping
you chose; making a pool doesn't change it. Home stays the whole desktop:
a pool's own view is its page.

## A pool's page

From a project (its **New pool for web…**, at the top of its Pools tab, or
its own pool), Start from scratch is the open path and the templates wait
under **Or start from a template**.

A pool with no members yet opens in its setup: New pool's own steps, on
the pool's page, with its projects (and name) filled in. A project's own
pool keeps its project: every session in it is part of the team, and
**Who builds, who checks** sets the role they join as. **Show the diagram** leaves
the setup; once the pool has members, it opens on its diagram.

While a pool is being set up, its header says **Setting up · 5 of 7** and
the panel at the right is its checklist: a ring with how far it is, then
each step (Where, Who's in, Who leads, Roles, Messages, On their own, and
**Let them message each other**, the team tools) with what it's set to
and a **Change**. The first step not done is open, with what it does and
the one button that does it. Before any session joins, the diagram draws
the planned team: the lead on top, the other members under it, each card
opening in the panel. **Add a member** at the diagram's top left lists
the sessions open now and the Apps and groups that can join.

Once a session has joined and every step is done, or when you pick
**Hide setup**, the panel is the pool's live view: what's happening now,
its tasks, its lead, what they may do on their own, its messages this
week and Live; the diagram has no **Add a member** then. **Edit setup**
at the panel's foot brings the checklist (and **Add a member**) back; a
hidden setup that isn't done says how far it got above it.

On the diagram, the panel at the right follows what you select. Drag its
left edge (or focus it and use ← →) to make it wider, 360 to 420 px; the
width is kept. A **Change** or a checklist item opens its section at the
top of the panel, its control ready. Every card on the diagram is a
button: click it, press Enter on it, or use VoiceOver's press. A
session's card leads with its own title, else its task, else its folder
numbered where two read alike ("web (12)"), with when it started under it.
A member's own choice of what it may do on its own in this pool is near
the top of its panel, and in the List view behind its row's arrow.

- **Members**: each session in the pool, as a row like Home's: its name and
  state (in the list's words: Asks you, Working, Idle, Ended), where it
  runs (its project and app), its tool, its role and when it was last
  active.
  The arrow on a row shows more:
  - **It says**: the step the agent's own task list has in progress, in its
    words and marked as said by the agent.
  - **Task**: the task it's on (in progress, in review or blocked), with
    its id, or **None**. See [Tasks](#tasks) below.
  - **Files changed**: the files it changed most recently.
  - **On its own**: what it may do on its own in this pool, the same choice
    as its panel's.
  - What AI Juggler doesn't know says **Unavailable** and why. It never
    guesses. What a session says and changes is read from its transcript
    while it runs in AI Juggler.
- **Tasks**: the pool's work, ready work first, and its plan; see
  [Tasks](#tasks) below.
- **Messages**: the pool's threads and shared documents; see
  [Messages](#messages) below.
- **Agent access**: see below.
- **Edit pool**: rename it, set each session's role (Lead, Member or
  Observer), add or take out sessions, or delete it. Deleting a pool puts
  its sessions back in their projects' pools; nothing else changes.

## Team tools

The pool's members work as a team through AI Juggler's team tools: they
message each other and share notes and tasks (Claude sees that Codex is
reviewing, say, and sends it a note). They cover the sessions AI Juggler
starts or connects in your pools, never a session outside a pool, and
cost nothing extra: your assistants use their own turns only when they
act. It's a setting in AI Juggler and changes nothing in your apps.
Without them, a pool is set up, but nobody in it can message.

It's for every pool you make. AI Juggler asks once, the first time you
use pools: as New pool's own step and in Settings › Agent access (**Yes,
let them work together** or **Not now**), and as the last item of a
pool's setup checklist (**Turn on team tools** or **Not now**). Once
you've answered, it doesn't ask again; you can switch them off per project
or app in **Settings › Agent access › Agent tools in sessions**. After
**Not now**, or with them off for the pool's project or app, the pool
says so in its setup checklist, with **Turn on team tools**.

## Desktop apps in a pool

A session in the Claude app's Code tab is a pool member like a terminal
one: it reads and sends the pool's messages, shared notes and tasks, under
the pool's rules. It does that through **AI Juggler's connector**, which
you add to that app profile once, with your OK.

When a Claude app profile, or one of its sessions, is in a pool without
the connector, its card says **Watched: its app has no AI Juggler
connector**, with **Add**. Watched means shown and counted, without the
pool's tools. **Add** opens one card: "Add AI Juggler's connector to
Claude · Work so its sessions can talk in this pool", what it does, an
example, and this section. In New pool's Who's in, a desktop app you
tick says it in one line, with **Preview and add**, which opens straight
on what Add would write.

- **Add** first says in one sentence what it does ("This adds a small
  helper to your Claude app (Work) so its sessions can talk in this pool:
  2 new files in its own settings folder; nothing of yours changes."),
  with each file and its exact text under **Show the files**. Nothing is
  written until you click **Add to Claude · Work**. It
  writes only AI Juggler's own folder in that profile's Claude Code
  settings: its pool tools and a note on new messages. Never a secret,
  never your Keychain, and nothing of yours changes.
- **After Add**: a running session picks it up at its next start; new
  sessions have it at once. AI Juggler doesn't restart a session for you.
- **Remove** shows exactly what it deletes first. If you changed one of
  its files since, Remove leaves that file and names it.
- **Show me how to add it by hand** shows the steps to install AI Juggler's
  plugin from the app (**+ › Add plugins**), with a **Copy** button for
  the command. While the steps are open, AI Juggler watches for it and
  says when it's done: the connector is there, or one of the app's
  sessions called AI Juggler.

Once connected, a member's card says how a new message reaches it:
**Wakes on new messages** (the pool lets its members be woken: at Full,
or a rule you made) or **Reads messages on its next turn**. AI Juggler
never types into the app's window.

The ChatGPT Codex app can't take the connector yet (AI Juggler can't tell
which of its threads is calling), and its threads can't be woken: they
stay Watched, and their cards say so.

To carry on a Watched session in AI Juggler instead, use **Continue as
CLI** in its card's menu: a new process here resumes it, with the pool's
tools.

## What agents may do

Agents reach their pool through AI Juggler's agent tools, in the sessions
AI Juggler starts once you say yes. You set **what these sessions may do
on their own**, in the app only, per pool, each project's own pool too (on
the pool's page, and every pool at once in **Settings › Agent access**):

- **Look only**: they see the team, its cards, tasks and their own
  messages. Anything more waits for your OK in Needs you.
- **Work together** (where every pool starts): they message each other and
  share notes and tasks, and get a new session ready for you to start (it
  waits in Needs you for your click). Anything more waits for your OK in
  Needs you. A message reaches a session on its next turn.
- **Work together and wake each other**: as Work together, and a message
  wakes an idle session so the work keeps going, up to the hop limit. Each
  wake starts a turn on your subscription. For example, Codex finishes a
  review, and its note wakes Claude's idle session to carry on, without
  you.
- **Custom**: you pick each action. Chosen in the diagram's header, its
  list opens right there (the same list as the pool's settings), and the
  pool reads back "On their own they may do the 11 things you picked".

A member can follow the pool (**Same as the pool**, the default) or have
its own choice. The diagram says, per member, whether it **wakes on new
messages** or **reads messages on its next turn**; a Codex app thread
**can't be woken**.

Anything above that becomes a question for you in Needs you (the list of
things waiting for your OK, on Home), never a block: **Allow once**,
**Always allow in this pool** (offered the second time you allow the same
thing) or **Deny**. Every "always allow" is listed under the pool's
**Allowed without asking** and in **Settings › Agent access › Rules you
made**, and **Take back** ends it from the next request.

**Messages into sessions** says how a message reaches a session that is
working (by itself between its turns, or after your OK in Needs you), and
**Stop and ask you after** sets how many messages in a row between agents
go back and forth before AI Juggler stops and asks you (5 to start).
**Keep every message out of sessions** stops all of them in this pool.

No choice or rule lets an agent change a pool, what it may do, or a rule, start a
session, choose an app, or see a password or key. AI Juggler never
answers a tool's own permission questions for you.

## Messages

Agents in a pool can write to each other and to you, and you can write to
them. A pool's **Messages** tab shows its threads, newest first, and the
one you open as a conversation: each agent's message on the left, yours on
the right, with its kind (a handoff, a question, a review request…) and
where it is for each recipient: waiting in its inbox, read, handled, gone
in at its next turn. A session's own page has a **Messages** tab too once
it has any.

- **A message waits in its recipient's inbox.** The agent reads it when it
  chooses (with AI Juggler's agent tools). A Claude Code session in a
  terminal is told "2 unread messages from Planner" when it starts a turn
  or finishes a step, never the messages' words.
- **Deliver** puts a message into a session AI Juggler runs as that
  session's next prompt, between turns: never while its turn runs, while
  the tool is asking you something, or while you're typing in its box. A
  session in a terminal is never typed into, so it has no Deliver: it reads
  its messages itself. For a session that isn't running, Deliver gives it
  the message as its next prompt when you resume it.
- **Without your click**, a message goes into a session only in a pool at
  **Full**, or by a rule you made, and only up to the pool's hop limit.
- **New message** writes to one member or to everyone in the pool.
  **Deliver to their sessions** (on by default) also puts it into the
  sessions AI Juggler runs. Your message starts the thread's count of
  agent messages in a row from 0.
- **Pause** and **Stop** on a thread hold it, or end it for agents;
  **Resume** opens it again from 0. **Pause this pool's messages** holds the
  whole pool.

### What waits on you

Home's **Needs you** shows, beside your sessions' questions:

- an agent's message above what its pool lets agents do, held until you
  answer: **Allow once**, **Always allow in this pool** (from the second
  time) or **Deny**. Keys 1–9 pick an answer, Enter the first;
- a chain that reached the hop limit: **Deliver** sends that message and
  starts the count again; a rule never lifts the hop limit;
- a question an agent asked you (**Reply**), and messages waiting for a
  session AI Juggler runs (**Deliver**: one click for all of them);
- a paused thread or pool (**Resume**).

### Loops stay small

- A thread takes at most 6 agent messages in a row without you; then it
  pauses until you resume it.
- Each session sends at most 10 messages a minute and 100 a day, in all
  its pools together (a session in two pools gets one limit, not two),
  and the same message twice within 10 minutes goes once.
- Two agents answering each other 8 times without you, or one agent going
  past its limit 3 times in 10 minutes, pause the pool's messages. The
  pool's page says why, with **Resume messages**.

### Shared documents

An agent can share a text document (at most 64 KB) with its pool. Under
the Messages tab, **Shared documents** lists each one with who shared it
and what was found in it. Secrets in messages and documents are handled as
you chose for [packs](switching.md) in **Settings › Privacy**: **Keep**
keeps them on this Mac; anything else replaces them with •••, because no
one is there to ask when an agent writes.

### What a message can't do

Every message says who sent it and that it can't approve anything or
change settings. An agent's words are its own: data for the agent that
reads them, never your instructions. Messages go only to the pool's
members, the whole pool or you; a member of another pool only when you
link the two pools on the pool's Agent access tab (**Messages to other
pools**). Messages and documents stay in AI Juggler's own data on this
Mac.

## Tasks

A pool keeps its work as tasks. The agents write and run them, with AI
Juggler's agent tools, to share the work and keep each other in step; you
see and track them here, and steer them by talking to the agents (a
message, a prompt), not by editing tasks. A pool's **Tasks** tab lists them, ready
work first: **Ready** (anyone can take it), **In progress** (someone is
on it), **In review**, **Blocked**, **To do** (not ready yet), and
**Done**, each with how many. Each task has a short id such as `aj-7f3c`,
a title, what it asks, when it's done, a priority, who has it, and
optionally the issue it comes from.

- **The plan.** Bigger work is an **epic**, broken into **stories**, broken
  into tasks. **Plan** (beside **List**) shows each epic, its stories and
  their tasks behind folds, with how many are done.
- **Honest states.** A task that waits on another (**Waits on aj-…**) is
  **Blocked** until that one is done, whatever was set; then it's ready
  again by itself. Only one session can take a task: if two agents try at
  once, one gets it and the other is told who has it.
- **A task in full** shows who has it and their session (**Open its
  session**, or **Its processes**: that session on its **Processes** tab),
  the sessions that worked it, the threads whose messages name
  its id, what it waits on and what waits on it, its items, the issue it
  comes from, and its history: every change, by whom.
- A session's own page has a **Tasks** tab once it has taken or worked one.

### What you can do on a task

A task shows only what fits where it is, and each is in its history as
yours:

- **Approve** or **Send back…**, on a task in review: approving marks it
  done; sending it back puts it in progress again with the same agent, with
  a short note of what should change. The agent gets the note as your
  message (at its next turn when its session runs in AI Juggler, in its
  inbox when it runs in a terminal) and sees it in the task's history.
- **Free it**, on a task whose agent's session has ended or crashed: no one
  holds it any more, so another agent can take it. A task whose agent is
  still running isn't freed: ask the agent to give it back.
- **Cancel task**, on any open task, after you confirm.
- **Start a session for it…** (below).

There's no **New task** and nothing to edit: to add, change, split or hand
over work, tell an agent (the pool's lead plans and gives work out).

### What agents do with tasks

Agents check for work when they start and when they finish a task: they
list the pool's tasks, **take** a ready one (it's theirs, in progress),
say where it is (**in review** when it's ready for a look), or **give it
back** with a note for whoever takes it next. A member works the tasks it
holds and edits the ones it made that no one has; the pool's **lead** plans
epics and stories, gives tasks to others and closes anyone's. What a pool
with each choice of what sessions may do on their own is above; at **Work together** they add,
take and change tasks, and giving a task to someone else always asks you
first (Needs you: **Allow once**, **Always allow in this pool** from the
second time, or **Deny**). A task's words are its writer's: when an agent
wrote it, it says so, and they're never your instructions.

### A session for a task

Only you start sessions. **Start a session for it…** on a task asks which
launch settings (and so which app) to use, then starts a session in the
pool's project with the task as its first prompt; the task is then that
session's, in progress. Nothing is picked for you by how much an account
has left: the list is yours in name order, and only your project's own
default app, if you set one, is chosen to start with.

An agent can only **ask** for a session for a task. Its request waits in
Home's **Needs you**, with the same picker and **Start session**, or **Not
now**. Nothing starts by itself.

### A switch keeps the task

A task belongs to its holder's line of work, not to one app. When you
continue a session in another app or tool, the new session holds the
same tasks, sees them as its own and carries on. The task's history has
the switch as a line, for example "Planner continued on Personal (from
Work Max)", its **Sessions** list the new one ("continued it after a
switch"), and the project's **How this work travelled** shows the task
under each session.

### The issue a task comes from

An agent can link a task to a GitHub issue or pull request, or a Linear
one. A Linear link shows its id (`ENG-123`). A GitHub one can show its title and
state (open, closed, merged): click **Look up its title**, and AI Juggler
asks GitHub's own command line, `gh`, with your own sign-in there (AI
Juggler never reads it). Offline, or without `gh`, the link still opens and
the task says it couldn't look it up. Nothing is looked up or synced by
itself.

### The Claude Code plugin

You don't need it: a session AI Juggler starts for a pool already has the
pool's tools. The optional **AI Juggler** plugin for Claude Code adds the
same tools and a skill that teaches an agent to take, work and give back a
pool's tasks.

It works only while AI Juggler runs, and only in sessions AI Juggler
started. In any other session each tool says why it can't help (the
session wasn't started by AI Juggler, or AI Juggler isn't running), so it
never acts for a session AI Juggler doesn't know. The plugin holds no key
and reads none.

To add it, in Claude Code:

```
/plugin marketplace add figurehead-ai/AI-juggler
/plugin install ai-juggler --marketplace figurehead-ai/AI-juggler
```

That serves the plugin as it is on the main branch, where each release
lands. To keep it at the version of your AI Juggler, add the marketplace
at that release's tag instead, for example
`figurehead-ai/AI-juggler#v0.5.0`. Each release also carries the plugin
as a zip (`AI-Juggler-Claude-Code-plugin-X.Y.Z.zip`, in the release's
checksums), which `claude --plugin-dir` loads for one session. To remove
it, `/plugin uninstall ai-juggler`.

## Your other Macs

**Include my paired Macs** on a pool's Agent access tab lets the pool span
your [paired Macs](fleet.md). It is off until you turn it on. Message and
task content will travel to your other Macs.
