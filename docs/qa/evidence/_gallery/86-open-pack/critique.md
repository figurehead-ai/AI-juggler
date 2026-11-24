<!-- captures: 7afa6650b6df -->
# 86-open-pack: New session from a saved handover, what it carries, where to work, launch

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

The dialog lifts clearly off the blurred page in both themes, and "Launch on Work Max" is one solid violet fill, the only primary, with Close beside it. The handover box is a tidy two-column list (From, Where, Continues as, Carries, Doesn't carry, Withheld) with labels a step dimmer than values, and the brief opens as a monospace block that stays readable on both the dark and the light grey well. In dark the lighter accent now shows on the payments-api chip, the handover box's outline, the chosen tile's outline and Launch on Work Max. The second pass changed only the blurred page behind the dialog, in both themes. Under "Where do you want to work?" the chosen payments-api tile is tinted violet, but a cyan dot sits on the unchosen ai-juggler tile, so two different marks compete for "this one"; the footer's "Launching in payments-api" settles it, but only after reading. The dashed "Choose folder… or drop it anywhere" tile sits alone in its row with the right half empty, and "Don't use it", the way out of the handover, is plain grey text at the box's top right that is easy to miss.

## Weak

- summary-1280x800-dark.png, summary-1280x800-light.png: the cyan dot on the unchosen ai-juggler tile competes with the violet tint on the chosen payments-api tile.
- summary-1280x800-dark.png, summary-1280x800-light.png: "Don't use it" is low-weight grey text for the only way to start without the handover.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B2)

Re-captured after F53, both themes. New session from a saved handover is New session's sheet with a "Starting from a saved handover" block on top (From, Where, Continues as, Carries, Doesn't carry, Withheld, the brief folded open), then the stated choice "Start in payments-api on Claude Code CLI · Work Max with the handover", More options open, and the one primary "Start in payments-api with handover ⌘↩". The 0.3.2 notes on folder tiles no longer apply.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | |
| 2 | Prominence | 4 | The handover first, then where. |
| 3 | Default | 5 | |
| 4 | Mode | 4 | |
| 5 | Mock-up | 4 | |
| 6 | Areas | 4 | Two tinted blocks stacked. |
| 7 | Colour | 4 | |
| 8 | Type | 4 | Labels muted, values ink. |
| 9 | Actions | 4 | "Don't use it", the way out, is plain muted text at the block's corner. |
| 10 | Serious | 4 | Withheld said. |
| 11 | Doors | 4 | |
| 12 | Words | 4 | "From Claude Code · Work Max" where the sheet below says "Claude Code CLI · Work Max". |
| 13 | States | 4 | |
| 14 | Consistency | 4 | |
| 15 | Accessibility | 4 | |
| 16 | Hand walk | 4 | Preview. |
| 17 | System prompts | 5 | None. |
| 18 | Usability | 4 | |

Lowest: 4.
