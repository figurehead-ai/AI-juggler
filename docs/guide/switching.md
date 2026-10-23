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

## Two kinds of switch

What carries over depends on where you're going. The switch screen says
which kind it is before you confirm.

- **Same conversation, new account.** Between two accounts of the same
  tool, for example two Claude Code accounts. The conversation's files are
  copied to the new account, which resumes the conversation. The task list
  is rebuilt from the conversation.
- **Full story, fresh start.** Between two tools, for example Claude Code to
  Codex. The other tool starts a new session that's told the story so far:
  - the newest turns that fit, word for word;
  - each tool call, in one line (such as "Edit calc.py");
  - the task list as it was last set;
  - which files aren't committed yet. They're already in your project, so
    the new session works on them where they are.

  When earlier turns don't fit, the new session is told how many were left
  out. A very long last turn is shortened to fit.

A conversation that was compacted carries its early part as a summary, so
the new session may ask about early decisions. The screen tells you when
this applies.

### Beta

Every pair of accounts says **Beta · Not proven yet** until every part of
that pair has been checked live. A pair that has been reads **Proven**, and
a new version of the tool puts it back to Beta. AI Juggler never calls a
switch lossless.

So far, one journey has been run live, in a small test project, with
Claude Code 2.1.302 and Codex 0.163.0: Claude Code account A → account B →
Codex → back to A. At every step, the new session knew what had been
decided before it, and the open tasks. A switch from one Codex account to
another, and from Codex forward to Claude Code, haven't been run live yet.

## The switch screen

When the project's usual account is on the same tool, has room, and nothing
would be left behind, switching to it takes one click: the screen shows
what's carried, and **Switch ▸** (or Enter). Otherwise it has three short
steps: **Target**, **What's carried** and **Confirm**. Only the last one
has the button.

### Freeze the source first

A conversation can't be copied while it's still writing, so you choose:

- **Finish this turn, then switch** (the default). AI Juggler waits for the
  current reply to end.
- **Stop now and switch.** An unfinished reply is marked as cut off.

AI Juggler can stop only sessions it started. A session you started in a
terminal or in the Claude app keeps going there if it's still open, and the
switch starts a separate continuation. The screen says so.

### Secrets in the conversation

The first time you switch, you choose how secrets that appear in a
conversation (such as an API key you pasted) are handled:

- **Strip:** they're replaced with •••.
- **Keep:** they stay as they are, and the account you switch to can read
  them.
- **Ask each time.**

You can change this in Settings › Context packs & secrets.

### What the other account doesn't have yet

The confirm step lists what your current account has set up and the other
one lacks: MCP servers, skills, commands, subagents, rules and plugins.
Each line says what will happen:

- **Set up as it is:** copied exactly.
- **Set up as …:** translated into the other tool's form, for example a
  Codex prompt set up as a Claude Code command.
- **Needs you:** something only you can do, such as providing a token or
  installing a plugin with the tool's own command.
- **Can't be set up:** with the reason, such as a skill that comes from your
  claude.ai account.

Lines that can be set up, as they are or translated, start ticked. **Needs
you** lines don't. Nothing is set up until you confirm, and only what's
ticked.

A secret is never copied:

- A server with a secret in its environment or headers is set up to read
  it from a named variable instead, and the line names it. It starts
  unticked, as **Needs you**, until you've set that variable.
- A server with a secret in its address or arguments can't be set up.

A sign-in never moves: you sign in on the other account with that tool's
own sign-in.

## After the switch

The result says which account is picking up the conversation. The original
stops, and you can switch back to it at any time.

- **Undo the switch:** a one-click switch that set nothing up offers
  **Undo** for 10 seconds. It stops the new session and resumes the
  original.
- **Undo a set-up:** each set-up is listed with its own **Undo**. Later, find
  them in Settings › Context packs & secrets › **Set up by AI Juggler**. An
  undo removes only what AI Juggler added, and only while it's unchanged.
- **Usage:** the new account counts only what was done there, not the
  history it carried.

## Switch back

When the original account has room again, the session's panel offers
**Resume the original conversation ▸**. Under **Go back to**, pick the
previous account or the one where the work started.

The original is told what happened while it was away, on every account
the work passed through: their new turns, the changes and the task list.

## What a switch changes

- **On the account you switch to:** the conversation's files, added beside
  that account's own and never over them, and the set-ups you ticked.
- **On the original account:** nothing, apart from stopping its session if
  you chose to.
- **Your project:** nothing. Its own rules files and `.mcp.json` are shared
  by both sides already.
