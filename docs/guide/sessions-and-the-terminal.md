# Sessions and the terminal

## Your sessions

The list on the left shows the sessions open right now, in every app,
grouped by place: the app and profile each runs in. What needs you is on
top, and ended or inactive sessions are one click away under the "Also:"
line. To see your projects with their sessions instead, choose **View** ›
**Group by** › **Project**. [What's open, and getting to it](whats-open.md)
explains the list, **Go to it** and **View**.

History holds every session that has ended, and you can search it.

### Searching what was said

History finds a session by what it was about, its project, account, tool,
model, branch or date, and by the words said in it. For sessions AI Juggler
started, words are searched as they are. For the ones started outside it,
turn on **Search the words of every session** in Settings › Privacy:

- **Off by default.** It builds an index of those sessions' words on your
  Mac, and the setting says how much room the index would take before you
  turn it on.
- **Built in the background**, newest first, from the tools' own files,
  which AI Juggler only reads. The setting shows how far it has got.
- **The index holds the words to search, not the conversations.** A match
  is shown by reading the words around it from the tool's own file. A
  session the tool deletes leaves the index too.
- **Turning it off deletes the index.**

To start one, choose **New session**. Pick the project folder, the tool and
the profile. A project can have its own profile (set it from the project's
menu). Otherwise new sessions use your default profile.

A session you started elsewhere can be picked up with **Continue in AI
Juggler**. To carry one to another account or tool, see
[Continue on another account](switching.md).

## The terminal

A session AI Juggler started in a terminal shows it inside its page (how
else a session can run is [below](#how-a-session-runs)). The strip
above it says whether it's **Live**, **Reconnecting** (while it catches up
after the window reopens) or **Ended**, and holds its tools. Each tool's key
is in its tooltip, and the **?** sheet lists them all.

- **Find** (⌘F): Enter for the next match, Shift+Enter for the one before.
  Esc closes it and gives the keys back to the terminal.
- **Links:** web addresses and file paths that the terminal prints are
  underlined when you point at them. ⌘-click opens them:
  - a web address in your browser;
  - a file in the editor you chose for projects;
  - a folder in Finder.

  A plain click still selects text. Nothing the terminal printed is ever
  run.
- **Copy:** the selection, or what's on screen.
- **Text size:** A− and A+, or ⌘− and ⌘+, with ⌘0 to go back. The size is
  saved in Settings › Sessions › **Terminal**, so every terminal follows it.
- **Jump to the bottom** when you've scrolled up.
- **Maximize** (⌘↩): the terminal fills the window. ⌘↩ or **Restore** puts
  it back.
- **Own window:** the terminal moves into a window of its own, for example
  on another display. The session's page then says "This terminal is in its
  own window", with **Show its window** and **Bring it back here**. The
  session keeps running either way.

Keeping sessions going when they crash or the Mac would sleep is in
[Keep work going](keep-work-going.md).

## How a session runs

A session AI Juggler starts runs one of two ways, and its page says which:

- **In a terminal**, the tool's own screen, as when you run it yourself.
  This is how Claude Code and the other tools run.
- **Through the tool's own app-server**, with no terminal. This is how a
  Codex session runs by default. The page shows the conversation as it goes,
  the session's state, and its usage limits as Codex reports them. A box at
  the bottom sends a message, or adds to what it's doing while a turn runs.

Either way it's your own installed tool, unmodified, signed in as you signed
it in. AI Juggler doesn't talk to OpenAI or Anthropic itself.

**Open in terminal** (on a Codex session's page) waits for the turn to end,
then continues the same conversation in Codex's own terminal. A Codex
session that can't run through its app-server, for example with a Codex
version AI Juggler doesn't know yet, starts in a terminal instead, and its
page says why.

**When AI Juggler quits or crashes**, a session it runs through the tool's
app-server ends with it, and comes back when AI Juggler opens again: the
same session, on the same conversation, with nothing sent for you. The
quit sheet says so. If the session was in the middle of a turn, its page
says "Its turn was cut off when AI Juggler stopped" and waits for you:
**Resume** continues the conversation and sends nothing, and **End it**
ends it. AI Juggler keeps only which conversation each session is on, so
it can find it again, never what was said.

**A folder Codex hasn't been told to trust.** Codex's own terminal would
ask "Trust this folder?" and save your answer to its `config.toml`.
AI Juggler asks the same question itself, once per folder, the first time
you start a Codex session there or open one in its terminal: **Trust and
continue** or **Open restricted** (Codex asks before it edits or runs
things). AI Juggler keeps your answer and passes it to each Codex session
in that folder, so Codex's `config.toml` is never changed. A folder you
already decided in Codex itself is left to Codex. **Cancel** starts
nothing.

To choose per profile, go to Settings › Sessions › **How sessions run**: a
profile can always start in a terminal. **New session** has **How it runs**
for just that session.

A Claude Code session in a terminal still tells AI Juggler exactly what it's
doing (working, waiting for you, asking a question) through Claude Code's
own hooks. They're added to that launch only, in AI Juggler's folder, and
never written into your profile.

## Answering a session's questions

When a session asks something, it's on top under **Needs you**, on Home and
on its page:

- **The tool's own choices**, in its words: for example **Allow once**,
  **Always allow `npm test`** or **Deny**. "Always allow" is the tool's own
  rule, which the tool keeps. Take it back in the tool.
- **A question** shows its choices, and a box for your own answer.
- **Keys:** 1 to 9 answer with that choice, and Enter with the first.
- Several open questions are listed together, each with its session.

Nothing is answered until you click. AI Juggler never answers by itself, on
a timer or by default. The answer goes to the tool, and the account's log
records it. A Claude Code session in a terminal is answered in its terminal:
**Go to it**.

### Claude Code without a terminal (Beta)

Settings › Sessions › **Run without a terminal (Beta)** lets you start a
Claude Code session with no terminal, its questions and approvals answered
here. It's off by default. Once it's on, pick **Run without a terminal:
answer from AI Juggler (Beta)** under **How it runs** when you start a
session. The terminal stays the default.

It runs your own Claude Code with your own sign-in, which Anthropic's terms
allow, but Anthropic may count it as Agent SDK use rather than ordinary
Claude Code use. **Open in terminal** hands the conversation back to Claude
Code's terminal when a turn ends.

It stays a Beta until all four of these hold:

1. Anthropic's published terms, rechecked at each release, still allow it.
2. Claude Code still signs in with your subscription when it runs this way.
   Anthropic says a future version may stop reading it by default.
3. A month of use on the developers' own lab accounts changed nothing in
   how they're billed.
4. The conversation view passes a walk by hand.
