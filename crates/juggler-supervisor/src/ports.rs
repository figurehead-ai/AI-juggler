//! Port occupancy map (AJ-E505, pass2 §7): joins ledger ownership to
//! the OS listener table so a left-behind process reads as
//! "PID 8123 holds :5173 for Instance X", and launch can warn BEFORE
//! `EADDRINUSE`.
//!
//! Mechanism pin: `netstat2` on macOS/Linux (libproc / netlink — the
//! low-level APIs, NOT a shell-out); Windows is the
//! `GetExtendedTcpTable` seam left to the M7-5 bring-up. `lsof` is
//! BANNED (slow, permission-flaky, unparsable under load — pass2 §7);
//! `netstat`-CLI parsing likewise. Well-known dev ports are DISPLAY
//! HINTS only — never ownership proof; identity stays AJ-E503's.

use std::collections::HashSet;

use juggler_core::storage::LedgerRow;

/// Well-known dev ports (hints for labels, never identity evidence).
pub const WELL_KNOWN_DEV_PORTS: &[u16] = &[3000, 4173, 5173, 8000, 8080, 9229];

/// The honest note when the platform has no listener probe.
pub const PORTS_UNKNOWN_ON_THIS_PLATFORM: &str = "ports unknown on this platform";

/// One listening TCP socket: the owning pid and the local port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Listener {
    /// The process holding the socket.
    pub pid: u32,
    /// The local TCP port.
    pub port: u16,
}

/// Probe failures: `Unsupported` is the honest platform seam;
/// `Probe` is a runtime failure (degrades one tick, retried at the
/// tick cadence — never a retry storm).
#[derive(Debug)]
pub enum PortProbeError {
    /// This platform has no probe yet (Windows: M7-5 seam).
    Unsupported,
    /// The OS API failed; text for the log.
    Probe(String),
}

impl std::fmt::Display for PortProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported => write!(f, "{PORTS_UNKNOWN_ON_THIS_PLATFORM}"),
            Self::Probe(message) => write!(f, "listener probe failed: {message}"),
        }
    }
}

/// The listener-table seam (tests inject fixtures; production reads
/// the OS via [`NetstatProbe`]).
pub trait PortProbe: Send + Sync {
    /// Every LISTENING TCP socket, pid + local port.
    ///
    /// # Errors
    ///
    /// [`PortProbeError`] — never a panic, never an empty map
    /// presented as truth.
    fn listeners(&self) -> Result<Vec<Listener>, PortProbeError>;
}

/// The real probe on macOS/Linux: one `netstat2` enumeration of the
/// listening TCP table (both families), pids attached.
#[cfg(unix)]
#[derive(Debug, Default, Clone, Copy)]
pub struct NetstatProbe;

#[cfg(unix)]
impl PortProbe for NetstatProbe {
    fn listeners(&self) -> Result<Vec<Listener>, PortProbeError> {
        let sockets = netstat2::iterate_sockets_info(
            netstat2::AddressFamilyFlags::IPV4 | netstat2::AddressFamilyFlags::IPV6,
            netstat2::ProtocolFlags::TCP,
        )
        .map_err(|e| PortProbeError::Probe(e.to_string()))?;
        let mut out = Vec::new();
        for socket in sockets {
            let socket = socket.map_err(|e| PortProbeError::Probe(e.to_string()))?;
            let netstat2::ProtocolSocketInfo::Tcp(tcp) = socket.protocol_socket_info else {
                continue;
            };
            if tcp.state != netstat2::TcpState::Listen {
                continue;
            }
            for pid in socket.associated_pids {
                out.push(Listener {
                    pid,
                    port: tcp.local_port,
                });
            }
        }
        Ok(out)
    }
}

/// The non-unix seam: honest unsupported, never a faked table and
/// never `netstat.exe` parsing (the Windows bring-up owns the real
/// `GetExtendedTcpTable` probe, M7-5).
#[cfg(not(unix))]
#[derive(Debug, Default, Clone, Copy)]
pub struct UnsupportedProbe;

#[cfg(not(unix))]
impl PortProbe for UnsupportedProbe {
    fn listeners(&self) -> Result<Vec<Listener>, PortProbeError> {
        Err(PortProbeError::Unsupported)
    }
}

/// The display hint for a well-known dev port (`5173` → "vite dev
/// server") — a LABEL, never identity evidence.
#[must_use]
pub fn well_known_hint(port: u16) -> Option<&'static str> {
    match port {
        3000 | 8000 | 8080 => Some("common dev server port"),
        4173 => Some("vite preview port"),
        5173 => Some("vite dev server port"),
        9229 => Some("node inspector port"),
        _ => None,
    }
}

/// Exactly one classification per listener (Req.3). `NotManaged`
/// rows are display-only — the FMEA/F3 collateral guard: no kill
/// affordance, ever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenerClass {
    /// A left-behind helper of a dead Instance (T2) — killable via
    /// the AJ-E507 verified sheet only.
    ManagedOrphan,
    /// A listener inside a live Juggler-owned tree.
    ManagedRunning,
    /// A listener of a re-adopted (restart-surviving) Instance (T3).
    Readopted,
    /// Not in the ledger (the user's own process) or matching only
    /// `unverifiable` rows — display-only, zero kill affordances.
    NotManaged,
}

impl ListenerClass {
    /// The stable API string (the UI's classification key).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ManagedOrphan => "managed-orphan",
            Self::ManagedRunning => "managed-running",
            Self::Readopted => "re-adopted",
            Self::NotManaged => "not-managed",
        }
    }
}

/// One inventory row: port → PID → Instance → attribution + age.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortRow {
    /// The listening port (the headline; the PID is evidence).
    pub port: u16,
    /// The owning process id.
    pub pid: u32,
    /// The classification (exactly one per row).
    pub class: ListenerClass,
    /// Owning Instance (ledger session id), when managed.
    pub session: Option<String>,
    /// Provider id, when managed.
    pub provider: Option<String>,
    /// Account id, when managed.
    pub account: Option<String>,
    /// Project id, when managed.
    pub project: Option<String>,
    /// Listener age from the KERNEL process start time (a listener
    /// begins with its process), epoch ms; `None` when unmanaged and
    /// the pid left the table.
    pub listener_age_ms: Option<i64>,
    /// Process name (display).
    pub name: Option<String>,
    /// The well-known-port display hint, when one exists.
    pub hint: Option<&'static str>,
}

/// Joins listeners to ledger rows (pure — the unit-test seam with
/// recorded fixtures, including an unmanaged pid and a recycled one).
/// A pid matches at most its NEWEST `running|orphaned` row;
/// `unverifiable` rows never confer ownership (fail closed).
#[must_use]
pub fn classify_listeners<S: std::hash::BuildHasher>(
    listeners: &[Listener],
    rows: &[LedgerRow],
    adopted: &HashSet<String, S>,
    now_ms: i64,
) -> Vec<PortRow> {
    // pid → newest owner row (running|orphaned only).
    let mut owner: std::collections::HashMap<u32, &LedgerRow> = std::collections::HashMap::new();
    for row in rows {
        let Some(pid) = row.pid else { continue };
        if row.state != juggler_core::storage::STATE_RUNNING
            && row.state != juggler_core::storage::STATE_ORPHANED
        {
            continue;
        }
        match owner.get(&pid) {
            Some(current) if current.first_seen >= row.first_seen => {}
            _ => {
                owner.insert(pid, row);
            }
        }
    }
    // Root liveness per session (a helper of a dead root is left
    // behind even while its orphan flag waits out the grace).
    let mut root_alive: std::collections::HashMap<&str, bool> = std::collections::HashMap::new();
    for row in rows {
        if row.role == juggler_core::storage::ROLE_ROOT {
            let alive = row.state == juggler_core::storage::STATE_RUNNING;
            root_alive.insert(row.session_id.as_str(), alive);
        }
    }

    let mut out = Vec::new();
    for listener in listeners {
        let Some(row) = owner.get(&listener.pid) else {
            // Unmanaged (or only-unverifiable — same treatment, the
            // F3 guard): display-only, no attribution claimed.
            out.push(PortRow {
                port: listener.port,
                pid: listener.pid,
                class: ListenerClass::NotManaged,
                session: None,
                provider: None,
                account: None,
                project: None,
                listener_age_ms: None,
                name: None,
                hint: well_known_hint(listener.port),
            });
            continue;
        };
        let session = row.session_id.clone();
        let root_dead = !root_alive.get(session.as_str()).copied().unwrap_or(false);
        let class = if adopted.contains(&session) {
            ListenerClass::Readopted
        } else if row.state == juggler_core::storage::STATE_ORPHANED
            || (row.role == juggler_core::storage::ROLE_HELPER && root_dead)
        {
            ListenerClass::ManagedOrphan
        } else {
            ListenerClass::ManagedRunning
        };
        out.push(PortRow {
            port: listener.port,
            pid: listener.pid,
            class,
            session: Some(session),
            provider: Some(row.provider.clone()),
            account: row.account_id.clone(),
            project: row.project_id.clone(),
            listener_age_ms: row
                .started_at
                .map(|start| (now_ms - start.saturating_mul(1000)).max(0)),
            name: Some(row.exe_path.clone()),
            hint: well_known_hint(listener.port),
        });
    }
    out.sort_by_key(|row| (row.port, row.pid));
    out
}

#[cfg(test)]
mod tests {
    //! Recorded-fixture classification tests (the quality gate),
    //! including the unmanaged pid and the recycled pid.

    use super::{
        Listener, ListenerClass, PORTS_UNKNOWN_ON_THIS_PLATFORM, classify_listeners,
        well_known_hint,
    };
    use juggler_core::storage::LedgerRow;
    use std::collections::HashSet;

    fn row(pid: u32, session: &str, state: &str, role: &str) -> LedgerRow {
        LedgerRow {
            pid: Some(pid),
            pgid: Some(pid),
            started_at: Some(1_792_000_000),
            exe_path: format!("/usr/bin/p{pid}"),
            cmdline_hash: format!("h{pid}"),
            cmdline_trunc: format!("p{pid}"),
            boot_id: "boot".into(),
            session_id: session.into(),
            instance_id: None,
            provider: "claude-code".into(),
            account_id: Some("acc-1".into()),
            project_id: Some("proj-1".into()),
            machine_id: "m".into(),
            root_pid: None,
            parent_pid: None,
            depth: if role == "root" { 0 } else { 1 },
            role: role.into(),
            state: state.into(),
            listens_on: None,
            cwd: "/tmp".into(),
            peak_rss: None,
            peak_cpu: None,
            exit_kind: None,
            reaped_by_us: None,
            escaped_group: false,
            down_period: false,
            first_seen: 1_000,
            last_seen: 1_000,
        }
    }

    #[test]
    fn hints_label_but_never_identify() {
        assert_eq!(well_known_hint(5173), Some("vite dev server port"));
        assert_eq!(well_known_hint(9229), Some("node inspector port"));
        assert_eq!(well_known_hint(65534), None);
    }

    #[test]
    fn unsupported_probe_is_honest() {
        #[cfg(not(unix))]
        {
            use super::PortProbe as _;
            use super::PortProbeError;
            let probe = super::UnsupportedProbe;
            match probe.listeners() {
                Err(PortProbeError::Unsupported) => {}
                other => panic!("expected unsupported, got {other:?}"),
            }
        }
        #[cfg(unix)]
        {
            assert_eq!(
                PORTS_UNKNOWN_ON_THIS_PLATFORM,
                "ports unknown on this platform"
            );
        }
    }

    #[test]
    fn four_classifications_exactly() {
        let rows = vec![
            row(11, "s-live", "running", "root"),
            row(12, "s-orphan", "orphaned", "helper"),
            row(13, "s-adopt", "running", "root"),
            row(14, "s-dead-root", "exited", "root"),
            row(15, "s-dead-root", "running", "helper"),
        ];
        let mut adopted = HashSet::new();
        adopted.insert("s-adopt".to_string());
        let listeners = vec![
            Listener {
                pid: 11,
                port: 3001,
            },
            Listener {
                pid: 12,
                port: 3002,
            },
            Listener {
                pid: 13,
                port: 3003,
            },
            Listener {
                pid: 15,
                port: 3004,
            },
            Listener {
                pid: 99,
                port: 3005,
            }, // unmanaged
        ];
        let out = classify_listeners(&listeners, &rows, &adopted, 1_792_000_200_000);
        let by_port = |p: u16| out.iter().find(|r| r.port == p).unwrap();
        assert_eq!(by_port(3001).class, ListenerClass::ManagedRunning);
        assert_eq!(by_port(3002).class, ListenerClass::ManagedOrphan);
        assert_eq!(by_port(3003).class, ListenerClass::Readopted);
        // A helper of a DEAD root is left behind even pre-grace.
        assert_eq!(by_port(3004).class, ListenerClass::ManagedOrphan);
        let unmanaged = by_port(3005);
        assert_eq!(unmanaged.class, ListenerClass::NotManaged);
        assert!(unmanaged.session.is_none());
    }

    #[test]
    fn unmanaged_and_recycled_pids_never_gain_attribution() {
        let rows = vec![
            // An exited old row and a stranger's unverifiable row for
            // the SAME pid (the recycled shape): neither confers
            // ownership.
            row(21, "s-old", "exited", "root"),
            row(22, "s-unver", "unverifiable", "helper"),
        ];
        let listeners = vec![
            Listener {
                pid: 21,
                port: 3010,
            },
            Listener {
                pid: 22,
                port: 3011,
            },
        ];
        let out = classify_listeners(&listeners, &rows, &HashSet::new(), 0);
        assert!(out.iter().all(|r| r.class == ListenerClass::NotManaged));
        assert!(out.iter().all(|r| r.session.is_none()));
        // The well-known hint still labels without identifying.
        assert_eq!(out[0].hint, None);
    }

    #[test]
    fn newest_row_wins_a_pid() {
        let old = row(31, "s-a", "running", "root");
        let mut new = row(31, "s-b", "running", "root");
        new.first_seen = 2_000;
        let listeners = [Listener {
            pid: 31,
            port: 3020,
        }];
        let out = classify_listeners(&listeners, &[old, new], &HashSet::new(), 0);
        assert_eq!(out[0].session.as_deref(), Some("s-b"));
    }
}
