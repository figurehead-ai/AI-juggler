// A trimmed copy of an adapter's spec, in the shape the watcher reads.
pub const SAMPLE_ID: &str = "sample-tool";

pub fn sample_spec() -> juggler_adapter::ToolSpec {
    ToolSpec {
        id: SAMPLE_ID.to_owned(),
        display_name: "Sample Tool".to_owned(),
        release_feed: Some(juggler_adapter::ReleaseFeed::Npm {
            package: "@example/tool-cli".to_owned(),
        }),
        more_release_feeds: vec![
            juggler_adapter::ReleaseFeed::Appcast {
                url: "https://example.com/app/appcast.xml".to_owned(),
            },
            juggler_adapter::ReleaseFeed::GithubRelease {
                repo: "example/tool".to_owned(),
            },
        ],
        update_command: None,
    }
}
