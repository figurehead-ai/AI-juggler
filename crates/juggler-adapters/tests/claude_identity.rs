//! AJ-E1001: Claude Code's identity probe reads only the `oauthAccount`
//! metadata of `.claude.json` — and never a credential file.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::pedantic)]

use juggler_adapter::IdentityProbe;
use juggler_adapters::claude_code::probe_claude_identity;

#[test]
fn a_signed_in_config_dir_reports_its_account_masked() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join(".claude.json"),
        r#"{"numStartups":3,"oauthAccount":{"accountUuid":"acct-123","emailAddress":"sam@example.com","organizationName":"Acme"}}"#,
    )
    .unwrap();
    assert_eq!(
        probe_claude_identity(dir.path()),
        IdentityProbe::SignedIn {
            subject: "acct-123".into(),
            label: "s…@example.com · Acme".into(),
            org: None,
        }
    );
}

#[test]
fn no_state_file_or_no_account_is_signed_out_and_garbage_is_unverifiable() {
    let dir = tempfile::tempdir().unwrap();
    assert_eq!(probe_claude_identity(dir.path()), IdentityProbe::SignedOut);
    std::fs::write(dir.path().join(".claude.json"), r#"{"numStartups":1}"#).unwrap();
    assert_eq!(probe_claude_identity(dir.path()), IdentityProbe::SignedOut);
    std::fs::write(dir.path().join(".claude.json"), "not json").unwrap();
    assert!(matches!(
        probe_claude_identity(dir.path()),
        IdentityProbe::Unverifiable { .. }
    ));
}

#[test]
fn credential_content_never_reaches_the_probe_result() {
    let dir = tempfile::tempdir().unwrap();
    // A credentials file beside it, and a token-shaped field inside the
    // state file: neither may surface.
    std::fs::write(
        dir.path().join(".credentials.json"),
        r#"{"claudeAiOauth":{"accessToken":"sk-ant-oat01-SECRET"}}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join(".claude.json"),
        r#"{"oauthAccount":{"accountUuid":"acct-9","emailAddress":"a@b.co","accessToken":"sk-ant-oat01-SECRET"}}"#,
    )
    .unwrap();
    let probe = format!("{:?}", probe_claude_identity(dir.path()));
    assert!(!probe.contains("SECRET"), "{probe}");
    assert!(
        !probe.contains("a@b.co"),
        "the full email is never shown: {probe}"
    );
}

#[test]
fn a_personal_plan_and_a_team_seat_on_one_login_name_their_organisations() {
    // D-35: one email, two subscriptions. The subject (the guard's id)
    // is the same; the organisation tells the two accounts apart.
    let probe = |org: &str, name: &str| {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(".claude.json"),
            format!(
                r#"{{"oauthAccount":{{"accountUuid":"acct-1","emailAddress":"sam@example.com","organizationUuid":"{org}","organizationName":"{name}"}}}}"#
            ),
        )
        .unwrap();
        probe_claude_identity(dir.path())
    };
    let seat = |probe: IdentityProbe| match probe {
        IdentityProbe::SignedIn { subject, org, .. } => Some((subject, org)),
        _ => None,
    };
    let personal = seat(probe("org-personal", "Personal"));
    let team = seat(probe("org-team", "Acme"));
    assert_eq!(
        personal,
        Some(("acct-1".to_owned(), Some("org-personal".to_owned())))
    );
    assert_eq!(
        team,
        Some(("acct-1".to_owned(), Some("org-team".to_owned())))
    );
}
