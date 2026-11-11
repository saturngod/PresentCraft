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

  theme-check THEME_DIR
      Validates a theme folder (template.json, presets.json, stylesheet,
      fonts) and warns about hard-to-read colour combinations.

  font "FAMILY" FONTS_DIR [--weights 400,700] [--italic]
      Downloads static .ttf files of a Google Fonts family plus its licence
      into FONTS_DIR and prints matching @font-face rules for the theme CSS.
"""

import argparse
import json
import struct
import urllib.request
import os
import re
import shutil
import sys
import tempfile
import urllib.parse
import zipfile
from pathlib import Path

BUILTIN_THEMES = [
    "Yangon", "Mandalay", "Bagan", "Taunggyi", "Singapore", "Bangkok", "Chiang Mai",
    "Hanoi", "Hoi An", "Da Nang", "Kuala Lumpur", "Penang", "Malacca", "Siem Reap",
    "Phnom Penh", "Luang Prabang", "Vientiane", "Bali", "Jakarta", "Yogyakarta",
    "Manila", "Cebu", "Bandar Seri Begawan", "Dili",
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
NS = "app.myanmars.presentcraft"
IA_NS = "net.ia.presenter"
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

    ours = {
        "creator": "prcraft.py",
        "appearance": a.appearance,
        "author": a.author,
        "audience": a.audience,
    }
    if a.theme:
        ours["theme"] = a.theme
    # The standard info.json top-level keys.
    info = {
        "creatorIdentifier": NS,
        "type": "net.daringfireball.markdown",
        "version": 2,
        "transient": False,
        NS: ours,
    }
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
    # Theme files may have trailing commas; accept them like the app does.
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
        # Settings live under our identifier; the first version used flat keys.
        ours = info.get(NS)
        if ours is None:
            ours = {k[0].lower() + k[1:]: v for k, v in info.items() if k[:1].isupper()}
            if ours:
                warnings.append(f'info.json uses old flat keys; put them under "{NS}"')
        if not isinstance(ours, dict):
            errors.append(f'info.json "{NS}" must be an object')
            ours = {}
        ia = info.get(IA_NS) if isinstance(info.get(IA_NS), dict) else {}
        if ours.get("appearance", "light") not in ("light", "dark"):
            errors.append('info.json appearance must be "light" or "dark"')
        slots = ours.get("slots", [])
        if not isinstance(slots, list) or len(slots) > 6:
            errors.append("info.json Slots must be a list of at most 6 objects")
        else:
            for s in slots:
                kind = s.get("kind", s.get("Kind", "Empty")) if isinstance(s, dict) else None
                if kind not in SLOT_KINDS:
                    errors.append(f"info.json slots entry {s!r}: kind must be one of {sorted(SLOT_KINDS)}")
        logo = ours.get("logo") or ia.get("logo")
        if logo and not is_web(logo) and resolve(logo, root) is None:
            errors.append(f"logo not found: {logo}")
        theme_dir = root / "Theme"
        if theme_dir.is_dir():
            e2, w2, s2 = check_theme(theme_dir)
            errors += e2
            warnings += w2
            for line in s2:
                print(f"Theme: {line}")
        elif ours.get("theme") and ours["theme"] not in BUILTIN_THEMES:
            warnings.append(
                f'theme "{ours["theme"]}" is not built in; it is used only if the person '
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


# ---------- themes ----------

GENERIC_FONTS = {
    "system", "system-ui", "-apple-system", "blinkmacsystemfont", "ui-sans-serif",
    "sans", "sans-serif", "ui-rounded", "ui-monospace", "monospace", "serif",
    "ui-serif", "-apple-system-ui-serif",
}
COLOR_KEYS = [
    "DarkBodyTextColor", "LightBodyTextColor", "DarkTitleTextColor",
    "LightTitleTextColor", "DarkBackgroundColor", "LightBackgroundColor",
]


def parse_color(s):
    """(r, g, b, a) from #RGB, #RGBA, #RRGGBB or #RRGGBBAA; None otherwise."""
    if not isinstance(s, str) or not s.startswith("#"):
        return None
    h = s[1:]
    if not re.fullmatch(r"[0-9a-fA-F]+", h) or len(h) not in (3, 4, 6, 8):
        return None
    if len(h) in (3, 4):
        h = "".join(c * 2 for c in h)
    v = [int(h[i:i + 2], 16) for i in range(0, len(h), 2)]
    return tuple(v + [255] * (4 - len(v)))


def luminance(c):
    def lin(v):
        v /= 255
        return v / 12.92 if v <= 0.04045 else ((v + 0.055) / 1.055) ** 2.4
    return 0.2126 * lin(c[0]) + 0.7152 * lin(c[1]) + 0.0722 * lin(c[2])


def contrast(fg, bg):
    a = fg[3] / 255  # see-through text is blended over the background
    fg = tuple(round(fg[i] * a + bg[i] * (1 - a)) for i in range(3))
    x, y = luminance(fg) + 0.05, luminance(bg) + 0.05
    return max(x, y) / min(x, y)


def font_families(path):
    """Family names in a .ttf/.otf/.ttc (typographic name first)."""
    try:
        b = open(path, "rb").read()
    except OSError:
        return set()
    offsets = [0]
    if b[:4] == b"ttcf":
        n = struct.unpack(">I", b[8:12])[0]
        offsets = list(struct.unpack(f">{n}I", b[12:12 + 4 * n]))
    names = set()
    for base in offsets:
        try:
            num = struct.unpack(">H", b[base + 4:base + 6])[0]
            found = {}
            for i in range(num):
                tag, _, off, _ = struct.unpack(">4sIII", b[base + 12 + 16 * i:base + 28 + 16 * i])
                if tag != b"name":
                    continue
                count, strings = struct.unpack(">HH", b[off + 2:off + 6])
                for j in range(count):
                    pid, _, lid, nid, ln, o = struct.unpack(">6H", b[off + 6 + 12 * j:off + 18 + 12 * j])
                    if nid in (1, 16) and pid == 3 and lid == 0x409:
                        found[nid] = b[off + strings + o:off + strings + o + ln].decode("utf-16-be", "ignore")
            if found:
                names.add(found.get(16, found.get(1)))
        except struct.error:
            continue
    return names


def check_theme(theme_dir, label="Theme"):
    """(errors, warnings, summary lines) for a theme folder."""
    d = Path(theme_dir)
    errors, warnings, summary = [], [], []
    try:
        template = lenient_json((d / "template.json").read_text(encoding="utf-8"))
    except FileNotFoundError:
        return [f"{label}/template.json missing"], [], []
    except ValueError as e:
        return [f"{label}/template.json: {e}"], [], []
    try:
        presets = lenient_json((d / "presets.json").read_text(encoding="utf-8")).get("Presets", [])
    except FileNotFoundError:
        presets = []
        warnings.append(f"{label}/presets.json missing: default colours are used")
    except ValueError as e:
        return [f"{label}/presets.json: {e}"], warnings, []
    if not str(template.get("Name", "")).strip():
        errors.append(f"{label}/template.json needs a Name")
    css = template.get("Css") or ""
    css_text = ""
    if not css or not (d / css).is_file():
        warnings.append(f"{label}: stylesheet {css or '(none)'} missing; the app writes one on save")
    else:
        css_text = (d / css).read_text(encoding="utf-8", errors="ignore")
    for i, layout in enumerate(template.get("Layouts", []) or []):
        if not isinstance(layout, dict) or "Name" not in layout:
            errors.append(f"{label}/template.json Layouts[{i}] needs a Name")

    # Fonts: files anywhere in the folder, matched by their family name.
    bundled, web_only = {}, set()
    for f in d.rglob("*"):
        ext = f.suffix.lower()
        if ext in (".ttf", ".otf", ".ttc", ".otc"):
            for fam in font_families(f):
                bundled.setdefault(fam.lower(), fam)
        elif ext in (".woff", ".woff2"):
            web_only.add(f.stem.split("-")[0].lower())
    wanted = []
    for src in [template] + presets:
        for key in ("TitleFont", "BodyFont"):
            value = src.get(key)
            if value:
                first = next((n.strip(" '\"") for n in str(value).split(",")
                              if n.strip(" '\"").lower() not in GENERIC_FONTS), None)
                if first and first.lower() not in (w.lower() for w in wanted):
                    wanted.append(first)
    for fam in wanted:
        if fam.lower() in bundled:
            if css_text and fam.lower() not in css_text.lower():
                warnings.append(f"{label}: {fam} is bundled but has no @font-face rule in {css}")
        elif fam.replace(" ", "").lower() in web_only:
            warnings.append(f"{label}: {fam} is only .woff/.woff2; add .ttf files for PresentCraft")
        else:
            warnings.append(f"{label}: font {fam!r} is not bundled; PresentCraft uses an installed font "
                            "of that name or falls back to the built-in sans/serif")
    summary.append(f"{template.get('Name')}: fonts {', '.join(wanted) or 'System'}; "
                   f"bundled {', '.join(sorted(bundled.values())) or 'none'}")

    # Colours and readability.
    if not presets:
        return errors, warnings, summary
    looks = set()
    for i, pr in enumerate(presets):
        name = pr.get("Name", f"#{i}")
        app = str(pr.get("Appearance", "light")).lower()
        if app not in ("light", "dark"):
            errors.append(f"{label}/presets.json {name}: Appearance must be light or dark")
        looks.add(app)
        for key in COLOR_KEYS + [f"Accent{k}" for k in range(1, 7)] + ["DarkAccent1", "LightAccent1"]:
            v = pr.get(key)
            if v is not None and v != "transparent" and parse_color(v) is None:
                errors.append(f"{label}/presets.json {name}: {key} {v!r} is not a #RGB/#RRGGBB(AA) colour")
        for key in ("LightBgGradient", "DarkBgGradient"):
            stops = pr.get(key)
            if stops is not None and (not isinstance(stops, list) or len(stops) < 2
                                      or any(parse_color(c) is None for c in stops)):
                errors.append(f"{label}/presets.json {name}: {key} needs 2+ colours")
        for key in ("LightGraphDataColors", "DarkGraphDataColors"):
            g = pr.get(key)
            if g is not None and (not isinstance(g, dict) or not all(re.fullmatch(r"data-\d+", k) for k in g)):
                errors.append(f"{label}/presets.json {name}: {key} must look like {{\"data-0\": \"#...\"}}")
    first = presets[0]
    summary.append(f"starts {str(first.get('Appearance', 'light')).lower()}; looks: {', '.join(sorted(looks))}")
    # Light slides: Dark… text on LightBackground (or the light gradient).
    for look, bg_key, grad_key, title_key, body_key, fallback in (
        ("light", "LightBackgroundColor", "LightBgGradient", "DarkTitleTextColor", "DarkBodyTextColor", (255, 255, 255, 255)),
        ("dark", "DarkBackgroundColor", "DarkBgGradient", "LightTitleTextColor", "LightBodyTextColor", (0, 0, 0, 255)),
    ):
        stops = first.get(grad_key)
        bgs = [parse_color(c) for c in stops] if stops else [parse_color(first.get(bg_key)) or fallback]
        bgs = [b for b in bgs if b]
        for text_key, need in ((title_key, 3.0), (body_key, 4.5)):
            fg = parse_color(first.get(text_key))
            if not fg:
                continue
            worst = min(contrast(fg, b) for b in bgs)
            if worst < need:
                warnings.append(f"{label}: {look} slides: {text_key} {first.get(text_key)} has contrast "
                                f"{worst:.2f}:1 on its {'gradient' if stops else 'background'} (aim for {need}:1)")
    return errors, warnings, summary


def cmd_theme_check(a):
    errors, warnings, summary = check_theme(a.theme_dir, Path(a.theme_dir).name)
    for line in summary:
        print(line)
    for w in warnings:
        print(f"warning: {w}")
    for e in errors:
        print(f"error: {e}")
    print("OK" if not errors else f"{len(errors)} error(s)")
    return 1 if errors else 0


def cmd_font(a):
    family = a.family
    weights = [int(w) for w in a.weights.split(",")]
    faces = [(0, w) for w in weights] + ([(1, 400)] if a.italic else [])
    spec = "ital,wght@" + ";".join(f"{i},{w}" for i, w in sorted(faces))
    url = "https://fonts.googleapis.com/css2?family=" + urllib.parse.quote(family) + ":" + spec

    def get(u):
        # An old user agent makes Google Fonts serve plain .ttf files.
        req = urllib.request.Request(u, headers={"User-Agent": "Mozilla/4.0"})
        with urllib.request.urlopen(req, timeout=60) as r:
            return r.read()

    out = Path(a.fonts_dir)
    out.mkdir(parents=True, exist_ok=True)
    slug = family.replace(" ", "")
    rules = []
    for block in re.findall(r"@font-face\s*{([^}]*)}", get(url).decode()):
        style = re.search(r"font-style:\s*(\w+)", block).group(1)
        weight = int(re.search(r"font-weight:\s*(\d+)", block).group(1))
        src = re.search(r"url\((https://[^)]+\.ttf)\)", block).group(1)
        name = {400: "Regular", 700: "Bold"}.get(weight, str(weight))
        fname = f"{slug}-{'Italic' if style == 'italic' and weight == 400 else name + ('Italic' if style == 'italic' else '')}.ttf"
        (out / fname).write_bytes(get(src))
        rules.append(f"@font-face {{\n  font-family: '{family}';\n  font-style: {style};\n"
                     f"  font-weight: {weight};\n  src: url('{out.name}/{fname}') format('truetype');\n}}")
    lic = family.lower().replace(" ", "")
    for base, file in (("ofl", "OFL.txt"), ("apache", "LICENSE.txt"), ("ufl", "LICENCE.txt")):
        try:
            (out / f"{slug}-{file}").write_bytes(
                get(f"https://raw.githubusercontent.com/google/fonts/main/{base}/{lic}/{file}"))
            break
        except Exception:
            continue
    else:
        print(f"warning: no licence file found for {family}; add it by hand", file=sys.stderr)
    print("\n".join(rules))
    return 0


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
    t = sub.add_parser("theme-check")
    t.add_argument("theme_dir")
    f = sub.add_parser("font")
    f.add_argument("family")
    f.add_argument("fonts_dir")
    f.add_argument("--weights", default="400,700")
    f.add_argument("--italic", action="store_true")
    a = ap.parse_args()
    return {"build": cmd_build, "check": cmd_check, "zip": cmd_zip, "unzip": cmd_unzip,
            "theme-check": cmd_theme_check, "font": cmd_font}[a.cmd](a)


if __name__ == "__main__":
    sys.exit(main())
