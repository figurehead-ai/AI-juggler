//! `remote-config-publish`: the `Publish config` workflow's tool (ADR-0035
//! §8). Offline; it reads files and the one secret, and writes files.
//!
//! ```text
//! remote-config-publish validate    --schema S --source F --channel C
//! remote-config-publish next-serial --channel C [--live F]
//! remote-config-publish sign        --schema S --source F --channel C --serial N
//!                                   --now T [--days D] [--live F] --out O
//! remote-config-publish verify      --file F --channel C [--now T] [--serial N]
//! remote-config-publish inspect     --file F --channel C
//! remote-config-publish keygen      [--id ID]
//! ```
//!
//! - `sign` reads the private key from `REMOTE_CONFIG_SIGNING_KEY` (the
//!   32-byte ed25519 seed, base64) and never prints it. The key must be
//!   pinned in this build ([`juggler_remote_config::keys::COMPILED_KEYS`])
//!   as `current` or `next`. `--live` carries the live file's revocation
//!   parts over unchanged.
//! - `verify`, `inspect` and `next-serial` check with this build's pinned
//!   keys and compiled baseline, exactly as the app does.
//! - `inspect` prints `serial=`, `issued_at=` and `expires_at=` lines (for
//!   `$GITHUB_OUTPUT`).
//! - `keygen` prints a new private seed on stdout (pipe it straight into
//!   `gh secret set`), and the public key's line for `COMPILED_KEYS` and
//!   its fingerprint on stderr.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::process::ExitCode;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use juggler_remote_config::envelope::Channel;
use juggler_remote_config::keys::{KeyRing, fingerprint};
use juggler_remote_config::publish::{
    self, DEFAULT_VALIDITY_DAYS, PublishError, Stamp, check_source, read_schema, read_source,
};
use juggler_remote_config::version::Version;

/// The environment variable that carries the private key's seed.
const KEY_VAR: &str = "REMOTE_CONFIG_SIGNING_KEY";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            let _ = std::io::stdout().write_all(out.as_bytes());
            ExitCode::SUCCESS
        }
        Err(message) => {
            let _ = writeln!(std::io::stderr(), "remote-config-publish: {message}");
            ExitCode::FAILURE
        }
    }
}

/// The command's flags, `--name value` each.
struct Flags(BTreeMap<String, String>);

impl Flags {
    fn parse(args: &[String], known: &[&str]) -> Result<Self, String> {
        let mut flags = BTreeMap::new();
        let mut rest = args.iter();
        while let Some(flag) = rest.next() {
            let name = flag
                .strip_prefix("--")
                .filter(|name| known.contains(name))
                .ok_or_else(|| format!("unknown argument {flag:?}"))?;
            let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
            if flags.insert(name.to_owned(), value.clone()).is_some() {
                return Err(format!("{flag} given twice"));
            }
        }
        Ok(Self(flags))
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(String::as_str)
    }

    fn need(&self, name: &str) -> Result<&str, String> {
        self.get(name)
            .ok_or_else(|| format!("--{name} is required"))
    }

    fn number(&self, name: &str) -> Result<Option<u64>, String> {
        self.get(name)
            .map(|raw| {
                raw.parse()
                    .map_err(|_| format!("--{name} takes a whole number"))
            })
            .transpose()
    }

    fn channel(&self) -> Result<Channel, String> {
        match self.need("channel")? {
            "nightly" => Ok(Channel::Nightly),
            "beta" => Ok(Channel::Beta),
            "stable" => Ok(Channel::Stable),
            other => Err(format!(
                "--channel {other:?} is not nightly, beta or stable"
            )),
        }
    }

    fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        let path = self.need(name)?;
        std::fs::read(path).map_err(|e| format!("{path}: {e}"))
    }
}

fn run(args: &[String]) -> Result<String, String> {
    let (command, rest) = args.split_first().ok_or("a command is required")?;
    match command.as_str() {
        "validate" => validate(rest),
        "next-serial" => next_serial(rest),
        "sign" => sign(rest),
        "verify" => verify(rest),
        "inspect" => inspect(rest),
        "keygen" => keygen(rest),
        other => Err(format!("unknown command {other:?}")),
    }
}

#[expect(clippy::needless_pass_by_value, reason = "used as map_err(error)")]
fn error(e: PublishError) -> String {
    e.to_string()
}

fn validate(rest: &[String]) -> Result<String, String> {
    let flags = Flags::parse(rest, &["schema", "source", "channel"])?;
    let registry = read_schema(&flags.read("schema")?).map_err(error)?;
    let source = read_source(&flags.read("source")?).map_err(error)?;
    check_source(&source, &registry, flags.channel()?).map_err(error)?;
    Ok("valid\n".to_owned())
}

fn next_serial(rest: &[String]) -> Result<String, String> {
    let flags = Flags::parse(rest, &["channel", "live"])?;
    let channel = flags.channel()?;
    let live = match flags.get("live") {
        None => None,
        Some(_) => Some(publish::inspect(&flags.read("live")?, &ring()?, channel).map_err(error)?),
    };
    Ok(format!(
        "{}\n",
        publish::next_serial(live.as_ref(), channel)
    ))
}

fn sign(rest: &[String]) -> Result<String, String> {
    let flags = Flags::parse(
        rest,
        &[
            "schema", "source", "channel", "serial", "now", "days", "live", "out",
        ],
    )?;
    let channel = flags.channel()?;
    let registry = read_schema(&flags.read("schema")?).map_err(error)?;
    let source = read_source(&flags.read("source")?).map_err(error)?;
    check_source(&source, &registry, channel).map_err(error)?;
    let stamp = Stamp::new(
        flags.number("serial")?.ok_or("--serial is required")?,
        flags.number("now")?.ok_or("--now is required")?,
        flags.number("days")?.unwrap_or(DEFAULT_VALIDITY_DAYS),
    )
    .map_err(error)?;
    let revocations = match flags.get("live") {
        None => Vec::new(),
        Some(_) => publish::revocations_of(&flags.read("live")?).map_err(error)?,
    };
    let secret = std::env::var(KEY_VAR).map_err(|_| format!("{KEY_VAR} is not set"))?;
    let key = publish::signing_key(&secret).map_err(error)?;
    drop(secret);
    let file = publish::sign(&source, &stamp, &key, &ring()?, &revocations).map_err(error)?;
    let out = flags.need("out")?;
    std::fs::write(out, &file).map_err(|e| format!("{out}: {e}"))?;
    Ok(format!("signed serial {} to {out}\n", stamp.serial))
}

fn verify(rest: &[String]) -> Result<String, String> {
    let flags = Flags::parse(rest, &["file", "channel", "now", "serial", "app"])?;
    let channel = flags.channel()?;
    let now = match flags.number("now")? {
        Some(now) => now,
        None => unix_now()?,
    };
    let app: Version = flags
        .get("app")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
        .parse()
        .map_err(|e| format!("--app: {e}"))?;
    let verified =
        publish::verify(&flags.read("file")?, &ring()?, channel, &app, now).map_err(error)?;
    if let Some(serial) = flags.number("serial")?
        && verified.envelope.serial != serial
    {
        return Err(format!(
            "the file carries serial {}, not {serial}",
            verified.envelope.serial
        ));
    }
    Ok(format!(
        "verified: {channel} serial {} by {}, expires_at {}\n",
        verified.envelope.serial, verified.envelope.key_id, verified.envelope.expires_at
    ))
}

fn inspect(rest: &[String]) -> Result<String, String> {
    let flags = Flags::parse(rest, &["file", "channel"])?;
    let verified =
        publish::inspect(&flags.read("file")?, &ring()?, flags.channel()?).map_err(error)?;
    Ok(format!(
        "serial={}\nissued_at={}\nexpires_at={}\n",
        verified.envelope.serial, verified.envelope.issued_at, verified.envelope.expires_at
    ))
}

fn keygen(rest: &[String]) -> Result<String, String> {
    let flags = Flags::parse(rest, &["id"])?;
    let id = flags.get("id").unwrap_or("rc-YYYY-a");
    let mut seed = [0u8; 32];
    getrandom::fill(&mut seed).map_err(|e| format!("random source: {e}"))?;
    let key = ed25519_dalek::SigningKey::from_bytes(&seed);
    let secret = BASE64.encode(seed);
    seed.fill(0);
    std::hint::black_box(&seed);
    let public = key.verifying_key().to_bytes();
    let bytes: Vec<String> = public.iter().map(|b| format!("0x{b:02x}")).collect();
    let _ = writeln!(
        std::io::stderr(),
        "public key fingerprint: {}\nits COMPILED_KEYS line (the role: Current, Next or Emergency):\n    (\"{id}\", KeyRole::Current, [{}]),",
        fingerprint(&public),
        bytes.join(", ")
    );
    Ok(format!("{secret}\n"))
}

fn ring() -> Result<KeyRing, String> {
    KeyRing::compiled().map_err(|e| format!("the compiled keys: {e}"))
}

fn unix_now() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|e| e.to_string())
}
