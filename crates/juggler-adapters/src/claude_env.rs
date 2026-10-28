//! What a Claude Code launch removes from the environment it inherits:
//! keys that would sign every account in as one, a parent Claude Code
//! session's markers, and, under such a parent, where it routes its
//! requests. Re-exported from [`crate::claude_code`].

use juggler_adapter::EnvMap;

/// Credentials Claude Code takes from the environment ahead of the
/// folder's own sign-in. Started from a terminal that exports one, AI
/// Juggler would otherwise run EVERY account on that one key.
pub const INHERITED_CREDENTIAL_KEYS: [&str; 3] = [
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_AUTH_TOKEN",
    "CLAUDE_CODE_OAUTH_TOKEN",
];

/// What a running Claude Code session marks its own child processes with:
/// whose session they belong to, how to reach its host, whose account it
/// is. Started from a terminal inside a Claude Code session (AI Juggler Dev
/// run from one, or a harness), AI Juggler passed them on, and every
/// session it started took itself for that session's child: "Transcript
/// saving is off — inherited `CLAUDE_CODE_CHILD_SESSION` marker", so its
/// conversation was never written and couldn't be found or switched (the
/// lab of 2026-10-28). A new session is nobody's child.
pub const INHERITED_SESSION_MARKERS: [&str; 14] = [
    "CLAUDECODE",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_HOST_SESSION_ID",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_SDK_HAS_HOST_AUTH_REFRESH",
    "CLAUDE_CODE_ACCOUNT_UUID",
    "CLAUDE_CODE_ORGANIZATION_UUID",
    "CLAUDE_CODE_USER_EMAIL",
    "CLAUDE_CODE_OAUTH_SCOPES",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_PID",
];

/// Where a parent Claude Code session routes its requests. Started from a
/// terminal inside one, AI Juggler would inherit the host session's
/// gateway and every session it started would go that way. Removed only
/// when [`INHERITED_SESSION_MARKERS`] show such a parent: without one, it
/// is the user's own gateway exported in their shell, and it stays. An
/// account's own endpoint lives in its settings (`claude_endpoint`) and a
/// profile's in its Launch settings; neither is touched.
pub const PARENT_SESSION_ROUTING_KEYS: [&str; 1] = ["ANTHROPIC_BASE_URL"];

/// Whether an inherited environment carries a parent Claude Code
/// session's markers. `inherited` answers whether a variable is set; only
/// names are asked, never values.
pub fn has_parent_session(inherited: impl Fn(&str) -> bool) -> bool {
    INHERITED_SESSION_MARKERS.into_iter().any(inherited)
}

/// Everything a launch removes from what it inherits: the credentials
/// ([`INHERITED_CREDENTIAL_KEYS`]) and a parent session's markers
/// ([`INHERITED_SESSION_MARKERS`]); with a parent session, its routing too
/// ([`PARENT_SESSION_ROUTING_KEYS`]).
pub fn launch_unset_keys(parent_session: bool) -> impl Iterator<Item = &'static str> {
    INHERITED_CREDENTIAL_KEYS
        .into_iter()
        .chain(INHERITED_SESSION_MARKERS)
        .chain(
            PARENT_SESSION_ROUTING_KEYS
                .into_iter()
                .filter(move |_| parent_session),
        )
}

/// Each account signs in as itself, as a session of its own: remove
/// [`launch_unset_keys`] from what the launch inherits (an account set up
/// with a key keeps it in its own settings, which this doesn't touch).
/// Asks AI Juggler's own environment only whether a parent session's
/// markers are set (names, never values).
pub(crate) fn unset_inherited_credentials(env: &mut EnvMap) {
    unset_inherited_in(env, |key| std::env::var_os(key).is_some());
}

/// `unset_inherited_credentials` against an explicit inherited
/// environment (pure, testable: tests build their own maps). A routing key
/// the profile's Launch settings set stays as set.
pub fn unset_inherited_in(env: &mut EnvMap, inherited: impl Fn(&str) -> bool) {
    let parent = has_parent_session(inherited);
    for key in launch_unset_keys(parent) {
        let unset = juggler_adapter::launch::ENV_UNSET.to_owned();
        if PARENT_SESSION_ROUTING_KEYS.contains(&key) {
            env.entry(key.to_owned()).or_insert(unset);
        } else {
            env.insert(key.to_owned(), unset);
        }
    }
}
