<!-- captures: 9ec1850dcc48 -->
# 63-consent: the "Usage statistics changed" dialog asking again over Home

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The dialog reads well in both themes and at both sizes. The three choices are full-width boxes with a bold title and a quieter line under each, the selected "Share weekly usage stats" is outlined and tinted, and Save is one solid violet fill that clearly outranks the quieter "Ask me next time". In dark, Save, the selected ring and tile, Open its page, the runway bar and New session now use the lighter violet, which lifts them off the dimmed page. Behind the dialog, the Needs you and Runway by account cards stand off the darker canvas (dark) and sit white on grey (light). At 1024×680 the dialog covers most of the two columns, so fragments of the Runway card still peek out on its right ("58% left", "0% left", "Measure its limits ›"), which looks untidy but is not wrong for a modal. In the 1280 rail, the first ai-juggler row now reads "Stopped with an error" whole; the first pass cut it to "Stopped with an error · 7…".

## Weak

- reask-1024x680-dark/light: text fragments of the Runway card show at the dialog's right edge; a scrim or a narrower dialog would hide them.
