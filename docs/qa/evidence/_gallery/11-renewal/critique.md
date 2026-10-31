<!-- captures: 5899d74ebec1 -->
# 11-renewal: an account's page, meant to show its renewal date in each state (countdown, warning, danger, date only, unknown), the editor, and the chip on the accounts list

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

None of the captures shows a renewal date any more. account-countdown, account-danger, account-date-only, account-unknown, account-warning and editor-1440x900 are pixel-for-pixel the same account overview (Limits "Not measured yet", the four usage cards and Usage over time), with no countdown, no "Renews in 6 days" or "Renewal date passed" chip and no date field; the previous render showed those chips and the renewal date editor on the account card. editor-1280x800 is the same overview at a smaller width, and accounts-row-chip-1440x900 shows the Accounts list with no renewal chip on any row. As screens they read well (raised cards, clear numbers, the darker canvas), but as evidence the family no longer proves its feature. All captures were re-rendered.

## Weak

- account-countdown, account-danger, account-date-only, account-unknown, account-warning, editor-1440x900: identical captures; no renewal state or date editor is visible.
- editor-1280x800: no editor is visible.
- accounts-row-chip-1440x900: no renewal chip on the account rows.
