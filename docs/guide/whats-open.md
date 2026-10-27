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
  for you. It's a shortcut: each of them also stays in its place under Open
  now, its state in amber.
- **Open now:** every session that is running, grouped by place. A place's
  header has **Open**, which brings that app forward. Click a place's
  chevron to fold it; it stays folded until you open it again.
- **One line under them,** such as "Also: 3 ended today · 2 inactive · 1
  background · History ›". Each part shows those sessions, and the list
  scrolls to them. Nothing is removed; they're just not shown until you
  ask.

Click **Needs you** or **Open now** to fold that part, and again to open
it.

## Keeping the order

Open now stays put. A session keeps its place in the list while it works,
waits or asks you something; only its dot and word change. A new session
joins the end of its place, and one that ends leaves a gap the rows under
it close. A place with more than 8 sessions shows the first 8, then
"+n more".

- **Pin** a session to keep it at the top of its place: point at it and
  click its pin, or use **Pin to top** in its menu. This works for every
  session, the ones in the Claude app, the Codex app and your terminals
  too. AI Juggler keeps the pin; it never changes the app.
- **Drag** a session by its handle (it shows when you point at the row) to
  move it within its place, or drag a place's handle to move the place.
  ⌘1 to ⌘9 follow the new order of places.
- **From the keyboard**, select a session or a place and press ⌥⌘↑ or ⌥⌘↓
  (Ctrl+Alt+↑ or ↓ on Windows and Linux).

The menu on Open now's header (**Your order ▾**) picks the order:

| Order | What it does |
| --- | --- |
| **Your order** | Stays put. New ones join at the end. Drag or pin to arrange. |
| **Latest activity** | Newest event first; holds still under the pointer. A quiet place folds, and a place shows its working sessions and the last 3. |
| **Name** | A to Z. |

**Reset my order** puts sessions back in the order they opened and places
in their usual order; pins stay. Settings › General has the same choice.
AI Juggler keeps all of this with its settings, so a restart or a backup
keeps it.

## What a row says

Each session shows:

- the app's icon, with the profile's badge on it;
- its title;
- where it is: project · app · profile · window or tab;
- its state, as a dot and a word: **Asks you**, **Working**, **Idle** or
  **Done (review it)**. Ended sessions are folded under the "Also:" line.

When a state is worked out rather than read from the app, it says
"probably".

## Everything opens here

Everything in the list opens in AI Juggler, so you can check on a session
without leaving it:

| Click | Opens |
| --- | --- |
| A session, or Enter on it | Its page: its state, its last messages, and where it runs. |
| The project's name on a session | The project's page. |
| A place's name, such as **Claude · Work** | That profile's page: its account, and the sessions open on it. |
| **AI Juggler** (the place of its own sessions) | Home. |
| A project's name, when the list is grouped by project | The project's page. |
| **Also: 2 inactive** (or ended today) | Those sessions, under the line. |
| **History ›** | History. |

The menu on a right-click or ⋯ repeats these. You never need it.

## Go to it

To answer a session in its own app, use its **↗** (**Go to it**), or press
⌘Enter on it. Sessions that need you always show it; other sessions show it
when you point at them or select them. AI Juggler brings that session's app
and profile to the front, and then says what it did, for example "Brought
Claude · Work forward; pick the session in its sidebar". A place's **Open**
brings its app forward too.

Sessions AI Juggler runs itself have no ↗: their page is where they run.
Ended sessions have none either.

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

Search takes the same words: `is:ended`, `is:inactive`, `app:claude`,
`profile:work`, `project:` and `mac:`, then any other words. ⌘1 to ⌘9
bring the places in the list forward, in the list's order (Home is ⌘0).

## Last messages

A session's page shows its last two or three turns, so you can read what
it's asking before you go to it. They're read from the session's
transcript on this Mac when you open the page, with secrets hidden, and
kept nowhere. Claude app chats show their title only, because those
conversations aren't kept on your Mac.

## Subagents

When a session starts subagents (helpers it hands part of the work to),
its page gets a **Subagents** tab. It shows for any session, whichever app
runs it, once the session has started one. Each row says:

- the subagent's type and what it was asked to do (point at it for the
  whole task);
- where it stands: **Running**, **Done**, **Failed**, **Stopped** or
  **Unknown**;
- when it started, how long it took, and whether it ran in the background.

The newest come first, and a subagent that another subagent started sits
under it. Click one to read its own conversation, in the same view as the
session's; **‹ Subagents** (or Esc) brings you back. With **View › Show ›
sub-agents** on, a session's "+2 agents" in the list opens this tab.

AI Juggler reads all this from the tool's own records on your Mac, only
reads them, and keeps nothing. "Running" means the records don't say it
ended and the session is still open. Once the session has ended, such a
subagent shows **Unknown**: AI Juggler never guesses.

| Tool | Subagents | Their conversations |
| --- | --- | --- |
| Claude Code, and the Claude app's Code tab | Yes | Yes |
| Codex, and Codex in ChatGPT | Yes | Yes |
| Kimi Code, and Kimi Code Desktop | Yes | Not yet |
| Grok Build | Yes | Not yet |
| OpenCode, and OpenCode Desktop | Yes | Not yet |

Where the conversation isn't shown yet, the tab says so. Turning last
messages off in Settings › Desktop apps turns subagents' conversations off
too.

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
