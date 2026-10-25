//! Which runtime a launch gets (ADR-0022 decision 1): the user's choice for
//! the session, else for the profile, else the tool's own default — and a
//! Beta runtime only when the user picked it for that session, with
//! "Run without a terminal (Beta)" on in Settings › Sessions.

use std::collections::BTreeMap;

use juggler_adapter::runtime::{DrivenRuntime, Maturity, Runtime};
use juggler_core::storage::Storage;

/// The per-profile choices, a JSON map of profile id → `terminal`|`driven`.
const PROFILES: &str = "runtime.profiles";
/// The Beta switch: `on` (absent: off).
const BETA: &str = "runtime.beta";

/// What the user chose per profile.
#[must_use]
pub fn profile_choices(storage: &Storage) -> BTreeMap<String, Runtime> {
    let raw: BTreeMap<String, String> = storage
        .app_settings()
        .get(PROFILES)
        .ok()
        .flatten()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    raw.into_iter()
        .filter_map(|(profile, word)| Runtime::from_word(&word).map(|r| (profile, r)))
        .collect()
}

/// Sets (or, with `None`, clears) a profile's choice.
///
/// # Errors
///
/// Storage failures.
pub fn set_profile_choice(
    storage: &Storage,
    profile: &str,
    choice: Option<Runtime>,
) -> Result<(), juggler_core::CoreError> {
    let mut choices: BTreeMap<String, String> = profile_choices(storage)
        .into_iter()
        .map(|(id, runtime)| (id, runtime.as_str().to_owned()))
        .collect();
    match choice {
        Some(runtime) => choices.insert(profile.to_owned(), runtime.as_str().to_owned()),
        None => choices.remove(profile),
    };
    let text = serde_json::to_string(&choices).unwrap_or_else(|_| "{}".into());
    storage.app_settings().set(PROFILES, &text)
}

/// Whether the Beta switch is on.
#[must_use]
pub fn beta_on(storage: &Storage) -> bool {
    storage.app_settings().get(BETA).ok().flatten().as_deref() == Some("on")
}

/// Turns the Beta switch on or off.
///
/// # Errors
///
/// Storage failures.
pub fn set_beta(storage: &Storage, on: bool) -> Result<(), juggler_core::CoreError> {
    storage
        .app_settings()
        .set(BETA, if on { "on" } else { "off" })
}

/// The runtime a launch gets, and why not driven when it was asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Plan {
    /// The tool's terminal UI; the reason when driven was wanted.
    Terminal(Option<String>),
    /// The tool's protocol.
    Driven,
}

/// Pure: the plan for a launch, from the session's request, the profile's
/// choice, the tool's default, its maturity, the Beta switch, and why the
/// installed tool can't be driven (checked only when driven is wanted).
#[must_use]
pub fn plan(
    driven: Option<&dyn DrivenRuntime>,
    requested: Option<Runtime>,
    profile_choice: Option<Runtime>,
    beta_switch: bool,
) -> Plan {
    let Some(driven) = driven else {
        return Plan::Terminal(
            (requested == Some(Runtime::Driven))
                .then(|| "this tool runs in a terminal only".to_owned()),
        );
    };
    let beta = driven.maturity() == Maturity::Beta;
    let wanted = if beta {
        // A Beta is never a default or a profile's standing choice: only
        // this session's pick, with the switch on.
        requested.unwrap_or(Runtime::Terminal)
    } else {
        requested
            .or(profile_choice)
            .unwrap_or_else(|| driven.default_runtime())
    };
    if wanted == Runtime::Terminal {
        return Plan::Terminal(None);
    }
    if beta && !beta_switch {
        return Plan::Terminal(Some(
            "Run without a terminal (Beta) is off in Settings › Sessions".into(),
        ));
    }
    match driven.unavailable() {
        Some(why) => Plan::Terminal(Some(why)),
        None => Plan::Driven,
    }
}

#[cfg(test)]
mod tests {
    use super::{Plan, plan};
    use juggler_adapter::AdapterError;
    use juggler_adapter::runtime::{
        Connection, DrivenLaunch, DrivenRuntime, DrivenSession, DrivenSpec, Maturity, Runtime,
        TerminalView, ViewOf,
    };
    use juggler_core::model::{Account, Profile};
    use std::path::Path;

    struct Tool {
        default: Runtime,
        maturity: Maturity,
        unavailable: Option<&'static str>,
    }

    impl DrivenRuntime for Tool {
        fn label(&self) -> &'static str {
            "test"
        }
        fn default_runtime(&self) -> Runtime {
            self.default
        }
        fn maturity(&self) -> Maturity {
            self.maturity
        }
        fn unavailable(&self) -> Option<String> {
            self.unavailable.map(str::to_owned)
        }
        fn driven_spec(
            &self,
            _: &Account,
            _: &Profile,
            _: &Path,
            _: &DrivenLaunch,
        ) -> Result<DrivenSpec, AdapterError> {
            Err(AdapterError::Unsupported("test".into()))
        }
        fn connect(
            &self,
            _: Connection,
            _: &DrivenSpec,
            _: &Account,
        ) -> Result<Box<dyn DrivenSession>, AdapterError> {
            Err(AdapterError::Unsupported("test".into()))
        }
        fn terminal_view(
            &self,
            _: &Account,
            _: &Profile,
            _: &Path,
            _: &ViewOf,
        ) -> Result<TerminalView, AdapterError> {
            Err(AdapterError::Unsupported("test".into()))
        }
    }

    #[test]
    fn a_stable_default_drives_and_the_user_can_choose_the_terminal() {
        let codex = Tool {
            default: Runtime::Driven,
            maturity: Maturity::Stable,
            unavailable: None,
        };
        assert_eq!(plan(Some(&codex), None, None, false), Plan::Driven);
        assert_eq!(
            plan(Some(&codex), None, Some(Runtime::Terminal), false),
            Plan::Terminal(None),
            "the profile's Start in terminal"
        );
        assert_eq!(
            plan(
                Some(&codex),
                Some(Runtime::Terminal),
                Some(Runtime::Driven),
                false
            ),
            Plan::Terminal(None),
            "this session's Start in terminal"
        );
        let old = Tool {
            unavailable: Some("too old"),
            ..codex
        };
        assert_eq!(
            plan(Some(&old), None, None, false),
            Plan::Terminal(Some("too old".into()))
        );
    }

    #[test]
    fn a_beta_runs_only_when_picked_for_the_session_with_the_switch_on() {
        let claude = Tool {
            default: Runtime::Terminal,
            maturity: Maturity::Beta,
            unavailable: None,
        };
        assert_eq!(plan(Some(&claude), None, None, true), Plan::Terminal(None));
        assert_eq!(
            plan(Some(&claude), None, Some(Runtime::Driven), true),
            Plan::Terminal(None),
            "never a profile's standing choice"
        );
        assert!(matches!(
            plan(Some(&claude), Some(Runtime::Driven), None, false),
            Plan::Terminal(Some(_))
        ));
        assert_eq!(
            plan(Some(&claude), Some(Runtime::Driven), None, true),
            Plan::Driven
        );
        assert!(matches!(
            plan(None, Some(Runtime::Driven), None, true),
            Plan::Terminal(Some(_))
        ));
        assert_eq!(plan(None, None, None, true), Plan::Terminal(None));
    }
}
