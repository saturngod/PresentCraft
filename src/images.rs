//! Loading pictures from disk or the web, applying filters, and drawing
//! them with cover / contain sizing. Web pictures download in the
//! background into a cache folder. Pictures that can't be loaded (missing
//! files, failed downloads, videos) are drawn as a soft gradient placeholder.

use crate::parse::{Filter, ImageSpec};
use egui::{pos2, Color32, ColorImage, Mesh, Painter, Rect, TextureHandle, TextureOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

type Key = (String, Option<Filter>);

#[derive(Clone)]
enum Web {
    Pending,
    Ready(PathBuf),
    Failed,
}

#[derive(Default)]
pub struct ImageCache {
    map: HashMap<Key, Option<(TextureHandle, [usize; 2])>>,
    pub doc_dir: Option<PathBuf>,
    /// Folder of the theme in use, for `/Theme/…` pictures.
    pub theme_dir: Option<PathBuf>,
    web: Arc<Mutex<HashMap<String, Web>>>,
    /// Videos playing inline (while presenting), by source.
    players: HashMap<String, crate::video::Player>,
    /// Draw the play button, time and progress bar over playing videos.
    pub show_controls: bool,
}

/// An inline video's current frame and where it is in the video.
struct VideoFrame {
    tex: egui::TextureId,
    size: [usize; 2],
    progress: f32,
    playing: bool,
    position: f32,
    duration: f32,
}

/// Undoes %XX escapes (used for spaces in file names).
fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            let hex = std::str::from_utf8(&b[i + 1..i + 3]).ok();
            if let Some(v) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

pub fn is_web(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

const VIDEO_EXTS: [&str; 4] = [".mp4", ".mov", ".m4v", ".webm"];

pub fn is_video(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    let path_part = lower.split(['?', '#']).next().unwrap_or(&lower);
    VIDEO_EXTS.iter().any(|e| path_part.ends_with(e)) || youtube_id(src).is_some()
}

/// Splits a web address into host, path and query (no fragment).
fn url_parts(src: &str) -> Option<(String, &str, &str)> {
    let rest = src
        .get(..8)
        .filter(|p| p.eq_ignore_ascii_case("https://"))
        .map(|_| &src[8..])
        .or_else(|| {
            src.get(..7)
                .filter(|p| p.eq_ignore_ascii_case("http://"))
                .map(|_| &src[7..])
        })?;
    let rest = rest.split('#').next().unwrap_or(rest);
    let (addr, query) = rest.split_once('?').unwrap_or((rest, ""));
    let (host, path) = addr.split_once('/').unwrap_or((addr, ""));
    Some((host.to_ascii_lowercase(), path, query))
}

/// The video id of a YouTube address: `youtube.com/watch?v=…`,
/// `youtu.be/…`, and the `/embed/`, `/shorts/` and `/live/` forms.
pub fn youtube_id(src: &str) -> Option<String> {
    let (host, path, query) = url_parts(src)?;
    let host = host.strip_prefix("www.").unwrap_or(&host);
    let id = match host {
        "youtu.be" => path.split('/').next(),
        "youtube.com" | "m.youtube.com" | "music.youtube.com" | "youtube-nocookie.com" => {
            if path == "watch" || path == "watch/" {
                query.split('&').find_map(|kv| kv.strip_prefix("v="))
            } else {
                ["embed/", "shorts/", "live/", "v/"]
                    .iter()
                    .find_map(|p| path.strip_prefix(p))
                    .and_then(|r| r.split('/').next())
            }
        }
        _ => None,
    }?;
    let ok = id.len() == 11
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    ok.then(|| id.to_string())
}

/// Where a YouTube address asks to start, in seconds (`t=90`, `t=1m30s`,
/// `start=90`); 0 when it doesn't say.
pub fn youtube_start(src: &str) -> f32 {
    let Some((_, _, query)) = url_parts(src) else {
        return 0.0;
    };
    let Some(v) = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("t=").or_else(|| kv.strip_prefix("start=")))
    else {
        return 0.0;
    };
    let (mut total, mut num) = (0.0_f32, String::new());
    for c in v.chars() {
        match c {
            '0'..='9' => num.push(c),
            'h' | 'm' | 's' => {
                let n: f32 = num.parse().unwrap_or(0.0);
                total += n * match c {
                    'h' => 3600.0,
                    'm' => 60.0,
                    _ => 1.0,
                };
                num.clear();
            }
            _ => return 0.0,
        }
    }
    total + num.parse::<f32>().unwrap_or(0.0)
}

/// Where a YouTube video's preview picture is cached.
fn youtube_poster_path(id: &str) -> PathBuf {
    std::env::temp_dir()
        .join("presentcraft-web")
        .join(format!("youtube-{id}.jpg"))
}

/// The cached preview picture of a YouTube video, once it has been shown.
pub fn youtube_poster_file(id: &str) -> Option<PathBuf> {
    let p = youtube_poster_path(id);
    p.exists().then_some(p)
}

/// The plain watch page of a YouTube video.
pub fn youtube_watch_url(id: &str) -> String {
    format!("https://www.youtube.com/watch?v={id}")
}

/// A video drawn on screen: where and how, so presenting can play it and
/// exports can put a real video in the same place.
#[derive(Clone, Debug)]
pub struct VideoSpot {
    /// The area the video fills.
    pub rect: Rect,
    pub src: String,
    pub filter: Option<Filter>,
    pub background: bool,
    /// Whole video shown (true) or cropped to fill the area (false).
    pub contain: bool,
    /// Alignment of the crop: 0 = left/top, 0.5 = centre, 1 = right/bottom.
    pub align: [f32; 2],
    pub opacity: f32,
}

thread_local! {
    /// Where videos were drawn during a recording pass (for click-to-play).
    static VIDEO_SPOTS: std::cell::RefCell<Option<Vec<VideoSpot>>> =
        const { std::cell::RefCell::new(None) };
}

/// Start noting where videos are drawn.
pub fn begin_video_spots() {
    VIDEO_SPOTS.with(|v| *v.borrow_mut() = Some(Vec::new()));
}

/// The videos drawn since `begin_video_spots`: their rectangles and sources.
pub fn take_video_spots() -> Vec<VideoSpot> {
    VIDEO_SPOTS.with(|v| v.borrow_mut().take().unwrap_or_default())
}

/// Runs a helper program without flashing a console window on Windows.
fn run_quiet(program: &str, args: &[&std::ffi::OsStr]) -> bool {
    let mut cmd = std::process::Command::new(program);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd.status().map(|s| s.success()).unwrap_or(false)
}

/// Grabs one frame of a video as a picture: with ffmpeg when it is
/// installed, otherwise with Quick Look on macOS.
fn make_poster(video: &Path, out: &Path) -> Result<(), String> {
    use std::ffi::OsStr;
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    if let Some(ff) = crate::video::tool("ffmpeg") {
        for seek in ["0.5", "0"] {
            let ok = run_quiet(
                ff,
                &[
                    OsStr::new("-y"),
                    OsStr::new("-loglevel"),
                    OsStr::new("error"),
                    OsStr::new("-ss"),
                    OsStr::new(seek),
                    OsStr::new("-i"),
                    video.as_os_str(),
                    OsStr::new("-frames:v"),
                    OsStr::new("1"),
                    OsStr::new("-vf"),
                    OsStr::new("scale='min(1600,iw)':-2"),
                    out.as_os_str(),
                ],
            );
            if ok && out.exists() {
                return Ok(());
            }
        }
    }
    if cfg!(target_os = "macos") {
        let dir = out.with_extension("ql");
        let _ = std::fs::create_dir_all(&dir);
        run_quiet(
            "qlmanage",
            &[
                OsStr::new("-t"),
                OsStr::new("-s"),
                OsStr::new("1600"),
                OsStr::new("-o"),
                dir.as_os_str(),
                video.as_os_str(),
            ],
        );
        let name = format!(
            "{}.png",
            video.file_name().unwrap_or_default().to_string_lossy()
        );
        let made = dir.join(name);
        let result = if made.exists() {
            std::fs::rename(&made, out).map_err(|e| e.to_string())
        } else {
            Err("no preview".into())
        };
        let _ = std::fs::remove_dir_all(&dir);
        return result;
    }
    Err("no ffmpeg".into())
}

/// The cached full still frame of a local video, if one has been made.
pub fn poster_file(video: &Path) -> Option<PathBuf> {
    let p = poster_path(video);
    p.exists().then_some(p)
}

/// Where a video's preview frame is cached (changes when the video changes).
fn poster_path(video: &Path) -> PathBuf {
    let meta = std::fs::metadata(video).ok();
    let stamp = meta
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let size = meta.map(|m| m.len()).unwrap_or(0);
    let key = format!("{}|{stamp}|{size}", video.to_string_lossy());
    std::env::temp_dir().join("presentcraft-web").join(format!(
        "poster-{:08x}{:08x}.jpg",
        hash(&key),
        hash(&key.chars().rev().collect::<String>())
    ))
}

/// Where a downloaded picture is kept (reused between runs).
fn web_cache_path(url: &str) -> PathBuf {
    let name = url.split(['?', '#']).next().unwrap_or(url);
    let ext = name
        .rsplit('/')
        .next()
        .and_then(|f| f.rsplit_once('.').map(|(_, e)| e.to_ascii_lowercase()))
        .filter(|e| e.len() <= 5 && e.chars().all(|c| c.is_ascii_alphanumeric()))
        .unwrap_or_else(|| "img".into());
    std::env::temp_dir().join("presentcraft-web").join(format!(
        "{:08x}{:08x}.{ext}",
        hash(url),
        hash(&url.chars().rev().collect::<String>())
    ))
}

/// Downloads with the system's `curl` (built into macOS, Windows 10+ and
/// nearly every Linux), so no TLS library has to be compiled in.
fn download(url: &str, dest: &Path) -> Result<(), String> {
    if let Some(dir) = dest.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = dest.with_extension("part");
    let mut cmd = std::process::Command::new("curl");
    cmd.args([
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--max-time",
        "20",
        "--max-filesize",
        "41943040",
        "--user-agent",
        "PresentCraft/0.1 (desktop presentation app)",
        "--output",
    ])
    .arg(&tmp)
    .arg("--")
    .arg(url)
    .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    {
        // Don't flash a console window.
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().map_err(|e| format!("curl: {e}"))?;
    if !out.status.success() {
        let _ = std::fs::remove_file(&tmp);
        return Err(String::from_utf8_lossy(&out.stderr).into_owned());
    }
    let bytes = std::fs::read(&tmp).map_err(|e| e.to_string())?;
    if image::load_from_memory(&bytes).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return Err("not a picture".into());
    }
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())
}

impl ImageCache {
    pub fn set_doc_dir(&mut self, dir: Option<PathBuf>) {
        if dir != self.doc_dir {
            self.doc_dir = dir;
            self.map.clear();
        }
    }

    /// Forgets loaded pictures so they are read and decoded again.
    pub fn clear_cache(&mut self) {
        self.map.clear();
    }

    pub fn resolve(&self, src: &str) -> Option<PathBuf> {
        if is_web(src) {
            return None;
        }
        let decoded = percent_decode(src);
        let src = decoded.as_str();
        let src = src.strip_prefix("file://").unwrap_or(src);
        let p = Path::new(src);
        if p.is_absolute() && p.exists() {
            return Some(p.to_path_buf());
        }
        if let Some(home) = src.strip_prefix("~/") {
            if let Some(h) = std::env::var_os("HOME") {
                return Some(PathBuf::from(h).join(home));
            }
        }
        let rel = src.trim_start_matches('/');
        // "/Theme/x.webp": the package's copy of its theme, else the theme
        // folder itself (before the theme has been saved into the package).
        if let Some(inner) = rel.strip_prefix("Theme/") {
            if let Some(c) = self
                .doc_dir
                .as_ref()
                .map(|d| d.join(rel))
                .filter(|c| c.exists())
                .or_else(|| self.theme_dir.as_ref().map(|t| t.join(inner)).filter(|c| c.exists()))
            {
                return Some(c);
            }
        }
        let dir = self.doc_dir.as_ref()?;
        let candidate = dir.join(rel);
        if candidate.exists() {
            return Some(candidate);
        }
        // "/assets/x.png" also looks next to the document.
        if let Some(name) = Path::new(rel).file_name() {
            let c = dir.join(name);
            if c.exists() {
                return Some(c);
            }
        }
        None
    }

    /// Plays or pauses a video inline. `Err` when it can't play here (no
    /// ffmpeg, web video or missing file), so the caller can fall back.
    pub fn toggle_video(&mut self, src: &str, filter: Option<Filter>) -> Result<(), ()> {
        if let Some(p) = self.players.get_mut(src) {
            if p.playing {
                p.pause();
            } else {
                p.resume().ok_or(())?;
            }
            return Ok(());
        }
        if is_web(src) || !crate::video::available() {
            return Err(());
        }
        let path = self.resolve(src).ok_or(())?;
        let player = crate::video::Player::start(&path, filter, false, false).ok_or(())?;
        self.players.insert(src.to_string(), player);
        Ok(())
    }

    /// Starts a background video (silent, looping) if it isn't running yet.
    pub fn autoplay(&mut self, src: &str, filter: Option<Filter>) {
        if self.players.contains_key(src) || is_web(src) || !crate::video::available() {
            return;
        }
        if let Some(path) = self.resolve(src) {
            if let Some(p) = crate::video::Player::start(&path, filter, true, true) {
                self.players.insert(src.to_string(), p);
            }
        }
    }

    /// Stops every inline video (leaving a slide, ending a presentation).
    pub fn stop_videos(&mut self) {
        self.players.clear();
    }

    /// A video that was asked to play and hasn't shown a frame yet.
    fn video_loading(&self, src: &str) -> bool {
        self.players
            .get(src)
            .is_some_and(|p| p.texture.is_none() && !p.looping)
    }

    /// Jumps to `fraction` (0–1) of a video that is playing or paused.
    pub fn seek_video(&mut self, src: &str, fraction: f32) {
        if let Some(p) = self.players.get_mut(src) {
            p.seek(fraction);
        }
    }

    /// Whether a video has a player (playing or paused) right now.
    pub fn has_player(&self, src: &str) -> bool {
        self.players.contains_key(src)
    }

    /// Moves playing videos to their current frame; true while any plays.
    pub fn tick_videos(&mut self, ctx: &egui::Context) -> bool {
        let mut any = false;
        for (src, p) in self.players.iter_mut() {
            any |= p.tick();
            if let Some(bytes) = p.fresh.take() {
                let img = ColorImage::from_rgba_unmultiplied(p.size, &bytes);
                match &mut p.texture {
                    Some(t) => t.set(img, TextureOptions::LINEAR),
                    None => {
                        p.texture = Some(ctx.load_texture(
                            format!("video:{src}"),
                            img,
                            TextureOptions::LINEAR,
                        ))
                    }
                }
            }
        }
        any
    }

    /// The current frame of an inline video: texture, size, progress, playing.
    fn video_frame(&self, src: &str) -> Option<VideoFrame> {
        let p = self.players.get(src)?;
        let t = p.texture.as_ref()?;
        Some(VideoFrame {
            tex: t.id(),
            size: p.size,
            progress: p.progress(),
            playing: p.playing,
            position: p.position().min(p.info.duration.max(0.0)),
            duration: p.info.duration,
        })
    }

    /// A YouTube video's preview picture: downloaded the first time.
    fn youtube_poster_state(&self, ctx: &egui::Context, id: &str) -> Web {
        let key = format!("poster:youtube:{id}");
        let mut w = match self.web.lock() {
            Ok(w) => w,
            Err(_) => return Web::Failed,
        };
        if let Some(state) = w.get(&key) {
            return state.clone();
        }
        let out = youtube_poster_path(id);
        if out.exists() {
            w.insert(key, Web::Ready(out.clone()));
            return Web::Ready(out);
        }
        w.insert(key.clone(), Web::Pending);
        let (web, ctx, id) = (self.web.clone(), ctx.clone(), id.to_string());
        std::thread::spawn(move || {
            // The large picture isn't there for every video; the medium one
            // is, and both are 16:9 without black bars.
            let got = ["maxresdefault", "mqdefault"].iter().any(|name| {
                download(&format!("https://i.ytimg.com/vi/{id}/{name}.jpg"), &out).is_ok()
            });
            let state = if got { Web::Ready(out) } else { Web::Failed };
            if let Ok(mut w) = web.lock() {
                w.insert(key, state);
            }
            ctx.request_repaint();
        });
        Web::Pending
    }

    /// Number of web pictures still downloading.
    pub fn pending(&self) -> usize {
        self.web
            .lock()
            .map(|w| w.values().filter(|s| matches!(s, Web::Pending)).count())
            .unwrap_or(0)
    }

    /// The local file for a web picture: starts a download the first time.
    fn web_state(&self, ctx: &egui::Context, url: &str) -> Web {
        let mut w = match self.web.lock() {
            Ok(w) => w,
            Err(_) => return Web::Failed,
        };
        if let Some(state) = w.get(url) {
            return state.clone();
        }
        let lower = url.to_ascii_lowercase();
        let path_part = lower.split(['?', '#']).next().unwrap_or(&lower);
        if VIDEO_EXTS.iter().any(|e| path_part.ends_with(e)) {
            w.insert(url.to_string(), Web::Failed);
            return Web::Failed;
        }
        let dest = web_cache_path(url);
        if dest.exists() {
            w.insert(url.to_string(), Web::Ready(dest.clone()));
            return Web::Ready(dest);
        }
        w.insert(url.to_string(), Web::Pending);
        let (web, ctx, url) = (self.web.clone(), ctx.clone(), url.to_string());
        std::thread::spawn(move || {
            let state = match download(&url, &dest) {
                Ok(()) => Web::Ready(dest),
                Err(_) => Web::Failed,
            };
            if let Ok(mut w) = web.lock() {
                w.insert(url, state);
            }
            ctx.request_repaint();
        });
        Web::Pending
    }

    /// A video's preview frame: made in the background the first time.
    fn poster_state(&self, ctx: &egui::Context, video: &Path) -> Web {
        let key = format!("poster:{}", video.to_string_lossy());
        let mut w = match self.web.lock() {
            Ok(w) => w,
            Err(_) => return Web::Failed,
        };
        if let Some(state) = w.get(&key) {
            return state.clone();
        }
        let out = poster_path(video);
        if out.exists() {
            w.insert(key, Web::Ready(out.clone()));
            return Web::Ready(out);
        }
        w.insert(key.clone(), Web::Pending);
        let (web, ctx, video) = (self.web.clone(), ctx.clone(), video.to_path_buf());
        std::thread::spawn(move || {
            let state = match make_poster(&video, &out) {
                Ok(()) => Web::Ready(out),
                Err(_) => Web::Failed,
            };
            if let Ok(mut w) = web.lock() {
                w.insert(key, state);
            }
            ctx.request_repaint();
        });
        Web::Pending
    }

    pub fn get(
        &mut self,
        ctx: &egui::Context,
        src: &str,
        filter: Option<Filter>,
    ) -> Option<(egui::TextureId, [usize; 2])> {
        let key = (src.to_string(), filter);
        if !self.map.contains_key(&key) {
            let path = if let Some(id) = youtube_id(src) {
                match self.youtube_poster_state(ctx, &id) {
                    Web::Pending => return None,
                    Web::Ready(p) => Some(p),
                    Web::Failed => None,
                }
            } else if is_video(src) {
                match self.resolve(src) {
                    Some(v) => match self.poster_state(ctx, &v) {
                        Web::Pending => return None,
                        Web::Ready(p) => Some(p),
                        Web::Failed => None,
                    },
                    None => None,
                }
            } else if is_web(src) {
                match self.web_state(ctx, src) {
                    // not cached yet: try again on a later frame
                    Web::Pending => return None,
                    Web::Ready(p) => Some(p),
                    Web::Failed => None,
                }
            } else {
                self.resolve(src)
            };
            let loaded = path.and_then(|p| load(ctx, &p, filter));
            self.map.insert(key.clone(), loaded);
        }
        self.map
            .get(&key)
            .and_then(|o| o.as_ref())
            .map(|(t, s)| (t.id(), *s))
    }
}

fn load(
    ctx: &egui::Context,
    path: &Path,
    filter: Option<Filter>,
) -> Option<(TextureHandle, [usize; 2])> {
    let img = image::ImageReader::open(path)
        .ok()?
        .with_guessed_format()
        .ok()?
        .decode()
        .ok()?;
    // Keep textures a sensible size.
    let img = if img.width() > 2400 || img.height() > 2400 {
        img.thumbnail(2400, 2400)
    } else {
        img
    };
    let img = if filter == Some(Filter::Blur) {
        let small = img.thumbnail((img.width() / 12).max(8), (img.height() / 12).max(8));
        small.resize_exact(
            img.width() / 4 + 1,
            img.height() / 4 + 1,
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let mut rgba = img.to_rgba8();
    if matches!(
        filter,
        Some(Filter::Grayscale | Filter::Sepia | Filter::Darken | Filter::Lighten)
    ) {
        for p in rgba.pixels_mut() {
            let [r, g, b, a] = p.0;
            let [r, g, b] = filter_rgb([r, g, b], filter);
            p.0 = [r, g, b, a];
        }
    }
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    let tex = ctx.load_texture(
        format!("{}#{:?}", path.to_string_lossy(), filter),
        color,
        TextureOptions::LINEAR,
    );
    Some((tex, size))
}

/// One pixel through a colour filter (blur is applied to the whole picture).
fn filter_rgb([r, g, b]: [u8; 3], filter: Option<Filter>) -> [u8; 3] {
    let (rf, gf, bf) = (r as f32, g as f32, b as f32);
    let out = match filter {
        Some(Filter::Grayscale) => {
            let l = 0.299 * rf + 0.587 * gf + 0.114 * bf;
            [l, l, l]
        }
        Some(Filter::Sepia) => [
            0.393 * rf + 0.769 * gf + 0.189 * bf,
            0.349 * rf + 0.686 * gf + 0.168 * bf,
            0.272 * rf + 0.534 * gf + 0.131 * bf,
        ],
        Some(Filter::Darken) => [rf * 0.55, gf * 0.55, bf * 0.55],
        Some(Filter::Lighten) => [
            rf + (255.0 - rf) * 0.45,
            gf + (255.0 - gf) * 0.45,
            bf + (255.0 - bf) * 0.45,
        ],
        _ => [rf, gf, bf],
    };
    [
        out[0].min(255.0) as u8,
        out[1].min(255.0) as u8,
        out[2].min(255.0) as u8,
    ]
}

fn filter_color(c: Color32, filter: Option<Filter>) -> Color32 {
    let [r, g, b] = filter_rgb([c.r(), c.g(), c.b()], filter);
    Color32::from_rgb(r, g, b)
}

/// Draws a picture into `rect`. Returns the rect actually covered.
pub fn draw_image(
    painter: &Painter,
    rect: Rect,
    spec: &ImageSpec,
    cache: &mut ImageCache,
    accents: &[Color32; 6],
    contain_override: Option<bool>,
) -> Rect {
    let alpha = spec.opacity.clamp(0.0, 1.0);
    let contain = contain_override.unwrap_or(spec.contain);
    let ctx = painter.ctx().clone();
    // A video playing inline shows its current frame.
    if let Some(f) = cache.video_frame(&spec.src) {
        let drawn = paint_texture(painter, rect, f.tex, f.size, spec, contain, alpha);
        note_video_spot(drawn, spec, contain);
        if !spec.background {
            if !f.playing {
                video_badge(painter, drawn, alpha);
            }
            if cache.show_controls {
                video_controls(painter, drawn, &f);
            }
        }
        // The controls sit where the caption would be.
        if !cache.show_controls || spec.background {
            draw_caption(painter, drawn, spec);
        }
        return drawn;
    }
    let loading = cache.video_loading(&spec.src);
    if let Some((tex, size)) = cache.get(&ctx, &spec.src, spec.filter) {
        let drawn = paint_texture(painter, rect, tex, size, spec, contain, alpha);
        if is_video(&spec.src) {
            note_video_spot(drawn, spec, contain);
            if loading {
                video_spinner(painter, drawn, alpha);
            } else if !spec.background {
                video_badge(painter, drawn, alpha);
            }
        }
        draw_caption(painter, drawn, spec);
        return drawn;
    }
    let video = is_video(&spec.src);
    placeholder(
        painter,
        rect,
        &spec.src,
        accents,
        alpha,
        !spec.background && !video,
        spec.filter,
    );
    if video {
        note_video_spot(rect, spec, contain);
        if loading {
            video_spinner(painter, rect, alpha);
        } else if !spec.background {
            video_badge(painter, rect, alpha);
        }
    }
    draw_caption(painter, rect, spec);
    rect
}

/// Draws a texture into `rect` with cover / contain sizing and alignment.
fn paint_texture(
    painter: &Painter,
    rect: Rect,
    tex: egui::TextureId,
    [iw, ih]: [usize; 2],
    spec: &ImageSpec,
    contain: bool,
    alpha: f32,
) -> Rect {
    let (iw, ih) = (iw as f32, ih as f32);
    let tint = Color32::WHITE.gamma_multiply(alpha);
    let mut mesh = Mesh::with_texture(tex);
    let drawn;
    if contain {
        let s = (rect.width() / iw).min(rect.height() / ih);
        let (w, h) = (iw * s, ih * s);
        let x = rect.left() + (rect.width() - w) * spec.x;
        let y = rect.top() + (rect.height() - h) * spec.y;
        drawn = Rect::from_min_size(pos2(x, y), egui::vec2(w, h));
        mesh.add_rect_with_uv(
            drawn,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            tint,
        );
    } else {
        let s = (rect.width() / iw).max(rect.height() / ih);
        let uw = (rect.width() / (iw * s)).min(1.0);
        let vh = (rect.height() / (ih * s)).min(1.0);
        let u0 = (1.0 - uw) * spec.x;
        let v0 = (1.0 - vh) * spec.y;
        drawn = rect;
        mesh.add_rect_with_uv(
            rect,
            Rect::from_min_max(pos2(u0, v0), pos2(u0 + uw, v0 + vh)),
            tint,
        );
    }
    painter.add(mesh);
    drawn
}

/// Notes where a video was drawn, for click-to-play.
fn note_video_spot(rect: Rect, spec: &ImageSpec, contain: bool) {
    VIDEO_SPOTS.with(|v| {
        if let Some(list) = v.borrow_mut().as_mut() {
            list.push(VideoSpot {
                rect,
                src: spec.src.clone(),
                filter: spec.filter,
                background: spec.background,
                contain,
                align: [spec.x, spec.y],
                opacity: spec.opacity.clamp(0.0, 1.0),
            });
        }
    });
}

/// What a click on a video's controls means.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlHit {
    /// The play / pause button.
    Toggle,
    /// A place on the progress bar, as a fraction of the video.
    Seek(f32),
}

/// Where the controls sit over a video drawn in `rect`: the strip along
/// the bottom, the play / pause button, and the progress bar.
fn control_layout(rect: Rect, duration: f32) -> (Rect, Rect, Rect) {
    let h = (rect.height() * 0.085).clamp(16.0, 52.0).min(rect.height());
    let strip = Rect::from_min_max(pos2(rect.left(), rect.bottom() - h), rect.max);
    let button = Rect::from_min_size(strip.min, egui::vec2(h * 1.15, h));
    // Room for "0:00 / 0:00" (wider once there are hours).
    let time_w = h * if duration >= 3600.0 { 4.4 } else { 3.2 };
    let left = (button.right() + time_w).min(strip.right() - h * 0.5);
    let track = Rect::from_min_max(
        pos2(left, strip.top()),
        pos2(strip.right() - h * 0.5, strip.bottom()),
    );
    (strip, button, track)
}

/// Which control of a video drawn in `rect` is at `pos`, if any.
pub fn control_hit(rect: Rect, pos: egui::Pos2) -> Option<ControlHit> {
    let (strip, _, track) = control_layout(rect, 0.0);
    if !strip.contains(pos) {
        return None;
    }
    // The time's exact width doesn't matter here: anything right of the
    // button and the shortest time label seeks.
    if pos.x < track.left() || track.width() <= 0.0 {
        return Some(ControlHit::Toggle);
    }
    Some(ControlHit::Seek(
        ((pos.x - track.left()) / track.width()).clamp(0.0, 1.0),
    ))
}

/// Minutes and seconds (hours when needed): `1:05`, `1:02:03`.
fn clock(seconds: f32) -> String {
    let t = seconds.max(0.0) as u32;
    let (h, m, s) = (t / 3600, t / 60 % 60, t % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

/// Play / pause button, time and progress bar along a video's bottom edge.
fn video_controls(painter: &Painter, rect: Rect, f: &VideoFrame) {
    let (strip, button, track) = control_layout(rect, f.duration);
    let h = strip.height();
    let white = Color32::WHITE.gamma_multiply(0.95);
    painter.rect_filled(strip, 0.0, Color32::from_black_alpha(140));
    let c = button.center();
    let g = h * 0.2;
    if f.playing {
        for dx in [-g * 0.75, g * 0.75] {
            painter.rect_filled(
                Rect::from_center_size(pos2(c.x + dx, c.y), egui::vec2(g * 0.6, g * 2.0)),
                g * 0.12,
                white,
            );
        }
    } else {
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(c.x - g * 0.75, c.y - g),
                pos2(c.x + g, c.y),
                pos2(c.x - g * 0.75, c.y + g),
            ],
            white,
            egui::Stroke::NONE,
        ));
    }
    if f.duration > 0.0 && track.left() - button.right() > h {
        painter.text(
            pos2(button.right(), c.y),
            egui::Align2::LEFT_CENTER,
            format!("{} / {}", clock(f.position), clock(f.duration)),
            egui::FontId::new(h * 0.4, crate::theme::family(false, false, false)),
            white,
        );
    }
    if track.width() > h {
        let bar_h = (h * 0.1).max(2.0);
        let line = Rect::from_center_size(track.center(), egui::vec2(track.width(), bar_h));
        painter.rect_filled(line, bar_h / 2.0, Color32::from_white_alpha(70));
        let x = line.left() + line.width() * f.progress;
        let mut done = line;
        done.max.x = x;
        painter.rect_filled(done, bar_h / 2.0, white);
        painter.circle_filled(pos2(x, line.center().y), bar_h * 1.6, white);
    }
}

/// Turning dots over a video that is about to start.
fn video_spinner(painter: &Painter, rect: Rect, alpha: f32) {
    let r = (rect.width().min(rect.height()) * 0.09).clamp(5.0, 44.0);
    let c = rect.center();
    painter.circle_filled(c, r, Color32::from_black_alpha((150.0 * alpha) as u8));
    let ctx = painter.ctx();
    let turn = (ctx.input(|i| i.time) * 1.25).fract() as f32;
    const DOTS: usize = 8;
    for k in 0..DOTS {
        let a = k as f32 / DOTS as f32;
        let ang = a * std::f32::consts::TAU;
        let age = (turn - a).rem_euclid(1.0);
        painter.circle_filled(
            c + egui::vec2(ang.cos(), ang.sin()) * r * 0.52,
            r * 0.09,
            Color32::WHITE.gamma_multiply((1.0 - age * 0.85) * alpha),
        );
    }
    ctx.request_repaint();
}

/// Round play button over a video.
fn video_badge(painter: &Painter, rect: Rect, alpha: f32) {
    let r = (rect.width().min(rect.height()) * 0.09).clamp(5.0, 44.0);
    let c = rect.center();
    painter.circle_filled(c, r, Color32::from_black_alpha((150.0 * alpha) as u8));
    painter.circle_stroke(
        c,
        r,
        egui::Stroke::new(
            (r * 0.06).max(0.8),
            Color32::WHITE.gamma_multiply(0.85 * alpha),
        ),
    );
    let t = r * 0.42;
    painter.add(egui::Shape::convex_polygon(
        vec![
            pos2(c.x - t * 0.7, c.y - t),
            pos2(c.x + t * 1.0, c.y),
            pos2(c.x - t * 0.7, c.y + t),
        ],
        Color32::WHITE.gamma_multiply(alpha),
        egui::Stroke::NONE,
    ));
}

fn draw_caption(painter: &Painter, rect: Rect, spec: &ImageSpec) {
    if let Some(c) = &spec.caption {
        let size = (rect.height() * 0.06).clamp(9.0, 28.0);
        let g = painter.layout(
            c.clone(),
            egui::FontId::new(size, crate::theme::family(false, false, false)),
            Color32::WHITE,
            rect.width() - size * 1.6,
        );
        let h = g.size().y + size * 0.8;
        let strip = Rect::from_min_max(pos2(rect.left(), rect.bottom() - h), rect.max);
        painter.rect_filled(strip, 0.0, Color32::from_black_alpha(120));
        painter.galley(
            pos2(rect.left() + size * 0.8, strip.top() + size * 0.4),
            g,
            Color32::WHITE,
        );
    }
}

fn hash(s: &str) -> u32 {
    s.bytes()
        .fold(2166136261u32, |h, b| (h ^ b as u32).wrapping_mul(16777619))
}

/// A soft four-corner gradient standing in for a picture that can't be shown.
pub fn placeholder(
    painter: &Painter,
    rect: Rect,
    src: &str,
    accents: &[Color32; 6],
    alpha: f32,
    icon: bool,
    filter: Option<Filter>,
) {
    let h = hash(src) as usize;
    let pick = |k: usize| {
        let c = accents[(h / (k + 1) + k * 2) % 6];
        let soft = Color32::from_rgb(
            ((c.r() as u16 + 255) / 2) as u8,
            ((c.g() as u16 + 255) / 2) as u8,
            ((c.b() as u16 + 255) / 2) as u8,
        );
        filter_color(soft, filter).gamma_multiply(alpha)
    };
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), pick(0));
    mesh.colored_vertex(rect.right_top(), pick(1));
    mesh.colored_vertex(rect.right_bottom(), pick(2));
    mesh.colored_vertex(rect.left_bottom(), pick(3));
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(mesh);
    // simple "mountain" pictogram
    let s = rect.width().min(rect.height()) * 0.18;
    if icon && s > 6.0 {
        let c = rect.center();
        let col = Color32::WHITE.gamma_multiply(0.75 * alpha);
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(c.x - s, c.y + s * 0.6),
                pos2(c.x - s * 0.25, c.y - s * 0.4),
                pos2(c.x + s * 0.35, c.y + s * 0.6),
            ],
            col,
            egui::Stroke::NONE,
        ));
        painter.add(egui::Shape::convex_polygon(
            vec![
                pos2(c.x + s * 0.05, c.y + s * 0.6),
                pos2(c.x + s * 0.5, c.y),
                pos2(c.x + s, c.y + s * 0.6),
            ],
            col,
            egui::Stroke::NONE,
        ));
        painter.circle_filled(pos2(c.x + s * 0.55, c.y - s * 0.45), s * 0.16, col);
        let name = src.rsplit('/').next().unwrap_or(src);
        let fs = (s * 0.32).clamp(8.0, 18.0);
        painter.text(
            pos2(c.x, c.y + s * 0.95),
            egui::Align2::CENTER_TOP,
            name,
            egui::FontId::proportional(fs),
            Color32::WHITE.gamma_multiply(0.85 * alpha),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_change_colours() {
        assert_eq!(
            filter_rgb([200, 100, 50], Some(Filter::Grayscale)),
            [124, 124, 124]
        );
        assert_eq!(filter_rgb([200, 100, 50], None), [200, 100, 50]);
        assert_eq!(
            filter_rgb([100, 100, 100], Some(Filter::Darken)),
            [55, 55, 55]
        );
    }

    #[test]
    fn web_cache_names_keep_the_picture_type() {
        let p = web_cache_path("https://example.com/a/photo.JPG?size=2");
        assert_eq!(p.extension().unwrap(), "jpg");
        assert_ne!(p, web_cache_path("https://example.com/a/other.jpg"));
        assert_eq!(
            web_cache_path("https://x.com/pic").extension().unwrap(),
            "img"
        );
    }
}

#[cfg(test)]
mod path_tests {
    use super::*;

    #[test]
    fn decodes_spaces_and_spots_videos() {
        assert_eq!(percent_decode("./My%20Clip.mp4"), "./My Clip.mp4");
        assert_eq!(percent_decode("100%"), "100%");
        assert!(is_video("./sagarwar.MP4"));
        assert!(!is_video("./sagarwar.jpg"));
    }

    #[test]
    fn spots_youtube_videos() {
        let id = Some("Eu4vnYbKvCI".to_string());
        assert_eq!(youtube_id("https://www.youtube.com/watch?v=Eu4vnYbKvCI"), id);
        assert_eq!(youtube_id("https://youtube.com/watch?t=5&v=Eu4vnYbKvCI&list=x"), id);
        assert_eq!(youtube_id("https://youtu.be/Eu4vnYbKvCI?si=abc"), id);
        assert_eq!(youtube_id("https://www.youtube.com/embed/Eu4vnYbKvCI"), id);
        assert_eq!(youtube_id("https://www.youtube.com/shorts/Eu4vnYbKvCI"), id);
        assert_eq!(youtube_id("HTTPS://M.YouTube.com/watch?v=Eu4vnYbKvCI#x"), id);
        assert_eq!(youtube_id("https://www.youtube.com/watch?v=short"), None);
        assert_eq!(youtube_id("https://www.youtube.com/@channel"), None);
        assert_eq!(youtube_id("https://example.com/watch?v=Eu4vnYbKvCI"), None);
        assert_eq!(youtube_id("/assets/watch?v=Eu4vnYbKvCI"), None);
        assert!(is_video("https://www.youtube.com/watch?v=Eu4vnYbKvCI"));
        assert!(!is_video("https://example.com/picture.png"));
    }

    #[test]
    fn reads_youtube_start_times() {
        assert_eq!(youtube_start("https://youtu.be/Eu4vnYbKvCI?t=90"), 90.0);
        assert_eq!(youtube_start("https://youtu.be/Eu4vnYbKvCI?t=1m30s"), 90.0);
        assert_eq!(
            youtube_start("https://www.youtube.com/watch?v=Eu4vnYbKvCI&start=12"),
            12.0
        );
        assert_eq!(youtube_start("https://youtu.be/Eu4vnYbKvCI?t=1h2m3s"), 3723.0);
        assert_eq!(youtube_start("https://youtu.be/Eu4vnYbKvCI"), 0.0);
    }

    #[test]
    fn finds_video_controls() {
        let r = Rect::from_min_size(pos2(0.0, 0.0), egui::vec2(800.0, 450.0));
        assert_eq!(control_hit(r, pos2(400.0, 200.0)), None);
        assert_eq!(control_hit(r, pos2(10.0, 440.0)), Some(ControlHit::Toggle));
        match control_hit(r, pos2(780.0, 440.0)) {
            Some(ControlHit::Seek(f)) => assert!(f > 0.9),
            other => panic!("expected a seek, got {other:?}"),
        }
        assert_eq!(clock(65.0), "1:05");
        assert_eq!(clock(3723.0), "1:02:03");
    }

    #[test]
    fn resolves_dot_slash_next_to_document() {
        let dir = std::env::temp_dir().join("presentcraft-resolve-test");
        let _ = std::fs::create_dir_all(&dir);
        std::fs::write(dir.join("a b.jpg"), b"x").unwrap();
        let mut c = ImageCache::default();
        c.set_doc_dir(Some(dir.clone()));
        assert_eq!(c.resolve("./a%20b.jpg"), Some(dir.join("./a b.jpg")));
    }
}
