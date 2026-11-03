//! Checks GitHub for a newer PresentCraft release.
//!
//! The check runs on a background thread and only reads the latest release's
//! tag and page address. Nothing is downloaded or installed: the person is
//! offered the release page in their browser.

use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The version this build was made from.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// GitHub's description of the newest published (non-draft, non-pre-release) release.
const LATEST_API: &str = "https://api.github.com/repos/saturngod/PresentCraft/releases/latest";
/// Where to send people when the API doesn't give a page address.
pub const RELEASES_PAGE: &str = "https://github.com/saturngod/PresentCraft/releases/latest";
/// Automatic checks happen at most this often.
pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Version without the leading "v", for example "0.0.4".
    pub version: String,
    /// The release's web page.
    pub url: String,
}

#[derive(Debug, PartialEq)]
pub enum Outcome {
    Newer(Release),
    UpToDate,
    Failed(String),
}

/// One check in progress, started from the menu (`manual`) or at launch.
pub struct Check {
    rx: Receiver<Outcome>,
    pub manual: bool,
}

impl Check {
    pub fn start(ctx: &eframe::egui::Context, manual: bool) -> Self {
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let outcome = match fetch_latest() {
                Ok(release) if is_newer(&release.version, CURRENT) => Outcome::Newer(release),
                Ok(_) => Outcome::UpToDate,
                Err(e) => Outcome::Failed(e),
            };
            let _ = tx.send(outcome);
            ctx.request_repaint();
        });
        Check { rx, manual }
    }

    /// The result, once the check has finished.
    pub fn poll(&self) -> Option<Outcome> {
        match self.rx.try_recv() {
            Ok(outcome) => Some(outcome),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Some(Outcome::Failed("the check stopped unexpectedly".into()))
            }
        }
    }
}

/// Seconds since 1970, used to space out automatic checks.
pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Whether an automatic check is due, given when the last one succeeded.
pub fn due(last_check: u64, now: u64) -> bool {
    // A clock set backwards also counts as due.
    now < last_check || now - last_check >= CHECK_INTERVAL_SECS
}

fn fetch_latest() -> Result<Release, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .into();
    let mut response = agent
        .get(LATEST_API)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", &format!("PresentCraft/{CURRENT}"))
        .call()
        .map_err(|e| e.to_string())?;
    let body = response
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    parse_release(&body)
}

/// Reads the tag and page address from GitHub's release JSON.
fn parse_release(body: &str) -> Result<Release, String> {
    let json = crate::json::Json::parse(body)?;
    let tag = json
        .str("tag_name")
        .ok_or("the release has no tag")?
        .trim();
    if parse_version(tag).is_none() {
        return Err(format!("unexpected release tag \"{tag}\""));
    }
    let url = json
        .str("html_url")
        .filter(|u| u.starts_with("https://github.com/"))
        .unwrap_or(RELEASES_PAGE);
    Ok(Release {
        version: tag.trim_start_matches(['v', 'V']).to_string(),
        url: url.to_string(),
    })
}

/// "v1.2.3", "1.2" or "1.2.3-beta" → [1, 2, 3]. Pre-release and build
/// suffixes are ignored.
fn parse_version(text: &str) -> Option<[u64; 3]> {
    let core = text
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()?;
    let mut out = [0u64; 3];
    let mut parts = core.split('.');
    for slot in out.iter_mut() {
        match parts.next() {
            Some(p) => *slot = p.parse().ok()?,
            None => break,
        }
    }
    if parts.next().is_some() || core.is_empty() {
        return None;
    }
    Some(out)
}

/// Whether `candidate` is a later version than `current`.
pub fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(a), Some(b)) => a > b,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_numerically() {
        assert!(is_newer("v0.0.4", "0.0.3"));
        assert!(is_newer("0.1.0", "0.0.9"));
        assert!(is_newer("v0.0.10", "0.0.9"));
        assert!(is_newer("1.0", "0.9.9"));
        assert!(!is_newer("v0.0.3", "0.0.3"));
        assert!(!is_newer("0.0.2", "0.0.3"));
        assert!(!is_newer("nightly", "0.0.3"));
        assert!(!is_newer("1.2.3.4", "0.0.3"));
    }

    #[test]
    fn prerelease_suffix_is_ignored() {
        assert_eq!(parse_version("v1.2.3-beta.1"), Some([1, 2, 3]));
        assert_eq!(parse_version("1.2.3+build5"), Some([1, 2, 3]));
    }

    #[test]
    fn reads_github_release_json() {
        let body = r#"{"tag_name":"v0.0.4","html_url":"https://github.com/saturngod/PresentCraft/releases/tag/v0.0.4","draft":false}"#;
        assert_eq!(
            parse_release(body),
            Ok(Release {
                version: "0.0.4".into(),
                url: "https://github.com/saturngod/PresentCraft/releases/tag/v0.0.4".into(),
            })
        );
    }

    #[test]
    fn unexpected_page_falls_back_to_releases() {
        let body = r#"{"tag_name":"v0.0.4","html_url":"https://example.com/x"}"#;
        assert_eq!(parse_release(body).unwrap().url, RELEASES_PAGE);
        assert!(parse_release(r#"{"tag_name":"latest"}"#).is_err());
    }

    /// Talks to GitHub: `cargo test --locked -- --ignored live_release`.
    #[test]
    #[ignore]
    fn live_release_check() {
        let release = fetch_latest().expect("latest release");
        assert!(parse_version(&release.version).is_some());
        println!("latest release: {release:?}");
    }

    #[test]
    fn checks_are_spaced_a_day_apart() {
        assert!(due(0, 1_000_000));
        assert!(!due(1_000_000, 1_000_000 + 3600));
        assert!(due(1_000_000, 1_000_000 + CHECK_INTERVAL_SECS));
        assert!(due(2_000_000, 1_000_000));
    }
}
