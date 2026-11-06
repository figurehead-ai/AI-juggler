<!-- captures: e263dd174314 -->
<!-- captures: pending -->
# 99-account-quality: how an Account's sessions went

Design critique (D-57) against the Account page (99-account-page) and a session's signals (92-session-pages). New family (AJ-E2903).

## Checklist

| Check | Finding |
|---|---|
| Grid | A section under Limits; each signal a row: its name in a left column, the figure and its source on the right. |
| Hierarchy | The figure bold, its source muted under it; a figure that isn't reported is plain, not bold. |
| Spacing | The section's row rhythm; the pending note under the rows. |
| Icons | The gauge icon Limits uses. |
| Shared controls | Section. |
| Disabled reasons | "Codex doesn't record errors in its files", "Not reported by Codex" in place of numbers. |
| Plain words | "9 errors or retries / 300 turns" (never a bare percentage), "2 times probably stuck", "1 session just ended: its figures come in a moment." |
| Type sizes | 14 px figures, 12 px sources. |
| Both themes | Checked. |
| 1280×800 and 1024×700 | Both; the two columns stack below 560 px of content. |

## Good

- Each figure says where it comes from, as a session's signals do, and there's no overall score.
- A tool without the data says so in its own name rather than showing zeros.

## Weak

- "Not once" for stuck episodes is plain where a reader might want reassurance; it's honest but quiet.
- The section shows no comparison with the account's own earlier weeks.

## Fixed

- Nothing on first render.
