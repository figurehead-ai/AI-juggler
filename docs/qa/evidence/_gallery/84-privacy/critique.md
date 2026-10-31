<!-- captures: d6d291886626 -->
# 84-privacy: Settings › Privacy, what AI Juggler reads, keeps and sends, the tools' terms, and the terms note on the import screen

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

On page, terms and tests-open the cards lift clearly off the canvas in both themes, section headings sit above brighter sentence text, and the test names in tests-open are underlined monospace links with their file paths a step dimmer, readable in dark and light; in dark those links, the ticked states and New session now use the lighter accent. terms is the strongest screen: one block per tool with its letter mark, a sentence, the link and a "checked Oct 23, 2026" date, all on one rhythm. On page the Usage statistics card still says "Off. Nothing is counted, and there is nothing on this computer." while none of its three radios (Share weekly usage stats, Keep stats on this computer only, Off) is selected, so the control does not show the state the sentence states; the radios also sit in boxes inside a fieldset inside a card, three borders deep. The rail on page, terms and tests-open still shows "ai-jugg… · 3 … Probably asks you", losing the age to the ellipsis. add-account-dark and add-account-light are now re-rendered on 0.3.2: "You already have 2 Claude Code setups on this computer." over a two-step header, the setups grouped under The default setup and Other Claude setups with Signed in in green and Needs sign-in in grey, "Use them where they are" as the chosen violet-outlined card, and "Bring in 1 account" as a solid violet primary in both themes, with the Consumer Terms note under a rule at the bottom.

## Weak

- page-dark.png, page-light.png: "Off. Nothing is counted" with no radio selected; the effective choice isn't marked.
- page-dark.png, page-light.png: option boxes inside a fieldset box inside a card, three nested borders for one choice.
- page-dark.png, terms-dark.png, tests-open-dark.png (and light): the rail row "ai-jugg… · 3 … Probably asks you" cuts the age.
