<!-- captures: 0e968d61359f -->
# 38-flows-cde: Add an account's tool cards and the New session dialog with its room-left checks

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

Add an account (c-provider-cards, c-provider-cards-unchecked) is a tidy list of tool cards, each with its status after the name and one solid violet "Claude Code →", "Fake Desktop →" or "Codex →" on the right; Gemini CLI, not installed, gets a quieter "Install Gemini CLI →" link, and the unchecked state says "Found; couldn't check its version" in plain words. The New session dialog keeps its order (how it runs, where, ranked for you, which account) and in e-launch-ok and e-launch-tight its one solid "Launch on Work Max" in the footer is clearly the action, with the "Tight — this session may use most of the room left" chip spelling out the risk. The dialog surface is a step lighter than the dimmed page in dark and white in light. Since the first pass, the lighter dark accent shows on the ai-juggler tile's ring, the "used last here" chip, the ai-juggler tag and "Launch on Work Max"; at 390 only the blurred top bar behind the dialog changed (the short broom form of the cleanup pill). The 390 problems inside the dialog are unchanged: the footer breaks, with "Launch on Work Max" wrapping onto two lines inside its button and "Launching in ai-juggler" wrapping beside it, and in e-launch-tight-390x844 the tight chip runs past the card to the dialog edge and is cut at "room le…". In e-launch-expanded the account rows' "Not measured yet ·" still leaves a dangling separator when the line wraps at 390. The second pass changed only the blurred rail rows behind the 390 dialogs; the dialogs, and their footer problems, are as they were ("Launching in ai-" / "juggler" still breaks at the hyphen). All captures were re-rendered.

## Weak

- e-launch-ok-390x844-*, e-launch-tight-390x844-*: "Launch on Work Max" wraps to two lines inside the button, and "Launching in ai-juggler" wraps next to it at the hyphen ("ai-" / "juggler").
- e-launch-tight-390x844-*: the "Tight — this session may use most of the room left" chip overflows its card and is cut at the dialog edge.
- e-launch-expanded-390x844-*, e-launch-ok-390x844-*: "Not measured yet ·" ends its line with a dangling dot.
