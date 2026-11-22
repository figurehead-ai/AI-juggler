//! The layer merge (§4): one pure function.
//!
//! From lowest to highest: the compiled defaults, the public channel file,
//! the account file, the user's own settings. A higher layer overrides a
//! lower one, by each control kind's rule:
//!
//! | Control  | Public, account                  | User                                         |
//! |----------|----------------------------------|----------------------------------------------|
//! | Switch   | replaces (kill switches: `false`) | replaces, **except** a remote kill of an `OffOnly` switch |
//! | Versions | replaces, within compiled bounds | may only narrow (intersects); remote narrowing stands |
//! | Number   | replaces, clamped                | replaces, clamped                            |
//! | Choice   | replaces                         | replaces                                     |
//!
//! A user's explicit choice wins over a remote *preference*, never over
//! remote *safety*: a kill switch and compat narrowing beat it.
//!
//! **Only an `OffOnly` switch set off is a kill.** An `OnOff` or `Rollout`
//! switch set off is a preference: the user and the account layer can
//! override it. So an emergency "kill" must target an `OffOnly` switch
//! (the publish workflow's `--kill` should refuse anything else). The
//! account layer may also widen a public compat narrowing back, within the
//! compiled bounds; whether it should is decided with the account layer
//! (ADR-0035 step 4).

use std::collections::BTreeMap;

use crate::schema::{Control, LayerValues, Registry, RemoteMode, Setting, SettingKey};

/// Which layer set a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Layer {
    /// The compiled default.
    Default,
    /// The public channel file.
    Public,
    /// The account file.
    Account,
    /// The user's own setting.
    User,
}

/// A value and the layer that set it ("About this configuration").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The value in effect.
    pub value: Setting,
    /// The layer it came from.
    pub layer: Layer,
}

/// The user's own settings, by control.
pub type UserChoices = BTreeMap<SettingKey, Setting>;

/// The configuration in effect: every registered control's value.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Effective {
    values: BTreeMap<SettingKey, Resolved>,
}

impl Effective {
    /// The value at `section.control` and where it came from.
    #[must_use]
    pub fn get(&self, section: &str, control: &str) -> Option<&Resolved> {
        self.values.get(&SettingKey::new(section, control))
    }

    /// A switch's state.
    #[must_use]
    pub fn switch(&self, section: &str, control: &str) -> Option<bool> {
        match self.get(section, control)?.value {
            Setting::Switch(on) => Some(on),
            _ => None,
        }
    }

    /// A number's value.
    #[must_use]
    pub fn number(&self, section: &str, control: &str) -> Option<i64> {
        match self.get(section, control)?.value {
            Setting::Number(n) => Some(n),
            _ => None,
        }
    }

    /// Every value, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&SettingKey, &Resolved)> {
        self.values.iter()
    }
}

/// `effective = merge(defaults, public, account, user)`.
///
/// `public` and `account` come from [`Registry::resolve`] (validated and
/// targeted); `None` is a layer that isn't there (no file, an off switch,
/// no linked account). User values that don't fit their control are
/// ignored.
#[must_use]
pub fn merge(
    registry: &Registry,
    public: Option<&LayerValues>,
    account: Option<&LayerValues>,
    user: &UserChoices,
) -> Effective {
    let values = registry
        .controls()
        .map(|(key, control)| {
            let mut resolved = Resolved {
                value: control.default_setting(),
                layer: Layer::Default,
            };
            for (layer, values) in [(Layer::Public, public), (Layer::Account, account)] {
                if let Some(value) = values.and_then(|values| values.get(&key)) {
                    resolved = Resolved {
                        value: value.clone(),
                        layer,
                    };
                }
            }
            if let Some(mine) = user.get(&key).and_then(|v| control.user_value(v)) {
                resolved = apply_user(control, resolved, mine);
            }
            (key, resolved)
        })
        .collect();
    Effective { values }
}

fn apply_user(control: &Control, below: Resolved, mine: Setting) -> Resolved {
    let remote = matches!(below.layer, Layer::Public | Layer::Account);
    match (control, &below.value, mine) {
        // Safety: a remote kill stands.
        (
            Control::Switch {
                remote: RemoteMode::OffOnly,
                ..
            },
            Setting::Switch(false),
            _,
        ) if remote => below,
        // Ranges only narrow; an empty result keeps the range below.
        (Control::Versions { .. }, Setting::Versions(current), Setting::Versions(narrower)) => {
            let both = current.intersect(&narrower);
            if both.is_empty() {
                tracing::warn!("remote config: a user range left nothing; ignored");
                below
            } else {
                Resolved {
                    value: Setting::Versions(both),
                    layer: Layer::User,
                }
            }
        }
        (_, _, value) => Resolved {
            value,
            layer: Layer::User,
        },
    }
}
