//! Inline video playback with the system's ffmpeg tools.
//!
//! `ffmpeg` reports the size, frame rate, length and whether there is sound,
//! then decodes frames (already scaled and filtered) into a pipe that a
//! background thread reads; the window shows the frame that matches the
//! time since playback started. Sound plays alongside: with `ffplay` when
//! installed, otherwise ffmpeg writes a WAV file that the system plays.
//! Pausing stops both processes; resuming starts them again from the paused
//! time. Without ffmpeg, callers fall back to the system's video player.
//!
//! YouTube videos don't come through here: they play in YouTube's own
//! player (see `webvideo`).

use crate::parse::Filter;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, Receiver, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

/// Longest side of decoded frames (keeps the pipe and textures light).
const MAX_SIDE: usize = 1280;

fn quiet(cmd: &mut Command) -> &mut Command {
    cmd.stdin(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        // Don't flash console windows.
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Folders where release packages put their own ffmpeg tools:
/// `SLIDE_WRITER_FFMPEG_DIR` if set, then next to the program
/// (`ffmpeg/` beside the executable, the macOS bundle's
/// `Contents/Resources/ffmpeg`, or `../lib/presentcraft/ffmpeg` on Linux).
fn bundled_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("SLIDE_WRITER_FFMPEG_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Some(exe_dir) = std::env::current_exe()
        .ok()
        .and_then(|e| e.canonicalize().ok().or(Some(e)))
        .and_then(|e| e.parent().map(Path::to_path_buf))
    {
        dirs.push(exe_dir.join("ffmpeg"));
        dirs.push(exe_dir.join("..").join("Resources").join("ffmpeg"));
        dirs.push(
            exe_dir
                .join("..")
                .join("lib")
                .join("presentcraft")
                .join("ffmpeg"),
        );
        dirs.push(exe_dir);
    }
    dirs
}

/// Finds a working copy of an ffmpeg tool (`ffmpeg`, `ffprobe`, `ffplay`).
/// Apps started from the Finder don't see Homebrew's folder on PATH, so
/// common install places are tried too.
pub fn tool(name: &'static str) -> Option<&'static str> {
    static FOUND: OnceLock<
        std::sync::Mutex<std::collections::HashMap<&'static str, Option<&'static str>>>,
    > = OnceLock::new();
    let map = FOUND.get_or_init(Default::default);
    if let Some(hit) = map.lock().ok().and_then(|m| m.get(name).copied()) {
        return hit;
    }
    let exe_name = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    // Copies bundled with the app come first, then the system's.
    let mut candidates: Vec<String> = bundled_dirs()
        .into_iter()
        .map(|d| d.join(&exe_name).to_string_lossy().into_owned())
        .collect();
    candidates.extend([
        name.to_string(),
        format!("/opt/homebrew/bin/{name}"),
        format!("/usr/local/bin/{name}"),
        format!("/usr/bin/{name}"),
    ]);
    let found = candidates.into_iter().find(|c| {
        quiet(Command::new(c).arg("-version").stdout(Stdio::null()))
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    });
    let found: Option<&'static str> = found.map(|s| &*Box::leak(s.into_boxed_str()));
    if let Ok(mut m) = map.lock() {
        m.insert(name, found);
    }
    found
}

/// What ffprobe says about a video.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Info {
    pub width: usize,
    pub height: usize,
    pub fps: f32,
    pub duration: f32,
    pub has_audio: bool,
}

/// Reads size, frame rate, length, sound and rotation from what
/// `ffmpeg -i video` prints (so ffprobe isn't needed).
fn parse_ffmpeg_info(text: &str) -> Option<Info> {
    let mut duration = 0.0;
    let mut size: Option<(usize, usize)> = None;
    let mut fps = 0.0_f32;
    let mut tbr = 0.0_f32;
    let mut has_audio = false;
    let mut rotated = false;
    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Duration:") {
            let hms = rest.trim().split(',').next().unwrap_or("");
            let parts: Vec<f32> = hms
                .split(':')
                .filter_map(|p| p.trim().parse().ok())
                .collect();
            if parts.len() == 3 {
                duration = parts[0] * 3600.0 + parts[1] * 60.0 + parts[2];
            }
        } else if t.starts_with("Stream #") && t.contains("Video:") && size.is_none() {
            let words: Vec<&str> = t
                .split(|c: char| c == ',' || c.is_whitespace())
                .filter(|w| !w.is_empty())
                .collect();
            for (i, w) in words.iter().enumerate() {
                if let Some((a, b)) = w.split_once('x') {
                    if let (Ok(a), Ok(b)) = (a.parse::<usize>(), b.parse::<usize>()) {
                        if a > 0 && b > 0 && size.is_none() {
                            size = Some((a, b));
                        }
                    }
                }
                let num = |k: usize| words.get(k).and_then(|v| v.parse::<f32>().ok());
                if *w == "fps" {
                    fps = i.checked_sub(1).and_then(num).unwrap_or(fps);
                }
                if *w == "tbr" {
                    tbr = i.checked_sub(1).and_then(num).unwrap_or(tbr);
                }
            }
        } else if t.starts_with("Stream #") && t.contains("Audio:") {
            has_audio = true;
        } else if t.contains("rotation of") || t.starts_with("rotate") {
            let n: String = t
                .chars()
                .skip_while(|c| !(c.is_ascii_digit() || *c == '-'))
                .take_while(|c| c.is_ascii_digit() || *c == '-' || *c == '.')
                .collect();
            if let Ok(deg) = n.parse::<f32>() {
                let q = ((deg.abs() / 90.0).round() as i32) % 2;
                rotated = q == 1;
            }
        }
    }
    let (mut width, mut height) = size?;
    // ffmpeg turns rotated (phone) videos upright, so the frames come out swapped.
    if rotated {
        std::mem::swap(&mut width, &mut height);
    }
    if !(1.0..=240.0).contains(&fps) {
        fps = if (1.0..=240.0).contains(&tbr) {
            tbr
        } else {
            30.0
        };
    }
    Some(Info {
        width,
        height,
        fps,
        duration,
        has_audio,
    })
}

pub fn probe(path: &Path) -> Option<Info> {
    let ffmpeg = tool("ffmpeg")?;
    let out = quiet(Command::new(ffmpeg).args(["-hide_banner", "-i"]))
        .arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .ok()?;
    parse_ffmpeg_info(&String::from_utf8_lossy(&out.stderr))
}

/// Output frame size: fits within MAX_SIDE, even numbers (ffmpeg needs them).
fn out_size(w: usize, h: usize) -> (usize, usize) {
    let s = (MAX_SIDE as f32 / w.max(h) as f32).min(1.0);
    let even = |v: f32| ((v / 2.0).round() as usize * 2).max(2);
    (even(w as f32 * s), even(h as f32 * s))
}

/// ffmpeg's version of the picture filters.
fn filter_chain(f: Option<Filter>) -> &'static str {
    match f {
        Some(Filter::Grayscale) => "hue=s=0,",
        Some(Filter::Sepia) => {
            "colorchannelmixer=.393:.769:.189:0:.349:.686:.168:0:.272:.534:.131,"
        }
        Some(Filter::Darken) => "colorlevels=romax=0.55:gomax=0.55:bomax=0.55,",
        Some(Filter::Lighten) => "colorlevels=romin=0.45:gomin=0.45:bomin=0.45,",
        Some(Filter::Blur) => "gblur=sigma=14,",
        None => "",
    }
}

/// Sound for a playing video: an `ffplay` process, or a WAV file being
/// prepared and then played by the system's own player.
struct Sound {
    child: Arc<Mutex<Option<Child>>>,
    cancelled: Arc<AtomicBool>,
}

impl Sound {
    fn stop(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        if let Ok(mut c) = self.child.lock() {
            if let Some(mut c) = c.take() {
                let _ = c.kill();
                let _ = c.wait();
            }
        }
    }
}

/// The command that plays a WAV file with what the system already has.
fn wav_player(wav: &Path) -> Option<Command> {
    if cfg!(target_os = "macos") {
        let mut c = Command::new("/usr/bin/afplay");
        c.arg(wav);
        return Some(c);
    }
    if cfg!(windows) {
        // .NET's SoundPlayer ships with Windows PowerShell.
        let p = wav.to_string_lossy().replace('\'', "''");
        let mut c = Command::new("powershell");
        c.args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &format!("(New-Object System.Media.SoundPlayer '{p}').PlaySync()"),
        ]);
        return Some(c);
    }
    for player in ["pw-play", "paplay", "aplay"] {
        let found = quiet(Command::new(player).arg("--version").stdout(Stdio::null()))
            .status()
            .is_ok();
        if found {
            let mut c = Command::new(player);
            c.arg(wav);
            return Some(c);
        }
    }
    None
}

/// Starts the sound from `seek`: ffplay when installed, otherwise a WAV
/// made by ffmpeg (in the background) and played by the system.
fn start_sound(video: &Path, seek: &str) -> Option<Sound> {
    let sound = Sound {
        child: Arc::new(Mutex::new(None)),
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    if let Some(ffplay) = tool("ffplay") {
        let c = quiet(
            Command::new(ffplay)
                .args(["-nodisp", "-autoexit", "-loglevel", "quiet", "-ss", seek])
                .arg(video)
                .stdout(Stdio::null()),
        )
        .spawn()
        .ok()?;
        *sound.child.lock().ok()? = Some(c);
        return Some(sound);
    }
    let ffmpeg = tool("ffmpeg")?;
    let dir = std::env::temp_dir().join("presentcraft-web");
    std::fs::create_dir_all(&dir).ok()?;
    let wav = dir.join(format!("sound-{}.wav", std::process::id()));
    let (child, cancelled) = (sound.child.clone(), sound.cancelled.clone());
    let (video, seek) = (video.to_path_buf(), seek.to_string());
    std::thread::spawn(move || {
        let made =
            quiet(Command::new(ffmpeg).args(["-y", "-loglevel", "error", "-ss", &seek, "-i"]))
                .arg(&video)
                .args(["-vn", "-ac", "2", "-ar", "44100", "-c:a", "pcm_s16le"])
                .arg(&wav)
                .stdout(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false);
        if !made || cancelled.load(Ordering::SeqCst) {
            return;
        }
        let Some(mut cmd) = wav_player(&wav) else {
            return;
        };
        if let Ok(c) = quiet(cmd.stdout(Stdio::null())).spawn() {
            if let Ok(mut slot) = child.lock() {
                if cancelled.load(Ordering::SeqCst) {
                    let mut c = c;
                    let _ = c.kill();
                    let _ = c.wait();
                } else {
                    *slot = Some(c);
                }
            }
        }
    });
    Some(sound)
}

/// A video being played.
pub struct Player {
    pub path: PathBuf,
    pub filter: Option<Filter>,
    pub info: Info,
    pub size: [usize; 2],
    frames: Option<Receiver<Vec<u8>>>,
    decoder: Option<Child>,
    sound: Option<Sound>,
    /// Seconds into the video when (re)started.
    offset: f32,
    started: Instant,
    shown: u64,
    pub playing: bool,
    pub ended: bool,
    /// Background videos: start over at the end, no sound.
    pub looping: bool,
    pub muted: bool,
    /// The newest frame waiting to go on screen.
    pub fresh: Option<Vec<u8>>,
    pub texture: Option<egui::TextureHandle>,
}

impl Player {
    /// Starts playing `path` from the beginning; `None` if ffmpeg can't.
    pub fn start(
        path: &Path,
        filter: Option<Filter>,
        looping: bool,
        muted: bool,
    ) -> Option<Player> {
        let info = probe(path)?;
        Self::begin(path.to_path_buf(), info, filter, looping, muted)
    }

    fn begin(
        path: PathBuf,
        info: Info,
        filter: Option<Filter>,
        looping: bool,
        muted: bool,
    ) -> Option<Player> {
        tool("ffmpeg")?;
        let (w, h) = out_size(info.width, info.height);
        let mut p = Player {
            path,
            filter,
            info,
            size: [w, h],
            frames: None,
            decoder: None,
            sound: None,
            offset: 0.0,
            started: Instant::now(),
            shown: 0,
            playing: false,
            ended: false,
            looping,
            muted,
            fresh: None,
            texture: None,
        };
        p.resume()?;
        Some(p)
    }

    /// Seconds into the video right now.
    pub fn position(&self) -> f32 {
        if self.playing {
            self.offset + self.started.elapsed().as_secs_f32()
        } else {
            self.offset
        }
    }

    pub fn progress(&self) -> f32 {
        if self.info.duration > 0.0 {
            (self.position() / self.info.duration).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    fn kill(&mut self) {
        if let Some(mut c) = self.decoder.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        if let Some(s) = self.sound.take() {
            s.stop();
        }
        self.frames = None;
    }

    pub fn pause(&mut self) {
        if self.playing {
            self.offset = self.position();
            self.playing = false;
            self.kill();
        }
    }

    /// Jumps to `fraction` (0–1) of the video, keeping it playing or paused.
    pub fn seek(&mut self, fraction: f32) {
        if self.info.duration <= 0.0 {
            return;
        }
        let was_playing = self.playing;
        self.pause();
        self.ended = false;
        self.offset = (fraction.clamp(0.0, 1.0) * self.info.duration)
            .min((self.info.duration - 0.5).max(0.0));
        if was_playing {
            let _ = self.resume();
        }
    }

    /// Starts (again) from the current position; restarts from the
    /// beginning if the video had ended.
    pub fn resume(&mut self) -> Option<()> {
        if self.playing {
            return Some(());
        }
        if self.ended {
            self.offset = 0.0;
            self.ended = false;
        }
        let ffmpeg = tool("ffmpeg")?;
        let [w, h] = self.size;
        let vf = format!("{}scale={w}:{h}", filter_chain(self.filter));
        let seek = format!("{:.3}", self.offset);
        let mut child =
            quiet(Command::new(ffmpeg).args(["-loglevel", "error", "-ss", &seek, "-i"]))
                .arg(&self.path)
                .args(["-an", "-vf", &vf, "-f", "rawvideo", "-pix_fmt", "rgba", "-"])
                .stdout(Stdio::piped())
                .spawn()
                .ok()?;
        let mut out = child.stdout.take()?;
        let frame_len = w * h * 4;
        // A few frames of buffer; ffmpeg waits when it is full.
        let (tx, rx) = sync_channel::<Vec<u8>>(6);
        std::thread::spawn(move || loop {
            let mut buf = vec![0u8; frame_len];
            if out.read_exact(&mut buf).is_err() || tx.send(buf).is_err() {
                break;
            }
        });
        self.decoder = Some(child);
        self.frames = Some(rx);
        if self.info.has_audio && !self.muted {
            self.sound = start_sound(&self.path, &seek);
        }
        self.started = Instant::now();
        self.shown = 0;
        self.playing = true;
        Some(())
    }

    /// Takes the frames that are due by now; keeps the newest in `fresh`.
    /// Returns true while the video is still playing.
    pub fn tick(&mut self) -> bool {
        if !self.playing {
            return false;
        }
        let due = (self.started.elapsed().as_secs_f32() * self.info.fps) as u64 + 1;
        while self.shown < due {
            let Some(rx) = &self.frames else { break };
            match rx.try_recv() {
                Ok(f) => {
                    self.fresh = Some(f);
                    self.shown += 1;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    // decoder finished: the video is over
                    self.offset = self.info.duration;
                    self.playing = false;
                    self.ended = true;
                    self.kill();
                    if self.looping {
                        return self.resume().is_some();
                    }
                    return false;
                }
            }
        }
        true
    }
}

impl Drop for Player {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Whether inline playback is possible on this computer.
pub fn available() -> bool {
    tool("ffmpeg").is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_ffmpeg_info() {
        let text = "  Duration: 00:01:02.50, start: 0.000000, bitrate: 1757 kb/s\n  Stream #0:0[0x1](und): Video: hevc (Main) (hvc1 / 0x31637668), yuv420p(tv, bt709), 1080x1920, 1753 kb/s, 29.97 fps, 30 tbr, 600 tbn (default)\n  Stream #0:1[0x2](und): Audio: aac (LC), 44100 Hz, stereo, fltp\n";
        let i = parse_ffmpeg_info(text).unwrap();
        assert_eq!((i.width, i.height), (1080, 1920));
        assert!((i.fps - 29.97).abs() < 0.01);
        assert_eq!(i.duration, 62.5);
        assert!(i.has_audio);
        let rot = "  Stream #0:0: Video: h264, yuv420p, 1920x1080 [SAR 1:1 DAR 16:9], 30 fps\n      displaymatrix: rotation of -90.00 degrees\n";
        let r = parse_ffmpeg_info(rot).unwrap();
        assert_eq!((r.width, r.height), (1080, 1920));
        assert!(!r.has_audio);
        assert!(parse_ffmpeg_info("At least one output file must be specified").is_none());
    }

    #[test]
    fn frame_sizes_fit_and_stay_even() {
        assert_eq!(out_size(1080, 1920), (720, 1280));
        assert_eq!(out_size(640, 360), (640, 360));
        let (w, h) = out_size(1921, 1081);
        assert!(w <= MAX_SIDE && w % 2 == 0 && h % 2 == 0);
    }
}
