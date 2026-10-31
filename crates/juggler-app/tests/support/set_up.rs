//! A test account as a real one is once signed in: Claude Code's first-run
//! setup done in its folder (`hasCompletedOnboarding`), which a switch now
//! checks before it changes anything (0.3.2). Other tools ignore the file.

use std::sync::Arc;

use juggler_app::commands::AccountInfo;
use juggler_app::state::AppState;
use juggler_core::model::AccountId;

/// `account`, set up in its folder; returned as it was.
pub fn set_up(state: &Arc<AppState>, account: AccountInfo) -> AccountInfo {
    let dir = state
        .storage
        .lock()
        .unwrap()
        .accounts()
        .get(&AccountId::new(account.id.as_str()))
        .unwrap()
        .state_dir;
    let file = dir.join(".claude.json");
    let mut config: serde_json::Value = std::fs::read_to_string(&file)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(|| serde_json::json!({}));
    config["hasCompletedOnboarding"] = serde_json::Value::Bool(true);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(&file, config.to_string()).unwrap();
    account
}
