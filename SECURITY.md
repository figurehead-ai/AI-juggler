# Security Policy

AI Juggler runs vendor CLIs on your machine, keeps their account
folders isolated, and can stop processes — so we take security reports
seriously.

## Reporting a vulnerability

Please use **GitHub's private vulnerability reporting** on this
repository: *Security → Report a vulnerability*. It reaches the
maintainers privately.

- Please don't open public issues, discussions or PRs for security problems.
- We aim to acknowledge reports within 3 business days and to keep you
  updated until a fix ships. We're happy to credit you in the advisory.
- Include what you found, how to reproduce it, and the impact you expect.

## Supported versions

There are no releases yet. Until `1.0`, security fixes land on `main` and
in the next `0.x` beta. After `1.0`, the latest minor release is supported.

## Scope

In scope:

- The AI Juggler application: Rust core, Tauri commands, UI, updater
- Account isolation: anything that lets one Account's state leak into
  another's, or into your own (unmanaged) install
- Process control: stopping a process AI Juggler doesn't own, or
  bypassing the verify-before-stop checks
- Context packs: secret redaction that doesn't honour the user's policy
- Fleet: pairing, transport security, and remote control between machines
- The update channel: signature checks, downgrade protection
- The optional Figurehead account client: device-token handling,
  backup encryption, and anything that sends data the user didn't turn on

Out of scope: vulnerabilities in the vendor CLIs themselves (report those
to the vendor), and issues that need an already-compromised machine.

## Design commitments

These are binding goals of the architecture (see the [spec](./docs/SPEC.md)
and the [decision log](./docs/review/decision-log.md)):

- **Vendor credentials are never read, copied out, or automated.**
  Sign-in always happens in the vendor's own flow; AI Juggler detects
  login state from metadata only.
- **No traffic proxying or interception.** AI Juggler spawns the real,
  unmodified CLIs.
- **No writes outside AI Juggler's own managed folders.** Imports are
  byte copies; originals are never modified.
- **Local-first, account optional.** With no account, AI Juggler makes
  no calls to figurehead.ai. An optional Figurehead account adds cloud
  features the user turns on one by one; backups are encrypted on the
  Mac and the server stores only ciphertext. Transcripts and vendor
  credentials never leave the Mac.
- **Usage statistics and crash reports are strictly opt-in,** aggregated
  on the device, carry no identifiers, and are documented in
  [docs/usage-stats.md](./docs/usage-stats.md). Until the user chooses,
  nothing is counted and no request is made.
- **Signed updates only**, verified before installation.
- **Fleet links** use mutual TLS with per-machine identity keys and a
  user-verified pairing code; vendor credentials never travel over them.
