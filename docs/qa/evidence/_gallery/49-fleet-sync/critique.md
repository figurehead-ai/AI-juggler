<!-- captures: 765cd211a74d -->
# 49-fleet-sync: Fleet with a paired Mac mini synced, offline with queued changes, with clock skew, and Home filtered to the offline Mac

Design critique (D-57, AJ-E2814). First critique of this family, written at the 0.3.2 re-render from its captures as they are now.

## 0.3.2

Fleet reads as two clear columns of raised cards (Link on and Paired machines on the left, This Mac on the right), with Add a machine… as the solid violet primary in the header. synced shows Mac mini's state as a row of chips (green Connected · mTLS · v1.0, Synced, View only, Access…) and its sessions with Working and Asks you, and offline-queued swaps in Offline and an amber "↑ 12 changes queued" plus "was working" history, which tells the story at a glance. In the offline captures an extra strip above ("Mac mini — offline, last seen…" with Retry and Unpair…) repeats the Last seen line and the Unpair… button that the card shows again just below. clock-skew adds its "⚠ Mac mini's clock is 12 min ahead…" warning as plain grey text, easy to miss. offline-board opens the View menu filtered to Mac mini, but the page is scrolled a few pixels so the "5 sessions need you" heading is cut by the top bar. Every capture was re-rendered.

## Weak

- offline-board: the "5 sessions need you" heading is cut along its top by the top bar.
- offline-queued, clock-skew: Unpair… and the last-seen time appear twice, in the offline strip and again in the Mac mini card.
- clock-skew: the clock warning has no warning colour and reads like ordinary help text.
- Rail (clock-skew, offline-queued, synced): "Stopped with an error · 7 …" cuts the age.
