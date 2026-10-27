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

A conversation that was compacted carries its early part as a summary, so
the new session may ask about early decisions. The screen tells you when
this applies.

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

You can change this in Settings › Switching › Context packs & secrets.

### What the other profile doesn't have yet

The confirm step lists what your current profile has set up and the other
one lacks: MCP servers, skills, commands, subagents, rules and plugins.
What this conversation used comes first, marked **Used here**. The rest
waits under one closed line, **Also on Work Max, not on Personal (not used
in this conversation)**: open it to see them. When a tool's history
doesn't say what was used, its lines stay in the main list, never marked
as unused.

Each line says what will happen, and whether the switch waits for it:

- **Set up as it is when you switch:** copied exactly. You can undo it
  after.
- **Set up as … when you switch:** translated into the other tool's form,
  for example a Codex prompt set up as a Claude Code command.
- **You can switch now, then finish it:** something only you can do, such
  as providing a token or installing a plugin with the tool's own command.
- **You can switch now. Personal won't have it until you turn it on:**
  it comes with the account, so the other account turns it on itself. See
  [Things the other account doesn't have](#things-the-other-account-doesnt-have).
- **You can switch now. Personal won't have it:** the other tool has no
  place for it, with the reason.

Lines that can be set up, as they are or translated, and that this
conversation used, start ticked. Lines that need you, or that weren't used
here, don't. Nothing is set up until you confirm, and only what's ticked.
Each line has **What does this mean?**, with an example and a link here.

A secret is never copied:

- A server with a secret in its environment or headers is set up to read
  it from a named variable instead, and the line names it. It starts
  unticked, as **Needs you**, until you've set that variable.
- A server with a secret in its address or arguments can't be set up.

A sign-in never moves: you sign in on the other profile with that tool's
own sign-in.

### Things the other account doesn't have

Some things come with an account, not with your computer: claude.ai syncs
the skills and plugins turned on for an account into Claude Code when one
of that account's sessions starts, and OpenAI's plugins come with Codex
and your ChatGPT workspace. AI Juggler doesn't copy them to another
account. A copy wouldn't update when the original does, the account
decides who may have them, and Anthropic's own skills come with a license
that doesn't allow copies. The other account turns them on itself, and the
switch shows how: **Show me how** lists the steps, the button beside it
opens the right settings page, and **Check again** looks at the other
account again once you've done it.

You can switch straight away. The other account just won't have the item
until you turn it on.

**A skill from Anthropic** (the line says "From Anthropic"):

1. Sign in to claude.ai as the account you're switching to.
2. In Settings > Capabilities, turn on **Code execution and file
   creation** (it may be called **Create and edit files**). Skills need it.
3. In Customize > Skills, find the skill under **From Anthropic &
   Partners** and turn it on.
4. Start a session on that account so it syncs. It arrives when the
   session starts, or within about 10 minutes in one that's already
   running. Then choose **Check again**.

Some skills also need a plan, an owner or another product. The line says
which:

- **computer-use:** a Pro or Max plan (computer use isn't offered on Team
  or Enterprise), and computer use turned on in the Claude desktop app.
- **chrome-browser:** a paid plan, and the Claude in Chrome extension,
  installed and connected.
- **built-in-browser:** Cowork in the Claude desktop app. On Team and
  Enterprise, an owner may have turned it off.
- **deep-research:** a paid plan, and web search turned on.
- On a Team or Enterprise plan, your organization's owner may need to
  allow Skills first.

**A skill from your claude.ai account** (the line says "From Work Max's
account: yours, your organization's or shared with you"): AI Juggler can't
tell from your computer whether you made it, your organization provides
it or someone shared it. In Customize > Skills, look for it under **From
your organization** or **Shared with you** and turn it on. If you made it,
you can upload it there (**+** > **Create skill** > **Upload a skill**, a
ZIP of its folder), or tick **Copy it to Personal now instead (only if you
made it)**: AI Juggler copies it as new files, with an **Undo**.

**A plugin from claude.ai:** in Customize > Plugins, choose **Discover**,
find it and add it. Then start a new session on that account, or type
`/reload-plugins` in one that's running.

**A Codex plugin from OpenAI:** in Codex on the account you're switching
to, type `/plugins`, find it and install it (one that comes with the Codex
app may only need turning on there). If it connects an app, sign in to it
when Codex asks. On an Enterprise or Edu workspace, your admin may have to
allow it first.

After the switch, whatever this conversation used and the other account
still has to turn on stays in Settings › Switching › Context packs &
secrets › **Set up by AI Juggler**, under **Still to turn on**, with the
same steps and **Check again**. Each one leaves the list once the account
has it, or when you choose **Stop reminding me**.

## After the switch

The result says which account is picking up the conversation. The original
stops, and you can switch back to it at any time.

- **Undo the switch:** a one-click switch that set nothing up offers
  **Undo** for 10 seconds. It stops the new session and resumes the
  original.
- **Undo a set-up:** each set-up is listed with its own **Undo**. Later, find
  them in Settings › Switching › Context packs & secrets › **Set up by AI
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

## Start from a context pack

A context pack is a conversation's story saved as a folder, the same thing
a switch hands over. Save one from a project's **Overview**, under **How
this work travelled**: choose a step, then **Export pack…**. Secrets are
handled by your choice in Settings › Switching › Context packs & secrets.

To pick the work up later, in any tool, choose **New session** › **Start
from a context pack…** and pick the pack's folder. AI Juggler checks the
pack, then shows what it is: where it came from, what it continues as,
what it carries, what it doesn't carry, and what was withheld. Choose the
profile and folder (the pack's own project is picked for you when it's on
this Mac), then **Launch**.

A pack that was edited by hand, holds something that looks like a key,
points outside itself, or comes from a newer AI Juggler is refused, with a
one-line reason, and nothing starts.

## What a switch changes

- **On the profile you switch to:** the conversation's files, added beside
  that profile's own and never over them, and the set-ups you ticked.
- **On the original profile:** nothing, apart from stopping its session if
  you chose to.
- **Your project:** nothing. Its own rules files and `.mcp.json` are shared
  by both sides already.
