//! What every tool's adapter module names: the adapter traits and the
//! types their methods take and give (`use crate::common::prelude::*;`).

pub(crate) use std::path::{Path, PathBuf};

pub(crate) use juggler_adapter::environment::{DiscoveredInstall, ImportOutcome, LoginProbe};
pub(crate) use juggler_adapter::prelude::*;
pub(crate) use juggler_adapter::{
    AdapterError, Capabilities, DiscoveryReport, EnvMap, InstallStatus, LaunchMode, LaunchSpec,
    SessionWatcher, VerificationStatus,
};
pub(crate) use juggler_core::model::{Account, HarnessId, Profile};

pub(crate) use crate::claude_code::{LoginState, home_dir, prepare_managed_dir_for};
