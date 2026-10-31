//! `SuiteConfig` fixtures: the edges the per-tool loaders disagreed on.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use gaugeblock_workspace::{SuiteConfig, WorkspaceError};
use std::path::PathBuf;

fn suite_root(tag: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("gb-suite-{}-{tag}", std::process::id()));
    std::fs::remove_dir_all(&base).ok();
    std::fs::create_dir_all(base.join(".gaugeblock")).unwrap();
    base
}

#[test]
fn test_suite_config_exposes_tool_tables_and_survives_a_missing_file() {
    let root = suite_root("tables");
    std::fs::write(
        root.join(".gaugeblock/config.toml"),
        "[sawhorse]\nplanner = \"anthropic/claude\"\n\n[miter]\nindex = \"redb\"\n",
    )
    .unwrap();

    let config = SuiteConfig::load(&root).unwrap();
    assert_eq!(config.sections(), vec!["miter", "sawhorse"], "sorted");
    assert_eq!(
        config.string("sawhorse", "planner"),
        Some("anthropic/claude")
    );
    // A tool that is not configured, and a key that is not set, are both None rather than defaults.
    assert!(config.section("rabbet").is_none());
    assert!(config.string("miter", "spool").is_none());

    // A fresh checkout has never been configured; that is empty, not an error.
    let bare = suite_root("bare");
    let empty = SuiteConfig::load(&bare).unwrap();
    assert!(empty.is_empty());
    assert!(empty.sections().is_empty());

    std::fs::remove_dir_all(&root).ok();
    std::fs::remove_dir_all(&bare).ok();
}

#[test]
fn test_credential_reads_the_file_and_strips_quotes_and_comments() {
    let root = suite_root("creds");
    std::fs::write(
        root.join(".gaugeblock/.env"),
        "# a comment\n\nKEY_FROM_FILE=from-file\nKEY_QUOTED=\"quoted value\"\n\nKEY_BLANK=\n",
    )
    .unwrap();
    let config = SuiteConfig::load(&root).unwrap();

    assert_eq!(
        config.credential_with("KEY_FROM_FILE", |_| None).as_deref(),
        Some("from-file")
    );
    // Quotes are a shell convention around the secret, not part of it.
    assert_eq!(
        config.credential_with("KEY_QUOTED", |_| None).as_deref(),
        Some("quoted value")
    );
    // An empty assignment is not a credential.
    assert!(config.credential_with("KEY_BLANK", |_| None).is_none());
    // Neither source.
    assert!(config.credential_with("KEY_ABSENT", |_| None).is_none());

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn test_environment_outranks_the_file_unless_it_is_blank() {
    let root = suite_root("precedence");
    std::fs::write(root.join(".gaugeblock/.env"), "KEY_ORDERED=from-file\n").unwrap();
    let config = SuiteConfig::load(&root).unwrap();

    // A deliberate export or CI secret beats the file a developer edits.
    assert_eq!(
        config
            .credential_with("KEY_ORDERED", |_| Some("from-env".to_string()))
            .as_deref(),
        Some("from-env")
    );
    // A blank variable is a shell-profile mistake, so the file still answers.
    assert_eq!(
        config
            .credential_with("KEY_ORDERED", |_| Some("   ".to_string()))
            .as_deref(),
        Some("from-file")
    );
    // With no environment at all, the file is the only source.
    assert_eq!(
        config.credential_with("KEY_ORDERED", |_| None).as_deref(),
        Some("from-file")
    );

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn test_malformed_config_is_an_error_that_names_the_file() {
    let root = suite_root("broken");
    let path = root.join(".gaugeblock/config.toml");
    std::fs::write(&path, "[sawhorse\nplanner = ").unwrap();

    let error = SuiteConfig::load(&root).expect_err("malformed toml must not silently default");
    match error {
        WorkspaceError::InvalidConfig { path: reported, .. } => {
            assert!(
                reported.ends_with(".gaugeblock/config.toml"),
                "got {reported}"
            );
        }
        other => panic!("expected InvalidConfig, got {other:?}"),
    }

    std::fs::remove_dir_all(&root).ok();
}
