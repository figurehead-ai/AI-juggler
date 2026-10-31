<!-- captures: ca5528b77105 -->
# 63-consent: the "Usage statistics changed" dialog asking again over Home

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The dialog reads well in both themes and at both sizes. The three choices are full-width boxes with a bold title and a quieter line under each, the selected "Share weekly usage stats" is outlined and tinted, and Save is now one solid violet fill that clearly outranks the outlined "Ask me next time". Behind it, the Needs you and Runway by account cards stand off the darker canvas (dark) and sit white on grey (light), and "Open its page" and New session turn solid violet too. At 1024×680 the dialog covers most of the two columns, so only fragments of the Runway card peek out on its right ("ax", "58% left", "0% left"), which looks untidy but is not wrong for a modal. In the 1280 rail, the first ai-juggler row reads "Stopped with an error · 7…", losing the age. All four captures were re-rendered.

## Weak

- reask-1280x800-dark/light: rail row "Stopped with an error · 7…" drops the age behind the ellipsis.
- reask-1024x680-dark/light: text fragments of the Runway card show at the dialog's right edge; a scrim or a narrower dialog would hide them.
