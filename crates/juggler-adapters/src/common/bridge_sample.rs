//! The agent bridge launch the adapters' unit tests share.

use std::path::{Path, PathBuf};

use juggler_adapter::agent_bridge::BridgeLaunch;

/// Three of AI Juggler's tools, the key and socket variables, the app in
/// `/Applications`, and `state_dir` as the Account's folder.
pub(crate) fn sample(state_dir: &Path) -> BridgeLaunch {
    BridgeLaunch {
        program: PathBuf::from("/Applications/AI Juggler.app/Contents/MacOS/juggler-agent"),
        server: "juggler".into(),
        tools: vec!["whoami".into(), "pool_roster".into(), "set_status".into()],
        env_names: vec!["JUGGLER_AGENT_TOKEN".into(), "JUGGLER_AGENT_SOCKET".into()],
        dir: PathBuf::from("/data/overlays/agent/sess-1"),
        state_dir: state_dir.to_path_buf(),
    }
}
