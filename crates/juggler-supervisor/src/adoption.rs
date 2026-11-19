//! Picking sessions up again after a restart (AJ-E510): the startup
//! recovery pass and the handles it rebuilds for verified live roots. A
//! child of `supervisor.rs`.

#![allow(
    clippy::wildcard_imports,
    reason = "a child of supervisor.rs: its handles, imports and exit fan-out"
)]

use super::*;

/// The row's identity facts as the E503 record (the verify input).
fn row_identity(row: &juggler_core::storage::LedgerRow) -> crate::tree::ProcessIdentity {
    crate::tree::ProcessIdentity {
        pid: row.pid.unwrap_or(0),
        pgid: row.pgid,
        start_time: row.started_at.unwrap_or_default(),
        exe: std::path::PathBuf::from(&row.exe_path),
        boot_id: row.boot_id.clone(),
    }
}

/// The root row's identity for the start-time-only liveness probe
/// (E504's exec-aliasing rule: a root's exe moves legitimately).
fn root_identity(root: &juggler_core::storage::LedgerRow) -> crate::tree::ProcessIdentity {
    row_identity(root)
}

impl Supervisor {
    /// Sessions from previous app runs whose processes are still alive
    /// (journal adoption; spec §5.3 crash re-adoption).
    /// One startup recovery pass (AJ-E510, pass2 §11.2): over the
    /// ledger's live rows — (1) identity-verify each (failures become
    /// `Unverifiable`: listed, never killed); (2) rebuild ADOPTED
    /// handles for verified live roots (output unavailable, stop =
    /// the verified group ladder through [`Supervisor::stop`] —
    /// closing F-3); (3) helpers of roots that died while Juggler was
    /// down enter `Orphaned` (their `reaped_by_us=false` root is the
    /// `left behind while Juggler was closed` marker — attribution
    /// preserved); (4) dead roots patch `exited`. Returns the report
    /// for the one-time summary (Req.3) and the tier suspension.
    #[allow(clippy::similar_names, reason = "pid/pgid are the kernel's own names")]
    ///
    /// `ends` names the sessions whose live roots can't be picked up
    /// (rc27 live pass 2: a driven Codex app-server outlived Quit anyway,
    /// was counted "picked up again" and read Working with no connection,
    /// no output and, once it ended, a Stop that refused): their verified
    /// roots are stopped through the verified ladder, never adopted.
    pub fn recover(
        &self,
        rows_by_session: &[(SessionId, Vec<juggler_core::storage::LedgerRow>)],
        ends: &dyn Fn(&SessionId) -> bool,
    ) -> RecoveryReport {
        use crate::tree::Verification;
        let mut report = RecoveryReport::default();
        for (session, rows) in rows_by_session {
            let Some(root) = rows.iter().find(|row| row.role == "root") else {
                continue;
            };
            let Some(root_pid) = root.pid else { continue };
            let root_alive = crate::tree::verify_start_time(root_pid, &root_identity(root))
                == Verification::Verified;

            for row in rows {
                let Some(pid) = row.pid else { continue };
                if !matches!(row.state.as_str(), "running" | "orphaned") {
                    continue;
                }
                let identity = row_identity(row);
                // E504/E507's aliasing rule: a ROOT's exe legitimately
                // moves across `exec` (`/bin/sh` records, the kernel
                // later reads `/bin/bash`) — roots verify by
                // boot + start time; helpers carry the kernel-true
                // triple from the reconciler's table.
                // A helper never observed past the exec-settle window
                // (the app went down inside it) may still carry its
                // pre-exec image: its exe leg waits, as at kill time
                // (AJ-E508); boot + start time never wait.
                let verified = if row.role == "root" {
                    crate::tree::verify_start_time(pid, &identity) == Verification::Verified
                } else {
                    let settling = row.last_seen.saturating_sub(row.first_seen)
                        < juggler_core::storage::EXEC_SETTLE_MS;
                    crate::tree::verify_settling(pid, &identity, settling) == Verification::Verified
                };
                if row.role == "root" {
                    if root_alive && verified && ends(session) {
                        self.end_unadoptable(root, pid);
                        report.ended.push(AdoptionRecord {
                            session: session.clone(),
                            pid,
                            program: root.exe_path.clone(),
                        });
                    } else if root_alive && verified {
                        // (2) re-adopt with a working stop path.
                        self.register_adopted(session, root);
                        report.adopted.push(AdoptionRecord {
                            session: session.clone(),
                            pid,
                            program: root.exe_path.clone(),
                        });
                    } else if crate::orphans::process_alive(pid).is_some() {
                        // (1) alive but unverifiable: listed, never killed.
                        self.ledger_write(|ledger| ledger.mark_unverifiable(pid, &row.boot_id));
                        report.unverifiable.push(pid);
                    } else {
                        // (4) died while we were down.
                        self.ledger_write(|ledger| {
                            ledger.mark_exited(pid, &row.boot_id, "unknown", false)
                        });
                    }
                } else if !verified {
                    if crate::orphans::process_alive(pid).is_some() {
                        self.ledger_write(|ledger| ledger.mark_unverifiable(pid, &row.boot_id));
                        report.unverifiable.push(pid);
                    }
                } else if !root_alive {
                    // (3) a live helper of a root that died while
                    // Juggler was closed — the durable down-period
                    // marker + original attribution.
                    self.ledger_write(|ledger| ledger.mark_orphaned_down(pid, &row.boot_id));
                    report.left_behind.push(pid);
                }
            }
        }
        report
    }

    /// Stops a verified live root nobody can pick up, and its own group
    /// (the group guard: only a group it leads), then records its exit.
    #[allow(clippy::similar_names, reason = "pid/pgid are the kernel's own names")]
    fn end_unadoptable(&self, root: &juggler_core::storage::LedgerRow, pid: u32) {
        let pgid = root.pgid;
        crate::tree::terminate_group(
            Duration::from_secs(1),
            move || {
                let mut members = vec![pid];
                members.extend(crate::stop_guard::group_helpers(pid, pgid));
                members
            },
            || false,
        );
        self.ledger_write(|ledger| ledger.mark_exited(pid, &root.boot_id, "stopped", false));
    }

    /// Settles an adopted handle whose root ended since (no wait thread
    /// watches a process that isn't our fork): it is finished and no
    /// longer adopted. Returns whether it ended.
    pub(super) fn settle_adopted(&self, session: &SessionId, handle: &ChildHandle) -> bool {
        if handle.finished.load(Ordering::Acquire) {
            return true;
        }
        let adopted = self
            .adopted
            .lock()
            .is_ok_and(|adopted| adopted.contains_key(session));
        if !adopted {
            return false;
        }
        let identity = handle.identity.lock().ok().and_then(|id| id.clone());
        if !crate::stop_guard::adopted_root_gone(handle.pid, identity.as_ref()) {
            return false;
        }
        handle.finished.store(true, Ordering::Release);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
            .unwrap_or_default();
        handle.exit_at_ms.store(now, Ordering::Release);
        if let Ok(mut adopted) = self.adopted.lock() {
            adopted.remove(session);
        }
        true
    }

    /// Registers an ADOPTED handle (AJ-E510, F-3): no ring readers, no
    /// PTY (output from before the crash is honestly unavailable),
    /// `finished` live — `stop()` now works through the same verified
    /// group ladder as any supervised session.
    fn register_adopted(&self, session: &SessionId, root: &juggler_core::storage::LedgerRow) {
        let pid = root.pid.unwrap_or(0);
        let kill_pid = pid;
        let force_kill: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
            // No std Child to `.kill()` — the adopted root is not our
            // fork. The honest fallback: a direct KILL through the ONE
            // platform signal seam (unix signals / no-op off-unix).
            for member in crate::tree::pgid_members(kill_pid) {
                let _ = crate::tree::signal_pid(member, crate::tree::Signal::Kill);
            }
        });
        let handle = Arc::new(ChildHandle {
            pid,
            pgid: root.pgid,
            identity: Mutex::new(Some(row_identity(root))),
            exit_at_ms: Arc::new(AtomicI64::new(0)),
            ring: Arc::new(Mutex::new(OutputRing::new(DEFAULT_RING_CAPACITY))),
            finished: Arc::new(AtomicBool::new(false)),
            last_output_ms: Arc::new(std::sync::atomic::AtomicI64::new(0)),
            force_kill,
            pty: None,
        });
        if let Ok(mut children) = self.children.lock() {
            children.insert(session.clone(), Arc::clone(&handle));
        }
        if let Ok(mut adopted) = self.adopted.lock() {
            adopted.insert(session.clone(), handle);
        }
        // The stop path reads the spawn context for nothing, but the
        // reconciler's helper upserts DO — re-seed it from the root
        // row's attribution so helpers keep their provider/account.
        if let Ok(mut contexts) = self.spawn_contexts.lock() {
            contexts
                .entry(session.clone())
                .or_insert_with(|| crate::tree::SpawnContext {
                    provider: root.provider.clone(),
                    account_id: root.account_id.clone(),
                    project_id: root.project_id.clone(),
                    machine: juggler_core::model::MachineId::new(root.machine_id.clone()),
                    role: "root",
                });
        }
    }

    /// The re-adopted session set (AJ-E510): sessions whose verified
    /// live roots got handles back at startup — the `Running —
    /// re-adopted` marker for the Cleanup Centre, port map, and the
    /// read-model reconciliation (they stay `Running`, never
    /// `Stopped`).
    pub fn alive_adopted_sessions(&self) -> Vec<SessionId> {
        // AJ-E510: the marker set built by crash recovery — the
        // re-adopted sessions ARE supervised again (stoppable via the
        // E507 path); the pre-E510 heuristic (journal-live minus
        // supervised) could not express that and double-counted live
        // sessions (the AJ-E505 port-classification bug).
        // A root that ended since is no longer one (rc27 live pass 2:
        // a gone one kept New session's worktree question busy).
        let adopted: Vec<(SessionId, Arc<ChildHandle>)> = self
            .adopted
            .lock()
            .map(|adopted| {
                adopted
                    .iter()
                    .map(|(id, handle)| (id.clone(), Arc::clone(handle)))
                    .collect()
            })
            .unwrap_or_default();
        adopted
            .into_iter()
            .filter(|(id, handle)| !self.settle_adopted(id, handle))
            .map(|(id, _)| id)
            .collect()
    }
}
