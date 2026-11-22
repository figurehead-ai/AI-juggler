//! The system's own on-device AI, as far as the OS can tell without
//! asking the model (ADR-0033 §10, ADR-0034 §10, D-52): whether this
//! computer could have a built-in language model at all, and its chip.
//!
//! - [`system_model`]: a cheap pre-check, no process started. Where it
//!   says no, AI Juggler never starts the helper that asks the model
//!   itself (on an Intel Mac, on a macOS older than the model, on Windows
//!   and Linux). Where it says [`SystemModel::MayHave`], only the model's
//!   own answer (through the app's helper) says whether it is on.
//! - [`machine`]: the processor's maker and generation and the memory,
//!   for which models fit this computer (AJ-E3811).
//!
//! Windows and Linux have no reading yet: any on-device system AI there is
//! researched in each OS's own release. Until then they say
//! [`SystemModel::NotThisSystem`], which fails closed.

/// Whether this computer could have the system's own on-device model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemModel {
    /// It may: only the model itself can say whether it is on and ready.
    MayHave,
    /// This operating system has none (or none this build can reach).
    NotThisSystem,
    /// The system has one, but not on this processor.
    NotThisChip,
    /// The system has one from a later version than the one running.
    NotThisVersion,
}

/// The first macOS major version with a built-in language model.
pub const FIRST_MACOS_WITH_A_MODEL: u32 = 26;

/// This computer's processor, as far as model fit goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chip {
    /// Not Apple silicon (an Intel Mac, or any PC).
    Other,
    /// Apple silicon of generation `n` (M1 is 1); `None` when the system
    /// names a chip this build can't read the generation of.
    AppleSilicon(Option<u8>),
}

/// The pre-check for this computer.
#[must_use]
pub fn system_model() -> SystemModel {
    system_model_on(
        std::env::consts::OS,
        std::env::consts::ARCH,
        sysinfo::System::os_version().as_deref(),
    )
}

/// [`system_model`] for `os`, `arch` and the OS's version text (pure).
#[must_use]
pub fn system_model_on(os: &str, arch: &str, version: Option<&str>) -> SystemModel {
    if os != "macos" {
        return SystemModel::NotThisSystem;
    }
    if arch != "aarch64" {
        return SystemModel::NotThisChip;
    }
    let major = version
        .and_then(|text| text.split('.').next())
        .and_then(|major| major.trim().parse::<u32>().ok());
    match major {
        Some(major) if major >= FIRST_MACOS_WITH_A_MODEL => SystemModel::MayHave,
        // A version the system won't tell fails closed.
        _ => SystemModel::NotThisVersion,
    }
}

/// What a model's fit reads of this computer besides its disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Machine {
    /// Its chip.
    pub chip: Chip,
    /// Its memory in bytes, when the system says.
    pub memory_bytes: Option<u64>,
}

/// This computer's chip and memory, read once by the caller.
#[must_use]
pub fn machine() -> Machine {
    Machine {
        chip: chip(),
        memory_bytes: crate::room::total_memory(),
    }
}

/// This computer's chip, read once by the caller (it costs a system
/// call).
#[must_use]
pub fn chip() -> Chip {
    let mut system = sysinfo::System::new();
    system.refresh_cpu_list(sysinfo::CpuRefreshKind::nothing());
    let brand = system
        .cpus()
        .first()
        .map(|cpu| cpu.brand().to_owned())
        .unwrap_or_default();
    chip_from(std::env::consts::OS, std::env::consts::ARCH, &brand)
}

/// [`chip`] from `os`, `arch` and the processor's brand text (pure):
/// "Apple M4 Pro" is Apple silicon of generation 4.
#[must_use]
pub fn chip_from(os: &str, arch: &str, brand: &str) -> Chip {
    if os != "macos" || arch != "aarch64" {
        return Chip::Other;
    }
    let generation = brand
        .split_whitespace()
        .find_map(|word| word.strip_prefix('M'))
        .and_then(|digits| digits.parse::<u8>().ok())
        .filter(|n| *n > 0);
    Chip::AppleSilicon(generation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_apple_silicon_on_a_new_enough_macos_may_have_a_model() {
        let cases = [
            ("macos", "aarch64", Some("27.0.1"), SystemModel::MayHave),
            ("macos", "aarch64", Some("26.0"), SystemModel::MayHave),
            (
                "macos",
                "aarch64",
                Some("15.5"),
                SystemModel::NotThisVersion,
            ),
            ("macos", "aarch64", None, SystemModel::NotThisVersion),
            (
                "macos",
                "aarch64",
                Some("garbage"),
                SystemModel::NotThisVersion,
            ),
            ("macos", "x86_64", Some("27.0"), SystemModel::NotThisChip),
            ("windows", "x86_64", Some("11"), SystemModel::NotThisSystem),
            ("linux", "aarch64", Some("6.8"), SystemModel::NotThisSystem),
        ];
        for (os, arch, version, want) in cases {
            assert_eq!(
                system_model_on(os, arch, version),
                want,
                "{os} {arch} {version:?}"
            );
        }
    }

    #[test]
    fn the_chip_generation_comes_from_the_brand() {
        assert_eq!(
            chip_from("macos", "aarch64", "Apple M4 Pro"),
            Chip::AppleSilicon(Some(4))
        );
        assert_eq!(
            chip_from("macos", "aarch64", "Apple M1"),
            Chip::AppleSilicon(Some(1))
        );
        assert_eq!(
            chip_from("macos", "aarch64", "Apple M12 Ultra"),
            Chip::AppleSilicon(Some(12))
        );
        assert_eq!(
            chip_from("macos", "aarch64", "Apple Silicon"),
            Chip::AppleSilicon(None)
        );
        assert_eq!(
            chip_from("macos", "x86_64", "Intel(R) Core(TM) i7"),
            Chip::Other
        );
        assert_eq!(chip_from("linux", "aarch64", "Apple M2"), Chip::Other);
    }

    #[test]
    fn this_computer_answers_without_panicking() {
        let _ = (system_model(), chip());
    }
}
