<!-- captures: 24cd37272ba3 -->
# 84-privacy: Settings › Privacy, what AI Juggler reads, keeps and sends, the tools' terms, and the terms note on the import screen

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

On page, terms and tests-open the cards lift clearly off the canvas in both themes, section headings sit above brighter sentence text, and the test names in tests-open are underlined monospace links with their file paths a step dimmer, readable in dark and light. terms is the strongest screen: one block per tool with its letter mark, a sentence, the link and a "checked Oct 23, 2026" date, all on one rhythm. On page the Usage statistics card says "Off. Nothing is counted, and there is nothing on this computer." while none of its three radios (Share weekly usage stats, Keep stats on this computer only, Off) is selected, so the control does not show the state the sentence states; the radios also sit in boxes inside a fieldset inside a card, three borders deep. The rail on page, terms and tests-open shows "ai-jugg… · 3 … Probably asks you", losing the age to the ellipsis like the known "Stopped with an error · 7 …" row. add-account-dark and add-account-light were not re-rendered and still show 0.3.1 colours (a near-white canvas in light, the paler "Bring in 1 account" fill).

## Weak

- page-dark.png, page-light.png: "Off. Nothing is counted" with no radio selected; the effective choice isn't marked.
- page-dark.png, page-light.png: option boxes inside a fieldset box inside a card, three nested borders for one choice.
- page-dark.png, terms-dark.png, tests-open-dark.png (and light): the rail row "ai-jugg… · 3 … Probably asks you" cuts the age.
- add-account-dark.png, add-account-light.png: not re-rendered; still the 0.3.1 colours.
