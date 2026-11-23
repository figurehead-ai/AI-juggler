//! AJ-E1547 (PAT-OSP): the one gate every system prompt AI Juggler can
//! cause goes through. The maintainer's rule: AI Juggler never raises a
//! system prompt the user wasn't told about first ("users will think it's
//! a bug or their Mac is crashing").
//!
//! - **What a prompt is** ([`Kind`]): the system's own dialog, with its
//!   words ([`dialog`]) and where its answer can be changed later
//!   ([`settings_place`]). The registry of every prompt the app can cause
//!   (its ids, why, what triggers each) is the app's
//!   (`juggler-app/src/os_prompts`), written out in
//!   `docs/design/os-prompts.md`.
//! - **The gate** ([`Gate::through`]): an entry not primed yet gets the
//!   user's word first ([`Primer::ask`]: our sheet, "the system will now
//!   ask… click X", Continue or Not now); the call is made only after
//!   Continue; then the outcome is read back and said ([`Outcome`]).
//!   Background work ([`Presence::Background`]) never prompts: an entry
//!   not primed (and not already allowed) waits ([`Gated::Waiting`]).
//! - **The calls** that raise a prompt live here and nowhere else
//!   (`scripts/os_prompt_lint.py`): the Accessibility ask, `AppleScript`
//!   and Apple events to another app, Screen Recording, the login item,
//!   an app's first open. Each comes in its gated form only.
//! - **Never** a privacy reset: no `tccutil`, no write to the system's
//!   privacy database. Those are the user's security settings.
//!
//! Per platform: macOS has every kind today. Windows (UAC's elevation,
//! `SmartScreen`'s "Windows protected your PC") and Linux (portals, polkit)
//! get their kinds and words when those ports start (PAT-OSP-6, D-52);
//! until then [`dialog`] has none there and every probe says unknown.

use std::sync::OnceLock;

#[cfg(target_os = "macos")]
mod mac;
mod words;

pub use words::{dialog, settings_place};

/// An app another app's Apple events go to, as the system names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Target {
    /// Its bundle identifier; empty when the app is only known at the
    /// moment of the call (a link sent to whichever app keeps a session).
    pub bundle_id: &'static str,
    /// Its name as the system's dialog shows it.
    pub name: &'static str,
}

/// A protected folder in the user's home (the system asks once per app).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Folder {
    /// `~/Documents`.
    Documents,
    /// `~/Desktop`.
    Desktop,
    /// `~/Downloads`.
    Downloads,
}

impl Folder {
    /// Every protected folder.
    pub const ALL: [Self; 3] = [Self::Documents, Self::Desktop, Self::Downloads];

    /// Its name in the home folder (and in the system's dialog).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Documents => "Documents",
            Self::Desktop => "Desktop",
            Self::Downloads => "Downloads",
        }
    }
}

/// The kind of system prompt: what the system shows, whatever triggered it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Control this computer with accessibility features.
    Accessibility,
    /// Send Apple events to (script) another app, asked once per app.
    Automation(Target),
    /// Send notifications.
    Notifications,
    /// Open at login: the system's "login item added" notice.
    LoginItem,
    /// Record the screen.
    ScreenRecording,
    /// Find and connect to devices on the local network.
    LocalNetwork,
    /// Read files in a protected folder.
    Folder(Folder),
    /// Read another app's data (its container).
    AppData,
    /// A helper whose signer differs from the one opened before ("differs
    /// from previously opened versions").
    Differs,
    /// Use a keychain item another signature created.
    Keychain,
    /// Open an app downloaded from the internet the first time.
    FirstOpen,
}

/// What the system says of a prompt now, without asking.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    /// Allowed.
    Allowed,
    /// Asked, and the answer was no (or it's off in the settings).
    NotAllowed,
    /// The system hasn't asked yet.
    NotAsked,
    /// Asked; the system won't say what was answered.
    Asked,
}

impl Status {
    /// Its word on the wire.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Allowed => "allowed",
            Self::NotAllowed => "not_allowed",
            Self::NotAsked => "not_asked",
            Self::Asked => "asked",
        }
    }
}

/// How the system shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// A dialog that waits for an answer.
    Alert,
    /// A notice in the corner of the screen (a notification).
    Banner,
}

/// The system's dialog, in its own words, so our sheet can draw a picture
/// of it and say which button to click.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dialog {
    /// An alert or a banner.
    pub style: Style,
    /// Its title line.
    pub title: String,
    /// Its text under the title.
    pub body: String,
    /// Its buttons, left to right.
    pub buttons: Vec<&'static str>,
    /// The button that says yes (one of `buttons`; empty for a notice
    /// that asks nothing).
    pub click: &'static str,
}

/// One entry the gate guards: the registry's id and the kind of prompt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Prompt {
    /// The registry's short, stable id (the walks' dialog log names it).
    pub id: &'static str,
    /// What the system shows.
    pub kind: Kind,
}

/// What came of a call through the gate, said plainly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outcome {
    /// What the system says now.
    pub status: Status,
    /// The words for it: granted, or declined with how to change it later.
    pub words: String,
}

impl Outcome {
    /// The outcome for `kind` when the system now says `status` (`None`:
    /// it won't say).
    #[must_use]
    pub fn of(kind: Kind, status: Option<Status>) -> Self {
        let place = settings_place(kind);
        let later = |lead: &str| match place {
            Some(place) => format!("{lead} You can change it later in {place}."),
            None => lead.to_owned(),
        };
        match status {
            Some(Status::Allowed) => Self {
                status: Status::Allowed,
                words: "Allowed.".to_owned(),
            },
            Some(Status::NotAllowed) => Self {
                status: Status::NotAllowed,
                words: later("Not allowed. Nothing else changes."),
            },
            Some(Status::NotAsked) => Self {
                status: Status::NotAsked,
                words: "The system will ask the first time it's needed.".to_owned(),
            },
            Some(Status::Asked) | None => Self {
                status: Status::Asked,
                words: later("Asked."),
            },
        }
    }
}

/// The user's word on our sheet.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    /// Go ahead: the system may ask now.
    Continue,
    /// Not now: nothing is asked.
    NotNow,
}

/// Whether each entry has been primed (the user was told and said go),
/// kept per entry by the app.
pub trait Ledger: Send + Sync {
    /// Whether `id` has been primed.
    fn primed(&self, id: &str) -> bool;
    /// Records that `id` has been primed.
    fn set_primed(&self, id: &str);
}

/// Asks the user first: the app's sheet ("the system will now ask… here's
/// why, click X", Continue and Not now), and says the outcome after.
pub trait Primer: Send + Sync {
    /// Shows the sheet for `prompt` and waits for the user's answer.
    fn ask(&self, prompt: Prompt) -> Answer;
    /// Says what came of the call the user said go to.
    fn told(&self, prompt: Prompt, outcome: &Outcome);
}

/// Who is there when the call would be made.
#[derive(Clone, Copy)]
pub enum Presence<'a> {
    /// The user, at a click: an entry not primed gets the sheet first.
    User(&'a dyn Primer),
    /// The user, at a click on a priming card that already showed the
    /// system's dialog and what to click (setup's step): primes, then calls.
    Primed,
    /// Nobody (a tick, keep-alive, a resume at launch): never prompts.
    Background,
}

/// What the gate did.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Gated<T> {
    /// The call was made.
    Called {
        /// What it returned.
        value: T,
        /// What the system says now.
        outcome: Outcome,
    },
    /// The user said Not now: nothing was called.
    NotNow,
    /// Nobody was there and the entry isn't primed: nothing was called.
    Waiting,
}

impl<T> Gated<T> {
    /// The call's value, when it was made.
    pub fn value(self) -> Option<T> {
        match self {
            Self::Called { value, .. } => Some(value),
            Self::NotNow | Self::Waiting => None,
        }
    }
}

/// The gate over one [`Ledger`].
#[derive(Clone, Copy)]
pub struct Gate<'a> {
    ledger: &'a dyn Ledger,
}

impl<'a> Gate<'a> {
    /// The gate over `ledger`.
    #[must_use]
    pub const fn new(ledger: &'a dyn Ledger) -> Self {
        Self { ledger }
    }

    /// Whether `id` has been primed.
    #[must_use]
    pub fn primed(&self, id: &str) -> bool {
        self.ledger.primed(id)
    }

    /// Makes `call` for `prompt` only when the user was told first: it is
    /// primed already, the system already decided (`status` says allowed
    /// or not allowed: no dialog will follow, so no sheet either, PAT-USE
    /// (6)), or the user is there and says Continue on the sheet (then it
    /// is primed). Not now primes nothing: the next click asks again, so
    /// it is never a decline for good. Background work never prompts.
    /// After the call, `status` is read again and the outcome said.
    pub fn through<T>(
        &self,
        prompt: Prompt,
        presence: Presence<'_>,
        status: &dyn Fn() -> Option<Status>,
        call: impl FnOnce() -> T,
    ) -> Gated<T> {
        let mut asked = false;
        let decided = matches!(status(), Some(Status::Allowed | Status::NotAllowed));
        if !decided && !self.ledger.primed(prompt.id) {
            match presence {
                Presence::Background => return Gated::Waiting,
                Presence::User(primer) => {
                    if primer.ask(prompt) == Answer::NotNow {
                        return Gated::NotNow;
                    }
                    asked = true;
                }
                Presence::Primed => {}
            }
            self.ledger.set_primed(prompt.id);
        }
        let value = call();
        let outcome = Outcome::of(prompt.kind, status());
        if asked && let Presence::User(primer) = presence {
            primer.told(prompt, &outcome);
        }
        Gated::Called { value, outcome }
    }
}

/// Nothing primed, and nothing kept: what the gate uses before the app
/// installed its own ledger (every prompt waits).
struct NoLedger;

impl Ledger for NoLedger {
    fn primed(&self, _id: &str) -> bool {
        false
    }

    fn set_primed(&self, _id: &str) {}
}

/// Says Not now to everything: before the app installed its own primer.
struct NoPrimer;

impl Primer for NoPrimer {
    fn ask(&self, _prompt: Prompt) -> Answer {
        Answer::NotNow
    }

    fn told(&self, _prompt: Prompt, _outcome: &Outcome) {}
}

struct Installed {
    ledger: Box<dyn Ledger>,
    primer: Box<dyn Primer>,
}

static INSTALLED: OnceLock<Installed> = OnceLock::new();

/// Installs the app's ledger and primer, once, at start. Before (and in a
/// run that never installs one) nothing is primed and every prompt waits.
pub fn install(ledger: Box<dyn Ledger>, primer: Box<dyn Primer>) {
    let _ = INSTALLED.set(Installed { ledger, primer });
}

/// The process's gate (the installed ledger, or none).
#[must_use]
pub fn gate() -> Gate<'static> {
    static NONE: NoLedger = NoLedger;
    Gate::new(INSTALLED.get().map_or(&NONE as &dyn Ledger, |i| &*i.ledger))
}

/// The installed primer (one that says Not now before install).
#[must_use]
pub fn primer() -> &'static dyn Primer {
    static NONE: NoPrimer = NoPrimer;
    INSTALLED.get().map_or(&NONE as &dyn Primer, |i| &*i.primer)
}

/// The user, at a click, with the installed primer.
#[must_use]
pub fn user() -> Presence<'static> {
    Presence::User(primer())
}

// --- What the system says, never asking --------------------------------

/// Whether the system lets AI Juggler use accessibility features (never
/// asks).
#[must_use]
pub fn accessibility_status() -> Status {
    #[cfg(target_os = "macos")]
    {
        if mac::accessibility_trusted() {
            Status::Allowed
        } else {
            Status::NotAsked
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Status::NotAsked
    }
}

/// Whether AI Juggler may send Apple events to the app `bundle_id`, read
/// with an event that never asks (`None`: the app isn't running, or it
/// can't be told).
#[must_use]
pub fn automation_status(bundle_id: &str) -> Option<Status> {
    #[cfg(target_os = "macos")]
    {
        if bundle_id.is_empty() {
            return None;
        }
        mac::automation_status(bundle_id)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bundle_id;
        None
    }
}

/// Whether the system lets AI Juggler record the screen (never asks).
#[must_use]
pub fn screen_recording_status() -> Status {
    #[cfg(target_os = "macos")]
    {
        if mac::screen_recording_allowed() {
            Status::Allowed
        } else {
            Status::NotAsked
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Status::NotAsked
    }
}

/// Whether AI Juggler opens at login, as the system keeps it (never asks).
#[must_use]
pub fn login_item_status() -> juggler_login_item::LoginItem {
    juggler_login_item::status()
}

/// The protected folder `path` lies in, under `home` (`None`: none).
#[must_use]
pub fn protected_folder(path: &std::path::Path, home: &std::path::Path) -> Option<Folder> {
    if !cfg!(target_os = "macos") {
        return None;
    }
    Folder::ALL
        .into_iter()
        .find(|folder| path.starts_with(home.join(folder.name())))
}

/// Whether `path` is another app's data under `home` (its container),
/// which the system guards ("would like to access data from other apps").
#[must_use]
pub fn other_apps_data(path: &std::path::Path, home: &std::path::Path) -> bool {
    cfg!(target_os = "macos")
        && (path.starts_with(home.join("Library/Containers"))
            || path.starts_with(home.join("Library/Group Containers")))
}

/// Whether reading `path` may happen now without a prompt nobody was
/// told about: it isn't in a protected folder or another app's data, or
/// its entry (`folder_id` names the registry's id for each folder) has
/// been primed. For background readers (scans, ticks).
#[must_use]
pub fn may_read(
    path: &std::path::Path,
    home: &std::path::Path,
    folder_id: &dyn Fn(Folder) -> &'static str,
    app_data_id: &str,
) -> bool {
    if other_apps_data(path, home) {
        return gate().primed(app_data_id);
    }
    protected_folder(path, home).is_none_or(|folder| gate().primed(folder_id(folder)))
}

// --- The calls that ask: gated only -------------------------------------

/// Asks for Accessibility (the system's dialog that adds AI Juggler to
/// the list), through the gate.
#[must_use]
pub fn ask_accessibility(prompt: Prompt, presence: Presence<'_>) -> Gated<bool> {
    gate().through(prompt, presence, &|| Some(accessibility_status()), || {
        #[cfg(target_os = "macos")]
        {
            mac::accessibility_ask()
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    })
}

/// Asks for Screen Recording, through the gate.
#[must_use]
pub fn ask_screen_recording(prompt: Prompt, presence: Presence<'_>) -> Gated<bool> {
    gate().through(
        prompt,
        presence,
        &|| Some(screen_recording_status()),
        || {
            #[cfg(target_os = "macos")]
            {
                mac::screen_recording_ask()
            }
            #[cfg(not(target_os = "macos"))]
            {
                false
            }
        },
    )
}

/// Runs `source` (`AppleScript`) with `args` through the gate: the first
/// script to an app makes the system ask whether AI Juggler may control
/// it. The script's output, or its error words.
#[must_use]
pub fn script(
    prompt: Prompt,
    presence: Presence<'_>,
    source: &str,
    args: &[&str],
) -> Gated<Result<String, String>> {
    let status = || match prompt.kind {
        Kind::Automation(target) => automation_status(target.bundle_id),
        _ => None,
    };
    gate().through(prompt, presence, &status, || run_script(source, args))
}

fn run_script(source: &str, args: &[&str]) -> Result<String, String> {
    #[cfg(target_os = "macos")]
    {
        mac::osascript(source, args)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, args);
        Err(crate::unsupported("scripting another app").to_string())
    }
}

/// Sends `link` to process `pid` through the gate: the first link to an
/// app makes the system ask whether AI Juggler may send it events. Once
/// primed, a click may let the system ask; background work only ever
/// sends quietly ([`crate::apps::send_link_quietly`]).
///
/// # Errors
///
/// As [`crate::apps::send_link`].
#[must_use]
pub fn send_link(
    prompt: Prompt,
    presence: Presence<'_>,
    pid: u32,
    link: &str,
) -> Gated<std::io::Result<()>> {
    let status = || match prompt.kind {
        Kind::Automation(target) => automation_status(target.bundle_id),
        _ => None,
    };
    gate().through(prompt, presence, &status, || {
        crate::apps::send_link(pid, link)
    })
}

/// Turns opening at login on through the gate (the system shows its
/// "login item added" notice).
///
/// # Errors
///
/// The system's own words when it refuses.
#[must_use]
pub fn login_item_on(
    prompt: Prompt,
    presence: Presence<'_>,
) -> Gated<Result<juggler_login_item::LoginItem, String>> {
    let status = || match juggler_login_item::status() {
        juggler_login_item::LoginItem::On => Some(Status::Allowed),
        juggler_login_item::LoginItem::NeedsApproval => Some(Status::NotAllowed),
        juggler_login_item::LoginItem::Off | juggler_login_item::LoginItem::Unavailable => None,
    };
    gate().through(prompt, presence, &status, || juggler_login_item::set(true))
}

/// Turns opening at login off: the system asks nothing.
///
/// # Errors
///
/// The system's own words when it refuses.
pub fn login_item_off() -> Result<juggler_login_item::LoginItem, String> {
    juggler_login_item::set(false)
}

/// Why the app at `bundle` mustn't be started for the user (`Some`): it
/// was downloaded from the internet and never opened, so the system would
/// ask "Are you sure you want to open it?". The user opens it once
/// themselves; AI Juggler never makes that dialog appear.
#[must_use]
pub fn first_open_refusal(bundle: &std::path::Path) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let flags = mac::quarantine_flags(bundle)?;
        first_open_pending(flags).then(|| {
            let name = bundle
                .file_stem()
                .map_or_else(|| "it".to_owned(), |n| n.to_string_lossy().into_owned());
            format!(
                "Open {name} yourself once first: it came from the internet, and the system checks such an app the first time it opens."
            )
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = bundle;
        None
    }
}

/// Whether a download's quarantine `flags` say it was never approved to
/// open (the system's "user approved" flag, `0x40`, isn't set).
#[must_use]
pub const fn first_open_pending(flags: u32) -> bool {
    flags & 0x40 == 0
}

#[cfg(test)]
mod tests;
