<!-- captures: 21fbf912ed28 -->
# 59-vault: the Cloud backup recovery code: saving it before backups start, and the waiting state in Settings

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The "Save your recovery code" dialog is clear. The code sits large in a monospace well with Copy and Save… under it, an amber box says plainly that a lost code can't be recovered, and "Start backups" stays dim until "I've saved my recovery code" is ticked. Once it is ticked (confirmed), though, Start backups only gains a thin violet outline. It is not the solid violet primary used elsewhere in 0.3.2, so the finished step looks barely different from "Later — no backups until then". In waiting, the Figurehead & Cloud card stands off the canvas, and its amber "Cloud backup waits until you save your recovery code" strip with Show recovery code is the right focal point. But the page opens scrolled: the Diagnostics card above has no visible heading, and its paragraph starts at the top edge. All six captures were re-rendered.

## Weak

- confirmed-1280x800-dark/light: the enabled "Start backups" has only a thin violet outline, not the solid primary fill, so it hardly outranks "Later — no backups until then".
- waiting-1280x800-dark/light: the page opens scrolled, and the Diagnostics card's heading is above the top edge.
