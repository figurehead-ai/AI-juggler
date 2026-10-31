<!-- captures: ab35e9fb8837 -->
# 83-account-settings: copying one account's settings into another or a new account

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

collapsed, nothing and open show the Add a fresh account form with a styled "Start with the same settings as" chip and a solid violet "Create and sign in"; nothing explains in one line that Personal has nothing to copy. open and account-sheet list each item with its own checkbox and a plain explanation, and walk through the github token step as numbered instructions with a command and Copy button. The command block still breaks lines in the middle of words ("--sc / ope", "--scope u / ser", "comman / d"), which makes it hard to read and check before pasting. account-sheet-done lists what was set up with an Undo beside each and a solid violet Done. signin keeps a two-step header and one Claude Code row with a solid violet Sign in. The design fixes touched only the dark captures: the ticked checkboxes, the Name field's focus ring and signin's step "2" circle use the lighter accent, a little brighter on the near-black dialog; the light captures are unchanged.

## Weak

- account-sheet-dark/light, open-dark/light: the command block wraps mid-word, splitting flags, names and JSON keys.
- collapsed, open (both themes): the "Its MCP servers, permissions and status line ▸" line ends in a small stray triangle that reads as a glitch.
