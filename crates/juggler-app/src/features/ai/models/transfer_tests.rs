//! AJ-E3813 (ADR-0035 §11.8, §11.10): the transfer against a stub HTTP
//! server on 127.0.0.1, through the app's real streaming client, with
//! files of the entry's exact size. No real model is ever fetched: the
//! stub plays every host, and a pinned `https://` URL reaches it only by
//! the test's own fetch, which maps `https://<host>/<path>` to
//! `http://127.0.0.1:<port>/<host>/<path>` after the transfer has checked
//! the URL.

use std::io::{BufRead as _, BufReader, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use juggler_platform::room::DiskSpace;
use serde_json::json;
use sha2::{Digest as _, Sha256};

use super::transfer::{
    Answer, Control, Failure, Fetch, Hold, Machine, Phase, Plan, Run, Tuning, Wait, hold, run,
};
use super::{ModelEntry, read_untargeted};

/// The stub server, its fetch, a fake Mac, and entries.
pub(super) mod stub {
    use super::*;

    /// One request the stub saw.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Hit {
        /// The host it was meant for.
        pub host: String,
        /// The path (and query) on that host.
        pub path: String,
        /// Every header, lower-case names.
        pub headers: Vec<(String, String)>,
    }

    impl Hit {
        /// A header's value.
        pub fn header(&self, name: &str) -> Option<&str> {
            juggler_account::transport::header_named(&self.headers, name)
        }
    }

    /// How the stub answers one host and path.
    #[derive(Debug, Clone)]
    pub enum Reply {
        /// The file, honouring a `Range`.
        File(File),
        /// A redirect to this URL.
        Redirect(String),
        /// Just a status.
        Status(u16),
    }

    /// A file as the stub serves it.
    #[derive(Debug, Clone)]
    pub struct File {
        /// The bytes.
        pub bytes: Arc<Vec<u8>>,
        /// The `Content-Length` to claim instead of the true one.
        pub claim_length: Option<u64>,
        /// Send no `Content-Length` (the body ends when the connection
        /// closes) and these extra bytes after the file.
        pub extra: Option<Vec<u8>>,
        /// Close the connection after this many body bytes, the first
        /// `cut_times` times it is asked.
        pub cut_after: Option<usize>,
        /// How many times to cut.
        pub cut_times: usize,
        /// Answer a `Range` with the whole file (200).
        pub ignore_range: bool,
    }

    impl File {
        /// The file, served plainly.
        pub fn plain(bytes: &Arc<Vec<u8>>) -> Self {
            Self {
                bytes: Arc::clone(bytes),
                claim_length: None,
                extra: None,
                cut_after: None,
                cut_times: 0,
                ignore_range: false,
            }
        }
    }

    /// A stub HTTP/1.1 server on 127.0.0.1 that plays every host.
    pub struct Server {
        pub port: u16,
        routes: Arc<Mutex<Vec<(String, Reply)>>>,
        hits: Arc<Mutex<Vec<Hit>>>,
        stop: Arc<AtomicBool>,
        thread: Option<std::thread::JoinHandle<()>>,
    }

    impl Server {
        pub fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
            listener.set_nonblocking(true).expect("nonblocking");
            let port = listener.local_addr().expect("addr").port();
            let routes: Arc<Mutex<Vec<(String, Reply)>>> = Arc::default();
            let hits: Arc<Mutex<Vec<Hit>>> = Arc::default();
            let served: Arc<Mutex<Vec<String>>> = Arc::default();
            let stop = Arc::new(AtomicBool::new(false));
            let thread = {
                let (routes, hits, stop) =
                    (Arc::clone(&routes), Arc::clone(&hits), Arc::clone(&stop));
                std::thread::spawn(move || {
                    while !stop.load(Ordering::Relaxed) {
                        match listener.accept() {
                            Ok((stream, _)) => {
                                let (routes, hits, served) =
                                    (Arc::clone(&routes), Arc::clone(&hits), Arc::clone(&served));
                                std::thread::spawn(move || {
                                    serve(stream, &routes, &hits, &served);
                                });
                            }
                            Err(_) => std::thread::sleep(Duration::from_millis(2)),
                        }
                    }
                })
            };
            Self {
                port,
                routes,
                hits,
                stop,
                thread: Some(thread),
            }
        }

        /// Answers `url` (`https://host/path`) with `reply`.
        pub fn route(&self, url: &str, reply: Reply) {
            let key = url.strip_prefix("https://").expect("https").to_owned();
            self.routes.lock().unwrap().push((key, reply));
        }

        /// Every request so far.
        pub fn hits(&self) -> Vec<Hit> {
            self.hits.lock().unwrap().clone()
        }
    }

    impl Drop for Server {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }

    fn serve(
        stream: TcpStream,
        routes: &Mutex<Vec<(String, Reply)>>,
        hits: &Mutex<Vec<Hit>>,
        served: &Mutex<Vec<String>>,
    ) {
        let _ = stream.set_nonblocking(false);
        let mut reader = BufReader::new(stream.try_clone().expect("clone"));
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() {
            return;
        }
        let target = line.split_whitespace().nth(1).unwrap_or("/").to_owned();
        let mut headers = Vec::new();
        loop {
            let mut header = String::new();
            if reader.read_line(&mut header).is_err() || header.trim().is_empty() {
                break;
            }
            if let Some((name, value)) = header.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_owned()));
            }
        }
        let rest = target.trim_start_matches('/');
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let hit = Hit {
            host: host.to_owned(),
            path: format!("/{path}"),
            headers,
        };
        hits.lock().unwrap().push(hit.clone());
        let key = format!("{}{}", hit.host, hit.path);
        let reply = routes
            .lock()
            .unwrap()
            .iter()
            .find(|(route, _)| *route == key)
            .map(|(_, reply)| reply.clone());
        let mut out = stream;
        let _ = match reply {
            None => write!(
                out,
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            Some(Reply::Status(code)) => write!(
                out,
                "HTTP/1.1 {code} X\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            Some(Reply::Redirect(to)) => write!(
                out,
                "HTTP/1.1 302 Found\r\nLocation: {to}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            Some(Reply::File(file)) => send_file(&mut out, &file, &hit, &key, served),
        };
        let _ = out.flush();
    }

    fn send_file(
        out: &mut TcpStream,
        file: &File,
        hit: &Hit,
        key: &str,
        served: &Mutex<Vec<String>>,
    ) -> std::io::Result<()> {
        let size = file.bytes.len();
        let from = hit
            .header("range")
            .and_then(|range| range.strip_prefix("bytes="))
            .and_then(|range| range.trim_end_matches('-').parse::<usize>().ok())
            .filter(|_| !file.ignore_range)
            .unwrap_or(0);
        let times = {
            let mut served = served.lock().unwrap();
            served.push(key.to_owned());
            served.iter().filter(|seen| *seen == key).count()
        };
        let body = &file.bytes[from.min(size)..];
        let mut head = if from > 0 {
            format!(
                "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {from}-{}/{size}\r\n",
                size - 1
            )
        } else {
            "HTTP/1.1 200 OK\r\n".to_owned()
        };
        if file.extra.is_none() {
            let length = file.claim_length.unwrap_or(body.len() as u64);
            head.push_str(&format!("Content-Length: {length}\r\n"));
        }
        head.push_str("Connection: close\r\n\r\n");
        out.write_all(head.as_bytes())?;
        let cut = file.cut_after.filter(|_| times <= file.cut_times);
        let body = cut.map_or(body, |cut| &body[..cut.min(body.len())]);
        for chunk in body.chunks(16 * 1024) {
            out.write_all(chunk)?;
        }
        if cut.is_none()
            && let Some(extra) = &file.extra
        {
            out.write_all(extra)?;
        }
        Ok(())
    }

    /// The test's fetch: the app's real streaming client, pointed at the
    /// stub by mapping `https://<host>/<path>` to the loopback port.
    pub struct Loopback {
        http: juggler_account::UreqTransport,
        port: u16,
        pub calls: AtomicUsize,
    }

    impl Loopback {
        pub fn to(server: &Server) -> Self {
            Self {
                http: juggler_account::UreqTransport::new("0.0.0-test"),
                port: server.port,
                calls: AtomicUsize::new(0),
            }
        }

        pub fn calls(&self) -> usize {
            self.calls.load(Ordering::Relaxed)
        }
    }

    impl Fetch for Loopback {
        fn get(&self, url: &str, from: Option<u64>) -> Result<Answer, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            let rest = url
                .strip_prefix("https://")
                .ok_or("the transfer sent a URL that isn't https")?;
            let answer = self
                .http
                .get_stream(&format!("http://127.0.0.1:{}/{rest}", self.port), from)
                .map_err(|error| error.0)?;
            Ok(Answer {
                status: answer.status,
                headers: answer.headers,
                body: answer.body,
            })
        }
    }

    /// A Mac whose power, link and disk the test sets.
    pub struct FakeMac {
        pub low_power: AtomicBool,
        pub metered: AtomicBool,
        pub busy: AtomicBool,
        pub free: AtomicU64,
        pub total: u64,
    }

    impl FakeMac {
        pub fn with_free(free: u64) -> Self {
            Self {
                low_power: AtomicBool::new(false),
                metered: AtomicBool::new(false),
                busy: AtomicBool::new(false),
                free: AtomicU64::new(free),
                total: 1_000_000_000_000,
            }
        }

        pub fn roomy() -> Self {
            Self::with_free(500_000_000_000)
        }
    }

    impl Machine for FakeMac {
        fn low_power(&self) -> bool {
            self.low_power.load(Ordering::Relaxed)
        }

        fn metered(&self) -> bool {
            self.metered.load(Ordering::Relaxed)
        }

        fn busy(&self) -> bool {
            self.busy.load(Ordering::Relaxed)
        }

        fn disk(&self, _dir: &Path) -> Option<DiskSpace> {
            Some(DiskSpace {
                free: self.free.load(Ordering::Relaxed),
                total: self.total,
            })
        }
    }

    pub const COMMIT: &str = "90862c4b9d2787eaed51d12237eafdfe7c5f6077";

    /// A file of `size` bytes, its content depending on `seed`.
    pub fn bytes(size: usize, seed: u8) -> Arc<Vec<u8>> {
        Arc::new(
            (0..size)
                .map(|at| (at as u8).wrapping_mul(31).wrapping_add(seed))
                .collect(),
        )
    }

    pub fn sha256(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    /// The pinned URL of the model file `name`.
    pub fn pinned(name: &str) -> String {
        format!("https://huggingface.co/Qwen/Test-GGUF/resolve/{COMMIT}/{name}.gguf")
    }

    /// A downloadable entry `id` for `bytes` (the hash and size are the
    /// file's), read through the section's own reader.
    pub fn entry(id: &str, bytes: &[u8]) -> ModelEntry {
        entry_sized(id, &sha256(bytes), bytes.len() as u64)
    }

    /// A downloadable entry with this hash and size.
    pub fn entry_sized(id: &str, sha: &str, size: u64) -> ModelEntry {
        let record = json!({
            "id": id,
            "name": format!("Test {id}"),
            "engine": "gguf_local",
            "url": pinned(id),
            "sha256": sha,
            "size_bytes": size,
            "licence_name": "Apache-2.0",
            "licence_url": format!("https://huggingface.co/Qwen/Test-GGUF/blob/{COMMIT}/LICENSE"),
            "min_chip": "m1",
            "min_memory_gb": 16,
            "review": "beta",
            "chat": "beta"
        });
        let mut entries =
            read_untargeted(&json!({ "schema": 1, "items": [record] })).expect("reads");
        entries.pop().expect("one entry")
    }

    /// Fast knobs for tests: no rate cap, short waits.
    pub fn quick() -> Tuning {
        Tuning {
            chunk: 8 * 1024,
            rate: None,
            pause_poll: Duration::from_millis(50),
            check_every: Duration::ZERO,
            report_every: Duration::ZERO,
            first_wait: Duration::from_millis(5),
            longest_wait: Duration::from_millis(20),
            attempts: 4,
            mirror_after: 2,
        }
    }

    /// Every file in `dir`.
    pub fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map(|list| {
                list.flatten()
                    .map(|item| item.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    }
}

use stub::{FakeMac, File, Loopback, Reply, Server, bytes, entry, files, pinned, quick};

const CDN: &str = "https://cdn-lfs-us-1.hf.co/repos/ab/cd/blob?X-Amz-Signature=abc";

struct Bench {
    server: Server,
    fetch: Loopback,
    mac: FakeMac,
    dir: tempfile::TempDir,
    control: Control,
    tuning: Tuning,
}

impl Bench {
    fn new() -> Self {
        let server = Server::start();
        let fetch = Loopback::to(&server);
        Self {
            server,
            fetch,
            mac: FakeMac::roomy(),
            dir: tempfile::tempdir().expect("dir"),
            control: Control::default(),
            tuning: quick(),
        }
    }

    fn run(&self, entry: &ModelEntry) -> Result<std::path::PathBuf, Failure> {
        self.run_reporting(entry, &mut |_| {})
    }

    fn run_reporting(
        &self,
        entry: &ModelEntry,
        report: &mut dyn FnMut(super::transfer::Progress),
    ) -> Result<std::path::PathBuf, Failure> {
        let plan = Plan::of(entry)?;
        let part = plan.part_in(self.dir.path(), 1);
        run(
            &Run {
                plan: &plan,
                dir: self.dir.path(),
                part: &part,
                fetch: &self.fetch,
                machine: &self.mac,
                control: &self.control,
                tuning: &self.tuning,
            },
            report,
        )
    }
}

#[test]
fn a_file_arrives_verified_through_a_listed_redirect_and_is_named_by_its_hash() {
    let bench = Bench::new();
    let file = bytes(300_000, 1);
    let model = entry("qwen-test", &file);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::Redirect(CDN.to_owned()));
    bench.server.route(CDN, Reply::File(File::plain(&file)));
    let path = bench.run(&model).expect("fetched");
    let sha = stub::sha256(&file);
    assert_eq!(path, bench.dir.path().join(format!("{sha}.gguf")));
    assert_eq!(std::fs::read(&path).expect("read"), *file);
    assert_eq!(files(bench.dir.path()), vec![format!("{sha}.gguf")]);
    let hits = bench.server.hits();
    assert_eq!(
        hits.iter().map(|hit| hit.host.as_str()).collect::<Vec<_>>(),
        vec!["huggingface.co", "cdn-lfs-us-1.hf.co"]
    );
    // A plain GET: no cookie, credentials, referrer or id; the generic
    // user agent only; the host's own query is the only query sent.
    for hit in &hits {
        for (name, _) in &hit.headers {
            assert!(
                ["host", "user-agent", "accept", "range"].contains(&name.as_str()),
                "sent {name}"
            );
        }
        assert!(
            hit.header("user-agent")
                .is_some_and(|ua| ua.starts_with("AIJuggler/"))
        );
    }
    assert!(!hits[0].path.contains('?'));
}

#[test]
fn a_hash_mismatch_deletes_the_file_and_says_so() {
    let bench = Bench::new();
    let tested = bytes(200_000, 1);
    let other = bytes(200_000, 2);
    let model = entry("qwen-test", &tested);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::File(File::plain(&other)));
    assert_eq!(bench.run(&model), Err(Failure::HashMismatch));
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn a_host_off_the_list_is_never_asked() {
    let bench = Bench::new();
    let file = bytes(10_000, 1);
    let mut model = entry("qwen-test", &file);
    for url in [
        "https://evil.example/Qwen/Test-GGUF/resolve/90862c4b9d2787eaed51d12237eafdfe7c5f6077/x.gguf",
        "https://huggingface.co/someone/Test-GGUF/resolve/90862c4b9d2787eaed51d12237eafdfe7c5f6077/x.gguf",
        "http://huggingface.co/Qwen/Test-GGUF/resolve/90862c4b9d2787eaed51d12237eafdfe7c5f6077/x.gguf",
        "https://huggingface.co/Qwen/Test-GGUF/resolve/main/x.gguf",
    ] {
        model.download.as_mut().expect("file").url = url.to_owned();
        assert!(
            matches!(bench.run(&model), Err(Failure::UrlRefused(_))),
            "{url}"
        );
    }
    assert_eq!(bench.fetch.calls(), 0);
    assert!(bench.server.hits().is_empty());
}

#[test]
fn a_redirect_off_the_list_ends_it_unfollowed() {
    let bench = Bench::new();
    let file = bytes(10_000, 1);
    let model = entry("qwen-test", &file);
    for (to, why) in [
        ("https://evil.example/x.gguf", "another host"),
        ("http://cdn-lfs-us-1.hf.co/x.gguf", "not https"),
        ("/api/resolve-cache/x.gguf", "relative"),
        // Userinfo, its `@` kept apart so no address is written here.
        (
            concat!("https://user", "@", "cdn-lfs-us-1.hf.co/x.gguf"),
            "userinfo",
        ),
        ("https://cdn-lfs-us-1.hf.co:8443/x.gguf", "a port"),
    ] {
        let url = pinned(&format!("off-{}", why.replace(' ', "-")));
        let mut model = model.clone();
        model.download.as_mut().expect("file").url = url.clone();
        bench.server.route(&url, Reply::Redirect(to.to_owned()));
        assert_eq!(bench.run(&model), Err(Failure::RedirectRefused), "{why}");
    }
    assert!(
        bench
            .server
            .hits()
            .iter()
            .all(|hit| hit.host == "huggingface.co"),
        "a refused hop was followed"
    );
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn three_redirects_are_followed_and_a_fourth_ends_it() {
    let hop = |n: u32| format!("https://cdn-lfs-us-1.hf.co/hop{n}");
    let file = bytes(20_000, 1);
    let model = entry("qwen-test", &file);
    // `n` hops, then the file.
    let hops = |n: u32| {
        let bench = Bench::new();
        bench
            .server
            .route(&pinned("qwen-test"), Reply::Redirect(hop(1)));
        for at in 1..n {
            bench.server.route(&hop(at), Reply::Redirect(hop(at + 1)));
        }
        bench.server.route(&hop(n), Reply::File(File::plain(&file)));
        bench
    };
    assert!(hops(3).run(&model).is_ok());
    let bench = hops(4);
    assert_eq!(bench.run(&model), Err(Failure::TooManyRedirects));
    assert!(bench.server.hits().iter().all(|hit| hit.path != "/hop4"));
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn a_dropped_connection_resumes_from_the_pinned_url_with_a_range() {
    let bench = Bench::new();
    let file = bytes(400_000, 3);
    let model = entry("qwen-test", &file);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::Redirect(CDN.to_owned()));
    let mut cut = File::plain(&file);
    cut.cut_after = Some(150_000);
    cut.cut_times = 1;
    bench.server.route(CDN, Reply::File(cut));
    let path = bench.run(&model).expect("resumed");
    assert_eq!(std::fs::read(path).expect("read"), *file);
    let hits = bench.server.hits();
    assert_eq!(hits.len(), 4, "{hits:?}");
    // The resume asked the pinned URL again (the signed redirect may have
    // expired), from where the part file ended.
    assert_eq!(hits[2].host, "huggingface.co");
    let range = hits[2].header("range").expect("a range");
    let from: u64 = range
        .strip_prefix("bytes=")
        .and_then(|r| r.strip_suffix('-'))
        .and_then(|n| n.parse().ok())
        .expect("bytes=N-");
    assert!(from > 0 && from <= 150_000, "{range}");
    assert_eq!(hits[3].header("range"), Some(range));
    assert_eq!(hits[0].header("range"), None);
}

#[test]
fn a_host_that_ignores_the_range_starts_again_from_zero() {
    let bench = Bench::new();
    let file = bytes(300_000, 4);
    let model = entry("qwen-test", &file);
    let mut served = File::plain(&file);
    served.cut_after = Some(100_000);
    served.cut_times = 1;
    served.ignore_range = true;
    bench
        .server
        .route(&pinned("qwen-test"), Reply::File(served));
    let path = bench.run(&model).expect("fetched");
    assert_eq!(std::fs::read(path).expect("read"), *file);
}

#[test]
fn cancel_stops_it_and_leaves_no_partial_file() {
    let mut bench = Bench::new();
    // Slow enough to cancel half-way: 400 KB at 100 KB/s.
    bench.tuning.rate = Some(100_000);
    bench.mac.busy.store(true, Ordering::Relaxed);
    let file = bytes(400_000, 5);
    let model = entry("qwen-test", &file);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::File(File::plain(&file)));
    let started = Instant::now();
    let result = std::thread::scope(|scope| {
        let worker = scope.spawn(|| bench.run(&model));
        std::thread::sleep(Duration::from_millis(300));
        bench.control.cancel();
        worker.join().expect("joined")
    });
    assert_eq!(result, Err(Failure::Cancelled));
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "cancel was slow"
    );
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn a_size_that_differs_in_the_header_stops_it_before_a_byte() {
    let bench = Bench::new();
    let file = bytes(50_000, 6);
    let model = entry("qwen-test", &file);
    let mut lying = File::plain(&file);
    lying.claim_length = Some(50_001);
    bench.server.route(&pinned("qwen-test"), Reply::File(lying));
    let mut reports = Vec::new();
    let result = bench.run_reporting(&model, &mut |progress| reports.push(progress));
    assert_eq!(result, Err(Failure::SizeMismatch));
    assert!(reports.iter().all(|progress| progress.done == 0));
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn a_byte_past_the_size_aborts_it() {
    let bench = Bench::new();
    let file = bytes(50_000, 7);
    let model = entry("qwen-test", &file);
    let mut long = File::plain(&file);
    long.extra = Some(vec![0x42]);
    bench.server.route(&pinned("qwen-test"), Reply::File(long));
    assert_eq!(bench.run(&model), Err(Failure::TooLong));
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn a_host_that_never_sends_the_bytes_fails_and_leaves_nothing() {
    let bench = Bench::new();
    let file = bytes(50_000, 8);
    let model = entry("qwen-test", &file);
    // Every answer closes before its first byte.
    let mut short = File::plain(&file);
    short.cut_after = Some(0);
    short.cut_times = usize::MAX;
    bench.server.route(&pinned("qwen-test"), Reply::File(short));
    assert!(matches!(bench.run(&model), Err(Failure::Network(_))));
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn the_mirror_is_tried_only_after_the_pinned_url_fails() {
    let bench = Bench::new();
    let file = bytes(30_000, 9);
    let sha = stub::sha256(&file);
    let mut model = entry("qwen-test", &file);
    let mirror = format!("https://models.figurehead.ai/v1/{sha}/qwen-test.gguf");
    model.download.as_mut().expect("file").mirror_url = Some(mirror.clone());
    bench.server.route(&pinned("qwen-test"), Reply::Status(503));
    bench.server.route(&mirror, Reply::File(File::plain(&file)));
    assert!(bench.run(&model).is_ok());
    let hosts: Vec<String> = bench.server.hits().into_iter().map(|h| h.host).collect();
    assert_eq!(
        hosts,
        vec!["huggingface.co", "huggingface.co", "models.figurehead.ai"]
    );
}

#[test]
fn a_pause_waits_without_spinning_then_goes_on_by_itself() {
    let bench = Bench::new();
    let file = bytes(100_000, 10);
    let model = entry("qwen-test", &file);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::File(File::plain(&file)));
    bench.mac.low_power.store(true, Ordering::Relaxed);
    let phases = Mutex::new(Vec::new());
    let result = std::thread::scope(|scope| {
        let worker = scope.spawn(|| {
            bench.run_reporting(&model, &mut |progress| {
                phases.lock().unwrap().push(progress.phase);
            })
        });
        std::thread::sleep(Duration::from_millis(500));
        // Paused before any request, waiting on its bell: a poll every
        // 50 ms is at most ~11 waits in 500 ms, never a busy loop.
        assert!(bench.server.hits().is_empty());
        let waits = bench.control.waits();
        assert!((1..=14).contains(&waits), "{waits} waits in 500 ms");
        bench.mac.low_power.store(false, Ordering::Relaxed);
        bench.control.nudge();
        worker.join().expect("joined")
    });
    assert!(result.is_ok());
    let phases = phases.into_inner().unwrap();
    assert_eq!(phases.first(), Some(&Phase::Waiting(Wait::LowPower)));
    assert_eq!(phases.last(), Some(&Phase::Verifying));
}

#[test]
fn a_metered_link_pauses_it_too() {
    let bench = Bench::new();
    let file = bytes(10_000, 11);
    let model = entry("qwen-test", &file);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::File(File::plain(&file)));
    bench.mac.metered.store(true, Ordering::Relaxed);
    std::thread::scope(|scope| {
        let worker = scope.spawn(|| bench.run(&model));
        std::thread::sleep(Duration::from_millis(150));
        assert!(bench.server.hits().is_empty());
        bench.control.cancel();
        assert_eq!(worker.join().expect("joined"), Err(Failure::Cancelled));
    });
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn the_disk_margin_holds_at_the_boundary_and_the_low_disk_line_stops_it() {
    let dir = Path::new("/");
    let margin = 10_000_000_000;
    let remaining = 1_834_426_016;
    let mac = FakeMac::with_free(remaining + margin);
    assert_eq!(hold(&mac, dir, remaining, margin), Hold::Go);
    mac.free.store(remaining + margin - 1, Ordering::Relaxed);
    assert_eq!(
        hold(&mac, dir, remaining, margin),
        Hold::Wait(Wait::DiskShort {
            needed: remaining + margin,
            free: remaining + margin - 1,
        })
    );
    // Below the local cache's low-disk line (5 GB of 1 TB): stop.
    mac.free.store(4_999_999_999, Ordering::Relaxed);
    assert_eq!(hold(&mac, dir, 1, margin), Hold::Stop);
}

#[test]
fn a_disk_that_fills_up_while_it_runs_deletes_the_part() {
    let mut bench = Bench::new();
    bench.tuning.rate = Some(200_000);
    bench.mac.busy.store(true, Ordering::Relaxed);
    let file = bytes(200_000, 12);
    let model = entry("qwen-test", &file);
    bench
        .server
        .route(&pinned("qwen-test"), Reply::File(File::plain(&file)));
    let result = std::thread::scope(|scope| {
        let worker = scope.spawn(|| bench.run(&model));
        std::thread::sleep(Duration::from_millis(250));
        bench.mac.free.store(1_000_000_000, Ordering::Relaxed);
        worker.join().expect("joined")
    });
    assert_eq!(result, Err(Failure::DiskFull));
    assert_eq!(files(bench.dir.path()), Vec::<String>::new());
}

#[test]
fn apple_s_model_and_a_bad_hash_make_no_plan() {
    let mut model = entry("qwen-test", &bytes(10, 1));
    model.download.as_mut().expect("file").sha256 = "../../etc".to_owned();
    assert!(matches!(Plan::of(&model), Err(Failure::UrlRefused(_))));
    model.download = None;
    assert!(matches!(Plan::of(&model), Err(Failure::UrlRefused(_))));
}

#[test]
fn the_rate_cap_holds_only_while_the_mac_is_busy() {
    let file = bytes(200_000, 13);
    let model = entry("qwen-test", &file);
    let timed = |busy: bool| {
        let mut bench = Bench::new();
        bench.tuning.rate = Some(100_000);
        bench.mac.busy.store(busy, Ordering::Relaxed);
        bench
            .server
            .route(&pinned("qwen-test"), Reply::File(File::plain(&file)));
        let started = Instant::now();
        assert!(bench.run(&model).is_ok());
        started.elapsed()
    };
    // Idle: full speed (the lowest thread priority is the only brake).
    assert!(timed(false) < Duration::from_millis(1_200));
    // Agents working or the Mac busy: 200 KB at 100 KB/s.
    assert!(timed(true) >= Duration::from_millis(1_500));
}
