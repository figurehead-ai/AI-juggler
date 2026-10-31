<!-- captures: 9be71a6a11ca -->
# 27-ranking: how accounts are ranked for new work: the Home queue, the bell's notifications, and the New session dialog's Ranked for you list with its Why this order? explanation

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now; updated after the 0.3.2 design fixes.

## 0.3.2

In notification the bell popover is a clear raised panel in both themes, but every one of its five items still carries an equally solid violet button, so nothing leads, and the same action is labelled two ways ("Continue elsewhere" and "Continue on another account…"); at 1280x800 the last item's "Continue on another account…" button still runs flush into the popover footer's top border, its bottom edge sliced. In ranked and why the New session dialog groups well (folders as cards, a ranked list with a Suggested chip, then Which account? and a solid violet "Launch on Work Max"); in dark the Suggested chips, the Which account? ring and the "Why this order?" focus ring now carry the lighter violet. The layout problems are unchanged: at 1280x800 the Which account? card is cut at the bottom of the scroll area in ranked and pushed out of sight in why, in why-1440x900 its second line is sliced, the suggested account reads "82% headroom" in the ranked list but "58% left, from its latest session" in Which account?, and "82% left this week ·" ends on a stray separator. The queue captures are now re-rendered on the 0.3.2 Home: a violet "Work Max has room again" banner with a solid "Open feat/read-model" button, a Needs you card that leads with Personal and its solid "Open its page", Runway by account with bars and reset times, and Pick up where you left off. The banner button and "Open its page" are two solid primaries one above the other, and the banner button carries a focus ring in every queue capture, so it reads as already pressed.

## Weak

- notification-1280x800-dark.png, notification-1440x900-light.png: five equal solid violet buttons in one popover; two different labels for the same continue action.
- notification-1280x800-dark.png, notification-1280x800-light.png: the last notification's "Continue on another account…" button sits flush on the popover footer's border, its bottom edge cut.
- ranked-1280x800-dark.png, ranked-1280x800-light.png: the Which account? card is cut at the bottom of the dialog's scroll area.
- why-1440x900-dark.png, why-1440x900-light.png: the Which account? card is sliced mid-line; it shows 58% left while the ranked list above says 82% headroom.
- ranked-1440x900-light.png (and dark): "82% left this week ·" ends on an orphan separator.
- queue-*.png: two solid primaries stacked (the banner's "Open feat/read-model" and Personal's "Open its page"), and the banner button shows a focus ring on arrival.
- queue-*.png: Needs you labels the same action "Continue on another account…" on Personal and "Continue elsewhere" on the Personal row below it.
- queue-1280x800-*.png: Pick up where you left off titles are cut ("Keep the old shortcuts wor…", "ai-juggler · feat/read-model · …"); Fake Desk's runway shows a bare "—" on the right.
