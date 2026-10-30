//! "Is there a newer version?" — reads the latest release from GitHub. Read-only: one HTTPS
//! request to a fixed address, nothing about the user is sent, nothing is downloaded or run.

use std::time::Duration;

use serde::{Deserialize, Serialize};

const LATEST_URL: &str = "https://api.github.com/repos/ImgyeongLee/NoraSchedule/releases/latest";
/// Release pages we are willing to show (the link comes from the response, so it is checked).
const RELEASE_PAGE_PREFIX: &str = "https://github.com/ImgyeongLee/NoraSchedule/releases/";

#[derive(Debug, Serialize, PartialEq)]
pub struct LatestRelease {
    /// e.g. "0.6.0" (without the leading "v").
    pub version: String,
    /// The release page on GitHub.
    pub url: String,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
}

/// Turns GitHub's answer into a version and page, rejecting anything unexpected.
fn parse(release: GithubRelease) -> Option<LatestRelease> {
    let version = release.tag_name.trim().trim_start_matches(['v', 'V']).to_string();
    let valid = !version.is_empty()
        && version.len() <= 32
        && version.split('.').all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()));
    if !valid || !release.html_url.starts_with(RELEASE_PAGE_PREFIX) {
        return None;
    }
    Some(LatestRelease { version, url: release.html_url })
}

/// The newest published release, or `None` if there is none yet.
pub async fn latest_release() -> Result<Option<LatestRelease>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        // GitHub requires a User-Agent; it only names the app and its version.
        .user_agent(concat!("NoraSchedule/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(LATEST_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let release: GithubRelease = response.error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    Ok(parse(release))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(tag: &str, url: &str) -> GithubRelease {
        GithubRelease { tag_name: tag.into(), html_url: url.into() }
    }

    #[test]
    fn only_plain_versions_and_this_repos_pages_are_accepted() {
        let page = "https://github.com/ImgyeongLee/NoraSchedule/releases/tag/v0.6.0";
        assert_eq!(parse(release("v0.6.0", page)), Some(LatestRelease { version: "0.6.0".into(), url: page.into() }));
        assert_eq!(parse(release("1.2", page)).map(|r| r.version), Some("1.2".into()));
        assert!(parse(release("v0.6.0-beta", page)).is_none());
        assert!(parse(release("<script>", page)).is_none());
        assert!(parse(release("v0.6.0", "https://evil.example/download")).is_none());
    }

    /// Real request to GitHub; run with `cargo test -- --ignored`.
    #[test]
    #[ignore = "needs the internet"]
    fn reads_the_latest_release_from_github() {
        let latest = tauri::async_runtime::block_on(latest_release()).unwrap().expect("a published release");
        println!("latest: {} {}", latest.version, latest.url);
        assert!(latest.url.starts_with(RELEASE_PAGE_PREFIX));
    }
}
