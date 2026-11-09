//! The emission policy: whether this process may write a span at all (`GB-340`, `SP-2`).
//!
//! Split out of `telemetry.rs` when that file crossed the 250-line limit. The separation is also a
//! boundary worth having: `telemetry.rs` knows how to serialise and append a span, and this module
//! knows whether it is allowed to. One is a file format; the other is a decision about a process.

use std::path::Path;

/// Whether this process should record telemetry at all (`GB-340`).
///
/// Two reasons to stay quiet, both of which have put junk in a real log:
///
/// * `GAUGEBLOCK_NO_TELEMETRY` is set - an explicit opt-out for a scripted run that must not touch the log;
/// * the process **is a test binary**. Cargo builds them into `target/*/deps/<crate>-<hash>` and runs them from
///   there, and nextest does the same, so the executable path names the case without any test having to
///   cooperate. That matters because the setter cannot: this workspace denies `unsafe_code`, and `set_var` is
///   unsafe in edition 2024, so a test cannot turn emission off for itself.
#[must_use]
pub fn emission_enabled() -> bool {
    !disabled_by_env(std::env::var(NO_TELEMETRY_ENV).ok().as_deref())
        && !is_test_binary(&std::env::current_exe().unwrap_or_default())
        && !disabled_by_config()
}

/// Whether `.gaugeblock/config.toml` turns telemetry off (`SP-2`).
///
/// The precedence is **environment → config → default on**. The variable wins because it is set per
/// command by whoever runs it, and a scripted invocation that must not touch the log cannot wait for
/// someone to edit a committed file. The file sits below it and above the default, so a suite can opt
/// out permanently without every caller remembering a variable.
///
/// Two silences here are deliberate, and both fail towards *emitting*:
///
/// * no locatable suite root - there is no log to protect, and a tool run outside a checkout should not
///   silently develop different behaviour;
/// * a config file that will not parse - a malformed file is a bug the owning tool reports in its own
///   voice, and answering it by suppressing the only record of what ran would hide the bug's effects.
fn disabled_by_config() -> bool {
    let Ok(cwd) = std::env::current_dir() else {
        return false;
    };
    let Some(sink) = crate::invocation::sink_path(&cwd) else {
        return false;
    };
    // `<root>/.gaugeblock/telemetry/events.jsonl` - the config sits two levels up from the sink. Deriving
    // it from the sink rather than searching again keeps one definition of where the suite is.
    let Some(config_dir) = sink.parent().and_then(std::path::Path::parent) else {
        return false;
    };
    let Some(suite_root) = config_dir.parent() else {
        return false;
    };
    crate::suite_config::SuiteConfig::load(suite_root)
        .map(|config| config.boolean(TELEMETRY_SECTION, "enabled") == Some(false))
        .unwrap_or(false)
}

/// Config section that carries the emission switch.
pub const TELEMETRY_SECTION: &str = "telemetry";

/// Name of the opt-out variable.
pub const NO_TELEMETRY_ENV: &str = "GAUGEBLOCK_NO_TELEMETRY";

/// Whether the variable's value disables emission. Any value counts, including an empty one: a variable set to
/// nothing is still a deliberate act, and honouring it is cheaper than arguing about spellings.
#[must_use]
fn disabled_by_env(value: Option<&str>) -> bool {
    value.is_some()
}

/// Whether an executable path is a cargo-built test binary: `.../target/<profile>/deps/<name>-<hash>`.
///
/// Deliberately structural rather than name-based: a crate called `miter-tests` installed as a real binary lives
/// in `bin/`, not `deps/`, and a hash suffix is what cargo adds to a test artifact.
#[must_use]
fn is_test_binary(exe: &Path) -> bool {
    let in_deps = exe
        .parent()
        .and_then(|dir| dir.file_name())
        .is_some_and(|name| name == "deps");
    let hashed = exe
        .file_stem()
        .and_then(|stem| stem.to_str())
        .and_then(|stem| stem.rsplit_once('-'));
    in_deps
        && hashed.is_some_and(|(_, hash)| {
            hash.len() == 16 && hash.chars().all(|c| c.is_ascii_hexdigit())
        })
}

#[cfg(test)]
mod emission_tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use super::*;

    #[test]
    fn an_installed_binary_is_not_a_test_binary() {
        assert!(!is_test_binary(Path::new("/Users/x/.cargo/bin/miter")));
        assert!(!is_test_binary(Path::new("/repo/target/release/storypole")));
    }

    #[test]
    fn a_cargo_test_artifact_is_recognised_by_its_directory_and_hash() {
        assert!(is_test_binary(Path::new(
            "/repo/target/debug/deps/storypole_core-9f8a1c2b3d4e5f60"
        )));
        // A name with a dash but no hash is not a cargo artifact.
        assert!(!is_test_binary(Path::new(
            "/repo/target/debug/deps/miter-tests"
        )));
        // A hashed name outside deps/ is something else entirely.
        assert!(!is_test_binary(Path::new(
            "/repo/target/debug/miter-9f8a1c2b3d4e5f60"
        )));
    }

    #[test]
    fn any_value_of_the_opt_out_variable_disables_emission() {
        assert!(disabled_by_env(Some("1")));
        assert!(disabled_by_env(Some("")));
        assert!(!disabled_by_env(None));
    }

    #[test]
    fn the_config_switch_is_read_from_the_telemetry_table() {
        // The middle layer of the precedence: env, then config, then default on. A suite that writes
        // `enabled = false` opts out; a suite that writes `enabled = true` states the default; a suite that
        // has no table gets `None`, which is not the same as `false` - absence must not read as an opt-out.
        let dir = tempfile::tempdir().unwrap();
        let gaugeblock = dir.path().join(".gaugeblock");
        std::fs::create_dir_all(&gaugeblock).unwrap();
        let config_path = gaugeblock.join("config.toml");

        std::fs::write(&config_path, "[telemetry]\nenabled = false\n").unwrap();
        let off = crate::suite_config::SuiteConfig::load(dir.path()).unwrap();
        assert_eq!(off.boolean(TELEMETRY_SECTION, "enabled"), Some(false));

        std::fs::write(&config_path, "[telemetry]\nenabled = true\n").unwrap();
        let on = crate::suite_config::SuiteConfig::load(dir.path()).unwrap();
        assert_eq!(on.boolean(TELEMETRY_SECTION, "enabled"), Some(true));

        std::fs::write(&config_path, "[suite]\nmembers = []\n").unwrap();
        let unstated = crate::suite_config::SuiteConfig::load(dir.path()).unwrap();
        assert_eq!(unstated.boolean(TELEMETRY_SECTION, "enabled"), None);
    }
}
