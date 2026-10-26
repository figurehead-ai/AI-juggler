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
  - **Task**: the task it took. Tasks come in a later release, so this says
    "Unavailable" for now.
  - **Files changed**: the files it changed most recently.
  - What AI Juggler doesn't know says **Unavailable** and why. It never
    guesses. What a session says and changes is read from its transcript
    while it runs in AI Juggler.
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

## Your other Macs

**Include my paired Macs** on a pool's Agent access tab lets the pool span
your [paired Macs](fleet.md). It is off until you turn it on. Message and
task content will travel to your other Macs.
