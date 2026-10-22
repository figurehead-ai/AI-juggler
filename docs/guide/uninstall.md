# Uninstall

1. **Turn off measuring Claude limits for sessions started elsewhere**, on
   each account where you turned it on. That restores the account's
   `settings.json` exactly as it was.
2. **Quit AI Juggler** and move it to the Trash (and **AI Juggler Nightly**,
   if you have it).
3. **Delete its data** if you don't want to keep it:
   - `~/Library/Application Support/AI Juggler`
   - `~/Library/Application Support/AI Juggler Nightly`

   This holds the copies of setups you chose to copy, backups, settings and
   history.
4. **Optional:** in Keychain Access, delete the items named
   `ai.figurehead.juggler` and `ai.figurehead.juggler.vault` (and the
   `.nightly` ones). They exist only if you linked a Figurehead account or
   turned on cloud backup.
5. **Optional:** AI Juggler may have created worktrees for parallel
   sessions, in a folder beside the project named `<project>.juggler`, on
   branches named `juggler/…`. They're kept on purpose, in case they hold
   your work. Remove them with `git worktree remove` when you're done.

Your own `~/.claude` and `~/.codex` folders, and everything in them, stay as
they are.
