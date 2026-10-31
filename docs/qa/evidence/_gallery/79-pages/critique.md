<!-- captures: 3abaf219812e -->
# 79-pages: account, project, session and settings pages at 1024 and 1280

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

account, session-usage and session-terminal now separate their cards from the canvas in both themes; the Limits card, the four usage tiles and the "Its account's window" card read as distinct areas, and the session's chip row (Working · 1 min, claude) sits clearly under the title. At 1024x680 the pages fold the rail to marks and keep the main content full width; the bottom cards are cut by the window with a "More below" cue, which is acceptable. outside-session puts Go to it as the solid violet primary, but in the 1280 rail the selected row's hover buttons overlap its title and "Idle". tool now shows Settings › Accounts (Accounts and profiles, Tools on this computer, Renewal dates); its renewal date fields are unstyled native "mm/dd/yyyy" inputs, out of step with the styled chips around them. project-overview-1024x680 and project-sessions-1024x680 were not re-rendered this round and still show the 0.3.1 colours and older header, including a native select.

## Weak

- outside-session-1280x800 (both themes): the selected rail row's hover buttons overlap its title and "Idle" state.
- tool-1024x680, tool-1280x800: renewal dates are native date inputs, unlike the app's styled controls.
- project-overview-1280x800: the "Accounts used" tile cuts its list to "claude, claude-client-acme, cla…".
