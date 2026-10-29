//! The user's choice and what actually applies after the overrides
//! (checked in order; the first match wins).

/// The modes the user can choose.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Nothing counted, stored or sent (the default).
    Off,
    /// Counted on this Mac only; never sent.
    Local,
    /// Counted, and one report a week is sent.
    Share,
}

impl Mode {
    /// The setting's wire value.
    #[must_use]
    pub const fn wire(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Local => "local",
            Self::Share => "share",
        }
    }
}

/// Why the effective mode differs from the user's choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Override {
    /// `DO_NOT_TRACK` is set (to anything but empty or `0`).
    DoNotTrack,
    /// `AI_JUGGLER_STATS=off`.
    EnvOff,
    /// `AI_JUGGLER_STATS=local` capped the mode.
    EnvLocal,
    /// `CI` is set: counted locally if allowed, never uploaded.
    Ci,
    /// A debug build: counted locally if allowed, uploaded only to a
    /// test endpoint.
    DebugBuild,
}

impl Override {
    /// The reason in plain words, for Settings ▸ Privacy.
    #[must_use]
    pub const fn words(self) -> &'static str {
        match self {
            Self::DoNotTrack => "Off because DO_NOT_TRACK is set",
            Self::EnvOff => "Off because AI_JUGGLER_STATS=off is set",
            Self::EnvLocal => "Kept on this computer because AI_JUGGLER_STATS=local is set",
            Self::Ci => "Never sent from CI",
            Self::DebugBuild => "Never sent from a development build",
        }
    }
}

/// The environment the overrides read (once, at launch).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Environment {
    /// `DO_NOT_TRACK`.
    pub do_not_track: Option<String>,
    /// `AI_JUGGLER_STATS`.
    pub ai_juggler_stats: Option<String>,
    /// `CI`.
    pub ci: Option<String>,
    /// This is a debug build.
    pub debug_build: bool,
    /// `AI_JUGGLER_STATS_ENDPOINT` (honoured by debug builds only).
    pub endpoint: Option<String>,
}

impl Environment {
    /// Reads the process environment.
    #[must_use]
    pub fn from_process() -> Self {
        let var = |name: &str| std::env::var(name).ok();
        Self {
            do_not_track: var("DO_NOT_TRACK"),
            ai_juggler_stats: var("AI_JUGGLER_STATS"),
            ci: var("CI"),
            debug_build: cfg!(debug_assertions),
            endpoint: var("AI_JUGGLER_STATS_ENDPOINT"),
        }
    }

    /// `DO_NOT_TRACK` set to anything other than empty or `0`.
    #[must_use]
    pub fn do_not_track(&self) -> bool {
        self.do_not_track
            .as_deref()
            .is_some_and(|v| !v.trim().is_empty() && v.trim() != "0")
    }
}

/// What applies after the overrides.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub struct EffectiveMode {
    /// Counting happens (local or share).
    pub mode: Mode,
    /// Reports may be uploaded (share, and no override forbids it).
    pub can_upload: bool,
    /// Why it differs from the choice, if it does.
    pub reason: Option<Override>,
}

/// Applies the overrides to the user's choice.
#[must_use]
pub fn effective(chosen: Mode, env: &Environment) -> EffectiveMode {
    let off = |reason| EffectiveMode {
        mode: Mode::Off,
        can_upload: false,
        reason: Some(reason),
    };
    if env.do_not_track() {
        return off(Override::DoNotTrack);
    }
    match env.ai_juggler_stats.as_deref().map(str::trim) {
        Some("off") => return off(Override::EnvOff),
        Some("local") if chosen == Mode::Share => {
            return EffectiveMode {
                mode: Mode::Local,
                can_upload: false,
                reason: Some(Override::EnvLocal),
            };
        }
        _ => {}
    }
    if chosen == Mode::Share {
        if env.ci.as_deref().is_some_and(|v| !v.trim().is_empty()) {
            return EffectiveMode {
                mode: Mode::Share,
                can_upload: false,
                reason: Some(Override::Ci),
            };
        }
        if env.debug_build {
            return EffectiveMode {
                mode: Mode::Share,
                can_upload: env.endpoint.is_some(),
                reason: Some(Override::DebugBuild),
            };
        }
    }
    EffectiveMode {
        mode: chosen,
        can_upload: chosen == Mode::Share,
        reason: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release() -> Environment {
        Environment::default()
    }

    #[test]
    fn the_choice_applies_without_overrides() {
        for mode in [Mode::Off, Mode::Local, Mode::Share] {
            let got = effective(mode, &release());
            assert_eq!(got.mode, mode);
            assert_eq!(got.can_upload, mode == Mode::Share);
            assert_eq!(got.reason, None);
        }
    }

    #[test]
    fn overrides_win_in_order_with_their_reason() {
        let env = |f: fn(&mut Environment)| {
            let mut env = release();
            f(&mut env);
            env
        };
        let dnt = effective(Mode::Share, &env(|e| e.do_not_track = Some("1".into())));
        assert_eq!(
            (dnt.mode, dnt.can_upload, dnt.reason),
            (Mode::Off, false, Some(Override::DoNotTrack))
        );
        assert_eq!(
            effective(Mode::Share, &env(|e| e.do_not_track = Some("0".into()))).mode,
            Mode::Share
        );
        assert_eq!(
            effective(Mode::Share, &env(|e| e.do_not_track = Some(String::new()))).mode,
            Mode::Share
        );
        let off = effective(
            Mode::Share,
            &env(|e| e.ai_juggler_stats = Some("off".into())),
        );
        assert_eq!((off.mode, off.reason), (Mode::Off, Some(Override::EnvOff)));
        let local = effective(
            Mode::Share,
            &env(|e| e.ai_juggler_stats = Some("local".into())),
        );
        assert_eq!(
            (local.mode, local.can_upload, local.reason),
            (Mode::Local, false, Some(Override::EnvLocal))
        );
        // `local` caps; it never raises.
        assert_eq!(
            effective(
                Mode::Off,
                &env(|e| e.ai_juggler_stats = Some("local".into()))
            )
            .mode,
            Mode::Off
        );
        let ci = effective(Mode::Share, &env(|e| e.ci = Some("true".into())));
        assert_eq!(
            (ci.mode, ci.can_upload, ci.reason),
            (Mode::Share, false, Some(Override::Ci))
        );
        let debug = effective(Mode::Share, &env(|e| e.debug_build = true));
        assert_eq!(
            (debug.can_upload, debug.reason),
            (false, Some(Override::DebugBuild))
        );
        let debug_endpoint = effective(
            Mode::Share,
            &env(|e| {
                e.debug_build = true;
                e.endpoint = Some("http://localhost:8790/v1/reports".into());
            }),
        );
        assert!(debug_endpoint.can_upload);
        // DO_NOT_TRACK beats everything.
        let both = effective(
            Mode::Share,
            &env(|e| {
                e.do_not_track = Some("yes".into());
                e.ai_juggler_stats = Some("local".into());
            }),
        );
        assert_eq!(both.reason, Some(Override::DoNotTrack));
    }
}
