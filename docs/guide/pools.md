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
- **Agent access**: see below.
- **Edit pool**: rename it, set each session's role (Lead, Member or
  Observer), add or take out sessions, or delete it. Deleting a pool puts
  its sessions back in their projects' pools; nothing else changes.

## What agents may do

Agents can't reach pools yet; this is ready for when they can. You set it,
in the app only, per pool, each project's own pool too (on the pool's page,
and every pool at once in **Settings › Agent access**):

- **Read**: see who's here, what each one is doing, and the tasks.
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

## Your other Macs

**Include my paired Macs** on a pool's Agent access tab lets the pool span
your [paired Macs](fleet.md). It is off until you turn it on. Message and
task content will travel to your other Macs.
