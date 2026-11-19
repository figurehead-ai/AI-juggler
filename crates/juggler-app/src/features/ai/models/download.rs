//! AJ-E3813 (D-131, D-121 §2): the one model download at a time, from the
//! user's click to an installed model.
//!
//! [`Downloads::start`] is the only way a model byte is fetched, and the
//! Download click's command the only caller. It checks the store's rules
//! first ([`super::store::Store::check_start`]: not installed, nothing
//! else running, the free-disk margin, the approval for a further model),
//! then runs the transfer ([`super::transfer`]) on its own thread at the
//! lowest priority, and installs the verified file with its copy. Nothing
//! here is made, and no client exists, before that first click.
//!
//! What it reports ([`Event`]): progress while it runs or waits, and how
//! it ended. [`Downloads::cancel`] stops it and deletes its part file at
//! once.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use super::ModelEntry;
use super::store::{Refusal, Store};
use super::transfer::{self, Control, Failure, Fetch, Machine, Plan, Progress, Run, Tuning};

/// How a download ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Verified and installed.
    Installed,
    /// It failed, or was cancelled (the part file is gone either way).
    Failed(Failure),
}

/// What a download tells its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// Where the download of `id` stands.
    Progress {
        /// The model.
        id: String,
        /// Its progress.
        progress: Progress,
    },
    /// The download of `id` ended.
    Ended {
        /// The model.
        id: String,
        /// How.
        outcome: Outcome,
    },
}

/// Where a caller hears a download's events.
pub type Notify = Arc<dyn Fn(Event) + Send + Sync>;

/// The running download.
#[derive(Debug)]
struct Running {
    job: u64,
    id: String,
    control: Arc<Control>,
    progress: Progress,
    part: PathBuf,
}

/// The model downloads (one at a time).
pub struct Downloads {
    store: Arc<Store>,
    fetch: Arc<dyn Fetch>,
    machine: Arc<dyn Machine>,
    tuning: Tuning,
    running: Arc<Mutex<Option<Running>>>,
    last: Arc<Mutex<Option<(String, Outcome)>>>,
    jobs: AtomicU64,
}

impl std::fmt::Debug for Downloads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Downloads")
            .field("store", &self.store.dir())
            .finish_non_exhaustive()
    }
}

impl Downloads {
    /// Downloads into `store`, over `fetch`, reading `machine`.
    #[must_use]
    pub fn new(
        store: Arc<Store>,
        fetch: Arc<dyn Fetch>,
        machine: Arc<dyn Machine>,
        tuning: Tuning,
    ) -> Self {
        Self {
            store,
            fetch,
            machine,
            tuning,
            running: Arc::new(Mutex::new(None)),
            last: Arc::new(Mutex::new(None)),
            jobs: AtomicU64::new(0),
        }
    }

    /// The store it installs into.
    #[must_use]
    pub fn store(&self) -> &Arc<Store> {
        &self.store
    }

    /// Starts fetching `entry`'s file (the user's Download click), with
    /// `approval` for a further model; `notify` hears its progress and
    /// its end.
    ///
    /// # Errors
    ///
    /// The rule it breaks; nothing is fetched then.
    pub fn start(
        &self,
        entry: &ModelEntry,
        approval: Option<&str>,
        notify: Notify,
    ) -> Result<(), Refusal> {
        let plan = match Plan::of(entry) {
            Ok(plan) => plan,
            Err(_) if entry.download.is_none() => return Err(Refusal::NothingToFetch),
            Err(_) => return Err(Refusal::NotOffered),
        };
        let mut running = self.running.lock().map_err(|_| Refusal::Busy)?;
        let free = self.machine.disk(self.store.dir()).map(|disk| disk.free);
        // A cancelled download still winding down (a stalled read) holds
        // nothing up: its part file is its own and already gone.
        let busy = running
            .as_ref()
            .is_some_and(|running| !running.control.is_cancelled());
        self.store.check_start(entry, free, busy, approval)?;
        let control = Arc::new(Control::default());
        let size = plan.size_bytes;
        let job = self.jobs.fetch_add(1, Ordering::Relaxed) + 1;
        let part = plan.part_in(self.store.dir(), job);
        *running = Some(Running {
            job,
            id: entry.id.clone(),
            control: Arc::clone(&control),
            progress: Progress {
                done: 0,
                size,
                phase: transfer::Phase::Running,
            },
            part: part.clone(),
        });
        drop(running);
        let work = Job {
            number: job,
            part,
            entry: entry.clone(),
            plan,
            store: Arc::clone(&self.store),
            fetch: Arc::clone(&self.fetch),
            machine: Arc::clone(&self.machine),
            tuning: self.tuning.clone(),
            control,
            running: Arc::clone(&self.running),
            last: Arc::clone(&self.last),
            notify,
        };
        let spawned = std::thread::Builder::new()
            .name("ai-model-download".to_owned())
            .spawn(move || work.run());
        if spawned.is_err() {
            if let Ok(mut running) = self.running.lock() {
                *running = None;
            }
            return Err(Refusal::Busy);
        }
        Ok(())
    }

    /// Cancels the download of `id`: it stops at once and its part file
    /// goes now. Whether it was running.
    pub fn cancel(&self, id: &str) -> bool {
        let Ok(running) = self.running.lock() else {
            return false;
        };
        let Some(current) = running
            .as_ref()
            .filter(|running| running.id == id && !running.control.is_cancelled())
        else {
            return false;
        };
        current.control.cancel();
        let _ = std::fs::remove_file(&current.part);
        true
    }

    /// Wakes a waiting download to read this Mac again now.
    pub fn nudge(&self) {
        if let Ok(running) = self.running.lock()
            && let Some(running) = running.as_ref()
        {
            running.control.nudge();
        }
    }

    /// The running download: its model and progress (a cancelled one
    /// winding down isn't).
    #[must_use]
    pub fn running(&self) -> Option<(String, Progress)> {
        self.running
            .lock()
            .ok()?
            .as_ref()
            .filter(|running| !running.control.is_cancelled())
            .map(|running| (running.id.clone(), running.progress))
    }

    /// How the last download ended, if one has since the app started.
    #[must_use]
    pub fn last(&self) -> Option<(String, Outcome)> {
        self.last.lock().ok()?.clone()
    }

    /// How many times the running download has waited (tests: a paused
    /// download waits, it doesn't spin).
    #[must_use]
    pub fn waits(&self) -> Option<u64> {
        self.running
            .lock()
            .ok()?
            .as_ref()
            .map(|running| running.control.waits())
    }
}

/// One download's work, on its own thread.
struct Job {
    number: u64,
    part: PathBuf,
    entry: ModelEntry,
    plan: Plan,
    store: Arc<Store>,
    fetch: Arc<dyn Fetch>,
    machine: Arc<dyn Machine>,
    tuning: Tuning,
    control: Arc<Control>,
    running: Arc<Mutex<Option<Running>>>,
    last: Arc<Mutex<Option<(String, Outcome)>>>,
    notify: Notify,
}

impl Job {
    fn run(self) {
        // Never compete with the user's agents for the CPU.
        let _ = juggler_platform::thread::lowest_priority();
        let id = self.entry.id.clone();
        let run = Run {
            plan: &self.plan,
            dir: self.store.dir(),
            part: &self.part,
            fetch: self.fetch.as_ref(),
            machine: self.machine.as_ref(),
            control: &self.control,
            tuning: &self.tuning,
        };
        let running = Arc::clone(&self.running);
        let notify = Arc::clone(&self.notify);
        let report_id = id.clone();
        let job = self.number;
        let result = transfer::run(&run, &mut |progress| {
            if let Ok(mut running) = running.lock()
                && let Some(running) = running.as_mut().filter(|running| running.job == job)
            {
                running.progress = progress;
            }
            notify(Event::Progress {
                id: report_id.clone(),
                progress,
            });
        });
        let outcome = match result {
            Ok(file) => match self.store.install(&self.entry, &file) {
                Ok(_) => Outcome::Installed,
                Err(error) => Outcome::Failed(Failure::Disk(error.to_string())),
            },
            Err(failure) => Outcome::Failed(failure),
        };
        if let Outcome::Failed(failure) = &outcome {
            tracing::info!(%id, ?failure, "models: a download ended without a file");
        }
        // Only this download's own slot: a newer one may hold it already.
        let latest = self.running.lock().is_ok_and(|mut running| {
            let ours = running
                .as_ref()
                .is_none_or(|running| running.job == self.number);
            if ours {
                *running = None;
            }
            ours
        });
        if latest && let Ok(mut last) = self.last.lock() {
            *last = Some((id.clone(), outcome.clone()));
        }
        (self.notify)(Event::Ended { id, outcome });
    }
}

/// The app's downloads, made at the first Download click (the HTTPS
/// client with them): nothing exists before it.
static DOWNLOADS: OnceLock<Downloads> = OnceLock::new();

/// CPU use, percent of all cores, from which the Mac counts as busy.
const BUSY_CPU_PERCENT: f64 = 50.0;

/// Whether the user's agents are working on this Mac (the keep-awake
/// tick's count) or the Mac is busy (the machine sampler's CPU): both
/// cached readings, nothing measured here.
fn busy_now(state: &crate::state::AppState) -> bool {
    let working = crate::features::app::awake::current(&state.awake).working > 0;
    working
        || state.machine.lock().is_ok_and(|machine| {
            machine
                .sampler
                .snapshot()
                .latest
                .and_then(|sample| sample.cpu_pct)
                .is_some_and(|cpu| cpu >= BUSY_CPU_PERCENT)
        })
}

/// The app's downloads over the production client, made now if needed.
#[must_use]
pub fn shared(state: &Arc<crate::state::AppState>) -> &'static Downloads {
    DOWNLOADS.get_or_init(|| {
        let weak = Arc::downgrade(state);
        let busy: transfer::Busy =
            Box::new(move || weak.upgrade().is_some_and(|state| busy_now(&state)));
        Downloads::new(
            super::store::shared(&state.data_dir),
            Arc::new(transfer::HttpsFetch::new()),
            Arc::new(transfer::HostMachine::new(busy)),
            Tuning::default(),
        )
    })
}

/// The app's downloads if a click made them.
#[must_use]
pub fn shared_if_made() -> Option<&'static Downloads> {
    DOWNLOADS.get()
}
