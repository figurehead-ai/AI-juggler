# Two Macs: the maintainer's walk (0.4.x, last)

The checks only two real Macs can make, walked by hand, last in 0.4.x
(D-83). They close AJ-E710 R1 and R4 and AJ-E2102 R6. Each step says what
to do and what must happen; write the result in the last column (pass, or
what you saw) and the time where asked. About 30 minutes.

**Before:** both Macs on the same beta build (Settings › About & updates
shows the same version), on the same home or office network (or both on
Tailscale), AI Juggler open on each. Call them **A** (this one) and **B**.

| # | Do | Must happen | Result |
|---|---|---|---|
| 1 | On A, open Fleet. Turn Link on if it's off. Start pairing: A shows a code. Start a timer. | A code shows; A says it's waiting. | |
| 2 | On B, open Fleet › Join a machine. Pick A from the list (or Pair by address if it isn't listed) and type A's code. | Both say paired, each showing the other's name. Under 60 s from step 1. | time: |
| 3 | On B, type the code wrong three times in a new pairing. | Pairing pauses; it says so in words; A isn't paired twice. | |
| 4 | On A, open Home and the rail. | B's sessions show, marked as on B; A's own come first. | |
| 5 | On B, start a short session in a throwaway folder (`ls`). On A, find it. | It shows on A within a few seconds, with B's name. | |
| 6 | On A, open that session and use **Bring forward on B**. | B brings that session's window forward; A says it did. (AJ-E2102 R6) | |
| 7 | On A, switch a conversation from an Account on A to one on B, then back (A→B→A). | It continues on B with its history; back on A with what B added. Nothing duplicated. | |
| 8 | On B, leave a process from step 5 running after its session ends. On A, clean it up remotely from This Mac › B. | Only that process stops; nothing else on B is touched. | |
| 9 | Take B offline (Wi-Fi off) for a minute, then on again. | A shows B as offline with its last state, then live again, with no error loop. | |
| 10 | On A, Reset this Mac's identity, then try a command to B. | B refuses until you pair again; pairing again works. | |
| 11 | On A, open Fleet › B › Details and note the heartbeat round trip; open a session of B's and note how long its page takes; after step 9, note how long B took to catch up. | Measured numbers, written here; they go against the budgets of AJ-E703, AJ-E705 and AJ-E708 in the gate file. (AJ-E710 R4) | RTT: · page: · catch-up: |

**After:** if a third Mac is to hand, pair it too and check step 4 shows
all three (AJ-E710 R1's "one board across 3 machines"). Otherwise write
"two Macs" here; that is the waiver the gate names. Then the two tester
sign-offs AJ-E710 asks for: your name and date below.

Signed: ____________________ Date: __________
