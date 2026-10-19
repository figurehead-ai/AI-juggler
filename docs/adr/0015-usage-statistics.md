# ADR-0015: Opt-in usage statistics

**Status:** Accepted
**Date:** 2026-10-18
**Deciders:** maintainer (D-19); implementation agent (E11)

## Context

Building the right things for people who juggle several AI coding tools
needs some idea of which features they use. AI Juggler's promise is that
nothing about anyone's work leaves their Mac, and that with no choice
made, it makes no network calls of its own. The usage-statistics
specification (Figurehead, approved 2026-10-16) defines a way to learn
feature usage that keeps both promises. This ADR records how AI Juggler
implements its half.

## Decision

1. **A separate crate, `juggler-stats`,** behind a default-on `stats`
   cargo feature. Distro and from-source builds can leave it out
   entirely, and the Privacy screen then says so.
2. **Off until chosen.** The mode lives in the app settings
   (`usage_stats`: mode, decided-at, consent version). In `off`,
   `record()` returns at once, no `stats/` folder exists, and the crate
   makes no request. The effective mode applies the overrides first:
   `DO_NOT_TRACK`, `AI_JUGGLER_STATS`, CI or a debug build (count, never
   upload), then the user's choice. Each override carries the reason
   the UI shows.
3. **Typed, closed events.** `StatEvent` and `StatGauge` are generated
   from `stats/schema.v1.json` by a committed script, with a parity test.
   Every dimension is an enum or a bounded integer; there is no `String`
   field anywhere, so no free text, path or label can reach a report.
4. **Counting is fire-and-forget.** A bounded in-memory channel feeds a
   background writer to `stats/counters.sqlite` (0700). A full channel
   drops events and counts the drop.
5. **One report per closed ISO week (UTC).** It is assembled from the
   counters, rounded, clamped to the schema's caps, validated, and kept
   as the exact bytes in `stats/pending/`, readable in Settings ▸
   Privacy before it is sent. Sending happens on an hourly tick after a
   random 0–144 h delay, obeying the static config (enabled, version
   range, sampling, proof-of-work difficulty) and the server's
   `Retry-After`. The upload URL is compiled in, never taken from the
   config.
6. **Sampling without identity:** `sha256(salt ‖ week)` against the
   sample rate, with a local salt that is never sent.
7. **Personal sync** replaces the anonymous upload for a week the user
   ticked "Show my stats in my Figurehead dashboard" throughout, using
   the existing device token (ADR-0014). Never both.
8. **Turning it off deletes everything local** (counters, reports, salt,
   log) at once.

## Alternatives considered

- **An install id, so reports can be linked across weeks.** It would
  give better retention numbers but breaks the "no identifier" promise.
  Rejected.
- **Sending raw events.** That means more data than the question needs,
  and a stream that can reconstruct someone's day. Rejected: counters
  only.
- **A third-party analytics SDK.** It would bring opaque network
  behaviour and identifiers we don't control. Rejected.
- **Free-form dimensions with server-side filtering.** "Filter later"
  means the data has already left the Mac. Rejected: the closed schema
  is enforced on both sides.

## Consequences

- Adding a metric needs the server's schema changed first, then a
  regenerated enum, the CHANGELOG and the generated
  `docs/usage-stats.md`, in one release. `scripts/stats_schema_diff.sh`
  checks the two copies before each release.
- Most of what the maintainer would like to know (anything about code
  or projects) stays unknowable by design. That is the point.
