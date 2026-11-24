<!-- captures: 0c00adf41030 -->
# 59-vault: the Cloud backup recovery code: saving it before backups start, and the waiting state in Settings

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The "Save your recovery code" dialog is clear. The code sits large in a monospace well with Copy and Save… under it, an amber box says plainly that a lost code can't be recovered, and "Start backups" stays dim until "I've saved my recovery code" is ticked. Once it is ticked (confirmed), though, Start backups still only gains a thin violet outline. The solid violet primaries that the switch and wrong-account dialogs got did not reach this dialog, so the finished step looks barely different from "Later — no backups until then". In dark the checkbox and that outline now use the lighter violet; nothing else in the dialog changed. In waiting, the Figurehead & Cloud card stands off the canvas, and its amber "Cloud backup waits until you save your recovery code" strip with Show recovery code is the right focal point. The rail's ai-juggler row now reads "Stopped with an error" whole instead of ending in a cut "· 7…". But the page still opens scrolled: the Diagnostics card above has no visible heading, and its paragraph starts just under the top edge.

## Weak

- confirmed-1280x800-dark/light: the enabled "Start backups" has only a thin violet outline, not the solid primary fill, so it hardly outranks "Later — no backups until then".
- waiting-1280x800-dark/light: the page opens scrolled, and the Diagnostics card's heading is above the top edge.

## 0.5.1

Re-captured on 0.5.1; only the shared chrome changed in these screens. The rail's head now starts past the traffic lights, its rows hold a fixed dot column (tools on hover), cards and panels separate by tone instead of a border, a chosen item is a fill, and the top bar's bottom line is calmer. The recovery-code dialog is unchanged; Start backups is a filled primary button.

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
