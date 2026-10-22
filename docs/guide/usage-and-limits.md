# Usage and limits

AI Juggler shows two kinds of numbers, and it never mixes them up.

## Usage: what you've spent

Tokens and cost per session, per account and per project, counted from the
tools' own records on your Mac: Claude Code's transcripts and Codex's
session logs. It doesn't ask any provider for anything.

- A conversation that was resumed counts its earlier messages once, under
  the session where they first happened.
- Costs use the providers' published prices. A model without a published
  price says "No price for this model yet" instead of $0.

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

Claude Code reports its limits only to its status line. AI Juggler reads it
for sessions it starts. For sessions you start elsewhere, there's an option
per account, off by default:

1. Open the account's page.
2. Turn on measuring limits for sessions started elsewhere.
3. Review the change it shows you: one setting, `statusLine`, in that
   account's `settings.json`. Your own status line keeps working; AI
   Juggler only reads its limits and passes the rest through.
4. Confirm.

Turning it off restores the file exactly as it was. If you've edited the
file since, it removes only AI Juggler's part.

Because a limit belongs to the account, one session started in AI Juggler
also measures it for every other session on that account.
