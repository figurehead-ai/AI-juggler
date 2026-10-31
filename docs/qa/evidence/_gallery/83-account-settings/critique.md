<!-- captures: 3c08c8918c74 -->
# 83-account-settings: copying one account's settings into another or a new account

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

collapsed, nothing and open show the Add a fresh account form with a styled "Start with the same settings as" chip and a solid violet "Create and sign in"; nothing explains in one line that Personal has nothing to copy. open and account-sheet list each item with its own checkbox and a plain explanation, and walk through the github token step as numbered instructions with a command and Copy button. The command block breaks lines in the middle of words ("--sc / ope", "g / ithub", "GITHU / B_TOKEN"), which makes it hard to read and check before pasting. account-sheet-done lists what was set up with an Undo beside each and a solid violet Done. signin keeps a two-step header and one Claude Code row with a solid violet Sign in.

## Weak

- account-sheet-dark/light, open-dark/light: the command block wraps mid-word, splitting flags, names and the token placeholder.
- collapsed, open (both themes): the "Its MCP servers, permissions and status line •" line ends in a stray bullet that reads as a glitch.
