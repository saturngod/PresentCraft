//! Presentation documents.
//!
//! PresentCraft saves `Name.prcraft` package folders laid out like iA
//! Presenter's `.iapresenter` packages:
//!
//! ```text
//! Name.prcraft/
//! ├── text.md          the script
//! ├── info.json        document settings (design choices, author, …)
//! ├── assets/          pictures and videos used by the script
//! └── Theme/           the deck's theme: template.json + presets.json
//! ```
//!
//! Media in `assets/` is referenced as `/assets/name.jpg`, as in iA
//! Presenter. `.iapresenter` documents (folder or zip) and plain Markdown
//! files open too; saving always writes a `.prcraft` package.
//!
//! A package is either a folder (the macOS default, where Finder shows it
//! as one file) or a zip file with the same members (the Windows and Linux
//! default, so it can be double-clicked). Both open everywhere, and saving
//! keeps the form a document already has. A zipped document is edited in a
//! private working folder and zipped again on each save.

use crate::json::Json;
use crate::theme::Preset;
use std::path::{Path, PathBuf};

pub const EXT: &str = "prcraft";
pub const IA_EXT: &str = "iapresenter";
pub const TEXT: &str = "text.md";
pub const INFO: &str = "info.json";
pub const ASSETS: &str = "assets";
pub const THEME: &str = "Theme";

/// What kind of document a path is.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    /// A PresentCraft package; saving writes back in place.
    PrCraft,
    /// An iA Presenter document (folder or zip); saving makes a new package.
    IaPresenter,
    /// Plain Markdown or text; saving makes a new package.
    Text,
}

pub fn kind_of(path: &Path) -> Option<Kind> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        EXT => Some(Kind::PrCraft),
        IA_EXT => Some(Kind::IaPresenter),
        "md" | "markdown" | "txt" | "text" => Some(Kind::Text),
        _ => None,
    }
}

/// The package a path belongs to: the path itself, or the package holding
/// it when someone picked `text.md` (or another member) inside one.
pub fn package_for(path: &Path) -> PathBuf {
    for a in path.ancestors() {
        if matches!(kind_of(a), Some(Kind::PrCraft | Kind::IaPresenter)) {
            return a.to_path_buf();
        }
    }
    path.to_path_buf()
}

/// How a package is stored on disk.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Form {
    Folder,
    Zip,
}

/// The form new packages get on this platform.
pub fn default_form() -> Form {
    if cfg!(target_os = "macos") {
        Form::Folder
    } else {
        Form::Zip
    }
}

/// The form of an existing package at `dest`, else `fallback`.
pub fn form_of(dest: &Path, fallback: Form) -> Form {
    if dest.is_dir() {
        Form::Folder
    } else if dest.is_file() {
        Form::Zip
    } else {
        fallback
    }
}

/// An opened document.
pub struct Opened {
    pub kind: Kind,
    pub form: Form,
    pub text: String,
    /// Folder that `/assets/…` and `./…` media references resolve from.
    pub root: PathBuf,
    /// `info.json`, kept so unknown keys survive saving.
    pub info: Option<Json>,
    /// The document's own theme from `Theme/`.
    pub theme: Option<Preset>,
}

pub fn open(path: &Path) -> Result<Opened, String> {
    let path = package_for(path);
    let kind = kind_of(&path).unwrap_or(Kind::Text);
    if kind == Kind::Text || (path.is_file() && !is_zip(&path)) {
        let text = read_text(&path)?;
        let root = path.parent().map(Path::to_path_buf).unwrap_or_default();
        return Ok(Opened {
            kind: Kind::Text,
            form: Form::Folder,
            text,
            root,
            info: None,
            theme: None,
        });
    }
    let form = if path.is_dir() { Form::Folder } else { Form::Zip };
    let root = if form == Form::Folder {
        path.clone()
    } else {
        // Always unpack afresh so changes made elsewhere show up.
        let work = work_dir(&path);
        let _ = std::fs::remove_dir_all(&work);
        extract_into(&path, &work)?;
        work
    };
    let root = find_package_root(&root).unwrap_or(root);
    let text_file = script_in(&root).ok_or("The document has no text.md")?;
    let text = read_text(&text_file)?;
    let info = std::fs::read_to_string(root.join(INFO))
        .ok()
        .and_then(|t| Json::parse(&t).ok());
    let theme_dir = root.join(THEME);
    let theme = if crate::themefile::is_theme_dir(&theme_dir) {
        crate::themefile::load(&theme_dir).ok()
    } else {
        None
    };
    Ok(Opened {
        kind,
        form,
        text,
        root,
        info,
        theme,
    })
}

fn read_text(p: &Path) -> Result<String, String> {
    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    let text = String::from_utf8(bytes).map_err(|_| "The text isn't UTF-8".to_string())?;
    Ok(text.trim_start_matches('\u{feff}').replace("\r\n", "\n"))
}

/// The script file in a package folder: `text.md`, else any Markdown file.
fn script_in(root: &Path) -> Option<PathBuf> {
    let main = root.join(TEXT);
    if main.is_file() {
        return Some(main);
    }
    let mut md: Vec<PathBuf> = std::fs::read_dir(root)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && matches!(kind_of(p), Some(Kind::Text)))
        .collect();
    md.sort();
    md.into_iter().next()
}

/// A zip may wrap everything in one top folder; find where `text.md` is.
fn find_package_root(dir: &Path) -> Option<PathBuf> {
    if script_in(dir).is_some() {
        return Some(dir.to_path_buf());
    }
    let subdirs: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir() && !p.file_name().is_some_and(|n| n == "__MACOSX"))
        .collect();
    match subdirs.as_slice() {
        [only] => find_package_root(only),
        _ => None,
    }
}

fn is_zip(p: &Path) -> bool {
    use std::io::Read;
    let mut head = [0u8; 4];
    std::fs::File::open(p)
        .and_then(|mut f| f.read_exact(&mut head))
        .is_ok()
        && head == *b"PK\x03\x04"
}

/// The private working folder for the zipped document at `p`.
fn work_dir(p: &Path) -> PathBuf {
    let abs = std::path::absolute(p).unwrap_or_else(|_| p.to_path_buf());
    let mut h: u64 = 0xcbf29ce484222325;
    for b in abs.to_string_lossy().bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    let stem = p
        .file_stem()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    std::env::temp_dir()
        .join("presentcraft-work")
        .join(format!("{stem}-{h:016x}"))
}

/// Unpacks the zip at `p` into `dest`.
fn extract_into(p: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(p).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("Not a readable package: {e}"))?;
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        // enclosed_name rejects absolute paths and `..` escapes.
        let Some(rel) = entry.enclosed_name() else {
            continue;
        };
        let out = dest.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut f = std::fs::File::create(&out).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut f).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Already-compressed media is stored as is, so saving stays quick.
const STORED_EXTS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "webp", "heic", "avif", "mp4", "mov", "m4v", "webm", "mp3",
    "m4a", "zip", "pdf",
];

/// Zips the folder `src` into `dest`, replacing it only once the new
/// archive is complete.
fn write_zip(src: &Path, dest: &Path) -> Result<(), String> {
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;
    let mut files = Vec::new();
    collect_files(src, src, &mut files);
    files.sort();
    let name = dest
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dest.with_file_name(format!(".{name}.saving"));
    let result = (|| -> Result<(), String> {
        let out = std::fs::File::create(&tmp).map_err(|e| e.to_string())?;
        let mut zip = zip::ZipWriter::new(std::io::BufWriter::new(out));
        for rel in &files {
            let path = src.join(rel);
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            let ext = rel
                .extension()
                .map(|e| e.to_string_lossy().to_ascii_lowercase())
                .unwrap_or_default();
            let method = if STORED_EXTS.contains(&ext.as_str()) {
                CompressionMethod::Stored
            } else {
                CompressionMethod::Deflated
            };
            let opts = SimpleFileOptions::default()
                .compression_method(method)
                .large_file(size > u32::MAX as u64);
            let entry: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            zip.start_file(entry.join("/"), opts)
                .map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
        }
        let mut out = zip.finish().map_err(|e| e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, dest).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// Files below `dir`, relative to `base`, leaving out hidden and
/// half-written files.
fn collect_files(base: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "__MACOSX" || name.ends_with(".saving") {
            continue;
        }
        let path = e.path();
        if path.is_dir() {
            collect_files(base, &path, out);
        } else if let Ok(rel) = path.strip_prefix(base) {
            out.push(rel.to_path_buf());
        }
    }
}

/// Everything written into a package.
pub struct SaveRequest<'a> {
    pub text: &'a str,
    pub info: Json,
    pub theme: &'a Preset,
    pub dark: bool,
    /// The deck logo (a media reference), bundled like slide media.
    pub logo: &'a str,
    /// Finds the file a media reference points at, relative to where the
    /// document was before this save.
    pub resolve: &'a dyn Fn(&str) -> Option<PathBuf>,
}

/// What saving changed: references to bundled media now point into
/// `assets/`, so the editor text and logo are updated to match.
pub struct Saved {
    pub text: String,
    pub logo: String,
    pub copied: usize,
    pub form: Form,
    /// Folder media references now resolve from.
    pub root: PathBuf,
}

/// Writes `req` as the package `dest`. An existing package keeps its form;
/// a new one gets `form`. Media from outside the package is copied into its
/// `assets/` folder. Files already in the package that this save doesn't
/// write are left alone.
pub fn save(dest: &Path, form: Form, req: SaveRequest) -> Result<Saved, String> {
    let form = form_of(dest, form);
    if form == Form::Folder {
        let mut saved = save_folder(dest, req)?;
        saved.root = dest.to_path_buf();
        return Ok(saved);
    }
    let work = work_dir(dest);
    if dest.is_file() {
        if !work.is_dir() {
            // Working copy gone (or never made): start from the saved zip so
            // nothing in it is lost.
            extract_into(dest, &work)?;
        }
    } else {
        let _ = std::fs::remove_dir_all(&work);
    }
    let root = find_package_root(&work).unwrap_or(work);
    let mut saved = save_folder(&root, req)?;
    write_zip(&root, dest)?;
    saved.form = Form::Zip;
    saved.root = root;
    Ok(saved)
}

fn save_folder(dest: &Path, req: SaveRequest) -> Result<Saved, String> {
    std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let mut bundler = Bundler {
        dest,
        resolve: req.resolve,
        copied: 0,
        done: Vec::new(),
    };
    let text = bundler.bundle_text(req.text)?;
    let logo = if req.logo.trim().is_empty() {
        String::new()
    } else {
        bundler.bundle_ref(req.logo.trim())?
    };
    let mut info = req.info;
    if !logo.is_empty() {
        crate::docinfo::set_logo(&mut info, &logo);
    }
    write_atomic(&dest.join(TEXT), text.as_bytes())?;
    write_atomic(&dest.join(INFO), info.to_pretty().as_bytes())?;
    crate::themefile::save(
        &dest.join(THEME),
        req.theme,
        req.dark,
        req.theme.file.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    Ok(Saved {
        text,
        logo,
        copied: bundler.copied,
        form: Form::Folder,
        root: dest.to_path_buf(),
    })
}

fn write_atomic(p: &Path, bytes: &[u8]) -> Result<(), String> {
    let tmp = p.with_extension("saving");
    std::fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, p).map_err(|e| e.to_string())
}

struct Bundler<'a> {
    dest: &'a Path,
    resolve: &'a dyn Fn(&str) -> Option<PathBuf>,
    copied: usize,
    /// Source file → reference already written this save.
    done: Vec<(PathBuf, String)>,
}

impl Bundler<'_> {
    /// The media reference on `line`, tracking fenced code blocks.
    fn media_src(&self, line: &str, fenced: &mut bool) -> Option<String> {
        let t = line.trim_start_matches('\t').trim();
        if t.starts_with("```") || t.starts_with("~~~") {
            *fenced = !*fenced;
        }
        if *fenced {
            None
        } else if let Some((_, src)) = crate::parse::markdown_image(t) {
            Some(src)
        } else if crate::parse::is_image_src(t) {
            Some(t.to_string())
        } else {
            None
        }
    }

    /// Rewrites each media line whose file sits outside the package.
    fn bundle_text(&mut self, text: &str) -> Result<String, String> {
        let mut out = String::with_capacity(text.len());
        let mut fenced = false;
        for (i, line) in text.split('\n').enumerate() {
            if i > 0 {
                out.push('\n');
            }
            let src = self.media_src(line, &mut fenced);
            match src.filter(|s| !crate::images::is_web(s)) {
                Some(src) => {
                    let new = self.bundle_ref(&src)?;
                    out.push_str(&line.replacen(&src, &new, 1));
                }
                None => out.push_str(line),
            }
        }
        Ok(out)
    }

    /// The reference to use for `src` once the package holds its file.
    fn bundle_ref(&mut self, src: &str) -> Result<String, String> {
        if crate::images::is_web(src) {
            return Ok(src.to_string());
        }
        let Some(file) = (self.resolve)(src).filter(|f| f.is_file()) else {
            return Ok(src.to_string()); // missing: leave it as written
        };
        let file = file.canonicalize().unwrap_or(file);
        let dest = self.dest.canonicalize().unwrap_or(self.dest.to_path_buf());
        if let Ok(rel) = file.strip_prefix(&dest) {
            // Already inside: a package-relative reference keeps working,
            // but a full path to the file becomes a package reference.
            let full_path = Path::new(src).is_absolute() && Path::new(src).exists();
            return Ok(if (src.starts_with("./") || src.starts_with('/')) && !full_path {
                src.to_string()
            } else {
                to_ref(rel)
            });
        }
        if let Some((_, r)) = self.done.iter().find(|(f, _)| *f == file) {
            return Ok(r.clone());
        }
        let assets = self.dest.join(ASSETS);
        std::fs::create_dir_all(&assets).map_err(|e| e.to_string())?;
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "media".into());
        let target = free_name(&assets, &name, &file);
        if !target.exists() {
            std::fs::copy(&file, &target)
                .map_err(|e| format!("Couldn't copy {}: {e}", file.display()))?;
            self.copied += 1;
        }
        let r = to_ref(target.strip_prefix(self.dest).unwrap_or(&target));
        self.done.push((file, r.clone()));
        Ok(r)
    }
}

/// `/assets/a%20b.jpg` style reference for a package-relative path.
fn to_ref(rel: &Path) -> String {
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    format!("/{}", parts.join("/")).replace(' ', "%20")
}

/// `name` in `dir`, or `name-2`, `name-3`… when a different file has it.
/// A same-content file already there is reused.
fn free_name(dir: &Path, name: &str, source: &Path) -> PathBuf {
    let (stem, ext) = match name.rfind('.') {
        Some(i) if i > 0 => (&name[..i], &name[i..]),
        _ => (name, ""),
    };
    let mut n = 1;
    loop {
        let candidate = if n == 1 {
            dir.join(name)
        } else {
            dir.join(format!("{stem}-{n}{ext}"))
        };
        if !candidate.exists() || same_file_contents(&candidate, source) {
            return candidate;
        }
        n += 1;
    }
}

fn same_file_contents(a: &Path, b: &Path) -> bool {
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else {
        return false;
    };
    ma.len() == mb.len() && std::fs::read(a).ok() == std::fs::read(b).ok()
}

/// Copies a media file into a saved package's folder (for a zipped one,
/// its working folder) and returns its reference.
pub fn add_asset(package: &Path, file: &Path) -> Result<String, String> {
    let mut b = Bundler {
        dest: package,
        resolve: &|_| Some(file.to_path_buf()),
        copied: 0,
        done: Vec::new(),
    };
    b.bundle_ref(&file.to_string_lossy())
}

/// Root of the private working folders (zipped and unsaved documents).
fn work_root() -> PathBuf {
    std::env::temp_dir().join("presentcraft-work")
}

/// True for a working folder this app made.
pub fn is_work_dir(dir: &Path) -> bool {
    dir.starts_with(work_root())
}

/// A new, empty working folder for a document that isn't saved yet, so
/// media added to it is already inside "the package".
pub fn new_work_dir() -> std::io::Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = work_root().join(format!("untitled-{}-{nanos}", std::process::id()));
    std::fs::create_dir_all(dir.join(ASSETS))?;
    Ok(dir)
}

/// Copies the media `text` mentions into `dest/assets/` and returns the
/// text pointing at the copies, plus how many files were copied.
pub fn import_media(
    text: &str,
    dest: &Path,
    resolve: &dyn Fn(&str) -> Option<PathBuf>,
) -> Result<(String, usize), String> {
    let mut b = Bundler {
        dest,
        resolve,
        copied: 0,
        done: Vec::new(),
    };
    let t = b.bundle_text(text)?;
    Ok((t, b.copied))
}

/// Points every media line that uses `old` at `new` instead.
pub fn replace_media_ref(text: &str, old: &str, new: &str) -> String {
    let b = Bundler {
        dest: Path::new(""),
        resolve: &|_| None,
        copied: 0,
        done: Vec::new(),
    };
    let mut out = String::with_capacity(text.len());
    let mut fenced = false;
    for (i, line) in text.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        match b.media_src(line, &mut fenced) {
            Some(src) if src == old => out.push_str(&line.replacen(old, new, 1)),
            _ => out.push_str(line),
        }
    }
    out
}

const VIDEO_EXTS: &[&str] = &["mp4", "mov", "m4v", "webm"];
const PICTURE_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "tif", "tiff", "bmp", "svg", "apng", "heic", "avif",
];

pub fn is_video_file(p: &Path) -> bool {
    ext_in(p, VIDEO_EXTS)
}

pub fn is_media_file(p: &Path) -> bool {
    ext_in(p, VIDEO_EXTS) || ext_in(p, PICTURE_EXTS)
}

fn ext_in(p: &Path, list: &[&str]) -> bool {
    p.extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .is_some_and(|e| list.contains(&e.as_str()))
}

/// Every picture and video in `root/assets`, sorted by name.
pub fn list_assets(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_files(&root.join(ASSETS), &root.join(ASSETS), &mut files);
    let mut out: Vec<PathBuf> = files
        .into_iter()
        .map(|rel| root.join(ASSETS).join(rel))
        .filter(|p| is_media_file(p))
        .collect();
    out.sort_by_key(|p| {
        p.file_name()
            .map(|n| n.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    });
    out
}

/// The script reference for a file inside the package at `root`.
pub fn asset_ref(root: &Path, file: &Path) -> String {
    to_ref(file.strip_prefix(root).unwrap_or(file))
}

/// A package path for `name`: adds `.prcraft` when missing.
pub fn with_ext(p: PathBuf) -> PathBuf {
    if kind_of(&p) == Some(Kind::PrCraft) {
        p
    } else {
        let mut s = p.into_os_string();
        s.push(".");
        s.push(EXT);
        PathBuf::from(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("pc-pkg-test-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn saves_bundles_media_and_reopens() {
        let dir = scratch("save");
        let photo = dir.join("my photo.png");
        std::fs::write(&photo, b"png-bytes").unwrap();
        let text = "# Hi\n\n./my%20photo.png\n\n---\n\n![Alt](./my%20photo.png)\n\n```\n./my%20photo.png\n```\n";
        let preset = crate::theme::presets()[5].clone();
        let pkg = dir.join("Deck.prcraft");
        let root = dir.clone();
        let resolve = move |s: &str| {
            let s = s.replace("%20", " ");
            let p = root.join(s.trim_start_matches("./"));
            p.exists().then_some(p)
        };
        let info = crate::json::obj([(
            crate::docinfo::NS,
            crate::json::obj([("theme", preset.name.into())]),
        )]);
        let saved = save(
            &pkg,
            Form::Folder,
            SaveRequest {
                text,
                info,
                theme: &preset,
                dark: true,
                logo: "",
                resolve: &resolve,
            },
        )
        .unwrap();
        assert_eq!(saved.copied, 1);
        assert!(saved.text.contains("\n/assets/my%20photo.png\n"));
        assert!(saved.text.contains("![Alt](/assets/my%20photo.png)"));
        // code blocks are left alone
        assert!(saved.text.contains("```\n./my%20photo.png\n```"));
        assert!(pkg.join("assets/my photo.png").is_file());

        let opened = open(&pkg.join(TEXT)).unwrap();
        assert_eq!(opened.kind, Kind::PrCraft);
        assert_eq!(opened.root, pkg);
        assert_eq!(opened.text, saved.text);
        let theme = opened.theme.unwrap();
        assert_eq!(theme.name, preset.name);
        assert!(theme.starts_dark);
        let d = crate::docinfo::read(&opened.info.unwrap());
        assert_eq!(d.theme.as_deref(), Some(preset.name));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn opens_zipped_iapresenter() {
        use std::io::Write;
        let dir = scratch("zip");
        let p = dir.join("Talk.iapresenter");
        {
            let f = std::fs::File::create(&p).unwrap();
            let mut z = zip::ZipWriter::new(f);
            let o = zip::write::SimpleFileOptions::default();
            z.start_file("Talk/text.md", o).unwrap();
            z.write_all(b"# Talk\r\n\r\n/assets/a.jpg\r\n").unwrap();
            z.start_file("Talk/assets/a.jpg", o).unwrap();
            z.write_all(b"jpg").unwrap();
            z.start_file("Talk/info.json", o).unwrap();
            z.write_all(br#"{"Unknown": 1,}"#).unwrap();
            z.finish().unwrap();
        }
        let opened = open(&p).unwrap();
        assert_eq!(opened.kind, Kind::IaPresenter);
        assert_eq!(opened.form, Form::Zip);
        assert_eq!(opened.text, "# Talk\n\n/assets/a.jpg\n");
        assert!(opened.root.join("assets/a.jpg").is_file());
        assert_eq!(opened.info.unwrap().num("Unknown"), Some(1.0));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zip_packages_save_reopen_and_keep_their_form() {
        let dir = scratch("zipsave");
        let photo = dir.join("pic.jpg");
        std::fs::write(&photo, b"jpg-bytes").unwrap();
        let preset = crate::theme::presets()[0].clone();
        let pkg = dir.join("Deck.prcraft");
        let photo_path = photo.clone();
        let resolve = move |_: &str| Some(photo_path.clone());
        let req = |text: &'static str| SaveRequest {
            text,
            info: crate::json::obj([("Extra", "kept".into())]),
            theme: &preset,
            dark: false,
            logo: "",
            resolve: &resolve,
        };
        let saved = save(&pkg, Form::Zip, req("# One\n\n./pic.jpg\n")).unwrap();
        assert_eq!(saved.form, Form::Zip);
        assert!(pkg.is_file() && is_zip(&pkg));
        assert!(saved.text.contains("/assets/pic.jpg"));

        let opened = open(&pkg).unwrap();
        assert_eq!((opened.kind, opened.form), (Kind::PrCraft, Form::Zip));
        assert_eq!(opened.text, saved.text);
        assert!(opened.root.join("assets/pic.jpg").is_file());
        assert!(opened.theme.is_some());

        // An existing zip stays a zip even when the default is a folder,
        // and its media survives a lost working folder.
        let _ = std::fs::remove_dir_all(work_dir(&pkg));
        let none = |_: &str| None;
        let again = save(
            &pkg,
            Form::Folder,
            SaveRequest {
                text: "# Two\n\n/assets/pic.jpg\n",
                info: crate::json::obj([]),
                theme: &preset,
                dark: false,
                logo: "",
                resolve: &none,
            },
        )
        .unwrap();
        assert_eq!(again.form, Form::Zip);
        let reopened = open(&pkg).unwrap();
        assert!(reopened.text.starts_with("# Two"));
        assert!(reopened.root.join("assets/pic.jpg").is_file());
        let _ = std::fs::remove_dir_all(work_dir(&pkg));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn added_media_always_gets_a_package_reference() {
        let dir = scratch("add");
        let pkg = new_work_dir().unwrap();
        let outside = dir.join("a b.png");
        std::fs::write(&outside, b"x").unwrap();
        assert_eq!(add_asset(&pkg, &outside).unwrap(), "/assets/a%20b.png");
        // adding the copy again (by full path) reuses it
        let inside = pkg.join("assets/a b.png");
        assert_eq!(add_asset(&pkg, &inside).unwrap(), "/assets/a%20b.png");
        assert_eq!(list_assets(&pkg), vec![inside]);
        assert!(is_work_dir(&pkg));
        let t = replace_media_ref("x\n/old.png\n```\n/old.png\n```", "/old.png", "/assets/n.png");
        assert_eq!(t, "x\n/assets/n.png\n```\n/old.png\n```");
        let _ = std::fs::remove_dir_all(&pkg);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adds_extension_once() {
        assert_eq!(with_ext("/a/Deck".into()), PathBuf::from("/a/Deck.prcraft"));
        assert_eq!(with_ext("/a/Deck.prcraft".into()), PathBuf::from("/a/Deck.prcraft"));
        assert_eq!(package_for(Path::new("/a/D.prcraft/text.md")), PathBuf::from("/a/D.prcraft"));
    }
}
