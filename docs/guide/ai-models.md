# AI models, and how we test them

AI Juggler can write a short review when an agent's turn ends, and answer
Chat's questions in your own words. Both are Beta and off until you turn
them on. They run on this Mac: nothing you write is sent anywhere.

Open **Settings › AI › Models** to see which model AI Juggler recommends
for your Mac, the other models we tested, what is installed and how much
disk it uses, and which model each job uses.

## Which model you get

- **With Apple Intelligence on,** AI Juggler uses Apple's on-device model.
  It is part of macOS: nothing to download, no disk used.
- **Without it,** AI Juggler recommends one model to download: the
  smallest one that passed our tests and fits your Mac. Today that is
  Qwen3 1.7B, a 1.8 GB download.
- **One downloaded model is enough.** It covers every job it passed. A
  second one is never downloaded unless you say yes to it, and you are
  asked each time.
- **A model your Mac can't run isn't shown.** A model that only needs more
  free disk is shown, with how much it needs.

### Disk and memory

- A download starts only when your Mac has the model's size **plus 10 GB**
  free, so the rest of your work keeps its room. If free disk drops below
  that while it downloads, it stops and keeps nothing.
- Each model shows the **smallest Mac we list it for**, by its memory (for
  example "8 GB Mac or more"). We set it from the most memory the model
  used in our tests, measured at the context size we tested with: twice
  that peak, to leave room for AI Juggler, macOS and your agents, rounded
  up to the next common Mac size (8, 16, 18, 24, 32 GB …). When the result
  comes within 10 % of a size, we go one size up, to be safe. Longer
  contexts than ours may need more. Apple's model needs what macOS itself
  requires for Apple Intelligence.
- **Clear cache never removes a model.** Remove it in **Installed**: the
  size it frees is shown before you confirm.

## How we test models

Each model is tested job by job, against the plain version AI Juggler
shows without any AI (a line written from the facts, a list of what it
found).

- **The test data is made up for the tests.** We built sets of turns,
  Chat questions, digests and watch requests that look like real work.
  **No real messages were used:** the figures on the Models screen come
  from these synthetic sets, so they may differ on your own work.
- **Blind checks.** For each item, a separate AI reviewer saw the model's
  version and the plain one without knowing which was which, and picked
  the better one. "Preferred in 89 % of blind checks" means the model's
  version won 89 times in 100.
- **Mistakes count most.** A model fails a job if its text states
  something the facts contradict (an agent says it pushed when it didn't,
  for example). AI Juggler's own checks also stop such a claim being shown
  as a fact, whatever the model writes.
- **Speed** ("Quick", "Moderate") is how fast the model answered on our
  test Mac. Your Mac may be faster or slower.

### What each verdict means

- **Works well:** passed, and on by default.
- **Beta:** passed, and offered as a Beta you turn on yourself.
- **Not yet:** tested, and the plain version is still better. That job
  keeps its plain version.

Attention hints need no model at all: they are built from the facts.

### When a model changes

The list of tested models can change without an app update. A model we
stop listing keeps working on your Mac until you remove it, with a note
saying so.
