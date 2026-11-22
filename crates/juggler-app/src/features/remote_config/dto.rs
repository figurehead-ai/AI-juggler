//! The remote configuration's wire types (ADR-0035 §9, AJ-E2023):
//! `useRemote(section)`'s values, and "About this configuration".

use std::collections::BTreeMap;

/// A section the window reads values from (`useRemote(section)`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum RemoteSection {
    /// The feature flags.
    Flags,
}

/// A section's values in effect, typed per section.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(tag = "section", rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum RemoteValuesInfo {
    /// The feature flags.
    Flags {
        /// Every declared flag, on or off, by name.
        flags: BTreeMap<String, bool>,
    },
}

/// Which layer set a value (ADR-0035 §4).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub enum RemoteLayerInfo {
    /// Built into this version of AI Juggler.
    Default,
    /// The public configuration file for this channel.
    Public,
    /// The linked account's file.
    Account,
    /// The user's own setting.
    User,
}

/// One value in effect and the layer that set it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteValueSourceInfo {
    /// Its section.
    pub section: String,
    /// Its name.
    pub control: String,
    /// The value, in words ("on", "off", "from 4.0.0", "3").
    pub value: String,
    /// The layer that set it.
    pub layer: RemoteLayerInfo,
}

/// One record (a notice) in effect: always from the file.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteRecordSourceInfo {
    /// Its section.
    pub section: String,
    /// Its id.
    pub id: String,
    /// The layer it came from.
    pub layer: RemoteLayerInfo,
}

/// The verified file in use.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteFileInfo {
    /// Its serial (strictly increasing per channel).
    #[ts(type = "number")]
    pub serial: u64,
    /// The channel it is for.
    pub channel: String,
    /// When it was signed, ms epoch.
    #[ts(type = "number")]
    pub issued_at_ms: u64,
    /// When it stops being used, ms epoch.
    #[ts(type = "number")]
    pub expires_at_ms: u64,
}

/// Settings › Advanced "About this configuration".
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, ts_rs::TS)]
#[ts(export, export_to = "../../../ui/src/types/generated/")]
pub struct RemoteAboutInfo {
    /// "Check for configuration updates" is on.
    pub checking: bool,
    /// This build can verify a file (its keys are pinned); without, it
    /// fetches nothing and runs on its built-in values.
    pub keys_pinned: bool,
    /// This app's channel.
    pub channel: String,
    /// A verified file is in use.
    pub verified: bool,
    /// That file, if any.
    pub file: Option<RemoteFileInfo>,
    /// When the file was last checked for, ms epoch, if ever.
    #[ts(type = "number | null")]
    pub last_checked_ms: Option<u64>,
    /// Every value in effect and where it came from.
    pub values: Vec<RemoteValueSourceInfo>,
    /// Every record in effect (before the user's dismissals).
    pub records: Vec<RemoteRecordSourceInfo>,
}
