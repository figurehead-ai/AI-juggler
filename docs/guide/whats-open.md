# What's open, and getting to it

The list on the left shows what is really open on your Mac right now, in
every app, and gets you to any of it in one step.

## Places

A **place** is the app and the profile a session runs in, for example
**Claude app · Work**, **Terminal · Work** or **Codex app · Main**. A profile
is the setup an app runs on, such as a Claude Code folder or a Claude app
profile; see [Accounts and profiles](accounts.md). Sessions AI Juggler
started itself are a place of their own, **AI Juggler**.

The list has three parts:

- **Needs you:** sessions that ask you something, and anything else waiting
  for you.
- **Open now:** every session that is running, grouped by place. A place's
  header has **Open**, which brings that app forward. A place where nothing
  is happening folds into one line, such as "Work · 4 idle".
- **One line under them,** such as "Also: 3 ended today · 2 inactive · 1
  background · History ›". Each part shows those sessions. Nothing is
  removed; they're just not shown until you ask.

## What a row says

Each session shows:

- the app's icon, with the profile's badge on it;
- its title;
- where it is: project · app · profile · window or tab;
- its state, as a dot and a word: **Asks you**, **Working**, **Idle** or
  **Done (review it)**. Ended sessions are folded under the "Also:" line.

When a state is worked out rather than read from the app, it says
"probably".

## Go to it

Choose **Go to it**, or press Enter on a selected session. AI Juggler
brings that session's app and profile to the front, and then says what it
did, for example "Brought Claude · Work forward; pick the session in its
sidebar".

It brings the exact window or terminal tab forward only if you turned that
on in Settings › Desktop apps. Going to a session never asks for a macOS
permission by itself.

## View

**View** (⇧V) changes what the list shows. Its button has a dot when
anything differs from the defaults.

- **Group by:** Place (the default), Project, Pool or Account.
- **Show:** inactive, quiet, ended, background runs, sub-agents, your other
  Macs, archived.
- **Only:** one app, profile, account, project, pool or Mac.
- **Back to the defaults.**

## Last messages

A session's page shows its last two or three turns, so you can read what
it's asking before you go to it. They're read from the session's
transcript on this Mac when you open the page, with secrets hidden, and
kept nowhere. Claude app chats show their title only, because those
conversations aren't kept on your Mac.

## Settings › Desktop apps

- **Show sessions from:** one switch for each app (Claude app, Terminal,
  Codex app, Cursor), each saying what it found. AI Juggler reads which
  sessions each app has open from the apps' own files. It never types into
  them.
- **Going to a session:**
  - **Bring the exact window to the front** needs the Accessibility
    permission. **Set up…** walks you through it.
  - **Select the right terminal tab** needs the Automation permission. Off
    until you turn it on.
  - **Open AI Juggler from anywhere:** a keyboard shortcut you choose.
    There's none until you pick one. ⌥Space isn't accepted, because ChatGPT
    uses it.
- **Last messages:** on by default. Turn it off and session pages don't
  read transcripts.

AI Juggler never reads a running app's environment, key files or saved
sign-ins to find out what's open. [What AI Juggler touches](trust.md) has
the full list.
