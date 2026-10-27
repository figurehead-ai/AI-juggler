# Continue on another account

When an account runs out of room, or you'd rather use another tool, you can
carry a conversation over and keep working. You choose each switch, one
conversation at a time. AI Juggler never switches for you.

## Start a switch

Open the session, then do one of these:

- Choose **Switch account…** at the top of the session, or press ⌘⇧H.
- Open the session's menu in the list and choose **Continue on another
  account…**.
- In the panel on the right, open **Continue on another account**. It lists
  every account you can go to, ranked mostly by how much room each has
  left, then by how much you've used it in this project.

A finished conversation in your history can be continued the same way.

You pick a profile to go to, listed under its account. A profile signed in
to the same account as the one you're on is marked "no new room": profiles
on one account share its limits, so moving there doesn't help when you've
run out.

## Two kinds of switch

What carries over depends on where you're going. The switch screen says
which kind it is before you confirm.

- **Same conversation, new account.** Between two profiles of the same
  tool, for example two Claude Code folders signed in to different
  accounts. The conversation's files are copied to the other profile, which
  resumes the conversation. The task list
  is rebuilt from the conversation.
- **Full story, fresh start.** Between two tools, for example Claude Code to
  Codex. The other tool starts a new session and reads a handover that AI
  Juggler writes into its own folder, readable only by you. The handover
  holds the story so far:
  - the newest turns that fit, word for word;
  - each tool call, in one line (such as "Edit calc.py");
  - the earlier turns, listed by what was asked;
  - the task list as it was last set;
  - which files aren't committed yet. They're already in your project, so
    the new session works on them where they are;
  - the project's rules files, and a request to read the other tool's
    (`CLAUDE.md` or `AGENTS.md`).

  A very long last turn is shortened to fit. Claude Code is allowed to read
  the handover's folder, so it doesn't stop to ask.

A conversation that was compacted loses nothing in a switch. Between two
accounts of the same tool, the whole session file moves, every message from
the start, and the new account picks it up exactly as the tool itself would
(its summary and recent turns in mind, the full history kept). Between two
tools, the other tool reads the newest turns that fit, word for word, and
the earlier ones by what was asked, compacted or not.

### Beta

Every pair of accounts says **Beta · Not proven yet** until every part of
that pair has been checked live. A pair that has been reads **Proven**, and
a new version of the tool puts it back to Beta. AI Juggler never calls a
switch lossless.

These switches have been run live on real accounts, in small test projects,
with Claude Code 2.1.302 and Codex 0.163.0:
- Claude Code account A → account B → Codex → back to A;
- Codex → Claude Code, and back again;
- a switch in a git worktree, where the work stayed in the worktree;
- a project with a rules file and a subagent.

At every step, the new session knew what had been decided before it, the
open tasks and the project's rules. A switch from one Codex account to
another hasn't been run live yet.

## The switch screen

When the move is proven on your tools, keeps the conversation, leaves
nothing behind and the other account has room, it takes one screen: what
moves, what happens to the original, and **Switch now** (or Enter).
Otherwise it has three short steps: **Target**, **What's carried** and
**Confirm**. Switching to an account that isn't the one you usually use here
is never a warning: that's the point of a switch.

Every step has **Cancel** (or Esc) and a close button. While the copy runs,
the screen stays until it's done ("Finishing copy…"): the original is safe
the whole time. If the switch doesn't complete, nothing was changed, and
**Retry** runs it again.

### What goes along

The **What's carried** step lists what goes along, in plain words. Hover
over, or tab to, an item for a line on what it means; **What does this
mean?** lists them all.

- **The whole conversation:** every message from the start. Another tool
  reads it, newest first as much as fits.
- **Project rules:** the rule files in the project folder (such as
  `CLAUDE.md` or `AGENTS.md`). They stay where they are and are read again.
- **Your uncommitted changes:** they stay in the folder, and the new session
  works in the same folder. A switch to another tool also hands it a summary
  of them.
- **Connected tools (MCP servers)**, **Skills and commands**, **Model
  choice** and **Subagents' work**.
- **Recreated on the account you switch to:** the task list, from its latest
  version.
- **Stays behind:** the new session starts without it. This is the only part
  shown as a warning.

### Keep or stop the original

A conversation can't be copied while it's still being written, so the
original stops first. Either way it stays, and you can switch back to it any
time.

- **Let it finish its reply, then switch** (recommended). For example, if
  it's writing a test, the test is finished first.
- **Stop it now and switch.** An unfinished reply is marked as cut off, so
  the new session knows to pick it up again.

AI Juggler can stop only sessions it started. A session you started in a
terminal or in the Claude app keeps going there if it's still open, and the
switch starts a separate continuation. The screen says so.

### Keys and passwords in this conversation

A switch between two accounts of the same tool copies the conversation as it
is, on this Mac, so it never asks about keys. When the conversation's words go
somewhere new (another tool, and the company behind it, reads them to pick up
the work), the switch says what it found (an API key, a sign-in token, a
private key; only a masked preview such as `s•••a91f`, never the value) and
who will read it, and asks:

- **Hide them** (recommended): the other tool sees `OPENAI_API_KEY=•••`
  instead of the key. If it needs a key, it will ask you. You can still show
  one of them anyway, for this switch only.
- **Leave them in:** the other tool and the company behind it see the keys
  exactly as written. Use this only if the work needs them.

Nothing is chosen for you. Tick **Remember my choice** to use it next time;
unticked, AI Juggler asks on every switch like this one. Change it any time
in Settings › Switching › Handovers and keys. When nothing was found, the
switch says so and doesn't ask.

### Things the other account doesn't have

The **Confirm** step opens with **Ready to switch to …** and three lines:
what moves, what happens to the original, and whether anything needs you.
**Before you switch** appears only when something does; everything that's
fine is under **Details**.

It lists what your current profile has set up and the other one lacks: MCP
servers, skills, commands, subagents, rules and plugins. None of them stops
the switch. Each line says what will happen:

- **Set up as it is:** copied exactly.
- **Set up as …:** translated into the other tool's form, for example a
  Codex prompt set up as a Claude Code command.
- **Needs you:** something only you can do, such as providing a token or
  installing a plugin with the tool's own command.
- **Turned on by the account:** some skills and features come with the
  account itself, for example a claude.ai account's skills and its plan's
  features (such as research or the browser). They can't be copied to
  another account. The line says "You can switch now. *Account* won't have
  *it* until you turn it on", with the steps:
  1. Choose **Open claude.ai settings**: your browser opens claude.ai's
     settings. Sign in there as the account you're switching to (AI Juggler
     never signs in for you).
  2. Find the skill or feature there and turn it on.
  3. Come back and choose **Check again**: AI Juggler reads that account
     again, and the line goes once it's there.

  It's there only if that account's plan includes it.

Lines that can be set up, as they are or translated, start ticked. **Needs
you** lines don't. Nothing is set up until you confirm, and only what's
ticked.

A secret is never copied:

- A server with a secret in its environment or headers is set up to read
  it from a named variable instead, and the line names it. It starts
  unticked, as **Needs you**, with the steps to give the other account its
  own copy of the value (see below).
- A server with a secret in its address or arguments can't be set up.

#### A server that needs its token

For example, a GitHub MCP server that needs `GITHUB_TOKEN`. The line says
"Needs you: it needs your GITHUB_TOKEN, and AI Juggler never copies a
secret." and then "You can switch now; it doesn't stop the switch." Tick it
to set the server up on the other account without the value. To give that
account its own copy of the value, follow the steps on the line. They come
from the other account's tool, and the command already names that
account's folder.

**Claude Code.** Claude Code keeps a server and its environment in each
account's own settings
([Claude Code: MCP](https://code.claude.com/docs/en/mcp), "Add MCP servers
from JSON configuration" and "MCP installation scopes").

1. Open Terminal.
2. Paste the command from the line. Put your own value in place of
   `<your GITHUB_TOKEN>` (keep the quotes around it), then press Return.
   For example:

   ```sh
   CLAUDE_CONFIG_DIR="$HOME/.claude-personal" claude mcp remove github --scope user
   CLAUDE_CONFIG_DIR="$HOME/.claude-personal" claude mcp add-json --scope user github '{"args":["-y","@modelcontextprotocol/server-github"],"command":"npx","env":{"GITHUB_TOKEN":"<your GITHUB_TOKEN>"},"type":"stdio"}'
   ```

   The first line takes away a server set up without the value. If there
   isn't one, it says so, and that's fine. **Copy** on the line copies the
   command for you.
3. Come back and choose **Check again**. The line goes once the account has
   the server.

**Codex.** Codex keeps its servers in each account's `config.toml`
([Codex: MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli), and
[the sample configuration](https://learn.chatgpt.com/docs/config-file/config-sample)).

- If the server is a program on this Mac, the steps give two commands:
  `codex mcp remove`, then `codex mcp add … --env GITHUB_TOKEN=<your
  GITHUB_TOKEN> -- …`, both for that account's folder.
- If the server reads other values from your environment, the steps say
  which lines to add under `[mcp_servers.github.env]` in that account's
  `config.toml`, for example `GITHUB_TOKEN = "<your GITHUB_TOKEN>"`.
- If the server is reached over the web, Codex reads its token from your
  environment, so it can't be given to one account alone. The steps give
  the line for your shell's startup file (for zsh, `~/.zshrc`), for example
  `export GITHUB_TOKEN="<your GITHUB_TOKEN>"`. Every account on this Mac
  gets it.

AI Juggler never sees, keeps or writes the value. You type it into the
other tool's own settings yourself.

A sign-in never moves: you sign in on the other profile with that tool's
own sign-in.

## After the switch

The result says which account is picking up the conversation. The original
stops, and you can switch back to it at any time.

- **Undo the switch:** a one-click switch that set nothing up offers
  **Undo** for 10 seconds. It stops the new session and resumes the
  original.
- **Undo a set-up:** each set-up is listed with its own **Undo**. Later, find
  them in Settings › Switching › Handovers and keys › **Set up by AI
  Juggler**. An
  undo removes only what AI Juggler added, and only while it's unchanged.
- **Usage:** the new account counts only what was done there, not the
  history it carried.

## Switch back

When the original account has room again, the session's panel offers
**Resume the original conversation ▸**. Under **Go back to**, pick the
previous account or the one where the work started.

The original is told what happened while it was away, on every account
the work passed through: their new turns, the changes and the task list.

## Start from a saved handover

A saved handover is a conversation's story saved as a folder, the same thing
a switch hands over. Save one from a project's **Overview**, under **How
this work travelled**: choose a step, then **Save handover…**. Keys and
passwords in it are handled by your choice in Settings › Switching ›
Handovers and keys.

To pick the work up later, in any tool, choose **New session** › **Start
from a saved handover…** and pick its folder. AI Juggler checks it, then
shows what it is: where it came from, what it continues as, what it carries,
what it doesn't carry, and what was withheld. Choose the profile and folder
(its own project is picked for you when it's on this Mac), then **Launch**.

A handover that was edited by hand, holds something that looks like a key,
points outside itself, or comes from a newer AI Juggler is refused, with a
one-line reason, and nothing starts.

## What a switch changes

- **On the profile you switch to:** the conversation's files, added beside
  that profile's own and never over them, and the set-ups you ticked.
- **On the original profile:** nothing, apart from stopping its session if
  you chose to.
- **Your project:** nothing. Its own rules files and `.mcp.json` are shared
  by both sides already.
