# Security policy

AI Juggler runs your coding tools on your Mac, reads their transcripts,
keeps accounts apart and can stop processes, so we take security reports
seriously.

## Reporting a vulnerability

Use **GitHub's private vulnerability reporting** on this repository:
**Security › Report a vulnerability**. It reaches the maintainers privately.

- Please don't open public issues, discussions or pull requests for security
  problems.
- Include what you found, how to reproduce it, the version (Settings › About
  & updates) and the impact you expect.

What to expect:

- An acknowledgement within 3 business days, and an assessment within 14
  days.
- Updates until a fix ships. We coordinate the disclosure date with you and
  aim to publish within 90 days of the report, sooner when a fix is out.
- A GitHub security advisory for each confirmed vulnerability, with a CVE
  when one applies, and credit to you if you'd like it.

We won't pursue or support legal action against anyone who reports in good
faith under this policy: who avoids privacy violations, data destruction and
service disruption, tests only against their own installs and accounts, and
gives us reasonable time to fix the problem before telling others.

## Supported versions

| Version | Security fixes |
| --- | --- |
| Latest Stable release (0.1 and later) | Yes |
| Latest Beta release | Yes |
| Nightly | Best effort: fixes land in the next Nightly |
| Older releases | No: please update |

## Scope

In scope:

- The AI Juggler app: the Rust core, the Tauri commands, the interface and
  the updater.
- Account isolation: anything that lets one account's state leak into
  another's.
- Your own setups: any write into a Claude Code or Codex folder that you
  didn't choose, or any read of their credentials.
- Process control: stopping a process AI Juggler doesn't own, or getting
  around its checks before it stops one.
- Context packs: secret redaction that doesn't follow your settings.
- Fleet: pairing, transport security and control between Macs.
- Reading other tools' files: anything beyond what the
  [trust page](./docs/guide/trust.md) lists, from Claude Squad's or Vibe
  Kanban's files, or a sign-in or key file read at all.
- The opt-in search index of sessions started outside AI Juggler: anything
  of it left behind after it's turned off.
- Crash reports: anything sent without your choice, or not scrubbed first.
- Updates: signature checks and downgrade protection.
- The optional Figurehead account client: device-token handling, backup
  encryption, and anything that sends data you didn't turn on.

Out of scope: vulnerabilities in the coding tools themselves (report those
to their vendor), and problems that need an already compromised Mac.

## Design commitments

These are binding rules of the architecture (see the
[specification](./docs/SPEC.md), the [architecture decisions](./docs/adr/)
and the [security guidelines](./docs/engineering/SECURITY-GUIDELINES.md)):

- **Credentials are never read, copied out or automated.** Sign-in always
  happens in the tool's own flow. AI Juggler checks only whether a sign-in
  exists, never its contents, and never opens the tools' Keychain items.
- **No proxying or interception.** AI Juggler runs the real, unmodified tools.
- **No writes into your own setups that you didn't choose.** Each one is a
  change you apply, shown before it's made: measuring Claude limits for
  sessions started elsewhere (one setting, restored exactly when you turn it
  off); switching a conversation into an account (it adds files and never
  changes existing ones); and the settings, MCP servers, skills and rules
  files you tick to copy or bring in line, each with an Undo that takes out
  only what was added. A secret is never copied. Copies of whole setups are
  made in AI Juggler's own folder, and the original is never modified.
- **Local-first, account optional.** Without a Figurehead account, AI Juggler
  makes no calls to figurehead.ai. Backups are encrypted on your Mac and the
  server stores only ciphertext. Transcripts and credentials never leave
  your Mac.
- **Usage statistics are off unless you turn them on.** They're aggregated
  on your Mac, carry no identifiers and are documented in
  [docs/usage-stats.md](./docs/usage-stats.md). Crash reports are kept on
  your Mac and never sent automatically.
- **Signed updates only**, verified before they're installed.
- **Fleet** uses mutual TLS with a key per Mac and a pairing code you
  confirm. Your Macs talk directly, with no relay, and credentials never
  travel over the link.
