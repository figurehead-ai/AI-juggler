# Pools

A pool is a group of sessions that work together: a planner and two
builders on "Checkout redesign", say. You make pools, name them, and say
who leads. Nothing else makes or changes a pool.

Putting a session in a pool, giving it a role or deleting the pool never
starts, stops or types into it.

## All sessions

Every session starts in **All sessions**. It is always there and can't be
renamed or deleted. When agents can see their pool (a later release), an
agent in All sessions sees only the sessions of its own project, whatever
the account.

## Make a pool

1. Open a session's menu (right-click it in the list, or its **⋯**) and
   choose **Move to pool…**.
2. With no pool of yours yet, name the new one (for example "Checkout
   redesign"). You can keep it to one project: it then takes only that
   project's sessions.
3. **Make pool**. The session goes in, and the pool opens.

Once a pool exists, the list can group by pool (**Filters › Group: Pool**;
the first pool you make switches to it), Filters gains a **Pool** picker,
and Home gets one too. A pool's heading in the list opens its page.

## A pool's page

- **Members**: each session in the pool, as a row like Home's: its name and
  project, its tool and account label, its role and when it was last active.
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
  its sessions back in All sessions; nothing else changes.

## What agents may do

Agents can't reach pools yet; this is ready for when they can. You set it,
in the app only, per pool (on the pool's page, and every pool at once in
**Settings › Agent access**):

- **Read**: see who's here, what each one is doing, and the tasks.
- **Work together** (where every pool starts): Read, and post messages,
  share documents and work on tasks. A message for a session that's waiting
  goes in when you click.
- **Full**: Work together, and put messages straight into each other's
  sessions, up to the hop limit.
- **Custom**: you pick each action.

Anything above the level becomes a question for you in Needs you, never a
block: **Allow once**, **Always allow in this pool** (offered the second
time you allow the same thing) or **Deny**. Every "always allow" is listed
under the pool, and **Take back** ends it from the next request.

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
