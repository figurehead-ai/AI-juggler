<!-- captures: 0c260564e16e -->
# 25-costs: the Prices and details dialog: what usage would cost at API list prices, per session and day, with stale pricing and unpriced models

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

The dialog has a clear hierarchy: three summary cards (At API list prices, You pay, No list price) carry large values over small labels and sentence-level hints, then a plain estimate sentence with Export CSV, then the table and a bold total. In dark the dialog and its cards sit a visible step above the blurred page, and in light the white cards get a soft shadow on the white dialog, so the cards still separate. The stale-pricing warning is a single amber line that reads well in both themes, and in unpriced-model the "at least ~$0.21", "No list price" cells and the "without GLM-6-X, codex-auto-review" note on the total explain the gap honestly at desktop widths. At 390 px wide the table drops to Session and USD, names are cut ("Refactor the billin…", "Review the releas…"), and in unpriced-model-390x844 the total becomes "Total, all sessions…", which hides the very qualifier saying two models are left out, so the total looks complete when it isn't. The design fixes changed only the blurred page behind the dialog in the 390 px captures (the header's controls moved under the blur); the dialog itself is unchanged, so these problems remain.

## Weak

- unpriced-model-390x844-dark.png, unpriced-model-390x844-light.png: "Total, all sessions…" is cut, so the note that two models are not counted is lost on the total row.
- priced-390x844-*.png, unpriced-model-390x844-*.png: session names are cut to a few words, and Day and Model are dropped, so two rows can't be told apart by day.

## 0.5.1

Re-captured against 0.5.1: the dialog sits on a tonal step with a soft shadow in light and keeps its summary cards and table unchanged; Export CSV is a tonal button, and the stale-pricing line is amber words with no fill. F88 (F51's rule): the six 390×844 captures are removed and no longer captured: 390 is below the window's minimum (1024×680), a layout no one can get, and they had no gate baseline; the Weak notes on them above are history. The family is the matrix at 1440, 1280 and 1024.

Nothing broken (checked both themes).

## 0.5.1 final round (2026-11-24)

Re-captured once on the release HEAD after the last fixes. What changed: Behind the dialog the rail and Needs you are in their 0.5.1 style; the dialog itself is unchanged. Every state was compared with its last capture (the sweep's pixel diff, a Sonnet checker opening base, head and diff; Opus checked each suspect: none broken). Scores unchanged.
