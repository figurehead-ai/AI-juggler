# What's open, and getting to it

The list on the left shows what is really open on your Mac right now, in
every app, and gets you to any of it in one step.

## Apps

The list groups sessions by **app**: one app in one of its profiles, for
example **Claude app · Work Max** or **Claude Code · Work Max**. See
[Accounts and apps](accounts.md).

A Claude app profile is named by the Claude Code folder its sessions use,
as you name it there (**Claude app · team-max**). Until one of its
sessions has run, it's named by its own folder (**Claude app · b-team**).

The Claude app keeps each of your profiles in a folder of its own, side by
side. The first time you open the page of an app in one of them, AI
Juggler asks **Find
this app's other profiles**: with your yes, it reads the names of the
folders beside it (never anything inside them), so the profiles you
haven't opened today are apps too, each with **Start**. Settings ›
Desktop apps › **Find profiles that aren't running** turns it off, and
forgets the profiles it found that way.

A Claude Code or Codex session sits under its tool and folder, such as
**Claude Code · Work Max**. The terminal it runs in is a detail on its row
("in Terminal"), not a group of its own, so the sessions AI Juggler runs
and the ones in your terminals on the same folder are one app.

A session AI Juggler started shows once, under its app, even for a
tool that doesn't report to AI Juggler (OpenCode, Grok Build, Kimi Code):
AI Juggler tells which conversation the session started from the tool's
own records, or from the process it runs in. When more than one
conversation could be the session's (two started in the same folder at
once), it doesn't guess: the conversation shows as a row of its own too.
The same holds for an OpenCode or Grok Build app you use where it is
(shared with your own terminal): their records don't say which program
wrote a conversation, so one you started yourself in that folder can't be
told from AI Juggler's, and both rows stay.

The list has three parts:

- **Needs you:** sessions that ask you something, and anything else waiting
  for you. It's a shortcut: each of them also stays under its app in Open
  now, its state in amber.
- **Open now:** every session that is running, grouped by app. An app's
  header has **↗** (**Bring it forward**), which brings that app's window
  forward, or opens the first of its sessions AI Juggler runs. Click an
  app's chevron to fold it; it stays folded until you open it again.
- **One line under them,** such as "Also: 3 ended today · 2 inactive · 1
  background · History ›". Each part shows those sessions, and the list
  scrolls to them. Nothing is removed; they're just not shown until you
  ask.

Click **Needs you** or **Open now** to fold that part, and again to open
it.

## Keeping the order

Open now stays put. A session keeps its place in the list while it works,
waits or asks you something; only its dot and word change. A new session
joins the end of its app, and one that ends leaves a gap the rows under
it close. An app with more than 8 sessions shows the first 8, then
"+n more".

- **Pin** a session to keep it at the top of its app: point at it and
  click its pin, or use **Pin to top** in its menu. This works for every
  session, the ones in the Claude app, the Codex app and your terminals
  too. AI Juggler keeps the pin; it never changes the app.
- **Drag** a session by its handle (it shows when you point at the row) to
  move it within its app, or drag an app's header by its handle to move
  the app. ⌘1 to ⌘9 follow the new order of apps.
- **From the keyboard**, select a session or an app's header and press ⌥⌘↑ or ⌥⌘↓
  (Ctrl+Alt+↑ or ↓ on Windows and Linux).

The menu on Open now's header (**Your order ▾**) picks the order:

| Order | What it does |
| --- | --- |
| **Your order** | Stays put. New ones join at the end. Drag or pin to arrange. |
| **Latest activity** | Newest event first; holds still under the pointer. A quiet app folds, and an app shows its working sessions and the last 3. |
| **Name** | A to Z. |

**Reset my order** puts sessions back in the order they opened and apps
in their usual order; pins stay. Settings › General has the same choice.
AI Juggler keeps all of this with its settings, so a restart or a backup
keeps it.

## What a row says

Each session shows:

- its app's icon and badge;
- its title;
- where it is: project · app · window or tab, for example "shop · Claude
  Code · Work Max · in Terminal";
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
| An app's name, such as **Claude Code · Work Max** | Its page: its account, and the sessions open in it. |
| A project's name, when the list is grouped by project | The project's page. |
| **Also: 2 inactive** (or ended today) | Those sessions, under the line. |
| **History ›** | History. |

The menu on a right-click or ⋯ repeats these. You never need it.

## Go to it

To answer a session in its own app, use its **↗** (**Go to it**), or press
⌘Enter on it. Sessions that need you always show it; other sessions show it
when you point at them or select them. AI Juggler brings that session's app
to the front, and then says what it did, for example "Brought Claude app ·
Work forward; pick the session in its sidebar". An app header's **↗**
brings it forward too.

Sessions AI Juggler runs itself have no ↗: their page is where they run.
Ended sessions have none either. An ended session's page has **Open in
{app}** instead, for the app it was last open in: AI Juggler starts that
app if it isn't running, in its own profile (the Codex app
opens at the thread itself; in the Claude app, pick the session in its
sidebar).

It brings the exact window or terminal tab forward only if you turned that
on in Settings › Desktop apps. Going to a session never asks for a macOS
permission by itself.

## View

**View** (⇧V) changes what the list shows. Its button has a dot when
anything differs from the defaults.

- **Group by:** App (the default), Project, Account or Pool. **Account**
  puts the apps signed in to one subscription together, under its name.
- **Show:** inactive, quiet, ended, background runs, sub-agents, your other
  Macs, archived.
- **Only:** one app, account, project or Mac.
- **Back to the defaults.**

Search takes the same words: `is:ended`, `is:inactive`, `app:`,
`account:`, `project:` and `mac:`, then any other words.

- `app:` takes a word of the app's name (`app:claude`, `app:work`) or
  where its window is (`app:terminal`).
- `account:` means the subscription, by the name the Accounts page shows.
- `profile:` still works: it matches the part of an app's name after the
  "·", such as `profile:work`.

⌘1 to ⌘9 bring the apps in the list forward, in the list's order (Home is
⌘0).

Every name opens its own page: a session's app, its account, its project.
To go back to where you were, press ⌘[ or click ‹ at the left of the top
bar; ⌘] or › goes forward again, as in a browser.

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
sub-agents** on, a session's "+2 agents" in the list (any session open
now, whichever app runs it, for every tool in the table below) opens this
tab, and
on the page of a session AI Juggler runs, the line "6 subagents · 2
running › Subagents" does too.

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
  sessions each app has open from the tools' own files. It never types into
  them.
- **Going to a session:**
  - **Bring the exact window to the front** needs the Accessibility
    permission. **Set up…** walks you through it.
  - **Select the right terminal tab** needs the Automation permission. Off
    until you turn it on.
  - **Open AI Juggler from anywhere:** a keyboard shortcut you choose.
    There's none until you pick one. ⌥Space isn't accepted, because ChatGPT
    uses it.
- **Your apps:** every app AI Juggler has seen running. **Start** opens
  one that isn't running, in its own profile; **Open** brings a running
  one forward. **Keep alive** starts it again
  whenever it stops ([Keep work going](keep-work-going.md#keep-apps-alive)
  says when it doesn't). **Forget** removes one you no longer use. A
  profile found beside another has no Forget: remove its folder, or turn
  off **Find profiles that aren't running** below the list.
- **Last messages:** on by default. Turn it off and session pages don't
  read transcripts.

AI Juggler never reads a running app's environment, key files or saved
sign-ins to find out what's open. [What AI Juggler touches](trust.md) has
the full list.
