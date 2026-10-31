<!-- captures: 5cd0d156fcb3 -->
# 25-costs: the Prices and details dialog: what usage would cost at API list prices, per session and day, with stale pricing and unpriced models

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

The dialog has a clear hierarchy: three summary cards (At API list prices, You pay, No list price) carry large values over small labels and sentence-level hints, then a plain estimate sentence with Export CSV, then the table and a bold total. In dark the dialog and its cards sit a visible step above the blurred page, and in light the white cards get a soft shadow on the white dialog, so the cards still separate. The stale-pricing warning is a single amber line that reads well in both themes, and in unpriced-model the "at least ~$0.21", "No list price" cells and the "without GLM-6-X, codex-auto-review" note on the total explain the gap honestly at desktop widths. At 390 px wide the table drops to Session and USD, names are cut ("Refactor the billin…", "Review the releas…"), and in unpriced-model-390x844 the total becomes "Total, all sessions…", which hides the very qualifier saying two models are left out, so the total looks complete when it isn't.

## Weak

- unpriced-model-390x844-dark.png, unpriced-model-390x844-light.png: "Total, all sessions…" is cut, so the note that two models are not counted is lost on the total row.
- priced-390x844-*.png, unpriced-model-390x844-*.png: session names are cut to a few words, and Day and Model are dropped, so two rows can't be told apart by day.
