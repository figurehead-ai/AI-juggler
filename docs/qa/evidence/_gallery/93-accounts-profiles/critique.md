<!-- captures: d15b01fdf1ad -->
# 93-accounts-profiles: Accounts with one, two and six profiles

Design critique (D-57, AJ-E2814) against the reference screens: the row menus (77-menus), the rail's order menu, Usage and Processes. The baseline note says what changed on purpose; this says whether it is good. Written at the 0.3 second re-render, the first time these captures were taken since 0.1 (the spec lives in session-pages-gallery.spec.ts).

## Checklist

| Check | Finding |
|---|---|
| Grid | One card per account on the page's width: its head (tool mark, colour dot, name, address · plan), its runway and 7-day chart, then one row per profile. |
| Hierarchy | Account name, then its address and plan; "3 profiles share this account" on the head's right; profile rows below in smaller type. |
| Spacing | Rows on the 8 px rhythm; the chart band is the card's tallest part. |
| Icons | The tool's mark on each account, a lettered badge in the profile's colour on each profile ("Wo", "2", "CA"). |
| Shared controls | Open and ⋯ are the shared row actions; Add an account is the page's one primary. |
| Disabled reasons | "Nothing open" beside Open instead of a greyed control. |
| Plain words | The account and profile explainer at the top; "No current reading. Limits come from the Claude app's meter while it's open on this profile…". |
| Type sizes | Body 14 px, profile paths and hints 12–13 px; nothing under 12 px. |
| Both themes | Checked dark and light. |
| 1280×800 and 1024×700 | 1280 only (the spec's size). |

## Good

- The account versus profile distinction is said once at the top and then shown: one card, several profile rows, "3 profiles share this account".
- Each card shows usage at a glance (the 7-day tokens chart) even before limits are measured.

## Weak

- "Work, Work 2 and 1 more" as an account's name is built from its profiles' names; the address under it is what tells two accounts apart.
- With nothing open, the rail shows "Also: History ›" above "Nothing open yet", the footnote before the message.
- The ring with "–" for an unmeasured window is a large empty mark; the words beside it ("Not measured yet") carry the meaning.
- The explanation under the chart is one long line that runs to the card's edge at 1280, its link at the very end and cut to "Measure its limits…" (accounts-one-profile, accounts-two-profiles, accounts-six-profiles).

## Fixed

- Nothing in this family this round: the captures had not been re-taken since 0.1 (top bar with Home · Accounts · History · Settings · More, the old rail filters, two bars per window). They now show the 0.3 header, the rail's App · Project · Account switch and the account card with its runway ring and chart.

## Captures

| Capture | What is on it | What changed since the last capture |
|---|---|---|
| accounts-one-profile-1280x800 | One account "Work" (o…@work.com · Max), not measured, 64M tokens in 7 days, one profile row "Work · Claude Code ~/…/w1". | The 0.3 header and rail; the runway ring and 7-day chart replace two empty bars; the profile's folder is shortened to ~/…/w1. |
| accounts-two-profiles-1280x800 | "Work and Work 2" (o…@work.com · Max): 2 profiles share this account, 102M tokens in 7 days, rows Work (~/…/w1) and Work 2 (~/…/w2). | Same. |
| accounts-six-profiles-1280x800 | Three accounts: "Work, Work 2 and 1 more" (Acme · Team, 3 profiles share it), "Personal and Weekend" (2), and one more below the fold. | Same. |

## 0.3.2

The account cards stand off the page in both themes: in dark the card is a clear step lighter than the near-black canvas, and in light the white cards with a soft shadow sit on the grey canvas, so the one-, two- and three-profile cards (accounts-one-profile, accounts-two-profiles, accounts-six-profiles) read as separate objects. Add an account is one solid violet fill and the obvious primary on the page; the rail's New session stays the pale secondary beside it. After the design fixes the dark captures carry the lighter accent on the logo mark, Add an account and the top bar's New session, which now read a touch brighter; the light captures changed only in the top bar, where "This Mac · Plenty of room" moved by a sub-pixel and the bar stays one row with the whole "7 to clean up" pill. The explainer sentence under "Accounts" and the "No current reading…" line are brighter than the dim labels ("Last 7 days, in and out", "Not measured yet"), so the sentence level is visible. The "No current reading…" line still runs the full card width at 1280 and ends in the truncated link "Measure its limits…", and the rail still puts "Also: History ›" above "Nothing open yet". In accounts-six-profiles the second account's card is cut by the fold, which is the scroll position, not a layout fault.
