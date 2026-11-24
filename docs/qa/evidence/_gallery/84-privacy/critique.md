<!-- captures: 8a2b6615432b -->
# 84-privacy: Settings › Privacy, what AI Juggler reads, keeps and sends, the tools' terms, and the terms note on the import screen

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

On page, terms and tests-open the cards lift clearly off the canvas in both themes, section headings sit above brighter sentence text, and the test names in tests-open are underlined monospace links with their file paths a step dimmer, readable in dark and light; in dark those links, the ticked states and New session now use the lighter accent. terms is the strongest screen: one block per tool with its letter mark, a sentence, the link and a "checked Oct 23, 2026" date, all on one rhythm. On page the Usage statistics card still says "Off. Nothing is counted, and there is nothing on this computer." while none of its three radios (Share weekly usage stats, Keep stats on this computer only, Off) is selected, so the control does not show the state the sentence states; the radios also sit in boxes inside a fieldset inside a card, three borders deep. After the second pass the rail on page, terms and tests-open keeps the age whole ("ai-j… · 3 h ago Probably asks you", the project cut instead); nothing else changed. add-account-dark and add-account-light are now re-rendered on 0.3.2: "You already have 2 Claude Code setups on this computer." over a two-step header, the setups grouped under The default setup and Other Claude setups with Signed in in green and Needs sign-in in grey, "Use them where they are" as the chosen violet-outlined card, and "Bring in 1 account" as a solid violet primary in both themes, with the Consumer Terms note under a rule at the bottom.

## Weak

- page-dark.png, page-light.png: "Off. Nothing is counted" with no radio selected; the effective choice isn't marked.
- page-dark.png, page-light.png: option boxes inside a fieldset box inside a card, three nested borders for one choice.

## 0.5.1 design critique (2026-11-24, Opus, VALIDATION §5)

Opened at full size: page (dark), terms (light), tests-open (dark) and add-account (light) from the 0.5.1 shoot, and page (dark) after the fix; beside the calm reference. What 0.5.1 changed: AJ-E1548 R11, the tools' terms are closed until opened (the terms capture opens them on purpose, after checking they start closed; add-account's "Providers' terms" is a closed fold at the foot); #15, a "Configuration updates" section with "Check for configuration updates" (on, D-129) saying what it fetches and when; calm surfaces (the cards by tone, no borders); Settings' rail icons.

- **Before the fix:** under the configuration switch, which is on, the section's footer said "Off, AI Juggler uses the settings it shipped with.", read as the current state. **Fixed** (7e858e860): "When it's off, AI Juggler uses the settings it shipped with." Re-rendered (gate page-dark) and checked by eye.
- **Weak, kept from 0.3.2:** the "Usage statistics, crash reports and search" card opens with an empty band above "Off. Nothing is counted…" and puts its choices in boxes inside a fieldset inside the card (three edges deep). add-account still says "setups" (F52).

| # | Line | Score | Note |
|---|---|---|---|
| 1 | Job | 5 | What it reads, keeps and sends, each line with the tests that check it. |
| 2 | Prominence | 4 | Reads, then config, then stats, then sends; terms last and closed. |
| 3 | Default | 4 | Config updates on with its why; stats Off. |
| 4 | Mode | 4 | Calm settings pane. |
| 5 | Mock-up | 4 | No mock; Settings' pattern. |
| 6 | Areas | 4 | The stats card's nested boxes (above). |
| 7 | Colour | 4 | Accent only on links and switches; test names in the link tone. |
| 8 | Type | 4 | Section titles, row titles, body ink-2; test paths monospace muted. |
| 9 | Actions | 4 | Switches act in place; no primary needed. |
| 10 | Serious | 4 | Each send names what goes and where; Clear in Local cache. |
| 11 | Doors | 5 | Last messages…, Local cache…, every test name opens its file. |
| 12 | Words | 4 | After the fix; "All of it on this computer. Every line below names the tests that check it." |
| 13 | States | 4 | Terms open and closed, tests open. |
| 14 | Consistency | 4 | Settings rows and folds. |
| 15 | Accessibility | 4 | Folds are buttons with aria-expanded (the spec checks closed first). Not re-walked this round. |
| 16 | Hand walk | 4 | PL1 walk visited Settings; not re-walked here. |
| 17 | System prompts | 5 | None; the config fetch is background and silent by design (no OS prompt). |
| 18 | Usability | 4 | AJ-E1548 (below). |

**AJ-E1548 match:** R11 terms never open by themselves: closed in Privacy and in Add account. Match. #15 config updates with what and when, on by default, and Bot's history and watches lines ("kept on this Mac; Clear deletes them"). Config updates match on screen; the Bot and watches lines sit below what page captures, so they are checked by AJ-E1548's tests, not by eye here. (6) "a fetch nobody was told about": it's told. Match.

Lowest: 4 (line 12 was 3 before the fix).
