<!-- captures: 4e7280d47bb6 -->
# 90-processes: the Processes page, what sessions and apps run and left behind, ports, cleanup history, automatic cleanup and stopping, at 1280, 1680 and 2000

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The four count tiles (7 to clean up in amber, 1 didn't stop in red, 1 waiting, 24 running) and the process table now stand as raised cards in both themes, and the table keeps a clear hierarchy: app group rows, session rows with a dimmer summary, then processes with coloured state words and marks (amber Left behind, red Didn't stop, cyan Working) and small CPU sparklines. At 1280 the Owner column drops and group summaries truncate ("payments · 4 processes · 3.0 …"); at 2000 the page is a centred column with wide gutters and its 12 px type looks small. In every light capture with Automatic cleanup (ended-*, history-*, ports-*, rules-preview) the off switches are pale grey tracks on white that nearly vanish, unchanged from the last render. other-apps opens with the table scrolled so the first visible row's top half ("Working", "In a session Claude runs") sits under the sticky column header, the filters popover ends on its "App" heading with the options below it cut, and large's group row reads "2500 %" CPU, a sum that looks like an error. The stop dialogs use a coral (dark) or red (light) destructive button that names the PID and port, readable, but their radios sit far right of their labels; the rail repeats "ai-jugg… · 3 … Probably asks you" with the age cut.

## Weak

- history-1280x800-light.png, ended-1280x800-light.png, ports-1680x1000-light.png, rules-preview-1680x1000-light.png: the off switches in Automatic cleanup are nearly invisible on white.
- other-apps-1680x1000-dark.png, other-apps-1680x1000-light.png: a half-hidden row under the sticky table header.
- filters-1680x1000-dark.png, filters-1680x1000-light.png: the Filters popover ends on its "App" heading with the options cut.
- large-1680x1000-dark.png, large-1680x1000-light.png: "2500 %" CPU on the group row reads as a mistake.
- stop-1680x1000-dark.png, stop-1680x1000-light.png: the What to stop radios sit at the far right, away from their labels.
- attention-1280x800-dark.png and the other 1280/1680 captures: the rail row "ai-jugg… · 3 …" cuts the age.
