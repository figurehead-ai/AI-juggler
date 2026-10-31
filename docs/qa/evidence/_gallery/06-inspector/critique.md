<!-- captures: 83892121710a -->
# 06-inspector: the main panel for an account's sessions, a project's sessions and a session's terminal, and Home with nothing opened

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

account-sessions-* and project-sessions-* show the same five-row session list in a raised card: a name, a coloured status line, the account and an age per row, with clear tabs above ("Overview / Sessions", and on the project "Sessions 3" plus "1 failed / 1 stuck / 1 working" chips). The list is short, so two thirds of the panel below it is empty canvas; the account page's breadcrumb reads "Claude Code › Account", placing the account under a tool. session-terminal-* matches the session page of 05-session-panel, with its cards a clear step off the canvas and no solid violet competing in the panel. empty-1280x800 and empty-1440x900 are just Home with no panel opened, so "empty" shows no empty panel state. The rail's "Stopped with an error · 7…" row cuts its age in all captures. All captures were re-rendered.

## Weak

- empty-1280x800, empty-1440x900: Home with no panel; no empty panel state is shown.
- account-sessions-*, project-sessions-*: the list status "Stopped with an error" is amber, while the session header shows the same state in red.
- all captures: rail row "Stopped with an error · 7…" cuts its age.
