//! AJ-E3813: downloads end to end against the stub server: nothing
//! reaches the network before the click, the second model needs its
//! approval each time, cancel and failure leave nothing behind.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use super::commands::models_info;
use super::download::{Downloads, Event, Notify, Outcome};
use super::dto::{DownloadState, StartRefusal};
use super::store::{Refusal, Store};
use super::transfer::{Failure, Fetch, Machine};
use super::transfer_tests::stub::{
    FakeMac, File, Loopback, Reply, Server, bytes, entry, files, pinned, quick,
};
use super::{Listing, ModelEntry};

const GB: u64 = 1_000_000_000;

struct Bench {
    server: Server,
    fetch: Arc<Loopback>,
    mac: Arc<FakeMac>,
    _dir: tempfile::TempDir,
    downloads: Downloads,
    events: Arc<Mutex<Vec<Event>>>,
}

impl Bench {
    fn new() -> Self {
        let server = Server::start();
        let fetch = Arc::new(Loopback::to(&server));
        let mac = Arc::new(FakeMac::roomy());
        let dir = tempfile::tempdir().expect("dir");
        let store = Arc::new(Store::in_data_dir(dir.path()));
        let downloads = Downloads::new(
            store,
            Arc::clone(&fetch) as Arc<dyn Fetch>,
            Arc::clone(&mac) as Arc<dyn Machine>,
            quick(),
        );
        Self {
            server,
            fetch,
            mac,
            _dir: dir,
            downloads,
            events: Arc::default(),
        }
    }

    fn notify(&self) -> Notify {
        let events = Arc::clone(&self.events);
        Arc::new(move |event| events.lock().unwrap().push(event))
    }

    fn serve(&self, model: &ModelEntry, file: &Arc<Vec<u8>>) {
        self.server
            .route(&pinned(&model.id), Reply::File(File::plain(file)));
    }

    fn start(&self, model: &ModelEntry, approval: Option<&str>) -> Result<(), Refusal> {
        self.downloads.start(model, approval, self.notify())
    }

    /// Waits for the download of `id` to end; how it did.
    fn ended(&self, id: &str) -> Outcome {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let found = self
                .events
                .lock()
                .unwrap()
                .iter()
                .find_map(|event| match event {
                    Event::Ended { id: ended, outcome } if ended == id => Some(outcome.clone()),
                    _ => None,
                });
            if let Some(outcome) = found {
                return outcome;
            }
            assert!(
                Instant::now() < deadline,
                "the download of {id} never ended"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn clear_events(&self) {
        self.events.lock().unwrap().clear();
    }
}

#[test]
fn no_request_reaches_the_network_before_the_click() {
    let bench = Bench::new();
    let file = bytes(20_000, 1);
    let model = entry("model-a", &file);
    bench.serve(&model, &file);
    let store = Arc::clone(bench.downloads.store());
    // Everything the screen and the rules do short of a Download click.
    let _ = models_info(&store, Some(&bench.downloads), &Listing::Unread);
    let _ = models_info(
        &store,
        Some(&bench.downloads),
        &Listing::Read(vec![model.clone()]),
    );
    let _ = store.installed();
    let _ = store.total_bytes();
    let _ = store.check_start(&model, Some(100 * GB), false, None);
    let _ = store.approve("model-a");
    store.decline("model-a");
    let _ = store.declined();
    assert!(!bench.downloads.cancel("model-a"));
    assert!(bench.downloads.running().is_none());
    let _ = store.refresh(&Listing::Read(vec![model.clone()]));
    // A refused click fetches nothing either.
    bench.mac.free.store(GB, Ordering::Relaxed);
    assert!(matches!(
        bench.start(&model, None),
        Err(Refusal::NotEnoughDisk { .. })
    ));
    assert_eq!(bench.fetch.calls(), 0);
    assert!(bench.server.hits().is_empty());
    // The app's own downloads (and their HTTPS client) don't exist until
    // a click makes them.
    assert!(super::download::shared_if_made().is_none());
    // The click does reach it.
    bench.mac.free.store(100 * GB, Ordering::Relaxed);
    bench.start(&model, None).expect("started");
    assert_eq!(bench.ended("model-a"), Outcome::Installed);
    assert!(bench.fetch.calls() > 0);
}

#[test]
fn the_disk_margin_decides_a_start_at_the_boundary() {
    let bench = Bench::new();
    let file = bytes(10_000, 2);
    let model = entry("model-a", &file);
    bench.serve(&model, &file);
    bench
        .mac
        .free
        .store(10_000 + 10 * GB - 1, Ordering::Relaxed);
    assert_eq!(
        bench.start(&model, None),
        Err(Refusal::NotEnoughDisk {
            needed_bytes: 10_000 + 10 * GB,
            free_bytes: 10_000 + 10 * GB - 1,
        })
    );
    assert!(bench.server.hits().is_empty());
    bench.mac.free.store(10_000 + 10 * GB, Ordering::Relaxed);
    bench.start(&model, None).expect("started");
    assert_eq!(bench.ended("model-a"), Outcome::Installed);
}

#[test]
fn the_second_model_needs_an_approval_every_time() {
    let bench = Bench::new();
    let (one, two) = (bytes(8_000, 3), bytes(9_000, 4));
    let (a, b) = (entry("model-a", &one), entry("model-b", &two));
    bench.serve(&a, &one);
    bench.serve(&b, &two);
    bench.start(&a, None).expect("the first needs no approval");
    assert_eq!(bench.ended("model-a"), Outcome::Installed);
    let asks = bench.server.hits().len();
    assert_eq!(
        bench.start(&b, None),
        Err(Refusal::NeedsApproval {
            size_bytes: 9_000,
            total_after_bytes: 17_000,
        })
    );
    assert_eq!(bench.server.hits().len(), asks, "fetched without approval");
    let token = bench.downloads.store().approve("model-b");
    bench.start(&b, Some(&token)).expect("approved");
    assert_eq!(bench.ended("model-b"), Outcome::Installed);
    assert_eq!(bench.downloads.store().total_bytes(), 17_000);
    // Removed and asked again: the earlier approval doesn't stand.
    bench.downloads.store().remove("model-b").expect("removed");
    bench.clear_events();
    assert!(matches!(
        bench.start(&b, Some(&token)),
        Err(Refusal::NeedsApproval { .. })
    ));
    let info = models_info(
        bench.downloads.store(),
        Some(&bench.downloads),
        &Listing::Unread,
    );
    assert_eq!(info.installed.len(), 1);
    assert_eq!(info.total_bytes, 8_000);
}

#[test]
fn cancel_leaves_no_partial_file_and_frees_the_slot_at_once() {
    let bench = Bench::new();
    let mut tuning = quick();
    tuning.rate = Some(50_000);
    let slow = Downloads::new(
        Arc::clone(bench.downloads.store()),
        Arc::clone(&bench.fetch) as Arc<dyn Fetch>,
        Arc::clone(&bench.mac) as Arc<dyn Machine>,
        tuning,
    );
    let file = bytes(500_000, 5);
    let model = entry("model-a", &file);
    bench.serve(&model, &file);
    slow.start(&model, None, bench.notify()).expect("started");
    let deadline = Instant::now() + Duration::from_secs(5);
    while slow
        .running()
        .is_none_or(|(_, progress)| progress.done == 0)
    {
        assert!(Instant::now() < deadline, "never progressed");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(slow.cancel("model-a"));
    // The slot is free at once (a winding-down read holds nothing up).
    assert!(slow.running().is_none());
    assert_eq!(bench.ended("model-a"), Outcome::Failed(Failure::Cancelled));
    assert_eq!(files(bench.downloads.store().dir()), Vec::<String>::new());
    let info = models_info(slow.store(), Some(&slow), &Listing::Unread);
    assert_eq!(
        info.download.map(|d| d.state),
        Some(DownloadState::Cancelled)
    );
    assert!(info.installed.is_empty());
}

#[test]
fn a_failed_download_says_why_and_leaves_nothing() {
    let bench = Bench::new();
    let tested = bytes(30_000, 6);
    let model = entry("model-a", &tested);
    let other = bytes(30_000, 7);
    bench.serve(&model, &other);
    bench.start(&model, None).expect("started");
    assert_eq!(
        bench.ended("model-a"),
        Outcome::Failed(Failure::HashMismatch)
    );
    assert_eq!(files(bench.downloads.store().dir()), Vec::<String>::new());
    assert!(bench.downloads.store().installed().is_empty());
    let info = models_info(
        bench.downloads.store(),
        Some(&bench.downloads),
        &Listing::Unread,
    );
    let download = info.download.expect("the last download");
    assert_eq!(download.state, DownloadState::Failed);
    assert_eq!(
        download.failure,
        Some(super::dto::DownloadFailure::NotTheTestedFile)
    );
}

#[test]
fn one_download_at_a_time() {
    let bench = Bench::new();
    let mut tuning = quick();
    tuning.rate = Some(50_000);
    let slow = Downloads::new(
        Arc::clone(bench.downloads.store()),
        Arc::clone(&bench.fetch) as Arc<dyn Fetch>,
        Arc::clone(&bench.mac) as Arc<dyn Machine>,
        tuning,
    );
    let file = bytes(300_000, 8);
    let a = entry("model-a", &file);
    let b = entry("model-b", &bytes(1_000, 9));
    bench.serve(&a, &file);
    slow.start(&a, None, bench.notify()).expect("started");
    assert_eq!(slow.start(&b, None, bench.notify()), Err(Refusal::Busy));
    assert!(slow.cancel("model-a"));
    assert_eq!(bench.ended("model-a"), Outcome::Failed(Failure::Cancelled));
}

#[test]
fn a_refusal_maps_to_the_screen_s_answer() {
    let info = super::dto::StartInfo::refused(&Refusal::NeedsApproval {
        size_bytes: 5,
        total_after_bytes: 9,
    });
    assert_eq!(info.refusal, Some(StartRefusal::NeedsApproval));
    assert_eq!(
        (info.size_bytes, info.total_after_bytes),
        (Some(5), Some(9))
    );
    let info = super::dto::StartInfo::refused(&Refusal::NotEnoughDisk {
        needed_bytes: 7,
        free_bytes: 6,
    });
    assert_eq!(info.refusal, Some(StartRefusal::NotEnoughDisk));
    assert_eq!((info.needed_bytes, info.free_bytes), (Some(7), Some(6)));
}
