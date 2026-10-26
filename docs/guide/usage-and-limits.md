# Usage and limits

AI Juggler shows two kinds of numbers, and it never mixes them up.

Limits and money belong to an **account**, the subscription you pay for.
Sessions run in a **profile**, the setup an app runs on. Several profiles
signed in to one account share its limits. See
[Accounts and profiles](accounts.md).

## Usage: what you've spent

Tokens and cost per session, per profile, per account and per project,
counted from the tools' own records on your Mac: Claude Code's transcripts
and Codex's session logs. It doesn't ask any provider for anything.

- A conversation that was resumed counts its earlier messages once, under
  the session where they first happened.
- Costs use the providers' published prices. A model without a published
  price says "No price for this model yet" instead of $0.

### Monthly caps

If you work for clients, or keep an eye on what an API key costs, set a
monthly cap: **Usage** › **Set a monthly cap**. Give it a name, an amount
in US dollars, and what it counts: one account, or a group of projects on
any account.

- **Needs you** on Home tells you when this month reaches 80 % of the cap,
  and again at 100 %. **Got it** hides the warning until the next one, or
  next month.
- **Nothing is ever stopped.** A cap only tells you; your sessions keep
  running.
- The month is your calendar month, and the amounts are the estimates
  above, at the providers' published prices. Usage without a price isn't
  counted, and the cap says so.
- On a subscription, the estimate is what your use would cost by the
  token, not what you pay, so a cap there measures how much of the plan's
  value you use.

## Limits: how much room is left

Your plan's 5-hour and weekly limits, as the tool itself reports them.

- **"Not measured yet"** means the tool hasn't reported this limit to AI
  Juggler. AI Juggler never guesses a percentage. It shows the tokens you've
  used instead.
- **"As of 12 min ago"** says when the limit was last measured. A limit
  changes as you use it, so an older reading is less certain.
- **"Not measured since the reset"** means the limit has reset and nothing
  newer has been measured.
- A reset time is shown only when the tool reported one.

### Codex

Codex records its limits in its session logs, so AI Juggler can show them
for every Codex session, wherever you started it, along with your credit
balance. It shows only the limits Codex reports for your plan.

### Claude Code

AI Juggler reads Claude's limits from Claude's own records on your Mac,
with nothing to set up:

- **The Claude app's meter.** While the Claude app is open on a profile,
  it records the 5-hour and weekly use about every 15 minutes. A reading
  shows where it came from and how old it is: "60% left · from the Claude
  app, 12 min ago".
- **Limit hits.** When Claude Code hits a limit, it records which limit it
  was and when it resets: "Hit its limit at 2:02 PM · resets 7:00 PM".
- **Claude Code's last `/usage`.** When you run `/usage`, Claude Code keeps
  its answer, with both windows and their resets, and any weekly limit on
  one model only.

A 5-hour reading older than five hours says nothing about now, so it shows
as history: "Last reading 13 h ago: 40% used; that window has reset since".

**A week on one model.** Some plans limit one model by the week, on its
own: a team plan may have only that, with no weekly limit for all models.
It shows as its own line, "This week · Fable only: 100% left · resets
Thu", beside the 5-hour window and, when your plan has one, the week for
all models. It limits only that model, so it doesn't change how much room
the account has for other work; when it's running low, the account's line
says so ("Fable: 8% left this week"). The Claude app's meter shows only the
all-models windows, so until Claude Code's `/usage` has run, the week says
that the meter shows no all-models week and when the rest will show.

Sessions AI Juggler starts report their limits through Claude Code's status
line. For sessions you start with `claude` in a terminal, there's an option
per profile, off by default. The Claude app never runs a status line, so its
sessions don't need it:

1. On the Accounts page, open the profile's menu.
2. Choose **Measure its limits…**, then **Also measure terminal sessions…**.
3. Review the change it shows you: one setting, `statusLine`, in that
   profile's `settings.json`. Your own status line keeps working; AI
   Juggler only reads its limits and passes the rest through.
4. Confirm.

Turning it off restores the file exactly as it was. If you've edited the
file since, it removes only AI Juggler's part.

Because a limit belongs to the account, one measured session counts for
every profile signed in to that account.

## How long a limit will last

Each account's card on the **Usage** page shows, for its 5-hour window and
its week:

- **Sessions left:** for example "About 6 usual sessions left (each about
  10%)". A usual session is the median of your past sessions on that
  account, so it appears after a few of them. Until then it says when it
  will show.
- **A burn-down chart:** what's been used so far, a dashed line for where
  the current pace leads, and the reset at the right edge. The projection
  turns amber, and ends at a "runs out" line, when the limit would run out
  before the reset.
- **A line under the chart:** "At this pace it runs out about 3:40 PM,
  before the reset at 5:00 PM", or "At this pace it lasts past the reset".

These are worked out from the tool's own readings of the window and from
your past sessions, never guessed. With too little to go on, the card says
so instead of showing a number.
