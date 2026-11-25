<!-- captures: 2bf7e774d45b -->
# 11-renewal: an account's page, meant to show its renewal date in each state (countdown, warning, danger, date only, unknown), the editor, and the chip on the accounts list

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The account captures are framed on their subject again: each scrolls to the Settings card, where Plan and renewal shows a green "signed in" chip and the plan, a renewal line, the "Renewal date (optional)" field with Clear, "Entered by you · AI Juggler does not read billing" and a Flat fee field. Each state reads differently: account-countdown says "Renews in 10 days" in plain text, account-warning an amber "Renews in 6 days" chip, account-danger a red "Renewal date passed" chip, account-date-only "Renews 2024-11-02" and account-unknown "Expiry: unknown" with an empty mm/dd/yyyy field and a greyed Clear. The chips are calm and colour-coded, and the field sits right under its state. Two things are off: date-only writes the date as 2024-11-02 while the field beside it shows 11/02/2024, and unknown says "Expiry" where every other state says "Renews" or "Renewal". editor-1440x900 is the same image as account-warning, and editor-1280x800 is that state at a smaller width, so the editor is shown filled but never mid-edit. accounts-row-chip-1440x900 now shows the amber "Renews in 6 days" chip at the right of the Work Max row on the Accounts list. Above the Settings card, a cut sliver of the Sessions list and an empty card bottom fill the top of every account capture. All captures were re-rendered.

## Weak

- account-date-only-1440x900: "Renews 2024-11-02" beside a field showing 11/02/2024; two date formats.
- account-unknown-1440x900: "Expiry: unknown" uses a different word from the other states' "Renews".
- editor-1440x900: identical to account-warning-1440x900; no edit in progress is shown.
- account-*, editor-*: the top of the panel is a cut list and an empty card bottom above the Settings card.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5, batch B3)

Re-captured (account states at 1440, the editor at 1440 and 1280, the Accounts row chip); opened at full size: account-warning-1440x900 and editor-1280x800. Each state shows in the account header's chip (countdown, "⚠ Renews in 6 days", "⛔ Renewal date passed", the date, none when unknown); the editor is Settings › Plan, renewal and fee: Plan, Renewal date (a date picker), Monthly fee, and "Entered by you … AI Juggler doesn't read your billing."

- **Closed since 0.3.2:** two date formats and "Expiry: unknown" (the unknown state shows no chip; the date is the picker's); editor captures the Settings tab, not the warning page again.
- **Weak, logged F76:** "⚠ Renews in 6 days" is an amber chip with a fill behind its words.
- **Weak, kept:** the editor's Monthly fee Save is live while its field is empty.

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 4 | Know when a plan renews, and set it. |
| 2 | Prominence | 4 | The chip in the header where the account is named; the field in Settings. |
| 3 | Default | 4 | Nothing set until the user enters it; reminders said in the row. |
| 4 | Mode | 4 | Calm header; Settings rows. |
| 5 | Mock-up | 4 | AJ-E411's chip and field. |
| 6 | Areas | 4 | Header, tabs, settings rows by tone. |
| 7 | Colour | 4 | F76 (6311ad3cd): "⚠ Renews in 6 days" is amber words and mark on the neutral chip's tone; red for passed. |
| 8 | Type | 4 | Row titles ink, hints muted. |
| 9 | Actions | 4 | New session the page's primary; Save per row. |
| 10 | Serious | 4 | n/a. |
| 11 | Doors | 4 | The chip sits on the account it's about; Settings is its tab. |
| 12 | Words | 4 | "Renews in 6 days", "When the plan renews: reminders come before it." |
| 13 | States | 4 | Unknown shows nothing and says so in Settings. |
| 14 | Consistency | 4 | The Settings rows of every page. |
| 15 | Accessibility | 4 | axe clean in the capture runs (no serious or critical); keyboard and zoom not re-walked this round. |
| 16 | Hand walk | 4 | Not re-walked by hand here; captured from the fixtures, both themes. |
| 17 | System prompts | 5 | Causes no system prompt. |
| 18 | Usability | 4 | Re-read against PAT-USE: entered by the user, said so, never read from billing. Match. |

Lowest: 3, each line under 4 logged (F76); the rest 4 or 5. After F76: lowest 4.

**Fixed after the critique (F76), re-captured (every state; opened account-warning-1440x900 beside its old capture, the chip cropped):** "⚠ Renews in 6 days" sits on the neutral chip's grey with amber words and ⚠ (it was an amber-brown fill); the Accounts row chip the same. Line 7 3 → 4; lowest now 4.
