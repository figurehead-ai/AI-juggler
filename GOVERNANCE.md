# Governance

AI Juggler is an open-source project published by
[Figurehead](https://github.com/figurehead-ai) under the MIT License.

## Roles

- **Maintainers** review and merge pull requests, triage issues, cut
  releases, and own the settled product decisions in the
  [decision log](./docs/review/decision-log.md). Maintainers are the
  members of the repository's maintainer team.
- **Contributors** are everyone who files an issue, reviews, writes
  docs, or sends a pull request.

## How decisions are made

- Day-to-day changes are decided in pull-request review: one maintainer
  approval and green CI.
- **Product decisions** (the `D-xx` entries) change only through a new,
  recorded decision approved by a maintainer — propose one in a
  discussion with the problem, the options, and the trade-offs.
- **Architecture decisions** are recorded as [ADRs](./docs/adr/).
- **Quality gates** are never weakened silently: lowering a threshold or
  skipping a gate is itself a recorded decision.

## Releases

Releases follow [Semantic Versioning](https://semver.org/) on one version
stream: every build gets the next version, and the Nightly, Beta and Stable
channels point at builds that have earned them. 0.1 is the first Stable
release. [docs/RELEASING.md](./docs/RELEASING.md) describes how a build moves
between channels. Notable changes are listed in the
[CHANGELOG](./CHANGELOG.md).
