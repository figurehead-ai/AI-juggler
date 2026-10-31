<!-- captures: 546260728f2d -->
# 91-more-tools: adding accounts for more tools, the tool chooser and first run finding four setups

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design passes.

## 0.3.2

add-account-chooser lists four installed tools (Claude Code, OpenCode, Kimi Code, Grok Build), each with a solid violet "<tool> →" button; these are four equal primaries in one small dialog, so nothing says which is the usual pick, and an outlined button per row would carry the same choice more quietly. The dialog itself lifts well off the blurred page in both themes; in dark only its logo mark changed, to the lighter accent. first-run-found is a well-ordered page: a four-step header, a large heading ("You already have 4 setups on this computer."), the setups grouped by tool, then a colour dot, name and path per row, and two choice cards with "Use them where they are" marked Recommended and selected; in dark the step number, the four ticked checkboxes, the selected radio and the selected card's border are the lighter violet and stand out better against the near-black. At 1280 × 800 the capture still ends at "Add a fresh account instead · Import from another folder…", so the page's primary button is below the fold on first run. tools-on-this-mac (one capture per theme; it replaces the three identical grok-build-, kimi-code- and opencode-tool-page captures) shows Settings › Accounts: "Accounts and profiles" with Open Accounts, then "Tools on this computer" as one raised card with a row per tool (lettered mark, name, "1 account · Version … · not tested with AI Juggler yet"), OpenCode's second line "OpenCode 1.19.4 is out; … To update, run opencode upgrade yourself.", the ticked "Look for new versions of your tools once a day", and Renewal dates below. It reads cleanly in both themes, with the tool names bright and their facts a step dimmer, and its name now says what it shows. The tool rows still carry no chevron or button that would open a page of that tool.

## Weak

- add-account-chooser-1280x800-dark.png, add-account-chooser-1280x800-light.png: four solid violet buttons, no single primary.
- first-run-found-1280x800-dark.png, first-run-found-1280x800-light.png: the button that brings the setups in is below the first screen.
- tools-on-this-mac-1280x800-*: under Tools on this computer the rows are not doors; nothing on a row opens that tool's page or its accounts.
- tools-on-this-mac-1280x800-*: the rail shows "Also: History ›" above "Nothing open yet", the footnote before the message.
