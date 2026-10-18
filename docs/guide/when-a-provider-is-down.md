# When a Provider is down

AI Juggler does not detect outages. One failed request is not an outage,
and moving work on a false alarm does more harm than moving it by hand.
When you decide a Provider is down, move each Instance you need yourself:

1. Select the Instance in the rail (Sessions or Instances).
2. In the Inspector, find **Continue on another account**. It lists the
   destinations, ranked. A row for a different Provider shows what is
   carried over and what is dropped.
3. Pick a destination. The switch flow opens and shows what is carried
   and any conflicts.
4. Confirm with **Switch ▸** (or Enter). The source Instance is frozen,
   snapshotted, staged and verified before the new one launches. If a
   step fails, the flow says which step and why.
5. Repeat for each Instance in the project you want to keep working on.

When the Provider is back, use **Switch back** in the Instance's
Inspector to return the work to where it started.

There is no "move the whole project" button yet. AJ-E1008 is collecting
real outage reports first. If this happens to you, please tell us what
happened (see `docs/qa/evidence/AJ-E1008/outage-log.md`).
