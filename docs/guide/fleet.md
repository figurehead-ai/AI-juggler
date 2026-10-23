# Your other Macs (Fleet)

**Beta.** Fleet puts your own Macs in one window: see their sessions and
usage, start a session on one, or carry a conversation to it. Your Macs
connect directly to each other over your network. Nothing relays the
traffic, and no Mac ever reads another's sign-ins.

Fleet has been checked with two copies of AI Juggler on one Mac, not yet on
two real Macs. Tell us how it goes in
[Discussions](https://github.com/figurehead-ai/AI-juggler/discussions).

## Turn it on

Fleet is under **More › Fleet** (⌘4) until you pair a Mac; then it's in the
header too.

**Link** is off by default. While it's off, no other Mac can see or reach
this one. Turn **Link on** on both Macs to pair them. macOS asks for the
Local Network permission the first time: Fleet needs it to find your Macs.

## Pair two Macs

1. On the first Mac, choose **Add a machine…**. It shows a 6-digit code,
   good for two minutes.
2. On the second Mac, choose **Join a machine…**, pick the first Mac from
   the list, and enter the code. Not on the same network? Connect the Macs
   with Tailscale or WireGuard first, then use **Pair by address instead**.
3. Both Macs show a **fingerprint**. Check it matches the one the other
   Mac shows under **This Mac**, then choose **Fingerprints match**. If
   they don't, cancel: you may not be pairing with your own Mac.
4. Choose what the other Mac may do on this one:
   - **View only** (the default): it sees this Mac's accounts, sessions and
     usage, and can't change anything.
   - **View + control**: it can also start and stop work here, with launch
     settings that already exist on this Mac. It can never read sign-ins.

A wrong code pauses pairing for a minute after five tries in a row, and turns
pairing off after 20 in a day (**Turn pairing back on** brings it back).

## What a paired Mac sees

Which sessions run where, with their project and account names, states and
usage totals. Never your sign-ins, files or prompts. A conversation reaches
another Mac only when you switch one to it.

## What you can do from here

- **Watch** another Mac's sessions, live, in the list and on Fleet.
- **Stop** a session there, or bring its window to the front (**Stop on…**,
  **Focus on…** on the session's page).
- **Clean up** a leftover process there that AI Juggler can show it started.
- **Start a session there:** in **New session**, choose the Mac under
  **Runs on**, then the folder and account it offers, and **Start there →**.
- **Carry a conversation to it:** on a session's page, **Continue on another
  machine**. The switch screen says what carries over first.
- **Usage by machine:** **Last 24 hours, by machine**, on Fleet and on Usage,
  shows each Mac's tokens, estimated cost and sessions.

Watching and usage work under **View only**. Stopping, focusing, cleaning,
starting and switching need **View + control** on the other Mac, which
checks the permission, that the request is fresh and that the session
exists before it acts. A request older than 30 seconds, or sent twice, is
refused.

## Who decides

Only the Mac being controlled can change what another may do: **Access…** on
its row in Fleet, or Settings › **Fleet permissions**. **View + control, ask
each time** makes each request wait for you to **Allow** it on that Mac; one
you don't answer within 20 seconds is denied.

## Unpair, or start again

- **Unpair…** on a Mac's row: it no longer connects. To use it again, pair
  it again with a new code.
- **Reset this Mac's identity…** gives this Mac a new fingerprint and removes
  every paired Mac.
- **Trust broken — re-pair required** means a Mac presented a key this one
  doesn't know. Nothing connects until you pair it again.

## How it stays private

Your Macs prove who they are to each other, and each accepts only the key it
saved when you paired. Found on your network by Bonjour, or through your
Figurehead account if you turn **Find my Macs** on (it arrives in an
upcoming beta). Figurehead only lists your Macs; they still connect directly
and only after you compare fingerprints. See
[What AI Juggler touches](trust.md).
