//! Checks GitHub for a newer PresentCraft release.
//!
//! The check runs on a background thread and only reads the latest release's
//! tag, page address and downloads. On macOS the person can also install the
//! release in place: the download is unpacked and checked in the background,
//! then swapped in for the running app when PresentCraft quits and reopened.
//! Other systems, and copies that can't be replaced, are sent to the release
//! page instead.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The version this build was made from.
pub const CURRENT: &str = env!("CARGO_PKG_VERSION");
/// GitHub's description of the newest published (non-draft, non-pre-release) release.
const LATEST_API: &str = "https://api.github.com/repos/saturngod/PresentCraft/releases/latest";
/// Where to send people when the API doesn't give a page address.
pub const RELEASES_PAGE: &str = "https://github.com/saturngod/PresentCraft/releases/latest";
/// While the app stays open, automatic checks repeat this often.
pub const CHECK_INTERVAL_SECS: u64 = 24 * 60 * 60;
/// How long to wait before retrying an automatic check that failed.
pub const RETRY_SECS: u64 = 60 * 60;

#[derive(Clone, Debug, PartialEq)]
pub struct Release {
    /// Version without the leading "v", for example "0.0.4".
    pub version: String,
    /// The release's web page.
    pub url: String,
    /// The package for this computer, when the release has one.
    pub asset: Option<Asset>,
}

/// A downloadable file attached to a release.
#[derive(Clone, Debug, PartialEq)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
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
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
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

/// Reads the tag, page address and this computer's download from GitHub's release JSON.
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
    let version = tag.trim_start_matches(['v', 'V']).to_string();
    let asset = platform_suffix().and_then(|suffix| find_asset(&json, &version, suffix));
    Ok(Release {
        version,
        url: url.to_string(),
        asset,
    })
}

/// The end of the release file name for this computer, where one is published.
fn platform_suffix() -> Option<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Some("-macos-arm64.zip"),
        _ => None,
    }
}

/// The release file for `version` whose name ends with `suffix`, from GitHub's own address.
fn find_asset(json: &crate::json::Json, version: &str, suffix: &str) -> Option<Asset> {
    let prefix = format!("PresentCraft-{version}-");
    json.arr("assets").iter().find_map(|a| {
        let name = a.str("name")?;
        let url = a.str("browser_download_url")?;
        (name.starts_with(&prefix)
            && name.ends_with(suffix)
            && url.starts_with("https://github.com/"))
        .then(|| Asset {
            name: name.to_string(),
            url: url.to_string(),
            size: a.num("size").unwrap_or(0.0).max(0.0) as u64,
        })
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

/// The app bundle this copy can replace in place, and the signing team a
/// new version must be signed by.
#[derive(Clone, Debug)]
pub struct Target {
    pub app: PathBuf,
    team: String,
}

/// Where this copy can install an update in place: a PresentCraft.app signed
/// with a Developer ID (ad hoc and development builds don't qualify, since any
/// download could match their signature) in a folder this user can write to.
pub fn install_target() -> Option<Target> {
    let app = running_bundle()?;
    let team = team_id(&app)?;
    can_replace(&app).then_some(Target { app, team })
}

/// Numbers attempts, so each download gets a folder of its own.
static ATTEMPT: AtomicU64 = AtomicU64::new(0);

/// A release being downloaded and checked in the background.
pub struct Install {
    pub release: Release,
    /// The new app bundle, once it is downloaded, unpacked and checked.
    app: Option<PathBuf>,
    received: Arc<AtomicU64>,
    total: u64,
    cancel: Arc<AtomicBool>,
    rx: Option<Receiver<Result<PathBuf, String>>>,
    /// The folder the download is unpacked in.
    dir: PathBuf,
}

impl Install {
    pub fn start(
        ctx: &eframe::egui::Context,
        release: Release,
        asset: Asset,
        target: &Target,
    ) -> Self {
        let attempt = ATTEMPT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "presentcraft-update-{}-{}-{attempt}",
            release.version,
            std::process::id()
        ));
        let received = Arc::new(AtomicU64::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = channel();
        let job = Job {
            asset,
            version: release.version.clone(),
            team: target.team.clone(),
            dir: dir.clone(),
            received: received.clone(),
            cancel: cancel.clone(),
        };
        let total = job.asset.size;
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = job.run();
            if result.is_err() {
                let _ = std::fs::remove_dir_all(&job.dir);
            }
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        Install {
            release,
            app: None,
            received,
            total,
            cancel,
            rx: Some(rx),
            dir,
        }
    }

    /// Download progress from 0 to 1.
    pub fn fraction(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.received.load(Ordering::Relaxed) as f32 / self.total as f32).clamp(0.0, 1.0)
    }

    /// True once the new app is ready to be installed.
    pub fn ready(&self) -> bool {
        self.app.is_some()
    }

    /// The new app bundle, once ready.
    pub fn app(&self) -> Option<&Path> {
        self.app.as_deref()
    }

    /// Picks up the result of the download. `Some(Err)` when it failed.
    pub fn poll(&mut self) -> Option<Result<(), String>> {
        let rx = self.rx.as_ref()?;
        match rx.try_recv() {
            Ok(Ok(app)) => {
                self.app = Some(app);
                self.rx = None;
                Some(Ok(()))
            }
            Ok(Err(e)) => {
                self.rx = None;
                Some(Err(e))
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.rx = None;
                Some(Err("the download stopped unexpectedly".into()))
            }
        }
    }

    /// Stops the download and deletes its files. Used when the person
    /// chooses not to install, when it failed, and when the app quits
    /// without installing.
    pub fn discard(self) {
        self.cancel.store(true, Ordering::Relaxed);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The work done on the download thread.
struct Job {
    asset: Asset,
    version: String,
    team: String,
    dir: PathBuf,
    received: Arc<AtomicU64>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    fn run(&self) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let zip = self.dir.join(&self.asset.name);
        self.download(&zip)?;
        if self.cancel.load(Ordering::Relaxed) {
            return Err("cancelled".into());
        }
        unpack_and_check(&zip, &self.dir, &self.version, &self.team)
    }

    /// Saves the asset to `zip`, counting bytes into `received`. A stalled
    /// connection fails after a minute without data; a slow one keeps going.
    fn download(&self, zip: &Path) -> Result<(), String> {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_connect(Some(Duration::from_secs(30)))
            .timeout_recv_response(Some(Duration::from_secs(60)))
            .timeout_per_read(Some(Duration::from_secs(60)))
            .build()
            .into();
        let response = agent
            .get(&self.asset.url)
            .header("User-Agent", &format!("PresentCraft/{CURRENT}"))
            .call()
            .map_err(|e| e.to_string())?;
        let mut reader = response.into_body().into_reader();
        let mut file = std::fs::File::create(zip).map_err(|e| e.to_string())?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            if self.cancel.load(Ordering::Relaxed) {
                return Err("cancelled".into());
            }
            let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
            self.received.fetch_add(n as u64, Ordering::Relaxed);
        }
        file.sync_all().map_err(|e| e.to_string())?;
        let got = std::fs::metadata(zip).map(|m| m.len()).unwrap_or(0);
        let size = self.asset.size;
        if size > 0 && got != size {
            return Err(format!("the download was incomplete ({got} of {size} bytes)"));
        }
        Ok(())
    }
}

/// Unpacks the macOS zip with ditto (which keeps the bundle intact) and checks
/// that it is a PresentCraft app of the expected version, signed with an
/// Apple-issued certificate of the same team as the running app.
fn unpack_and_check(zip: &Path, dir: &Path, version: &str, team: &str) -> Result<PathBuf, String> {
    let status = Command::new("/usr/bin/ditto")
        .arg("-x")
        .arg("-k")
        .arg(zip)
        .arg(dir)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("the download could not be unpacked".into());
    }
    let app = dir.join("PresentCraft.app");
    if !app.join("Contents/MacOS/PresentCraft").is_file() {
        return Err("the download has no PresentCraft app in it".into());
    }
    let plist = app.join("Contents/Info.plist");
    let shown = Command::new("/usr/libexec/PlistBuddy")
        .args(["-c", "Print :CFBundleShortVersionString"])
        .arg(&plist)
        .output()
        .map_err(|e| e.to_string())?;
    let found = String::from_utf8_lossy(&shown.stdout).trim().to_string();
    if found != version {
        return Err(format!("the download is version {found}, not {version}"));
    }
    let signed = Command::new("/usr/bin/codesign")
        .args(["--verify", "--deep", "--strict", "-R"])
        .arg(signer_requirement(team))
        .arg(&app)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| e.to_string())?;
    if !signed.success() {
        return Err("the download isn't signed by the PresentCraft developer".into());
    }
    Ok(app)
}

/// A code requirement: an Apple-issued (Developer ID) certificate for `team`.
fn signer_requirement(team: &str) -> String {
    format!("=anchor apple generic and certificate leaf[subject.OU] = \"{team}\"")
}

/// The signing team of a bundle, or None for ad hoc and unsigned builds.
fn team_id(app: &Path) -> Option<String> {
    let out = Command::new("/usr/bin/codesign")
        .arg("-dv")
        .arg(app)
        .output()
        .ok()?;
    // codesign prints the details on stderr.
    parse_team_id(&String::from_utf8_lossy(&out.stderr))
}

fn parse_team_id(details: &str) -> Option<String> {
    let team = details
        .lines()
        .find_map(|l| l.strip_prefix("TeamIdentifier="))?
        .trim();
    (!team.is_empty() && team.chars().all(|c| c.is_ascii_alphanumeric()))
        .then(|| team.to_string())
}

/// The app bundle this program runs from, when it is a PresentCraft.app.
fn running_bundle() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let macos = exe.parent()?;
    if macos.file_name()? != "MacOS" {
        return None;
    }
    let app = macos.parent()?.parent()?;
    (app.extension()? == "app").then(|| app.to_path_buf())
}

/// Whether the app bundle's folder can be written to, so it can be replaced.
pub fn can_replace(app: &Path) -> bool {
    let Some(parent) = app.parent() else {
        return false;
    };
    let probe = parent.join(format!(".presentcraft-update-check-{}", std::process::id()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

/// Left by the swap script when it couldn't install the new version.
fn failed_marker() -> Option<PathBuf> {
    crate::settings::Settings::app_dir().map(|d| d.join("update-failed.txt"))
}

/// True once after a launch that follows a failed install (and clears the note).
pub fn take_failed_install() -> bool {
    let Some(marker) = failed_marker() else {
        return false;
    };
    let failed = marker.exists();
    if failed {
        let _ = std::fs::remove_file(&marker);
    }
    failed
}

/// Starts a small background script that waits for this process to exit,
/// then swaps `new_app` in for `target` and opens it.
///
/// The new bundle is first copied next to the target, so both swaps are
/// renames on one volume. If anything fails the old copy stays (or is put
/// back), a note is left for the next launch, and the old copy reopens.
pub fn spawn_swap(new_app: &Path, target: &Path) -> Result<(), String> {
    const SCRIPT: &str = r#"
pid="$1"; new="$2"; app="$3"; staging="$4"; marker="$5"
old="$app.update-old"; next="$app.update-new"
while kill -0 "$pid" 2>/dev/null; do sleep 0.2; done
rm -rf "$old" "$next"
ok=0
if /usr/bin/ditto "$new" "$next"; then
  if mv "$app" "$old"; then
    if mv "$next" "$app"; then
      ok=1
      rm -rf "$old"
    else
      mv "$old" "$app"
    fi
  fi
fi
rm -rf "$next" "$staging"
if [ "$ok" != 1 ] && [ -n "$marker" ]; then
  mkdir -p "$(dirname "$marker")" && echo failed > "$marker"
fi
open "$app"
"#;
    let staging = new_app.parent().ok_or("no folder for the update")?;
    let marker = failed_marker().unwrap_or_default();
    Command::new("/bin/sh")
        .arg("-c")
        .arg(SCRIPT)
        .arg("presentcraft-update")
        .arg(std::process::id().to_string())
        .arg(new_app)
        .arg(target)
        .arg(staging)
        .arg(marker)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Json;

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
                asset: None,
            })
        );
    }

    #[test]
    fn unexpected_page_falls_back_to_releases() {
        let body = r#"{"tag_name":"v0.0.4","html_url":"https://example.com/x"}"#;
        assert_eq!(parse_release(body).unwrap().url, RELEASES_PAGE);
        assert!(parse_release(r#"{"tag_name":"latest"}"#).is_err());
    }

    #[test]
    fn picks_this_computers_download() {
        let body = r#"{"tag_name":"v0.1.5","assets":[
            {"name":"PresentCraft-0.1.5-windows-x64.zip","browser_download_url":"https://github.com/a/w.zip","size":5},
            {"name":"PresentCraft-0.1.5-macos-arm64.zip","browser_download_url":"https://github.com/a/m.zip","size":42},
            {"name":"PresentCraft-0.1.4-macos-arm64.zip","browser_download_url":"https://github.com/a/old.zip","size":1}
        ]}"#;
        let json = Json::parse(body).unwrap();
        assert_eq!(
            find_asset(&json, "0.1.5", "-macos-arm64.zip"),
            Some(Asset {
                name: "PresentCraft-0.1.5-macos-arm64.zip".into(),
                url: "https://github.com/a/m.zip".into(),
                size: 42,
            })
        );
        assert_eq!(find_asset(&json, "0.1.5", "-linux-x86_64.AppImage"), None);
    }

    #[test]
    fn ignores_downloads_from_other_hosts() {
        let body = r#"{"assets":[{"name":"PresentCraft-0.1.5-macos-arm64.zip","browser_download_url":"https://example.com/m.zip","size":1}]}"#;
        assert_eq!(find_asset(&Json::parse(body).unwrap(), "0.1.5", "-macos-arm64.zip"), None);
    }

    #[test]
    fn reads_signing_team() {
        let details = "Executable=/A/PresentCraft\nAuthority=Developer ID Application: X (AB12CD34EF)\nTeamIdentifier=AB12CD34EF\n";
        assert_eq!(parse_team_id(details), Some("AB12CD34EF".into()));
        assert_eq!(parse_team_id("Signature=adhoc\nTeamIdentifier=not set\n"), None);
        assert_eq!(parse_team_id("TeamIdentifier=AB\"12\n"), None);
        assert_eq!(parse_team_id("Signature=adhoc\n"), None);
        assert_eq!(
            signer_requirement("AB12CD34EF"),
            "=anchor apple generic and certificate leaf[subject.OU] = \"AB12CD34EF\""
        );
    }

    /// Talks to GitHub: `cargo test --locked -- --ignored live_release`.
    #[test]
    #[ignore]
    fn live_release_check() {
        let release = fetch_latest().expect("latest release");
        assert!(parse_version(&release.version).is_some());
        println!("latest release: {release:?}");
    }
}
