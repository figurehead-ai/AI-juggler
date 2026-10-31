<!-- captures: 27c88954154f -->
# 27-ranking: how accounts are ranked for new work: the Home queue, the bell's notifications, and the New session dialog's Ranked for you list with its Why this order? explanation

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

In notification the bell popover is a clear raised panel in both themes, but every one of its five items carries an equally solid violet button, so nothing leads, and the same action is labelled two ways ("Continue elsewhere" and "Continue on another account…"); at 1280x800 the last item's button is cut by the popover footer. In ranked and why the New session dialog groups well (folders as cards, a ranked list with a Suggested chip, then Which account? and a solid violet "Launch on Work Max"), but at 1280x800 the Which account? card is cut at the bottom of the scroll area in ranked and pushed out of sight in why, and in why-1440x900 its second line is sliced. The suggested account reads "82% headroom" in the ranked list but "58% left, from its latest session" in Which account? within the same why dialog, and "82% left this week ·" in ranked ends on a stray separator. The four queue captures were not re-rendered this round; they still show the older look and can't be judged for 0.3.2 colours.

## Weak

- notification-1280x800-dark.png, notification-1440x900-light.png: five equal solid violet buttons in one popover; two different labels for the same continue action.
- notification-1280x800-dark.png, notification-1280x800-light.png: the last notification's "Continue on another account…" button is cut by the popover footer.
- ranked-1280x800-dark.png, ranked-1280x800-light.png: the Which account? card is cut at the bottom of the dialog's scroll area.
- why-1440x900-dark.png, why-1440x900-light.png: the Which account? card is sliced mid-line; it shows 58% left while the ranked list above says 82% headroom.
- ranked-1440x900-light.png (and dark): "82% left this week ·" ends on an orphan separator.
- queue-*.png: not re-rendered this round (still the older render).
