# Keep work going

Long sessions shouldn't stop because the Mac fell asleep or a tool crashed.
These settings live in Settings › Sessions › **Keep work going**.

## Restart sessions that crash

Off by default. When it's on, a session that crashes is started again on
the same launch settings, in the same folder, with its conversation resumed
when AI Juggler knows which one it was (otherwise it starts fresh).

- At most 3 times in 10 minutes. After that, the session stays ended and
  says why.
- Never after a usage limit, a stop you made, or a failed sign-in.
  Restarting can't fix those.
- Each restart is noted in the session's history.

A launch setting can choose for itself: open the profile's **Edit launch
settings**, and set **If a session crashes** to follow Settings, restart
it, or leave it ended.

## Keep apps alive

Off until you turn it on, for each app or profile: in Settings › Desktop
apps › **Your apps and profiles**, or with a right-click on its place in
the rail. When an app or profile you keep alive stops running (it crashed,
you pressed ⌘Q, or it was force quit), AI Juggler starts it again about
10 seconds later, on the same profile and behind the app you're in. Home
shows a tile for one that's down, with **Start**.

- Each start is a notice, with the way to turn it off.
- At most 3 times in 10 minutes. After that, keep alive pauses for that
  app and says why. Opening the app again ends the pause.
- Never while your screen is locked, in the first minute after AI Juggler
  starts or your Mac wakes, or while AI Juggler is quitting (so logging
  out and shutting down work as always).
- Stopping the app from AI Juggler pauses keep alive for it, until you
  open it again.
- It never changes the account a profile uses, and it starts an app only
  if no other user can change it.

## Keep the Mac awake while sessions work

On by default, when the Mac is plugged in. While at least one AI Juggler
session is working, the Mac doesn't go to sleep by itself.

- Only while a session is working, not while it waits for you.
- The screen can still sleep, and closing the lid or choosing Sleep still
  sleeps the Mac.
- **On battery too** is off by default, because an awake Mac drains its
  battery.

This Mac and Home's panel say when the Mac is kept awake, and why not when
it isn't.

macOS's App Nap is also held off while AI Juggler runs sessions, so they
aren't slowed down when the window is in the background.

## Reopen AI Juggler if it quits unexpectedly

On by default. If AI Juggler crashes, it opens again by itself: never after
you quit it, and at most 3 times in 10 minutes.

When it comes back, the sessions it was running say "ended when AI Juggler
quit unexpectedly", and come back where restarting is on.

A small watchdog, part of AI Juggler, does this. It ends when you quit AI
Juggler, and before an update restarts it. This Mac lists it under AI
Juggler itself.

## Open AI Juggler at login

Off by default, and new in 0.1. Turn on **Open AI Juggler at login**, here or in the
welcome's **Make it smoother** card. macOS may ask you to approve it in
System Settings › General › Login Items, and lists it there.
