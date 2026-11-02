<!-- captures: 8f618de3f179 -->
# 96-switch-setup: what the other account is missing before you switch, and how to set it up

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

These are close crops of the switch's set-up section: can-set-up lists release-notes (ticked, set up as it is) and github (needs your GITHUB_TOKEN) with numbered Terminal steps, a command block and Copy / Check again; turn-on adds computer-use with Show me how, Open claude.ai settings and its own steps; unused says Personal already has everything, with the extra items folded under one disclosure; nothing-missing is now re-rendered as the "Ready to switch to Personal" summary: a "Continues exactly · Beta · Not proven yet" chip on one line, three bullets (the whole conversation carried over, the original stays, nothing needs you), then an open Details with "Goes along" chips (The whole conversation, Project rules, Your uncommitted changes, Subagents' work) and "Recreated on Personal" (The task list). The words are good: each item says what it is, why it is listed ("Used here") and what you do, and the secret is never copied. The command block is a lighter surface that stands off the darker page and a grey well on white in light; in dark the ticked release-notes checkbox shows the lighter accent. The design fix landed where it mattered most: turn-on's "Switch now" is now the solid violet primary in both themes, beside a quiet Cancel. After the second pass the command block wraps at spaces, so "--scope user", "user github" and "command" are no longer split mid-word; it still breaks at hyphens, though: "github --" / "scope user" and "server-" / "github"]", so the flag and the package name still look split even though Copy pastes the command whole (the "-y" stays whole). turn-on's dialog is still wider than the frame: its × sits on the right edge, the computer-use steps box and Switch now are cut at it, and it now opens scrolled one line further, to "Tick it to set github up…", with its heading, the first items and the "…never copies a secret." line above the fold. nothing-missing says "Personal has everything this account has set up" in its bullets and again as "Personal already has 3 of the 3 things this account has set up" at the bottom, and its text runs flush to the frame's left edge.

## Weak

- can-set-up-dark, can-set-up-light, turn-on-dark, turn-on-light: the command block still breaks at hyphens ("github --" / "scope user", "server-" / "github"); mid-word breaks are fixed.
- turn-on-dark, turn-on-light: the dialog overflows the frame on the right (×, the computer-use steps box and Switch now cut at the edge) and opens scrolled past its heading, release-notes and github's first lines, to "Tick it to set github up…".
- nothing-missing-dark, nothing-missing-light: the content has no left margin (text flush to the frame's edge); "Personal has everything…" is said twice, in the bullets and in the last line.

## 0.3.5

The maintainer's report: the last step said "Continue here in AI Juggler", there was no choice between the Claude app and a terminal, and the wizard showed so much that it overwhelmed. Re-shot: can-set-up, turn-on, unused, nothing-missing; new: where-target-step, where-carried-step, where-desktop, where-carry-open, where-no-desktop, where-done-desktop, where-narrow (both themes).

- **where-desktop:** under "Ready to switch to Personal" and the fidelity badge, "Continue in" offers two cards side by side. **Desktop** has the app's mark, "Claude app · Personal" and a green "Same as the original" tag (the source ran in the Claude app), selected. **CLI** has a terminal mark and "Claude Code in a terminal". Under them, one quiet line: "AI Juggler starts it as it switches." Then one chip, "5 go along · 1 to set up" (warning tint, since one item waits on a decision), and one line: "The original stays open in Claude, untouched. It continues in Claude app · Personal." "Before you switch" keeps only the decision: copy Work Max's notes, one line, with "What does this mean?" closed. Details is closed. The button reads "Switch to Desktop".
- **where-carry-open:** the chip opened. "Goes along" chips are green with an icon each (speech bubble, document, branch, layers), "Recreated on Personal" is accent with a list icon, and "Not on Personal yet" is amber with a pencil.
- **where-no-desktop:** Desktop stays visible but greyed, with why: "No desktop app profile on Personal yet. Open a session on it in the app once, and Desktop is offered here." CLI is selected and the button reads "Switch to CLI".
- **where-done-desktop:** "Switched to Personal", a chip "Opened in Claude app · Personal", "The original stays open in Claude, untouched." and one button, "Open its page".
- **where-narrow:** the cards stack, the chip and lines wrap, and the footer stays put.
- **where-target-step / where-carried-step:** the step pills are tinted (done green with a check, current accent), and the Target step ends with "Nothing changes until the last step, where you pick Desktop or CLI and switch."
- **can-set-up / turn-on / unused:** each set-up row is one line: its name, kind, a coloured verdict tag ("Can copy", "Needs a step from you", "You turn it on") and "Used here". Where it came from, and the token steps, wait behind "What does this mean?" and "Show me how".

### Weak (0.3.5)

- The fidelity badge says "Continues exactly" and "Not proven yet" side by side, which can read as a contradiction. It is the register's honest pair, so it stays until the pair is validated.
- In the preview the Desktop card's mark is a generic app tile. The real app reads the Claude app's own icon at run time.
