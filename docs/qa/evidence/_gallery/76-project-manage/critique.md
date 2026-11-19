<!-- captures: a163d9e042f7 -->
# 76-project-manage: project and session menus, forget, relocate, rename and pins

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

project-actions and project-menu show the project menu with icons, grouped by rules, and a red Forget… last; forget-project's dialog is short and keeps Forget a quiet red outline. relocate puts a red "Project directory is missing" banner with Relocate… above the overview cards, which stand off the canvas in both themes, but the Activity chart's date labels are still cut by the card's bottom edge. session-menu now shows its subject: the feat/read-model row's menu open in the rail (Open its page, Show its project, then Continue on another account… ⌘⇧H, Rename…, Move to pool…, Snooze, Copy the resume command, Pin to top), wide enough that "Continue on another account…" is whole; it runs a few pixels past the rail's right edge onto the page. project-menu still captures the rail scrolled so far that the App / Project / Account switch is half cut under the top edge. rename-session is now re-rendered: the label reads "Name" in sentence case instead of small caps and Save is solid violet in dark instead of pale. After the design fixes the dark accent is lighter on the Tokens this week sparkline, the selected radio cards in settings-editor and the rail's New session outline, and the rail's feat/read-model row reads "Stopped with an error" whole in project-actions, project-menu and session-menu. settings-editor's theme, density and order choices are radio cards with the chosen one tinted, and rail-pins shows pinned projects cleanly.

## 0.4.6 (R4: Keep it apart…)

New: keep-apart-1280x800 and keep-apart-made-1280x800, both themes. Before, "Keep it apart…" opened New pool's nine steps; it now opens one sheet, here from the project's Agents card. keep-apart shows the sheet whole at 1280×800 with no scroll: the pool tile beside one plain sentence, a drawing of ai-juggler's sessions inside one closed accent frame with the way to "Other pools" stopped at its edge, three facts side by side with icons ("In this pool only", "Nothing in or out", "Still a team inside"), an example line under them, a neutral info box for what it doesn't do (no account tied, nothing started or stopped), "What does this mean?" as a link, then what it will make ("«Sealed client project»… join as Member and may Work together"). The foot follows the kit: Cancel as a ghost on the left, Customise… as the secondary, and Keep apart as the one filled primary, focused, with a check. No amber or red: it's a choice, not an alarm. keep-apart-made shows the result in place: the toast "ai-juggler is kept apart", naming the pool, with Open it, and the Agents card now reading "Kept apart: yes · in Sealed client project…" with Open it ›. Both themes hold the frame's accent and the facts' column dividers.

## Weak

- relocate-1280x800 (both themes): the Activity chart's date labels are cut by the card's bottom edge.
- project-menu-1280x800 (both themes): the rail's App / Project / Account switch is half hidden under the top edge.
- session-menu-1280x800 (both themes): the row menu is wider than the rail and overhangs its right edge onto the page by a few pixels.
- keep-apart-1280x800 (both themes): the facts' icons are small grey 24 px tiles, quieter than the kit's 32 px; the pool's name ("Sealed client project", New pool's own) doesn't name the project.
- keep-apart-made-1280x800 (both themes): the page is scrolled to the Agents card, so the card above it is cut at the top edge ("is one.").
