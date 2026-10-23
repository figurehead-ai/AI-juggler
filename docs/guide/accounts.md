# Accounts

Each account is its own sign-in, kept apart from the others: two Claude
subscriptions are two accounts, and so is each Codex setup. The Accounts
page lists them all, with how much room each has left.

## Add an account

Choose **Add account** on the Accounts page, then the tool.

- **A setup already on this Mac** (one AI Juggler found, or a folder you
  point it to): use it where it is, or make a separate copy, just as on
  [your first run](first-run.md).
- **A fresh account:** give it a name that says what it's for (you can
  rename it later), then choose **Create and sign in**. The tool's own
  sign-in opens in a terminal. AI Juggler never sees your password.

### Start with your usual settings

A fresh account starts empty. When you already have an account of the same
tool, **Start with the same settings as …** is ticked and names it. Open it
to see what comes with it, and pick another account to copy from if you
like:

- **MCP servers**, copied as they are. One that needs a secret or a
  sign-in starts unticked, and says what you need to do.
- **Permissions:** for Claude Code, the commands and tools the account may
  use without asking; for Codex, its approval policy and sandbox mode.
- **The status line**, if you set one up yourself.
- **Plugins**, listed with the command that installs them, since a plugin
  comes from its marketplace. Run that command in the new account.

Each item that can be copied starts ticked. Permissions that turn the
tool's checks off (such as Claude Code's `bypassPermissions`, or Codex's
`danger-full-access`) are offered, but never ticked for you.

The account's sign-in, history and caches are never copied. A setting with
a secret written in it, such as a token in a permission rule, stays behind,
and the line says how many did. Settings only copy between accounts of the
same tool, because each tool's permissions work in their own way.

Anything that couldn't be copied is shown before you sign in.

### Later, or for a copy

Every account's menu has **Copy settings from…**. It lists what this
account lacks from another account of the same tool, each with a box.
Choose **Copy 2 settings** (or however many are ticked). Nothing the
account already has is changed.

Every copied setting can be undone in Settings › Context packs & secrets.
An undo takes out only what was copied, and only while it's unchanged.

## Your default account

New sessions start on the default account, unless a project has its own
(the project's menu: **Set its account…**). To choose the default, open an
account's menu and choose **Make the default**. With no default, new
sessions start on the account you used last.

## Launch settings

An account can have launch settings: a name (for example "Deep work"), a
model, and environment overrides. Choose **Edit launch settings** in the
account's menu. A session started on the account uses them.

Under the launch settings' options, **If a session crashes** can follow
Settings, always restart it, or leave it ended. See
[Keep work going](keep-work-going.md).

## Rename or remove an account

- **Rename…** changes only the name AI Juggler shows.
- **Remove…** takes the account out of AI Juggler. Its past sessions stay
  in History, marked "removed account".
  - A setup used where it is stays on your Mac exactly as it was.
  - For a separate copy, you choose: **Keep the folder** in AI Juggler's
    data, or **Move it to the Trash**, where you can still get it back.
