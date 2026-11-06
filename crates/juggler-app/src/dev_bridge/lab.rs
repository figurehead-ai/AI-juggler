//! The lab-only write mode (ADR-0027): the bridge also serves a short
//! allowlist of write commands, so a walk can make pools, start, switch
//! and stop sessions in a browser — only on a throwaway lab.
//!
//! Its conditions, each held here or by a test:
//!
//! 1. Built only with the `lab-bridge` feature, on top of `dev-bridge`:
//!    debug builds only, in no release workflow
//!    (`scripts/dev_bridge_check.sh`).
//! 2. It refuses to start, loudly, unless `$HOME` is a throwaway lab:
//!    spelled as it resolves (no symlink in it), a folder directly under
//!    `/tmp` named `aij-lab…`, this user's and closed to everyone else
//!    (0700), outside the home the user record names (and so outside the
//!    real AI Juggler data), with the app's data folder inside it
//!    ([`checked_home`]); and with no tool's folder override inherited
//!    ([`check_this`]). Then nothing of the bridge starts.
//! 3. Each write is one of the app's own commands, the function the
//!    window calls, with its typed arguments (`deny_unknown_fields`):
//!    no logic of its own. Same token, same loopback, same Origin.
//! 4. Real accounts stay out: before each write, every folder it would
//!    touch — the session's, its launch settings' Account folder, a
//!    launch's working folder — must resolve inside the lab, and an
//!    Account on a tool's own default folder is refused (its sign-in can
//!    live outside the folder, in the login Keychain) ([`guard`]). A
//!    launch with no folder starts in the lab's; a switch never goes to a
//!    desktop app, which runs outside the lab.
//! 5. (ADR-0027 condition 7.) One exception, named when the lab starts
//!    (2026-11-05, the Codex live steps): `AIJ_LAB_ADOPT=<tool>` lets the
//!    lab use that tool's own default folder in the real home where it
//!    is, so its real sign-in runs small tasks in the lab's folders. Only
//!    a tool that can use a folder in place (ADR-0018: checked, never
//!    written by AI Juggler); only that one folder, resolved from the user
//!    record, a real folder of this user's; only for an Account of that
//!    tool used in place ([`adopted_folder`]). Said loudly at start;
//!    refused if anything doesn't hold.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use tauri::Manager as _;

use super::routes::{ReadRoute, Reply};
use crate::state::AppState;

/// The app's own handle, given once at start: the commands that open
/// windows or start processes take it.
static APP: OnceLock<tauri::AppHandle<tauri::Wry>> = OnceLock::new();

/// Gives the bridge the running app's handle (lib.rs, at setup).
pub fn arm(app: &tauri::AppHandle<tauri::Wry>) {
    let _ = APP.set(app.clone());
}

/// Whether `home` is a throwaway lab this mode may write in, and its
/// resolved path. `record_home` is the user's real home (from the user
/// record, not `$HOME`), `data_dir` the app's data folder, `me` this
/// user's id.
///
/// # Errors
///
/// What makes it not a lab, in words.
pub fn checked_home(
    home: &Path,
    record_home: Option<&Path>,
    data_dir: &Path,
    me: u32,
) -> Result<PathBuf, String> {
    if !home.is_absolute() {
        return Err(format!("HOME {} isn't an absolute path", home.display()));
    }
    let resolved =
        std::fs::canonicalize(home).map_err(|e| format!("HOME {}: {e}", home.display()))?;
    // Spelled as it resolves (only macOS's /tmp → /private/tmp allowed): a
    // symlink anywhere in it, a trailing `/` or `/.`, is refused.
    let spelled = home.to_string_lossy();
    let shown = resolved.to_string_lossy();
    if shown != spelled && shown != format!("/private{spelled}") {
        return Err(format!(
            "HOME {} is a symlink or isn't spelled as it resolves ({shown})",
            home.display()
        ));
    }
    let in_tmp = matches!(
        resolved.parent().and_then(Path::to_str),
        Some("/tmp" | "/private/tmp")
    );
    let named = resolved
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("aij-lab"));
    if !in_tmp || !named {
        return Err(format!(
            "HOME {} isn't a lab folder (/tmp/aij-lab…)",
            resolved.display()
        ));
    }
    // A real folder, this user's, and closed to everyone else (0700).
    if !juggler_platform::file_trust::dir_is_ours(
        &resolved,
        juggler_platform::user::UserId::from_uid(me),
    ) {
        return Err(format!(
            "HOME {} isn't a folder only this user can open (it must be theirs, 0700)",
            resolved.display()
        ));
    }
    let real = record_home
        .ok_or("the real home can't be read from the user record")?
        .canonicalize()
        .map_err(|e| format!("the real home: {e}"))?;
    if resolved.starts_with(&real) || real.starts_with(&resolved) {
        return Err(format!(
            "HOME {} overlaps the real home",
            resolved.display()
        ));
    }
    let data = std::fs::canonicalize(data_dir)
        .map_err(|e| format!("the data folder {}: {e}", data_dir.display()))?;
    if !data.starts_with(&resolved) {
        return Err(format!(
            "the data folder {} isn't inside the lab",
            data.display()
        ));
    }
    Ok(resolved)
}

/// The lab, once the check passed: every write's folders must be in it.
static LAB_HOME: OnceLock<PathBuf> = OnceLock::new();

/// The variable naming the one tool whose real default folder the lab may
/// use where it is (ADR-0027 condition 7).
pub const ENV_ADOPT: &str = "AIJ_LAB_ADOPT";

/// Condition 7's tool and its real default folder, once checked.
static ADOPTED: OnceLock<(String, PathBuf)> = OnceLock::new();

/// The real home's counterpart of a tool's default folder in the lab:
/// `lab_default` (the adapter's default folder, under the lab's HOME)
/// moved from `lab` to `record_home`. It must exist as a folder of this
/// user's, inside the real home.
///
/// # Errors
///
/// What makes it not that tool's real default folder, in words.
pub fn adopted_folder(
    lab_default: &Path,
    lab: &Path,
    lab_spelled: &Path,
    record_home: &Path,
    me: u32,
) -> Result<PathBuf, String> {
    let inside_lab = lab_default
        .strip_prefix(lab_spelled)
        .or_else(|_| lab_default.strip_prefix(lab))
        .map_err(|_| format!("{} isn't under the lab's HOME", lab_default.display()))?;
    if inside_lab
        .components()
        .any(|part| !matches!(part, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "{} isn't a plain folder name",
            inside_lab.display()
        ));
    }
    let real_home = record_home
        .canonicalize()
        .map_err(|e| format!("the real home: {e}"))?;
    let real = real_home
        .join(inside_lab)
        .canonicalize()
        .map_err(|e| format!("the real {}: {e}", inside_lab.display()))?;
    if !real.starts_with(&real_home) || real == real_home {
        return Err(format!("{} isn't inside the real home", real.display()));
    }
    if !real.is_dir()
        || juggler_platform::file_trust::owner_of(&real)
            != Some(juggler_platform::user::UserId::from_uid(me))
    {
        return Err(format!("{} isn't a folder of this user's", real.display()));
    }
    Ok(real)
}

/// Whether `account` is condition 7's: of the adopted tool, used in place,
/// on its real default folder.
fn adopted(account: &juggler_core::model::Account) -> bool {
    ADOPTED.get().is_some_and(|(tool, real)| {
        account.harness.as_str() == tool
            && account.in_place
            && resolved(&account.state_dir).as_ref() == Some(real)
    })
}

/// The lab check for this process: `$HOME`, the user record, the app's
/// data folder, and no tool's folder override inherited (a shell inside
/// a tool's session carries one, pointing at a real folder).
///
/// # Errors
///
/// As [`checked_home`], or an isolation variable is set.
pub fn check_this(state: &AppState) -> Result<PathBuf, String> {
    for id in state.registry.ids() {
        if let Some(adapter) = state.registry.get(&id) {
            for key in adapter.isolation_env_keys() {
                if std::env::var_os(key).is_some() {
                    return Err(format!(
                        "{key} is set in this process (start the lab through scripts/qa/lab_env.sh)"
                    ));
                }
            }
        }
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or("HOME isn't set")?;
    let me = juggler_platform::user::current()
        .map_err(|e| e.to_string())?
        .uid();
    let lab = checked_home(
        &home,
        juggler_platform::user::record_home().as_deref(),
        &state.data_dir,
        me,
    )?;
    let _ = LAB_HOME.set(lab.clone());
    crate::lab_scope::set(&lab);
    if let Some(tool) = std::env::var_os(ENV_ADOPT) {
        let tool = tool.to_string_lossy().into_owned();
        let adapter = state
            .registry
            .get(&juggler_core::model::HarnessId::new(tool.as_str()))
            .ok_or_else(|| format!("{ENV_ADOPT}={tool}: no such tool"))?;
        // Only a tool whose default folder holds its own sign-in (Codex's
        // auth.json, not Claude Code's login Keychain), used where it is
        // and never written (its review: any in-place tool was accepted).
        if !adapter.can_use_in_place() || !adapter.default_folder_holds_sign_in() {
            return Err(format!(
                "{ENV_ADOPT}={tool}: only a tool whose own folder holds its sign-in, used where it is, may be named"
            ));
        }
        let default = adapter
            .vendor_default_dir()
            .ok_or_else(|| format!("{ENV_ADOPT}={tool}: it has no default folder"))?;
        let real = adopted_folder(
            &default,
            &lab,
            &home,
            juggler_platform::user::record_home()
                .as_deref()
                .ok_or("the real home can't be read from the user record")?,
            me,
        )
        .map_err(|why| format!("{ENV_ADOPT}={tool}: {why}"))?;
        crate::early_note(&format!(
            "LAB WRITE MODE also uses the real {tool} folder {} where it is ({ENV_ADOPT}): its sign-in runs in the lab's folders",
            real.display()
        ));
        crate::lab_scope::adopt(&real);
        // Anything indexed from it before the scope held is read again.
        state.conversations.forget_under(&real);
        let _ = ADOPTED.set((tool, real));
    }
    // The app's own background work (the limits probe, the watchers) runs
    // every Account it holds: in a lab, each must be the lab's own.
    let accounts = state
        .storage
        .lock()
        .map_err(|_| "the store is busy".to_owned())?
        .accounts()
        .list()
        .map_err(|error| error.to_string())?;
    for account in &accounts {
        account_in_lab(state, &account.id, true)
            .map_err(|why| format!("the lab holds an Account it can't own: {why}"))?;
        // The index keys files by the folder as stored: read again too.
        if adopted(account) {
            state.conversations.forget_under(&account.state_dir);
        }
    }
    Ok(lab)
}

/// `path` resolved as far as it exists (the rest appended); `None` with a
/// `..` in it, when it isn't absolute, or when nothing of it resolves.
fn resolved(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute()
        || path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return None;
    }
    let mut existing = path;
    let mut rest = Vec::new();
    loop {
        if let Ok(found) = std::fs::canonicalize(existing) {
            let mut out = found;
            out.extend(rest.iter().rev());
            return Some(out);
        }
        rest.push(existing.file_name()?);
        existing = existing.parent()?;
    }
}

/// Whether `path` lies inside the checked lab.
fn inside(path: &Path) -> bool {
    LAB_HOME
        .get()
        .is_some_and(|lab| resolved(path).is_some_and(|path| path.starts_with(lab)))
}

/// An Account a lab write may use: its folder (and where it came from)
/// inside the lab, and not its tool's own default folder.
/// `may_adopt`: condition 7's Account is admitted too (a launch, a stop, a
/// message; never a switch, which would write into its folder).
fn account_in_lab(
    state: &AppState,
    id: &juggler_core::model::AccountId,
    may_adopt: bool,
) -> Result<(), String> {
    let account = state
        .storage
        .lock()
        .map_err(|_| "the store is busy".to_owned())?
        .accounts()
        .get(id)
        .map_err(|_| "no such Account".to_owned())?;
    if adopted(&account) {
        return if may_adopt {
            Ok(())
        } else {
            Err(format!(
                "{} is the real folder the lab uses where it is: nothing is written there",
                account.label
            ))
        };
    }
    if !inside(&account.state_dir) {
        return Err(format!("{}'s folder is outside the lab", account.label));
    }
    if let Some(source) = &account.import_source_path
        && !inside(Path::new(source))
    {
        return Err(format!("{} came from outside the lab", account.label));
    }
    // A tool's own default folder signs in outside the folder (Claude
    // Code's login Keychain item, which a lab HOME doesn't move).
    let default = state
        .registry
        .get(&account.harness)
        .and_then(|adapter| adapter.vendor_default_dir())
        .and_then(|dir| resolved(&dir));
    if default.is_some() && default == resolved(&account.state_dir) {
        return Err(format!(
            "{} is its tool's default folder, whose sign-in isn't the lab's",
            account.label
        ));
    }
    Ok(())
}

/// Launch settings a lab write may use: their Account, and their own
/// folder if they name one.
fn profile_in_lab(state: &AppState, id: &str, may_adopt: bool) -> Result<(), String> {
    let profile = state
        .storage
        .lock()
        .map_err(|_| "the store is busy".to_owned())?
        .profiles()
        .get(&juggler_core::model::ProfileId::new(id))
        .map_err(|_| "no such launch settings".to_owned())?;
    if let Some(cwd) = &profile.default_cwd
        && !inside(cwd)
    {
        return Err("their folder is outside the lab".to_owned());
    }
    account_in_lab(state, &profile.account, may_adopt)
}

/// A session a lab write may act on: its folder and its launch settings.
fn session_in_lab(state: &AppState, id: &str, may_adopt: bool) -> Result<(), String> {
    let record = state
        .storage
        .lock()
        .map_err(|_| "the store is busy".to_owned())?
        .sessions()
        .get(&juggler_core::model::SessionId::new(id))
        .map_err(|_| "no such session".to_owned())?;
    if !inside(&record.cwd) {
        return Err("its folder is outside the lab".to_owned());
    }
    profile_in_lab(state, record.profile.as_str(), may_adopt)
}

/// Checks a write's arguments against the lab before it's made, and
/// starts a launch with no folder in the lab's. The other writes touch
/// only the store (pools, messages, a test notice); the ones naming a
/// session are held to the lab's sessions all the same.
///
/// # Errors
///
/// What the write would reach outside the lab.
fn guard(state: &AppState, ipc: &str, args: &mut serde_json::Value) -> Result<(), String> {
    let text = |args: &serde_json::Value, key: &str| -> Result<String, String> {
        args.get(key)
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("{key} is missing"))
    };
    match ipc {
        "launch_session" => {
            profile_in_lab(state, &text(args, "profileId")?, true)?;
            match args.get("cwd") {
                None | Some(serde_json::Value::Null) => {
                    let lab = LAB_HOME.get().ok_or("the lab isn't checked")?;
                    if let Some(object) = args.as_object_mut() {
                        object.insert(
                            "cwd".into(),
                            serde_json::Value::String(lab.to_string_lossy().into_owned()),
                        );
                    }
                }
                Some(serde_json::Value::String(cwd)) if inside(Path::new(cwd)) => {}
                Some(_) => return Err("the folder is outside the lab".to_owned()),
            }
            Ok(())
        }
        // The person's message into the pool or to one member: a member
        // named on its own must be the lab's.
        "messages_send" => match args
            .pointer("/input/to")
            .and_then(serde_json::Value::as_str)
        {
            // A message to the whole pool is delivered member by member
            // (Deliver), each held to the lab's sessions (the review: a
            // pool could hold others).
            Some("pool")
                if args.pointer("/input/deliver") == Some(&serde_json::Value::Bool(true)) =>
            {
                Err("a message to the whole pool is delivered member by member here".to_owned())
            }
            Some("pool") => Ok(()),
            Some(to) => session_in_lab(state, to, true),
            None => Err("input.to is missing".to_owned()),
        },
        "messages_deliver" => session_in_lab(state, &text(args, "recipient")?, true),
        "stop_session" | "session_resume" | "pools_move" | "pools_set_role" => {
            session_in_lab(state, &text(args, "sessionId")?, true)
        }
        "switch_execute" => {
            // Neither side may be condition 7's folder: a switch writes the
            // carried conversation into the target's folder (the review).
            session_in_lab(state, &text(args, "sessionId")?, false)?;
            profile_in_lab(state, &text(args, "targetProfileId")?, false)?;
            if args.get("destination").and_then(serde_json::Value::as_str) == Some("desktop") {
                return Err("a desktop app runs outside the lab".to_owned());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// A write the lab refuses: 403, saying what it would have reached.
fn outside(why: &str) -> Reply {
    let body = serde_json::json!({
        "code": "outside_the_lab",
        "retryable": false,
        "user_message": format!("Not in this lab: {why}."),
    });
    Reply {
        status: hyper::StatusCode::FORBIDDEN,
        body: body.to_string().into_bytes(),
    }
}

macro_rules! lab_arg {
    (app, $app:ident) => {
        $app.clone()
    };
    (state, $app:ident) => {
        $app.state::<Arc<AppState>>()
    };
}

macro_rules! lab_routes {
    ($(
        $ipc:literal => $($seg:ident)::+ ($($pre:ident),+ $(; $($arg:ident : $ty:ty),+)?);
    )*) => {
        /// The write commands a lab serves (the allowlist): each the
        /// window's own command, classed `Write` in the registry.
        pub const LAB_ROUTES: &[ReadRoute] = &[
            $(ReadRoute { ipc: $ipc, path: stringify!($($seg)::+) },)*
        ];

        /// Calls lab route `ipc`; `None` for any other name, or before
        /// the app gave its handle.
        pub(super) async fn dispatch(ipc: &str, mut args: serde_json::Value) -> Option<Reply> {
            let app = APP.get()?;
            find(ipc)?;
            if let Err(why) = guard(&app.state::<Arc<AppState>>(), ipc, &mut args) {
                return Some(outside(&why));
            }
            Some(match ipc {
                $($ipc => {
                    #[derive(serde::Deserialize)]
                    #[serde(rename_all = "camelCase", deny_unknown_fields)]
                    struct Args {
                        $($($arg: $ty),+)?
                    }
                    match serde_json::from_value::<Args>(args) {
                        Err(error) => Reply::bad_arguments(&error),
                        Ok(Args { $($($arg),+)? }) => match $($seg)::+($(lab_arg!($pre, app)),+ $(, $($arg),+)?).await {
                            Ok(value) => Reply::value(&value),
                            Err(error) => Reply::failed(&error),
                        },
                    }
                })*
                _ => return None,
            })
        }
    };
}

/// The lab route for an IPC name.
pub(super) fn find(ipc: &str) -> Option<&'static ReadRoute> {
    LAB_ROUTES.iter().find(|route| route.ipc == ipc)
}

use crate::{commands, features};

lab_routes! {
    "pools_create" => features::pools::commands::pools_create(state; name: String, project_id: Option<String>);
    "pools_rename" => features::pools::commands::pools_rename(state; pool_id: String, name: String);
    "pools_delete" => features::pools::commands::pools_delete(state; pool_id: String);
    "pools_move" => features::pools::commands::pools_move(state; session_id: String, pool_id: String);
    "pools_set_role" => features::pools::commands::pools_set_role(state; session_id: String, role: String, pool_id: Option<String>);
    "pools_set_access" => features::pools::commands::pools_set_access(state; pool_id: String, level: String, actions: Vec<String>);
    "pools_set_delivery" => features::pools::commands::pools_set_delivery(state; pool_id: String, hop_limit: u32, stopped: bool);
    "pools_revoke_rule" => features::pools::commands::pools_revoke_rule(state; rule_id: i64);
    "pools_join" => features::pools::commands::pools_join(state; pool_id: String, member: String, role: String, level: Option<String>, actions: Option<Vec<String>>);
    "pools_leave" => features::pools::commands::pools_leave(state; pool_id: String, member: String);
    "pools_set_scope" => features::pools::commands::pools_set_scope(state; pool_id: String, every: bool, projects: Vec<Option<String>>);
    "pools_set_sealed" => features::pools::commands::pools_set_sealed(state; pool_id: String, sealed: bool);
    "pools_set_skip_held" => features::pools::commands::pools_set_skip_held(state; pool_id: String, skip: bool);
    "pools_set_talk" => features::pools::commands::pools_set_talk(state; pool_id: String, from: String, to: String, rule: Option<String>);
    "pools_canvas_place" => features::pools::commands::pools_canvas_place(state; pool_id: String, nodes: Vec<crate::features::pools::NodePositionInfo>);
    "pools_canvas_tidy" => features::pools::commands::pools_canvas_tidy(state; pool_id: String);
    "launch_session" => commands::launch_session(app, state; profile_id: String, cwd: Option<String>, runtime: Option<String>);
    "stop_session" => commands::stop_session(app, state; session_id: String);
    "session_resume" => commands::session_resume(state; session_id: String);
    "switch_execute" => commands::switch_execute(state; session_id: String, target_profile_id: String, freeze_choice: String, redaction_choice: Option<String>, redaction_unredact: Option<Vec<u32>>, trigger: Option<String>, approved: Option<Vec<String>>, destination: Option<String>, cut: Option<crate::features::switch_cut::SwitchCutArgs>);
    "notifications_send_test" => features::notifications::commands::notifications_send_test(state);
    // Opening a thread marks it read: the window's own click (the P9 walk
    // couldn't open a moved 0.3 thread on the read-only bridge).
    "messages_thread" => features::messages::commands::messages_thread(state; thread_id: String, cursor: Option<String>);
    // The pool page's own question, "Let agents … work together?": a
    // lab setting in its store (the live steps: without it no agent gets
    // its tools).
    "agent_bridge_answer" => features::agent_bridge::commands::agent_bridge_answer(state; on: bool);
    "messages_answer" => features::messages::commands::messages_answer(state; ask_id: String, answer: String);
    // The person's own message into the pool (the live steps start an
    // agent's turn with it, delivered at their click).
    "messages_send" => features::messages::commands::messages_send(state; input: crate::features::messages::dto::ComposeInput);
    "messages_deliver" => features::messages::commands::messages_deliver(state; message_id: String, recipient: String);
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]
    use super::*;

    fn me() -> u32 {
        juggler_platform::user::current().unwrap().uid()
    }

    /// A fresh lab folder under /tmp, with a data folder inside, removed after.
    struct Lab(PathBuf);
    impl Lab {
        fn new(tag: &str) -> Self {
            let path = PathBuf::from(format!("/tmp/aij-lab-test-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(path.join("data")).unwrap();
            use std::os::unix::fs::PermissionsExt as _;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
            Self(path)
        }
    }
    impl Drop for Lab {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Condition 7 names only the tools a ruling allowed: the adapters that
    /// say their folder holds their sign-in are exactly these (a new one
    /// is a reviewed change, not a silent opt-in).
    #[test]
    fn only_the_ruled_tools_can_be_adopted() {
        let registry = crate::state::default_registry();
        let mut adoptable: Vec<String> = registry
            .ids()
            .into_iter()
            .filter(|id| {
                registry
                    .get(id)
                    .is_some_and(|adapter| adapter.default_folder_holds_sign_in())
            })
            .map(|id| id.to_string())
            .collect();
        adoptable.sort();
        assert_eq!(adoptable, ["codex"]);
    }

    /// Condition 7: a lab, and a stand-in for the real home with the
    /// tool's folder in it (both under /tmp, removed after).
    fn adopt_case(tag: &str) -> (Lab, Lab) {
        let lab = Lab::new(&format!("adopt-{tag}"));
        let real = Lab::new(&format!("realhome-{tag}"));
        std::fs::create_dir_all(real.0.join(".tool")).unwrap();
        (lab, real)
    }

    #[test]
    fn the_adopted_folder_is_the_tools_own_in_the_real_home_and_nothing_else() {
        let (lab, real) = adopt_case("ok");
        let resolved = lab.0.canonicalize().unwrap();
        let found = adopted_folder(&lab.0.join(".tool"), &resolved, &lab.0, &real.0, me()).unwrap();
        assert_eq!(found, real.0.join(".tool").canonicalize().unwrap());
        // Not there in the real home: refused, nothing made.
        let missing = adopted_folder(&lab.0.join(".other"), &resolved, &lab.0, &real.0, me());
        assert!(missing.unwrap_err().contains("the real .other"));
        assert!(!real.0.join(".other").exists());
    }

    #[test]
    fn an_adopted_folder_that_leaves_the_real_home_or_is_it_is_refused() {
        let (lab, real) = adopt_case("out");
        let resolved = lab.0.canonicalize().unwrap();
        // A link in the real home that points elsewhere.
        std::os::unix::fs::symlink("/tmp", real.0.join(".linked")).unwrap();
        let away = adopted_folder(&lab.0.join(".linked"), &resolved, &lab.0, &real.0, me());
        assert!(away.unwrap_err().contains("isn't inside the real home"));
        // The lab's HOME itself (no folder name): the real home, refused.
        let home = adopted_folder(&lab.0, &resolved, &lab.0, &real.0, me());
        assert!(home.unwrap_err().contains("isn't inside the real home"));
        // A default folder that isn't under the lab, or climbs out of it.
        let elsewhere = adopted_folder(Path::new("/etc"), &resolved, &lab.0, &real.0, me());
        assert!(
            elsewhere
                .unwrap_err()
                .contains("isn't under the lab's HOME")
        );
        let climbs = adopted_folder(&lab.0.join("../x"), &resolved, &lab.0, &real.0, me());
        assert!(climbs.unwrap_err().contains("isn't a plain folder name"));
    }

    #[test]
    fn an_adopted_folder_of_another_owner_is_refused() {
        let (lab, real) = adopt_case("owner");
        let resolved = lab.0.canonicalize().unwrap();
        let error = adopted_folder(&lab.0.join(".tool"), &resolved, &lab.0, &real.0, me() + 1);
        assert!(error.unwrap_err().contains("isn't a folder of this user's"));
    }

    #[test]
    fn a_fresh_lab_folder_under_tmp_is_a_lab() {
        let lab = Lab::new("ok");
        let real = juggler_platform::user::record_home().unwrap();
        let resolved = checked_home(&lab.0, Some(&real), &lab.0.join("data"), me()).unwrap();
        assert!(resolved.ends_with(lab.0.file_name().unwrap()));
    }

    #[test]
    fn the_real_home_is_refused() {
        let real = juggler_platform::user::record_home().unwrap();
        let error = checked_home(&real, Some(&real), &real, me()).unwrap_err();
        assert!(error.contains("isn't a lab folder"), "{error}");
    }

    #[test]
    fn a_symlinked_tmp_folder_is_refused_even_to_a_lab() {
        let lab = Lab::new("target");
        let link = PathBuf::from(format!("/tmp/aij-lab-test-link-{}", std::process::id()));
        let _ = std::fs::remove_file(&link);
        std::os::unix::fs::symlink(&lab.0, &link).unwrap();
        let real = juggler_platform::user::record_home().unwrap();
        let error = checked_home(&link, Some(&real), &lab.0.join("data"), me());
        let _ = std::fs::remove_file(&link);
        assert!(error.unwrap_err().contains("is a symlink"));
        // Nor through a spelling that resolves the link itself.
        let real = juggler_platform::user::record_home().unwrap();
        let dotted = PathBuf::from(format!("{}/.", lab.0.display()));
        assert!(
            checked_home(&dotted, Some(&real), &lab.0.join("data"), me())
                .unwrap_err()
                .contains("isn't spelled as it resolves")
        );
    }

    #[test]
    fn a_lab_others_can_write_in_is_refused() {
        use std::os::unix::fs::PermissionsExt as _;
        let lab = Lab::new("open");
        std::fs::set_permissions(&lab.0, std::fs::Permissions::from_mode(0o777)).unwrap();
        let real = juggler_platform::user::record_home().unwrap();
        let error = checked_home(&lab.0, Some(&real), &lab.0.join("data"), me()).unwrap_err();
        assert!(error.contains("only this user can open"), "{error}");
    }

    #[test]
    fn a_path_with_dot_dot_or_relative_never_resolves() {
        assert_eq!(resolved(Path::new("/tmp/aij-lab-x/../etc")), None);
        assert_eq!(resolved(Path::new("relative/folder")), None);
        assert_eq!(
            resolved(Path::new("/tmp/aij-lab-no-such-folder/a/b"))
                .map(|p| p.ends_with("aij-lab-no-such-folder/a/b")),
            Some(true)
        );
    }

    #[test]
    fn a_lab_pointing_into_the_real_home_or_a_data_folder_outside_it_is_refused() {
        let lab = Lab::new("data");
        let real = juggler_platform::user::record_home().unwrap();
        // The real home named as the lab's own: it overlaps.
        assert!(
            checked_home(&lab.0, Some(&lab.0), &lab.0.join("data"), me())
                .unwrap_err()
                .contains("overlaps the real home")
        );
        // A data folder outside the lab (the real one, say).
        assert!(
            checked_home(&lab.0, Some(&real), &real, me())
                .unwrap_err()
                .contains("isn't inside the lab")
        );
        // Another user's folder.
        assert!(
            checked_home(&lab.0, Some(&real), &lab.0.join("data"), me() + 1)
                .unwrap_err()
                .contains("only this user can open")
        );
        // No real home to compare with: refused, never assumed.
        assert!(checked_home(&lab.0, None, &lab.0.join("data"), me()).is_err());
    }

    #[test]
    fn every_lab_route_calls_a_command_the_registry_classes_write() {
        for route in LAB_ROUTES {
            let class = crate::COMMANDS
                .iter()
                .find(|(path, _)| *path == route.path)
                .map(|(_, access)| *access);
            assert_eq!(class, Some(crate::CommandAccess::Write), "{}", route.ipc);
            assert!(
                super::super::routes::find(route.ipc).is_none(),
                "{} is also a read route",
                route.ipc
            );
        }
    }
}
