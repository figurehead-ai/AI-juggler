# Pools

A pool is a group of sessions that work together: a planner and two
builders on "Checkout redesign", say. You make pools, name them, and say
who leads. Nothing else makes or changes a pool.

Putting a session in a pool, giving it a role or deleting the pool never
starts, stops or types into it.

## Each project's own pool

Every project has its own pool, named after it: **shop · all sessions**,
say. It holds every session of that project, on any account, that isn't
in another pool of yours, so you can let agents do more in one project
than in another. It is always there and can't be renamed (renaming the
project renames it) or deleted. Sessions with no project are in **No
project · all sessions**.

## Make a pool

1. Open a session's menu (right-click it in the list, or its **⋯**) and
   choose **Move to pool…**.
2. With no pool of yours yet, name the new one (for example "Checkout
   redesign"). You can keep it to one project: it then takes only that
   project's sessions.
3. **Make pool**. The session goes in, and the pool's page opens.

Once a pool of yours exists, the list's **View** (⇧V) offers **Group by ›
Pool** (greyed until then): Open now shows your sessions under their
pools, and a pool's **Open** goes to its page. The list keeps the grouping
you chose; making a pool doesn't change it. Home stays the whole desktop:
a pool's own view is its page.

## A pool's page

- **Members**: each session in the pool, as a row like Home's: its name and
  state (in the list's words: Asks you, Working, Idle, Ended), where it
  runs (project · app · profile), its tool, its role and when it was last
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

## What agents may do

Agents reach their pool through AI Juggler's agent tools, in the sessions
AI Juggler starts once you say yes. You set what they may do,
in the app only, per pool, each project's own pool too (on the pool's page,
and every pool at once in **Settings › Agent access**):

- **Read**: see who's here, what each one is doing and the tasks, and
  read their own messages and the pool's shared documents.
- **Work together** (where every pool starts): Read, and post messages,
  share documents, work on tasks and get a new session ready for you to
  start (it waits in Needs you for your click). A message for a session
  that's waiting goes in when you click.
- **Full**: Work together, and put messages straight into each other's
  sessions, up to the hop limit.
- **Custom**: you pick each action.

Anything above the level becomes a question for you in Needs you, never a
block: **Allow once**, **Always allow in this pool** (offered the second
time you allow the same thing) or **Deny**. Every "always allow" is listed
under the pool and in **Settings › Agent access › Rules you made**, and
**Take back** ends it from the next request.

**Messages put into sessions** sets how many replies in a row may go back
and forth before AI Juggler pauses and asks you (5 to start), and stops all
of them in this pool.

No level or rule lets an agent change a pool, a level or a rule, start a
session, choose an account, or see a password or key. AI Juggler never
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
- Each agent sends at most 10 messages a minute and 100 a day in a pool,
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

A pool keeps its work as tasks: you write them, and so can its agents,
with AI Juggler's agent tools. A pool's **Tasks** tab lists them, ready
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
  comes from, and its history: every change, by whom. You can change any of
  it: its status, who has it (**With**: a member, you, or no one), its
  links.
- **New task** adds one: its title and what it asks first; when it's done,
  its epic or story, what it waits on and an issue link under **More**. It
  starts ready for anyone (or to do).
- A session's own page has a **Tasks** tab once it has taken or worked one.

### What agents do with tasks

Agents check for work when they start and when they finish a task: they
list the pool's tasks, **take** a ready one (it's theirs, in progress),
say where it is (**in review** when it's ready for a look), or **give it
back** with a note for whoever takes it next. A member works the tasks it
holds and edits the ones it made that no one has; the pool's **lead** plans
epics and stories, gives tasks to others and closes anyone's. What a pool
at each level lets agents do is above; at **Work together** they add,
take and change tasks, and giving a task to someone else always asks you
first (Needs you: **Allow once**, **Always allow in this pool** from the
second time, or **Deny**). A task's words are its writer's: when an agent
wrote it, it says so, and they're never your instructions.

### A session for a task

Only you start sessions. **Start a session for it…** on a task asks which
launch settings (and so which account) to use, then starts a session in the
pool's project with the task as its first prompt; the task is then that
session's, in progress. Nothing is picked for you by how much an account
has left: the list is yours in name order, and only your project's own
default account, if you set one, is chosen to start with.

An agent can only **ask** for a session for a task. Its request waits in
Home's **Needs you**, with the same picker and **Start session**, or **Not
now**. Nothing starts by itself.

### A switch keeps the task

A task belongs to its holder's line of work, not to one account. When you
continue a session on another account or tool, the new session holds the
same tasks, sees them as its own and carries on; the project's **How this
work travelled** shows each session's task.

### The issue a task comes from

Paste a GitHub issue or pull request link, or a Linear one, into a task.
A Linear link shows its id (`ENG-123`). A GitHub one can show its title and
state (open, closed, merged): click **Look up its title**, and AI Juggler
asks GitHub's own command line, `gh`, with your own sign-in there (AI
Juggler never reads it). Offline, or without `gh`, the link still opens and
the task says it couldn't look it up. Nothing is looked up or synced by
itself.

## Your other Macs

**Include my paired Macs** on a pool's Agent access tab lets the pool span
your [paired Macs](fleet.md). It is off until you turn it on. Message and
task content will travel to your other Macs.
