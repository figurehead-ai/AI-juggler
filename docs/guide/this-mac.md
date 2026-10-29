# This Mac

AI tools can use a lot of memory and CPU, especially with several sessions
and the Claude or Codex app open at once. This Mac shows what your AI tools use,
measured where they run. Nothing on it is sent anywhere.

Open it from Home's This Mac panel (**See all**), from the This Mac line
in the list on the left, or from More › This Mac.

## What it shows

- **Now:** how your Mac is doing, in words, with each reason. Then a card
  each for memory, the processor, swap and AI Juggler itself.
  - **Memory** shows how much of the whole Mac's memory is in use, and a
    gauge split into the AI tools, other apps and macOS, what macOS keeps
    compressed, and what's available.
  - **Memory pressure** is macOS's own measure of how hard it is working
    to make room: it compresses memory and moves some of it to disk
    (swap). Pressure can be high while some memory still looks available,
    and everything slows down while it lasts. Whenever macOS reports it,
    This Mac says so beside the figures; **What it means** says more.
  - CPU is counted the way Activity Monitor counts it: 100% is one core
    busy.
  - A desktop app's memory is an estimate: the sum of its helpers, so
    memory they share is counted more than once.
- **What's using it:** each AI tool, with its sessions or windows: the
  sessions AI Juggler started, then each tool's desktop app (the Claude
  app; the Codex app, which comes inside ChatGPT and is listed as
  "ChatGPT (Codex)"), then each tool you run in a terminal ("claude in a
  terminal", "codex in a terminal"), then any other app's AI sessions
  the Processes page finds, grouped by the app they run in and each by its
  title. A tool's own helpers count with it, such as the command-line
  `codex` the Codex app runs. Sessions AI Juggler didn't start are marked
  "not started by AI Juggler". AI
  Juggler itself is listed too, with its crash watchdog. Last, folded,
  **Other apps and macOS** lists the biggest apps that aren't AI tools, by
  name only (never their folders or how they were started). AI Juggler
  only reads these, and never stops them.
- **Working your Mac hard:** each session or window using a lot right now.
  A session AI Juggler started links to its page.
- **Alerts and advice:** recent alerts and what to do about them. When
  your Mac is short of memory, an alert names what holds most of it: an
  AI tool only while the AI tools hold a third or more of what's in use,
  otherwise the biggest other apps.
- **Left behind:** processes and ports a finished session left running,
  such as a dev server.

AI Juggler only offers controls for what it started. A window of a tool you
opened yourself has none.

## Clean up what's left behind

Under **Left behind**, choose **Clean up** (or press ⌘⇧C). It opens
[Processes](processes.md), on what needs you: what each session left
running, what it costs, and whose it is. Choose **Stop…** on a process to
stop it: AI Juggler checks it again and tells you what stopping it will do
before you confirm. A process AI Juggler can't be sure of has no button.

Automatic cleanup is off by default: AI Juggler stops nothing by itself.
To let it tidy left-behind dev servers, turn it on for a profile under
**Automatic cleanup** on the Processes page, or on the Accounts page.
Settings › Advanced › Cleanup sets what new profiles start with, and
**Pause automatic cleanup** stops it for every profile until you resume
it.

## Kept awake

While AI Juggler's sessions work, This Mac says so, for example "Kept awake
while 2 AI Juggler sessions work", or why it isn't. See
[Keep work going](keep-work-going.md).
