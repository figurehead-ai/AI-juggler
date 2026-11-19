//! A probe for the helper's sandbox (the 2026-11-19 review's M1), built
//! with the helper's tests and run by them under the helper's own profile
//! (`juggler_local_ai_protocol::SANDBOX_PROFILE`). Never shipped.
//!
//! `sandbox_probe <tmp-file> <home> <port> <models-dir> [control]`, or
//! `sandbox_probe alloc <MiB>` (touches that much memory: the cap's test)
//!
//! It tries each thing a compromised helper might, and writes one line
//! per try, `<name> allowed` or `<name> denied`: reading a file in
//! `/private/tmp`, reading `$HOME` (and through `/System/Volumes/Data`),
//! reading `/etc/hosts`, opening a URL with `/usr/bin/open`, running
//! another program, a TCP connection to `127.0.0.1:<port>`, and writing
//! into the models' folder. The tests require every one denied under the
//! profile, and allowed without it (so the probe can tell). The control
//! run (no profile) skips the URL: with nothing to stop it, `open` could
//! show the system's "no application" alert.

use std::io::Write as _;
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

/// `alloc <MiB>`: touches that much memory (the memory cap's test).
fn alloc(mib: usize) {
    let mut block = vec![0_u8; mib << 20];
    for page in (0..block.len()).step_by(4_096) {
        block[page] = 1;
    }
    let touched = std::hint::black_box(&block).len() >> 20;
    let _ = writeln!(std::io::stdout().lock(), "touched {touched}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [mode, mib] = args.as_slice()
        && mode == "alloc"
    {
        alloc(mib.parse().unwrap_or(0));
        return;
    }
    let control = args.len() == 5 && args[4] == "control";
    let [tmp_file, home, port, models] = args.get(..4).unwrap_or_default() else {
        std::process::exit(2);
    };
    let home = PathBuf::from(home);
    let data_home =
        PathBuf::from("/System/Volumes/Data").join(home.strip_prefix("/").unwrap_or(&home));
    let port: u16 = port.parse().unwrap_or(9);
    let tries: Vec<(&str, bool)> = vec![
        ("tmp_read", std::fs::read(tmp_file).is_ok()),
        ("home_list", std::fs::read_dir(&home).is_ok()),
        (
            "data_volume_home_list",
            std::fs::read_dir(&data_home).is_ok(),
        ),
        ("etc_hosts_read", std::fs::read("/etc/hosts").is_ok()),
        (
            "open_url",
            !control
                && Command::new("/usr/bin/open")
                    .arg("aij-sandbox-probe-unregistered://x")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_ok(),
        ),
        (
            "exec",
            Command::new("/usr/bin/true")
                .stdin(Stdio::null())
                .status()
                .is_ok(),
        ),
        (
            "connect",
            TcpStream::connect_timeout(
                &SocketAddrV4::new(Ipv4Addr::LOCALHOST, port).into(),
                Duration::from_secs(2),
            )
            .is_ok(),
        ),
        (
            "models_write",
            std::fs::write(PathBuf::from(models).join("written-by-probe"), b"x").is_ok(),
        ),
    ];
    let mut out = std::io::stdout().lock();
    for (name, allowed) in tries {
        if control && name == "open_url" {
            continue;
        }
        let _ = writeln!(out, "{name} {}", if allowed { "allowed" } else { "denied" });
    }
}
