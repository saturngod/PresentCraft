#!/usr/bin/env python3
"""Build, check and convert PresentCraft .prcraft presentation packages.

Standard library only. Commands:

  build  SCRIPT.md OUT.prcraft [--theme NAME] [--appearance light|dark]
         [--author TEXT] [--audience TEXT] [--theme-dir DIR] [--zip]
      Makes a package from a Markdown script. Local pictures and videos the
      script mentions are copied into assets/ and their lines rewritten to
      /assets/<name>. Web addresses are left alone.

  check  PACKAGE
      Validates a package (folder or zip) and prints a slide summary.
      Exits with status 1 when there are errors.

  zip    FOLDER.prcraft [OUT.prcraft]    folder package -> single zip file
  unzip  FILE.prcraft OUT_FOLDER.prcraft  zip file -> folder package
"""

import argparse
import json
import os
import re
import shutil
import sys
import tempfile
import urllib.parse
import zipfile
from pathlib import Path

BUILTIN_THEMES = [
    "Swiss", "Contrast Serif", "Paper", "Signal", "Graphite",
    "Studio", "Midnight", "Editorial", "Corporate", "Sunset",
]
MEDIA_EXTS = {
    ".png", ".jpg", ".jpeg", ".gif", ".webp", ".tif", ".tiff", ".svg", ".pdf",
    ".apng", ".bmp", ".mp4", ".mov", ".m4v",
}
STORED_EXTS = {
    ".jpg", ".jpeg", ".png", ".gif", ".webp", ".heic", ".avif", ".mp4", ".mov",
    ".m4v", ".webm", ".mp3", ".m4a", ".zip", ".pdf",
}
SLOT_KINDS = {"Empty", "Number", "Date", "Logo", "Custom"}
CHART_KINDS = {
    "bar", "bars", "column", "columns", "line", "lines", "spline", "curve",
    "area", "step", "pie", "donut", "doughnut", "radar", "gauge", "bubble",
    "scatter", "horizontal", "stacked",
}
MD_IMAGE = re.compile(r"^!\[(?P<alt>[^\]]*)\]\((?P<src>[^)\s]+)(?:\s[^)]*)?\)$")
SEPARATOR = re.compile(r"^\s*-{3,}\s*$")


def is_web(src):
    return src.lower().startswith(("http://", "https://"))


def media_src(line):
    """The media reference on a line, or None (same rules as the app)."""
    t = line.lstrip("\t").strip()
    m = MD_IMAGE.match(t)
    if m:
        return m.group("src")
    if not t or any(c.isspace() for c in t) or t.startswith("|"):
        return None
    if is_web(t) or Path(t.lower()).suffix in MEDIA_EXTS:
        return t
    return None


def media_lines(text):
    """(line index, src) for each media line outside fenced code."""
    fenced = False
    for i, line in enumerate(text.split("\n")):
        t = line.lstrip("\t").strip()
        if t.startswith("```") or t.startswith("~~~"):
            fenced = not fenced
            continue
        if fenced:
            continue
        src = media_src(line)
        if src:
            yield i, src


def resolve(src, base):
    """The local file a reference points at, relative to folder `base`."""
    if is_web(src):
        return None
    p = urllib.parse.unquote(src)
    if p.startswith("file://"):
        p = p[7:]
    if p.startswith("~/"):
        p = os.path.expanduser(p)
    cand = Path(p)
    if cand.is_absolute() and cand.is_file():
        return cand
    rel = base / p.lstrip("/")
    if rel.is_file():
        return rel
    return None


def to_ref(name):
    return "/assets/" + name.replace(" ", "%20")


def free_name(assets, source):
    stem, ext = source.stem, source.suffix
    n = 1
    while True:
        name = source.name if n == 1 else f"{stem}-{n}{ext}"
        target = assets / name
        if not target.exists() or target.read_bytes() == source.read_bytes():
            return target
        n += 1


def cmd_build(a):
    script = Path(a.script)
    text = script.read_text(encoding="utf-8").replace("\r\n", "\n").lstrip("﻿")
    base = script.resolve().parent
    out = Path(a.out)
    if out.suffix.lower() != ".prcraft":
        out = out.with_name(out.name + ".prcraft")
    if out.exists():
        sys.exit(f"{out} already exists; choose another name or remove it first")

    work = Path(tempfile.mkdtemp(prefix="prcraft-")) if a.zip else out
    work.mkdir(parents=True, exist_ok=True)
    assets = work / "assets"
    lines = text.split("\n")
    copied, missing, done = 0, [], {}
    for i, src in list(media_lines(text)):
        if is_web(src) or src.startswith("/assets/") and (work / src.lstrip("/")).is_file():
            continue
        f = resolve(src, base)
        if f is None:
            missing.append(src)
            continue
        f = f.resolve()
        if f not in done:
            assets.mkdir(exist_ok=True)
            target = free_name(assets, f)
            if not target.exists():
                shutil.copy2(f, target)
                copied += 1
            done[f] = to_ref(target.name)
        lines[i] = lines[i].replace(src, done[f], 1)
    (work / "text.md").write_text("\n".join(lines), encoding="utf-8")

    info = {
        "Format": "PresentCraft",
        "FormatVersion": 1,
        "Creator": "prcraft.py",
        "Appearance": a.appearance,
        "Author": a.author,
        "Audience": a.audience,
    }
    if a.theme:
        info["Theme"] = a.theme
    if a.theme_dir:
        tdir = Path(a.theme_dir)
        if not (tdir / "template.json").is_file():
            sys.exit(f"{tdir} has no template.json")
        shutil.copytree(tdir, work / "Theme")
    (work / "info.json").write_text(json.dumps(info, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")

    if a.zip:
        write_zip(work, out)
        shutil.rmtree(work)
    print(f"Wrote {out} ({'zip' if a.zip else 'folder'}), copied {copied} media file(s)")
    for m in missing:
        print(f"  warning: media not found, left as written: {m}")
    return 0


def write_zip(folder, out):
    folder = Path(folder)
    tmp = out.with_name(f".{out.name}.saving")
    with zipfile.ZipFile(tmp, "w") as z:
        for p in sorted(folder.rglob("*")):
            rel = p.relative_to(folder)
            if p.is_dir() or any(part.startswith(".") or part == "__MACOSX" for part in rel.parts):
                continue
            method = zipfile.ZIP_STORED if p.suffix.lower() in STORED_EXTS else zipfile.ZIP_DEFLATED
            z.write(p, rel.as_posix(), compress_type=method)
    os.replace(tmp, out)


def lenient_json(text):
    # The app (and iA Presenter themes) accept trailing commas.
    return json.loads(re.sub(r",(\s*[}\]])", r"\1", text))


def open_package(path):
    """(root folder, temp dir to clean up or None)."""
    path = Path(path)
    if path.is_dir():
        return path, None
    if zipfile.is_zipfile(path):
        tmp = Path(tempfile.mkdtemp(prefix="prcraft-check-"))
        with zipfile.ZipFile(path) as z:
            for name in z.namelist():
                if name.startswith("/") or ".." in Path(name).parts:
                    raise SystemExit(f"unsafe path in zip: {name}")
            z.extractall(tmp)
        root = tmp
        if not (root / "text.md").exists():
            subs = [d for d in root.iterdir() if d.is_dir() and d.name != "__MACOSX"]
            if len(subs) == 1:
                root = subs[0]
        return root, tmp
    raise SystemExit(f"{path} is neither a package folder nor a zip file")


def cmd_check(a):
    root, tmp = open_package(a.package)
    errors, warnings = [], []
    try:
        tf = root / "text.md"
        if not tf.is_file():
            errors.append("missing text.md")
            text = ""
        else:
            try:
                text = tf.read_text(encoding="utf-8")
            except UnicodeDecodeError:
                errors.append("text.md is not UTF-8")
                text = ""
        info = {}
        if (root / "info.json").is_file():
            try:
                info = lenient_json((root / "info.json").read_text(encoding="utf-8"))
            except ValueError as e:
                errors.append(f"info.json: {e}")
        else:
            warnings.append("no info.json (defaults are used)")
        if not isinstance(info, dict):
            errors.append("info.json must be an object")
            info = {}
        if info.get("Appearance", "light") not in ("light", "dark"):
            errors.append('info.json Appearance must be "light" or "dark"')
        slots = info.get("Slots", [])
        if not isinstance(slots, list) or len(slots) > 6:
            errors.append("info.json Slots must be a list of at most 6 objects")
        else:
            for s in slots:
                if not isinstance(s, dict) or s.get("Kind", "Empty") not in SLOT_KINDS:
                    errors.append(f"info.json Slots entry {s!r}: Kind must be one of {sorted(SLOT_KINDS)}")
        theme_dir = root / "Theme"
        if theme_dir.is_dir():
            for name in ("template.json", "presets.json"):
                p = theme_dir / name
                if not p.is_file():
                    (errors if name == "template.json" else warnings).append(f"Theme/{name} missing")
                    continue
                try:
                    lenient_json(p.read_text(encoding="utf-8"))
                except ValueError as e:
                    errors.append(f"Theme/{name}: {e}")
        elif info.get("Theme") and info["Theme"] not in BUILTIN_THEMES:
            warnings.append(
                f'Theme "{info["Theme"]}" is not built in; it is used only if the person '
                "has a saved theme with that name (otherwise the default theme is used)"
            )
        for _, src in media_lines(text):
            if is_web(src):
                continue
            if resolve(src, root) is None:
                errors.append(f"media not found: {src}")
            elif not src.startswith("/assets/"):
                warnings.append(f"media outside /assets/ (not portable): {src}")
        for n, line in enumerate(text.split("\n"), 1):
            m = re.match(r"^\s*(chart|type)\s*:\s*(.+)$", line, re.I)
            if m:
                words = [w for w in re.split(r"[,\s-]+", m.group(2).lower()) if w]
                bad = [w for w in words if w not in CHART_KINDS]
                if bad:
                    warnings.append(f"line {n}: unknown chart word(s) {bad}; Bar is used")

        slides = [s for s in re.split(r"(?m)^\s*-{3,}\s*$", text)]
        print(f"{a.package}: {len(slides)} slide(s)")
        for i, s in enumerate(slides, 1):
            heading = next((l.strip() for l in s.split("\n") if l.startswith("#")), "")
            visible = sum(1 for l in s.split("\n") if l.startswith("\t") or l.startswith("#")
                          or l.startswith("|") or media_src(l))
            note = "" if visible else "  (nothing visible: notes only)"
            print(f"  {i:>2}. {heading or '(no heading)'}{note}")
        for w in warnings:
            print(f"warning: {w}")
        for e in errors:
            print(f"error: {e}")
        print("OK" if not errors else f"{len(errors)} error(s)")
        return 1 if errors else 0
    finally:
        if tmp:
            shutil.rmtree(tmp, ignore_errors=True)


def cmd_zip(a):
    src = Path(a.folder)
    if not (src / "text.md").is_file():
        sys.exit(f"{src} is not a package folder (no text.md)")
    out = Path(a.out) if a.out else src
    if out == src:
        tmp = src.with_name(src.name + ".folder-tmp")
        src.rename(tmp)
        try:
            write_zip(tmp, out)
        except BaseException:
            tmp.rename(src)
            raise
        shutil.rmtree(tmp)
    else:
        write_zip(src, out)
    print(f"Wrote {out} (zip)")
    return 0


def cmd_unzip(a):
    out = Path(a.out)
    if out.exists():
        sys.exit(f"{out} already exists")
    root, tmp = open_package(a.file)
    shutil.copytree(root, out)
    if tmp:
        shutil.rmtree(tmp, ignore_errors=True)
    print(f"Wrote {out} (folder)")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    b = sub.add_parser("build")
    b.add_argument("script")
    b.add_argument("out")
    b.add_argument("--theme", help=f"theme name, e.g. {', '.join(BUILTIN_THEMES)}")
    b.add_argument("--appearance", choices=["light", "dark"], default="light")
    b.add_argument("--author", default="")
    b.add_argument("--audience", default="")
    b.add_argument("--theme-dir", help="folder with template.json + presets.json to embed as Theme/")
    b.add_argument("--zip", action="store_true", help="write a single zip file instead of a folder")
    c = sub.add_parser("check")
    c.add_argument("package")
    z = sub.add_parser("zip")
    z.add_argument("folder")
    z.add_argument("out", nargs="?")
    u = sub.add_parser("unzip")
    u.add_argument("file")
    u.add_argument("out")
    a = ap.parse_args()
    return {"build": cmd_build, "check": cmd_check, "zip": cmd_zip, "unzip": cmd_unzip}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
