# Accounts and profiles

Two words, kept apart:

- A **profile** is the setup an app runs on: a Claude Code folder (such as
  `~/.claude`, or one named by `CLAUDE_CONFIG_DIR`), a Claude app profile
  (its data folder), or a Codex folder (`CODEX_HOME`). Sessions run in a
  profile.
- An **account** is the subscription a profile is signed in to. Limits and
  money belong to the account.

One account can have several profiles. Signing in to the same account in
two profiles is normal, and AI Juggler doesn't warn about it. Those
profiles share the account's limits, so moving work between them gives you
no new room.

The Accounts page shows each account once, with its limits, and its
profiles under it: each with its badge, name, the apps that use it, its
folder and how many of its sessions are open. Accounts are worked out from
who each profile is signed in to, so a personal plan and a team seat on the
same email are two accounts.

## Add a profile

Add a profile from the Accounts page, then pick the tool.

- **A setup already on this Mac** (one AI Juggler found, or a folder you
  point it to): use it where it is, or make a separate copy, just as on
  [your first run](first-run.md).
- **A fresh profile:** give it a name that says what it's for (you can
  rename it later), then choose **Create and sign in**. The tool's own
  sign-in opens in a terminal. AI Juggler never sees your password. The
  account it belongs to is whichever one you sign in to.

### Start with your usual settings

A fresh profile starts empty. When you already have a profile of the same
tool, **Start with the same settings as …** is ticked and names it. Open it
to see what comes with it, and pick another profile to copy from if you
like:

- **MCP servers**, copied as they are. One that needs a secret or a
  sign-in starts unticked, and says what you need to do.
- **Permissions:** for Claude Code, the commands and tools the profile may
  use without asking; for Codex, its approval policy and sandbox mode.
- **The status line**, if you set one up yourself.
- **Plugins**, listed with the command that installs them, since a plugin
  comes from its marketplace. Run that command in the new profile.

Each item that can be copied starts ticked. Permissions that turn the
tool's checks off (such as Claude Code's `bypassPermissions`, or Codex's
`danger-full-access`) are offered, but never ticked for you.

The profile's sign-in, history and caches are never copied. A setting with
a secret written in it, such as a token in a permission rule, stays behind,
and the line says how many did. Settings only copy between profiles of the
same tool, because each tool's permissions work in their own way.

Anything that couldn't be copied is shown before you sign in.

### Later, or for a copy

Every profile's menu has **Copy settings from…**. It lists what this
profile lacks from another profile of the same tool, each with a box.
Choose **Copy 2 settings** (or however many are ticked). Nothing the
profile already has is changed.

Every copied setting can be undone in Settings › Switching › Context packs
& secrets. An undo takes out only what was copied, and only while it's
unchanged.

## Your default profile

New sessions start on the default profile, unless a project has its own
(set it from the project's menu). To choose the default, open a profile's
menu and choose **Make the default**. With no default, new sessions start
on the profile you used last.

## Launch settings

Launch settings sit under a profile: a name (for example "Deep work"), a
model, and environment overrides. Choose **Edit launch settings** in the
profile's menu. A session started on that profile uses them.

Under the launch settings' options, **If a session crashes** can follow
Settings, always restart it, or leave it ended. See
[Keep work going](keep-work-going.md).

## Renewal dates

In Settings › Accounts › **Renewal dates**, enter when each subscription
renews if you'd like a reminder, and how many days before. AI Juggler
never reads your billing and never guesses a date. The pane has a link to
the Accounts page.

## Rename or remove a profile

- **Rename…** changes only the name AI Juggler shows.
- **Remove…** takes the profile out of AI Juggler. Its past sessions stay
  in History, marked as removed.
  - A setup used where it is stays on your Mac exactly as it was.
  - For a separate copy, you choose: **Keep the folder** in AI Juggler's
    data, or **Move it to the Trash**, where you can still get it back.

An account has nothing to remove: it leaves the page when none of your
profiles is signed in to it.
