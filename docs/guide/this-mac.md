# This Mac

AI tools can use a lot of memory and CPU, especially with several sessions
and the Claude app open at once. This Mac shows what your AI tools use,
measured where they run. Nothing on it is sent anywhere.

Open it from Home's This Mac panel (**See all**), from the This Mac line
in the list on the left, or from More › This Mac.

## What it shows

- **Now:** how your Mac is doing, in words, with each reason. Then memory,
  CPU and swap.
  - CPU is counted the way Activity Monitor counts it: 100% is one core
    busy.
  - The Claude app's memory is an estimate: the sum of its helpers, so
    memory they share is counted more than once.
- **What's using it:** each AI tool, with its sessions or windows. Sessions
  AI Juggler didn't start are marked "not started by AI Juggler". AI
  Juggler itself is listed too, with its crash watchdog.
- **Working your Mac hard:** each session or window using a lot right now.
  A session AI Juggler started links to its page.
- **Alerts and advice:** recent alerts and what to do about them.
- **Left behind:** processes and ports a finished session left running,
  such as a dev server.

AI Juggler only offers controls for what it started. A window of a tool you
opened yourself has none.

## Clean up what's left behind

Under **Left behind**, choose **Clean up** (or press ⌘⇧C). The Cleanup
screen lists what each session left running, what it costs, and why AI
Juggler thinks it belongs to that session. Choose **Terminate…** on a
process to stop it: AI Juggler checks it again and tells you what stopping
it will do before you confirm. A process AI Juggler can't be sure of has no
button.

Automatic cleanup is off by default: AI Juggler stops nothing by itself.
To let it tidy left-behind dev servers, turn on **Automatically tidy
left-behind dev servers** for a profile on the Accounts page. Settings ›
Advanced › Cleanup sets what new profiles start with, and **Pause automatic
cleanup** stops it for every profile until you resume it.

## Kept awake

While AI Juggler's sessions work, This Mac says so, for example "Kept awake
while 2 AI Juggler sessions work", or why it isn't. See
[Keep work going](keep-work-going.md).
