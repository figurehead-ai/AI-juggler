//! What the remote configuration holds in effect now, in memory: built
//! after each pass, read by every section's reader with no I/O, and what
//! "About this configuration" shows.

use std::sync::{Arc, LazyLock, RwLock};

use juggler_remote_config::{
    Channel, Effective, Facts, Layer, Lists, Registry, Setting, UserChoices, Verified,
    VersionRange, merge,
};

use super::dto::{
    RemoteAboutInfo, RemoteFileInfo, RemoteLayerInfo, RemoteRecordSourceInfo, RemoteSection,
    RemoteValueSourceInfo, RemoteValuesInfo,
};

/// The verified file in use: what About shows of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFacts {
    /// Its serial.
    pub serial: u64,
    /// Its channel.
    pub channel: String,
    /// When it was signed, Unix seconds.
    pub issued_at: u64,
    /// When it stops being used, Unix seconds.
    pub expires_at: u64,
}

/// The configuration in effect. The default is the compiled one (no file).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    /// Every registered control's value and layer.
    pub effective: Effective,
    /// Every list section's records that matched this app.
    pub lists: Lists,
    /// The verified file in use, if any.
    pub file: Option<FileFacts>,
    /// When a fetch was last attempted, Unix seconds.
    pub checked_at: Option<u64>,
    /// "Check for configuration updates" is on.
    pub checking: bool,
    /// This build pins keys (it can verify a file).
    pub keys_pinned: bool,
    /// This app's channel, once known.
    pub channel: Option<Channel>,
}

impl Snapshot {
    /// The layers merged: the compiled defaults, then `file` (the public
    /// layer) resolved for `facts`. No file is the compiled defaults.
    #[must_use]
    pub fn build(registry: &Registry, file: Option<&Verified>, facts: &Facts) -> Self {
        let (public, lists) = file.map_or_else(
            || (None, Lists::new()),
            |file| {
                let sections = &file.envelope.sections;
                let (values, _) = registry.resolve(sections, facts);
                let (lists, _) = registry.resolve_lists(sections, facts);
                (Some(values), lists)
            },
        );
        Self {
            effective: merge(registry, public.as_ref(), None, &UserChoices::new()),
            lists,
            file: file.map(|file| FileFacts {
                serial: file.envelope.serial,
                channel: file.envelope.channel.clone(),
                issued_at: file.envelope.issued_at,
                expires_at: file.envelope.expires_at,
            }),
            ..Self::default()
        }
    }

    /// `section`'s values in effect.
    #[must_use]
    pub fn values(&self, section: RemoteSection) -> RemoteValuesInfo {
        match section {
            RemoteSection::Flags => RemoteValuesInfo::Flags {
                flags: crate::features::flags::states(&self.effective),
            },
        }
    }

    /// "About this configuration".
    #[must_use]
    pub fn about(&self) -> RemoteAboutInfo {
        let ms = |secs: u64| secs.saturating_mul(1000);
        RemoteAboutInfo {
            checking: self.checking,
            keys_pinned: self.keys_pinned,
            channel: self
                .channel
                .map(|c| c.as_str().to_owned())
                .unwrap_or_default(),
            verified: self.file.is_some(),
            file: self.file.as_ref().map(|file| RemoteFileInfo {
                serial: file.serial,
                channel: file.channel.clone(),
                issued_at_ms: ms(file.issued_at),
                expires_at_ms: ms(file.expires_at),
            }),
            last_checked_ms: self.checked_at.map(ms),
            values: self
                .effective
                .iter()
                .map(|(key, resolved)| RemoteValueSourceInfo {
                    section: key.section.clone(),
                    control: key.control.clone(),
                    value: words(&resolved.value),
                    layer: layer(resolved.layer),
                })
                .collect(),
            records: self
                .lists
                .iter()
                .flat_map(|(section, items)| {
                    items.iter().map(|item| RemoteRecordSourceInfo {
                        section: section.clone(),
                        id: item.id.clone(),
                        layer: RemoteLayerInfo::Public,
                    })
                })
                .collect(),
        }
    }
}

const fn layer(layer: Layer) -> RemoteLayerInfo {
    match layer {
        Layer::Default => RemoteLayerInfo::Default,
        Layer::Public => RemoteLayerInfo::Public,
        Layer::Account => RemoteLayerInfo::Account,
        Layer::User => RemoteLayerInfo::User,
    }
}

/// A value in words.
fn words(value: &Setting) -> String {
    match value {
        Setting::Switch(true) => "on".to_owned(),
        Setting::Switch(false) => "off".to_owned(),
        Setting::Versions(range) => range_words(range),
        Setting::Number(n) => n.to_string(),
        Setting::Choice(pick) => pick.clone(),
    }
}

fn range_words(range: &VersionRange) -> String {
    match (&range.min, &range.max) {
        (None, None) => "any version".to_owned(),
        (Some(min), None) => format!("from {min}"),
        (None, Some(max)) => format!("below {max}"),
        (Some(min), Some(max)) => format!("from {min}, below {max}"),
    }
}

static LIVE: RwLock<Option<Arc<Snapshot>>> = RwLock::new(None);

static COMPILED: LazyLock<Arc<Snapshot>> = LazyLock::new(Arc::default);

/// The configuration in effect: the compiled one until the service
/// [`install`]s another.
#[must_use]
pub fn current() -> Arc<Snapshot> {
    let live = match LIVE.read() {
        Ok(live) => live.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    };
    live.unwrap_or_else(|| Arc::clone(&COMPILED))
}

/// Puts `snapshot` in effect; whether anything changed.
pub fn install(snapshot: Snapshot) -> bool {
    let mut live = match LIVE.write() {
        Ok(live) => live,
        Err(poisoned) => poisoned.into_inner(),
    };
    if live.as_deref() == Some(&snapshot) {
        return false;
    }
    *live = Some(Arc::new(snapshot));
    true
}

/// Fixtures the sections' tests share.
#[cfg(test)]
pub(crate) mod fixtures {
    use std::collections::BTreeMap;

    use juggler_remote_config::{Channel, Envelope, Facts, InstallId, Os, Registry, Verified};

    /// The app's own registry, every section as registered.
    pub(crate) fn registry() -> Registry {
        super::super::registry(&crate::state::default_registry()).unwrap()
    }

    /// A verified stable file with `sections` (as if it passed every
    /// check: the sections are what's under test).
    pub(crate) fn file(sections: serde_json::Value) -> Verified {
        Verified {
            envelope: Envelope {
                format: 1,
                key_id: "rc-test".to_owned(),
                channel: "stable".to_owned(),
                serial: 418,
                issued_at: 1_800_000_000,
                expires_at: 1_801_209_600,
                min_app: None,
                max_app: None,
                sections: serde_json::from_value(sections).unwrap(),
            },
            digest: String::new(),
            key: String::new(),
        }
    }

    /// A stable 0.5 app on a Mac with `tool-a` 2.3.0, install `install`.
    pub(crate) fn facts(install: u8) -> Facts {
        Facts {
            app_version: "0.5.0".parse().unwrap(),
            channel: Channel::Stable,
            os: Os::Macos,
            os_version: None,
            tools: BTreeMap::from([("tool-a".to_owned(), "2.3.0".parse().unwrap())]),
            install: InstallId::from_bytes([install; 16]),
        }
    }
}
