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

**Pools › New pool** opens one guided setup, in order. Each step has a
heading and one line on why, and every choice reads back as a sentence
("New sessions in web join as Member and may Work together").

1. **How to start**: **Start from scratch**, or a template (Writer and
   reviewer, Feature team…) that fills in roles and who may message whom.
   The first time, a short intro says what a pool is. Each template's card
   says what it sets in plain words ("everyone may message", "members ask
   before messaging each other", "look only", "kept apart").
2. **Where**: its name and which projects. Sessions in these projects can
   join. A name and its projects already make a working pool: from the
   next step on, **Make pool** is there, and the rest is filled in
   (members join as Member, they may Work together, everyone may message
   everyone; an Observer only reads).
3. **Who's in**: whole apps (every session they start in these projects
   joins by itself, now and later) or single sessions open now; Accounts,
   Tools, Macs and everyone are under **More**. The side says who's in, in
   words: "2 apps, and every new session in web".
4. **Who leads**: one session leads (it plans the work and hands out
   tasks). Pick one to be in charge, or leave it for now: a session open
   now; or **[App]'s next session** ("Claude Code · Work takes charge as
   soon as one of its sessions starts in web"): until then nobody is in
   charge. On the pool's diagram that App is one card,
   "Its next session leads · its others join as Member", with **Start one
   now**: it opens New session on that App in the pool's project, and the
   session you start leads. An App's role picker offers the same: **Lead:
   its next session leads**. An Account, Tool, Mac or everyone can't lead,
   and says why beside its role. Or **No lead for now**.
5. **Everyone else**: **New sessions in web join as [Member]** (or don't
   join: only who's in), and each member's **Role** and what it may do
   **On its own** (Same as the pool, at first). What each role does:
   - **Lead**: plans the work and hands out tasks. One session.
   - **Member**: works on the tasks it's given, and messages the team.
   - **Reviewer**: reads everything, checks work in review and gives its
     verdict. Doesn't build.
   - **Observer**: watches and reads. Changes nothing.
6. **Who may message whom**: a table by role ("Lead can message…").
   **Allowed** needs nothing from you; **Ask first** waits for your OK in
   Needs you; **Blocked** stops it. Only the exceptions are in colour. One
   pair of sessions can differ: click the line between them on the diagram.
7. **What they may do on their own** (see below): how they work together
   through AI Juggler. What each may change in your files stays with its
   own app's settings. Then **Seal** if you want its work kept apart.
8. **Team tools**, only until you've answered it once: "Let the sessions
   in your pools message each other and share notes and tasks?", with
   **Yes, let them work together** or **Not now** (see
   [Team tools](#team-tools)).
9. **Review**: the whole pool in sentences, with what happens next in one
   line ("Open Claude Code · Work in web: it leads; Codex checks its work;
   their messages show here"), then **Make pool**.

A project's **Pools** tab opens its own pool ("web's own pool: every
session in web, as one team"), and **Move to pool…** on a session's menu
still puts one session in a pool of yours.

A pool's page shows it as a **Diagram** (who is in it, and who messages
whom) or a **List**. Its header's **On their own (everyone in this
pool)** is the same choice as the setup's. Beside the diagram, the pool
opens on a short summary in the Review's own sentences (who's in, who
leads, what they may do on their own, the team tools) and what waits for
you in **Needs you** (the list of things waiting for your OK, on Home).
**Set up this pool** keeps the setup's order below it, a tick on each
step done; a step left at its default counts as done ("default"). Once
who's in is set and the team tools are on, it says **This pool is
ready** and what happens next. Each step opens the same control as the
setup. The rest of the pool's settings are folds, closed at first, each
with a line on what it controls; the rarer ones (kept apart, your other
Macs, other pools its agents may write to) are under **More settings**.
Adding a member or changing a role shows at once, dimmed, until AI
Juggler has saved it; if it can't, it goes back and says why.

Once a pool of yours exists, the list's **View** (⇧V) offers **Group by ›
Pool** (greyed until then): Open now shows your sessions under their
pools, and a pool's **Open** goes to its page. The list keeps the grouping
you chose; making a pool doesn't change it. Home stays the whole desktop:
a pool's own view is its page.

## A pool's page

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
cost nothing extra: a session uses its own turns only when it acts.
Without them, a pool is set up, but nobody in it can message.

AI Juggler asks once, the first time you use pools: at the top of the
pool's page, as New pool's own step, and in Settings › Agent access, the
same question with **Yes, let them work together** and **Not now**. Once
you've answered, it doesn't ask again; you can switch them off per project
or app in **Settings › Agent access › Agent tools in sessions**. After
**Not now**, or with them off for the pool's project or app, the pool
says so in one quiet line with **Turn on** beside it.

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
example, and this section.

- **Add** first shows exactly what it writes: each file ("Creates
  ~/…/skills/ai-juggler/.claude-plugin/plugin.json") with its text under
  it. Nothing is written until you click **Add to Claude · Work**. It
  writes only AI Juggler's own folder in that profile's Claude Code
  settings: its pool tools and a note on new messages. Never a secret,
  never your Keychain, and nothing of yours changes.
- **After Add**: a running session picks it up at its next start; new
  sessions have it at once. AI Juggler doesn't restart a session for you.
- **Remove** shows exactly what it deletes first. If you changed one of
  its files since, Remove leaves that file and names it.
- **Add it yourself instead** shows the steps to install AI Juggler's
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
- **Custom**: you pick each action.

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

## Your other Macs

**Include my paired Macs** on a pool's Agent access tab lets the pool span
your [paired Macs](fleet.md). It is off until you turn it on. Message and
task content will travel to your other Macs.
