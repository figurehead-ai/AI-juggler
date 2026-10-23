# Sessions and the terminal

## Your sessions

The list on the left shows your projects, each with its sessions,
including ones you started in a terminal or in the Claude app. Those say
"outside AI Juggler". History holds every session that has ended, and you
can search it.

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
the account. A project can have its own account (the project's menu: **Set
its account…**). Otherwise new sessions use your default account.

A session you started elsewhere can be picked up with **Continue in AI
Juggler**. To carry one to another account or tool, see
[Continue on another account](switching.md).

## The terminal

A session AI Juggler started runs in a terminal inside its page. The strip
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
  saved in Settings › Sessions, so every terminal follows it.
- **Jump to the bottom** when you've scrolled up.
- **Maximize** (⌘↩): the terminal fills the window. ⌘↩ or **Restore** puts
  it back.
- **Own window:** the terminal moves into a window of its own, for example
  on another display. The session's page then says "This terminal is in its
  own window", with **Show its window** and **Bring it back here**. The
  session keeps running either way.

Keeping sessions going when they crash or the Mac would sleep is in
[Keep work going](keep-work-going.md).
