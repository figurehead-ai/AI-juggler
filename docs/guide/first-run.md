# Your first run

## Find your accounts

On first launch, choose **Find my accounts**. AI Juggler looks for your
Claude Code and Codex setups: `~/.claude`, `~/.codex`, other `~/.claude*` and
`~/.codex*` folders, the folders named by `CLAUDE_CONFIG_DIR` or `CODEX_HOME`
in your shell's startup files, and profiles made by other account managers.
It only reads; nothing is changed by looking.

## Use it where it is, or make a copy

For each setup it finds, you choose:

- **Use it where it is.** AI Juggler works with the folder you already
  have. Your existing sign-in, settings and history stay exactly where they
  are, and AI Juggler adds nothing to the folder unless you turn on an
  option that says so first.
- **Make a separate copy.** AI Juggler copies the setup into its own data
  folder and uses the copy. Your original is never changed. The copy doesn't
  include the sign-in, so you sign in once inside the copy.

You can change your mind later: add or remove accounts from the Accounts
page.

## Signing in

When an account needs a sign-in, AI Juggler opens that tool's own sign-in,
in a terminal. You sign in exactly as you would by hand. AI Juggler never
sees your password or your tokens.

## What you'll see

- **The list on the left** shows your projects and their sessions, including
  sessions you started in a terminal or in the Claude app.
- **Home** shows what needs you, how much room each account has left, and
  how your Mac is doing.
- **New session** starts the tool you choose, on the account you choose, in
  the folder you choose.

## Optional permissions

AI Juggler works without any special permission. Two are optional:

- **Accessibility and Automation**, to bring a session's own window to the
  front when you choose it.
- **Local Network**, only if you turn on Fleet to see your other Macs.

macOS asks for each one when you first use the feature that needs it.
