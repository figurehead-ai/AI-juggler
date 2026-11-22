//! The local engine: readiness, the cards, and the helper protocol
//! against fake helpers (malformed, oversized, crashed, wrong-id and
//! slow answers; cancel; timeout; a file that fails its hash; memory).
//! The real helper's own protocol tests and its smoke test with a model
//! live in `crates/juggler-local-ai/tests/`.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};

use super::{Host, Local, Seen, Setup, readiness, sampling_of};
use crate::features::ai::engine::{AiEngine, EngineError, Readiness, Reason, Request, Stop};
use crate::features::ai::models::store::Installed;
use crate::features::ai::models::{
    Chip, Engine, Job, JobVerdict, ModelEntry, Sampling, Verdict, read_untargeted,
};

const MODEL: &str = "local-model";
const BYTES: &[u8] = b"GGUF not a real model";

const MAC: Host = Host {
    macos: true,
    apple_silicon: true,
    memory_gb: 32,
};

fn entry(min_memory_gb: u64) -> ModelEntry {
    ModelEntry {
        id: MODEL.to_owned(),
        name: MODEL.to_owned(),
        engine: Engine::GgufLocal,
        download: None,
        min_chip: Chip::M1,
        min_memory_gb,
        min_free_gb: None,
        speed: None,
        sampling: Sampling {
            temperature_milli: Some(700),
            top_p_milli: Some(800),
            top_k: Some(20),
            min_p_milli: None,
            presence_milli: Some(1_500),
            context_tokens: Some(8_192),
            datamark: None,
        },
        verdicts: Job::ALL
            .iter()
            .map(|(job, _)| {
                (
                    *job,
                    JobVerdict {
                        verdict: Verdict::Beta,
                        note: None,
                    },
                )
            })
            .collect(),
        withdrawn: false,
    }
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A models' folder holding the model's file, named by its hash.
fn models(dir: &Path, min_memory_gb: u64, bytes: &[u8]) -> (PathBuf, Vec<Installed>) {
    let models = dir.join("ai-models");
    std::fs::create_dir_all(&models).unwrap();
    let sum = sha(BYTES);
    std::fs::write(models.join(format!("{sum}.gguf")), bytes).unwrap();
    let installed = Installed {
        entry: entry(min_memory_gb),
        sha256: sum,
        size_bytes: BYTES.len() as u64,
        installed_at_ms: 0,
    };
    (models, vec![installed])
}

/// A fake helper: logs its arguments, its environment and every request
/// line to `dir`, says `hello`, answers `load` with `loaded` and each
/// `generate` as `mode` says:
/// - `ok`: a text answer; `garbage`: not JSON; `big`: a line over the
///   bound; `crash`: exits; `wrong_id`: another id's answer;
/// - `cancel`: waits for the next line (the cancel), then says `stopped`;
/// - `hang`: never answers.
///
/// The engine starts a helper only on macOS (through `taskpolicy` and
/// `sandbox-exec`: [`Local::command`] is `None` elsewhere), so the tests
/// that start one run there only; those refused before a start run on
/// every Unix.
#[cfg(unix)]
fn fake(dir: &Path, mode: &str, hello: &str) -> PathBuf {
    let path = dir.join(format!("helper-{mode}"));
    let log = dir.join("requests.log");
    let script = format!(
        r#"#!/bin/sh
echo "$@" > '{args}'
env > '{env}'
echo '{hello}'
while read -r line; do
  printf '%s\n' "$line" >> '{log}'
  id=$(printf '%s' "$line" | sed 's/.*"id":\([0-9]*\).*/\1/')
  case "$line" in
    *'"op":"quit"'*) exit 0;;
    *'"op":"load"'*) echo '{{"kind":"loaded","id":'$id',"context_tokens":8192}}';;
    *'"op":"cancel"'*) ;;
    *'"op":"generate"'*)
      case '{mode}' in
        ok) echo '{{"kind":"text","id":'$id',"text":"fine","finish":"length","tokens":2}}';;
        garbage) echo 'not json';;
        big) head -c 70000 /dev/zero | tr '\0' a; echo;;
        crash) exit 1;;
        wrong_id) echo '{{"kind":"text","id":999999,"text":"x","finish":"stop","tokens":1}}';;
        cancel) read -r next; printf '%s\n' "$next" >> '{log}'; echo '{{"kind":"error","id":'$id',"error":"stopped"}}';;
        hang) sleep 30;;
      esac;;
  esac
done
"#,
        args = dir.join("args.log").display(),
        env = dir.join("env.log").display(),
        log = log.display(),
    );
    crate::test_support::script(&path, &script)
}

#[cfg(unix)]
const HELLO: &str = r#"{"kind":"hello","protocol":3,"gpu":true,"sandboxed":false}"#;

/// Memory pressure as these tests set it: calm for every engine but the
/// pressure test's, which reads [`CRITICAL`]. Only that engine reads the
/// switch, so a plain `cargo test` running the module's tests in parallel
/// threads can't see another test's pressure (nextest's own processes
/// hid it; the 0.5 pre-push run met it as "garbage: NotHere(Unknown)").
#[cfg(target_os = "macos")]
static CRITICAL: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "macos")]
fn critical_now() -> bool {
    CRITICAL.load(std::sync::atomic::Ordering::SeqCst)
}

const fn calm() -> bool {
    false
}

/// A fake helper `program` and these `models`, on a Mac with room,
/// unsandboxed.
fn setup(program: PathBuf, models: (PathBuf, Vec<Installed>)) -> Setup {
    Setup {
        program: Some(program),
        models: Some(models),
        host: Some(MAC),
        pressure: Some(calm),
        unsandboxed: true,
        sandbox_tool: None,
    }
}

#[cfg(unix)]
fn engine(dir: &Path, mode: &str, min_memory_gb: u64) -> Local {
    Local::with(setup(
        fake(dir, mode, HELLO),
        models(dir, min_memory_gb, BYTES),
    ))
}

fn request() -> Request {
    Request {
        instructions: "Say one line.".to_owned(),
        prompt: "Message to summarise".to_owned(),
        shape: Some(json!({
            "type": "object",
            "name": "Line",
            "properties": [{ "name": "line", "schema": "string" }]
        })),
        max_tokens: 60,
    }
}

fn soon() -> Stop {
    Stop::at(Instant::now() + Duration::from_secs(10))
}

#[cfg(unix)]
fn lines(dir: &Path) -> Vec<Value> {
    std::fs::read_to_string(dir.join("requests.log"))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

/// The job is refused for `why`, and the helper was asked to load
/// nothing.
#[cfg(unix)]
fn refused_unloaded(engine: &Local, dir: &Path, why: Reason) {
    assert_eq!(
        engine.respond(MODEL, &request(), &soon()),
        Err(EngineError::NotHere(why))
    );
    assert_eq!(ops(dir), Vec::<String>::new(), "nothing was loaded");
}

#[cfg(unix)]
fn ops(dir: &Path) -> Vec<String> {
    lines(dir)
        .iter()
        .map(|line| line["op"].as_str().unwrap().to_owned())
        .collect()
}

fn card(engine: &Local) -> crate::features::ai::engine::Card {
    engine.cards(1, &[entry(16)]).remove(0)
}

#[test]
fn readiness_needs_a_mac_with_apple_silicon_the_memory_and_the_helper() {
    let intel = Host {
        apple_silicon: false,
        ..MAC
    };
    let other = Host {
        macos: false,
        ..MAC
    };
    let cases = [
        (
            other,
            16,
            Seen::Ready,
            false,
            Readiness::NotHere(Reason::NotThisSystem),
        ),
        (
            intel,
            16,
            Seen::Ready,
            false,
            Readiness::NotHere(Reason::NotThisComputer),
        ),
        (
            MAC,
            64,
            Seen::Ready,
            false,
            Readiness::NotHere(Reason::NotThisComputer),
        ),
        (
            MAC,
            16,
            Seen::Missing,
            false,
            Readiness::NotHere(Reason::HelperMissing),
        ),
        (
            MAC,
            16,
            Seen::NotAsked,
            false,
            Readiness::NotHere(Reason::Unknown),
        ),
        (
            MAC,
            16,
            Seen::Failed,
            false,
            Readiness::NotHere(Reason::Unknown),
        ),
        (
            MAC,
            16,
            Seen::Ready,
            true,
            Readiness::NotHere(Reason::Unhealthy),
        ),
        (MAC, 32, Seen::Ready, false, Readiness::Ready),
    ];
    for (host, memory, seen, unhealthy, want) in cases {
        assert_eq!(
            readiness(host, memory, seen, unhealthy),
            want,
            "{host:?} {memory} {seen:?}"
        );
    }
}

#[test]
fn with_no_model_installed_there_is_no_card_and_nothing_is_started() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Local::with(setup(
        dir.path().join("never-run"),
        (dir.path().to_path_buf(), Vec::new()),
    ));
    engine.probe();
    assert_eq!(engine.seen(), Seen::NotAsked);
    assert!(engine.cards(1, &[entry(16)]).is_empty());
    assert_eq!(
        engine.respond(MODEL, &request(), &soon()),
        Err(EngineError::Unsupported)
    );
}

#[test]
fn a_missing_helper_hides_the_card_with_its_reason() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Local::with(setup(
        dir.path().join("not-built"),
        models(dir.path(), 16, BYTES),
    ));
    engine.probe();
    assert_eq!(engine.seen(), Seen::Missing);
    assert_eq!(
        card(&engine).ready,
        Readiness::NotHere(Reason::HelperMissing)
    );
    assert_eq!(
        engine.respond(MODEL, &request(), &soon()),
        Err(EngineError::NotHere(Reason::HelperMissing))
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_job_loads_the_model_once_then_generates_with_its_frame_shape_and_sampling() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine(dir.path(), "ok", 16);
    engine.probe();
    assert_eq!(engine.seen(), Seen::Ready);
    let first = card(&engine);
    assert_eq!(first.ready, Readiness::Ready);
    assert_eq!(first.context_tokens, 8_192);
    assert!(
        first.uses_download && !first.running,
        "a probe lets its helper go"
    );
    assert_eq!(
        ops(dir.path()),
        Vec::<String>::new(),
        "a probe sends nothing"
    );

    let draft = engine.respond(MODEL, &request(), &soon()).unwrap();
    assert_eq!(
        (draft.text.as_str(), draft.finish),
        ("fine", crate::features::ai::engine::Finish::Length)
    );
    assert!(card(&engine).running, "it stays loaded for the next job");
    engine.respond(MODEL, &request(), &soon()).unwrap();
    assert_eq!(ops(dir.path()), ["load", "generate", "generate"]);

    let sent = lines(dir.path());
    let sum = sha(BYTES);
    // Review L1: the helper is told which file on disk was hashed.
    let id_on_disk = juggler_platform::transfer::file_id(
        &dir.path().join("ai-models").join(format!("{sum}.gguf")),
    )
    .unwrap();
    assert_eq!(
        sent[0],
        json!({"op": "load", "id": 1, "file": format!("{sum}.gguf"),
               "size_bytes": BYTES.len(),
               "device": id_on_disk.device, "inode": id_on_disk.inode,
               "context_tokens": 8192}),
        "the file by its name only, never a path"
    );
    let generate = &sent[1];
    assert_eq!(generate["instructions"], "Say one line.");
    assert_eq!(generate["prompt"], "Message to summarise");
    assert_eq!(
        generate["guide"]["json_schema"],
        r#"{"type":"object","properties":{"line":{"type":"string"}},"required":["line"],"additionalProperties":false}"#
    );
    assert_eq!(
        generate["sampling"],
        json!({"temperature_milli": 700, "top_p_milli": 800, "top_k": 20, "presence_milli": 1500})
    );
    assert_eq!(generate["max_tokens"], 60);
    let timeout = generate["timeout_ms"].as_u64().unwrap();
    assert!(timeout > 5_000 && timeout <= 10_000, "{timeout}");

    let args = std::fs::read_to_string(dir.path().join("args.log")).unwrap();
    assert_eq!(
        args.trim(),
        format!(
            "--models-dir {}",
            std::fs::canonicalize(dir.path().join("ai-models"))
                .unwrap()
                .display()
        ),
        "the folder resolved, as the sandbox's parameter is"
    );
    let env = std::fs::read_to_string(dir.path().join("env.log")).unwrap();
    assert!(env.lines().any(|l| l == "GGML_NO_BACKTRACE=1"), "{env}");
    assert!(
        env.lines()
            .any(|l| l == "PATH=/usr/bin:/bin:/usr/sbin:/sbin")
    );

    engine.release();
    assert!(!card(&engine).running);
}

#[cfg(target_os = "macos")]
#[test]
fn broken_answers_are_crashes_and_three_make_it_unhealthy() {
    for mode in ["garbage", "big", "crash", "wrong_id"] {
        let dir = tempfile::tempdir().unwrap();
        let engine = engine(dir.path(), mode, 16);
        engine.probe();
        for _ in 0..3 {
            assert_eq!(
                engine.respond(MODEL, &request(), &soon()),
                Err(EngineError::Crashed),
                "{mode}"
            );
            assert!(!card(&engine).running, "{mode}: a broken helper is stopped");
        }
        assert_eq!(
            card(&engine).ready,
            Readiness::NotHere(Reason::Unhealthy),
            "{mode}"
        );
        assert_eq!(
            engine.respond(MODEL, &request(), &soon()),
            Err(EngineError::NotHere(Reason::Unhealthy))
        );
    }
}

#[cfg(target_os = "macos")]
#[test]
fn a_helper_on_another_protocol_is_never_used() {
    let dir = tempfile::tempdir().unwrap();
    for hello in [
        r#"{"kind":"hello","protocol":2,"gpu":true,"sandboxed":true}"#,
        r#"{"kind":"hello","protocol":4,"gpu":true,"sandboxed":true}"#,
        r#"{"kind":"hello","protocol":3,"gpu":true}"#,
        r#"{"kind":"hello","protocol":3,"gpu":true,"sandboxed":true,"shell":"sh"}"#,
        r#"{"hello":true,"protocol":2,"available":"available"}"#,
        "not json",
    ] {
        let engine = Local::with(setup(
            fake(dir.path(), "ok", hello),
            models(dir.path(), 16, BYTES),
        ));
        engine.probe();
        assert_eq!(engine.seen(), Seen::Failed, "{hello}");
        assert_eq!(card(&engine).ready, Readiness::NotHere(Reason::Unknown));
    }
}

#[cfg(target_os = "macos")]
#[test]
fn an_oversized_request_is_refused_before_it_is_sent() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine(dir.path(), "ok", 16);
    engine.probe();
    let mut big = request();
    big.prompt = "a".repeat(juggler_local_ai_protocol::MAX_REQUEST_BYTES);
    assert_eq!(
        engine.respond(MODEL, &big, &soon()),
        Err(EngineError::TooLong)
    );
    assert_eq!(ops(dir.path()), ["load"], "no generate line went out");
    assert!(card(&engine).running, "the helper stays");
    let mut nul = request();
    nul.prompt = "a\0b".to_owned();
    assert_eq!(
        engine.respond(MODEL, &nul, &soon()),
        Err(EngineError::Unsupported)
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_stop_cancels_the_generation_and_keeps_the_helper_that_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine(dir.path(), "cancel", 16);
    engine.probe();
    let stop = soon();
    let stopper = stop.clone();
    let started = Instant::now();
    let later = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        stopper.stop();
    });
    assert_eq!(
        engine.respond(MODEL, &request(), &stop),
        Err(EngineError::Stopped)
    );
    later.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(3));
    let sent = lines(dir.path());
    assert_eq!(ops(dir.path()), ["load", "generate", "cancel"]);
    assert_eq!(
        sent[2]["id"], sent[1]["id"],
        "the cancel names the generation"
    );
    assert!(card(&engine).running, "a helper that stopped stays loaded");
}

#[cfg(target_os = "macos")]
#[test]
fn a_helper_past_its_deadline_that_doesnt_stop_is_stopped() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine(dir.path(), "hang", 16);
    engine.probe();
    let started = Instant::now();
    assert_eq!(
        engine.respond(
            MODEL,
            &request(),
            &Stop::at(Instant::now() + Duration::from_millis(400))
        ),
        Err(EngineError::TimedOut)
    );
    let took = started.elapsed();
    assert!(
        took < Duration::from_millis(400) + super::CANCEL_GRACE + Duration::from_secs(1),
        "{took:?}"
    );
    // It never read the cancel (it hangs), so its log ends at the generate.
    assert_eq!(ops(dir.path()), ["load", "generate"]);
    assert!(
        !card(&engine).running,
        "a helper that didn't stop is stopped"
    );
}

#[cfg(target_os = "macos")]
#[test]
fn a_file_that_fails_its_hash_is_never_loaded() {
    let dir = tempfile::tempdir().unwrap();
    let program = fake(dir.path(), "ok", HELLO);
    // The same size, other bytes: the name's hash doesn't match.
    let engine = Local::with(setup(
        program,
        models(dir.path(), 16, b"GGUF not a real modeL"),
    ));
    engine.probe();
    refused_unloaded(&engine, dir.path(), Reason::Unknown);
    assert!(!card(&engine).running);
}

#[cfg(unix)]
#[test]
fn a_mac_below_the_models_memory_never_starts_it() {
    let dir = tempfile::tempdir().unwrap();
    let engine = engine(dir.path(), "ok", 64);
    engine.probe();
    assert_eq!(
        engine.cards(1, &[entry(64)])[0].ready,
        Readiness::NotHere(Reason::NotThisComputer)
    );
    refused_unloaded(&engine, dir.path(), Reason::NotThisComputer);
}

/// The section's sampling bounds and the protocol's are the same: a value
/// the section accepts is one the helper accepts, at each upper bound.
#[test]
fn the_sections_sampling_bounds_are_the_protocols() {
    let record = |extra: Value| {
        let mut record = json!({
            "id": "m",
            "name": "M",
            "engine": "gguf_local",
            "url": "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/resolve/90862c4b9d2787eaed51d12237eafdfe7c5f6077/Qwen3-1.7B-Q8_0.gguf",
            "sha256": "061b54daade076b5d3362dac252678d17da8c68f07560be70818cace6590cb1a",
            "size_bytes": 1_834_426_016u64,
            "licence_name": "Apache-2.0",
            "licence_url": "https://huggingface.co/Qwen/Qwen3-1.7B-GGUF/blob/90862c4b9d2787eaed51d12237eafdfe7c5f6077/LICENSE",
            "min_chip": "m1",
            "min_memory_gb": 16,
            "review": "beta"
        });
        for (key, value) in extra.as_object().unwrap() {
            record[key] = value.clone();
        }
        read_untargeted(&json!({ "schema": 1, "items": [record] })).unwrap()
    };
    use juggler_local_ai_protocol::bounds;
    let fields = [
        ("temperature_milli", bounds::TEMPERATURE_MILLI),
        ("top_p_milli", bounds::TOP_P_MILLI),
        ("top_k", bounds::TOP_K),
        ("min_p_milli", bounds::MIN_P_MILLI),
        ("presence_milli", bounds::PRESENCE_MILLI),
        ("context_tokens", bounds::CONTEXT_TOKENS),
    ];
    for (field, range) in fields {
        let below = range.min.checked_sub(1);
        for (value, kept) in [(range.min, true), (range.max, true), (range.max + 1, false)]
            .into_iter()
            .chain(below.map(|n| (n, false)))
        {
            // The section keeps the record and reads an out-of-bounds
            // value as absent (the engine's default), never passes it on.
            let model = record(json!({ field: value })).remove(0);
            let sampling = sampling_of(&model);
            assert!(sampling.in_bounds(), "{field}={value}");
            let sent = serde_json::to_value(sampling).unwrap();
            let reached = if field == "context_tokens" {
                model.sampling.context_tokens.map(Value::from)
            } else {
                sent.get(field).cloned()
            };
            assert_eq!(reached, kept.then(|| json!(value)), "{field}={value}");
        }
    }
}

/// Review M1, L3 and the re-review's N1: the helper runs as
/// `sandbox-exec -p <profile> -D HELPER=… -D MODELS=… -D CACHE=…
/// taskpolicy -c utility -m <cap> <helper> --models-dir <dir>` (the cap
/// set after the sandbox's exec, so it holds; the helper's own tests kill
/// an allocation over it through this very chain), in the models' folder,
/// with the allowed environment and GGML_NO_BACKTRACE=1.
#[cfg(target_os = "macos")]
#[test]
fn the_helper_starts_sandboxed_then_capped_and_in_its_folder() {
    let program = Path::new("/Applications/AI Juggler.app/Contents/MacOS/juggler-local-ai");
    let dir = Path::new("/Users/someone/Library/Application Support/AI Juggler/ai-models");
    let tool = Path::new("/usr/bin/sandbox-exec");
    let command = Local::command(program, dir, 5_000, Some(tool)).unwrap();
    assert_eq!(command.get_program(), "/usr/bin/sandbox-exec");
    let args: Vec<String> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let cache = super::metal_cache();
    assert_eq!(
        args,
        [
            "-p",
            juggler_local_ai_protocol::SANDBOX_PROFILE,
            "-D",
            &format!("HELPER={}", program.display()),
            "-D",
            &format!("MODELS={}", dir.display()),
            "-D",
            &format!("CACHE={}", cache.display()),
            "/usr/sbin/taskpolicy",
            "-c",
            "utility",
            "-m",
            "5000",
            &program.display().to_string(),
            "--models-dir",
            &dir.display().to_string(),
        ]
    );
    assert_eq!(command.get_current_dir(), Some(dir));
    let env: Vec<(String, Option<String>)> = command
        .get_envs()
        .map(|(k, v)| {
            (
                k.to_string_lossy().into_owned(),
                v.map(|v| v.to_string_lossy().into_owned()),
            )
        })
        .collect();
    assert!(env.contains(&("GGML_NO_BACKTRACE".to_owned(), Some("1".to_owned()))));
    assert!(env.contains(&(
        "PATH".to_owned(),
        Some("/usr/bin:/bin:/usr/sbin:/sbin".to_owned())
    )));
    assert!(
        !env.iter()
            .any(|(k, _)| k == juggler_local_ai_protocol::UNSANDBOXED_FOR_TESTS),
        "the app never lets the helper serve unsandboxed"
    );
    assert!(
        cache.ends_with("ai.figurehead.juggler.local-ai") || cache.starts_with("/nonexistent"),
        "{}",
        cache.display()
    );
    // Without the sandbox (a fake helper's tests) it starts at taskpolicy.
    let bare = Local::command(program, dir, 1_024, None).unwrap();
    assert_eq!(bare.get_program(), "/usr/sbin/taskpolicy");
}

/// A stand-in for the sandbox tool: `refuses` exits as `sandbox-exec`
/// does on a profile it can't read; `ignores` runs the rest without
/// applying anything (what N3 guards against).
#[cfg(target_os = "macos")]
fn fake_sandbox_tool(dir: &Path, mode: &str) -> PathBuf {
    let path = dir.join(format!("sandbox-{mode}"));
    let script = match mode {
        "refuses" => "#!/bin/sh\nexit 65\n".to_owned(),
        _ => "#!/bin/sh\nwhile [ \"$1\" = -p ] || [ \"$1\" = -D ]; do shift 2; done\nexec \"$@\"\n"
            .to_owned(),
    };
    crate::test_support::script(&path, &script)
}

/// The re-review's N3, through the engine: a missing sandbox tool, a
/// refused profile, and a helper that reports it isn't sandboxed each
/// hide the jobs (as a missing helper), are recorded as the engine's own
/// reason, and never run a job: none fails open or quietly.
#[cfg(target_os = "macos")]
#[test]
fn a_sandbox_that_can_t_be_applied_hides_the_jobs_and_says_why() {
    let cases = [
        ("missing", Seen::SandboxToolMissing),
        ("refuses", Seen::SandboxRejected),
        ("ignores", Seen::NotSandboxed),
    ];
    for (mode, want) in cases {
        let dir = tempfile::tempdir().unwrap();
        let tool = match mode {
            "missing" => dir.path().join("no-sandbox-tool"),
            _ => fake_sandbox_tool(dir.path(), mode),
        };
        let engine = Local::with(Setup {
            unsandboxed: false,
            sandbox_tool: Some(tool),
            ..setup(fake(dir.path(), "ok", HELLO), models(dir.path(), 16, BYTES))
        });
        engine.probe();
        assert_eq!(engine.seen(), want, "{mode}");
        assert!(want.sandbox_fault());
        assert_eq!(
            card(&engine).ready,
            Readiness::NotHere(Reason::HelperMissing),
            "{mode}: the jobs are hidden"
        );
        assert_eq!(
            engine.respond(MODEL, &request(), &soon()),
            Err(EngineError::NotHere(Reason::HelperMissing)),
            "{mode}"
        );
        assert_eq!(
            ops(dir.path()),
            Vec::<String>::new(),
            "{mode}: nothing loaded"
        );
    }
}

#[test]
fn the_memory_cap_follows_the_model_within_the_mac() {
    let gib = 1_u64 << 30;
    // 2 GiB file: twice it and 2 GiB more.
    assert_eq!(super::cap_mib(2 * gib, MAC), 6_144);
    // Never under the bare cap, never over three quarters of memory.
    assert_eq!(super::cap_mib(1, MAC), 2_050);
    assert_eq!(
        super::cap_mib(
            0,
            Host {
                memory_gb: 1,
                ..MAC
            }
        ),
        super::BARE_CAP_MIB
    );
    let small = Host {
        memory_gb: 8,
        ..MAC
    };
    assert_eq!(
        super::cap_mib(8 * gib, small),
        8_000_000_000 / (1 << 20) * 3 / 4
    );
}

#[cfg(target_os = "macos")]
#[test]
fn critical_memory_pressure_never_starts_a_model_and_stops_one_running() {
    let dir = tempfile::tempdir().unwrap();
    let engine = Local::with(Setup {
        pressure: Some(critical_now),
        ..setup(
            fake(dir.path(), "hang", HELLO),
            models(dir.path(), 16, BYTES),
        )
    });
    engine.probe();
    CRITICAL.store(true, std::sync::atomic::Ordering::SeqCst);
    refused_unloaded(&engine, dir.path(), Reason::Unknown);
    CRITICAL.store(false, std::sync::atomic::Ordering::SeqCst);
    let later = std::thread::spawn(|| {
        std::thread::sleep(Duration::from_millis(400));
        CRITICAL.store(true, std::sync::atomic::Ordering::SeqCst);
    });
    let started = Instant::now();
    assert_eq!(
        engine.respond(MODEL, &request(), &soon()),
        Err(EngineError::Stopped)
    );
    later.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(4));
    assert_eq!(ops(dir.path()), ["load", "generate"]);
    assert!(!card(&engine).running, "its memory is let go");
    CRITICAL.store(false, std::sync::atomic::Ordering::SeqCst);
}
