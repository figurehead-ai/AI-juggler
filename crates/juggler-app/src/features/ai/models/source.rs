//! ADR-0035 §11.2 and §11.8: where a model file may come from. The hosts,
//! the path shapes, the official organisations and the redirect hosts are
//! compiled here; the signed `models` section can't widen any of them.
//!
//! One parser serves the publisher's check and the app:
//! - `https://` only;
//! - no userinfo, no port, no IP literal, no percent-encoding before the
//!   query, no backslash, no white space or control character, ASCII
//!   only;
//! - no query or fragment in a pinned URL (a redirect hop may carry the
//!   host's own query);
//! - the host matched exactly, in lower case;
//! - path segments are `[A-Za-z0-9._-]`, never empty, `.` or `..`.

/// Where a model file comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Hugging Face, at a pinned commit, from an official organisation.
    HuggingFace,
    /// Our own files, addressed by their hash (needs the maintainer's
    /// setup: ADR-0035 open question 3).
    Own,
}

/// Hugging Face's host for pinned files.
pub const HF_HOST: &str = "huggingface.co";

/// The only organisations a Hugging Face URL may name: the official
/// makers of the tested models. A new one is a release, on purpose.
pub const HF_ORGS: &[&str] = &["Qwen", "ibm-granite", "nvidia", "google"];

/// Hugging Face's published file hosts: the only places its pinned URL may
/// redirect to (its firewall page; re-checked at each release).
pub const HF_REDIRECT_HOSTS: &[&str] = &[
    "cdn-lfs-us-1.hf.co",
    "cdn-lfs-eu-1.hf.co",
    "us.aws.cdn.hf.co",
    "us.gcp.cdn.hf.co",
    "transfer.xethub.hf.co",
    "transfer.xethub-eu.hf.co",
    "cas-server.xethub.hf.co",
    "cas-server.xethub-eu.hf.co",
];

/// Our own host for model files.
pub const OWN_HOST: &str = "models.figurehead.ai";

/// The hosts a download URL may name (the list field's own check; the
/// path rules below come on top).
pub const DOWNLOAD_HOSTS: &[&str] = &[HF_HOST, OWN_HOST];

/// The hosts a licence link may name.
pub const LICENCE_HOSTS: &[&str] = &[HF_HOST, "figurehead.ai", "www.figurehead.ai"];

/// The most redirects a download follows.
pub const MAX_REDIRECTS: usize = 3;

/// The longest redirect URL followed (a signed CDN URL carries a long
/// query).
const MAX_REDIRECT_LEN: usize = 4096;

/// The file type every downloadable model has (the local engine's).
const MODEL_SUFFIX: &str = ".gguf";

/// A URL split into its host, its path segments and whether it carried a
/// query; `None` if it breaks a rule in the module docs.
fn parse(url: &str, allow_query: bool) -> Option<(&str, Vec<&str>, bool)> {
    // A redirect hop's signed query can be long; a pinned URL can't.
    let limit = if allow_query {
        MAX_REDIRECT_LEN
    } else {
        juggler_remote_config::list::MAX_LINK_LEN
    };
    if url.len() > limit
        || !url.is_ascii()
        || url
            .bytes()
            .any(|b| b.is_ascii_control() || matches!(b, b' ' | b'\\'))
    {
        return None;
    }
    let rest = url.strip_prefix("https://")?;
    let (rest, query) = match rest.split_once('?') {
        Some((rest, query)) => (rest, Some(query)),
        None => (rest, None),
    };
    // Percent-encoding and `@` only in a host's own query, never before it.
    if rest.contains(['%', '@']) {
        return None;
    }
    if rest.contains('#') || query.is_some_and(|query| query.contains('#')) {
        return None;
    }
    if query.is_some() && !allow_query {
        return None;
    }
    let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
    let host_ok = !host.is_empty()
        && !host.contains(':')
        && !host.starts_with('[')
        && host
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
        && !host.bytes().all(|b| b.is_ascii_digit() || b == b'.');
    if !host_ok {
        return None;
    }
    let segments: Vec<&str> = if path.is_empty() {
        Vec::new()
    } else {
        path.split('/').collect()
    };
    let segments_ok = segments.iter().all(|segment| {
        !segment.is_empty()
            && *segment != "."
            && *segment != ".."
            && segment
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    });
    segments_ok.then_some((host, segments, query.is_some()))
}

fn is_commit(text: &str) -> bool {
    text.len() == 40
        && text
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn is_model_file(name: &str) -> bool {
    name.len() > MODEL_SUFFIX.len() && name.ends_with(MODEL_SUFFIX)
}

/// Where the pinned `url` of a file whose hash is `sha256` comes from, or
/// why it can't be used:
/// - `https://huggingface.co/<org>/<repo>/resolve/<40-hex commit>/<file>.gguf`
///   with `<org>` one of [`HF_ORGS`];
/// - `https://models.figurehead.ai/v1/<sha256>/<file>.gguf`, the hash
///   being the entry's own.
///
/// # Errors
///
/// The reason, in words, when the URL breaks a rule.
pub fn pinned(url: &str, sha256: &str) -> Result<Source, &'static str> {
    let (host, segments, _) = parse(url, false).ok_or("not a plain https URL")?;
    match (host, segments.as_slice()) {
        (HF_HOST, [org, _repo, "resolve", commit, file]) => {
            if !HF_ORGS.contains(org) {
                Err("not an official organisation")
            } else if !is_commit(commit) {
                Err("not pinned to a commit")
            } else if !is_model_file(file) {
                Err("not a GGUF file")
            } else {
                Ok(Source::HuggingFace)
            }
        }
        (HF_HOST, _) => Err("not a pinned Hugging Face file"),
        (OWN_HOST, ["v1", hash, file]) => {
            if *hash != sha256 {
                Err("the path's hash isn't the entry's")
            } else if !is_model_file(file) {
                Err("not a GGUF file")
            } else {
                Ok(Source::Own)
            }
        }
        (OWN_HOST, _) => Err("not one of our model paths"),
        _ => Err("a host off the list"),
    }
}

/// Whether a download from `source` may follow a redirect to `url`: an
/// https URL on one of the source's redirect hosts (our own host
/// redirects nowhere). The host's own query is allowed.
#[must_use]
pub fn redirect_allowed(source: Source, url: &str) -> bool {
    let hosts: &[&str] = match source {
        Source::HuggingFace => HF_REDIRECT_HOSTS,
        Source::Own => &[],
    };
    parse(url, true).is_some_and(|(host, _, _)| hosts.contains(&host))
}

/// Whether `url` may be a licence link: a plain https URL on one of
/// [`LICENCE_HOSTS`].
#[must_use]
pub fn licence_link_ok(url: &str) -> bool {
    parse(url, false).is_some_and(|(host, _, _)| LICENCE_HOSTS.contains(&host))
}
