#!/usr/bin/env python3
"""Regenerate committed platform icons from icon.png on macOS (sips/iconutil)."""
from pathlib import Path
import struct
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'packaging' / 'icons'
SIZES = (16, 24, 32, 48, 64, 128, 256, 512, 1024)
source = (ROOT / 'icon.png').read_bytes()
if source[:8] != b'\x89PNG\r\n\x1a\n' or struct.unpack('>II', source[16:24]) != (1024, 1024):
    raise SystemExit('icon.png must be a square 1024x1024 PNG')
OUT.mkdir(parents=True, exist_ok=True)
for size in SIZES:
    subprocess.run(['sips', '-z', str(size), str(size), str(ROOT / 'icon.png'),
                    '--out', str(OUT / f'{size}.png')], check=True, stdout=subprocess.DEVNULL)
# ICO directory with PNG images, including intermediate sizes for display scaling.
frames = [(size, (OUT / f'{size}.png').read_bytes()) for size in SIZES if size <= 256]
offset = 6 + 16 * len(frames)
entries = []
for size, data in frames:
    entries.append(struct.pack('<BBBBHHII', size % 256, size % 256, 0, 0, 1, 32, len(data), offset))
    offset += len(data)
(OUT / 'PresentCraft.ico').write_bytes(struct.pack('<HHH', 0, 1, len(frames)) + b''.join(entries) + b''.join(data for _, data in frames))
with tempfile.TemporaryDirectory() as tmp:
    mac_source = Path(tmp) / 'macos.png'
    subprocess.run(['swift', str(ROOT / 'packaging/pad-macos-icon.swift'),
                    str(ROOT / 'icon.png'), str(mac_source)], check=True)
    mac_sizes = {}
    for size in (16, 32, 64, 128, 256, 512, 1024):
        target = Path(tmp) / f'macos-{size}.png'
        subprocess.run(['sips', '-z', str(size), str(size), str(mac_source),
                        '--out', str(target)], check=True, stdout=subprocess.DEVNULL)
        mac_sizes[size] = target
    (OUT / 'macos-256.png').write_bytes(mac_sizes[256].read_bytes())
    iconset = Path(tmp) / 'PresentCraft.iconset'
    iconset.mkdir()
    for size in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            suffix = '@2x' if scale == 2 else ''
            (iconset / f'icon_{size}x{size}{suffix}.png').write_bytes(mac_sizes[size * scale].read_bytes())
    subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(OUT / 'PresentCraft.icns')], check=True)
print('Generated macOS ICNS, Windows ICO, and Linux PNG sizes in', OUT)
