<!-- captures: efdf1bf37daf -->
# 11-renewal: an account's page, meant to show its renewal date in each state (countdown, warning, danger, date only, unknown), the editor, and the chip on the accounts list

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The account captures are framed on their subject again: each scrolls to the Settings card, where Plan and renewal shows a green "signed in" chip and the plan, a renewal line, the "Renewal date (optional)" field with Clear, "Entered by you · AI Juggler does not read billing" and a Flat fee field. Each state reads differently: account-countdown says "Renews in 10 days" in plain text, account-warning an amber "Renews in 6 days" chip, account-danger a red "Renewal date passed" chip, account-date-only "Renews 2024-11-02" and account-unknown "Expiry: unknown" with an empty mm/dd/yyyy field and a greyed Clear. The chips are calm and colour-coded, and the field sits right under its state. Two things are off: date-only writes the date as 2024-11-02 while the field beside it shows 11/02/2024, and unknown says "Expiry" where every other state says "Renews" or "Renewal". editor-1440x900 is the same image as account-warning, and editor-1280x800 is that state at a smaller width, so the editor is shown filled but never mid-edit. accounts-row-chip-1440x900 now shows the amber "Renews in 6 days" chip at the right of the Work Max row on the Accounts list. Above the Settings card, a cut sliver of the Sessions list and an empty card bottom fill the top of every account capture. All captures were re-rendered.

## Weak

- account-date-only-1440x900: "Renews 2024-11-02" beside a field showing 11/02/2024; two date formats.
- account-unknown-1440x900: "Expiry: unknown" uses a different word from the other states' "Renews".
- editor-1440x900: identical to account-warning-1440x900; no edit in progress is shown.
- account-*, editor-*: the top of the panel is a cut list and an empty card bottom above the Settings card.
