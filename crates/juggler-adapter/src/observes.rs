//! ADR-0036 §2.2 (D-127, D-128): what AI Juggler can observe of a tool's
//! sessions, so a watch condition an App can't report is never offered for
//! it. Each adapter's [`crate::ToolSpec::observes`] declares its set, per
//! kind of App (a command-line session AI Juggler runs sees far more than a
//! desktop app's window); [`ToolObserves::backed_by`] is what its
//! [`Capabilities`] can back, and the adapters' conformance test holds
//! every declaration inside it. A tool may declare less (narrowed for a
//! version that misbehaves, D-128), never more.

use crate::Capabilities;
use crate::spec::ToolKind;

/// The kinds of observation a session's events can carry (ADR-0036
/// §2.3's bus). A watch condition names the ones it can be met from.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Observes(u32);

bitflags::bitflags! {
    impl Observes: u32 {
        /// A turn's exact end and its outcome (the tool's protocol or its
        /// state reports).
        const TURN_ENDS = 1 << 0;
        /// A question or an approval the session asks, and its answer.
        const ASKS = 1 << 1;
        /// The session's state: working, finished, failed, stuck.
        const STATES = 1 << 2;
        /// Sessions starting and ending.
        const LIFECYCLE = 1 << 3;
        /// A session reaching a usage limit.
        const LIMIT_HITS = 1 << 4;
        /// The account's room: its share left, and its resets.
        const RUNWAY = 1 << 5;
        /// Processes a session left running when it ended.
        const PROCESSES = 1 << 6;
        /// A crashed session or app started again by keep alive.
        const RESTARTS = 1 << 7;
        /// AI Juggler's own: a pool's messages.
        const POOL_MESSAGES = 1 << 8;
        /// AI Juggler's own: a task moving between states.
        const TASK_EVENTS = 1 << 9;
        /// AI Juggler's own, once the git journal exists (AJ-E3706): a
        /// commit landing in a project.
        const COMMITS = 1 << 10;
    }
}

impl Observes {
    /// What AI Juggler observes itself, whatever the tool: its own pools'
    /// messages and its tasks' moves. No adapter declares these. The git
    /// journal (AJ-E3706) adds [`Self::COMMITS`] here when it lands; until
    /// then no condition on commits is offered (D-127).
    pub const AI_JUGGLER_OWN: Self = Self::POOL_MESSAGES.union(Self::TASK_EVENTS);

    /// What AI Juggler sees of any command-line session it runs, whatever
    /// the tool: its state from the terminal (working, waiting on a
    /// prompt, stuck, finished or failed on exit), its start and end, the
    /// processes it leaves, and keep alive's restarts.
    pub const TERMINAL: Self = Self::STATES
        .union(Self::ASKS)
        .union(Self::LIFECYCLE)
        .union(Self::PROCESSES)
        .union(Self::RESTARTS);
}

/// A tool's declared observations, per kind of App.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToolObserves {
    /// Its command-line sessions.
    pub cli: Observes,
    /// Its desktop app's sessions.
    pub desktop: Observes,
}

impl ToolObserves {
    /// Nothing claimed.
    pub const NONE: Self = Self {
        cli: Observes::empty(),
        desktop: Observes::empty(),
    };

    /// The set for an App of `kind`.
    #[must_use]
    pub const fn for_kind(&self, kind: ToolKind) -> Observes {
        match kind {
            ToolKind::Cli => self.cli,
            ToolKind::Desktop => self.desktop,
        }
    }

    /// The most a tool with these capabilities and kinds can declare:
    ///
    /// - a command-line session: [`Observes::TERMINAL`]; exact turn ends
    ///   with the tool's protocol or state reports; limit hits with a
    ///   limit reader, a quota feed or the protocol (its rate-limit
    ///   errors); the account's room with a quota feed, reported windows
    ///   or plan caps;
    /// - a desktop app's session: keep alive's restarts, and the
    ///   account's room as above. Its states wait for the conversation
    ///   index's transitions (AJ-E3705).
    #[must_use]
    pub fn backed_by(caps: Capabilities, kinds: &[ToolKind]) -> Self {
        let exact = caps.intersects(Capabilities::PROTOCOL_SERVER | Capabilities::STATE_REPORTS);
        let limits = caps.intersects(
            Capabilities::LIMIT_HITS
                | Capabilities::USAGE_QUOTA_FEED
                | Capabilities::PROTOCOL_SERVER,
        );
        let runway = caps.intersects(
            Capabilities::USAGE_QUOTA_FEED
                | Capabilities::LIMIT_WINDOWS_REPORTED
                | Capabilities::LIMITS_FROM_PLAN_CAPS,
        );
        let mut room = Observes::empty();
        room.set(Observes::RUNWAY, runway);
        let mut cli = Observes::TERMINAL | room;
        cli.set(Observes::TURN_ENDS, exact);
        cli.set(Observes::LIMIT_HITS, limits);
        Self {
            cli: if kinds.contains(&ToolKind::Cli) {
                cli
            } else {
                Observes::empty()
            },
            desktop: if kinds.contains(&ToolKind::Desktop)
                || caps.contains(Capabilities::DESKTOP_APP)
            {
                Observes::RESTARTS | room
            } else {
                Observes::empty()
            },
        }
    }

    /// Every kind's set lies inside `other`'s.
    #[must_use]
    pub fn within(&self, other: &Self) -> bool {
        other.cli.contains(self.cli) && other.desktop.contains(self.desktop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_terminal_tool_with_no_protocol_has_no_exact_turn_ends() {
        let plain = ToolObserves::backed_by(Capabilities::RESUME, &[ToolKind::Cli]);
        assert_eq!(plain.cli, Observes::TERMINAL);
        assert!(plain.desktop.is_empty());
        let driven = ToolObserves::backed_by(Capabilities::PROTOCOL_SERVER, &[ToolKind::Cli]);
        assert!(
            driven
                .cli
                .contains(Observes::TURN_ENDS | Observes::LIMIT_HITS)
        );
        assert!(!driven.cli.contains(Observes::RUNWAY));
    }

    #[test]
    fn a_desktop_app_sees_restarts_and_its_accounts_room_only() {
        let app = ToolObserves::backed_by(
            Capabilities::DESKTOP_APP | Capabilities::USAGE_QUOTA_FEED,
            &[ToolKind::Desktop],
        );
        assert_eq!(app.desktop, Observes::RESTARTS | Observes::RUNWAY);
        assert!(app.cli.is_empty());
    }

    #[test]
    fn no_tool_declares_what_ai_juggler_observes_itself() {
        let all = ToolObserves::backed_by(Capabilities::all(), &[ToolKind::Cli, ToolKind::Desktop]);
        assert!(
            !all.cli
                .intersects(Observes::AI_JUGGLER_OWN | Observes::COMMITS)
        );
        assert!(
            !all.desktop
                .intersects(Observes::AI_JUGGLER_OWN | Observes::COMMITS)
        );
    }
}
