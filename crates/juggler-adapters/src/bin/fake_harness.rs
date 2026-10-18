//! `fake-harness` — the scriptable stand-in harness used across AI Juggler's
//! test suite (M0-4, spec §4 design rules).
//!
//! It behaves like a minimal coding-harness CLI so lifecycle, watcher, and
//! recovery paths can be tested deterministically:
//!
//! ```text
//! fake-harness --version
//! fake-harness run --dir DIR [--session-id ID | --resume ID] [--turns N]
//!               [--delay-ms MS] [--stall-secs S] [--burn-cpus SECS]
//!               [--exit CODE] [--prompt TEXT] [--wait-prompt TEXT]
//!               [--quota-exit] [--spawn-child PROG --child-args "A B"]
//!               [--child-ignore-term]
//! ```
//!
//! Sessions are JSONL files under `--dir`: a `session_meta` line, turn pairs
//! of `user`/`assistant` lines (assistant lines carry token usage), and a
//! terminal `ended` line. `--stall-secs` sleeps silently, `--burn-cpus`
//! busy-loops (stuck-detection fixtures), `--exit` sets the exit code
//! (crash fixtures). `--resume` appends to an existing session file.
//! `--spawn-child` starts a detached child (stdio null, so it never holds
//! the harness's output pipes) that OUTLIVES the harness — the orphan
//! fixture (AJ-E410); `--child-ignore-term` makes it ignore SIGTERM
//! (the stop-ladder escalation fixture, unix only).

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const VERSION: &str = "0.1.0";

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("fake-harness {VERSION}");
            ExitCode::SUCCESS
        }
        Some("run") => run(&args[1..]),
        _ => {
            eprintln!("usage: fake-harness --version | run --dir DIR [options]");
            ExitCode::from(2)
        }
    }
}

struct RunOptions {
    dir: PathBuf,
    session_id: Option<String>,
    resume: Option<String>,
    turns: u32,
    delay_ms: u64,
    stall_secs: u64,
    burn_cpus_secs: u64,
    exit_code: u8,
    prompt: String,
    /// Print a permission prompt and block until stdin closes (the
    /// wait-permission fixture, AJ-E405).
    wait_prompt: Option<String>,
    /// Print a rate-limit signature and exit non-zero (the quota-exit
    /// fixture, AJ-E405).
    quota_exit: bool,
    /// Detached child program to start (survives the harness — the
    /// orphan fixture, AJ-E410).
    spawn_child: Option<String>,
    /// Arguments for `--spawn-child`, whitespace-split.
    child_args: String,
    /// Make the child ignore SIGTERM (stop-ladder escalation fixture).
    child_ignore_term: bool,
}

fn parse_options(args: &[String]) -> Result<RunOptions, String> {
    let mut opts = RunOptions {
        dir: PathBuf::from("."),
        session_id: None,
        resume: None,
        turns: 2,
        delay_ms: 0,
        stall_secs: 0,
        burn_cpus_secs: 0,
        exit_code: 0,
        prompt: String::new(),
        wait_prompt: None,
        quota_exit: false,
        spawn_child: None,
        child_args: String::new(),
        child_ignore_term: false,
    };
    let mut i = 0;
    while i < args.len() {
        let arg = args[i].clone();
        let value = || {
            args.get(i + 1)
                .cloned()
                .ok_or_else(|| format!("missing value for {arg}"))
        };
        let takes_value = match arg.as_str() {
            "--dir" => {
                opts.dir = PathBuf::from(value()?);
                true
            }
            "--session-id" => {
                opts.session_id = Some(value()?);
                true
            }
            "--resume" => {
                opts.resume = Some(value()?);
                true
            }
            "--turns" => {
                opts.turns = parse_num(&value()?)?;
                true
            }
            "--delay-ms" => {
                opts.delay_ms = parse_num(&value()?)?;
                true
            }
            "--stall-secs" => {
                opts.stall_secs = parse_num(&value()?)?;
                true
            }
            "--burn-cpus-secs" => {
                opts.burn_cpus_secs = parse_num(&value()?)?;
                true
            }
            "--exit" => {
                opts.exit_code = parse_num(&value()?)?;
                true
            }
            "--prompt" => {
                opts.prompt = value()?;
                true
            }
            "--wait-prompt" => {
                opts.wait_prompt = Some(value().unwrap_or_else(|_| "yes".into()));
                true
            }
            // Boolean flags (AJ-E405 fixtures) advance by one.
            "--quota-exit" => {
                opts.quota_exit = true;
                false
            }
            "--child-ignore-term" => {
                opts.child_ignore_term = true;
                false
            }
            "--spawn-child" => {
                opts.spawn_child = Some(value()?);
                true
            }
            "--child-args" => {
                opts.child_args = value()?;
                true
            }
            other => return Err(format!("unknown option {other}")),
        };
        i += if takes_value { 2 } else { 1 };
    }
    Ok(opts)
}

fn parse_num<T>(s: &str) -> Result<T, String>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    s.parse().map_err(|e| format!("invalid number {s:?}: {e}"))
}

/// Builds the fixture-child command. `ignore_term` wraps the program in
/// `bash -c "trap '' TERM; exec …"` — `SIG_IGN` survives `exec(2)`, so the
/// child ignores SIGTERM and the supervisor's stop ladder must escalate
/// to SIGKILL. (The workspace forbids unsafe code, so `pre_exec` signal
/// dispositions are not an option; the wrapper shell is the portable
/// equivalent.) Unix-only effect; elsewhere the flag degrades to a
/// plain child.
fn spawn_child_command(
    program: &str,
    child_args: &str,
    ignore_term: bool,
) -> std::process::Command {
    #[cfg(unix)]
    if ignore_term {
        let script = format!("trap '' TERM; exec {program} {child_args}");
        let mut command = std::process::Command::new("/bin/bash");
        command.arg("-c").arg(script);
        return command;
    }
    #[cfg(not(unix))]
    if ignore_term {
        eprintln!("fake-harness: --child-ignore-term is a unix-only fixture");
    }
    let mut command = std::process::Command::new(program);
    command.args(child_args.split_whitespace());
    command
}

fn run(args: &[String]) -> ExitCode {
    let opts = match parse_options(args) {
        Ok(opts) => opts,
        Err(message) => {
            eprintln!("fake-harness: {message}");
            return ExitCode::from(2);
        }
    };

    if let Err(message) = std::fs::create_dir_all(&opts.dir) {
        eprintln!(
            "fake-harness: cannot create {}: {message}",
            opts.dir.display()
        );
        return ExitCode::from(2);
    }

    let session_id = opts.resume.clone().unwrap_or_else(|| {
        opts.session_id
            .clone()
            .unwrap_or_else(|| format!("sess-{}", nanos()))
    });
    let transcript = opts.dir.join(format!("{session_id}.jsonl"));

    let file = match OpenOptions::new()
        .create(true)
        .append(true)
        .open(&transcript)
    {
        Ok(file) => file,
        Err(message) => {
            eprintln!(
                "fake-harness: cannot open {}: {message}",
                transcript.display()
            );
            return ExitCode::from(2);
        }
    };
    let mut out = LineWriter::new(file, transcript.clone());

    // The orphan fixture (AJ-E410): a detached child that outlives the
    // harness. stdio is null so it never holds the harness's output
    // pipes — the supervisor's readers must see EOF when the harness
    // itself exits, not when the leftover child dies.
    if let Some(program) = &opts.spawn_child {
        let child = spawn_child_command(program, &opts.child_args, opts.child_ignore_term);
        let mut child = child;
        child
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        match child.spawn() {
            Ok(spawned) => println!("spawned child {program} (pid {})", spawned.id()),
            Err(message) => eprintln!("fake-harness: cannot spawn {program:?}: {message}"),
        }
    }

    let is_resume = opts.resume.is_some() && line_count(&transcript) > 0;
    let header = if is_resume {
        format!(r#"{{"type":"resumed","id":"{session_id}"}}"#)
    } else {
        format!(
            r#"{{"type":"session_meta","id":"{session_id}","prompt":{:?}}}"#,
            opts.prompt
        )
    };
    if out.line(&header).is_err() {
        return ExitCode::from(2);
    }

    for turn in 1..=opts.turns {
        if opts.delay_ms > 0 {
            thread::sleep(Duration::from_millis(opts.delay_ms));
        }
        println!("turn {}/{}", turn, opts.turns);
        let user = format!(r#"{{"type":"user","turn":{turn},"text":"turn {turn}"}}"#);
        let assistant = format!(
            r#"{{"type":"assistant","turn":{turn},"usage":{{"input_tokens":100,"output_tokens":50}}}}"#
        );
        if out.line(&user).is_err() || out.line(&assistant).is_err() {
            return ExitCode::from(2);
        }
    }

    if opts.stall_secs > 0 {
        thread::sleep(Duration::from_secs(opts.stall_secs));
    }
    if let Some(prompt) = &opts.wait_prompt {
        // The wait-permission fixture: an ALIVE, unanswered prompt. The
        // harness blocks (no stdin contract — supervised runs are
        // headless); the test's stop ladder reaps it.
        println!("Do you want to {prompt}? (y/n)");
        thread::sleep(Duration::from_secs(600));
    }
    if opts.quota_exit {
        // The quota-exit fixture: a rate-limit signature, then a
        // non-zero exit (the AJ-E405 classifier's target).
        eprintln!("Error: usage limit reached. Your 5-hour window resets shortly.");
        let _ = std::io::stderr().flush();
        return ExitCode::FAILURE;
    }
    if opts.burn_cpus_secs > 0 {
        let deadline = Instant::now() + Duration::from_secs(opts.burn_cpus_secs);
        let mut sink = 0_u128;
        while Instant::now() < deadline {
            sink = sink.wrapping_add(1);
        }
        let _ = sink;
    }

    let ended = format!(r#"{{"type":"ended","id":"{session_id}"}}"#);
    if out.line(&ended).is_err() {
        return ExitCode::from(2);
    }

    ExitCode::from(opts.exit_code)
}

/// Writes one JSONL line with immediate flush; errors are reported once.
struct LineWriter {
    file: File,
    path: PathBuf,
}

impl LineWriter {
    fn new(file: File, path: PathBuf) -> Self {
        Self { file, path }
    }

    fn line(&mut self, content: &str) -> Result<(), ()> {
        writeln!(self.file, "{content}")
            .and_then(|()| self.file.flush())
            .map_err(|message| {
                eprintln!("fake-harness: {}: {message}", self.path.display());
            })
    }
}

/// Counts existing lines so `--resume` on a fresh name behaves like a new
/// session instead of a corrupted one.
fn line_count(path: &Path) -> usize {
    File::open(path).map_or(0, |file| BufReader::new(file).lines().count())
}

fn nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or_default()
        % 1_000_000_000
}
