<!-- captures: 0bc9582632c5 -->
# 40-login: the Log in dialog for one account in each sign-in state

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

Each capture is the same small dialog, "Log in · Work Max", with its promise that AI Juggler never sees the credentials, then one card for the state: a status line, a sentence, the green command block and the actions. The primary is a solid violet ("Sign in", "Sign in again", "Reopen sign-in", "Continue") and stands clearly above "Open vendor page", "Copy command" and "Re-check"; the dialog is a step lighter than the dimmed page in dark and white in light. The design fixes barely touch this dialog: in dark the "API key account" chip takes the lighter accent, and at 390 only the blurred top bar behind the dialog changed (the cleanup pill is now the short broom form). So the first-pass problems stand. The status line marks itself twice: a coloured dot and then a ring or disc glyph before the words ("● ○ Sign-in expired"), and expired, didn't-finish and waiting all use the same cyan dot, so the state's urgency isn't carried by colour. card-api_key repeats "API key account" as both the status and a chip. At 390 wide the command's first line, "CLAUDE_CONFIG_DIR='~/Library/Application", runs out of the code box and past the card to the dialog edge in every state that shows it. All captures were re-rendered.

## Weak

- card-expired-390x844-*, card-logged_out-390x844-*, card-partial-390x844-*, card-sso_pending-390x844-*: the command's first line overflows the code box and the card.
- All captures: the status line shows a dot and a ring/disc glyph side by side; expired, didn't-finish, waiting and SSO-pending share one cyan dot.
- card-api_key-*: "API key account" appears twice, as the status and as a chip.
