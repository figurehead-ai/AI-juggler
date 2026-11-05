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
        account_in_lab(state, &account.id)
            .map_err(|why| format!("the lab holds an Account it can't own: {why}"))?;
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
fn account_in_lab(state: &AppState, id: &juggler_core::model::AccountId) -> Result<(), String> {
    let account = state
        .storage
        .lock()
        .map_err(|_| "the store is busy".to_owned())?
        .accounts()
        .get(id)
        .map_err(|_| "no such Account".to_owned())?;
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
fn profile_in_lab(state: &AppState, id: &str) -> Result<(), String> {
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
    account_in_lab(state, &profile.account)
}

/// A session a lab write may act on: its folder and its launch settings.
fn session_in_lab(state: &AppState, id: &str) -> Result<(), String> {
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
    profile_in_lab(state, record.profile.as_str())
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
            profile_in_lab(state, &text(args, "profileId")?)?;
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
        "stop_session" | "session_resume" | "pools_move" | "pools_set_role" => {
            session_in_lab(state, &text(args, "sessionId")?)
        }
        "switch_execute" => {
            session_in_lab(state, &text(args, "sessionId")?)?;
            profile_in_lab(state, &text(args, "targetProfileId")?)?;
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
    "messages_answer" => features::messages::commands::messages_answer(state; ask_id: String, answer: String);
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
