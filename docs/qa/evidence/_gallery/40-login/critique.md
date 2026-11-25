<!-- captures: 29c4d64754f9 -->
# 40-login: the Sign in dialog for one account in each sign-in state

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

Each capture is the same small dialog, "Log in · Work Max", with its promise that AI Juggler never sees the credentials, then one card for the state: a status line, a sentence, the green command block and the actions. The primary is a solid violet ("Sign in", "Sign in again", "Reopen sign-in", "Continue") and stands clearly above "Open vendor page", "Copy command" and "Re-check"; the dialog is a step lighter than the dimmed page in dark and white in light. The design fixes barely touch this dialog: in dark the "API key account" chip takes the lighter accent, and at 390 only the blurred top bar behind the dialog changed (the cleanup pill is now the short broom form). So the first-pass problems stand. The status line marks itself twice: a coloured dot and then a ring or disc glyph before the words ("● ○ Sign-in expired"), and expired, didn't-finish and waiting all use the same cyan dot, so the state's urgency isn't carried by colour. card-api_key repeats "API key account" as both the status and a chip. At 390 wide the command's first line, "CLAUDE_CONFIG_DIR='~/Library/Application", runs out of the code box and past the card to the dialog edge in every state that shows it. All captures were re-rendered.

## Weak

- card-expired-390x844-*, card-logged_out-390x844-*, card-partial-390x844-*, card-sso_pending-390x844-*: the command's first line overflows the code box and the card.
- All captures: the status line shows a dot and a ring/disc glyph side by side; expired, didn't-finish, waiting and SSO-pending share one cyan dot.
- card-api_key-*: "API key account" appears twice, as the status and as a chip.

## 0.5.1

Re-captured against 0.5.1: the sign-in card is a tonal box inside the dialog, the primary Sign in/Continue keeps the accent fill and the other buttons are tonal; no bright borders remain. Layout is as before. Then F82 (2c99973d7, feab01e21): the dialog's title is "Sign in · Claude Code CLI · Work Max" (was "Log in"); the status line has one mark, its colour carrying the state (green signed in, amber expired or didn't finish, the live dot while waiting), with no ring or disc glyph beside it; the three steps are a short numbered list ("Choose Sign in: Claude Code's own sign-in opens", "Follow Claude Code's prompts", "Come back here: we detect it"); "Open Claude Code's sign-in page" replaces "Open vendor page" and no word on the card says "vendor" or "login"; the card is a tone step with no outline; api_key no longer repeats "API key account" as a chip; at 390 the command wraps inside its box (it ran past the card).

Nothing broken (checked both themes).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
