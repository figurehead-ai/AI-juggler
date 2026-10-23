# When a provider is down

AI Juggler doesn't detect outages. One failed request isn't an outage, and
moving work on a false alarm does more harm than moving it by hand. When you
decide a provider is down, move each session you need yourself:

1. Open the session.
2. Choose **Switch account…** (or press ⌘⇧H). The accounts you can go to
   are listed, ranked. One on another tool says what carries over and what
   doesn't.
3. Pick one, check what's carried, and confirm with **Switch ▸** (or
   Enter). If something goes wrong, the screen says which step and why, and
   the original session is left as it was.
4. Repeat for each session you want to keep working on.

When the provider is back, the session's panel offers **Resume the original
conversation ▸**, which carries the work back to where it started, with
everything done in the meantime.

[Continue on another account](switching.md) explains what each kind of
switch carries.

There's no "move the whole project" button yet: we want to learn from real
outages first. If this happens to you, please tell us what happened in
[Discussions](https://github.com/figurehead-ai/AI-juggler/discussions).
