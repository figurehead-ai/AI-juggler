<!-- captures: 5990d8648928 -->
# 06-inspector: the main panel for an account's sessions, a project's sessions and a session's terminal, and Home with nothing opened

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

account-sessions-* and project-sessions-* show the same five-row session list in a raised card: a name, a coloured status line, the account and an age per row, with clear tabs above ("Overview / Sessions", and on the project "Sessions 3" plus "1 failed / 1 stuck / 1 working" chips). The list is short, so about half the panel below it is empty canvas; the account page's breadcrumb still reads "Claude Code › Account", placing the account under a tool. session-terminal-* matches the session page of 05-session-panel, with its cards a clear step off the canvas and no solid violet competing in the panel. empty-1280x800 and empty-1440x900 are still just Home with no panel opened, so "empty" shows no empty panel state. The design fixes show here as the lighter accent on the logo mark, the selected Sessions tab, "Open its page" and the rail's New session button, a one-row top bar, and rail error rows that read "Stopped with an error" with no cut age. All captures were re-rendered.

## Weak

- empty-1280x800, empty-1440x900: Home with no panel; no empty panel state is shown.
- account-sessions-*, project-sessions-*: the list status "Stopped with an error" is amber, while the session header shows the same state in red.
