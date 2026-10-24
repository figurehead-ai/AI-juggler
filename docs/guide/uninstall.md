# Uninstall

1. **In AI Juggler, open Settings › Advanced › Before you uninstall** and
   choose **Prepare to uninstall**. It turns off measuring Claude limits for
   sessions started elsewhere on every profile where it's on, which takes
   AI Juggler's line out of each profile's `settings.json` (any edits you
   made since stay), and lists what's left for you to remove.
2. **Optional: undo what you set up through AI Juggler.** MCP servers,
   skills and settings it copied into a profile, and rules files it
   changed, were your choice and stay where they are. To take any out, use
   its **Undo** in Settings › Switching › Context packs & secrets first. If
   you turned on **Open AI Juggler at login**, turn it off in Settings ›
   Sessions › Keep work going.
3. **Quit AI Juggler** and move it to the Trash (and **AI Juggler Nightly**,
   if you have it).
4. **Delete its data** if you don't want to keep it:
   - `~/Library/Application Support/AI Juggler`
   - `~/Library/Application Support/AI Juggler Nightly`

   This holds the copies of setups you chose to copy, backups, settings and
   history.
5. **Optional:** in Keychain Access, delete the items named
   `ai.figurehead.juggler` and `ai.figurehead.juggler.vault` (and the
   `.nightly` ones). They exist only if you linked a Figurehead account or
   turned on cloud backup, in Settings › Advanced › Figurehead & Cloud.
6. **Optional:** AI Juggler may have created worktrees for parallel
   sessions, in a folder beside the project named `<project>.juggler`, on
   branches named `juggler/…`. They're kept on purpose, in case they hold
   your work. Remove them with `git worktree remove` when you're done.

Apart from what you chose to set up (step 2), your own `~/.claude` and
`~/.codex` folders, and everything in them, stay as they are.
