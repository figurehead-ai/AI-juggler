<!-- captures: c1880e1386b4 -->
# 68-accounts: Accounts, Usage, the limit decision, and an account on its credits

Design critique (D-57, AJ-E2814). Written at 0.3.2 for the four new captures of the maintainer's report: a Codex account whose week is used up while its pre-bought credits carry the work on. Before 0.3.2 it was called "out of room". The family's other captures were not re-taken this round, and this critique doesn't judge them.

## Checklist

| Check | Finding |
|---|---|
| Grid | The account card keeps its two gauges and the 7-day chart. The credits line sits between "Limits read" and the chart, in the card's own column. In the switch list, the credits words sit where a row's room always sits, under the badge and the forecast. |
| Hierarchy | "Using credits · 14,150 left" is the row's medium-weight line. What it means for a session ("…goes on with its credits until the window resets Tue 7:13 AM") is the quieter line under it. |
| Spacing | Same rhythm as the low and out-of-room lines (`text-label`, 2 px between the two lines). Nothing new is boxed. |
| Icons | None added. The week's ring stays at 0%, but its empty track is amber, not red: the window is used up and the work isn't stopped. The account above it, at 0% with no credits, keeps its red track. |
| Shared controls | No new controls. The switch row is the same button, "Suggested" stays on the account with room (Personal), and Continue on another account… is unchanged. |
| Disabled reasons | None. An account on its credits can still be picked. |
| Plain words | "Using credits · 14,150 left" (a whole number with separators, never "14150.1635025000"). "Used up, on credits · back in 2 days" under the gauge, where it said "Limit hit". "Sessions on it go on with its credits until the window resets …". No "out of room" for it anywhere. |
| Type sizes | 12–13 px labels as the rest of the row; nothing under 12 px. |
| Both themes | Checked dark and light: the credits words are ink, not warning or danger, and read clearly on both. |
| 1280×800 and 1024×700 | 1280×800 captured; 1024 checked by hand in the preview (the switch list and the card wrap without clipping). |

## Good

- The switch list now has three honest states side by side: Personal with room (Suggested), Codex on its credits (says it goes on until the reset), and Fake Desk at 0 credits ("Out of room until Tue 7:13 AM", red, unchanged).
- The ranking's own words agree with the row ("using its credits · quiet"), so the explanation and the room line never contradict.
- The amber track tells the card's two 0% rings apart at a glance, without a new mark.

## Weak

- The switch row on credits is the tallest in the list: badge, forecast, two credits lines, the "Doesn't carry" line and the factors. In the Continue on another account dialog all three rows still fit at 1280×800.
- "using its credits" in the factors repeats the room line above it. It's kept because it explains the order, but it's the second time the row says it.
- The card's ring still reads "0%". The words beside it carry the credits truth; the ring alone doesn't.
- accounts-credits-1280x800 (both themes): the page opens scrolled, so the Fake Desk card's header (name and email) is cut under the top bar.

## Fixed

- The maintainer's report (0.3.2): the card and the switch list said "Limit hit" and "Out of room until …" for a Codex account that kept working on 14,150 credits. Both now say it's using its credits, and what that means for a session.

## Captures

| Capture | What is on it | What changed since the last capture |
|---|---|---|
| accounts-credits-1280x800 (dark, light) | Accounts, scrolled to the Codex card (c…@example.com · pro): 5-hour window not measured; This week 0% left · resets Tue 7:13 AM · "Used up, on credits · back in 2 days" with an amber track; "Using credits · 14,150 left" and "Sessions on it go on with its credits until the window resets Tue 7:13 AM."; above it, Fake Desk at 0% with a red track and "Limit hit". | New capture. |
| switch-credits-1280x800 (dark, light) | A Work Max session's page with "Ranked for you": Personal (Suggested, 60% left), Codex ("Using credits · 14,150 left", "A session there goes on with its credits until the window resets Tue 7:13 AM.", "using its credits · quiet"), then Fake Desk ("Out of room until Tue 7:13 AM", credits at 0). | New capture. |

## 0.3.2

The credits work reads well in its new home. In switch-credits the list sits in the "Continue on another account · Now on Work Max" dialog, which opens at its top, and all three rows fit inside it: Personal (Suggested, violet outline), Codex on its credits, and Fake Desk out of room in red. On Accounts (accounts-full, accounts-placement, accounts-signed-out), the account cards lift off the darker or grey canvas, the intro is the brighter sentence with bold "account" and "profile", and "Add an account" is a solid violet fill. In limit-decision, the full-width solid "Continue on Work Max" clearly outranks the outlined "Park until 4:13 AM". After the design fixes the dark accent is lighter: it shows on the 58% and 82% rings in accounts-full, the four Usage sparklines in usage-full, the Suggested card's outline in limit-decision and the stepper and checkboxes in first-run-codex; the top bar stays one row everywhere. add-accounts (dark and light) is now re-rendered: it says "setups on this computer", "Bring in 2 accounts" is solid violet instead of the pale gradient, the unselected radio is an empty ring instead of a filled white dot, a terms note sits under the footer, and in light the canvas is grey with white cards. In accounts-credits the amber 0% ring beside "Used up, on credits" still tells it apart from Fake Desk's red ring, but the page still opens scrolled with the Fake Desk card's header cut under the bar. After the second pass, the dark switch-credits shows the rail from its top: search, the App / Project / Account switch, then "Needs you 5" starting with "Personal · Asks you". The rail's feat/read-model row under Open now keeps its time whole ("ai-j… · 3 h ago", the project cut instead) in every capture, and the session terminal behind the switch-credits dialog is black to its card's edge; the other captures changed only in that rail line or in a blurred page behind a dialog.

## 0.3.4

One new capture, switch-setup (dark, light), from the maintainer's report. personal-max read "Ready: same tools set up", then the switch refused it ("hasn't been set up"). Codex was also suggested over a Claude account with room. The family's other captures weren't re-taken.

**switch-setup.** The same "Continue on another account · Now on Work Max" dialog. Lab (70% left, the session's own tool) is first, with the violet Suggested outline. Codex has 100% left but is second, with its amber Beta badge and "Doesn't carry: Subagents' work". Personal is last and held back. Its card keeps the "Continues exactly · Proven" badge, then the reason in three plain lines: the terminal Claude Code's first-run setup isn't done there, sessions in the Claude app don't do it (the maintainer's case: accounts used every day through the Claude app), and what to do. Under the card, "Start a session on Personal ›" in the link colour opens New session already on that account. The card is greyed only by its muted name, never red: nothing is wrong with the account, it needs one step. Axe is clean in both themes.

Weak: the link sits under the card rather than inside it, because the card is the whole row's button. It reads as the card's own next step, but a row whose only use is that link could carry it inside.
