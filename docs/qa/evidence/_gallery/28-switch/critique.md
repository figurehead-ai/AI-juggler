<!-- captures: 4c54f93c4798 -->
# 28-switch: switching a conversation to another account: the "Continue on another account" list, the 3-step switch dialog's Confirm step in its variants, the running and failed states, and the notes and switch-anyway prompts

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The Confirm step has a sound reading order in both themes: step chips, a large "Ready to switch to Personal" title, a short bullet summary, an amber Before you switch notice, then the stop-the-original radios with "recommended" in green, and the dialog panel stands well off the dimmed page in dark. The footer primaries ("Switch now", "Retry", "Switch anyway") are now solid violet, so the main action clearly outweighs Back and Cancel in every Confirm, notes, switch-anyway and failed capture; in dark the radio, checkbox and Confirm chip carry the lighter violet. At 800x600 the page behind the dialogs now keeps its top bar on one row, where the first pass had it wrapping. The long variants still clip: confirm-account-steps and confirm-set-up at 1280x800 end mid-section at the scroll edge (in confirm-set-up the github MCP row is sliced), confirm-token-steps opens scrolled so the title is gone and at 800x600 the github row's own heading is above the top edge, the token command wraps mid-word ("--sc / ope", "u / ser"), and dialog-stop-800x600 cuts the selected option's hint under the footer. targets-out-of-room shows the "Continue on another account" dialog over the session title (cut to "feat/"), with no dimmed backdrop in dark, and every option carries the same amber Beta chip, so the warning colour stops meaning anything. failed says "resume it on Work where you left it" (the account is Work Max) yet offers only Cancel and Retry, and executing lists five steps with no mark of which one is running.

## Weak

- confirm-token-steps-800x600-dark.png, confirm-token-steps-800x600-light.png: opens scrolled; the github row's checkbox and name are above the visible area.
- confirm-token-steps-1280x800-*.png: the command block wraps mid-word, hard to read or check.
- confirm-set-up-1280x800-*.png, confirm-account-steps-1280x800-*.png: the last set-up row is cut at the scroll edge.
- dialog-stop-800x600-*.png: the selected "Stop it now and switch" hint is cut by the footer.
- confirm-set-up-800x600-*.png: the set-up section the capture is named for is entirely below the fold.
- targets-out-of-room-1280x900-*.png: no backdrop in dark; the dialog covers the session title; the same amber Beta chip on all three accounts; in Codex and Personal "Not enough use to forecast" sits right-aligned on its own line, leaving a gap.
- failed-1280x800-*.png, failed-800x600-*.png: "resume it on Work" names the account wrongly, and there is no resume button.
- executing-*.png: no current-step or done marks on the five steps.
- notes-conflict-1280x900-*.png: "Before you switch" and "Set up on Personal" are two headings stacked with nothing between them.
