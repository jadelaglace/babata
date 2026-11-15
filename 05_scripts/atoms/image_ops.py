#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11,<3.15"
# dependencies = ["pillow>=11,<14", "numpy>=2,<3"]
# ///
"""Babata static image atoms.

Render side (asset edits): key-white, key-chroma, shadow-extract, over, resize.
Vision side (measurement/evidence): probe, measure, diff, multibg, tiles, layout, annotate.

Every command reads its inputs, writes a NEW run directory outside Git and writes
manifest.json last. Same run + same request replays after hash verification; a
different request against an existing run is refused. Pillow + NumPy only.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import sys
import time
import uuid
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageOps, __version__ as PIL_VERSION

SCHEMA = "babata.image-atom/v1"
TOOL_VERSION = "1.0.0"
DEFAULT_MAX_PIXELS = 80_000_000
DEFAULT_MAX_BYTES = 512 * 1024 * 1024
SUPPORTED_MODES = {"1", "L", "LA", "P", "PA", "RGB", "RGBA", "RGBa", "I;16", "I;16B", "I;16L", "I", "CMYK", "YCbCr"}
BACKGROUNDS = {
    "white": (255, 255, 255), "black": (0, 0, 0), "grey": (128, 128, 128),
    "green": (0, 200, 0), "magenta": (230, 0, 230), "checker": None,
}


class AtomError(Exception):
    """Expected, reportable failure (bad input, conflict, unsupported)."""

    def __init__(self, reason: str, message: str):
        super().__init__(message)
        self.reason = reason


# ---------------------------------------------------------------- identity / io

def sha256_file(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as fh:
        for chunk in iter(lambda: fh.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def canonical_hash(value) -> str:
    return hashlib.sha256(json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":")).encode()).hexdigest()


def is_link(path: Path) -> bool:
    """Symlink, junction or any other reparse point (checked by attribute, independent of os.path.isjunction)."""
    if path.is_symlink():
        return True
    try:
        attrs = getattr(os.lstat(path), "st_file_attributes", 0)
    except OSError:
        return False
    return bool(attrs & 0x400)  # FILE_ATTRIBUTE_REPARSE_POINT


def external_dir(directory: str) -> Path:
    out = Path(directory)
    if not out.is_absolute():
        raise AtomError("invalid-input", "output/run directory must be absolute")
    probe = out.resolve()
    while True:
        if (probe / ".git").exists():
            raise AtomError("invalid-input", "runtime artifacts must stay outside Git")
        if probe.parent == probe:
            break
        probe = probe.parent
    return out


def load_image(path_text: str, role: str, limits: dict, exif: str = "apply"):
    """Open one input with explicit guards; return (rgba uint8 array, input record)."""
    path = Path(path_text)
    if not path.is_absolute():
        raise AtomError("invalid-input", f"{role}: input path must be absolute")
    for part in [path, *path.parents]:
        if part.exists() and is_link(part):
            raise AtomError("invalid-input", f"{role}: symlink/junction input refused: {part}")
    if not path.is_file():
        raise AtomError("invalid-input", f"{role}: input missing: {path}")
    size = path.stat().st_size
    if size > limits["max_bytes"]:
        raise AtomError("too-large", f"{role}: {size} bytes exceeds {limits['max_bytes']}")
    digest = sha256_file(path)
    Image.MAX_IMAGE_PIXELS = None  # own explicit guard below, before decode
    try:
        im = Image.open(path)
        w, h = im.size
        if w * h > limits["max_pixels"]:
            raise AtomError("too-large", f"{role}: {w}x{h} exceeds {limits['max_pixels']} pixels")
        fmt, mode = im.format, im.mode
        if mode not in SUPPORTED_MODES:
            raise AtomError("unsupported", f"{role}: unsupported mode {mode}")
        if getattr(im, "is_animated", False) and getattr(im, "n_frames", 1) > 1:
            raise AtomError("unsupported", f"{role}: animated images are out of first-version scope")
        im.load()
    except AtomError:
        raise
    except Exception as exc:  # corrupt / truncated / not an image
        raise AtomError("invalid-input", f"{role}: cannot decode image: {type(exc).__name__}: {exc}") from exc
    orientation = 1
    try:
        orientation = int(im.getexif().get(0x0112, 1) or 1)
    except Exception:
        orientation = 1
    has_alpha = mode in ("LA", "PA", "RGBA", "RGBa") or (mode == "P" and "transparency" in im.info)
    conversions = []
    if exif == "apply" and orientation != 1:
        im = ImageOps.exif_transpose(im)
        conversions.append(f"exif-orientation-{orientation}-applied")
    if mode in ("I;16", "I;16B", "I;16L", "I"):
        arr = np.asarray(im, dtype=np.float64)
        peak = 65535.0 if mode.startswith("I;16") else max(float(arr.max()), 1.0)
        im = Image.fromarray(np.clip(arr / peak * 255.0 + 0.5, 0, 255).astype(np.uint8), "L")
        conversions.append(f"{mode}-scaled-to-8bit")
    if im.mode != "RGBA":
        conversions.append(f"{im.mode}->RGBA")
        im = im.convert("RGBA")
    rgba = np.asarray(im, dtype=np.uint8).copy()
    record = {
        "role": role, "path": str(path), "sha256": digest, "bytes": size, "format": fmt, "mode": mode,
        "width": int(rgba.shape[1]), "height": int(rgba.shape[0]), "source_size": [w, h],
        "has_alpha": bool(has_alpha), "exif_orientation": orientation, "exif_policy": exif,
        "conversions": conversions,
    }
    return rgba, record


def save_png(rgba: np.ndarray, path: Path):
    Image.fromarray(rgba, "RGBA").save(path, "PNG", optimize=False)


def producer() -> dict:
    return {"tool": "05_scripts/atoms/image_ops.py", "version": TOOL_VERSION, "python": platform.python_version(),
            "pillow": PIL_VERSION, "numpy": np.__version__}


class Run:
    """One run directory: replay, conflict, staging artifacts, manifest written last."""

    def __init__(self, out: str, command: str, request: dict):
        self.dir = external_dir(out)
        self.command = command
        self.request = request
        self.request_hash = canonical_hash({"command": command, "request": request})
        self.artifacts: list[dict] = []
        self.started = time.time()

    def existing(self):
        manifest = self.dir / "manifest.json"
        if manifest.exists():
            data = verify_dir(self.dir)
            if data["request_hash"] != self.request_hash:
                raise AtomError("conflict", "run directory already holds a different request; use a new run")
            data["replayed"] = True
            return data
        if self.dir.exists() and any(p.name != ".operation.lock" for p in self.dir.iterdir()):
            raise AtomError("conflict", "run directory has files but no manifest (incomplete run); use a new run")
        return None

    def __enter__(self):
        self.dir.mkdir(parents=True, exist_ok=True)
        self.lock = self.dir / ".operation.lock"
        try:
            fd = os.open(self.lock, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        except FileExistsError as exc:
            raise AtomError("conflict", "run directory is locked by another operation") from exc
        os.write(fd, json.dumps({"pid": os.getpid(), "started": self.started}).encode())
        os.close(fd)
        return self

    def __exit__(self, *exc):
        self.lock.unlink(missing_ok=True)
        return False

    def add(self, role: str, name: str, rgba: np.ndarray | None = None, data: bytes | None = None, relation: str = ""):
        target = self.dir / name
        if target.exists():
            raise AtomError("conflict", f"artifact already exists: {name}")
        if rgba is not None:
            save_png(rgba, target)
        else:
            target.write_bytes(data or b"")
        entry = {"role": role, "path": name, "sha256": sha256_file(target), "bytes": target.stat().st_size}
        if rgba is not None:
            entry.update(width=int(rgba.shape[1]), height=int(rgba.shape[0]))
        if relation:
            entry["relation"] = relation
        self.artifacts.append(entry)
        return target

    def finish(self, inputs: list[dict], measurements=None, findings=None, inspected_scope=None,
               limits=None, state="succeeded", extra=None) -> dict:
        for record in inputs:  # original files must be untouched
            if sha256_file(Path(record["path"])) != record["sha256"]:
                raise AtomError("integrity", f"input changed during run: {record['role']}")
        manifest = {
            "schema": SCHEMA, "command": self.command, "state": state, "request": self.request,
            "request_hash": self.request_hash, "inputs": inputs, "artifacts": self.artifacts,
            "measurements": measurements or {}, "findings": findings or [],
            "inspected_scope": inspected_scope or {}, "limits": limits or [], "producer": producer(),
            "started_at": time.strftime("%Y-%m-%dT%H:%M:%S%z", time.localtime(self.started)),
            "elapsed_ms": int((time.time() - self.started) * 1000), "replayed": False,
        }
        if extra:
            manifest.update(extra)
        tmp = self.dir / f"manifest.{uuid.uuid4().hex}.tmp"
        tmp.write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        os.replace(tmp, self.dir / "manifest.json")
        return manifest


def verify_dir(directory: Path) -> dict:
    manifest_path = directory / "manifest.json"
    if not manifest_path.is_file():
        raise AtomError("integrity", "manifest missing; run is not ready")
    try:
        data = json.loads(manifest_path.read_text(encoding="utf-8"))
    except Exception as exc:
        raise AtomError("integrity", f"manifest unreadable: {exc}") from exc
    if data.get("schema") != SCHEMA:
        raise AtomError("integrity", "manifest schema mismatch")
    if data.get("request_hash") != canonical_hash({"command": data.get("command"), "request": data.get("request")}):
        raise AtomError("integrity", "manifest request hash mismatch (tampered)")
    for art in data.get("artifacts", []):
        rel = Path(art["path"])
        if rel.is_absolute() or ".." in rel.parts:
            raise AtomError("integrity", f"invalid artifact path {art['path']}")
        target = directory / rel
        if not target.is_file() or sha256_file(target) != art["sha256"]:
            raise AtomError("integrity", f"artifact hash mismatch: {art['path']}")
    return data


# ---------------------------------------------------------------- pixel helpers

def to_float(rgba: np.ndarray):
    f = rgba.astype(np.float64) / 255.0
    return f[..., :3], f[..., 3]


def to_uint8(rgb: np.ndarray, alpha: np.ndarray) -> np.ndarray:
    out = np.concatenate([rgb, alpha[..., None]], axis=-1)
    return np.clip(np.floor(out * 255.0 + 0.5), 0, 255).astype(np.uint8)


def luminance(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0] * 0.299 + rgb[..., 1] * 0.587 + rgb[..., 2] * 0.114


def label(mask: np.ndarray, connectivity: int = 8):
    """Run-length union-find connected components. Returns (labels, stats list)."""
    h, w = mask.shape
    parent = [0]

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x

    rows, prev = [], []
    ext = 1 if connectivity == 8 else 0
    for y in range(h):
        row = mask[y]
        if not row.any():
            rows.append([])
            prev = []
            continue
        d = np.diff(np.concatenate(([0], row.astype(np.int8), [0])))
        starts, ends = np.flatnonzero(d == 1).tolist(), np.flatnonzero(d == -1).tolist()
        cur, j = [], 0
        for s, e in zip(starts, ends):
            lo, hi, lab = s - ext, e + ext, 0
            while j < len(prev) and prev[j][1] <= lo:
                j += 1
            k = j
            while k < len(prev) and prev[k][0] < hi:
                pl = find(prev[k][2])
                if lab == 0:
                    lab = pl
                elif pl != lab:
                    a, b = min(pl, lab), max(pl, lab)
                    parent[b] = a
                    lab = a
                k += 1
            if lab == 0:
                lab = len(parent)
                parent.append(lab)
            cur.append((s, e, lab))
        rows.append(cur)
        prev = cur
    remap, labels = {}, np.zeros((h, w), np.int32)
    for y, cur in enumerate(rows):
        for s, e, lab in cur:
            root = find(lab)
            labels[y, s:e] = remap.setdefault(root, len(remap) + 1)
    n = len(remap)
    stats = []
    if n:
        ys, xs = np.nonzero(labels)
        labs = labels[ys, xs]
        area = np.bincount(labs, minlength=n + 1)
        x0 = np.full(n + 1, w); y0 = np.full(n + 1, h); x1 = np.full(n + 1, -1); y1 = np.full(n + 1, -1)
        np.minimum.at(x0, labs, xs); np.minimum.at(y0, labs, ys)
        np.maximum.at(x1, labs, xs); np.maximum.at(y1, labs, ys)
        for i in range(1, n + 1):
            stats.append({"label": i, "area": int(area[i]), "bbox": [int(x0[i]), int(y0[i]), int(x1[i]), int(y1[i])]})
    return labels, stats


def shift_any(mask: np.ndarray, radius: int) -> np.ndarray:
    """Binary dilation by a square of the given radius (no wrap-around)."""
    out = mask.copy()
    h, w = mask.shape
    for dy in range(-radius, radius + 1):
        for dx in range(-radius, radius + 1):
            if dy == 0 and dx == 0:
                continue
            src = mask[max(0, -dy):h - max(0, dy), max(0, -dx):w - max(0, dx)]
            out[max(0, dy):h - max(0, -dy), max(0, dx):w - max(0, -dx)] |= src
    return out


def box_sum(values: np.ndarray, r: int) -> np.ndarray:
    """Sum over a (2r+1)^2 window via an integral image (edges clipped)."""
    h, w = values.shape
    ii = np.zeros((h + 1, w + 1)); ii[1:, 1:] = values.cumsum(0).cumsum(1)
    y0 = np.clip(np.arange(h) - r, 0, h); y1 = np.clip(np.arange(h) + r + 1, 0, h)
    x0 = np.clip(np.arange(w) - r, 0, w); x1 = np.clip(np.arange(w) + r + 1, 0, w)
    return ii[y1][:, x1] - ii[y0][:, x1] - ii[y1][:, x0] + ii[y0][:, x0]


def boundary_len(opaque: np.ndarray) -> int:
    pad = np.pad(opaque, 1, constant_values=False)
    nb = ~pad[:-2, 1:-1] | ~pad[2:, 1:-1] | ~pad[1:-1, :-2] | ~pad[1:-1, 2:]
    return int((opaque & nb).sum())


def cluster_boxes(mask: np.ndarray, min_area: int = 1, limit: int = 20, merge_radius: int = 3):
    if not mask.any():
        return []
    grown = shift_any(mask, merge_radius) if merge_radius else mask
    labels, stats = label(grown, 8)
    boxes = []
    for st in stats:
        sel = (labels == st["label"]) & mask
        cnt = int(sel.sum())
        if cnt < min_area:
            continue
        ys, xs = np.nonzero(sel)
        boxes.append({"bbox": [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())], "pixels": cnt})
    boxes.sort(key=lambda b: -b["pixels"])
    return boxes[:limit]


def checker(h: int, w: int, cell: int = 16) -> np.ndarray:
    yy, xx = np.mgrid[0:h, 0:w]
    v = np.where(((yy // cell) + (xx // cell)) % 2 == 0, 0.8, 0.6)
    return np.repeat(v[..., None], 3, axis=-1)


def composite_on(rgba: np.ndarray, background: str) -> np.ndarray:
    rgb, a = to_float(rgba)
    h, w = a.shape
    bg_spec = BACKGROUNDS[background]
    bg = checker(h, w) if bg_spec is None else np.broadcast_to(np.array(bg_spec, float) / 255.0, (h, w, 3))
    return rgb * a[..., None] + bg * (1 - a[..., None])


def over_pair(top: np.ndarray, bottom: np.ndarray) -> np.ndarray:
    tc, ta = to_float(top)
    bc, ba = to_float(bottom)
    ao = ta + ba * (1 - ta)
    num = tc * ta[..., None] + bc * (ba * (1 - ta))[..., None]
    co = np.where(ao[..., None] > 0, num / np.maximum(ao[..., None], 1e-12), 0.0)
    return to_uint8(co, ao)


def resize_rgba(rgba: np.ndarray, width: int, height: int) -> np.ndarray:
    """Premultiplied Lanczos resize so transparent RGB never bleeds into edges."""
    rgb, a = to_float(rgba)
    chans = [rgb[..., i] * a for i in range(3)] + [a]
    out = [np.asarray(Image.fromarray(c.astype(np.float32), "F").resize((width, height), Image.LANCZOS), np.float64)
           for c in chans]
    ao = np.clip(out[3], 0, 1)
    co = np.stack([np.where(ao > 1e-6, np.clip(c, 0, None) / np.maximum(ao, 1e-6), 0.0) for c in out[:3]], -1)
    return to_uint8(np.clip(co, 0, 1), ao)


def fit_layer(layer: np.ndarray, width: int, height: int, mode: str) -> np.ndarray:
    if mode == "stretch":
        return resize_rgba(layer, width, height)
    h, w = layer.shape[:2]
    scale = min(width / w, height / h)
    nw, nh = max(1, round(w * scale)), max(1, round(h * scale))
    small = resize_rgba(layer, nw, nh)
    canvas = np.zeros((height, width, 4), np.uint8)
    x, y = (width - nw) // 2, (height - nh) // 2
    canvas[y:y + nh, x:x + nw] = small
    return canvas


# ---------------------------------------------------------------- measurement

PROFILES = {
    # min_stray: components smaller than this many px (or fraction of subject) are specks.
    # edge_px lower bound 0.3: below it the alpha is practically binary (stair-stepped); a crisp
    # anti-aliased vector edge measures ~0.6-1.0 and must not be flagged.
    "cutout": {"single_component": True, "max_mid_band": 0.05, "edge_px": (0.3, 3.5), "binary_ok": False, "attached_parts": False},
    "multi-part": {"single_component": False, "max_mid_band": 0.05, "edge_px": (0.3, 3.5), "binary_ok": False, "attached_parts": True},
    "translucent": {"single_component": False, "max_mid_band": None, "edge_px": None, "binary_ok": True, "attached_parts": True},
    "pixel-art": {"single_component": False, "max_mid_band": 0.0, "edge_px": None, "binary_ok": True, "attached_parts": True},
}


def measure_alpha(rgba: np.ndarray, profile: str, min_stray: int | None = None, expect_parts: int | None = None):
    rgb, a = to_float(rgba)
    h, w = a.shape
    prof = PROFILES[profile]
    opaque = a >= 0.5
    coverage = float(opaque.mean())
    findings = []
    if not (a > 0).any():
        return {"empty_alpha": True, "alpha_coverage": 0.0}, [{"kind": "empty-alpha", "detail": "no visible pixels"}]
    labels, comps = label(opaque, 8)
    subject_area = max([c["area"] for c in comps], default=0)
    stray_limit = min_stray if min_stray is not None else max(50, int(0.002 * subject_area))
    big = [c for c in comps if c["area"] >= stray_limit]
    small = [c for c in comps if c["area"] < stray_limit]
    attach_px = max(4, round(0.03 * max(h, w)))
    attached = []
    if prof["attached_parts"] and big and small:
        # dots of i/j, diacritics, small detached parts close to a major shape are legal parts here
        near_major = shift_any(np.isin(labels, [c["label"] for c in big]), attach_px)
        attached = [c for c in small if near_major[labels == c["label"]].any()]
    attached_ids = {c["label"] for c in attached}
    specks = [c for c in small if c["label"] not in attached_ids]
    # faint floating noise: visible (alpha>=16/255) islands not touching any opaque component
    faint = (a >= 16 / 255) & ~opaque
    faint_islands = []
    if faint.any():
        near_any = shift_any(opaque, 3)
        fl, fstats = label(faint, 8)
        for st in fstats:
            sel = fl == st["label"]
            if not (sel & near_any).any() and st["area"] >= 4:
                faint_islands.append(st)
    mid = (a > 0.125) & (a <= 0.878)
    mid_ratio = float(mid.sum() / max(1, (a > 0).sum()))
    blen = boundary_len(opaque)
    edge_px = float(mid.sum() / blen) if blen else 0.0
    hist = np.histogram((a * 255).round(), bins=[0, 32, 128, 224, 256])[0] / a.size
    by_threshold = {}
    for t in (50, 96, 128):
        _, cs = label(a >= t / 255, 8)
        by_threshold[str(t)] = {"components": len(cs), "stray": sum(1 for c in cs if c["area"] < stray_limit)}
    # holes: transparent regions fully enclosed by the subject
    clear = ~opaque
    hl, hstats = label(clear, 4)
    holes = [s for s in hstats if s["bbox"][0] > 0 and s["bbox"][1] > 0 and s["bbox"][2] < w - 1 and s["bbox"][3] < h - 1]
    m = {
        "profile": profile, "alpha_coverage": round(coverage, 5), "components": len(comps),
        "major_components": len(big), "attached_small_parts": len(attached), "attach_px": attach_px,
        "stray_components": len(specks),
        "stray_pixels": int(sum(c["area"] for c in specks)), "stray_limit_px": stray_limit,
        "faint_islands": len(faint_islands), "mid_band_ratio": round(mid_ratio, 5),
        "edge_transition_px": round(edge_px, 3), "boundary_px": blen,
        "alpha_histogram": {k: round(float(v), 5) for k, v in zip(["0-32", "32-128", "128-224", "224-255"], hist)},
        "by_threshold": by_threshold, "enclosed_holes": len(holes),
        "component_boxes_inclusive": [c["bbox"] for c in sorted(big, key=lambda c: -c["area"])[:20]],
    }
    for c in specks[:50]:
        findings.append({"kind": "stray-speck", "bbox": c["bbox"], "pixels": c["area"],
                         "basis": f"opaque island < {stray_limit}px"})
    for c in faint_islands[:50]:
        findings.append({"kind": "faint-floating-noise", "bbox": c["bbox"], "pixels": c["area"],
                         "basis": "semi-transparent island detached from subject"})
    if prof["single_component"] and len(big) > 1:
        findings.append({"kind": "multiple-components", "count": len(big), "boxes": m["component_boxes_inclusive"],
                         "basis": "profile cutout expects one subject; use multi-part if parts are intended"})
    if expect_parts is not None and len(big) != expect_parts:
        findings.append({"kind": "part-count-mismatch", "expected": expect_parts, "actual": len(big)})
    if prof["max_mid_band"] is not None and mid_ratio > prof["max_mid_band"] + 1e-9:
        findings.append({"kind": "semi-transparent-excess", "value": m["mid_band_ratio"],
                         "limit": prof["max_mid_band"], "basis": f"profile {profile}"})
    if prof["edge_px"] is not None:
        lo, hi = prof["edge_px"]
        if edge_px > hi:
            findings.append({"kind": "wide-edge-ramp", "value": m["edge_transition_px"], "limit": hi})
        elif edge_px < lo and not prof["binary_ok"] and blen > 40:
            findings.append({"kind": "hard-binary-edge", "value": m["edge_transition_px"], "limit": lo,
                             "basis": "stair-stepped edge; not a defect for pixel-art profile"})
    findings.extend(fringe_findings(rgba, opaque))
    return m, findings


def fringe_findings(rgba: np.ndarray, opaque: np.ndarray):
    rgb, a = to_float(rgba)
    clear = a < 0.5
    if not clear.any() or not opaque.any():
        return []
    near_clear = shift_any(clear, 2)
    ring = opaque & near_clear
    interior = opaque & ~shift_any(clear, 5)
    if interior.sum() < 20 or ring.sum() < 20:
        return []
    lum = luminance(rgb) * 255
    spill = (rgb[..., 1] - np.maximum(rgb[..., 0], rgb[..., 2])) * 255
    # compare each edge pixel with the interior NEAR IT (radius 10), not with a global mean:
    # differently coloured parts and thin strokes without an interior are not judged against each other
    count = box_sum(interior.astype(np.float64), 10)
    has_local = count >= 8
    local_lum = box_sum(lum * interior, 10) / np.maximum(count, 1)
    local_spill = box_sum((spill >= 40) * interior, 10) / np.maximum(count, 1)
    out = []
    checks = [
        ("bright-fringe", (lum >= 225) & (local_lum < 190), "edge pixels near white while the adjacent interior is darker"),
        ("dark-fringe", (lum <= 35) & (local_lum > 80), "edge pixels near black while the adjacent interior is lighter"),
        ("green-spill", (spill >= 40) & (local_spill < 0.02), "green dominance on the edge but not in the adjacent interior"),
    ]
    for kind, cond, basis in checks:
        flagged = ring & has_local & cond
        ratio = float(flagged.sum() / ring.sum())
        boxes = cluster_boxes(flagged, min_area=12, merge_radius=4)
        if boxes and flagged.sum() >= 24:
            out.append({"kind": kind, "ring_ratio": round(ratio, 4), "boxes": boxes[:10], "basis": basis})
    return out


def layout_scan(rgba: np.ndarray, tolerance: int = 24, min_band: int | None = None):
    rgb = rgba[..., :3].astype(np.int32)
    h, w = rgb.shape[:2]
    corners = np.concatenate([rgb[:4, :4].reshape(-1, 3), rgb[:4, -4:].reshape(-1, 3), rgb[-4:, :4].reshape(-1, 3), rgb[-4:, -4:].reshape(-1, 3)])
    vals, counts = np.unique(corners, axis=0, return_counts=True)
    bg = vals[counts.argmax()]
    content = np.abs(rgb - bg).max(axis=-1) > tolerance
    findings = []
    edge_names = {"right": content[:, -2:].any(axis=1), "left": content[:, :2].any(axis=1),
                  "bottom": content[-2:, :].any(axis=0), "top": content[:2, :].any(axis=0)}
    for side, line in edge_names.items():
        if line.any():
            idx = np.flatnonzero(line)
            runs = np.split(idx, np.flatnonzero(np.diff(idx) > 3) + 1)
            spans = [[int(r[0]), int(r[-1])] for r in runs if len(r) >= 3]
            if spans:
                findings.append({"kind": "content-touches-edge", "side": side, "spans": spans[:20],
                                 "basis": "non-background pixels in the outermost 2px; possible clipping"})
    rows = content.any(axis=1)
    band = min_band or max(120, int(0.06 * h))
    empty = ~rows
    if empty.any():
        idx = np.flatnonzero(empty)
        runs = np.split(idx, np.flatnonzero(np.diff(idx) > 1) + 1)
        first, last = (np.flatnonzero(rows)[[0, -1]] if rows.any() else (0, h - 1))
        for r in runs:
            if len(r) >= band and r[0] > first and r[-1] < last:
                findings.append({"kind": "blank-band", "rows": [int(r[0]), int(r[-1])], "height": int(len(r)),
                                 "basis": f"interior empty band >= {band}px between content"})
    cols, rws = np.flatnonzero(content.any(0)), np.flatnonzero(rows)
    m = {"background_rgb": [int(x) for x in bg], "content_ratio": round(float(content.mean()), 5),
         "content_bbox_inclusive": [int(cols[0]), int(rws[0]), int(cols[-1]), int(rws[-1])] if content.any() else None,
         "min_blank_band_px": band, "tolerance": tolerance}
    return m, findings


def diff_measure(a_img: np.ndarray, b_img: np.ndarray, backgrounds: list[str], tolerance: int):
    if a_img.shape != b_img.shape:
        raise AtomError("invalid-input", f"size mismatch {a_img.shape[1]}x{a_img.shape[0]} vs {b_img.shape[1]}x{b_img.shape[0]}; diff requires equal sizes")
    A, B = a_img.astype(np.int32), b_img.astype(np.int32)
    alpha_d = np.abs(A[..., 3] - B[..., 3])
    rgb_d = np.abs(A[..., :3] - B[..., :3]).max(axis=-1)
    both_clear = (A[..., 3] == 0) & (B[..., 3] == 0)
    hidden = (rgb_d > tolerance) & both_clear
    visible_rgb = (rgb_d > tolerance) & ~both_clear
    raw = (alpha_d > tolerance) | visible_rgb | hidden

    def summary(mask, mag=None):
        out = {"changed_pixels": int(mask.sum())}
        if mask.any():
            ys, xs = np.nonzero(mask)
            out["bbox_inclusive"] = [int(xs.min()), int(ys.min()), int(xs.max()), int(ys.max())]
            out["regions"] = cluster_boxes(mask, merge_radius=2)[:10]
            if mag is not None:
                out["max_delta"] = int(mag[mask].max())
        return out

    m = {"coordinate_convention": "x0,y0,x1,y1 inclusive, origin top-left", "tolerance": tolerance,
         "raw_any": summary(raw), "raw_alpha": summary(alpha_d > tolerance, alpha_d),
         "raw_rgb_where_visible": summary(visible_rgb, rgb_d),
         "hidden_rgb_under_full_transparency": summary(hidden, rgb_d), "visible": {}}
    vis_masks = {}
    for bg in backgrounds:
        ca = np.floor(composite_on(a_img, bg) * 255 + 0.5)
        cb = np.floor(composite_on(b_img, bg) * 255 + 0.5)
        d = np.abs(ca - cb).max(axis=-1)
        vis_masks[bg] = d > tolerance
        m["visible"][bg] = summary(vis_masks[bg], d.astype(np.int32))
    return m, raw, vis_masks


# ---------------------------------------------------------------- commands

def parse_box(text: str):
    parts = text.split(",")
    if len(parts) < 4:
        raise AtomError("invalid-input", f"box needs x0,y0,x1,y1[,label]: {text}")
    return [int(p) for p in parts[:4]], ",".join(parts[4:])


def cmd_key_white(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    req = {"input_sha256": rec["sha256"], "level": args.level, "soft": args.soft, "connectivity": args.connectivity,
           "keep_largest": args.keep_largest, "exif": args.exif}
    run = Run(args.out, "key-white", req)
    if (prev := run.existing()):
        return prev
    rgb, a0 = to_float(img)
    mean = rgb.mean(axis=-1) * 255
    key_alpha = np.clip((args.level - mean) / args.soft, 0, 1)
    candidate = key_alpha < 1
    if args.connectivity == "border":
        labels, stats = label(candidate, 8)
        h, w = candidate.shape
        border = {s["label"] for s in stats if s["bbox"][0] == 0 or s["bbox"][1] == 0 or s["bbox"][2] == w - 1 or s["bbox"][3] == h - 1}
        bg_region = np.isin(labels, list(border)) if border else np.zeros_like(candidate)
        enclosed_white = candidate & ~bg_region
    else:
        bg_region, enclosed_white = candidate, np.zeros_like(candidate)
    alpha = np.where(bg_region, key_alpha, 1.0) * a0
    unblend = np.where(alpha[..., None] > 0.02, (rgb - (1 - alpha[..., None])) / np.maximum(alpha[..., None], 0.02), rgb)
    out = to_uint8(np.clip(unblend, 0, 1), alpha)
    limits_note = ["simple white keying: soft shadows become grey, complex hair/glass need a real matting backend"]
    with run:
        if args.keep_largest:
            labels, stats = label(out[..., 3] >= 128, 8)
            if stats:
                keep = max(stats, key=lambda s: s["area"])["label"]
                out[..., 3] = np.where(labels == keep, out[..., 3], np.where(labels == 0, out[..., 3], 0))
                limits_note.append("keep-largest removed every other component (explicitly requested)")
        run.add("cutout", "cutout.png", out, relation="derived from input")
        meas, finds = measure_alpha(out, args.profile)
        meas["enclosed_near_white_kept_px"] = int(enclosed_white.sum())
        if enclosed_white.sum() > 0:
            finds.append({"kind": "enclosed-near-white-kept", "pixels": int(enclosed_white.sum()),
                          "boxes": cluster_boxes(enclosed_white, min_area=16)[:10],
                          "basis": "near-white area not connected to the border was kept as subject; use --connectivity global if it is background"})
        return run.finish([rec], meas, finds, {"region": "full image"}, limits_note)


def cmd_key_chroma(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    req = {"input_sha256": rec["sha256"], "color": args.color, "tolerance": args.tolerance, "soft_lo": args.soft_lo,
           "soft_hi": args.soft_hi, "despill": args.despill, "exif": args.exif}
    run = Run(args.out, "key-chroma", req)
    if (prev := run.existing()):
        return prev
    rgb, a0 = to_float(img)
    idx = 1 if args.color == "green" else 2
    others = np.delete(rgb, idx, axis=-1).max(axis=-1)
    dom = (rgb[..., idx] - others) * 255
    t = np.clip(dom / args.tolerance, 0, 1)
    s = np.clip((t - args.soft_lo) / max(1e-6, args.soft_hi - args.soft_lo), 0, 1)
    bgness = s * s * (3 - 2 * s)
    alpha = (1 - bgness) * a0
    out_rgb = rgb.copy()
    if args.despill > 0:
        clamp = np.minimum(rgb[..., idx], others)
        out_rgb[..., idx] = rgb[..., idx] - args.despill * (rgb[..., idx] - clamp)
    out = to_uint8(out_rgb, alpha)
    with run:
        run.add("cutout", "cutout.png", out, relation="derived from input")
        meas, finds = measure_alpha(out, args.profile)
        return run.finish([rec], meas, finds, {"region": "full image"},
                          ["chroma keying assumes an evenly lit screen; spill on translucent edges is reduced, not removed"])


def cmd_shadow_extract(args, limits):
    plate, prec = load_image(args.input, "plate", limits, args.exif)
    inputs, ref, subj = [prec], None, None
    if args.reference:
        ref, r = load_image(args.reference, "reference", limits, args.exif); inputs.append(r)
    if args.subject:
        subj, s = load_image(args.subject, "subject", limits, args.exif); inputs.append(s)
    for other in (ref, subj):
        if other is not None and other.shape != plate.shape:
            raise AtomError("invalid-input", "plate/reference/subject sizes differ; resize explicitly first")
    req = {"inputs": [i["sha256"] for i in inputs], "floor": args.floor, "scale": args.scale,
           "scale_min": args.scale_min, "exif": args.exif}
    run = Run(args.out, "shadow-extract", req)
    if (prev := run.existing()):
        return prev

    def darkness(rgba):
        lum = luminance(to_float(rgba)[0])
        white = max(float(np.percentile(lum, 97)), 1e-6)
        d = np.clip(1 - lum / white - args.floor, 0, 1)
        return np.where(d < 0.03, 0, d)

    dark = darkness(plate)
    scale = args.scale
    if scale <= 0:
        if ref is not None and subj is not None:
            outside = subj[..., 3] < 128
            ratio = darkness(ref)[outside].mean() / max(dark[outside].mean(), 1e-6) if outside.any() else 1.0
            scale = float(np.clip(ratio, args.scale_min, 1.0))
        else:
            scale = 1.0
    alpha = np.clip(dark * scale, 0, 1)
    if subj is not None:
        alpha = alpha * (1 - subj[..., 3] / 255.0)
    out = to_uint8(np.zeros(alpha.shape + (3,)), alpha)
    with run:
        run.add("shadow", "shadow.png", out, relation="black AO layer derived from plate")
        meas = {"scale_used": round(scale, 4), "shadow_alpha_mean": round(float(alpha.mean()), 5),
                "shadow_alpha_max": round(float(alpha.max()), 5), "shadow_px": int((alpha > 0).sum())}
        finds = [] if (alpha > 0).any() else [{"kind": "empty-shadow", "basis": "no darkness above floor"}]
        return run.finish(inputs, meas, finds, {"region": "full image"},
                          ["ambient-occlusion estimate; coloured or directional shadows are approximated in black"])


def cmd_over(args, limits):
    layers, recs = [], []
    for i, p in enumerate(args.layers):
        arr, rec = load_image(p, f"layer{i}{'-top' if i == 0 else ''}", limits, args.exif)
        layers.append(arr); recs.append(rec)
    h, w = layers[0].shape[:2]
    mismatched = [r["role"] for r, l in zip(recs, layers) if l.shape[:2] != (h, w)]
    if mismatched and args.fit == "none":
        raise AtomError("invalid-input", f"layer size mismatch ({', '.join(mismatched)} vs top {w}x{h}); pass --fit stretch|contain to transform explicitly")
    req = {"layers_top_to_bottom": [r["sha256"] for r in recs], "fit": args.fit, "exif": args.exif}
    run = Run(args.out, "over", req)
    if (prev := run.existing()):
        return prev
    transforms = []
    for i in range(1, len(layers)):
        if layers[i].shape[:2] != (h, w):
            transforms.append({"layer": recs[i]["role"], "from": [layers[i].shape[1], layers[i].shape[0]], "to": [w, h], "fit": args.fit})
            layers[i] = fit_layer(layers[i], w, h, args.fit)
    result = layers[-1]
    for layer in reversed(layers[:-1]):
        result = over_pair(layer, result)
    with run:
        run.add("composite", "composite.png", result, relation="straight-alpha over, first input on top")
        meas = {"order": "first input is top", "alpha_convention": "straight (unpremultiplied) 8-bit, float math, round half up",
                "transforms": transforms, "width": w, "height": h}
        return run.finish(recs, meas, [], {"region": "full canvas"},
                          ["no colour management: channels treated as stored sRGB code values"])


def cmd_resize(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    h, w = img.shape[:2]
    width, height = args.width, args.height
    if width is None and height is None:
        raise AtomError("invalid-input", "give --width and/or --height")
    if width is None:
        width = max(1, round(w * height / h))
    if height is None:
        height = max(1, round(h * width / w))
    req = {"input_sha256": rec["sha256"], "width": width, "height": height, "exif": args.exif}
    run = Run(args.out, "resize", req)
    if (prev := run.existing()):
        return prev
    out = resize_rgba(img, width, height)
    with run:
        run.add("resized", "resized.png", out, relation="premultiplied Lanczos resize")
        return run.finish([rec], {"from": [w, h], "to": [width, height], "method": "premultiplied lanczos"}, [],
                          {"region": "full image"}, ["upscaling interpolates; it adds no detail"])


def refine_alpha(rgba_src: np.ndarray, coarse: np.ndarray, band: int = 8, radii=(4, 8, 16)):
    """Classical local-colour matting in a band around a coarse (segmentation) mask.

    Definite foreground/background come from the eroded coarse mask; in the unknown band each pixel's
    alpha is the projection of I-B onto F-B, with F and B the local means of nearby definite pixels.
    Returns (alpha, info). Pixels with no local F or B keep the coarse value.
    """
    rgb = rgba_src[..., :3].astype(np.float64)
    hard = coarse >= 0.5
    fg = hard & ~shift_any(~hard, band)
    bg = ~hard & ~shift_any(hard, band)
    unknown = ~(fg | bg)
    alpha = coarse.astype(np.float64).copy()
    alpha[fg], alpha[bg] = 1.0, 0.0
    todo = unknown.copy()
    solved = np.zeros_like(unknown)
    for r in radii:
        if not todo.any():
            break
        nf, nb = box_sum(fg.astype(np.float64), r), box_sum(bg.astype(np.float64), r)
        ok = todo & (nf >= 3) & (nb >= 3)
        if not ok.any():
            continue
        F = np.stack([box_sum(rgb[..., i] * fg, r) for i in range(3)], -1) / np.maximum(nf, 1)[..., None]
        B = np.stack([box_sum(rgb[..., i] * bg, r) for i in range(3)], -1) / np.maximum(nb, 1)[..., None]
        d = F - B
        den = (d * d).sum(-1)
        good = ok & (den > 400)  # foreground/background colours must differ to be separable
        est = np.clip(((rgb - B) * d).sum(-1) / np.maximum(den, 1e-9), 0, 1)
        alpha[good] = est[good]
        solved |= good
        todo &= ~good
    info = {"band_px": band, "radii": list(radii), "unknown_px": int(unknown.sum()), "solved_px": int(solved.sum()),
            "kept_coarse_px": int(todo.sum())}
    return alpha, info


def cmd_refine(args, limits):
    src, srec = load_image(args.input, "image", limits, args.exif)
    coarse_img, crec = load_image(args.coarse, "coarse", limits, args.exif)
    if coarse_img.shape != src.shape:
        raise AtomError("invalid-input", "coarse matte size differs from image")
    req = {"image": srec["sha256"], "coarse": crec["sha256"], "band": args.band, "exif": args.exif}
    run = Run(args.out, "refine", req)
    if (prev := run.existing()):
        return prev
    alpha, info = refine_alpha(src, coarse_img[..., 3] / 255.0, args.band)
    rgb = src[..., :3].astype(np.float64) / 255.0
    out = to_uint8(rgb, alpha)
    with run:
        run.add("refined", "refined.png", out, relation="original colours with locally solved alpha")
        meas, finds = measure_alpha(out, args.profile)
        meas["refine"] = info
        return run.finish([srec, crec], meas, finds, {"region": "unknown band around the coarse matte"},
                          ["local-colour model: fails where foreground and background colours are similar; colour is not un-mixed"])


def cmd_probe(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    rgb, a = to_float(img)
    q = Image.fromarray(img[..., :3], "RGB").quantize(colors=5, method=Image.Quantize.MEDIANCUT)
    pal = q.getpalette()[:15]
    counts = sorted(q.getcolors(), reverse=True)
    dom = [{"rgb": pal[i * 3:i * 3 + 3], "share": round(c / a.size, 4)} for c, i in counts]
    meas = {"width": rec["width"], "height": rec["height"], "mode": rec["mode"], "format": rec["format"],
            "has_alpha": rec["has_alpha"], "alpha_min": int(img[..., 3].min()), "alpha_max": int(img[..., 3].max()),
            "transparent_ratio": round(float((a == 0).mean()), 5), "brightness_mean": round(float(luminance(rgb).mean() * 255), 2),
            "dominant_colors": dom, "exif_orientation": rec["exif_orientation"], "conversions": rec["conversions"]}
    if args.out:
        run = Run(args.out, "probe", {"input_sha256": rec["sha256"], "exif": args.exif})
        if (prev := run.existing()):
            return prev
        with run:
            return run.finish([rec], meas, [], {"region": "full image"})
    return {"schema": SCHEMA, "command": "probe", "state": "succeeded", "inputs": [rec], "measurements": meas, "producer": producer()}


def cmd_measure(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    req = {"input_sha256": rec["sha256"], "profile": args.profile, "min_stray": args.min_stray,
           "expect_parts": args.expect_parts, "layout": args.layout, "exif": args.exif}
    run = Run(args.out, "measure", req)
    if (prev := run.existing()):
        return prev
    meas, finds = {}, []
    if rec["has_alpha"] or args.profile != "cutout":
        meas["alpha"], f = measure_alpha(img, args.profile, args.min_stray, args.expect_parts)
        finds += f
    if args.layout or not rec["has_alpha"]:
        meas["layout"], f = layout_scan(img)
        finds += f
    with run:
        if finds:
            marked = draw_marks(img, [b for f in finds for b in finding_boxes(f)])
            run.add("annotated-copy", "findings.png", marked, relation="copy of input with finding boxes")
        verdict = "no-measured-findings" if not finds else "findings"
        return run.finish([rec], meas, finds, {"region": "full image at native resolution", "method": "deterministic pixel rules",
                                               "not_covered": ["semantic content", "text legibility"]},
                          ["rules are profile gates, not proof that the image looks right; look at the multibg sheet"],
                          extra={"rule_verdict": verdict})


def finding_boxes(f: dict) -> list:
    boxes = [f["bbox"]] if f.get("bbox") else []
    for b in f.get("boxes") or []:
        boxes.append(b["bbox"] if isinstance(b, dict) else b)
    return [b for b in boxes if b]


def draw_marks(img: np.ndarray, boxes, labels=None, color=(255, 0, 0, 255)):
    im = Image.fromarray(img, "RGBA").copy()
    d = ImageDraw.Draw(im)
    width = max(2, round(max(img.shape[:2]) / 400))
    for i, b in enumerate(boxes):
        if not b:
            continue
        x0, y0, x1, y1 = b
        d.rectangle([x0 - width, y0 - width, x1 + width, y1 + width], outline=color, width=width)
        if labels and labels[i]:
            d.text((x0, max(0, y0 - 12 - width)), labels[i], fill=color)
    return np.asarray(im)


def cmd_diff(args, limits):
    a_img, ra = load_image(args.a, "a", limits, args.exif)
    b_img, rb = load_image(args.b, "b", limits, args.exif)
    bgs = args.backgrounds.split(",")
    for bg in bgs:
        if bg not in BACKGROUNDS:
            raise AtomError("invalid-input", f"unknown background {bg}")
    req = {"a": ra["sha256"], "b": rb["sha256"], "backgrounds": bgs, "tolerance": args.tolerance, "exif": args.exif}
    run = Run(args.out, "diff", req)
    if (prev := run.existing()):
        return prev
    meas, raw, vis = diff_measure(a_img, b_img, bgs, args.tolerance)
    finds = []
    if meas["raw_alpha"]["changed_pixels"]:
        finds.append({"kind": "alpha-changed", **{k: meas["raw_alpha"][k] for k in ("changed_pixels", "bbox_inclusive")}})
    if meas["hidden_rgb_under_full_transparency"]["changed_pixels"]:
        finds.append({"kind": "hidden-rgb-only", "changed_pixels": meas["hidden_rgb_under_full_transparency"]["changed_pixels"],
                      "basis": "RGB differs only where both images are fully transparent; not visible on any background"})
    for bg, s in meas["visible"].items():
        if s["changed_pixels"]:
            finds.append({"kind": "visible-change", "background": bg, "changed_pixels": s["changed_pixels"], "bbox": s["bbox_inclusive"]})
    with run:
        mask = np.zeros(raw.shape + (4,), np.uint8)
        mask[raw] = (255, 0, 0, 255)
        run.add("raw-diff-mask", "raw_diff_mask.png", mask, relation="red = any raw RGBA change")
        if raw.any():
            run.add("marked-b", "marked_b.png", draw_marks(b_img, [meas["raw_any"]["bbox_inclusive"]]), relation="copy of b")
        return run.finish([ra, rb], meas, finds, {"region": "full image", "backgrounds": bgs})


def cmd_multibg(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    req = {"input_sha256": rec["sha256"], "zoom": args.zoom, "exif": args.exif}
    run = Run(args.out, "multibg", req)
    if (prev := run.existing()):
        return prev
    h, w = img.shape[:2]
    scale = min(1.0, 600 / max(h, w))
    tw, th = max(1, round(w * scale)), max(1, round(h * scale))
    small = resize_rgba(img, tw, th) if scale < 1 else img
    tiles = []
    for name in ("white", "black", "checker", "grey", "green", "magenta"):
        tiles.append((name, to_uint8(composite_on(small, name), np.ones(small.shape[:2]))))
    if args.zoom:
        zx0, zy0, zx1, zy1 = [int(v) for v in args.zoom.split(",")]
    else:
        # zoom where the most edge pixels are (a window that actually contains the edge),
        # not the median of edge coordinates, which lands in the centre of symmetric shapes
        a = img[..., 3]
        opaque = a >= 128
        edge = ((a > 0) & (a < 255)) | (opaque & shift_any(~opaque, 1))
        half = max(8, min(h, w) // 8)
        if edge.any():
            density = box_sum(edge.astype(np.float64), half)
            cy, cx = (int(v) for v in np.unravel_index(int(np.argmax(density)), density.shape))
        else:
            cy, cx = h // 2, w // 2
        zx0, zy0, zx1, zy1 = max(0, cx - half), max(0, cy - half), min(w - 1, cx + half), min(h - 1, cy + half)
    crop = img[zy0:zy1 + 1, zx0:zx1 + 1]
    zf = max(1, min(8, 300 // max(1, max(crop.shape[:2]))))
    zoom = np.asarray(Image.fromarray(to_uint8(composite_on(crop, "checker"), np.ones(crop.shape[:2])), "RGBA")
                      .resize((crop.shape[1] * zf, crop.shape[0] * zf), Image.NEAREST))
    pad = 8
    sheet_w = pad + 3 * (tw + pad)
    sheet_h = pad + 2 * (th + pad + 14) + zoom.shape[0] + pad + 14
    sheet = Image.new("RGBA", (max(sheet_w, zoom.shape[1] + 2 * pad), sheet_h), (40, 40, 40, 255))
    d = ImageDraw.Draw(sheet)
    for i, (name, t) in enumerate(tiles):
        x, y = pad + (i % 3) * (tw + pad), pad + (i // 3) * (th + pad + 14)
        d.text((x, y), name, fill=(255, 255, 255, 255))
        sheet.paste(Image.fromarray(t, "RGBA"), (x, y + 14))
    zy = pad + 2 * (th + pad + 14)
    d.text((pad, zy), f"zoom x{zf} of [{zx0},{zy0},{zx1},{zy1}] on checker", fill=(255, 255, 255, 255))
    sheet.paste(Image.fromarray(zoom, "RGBA"), (pad, zy + 14))
    with run:
        run.add("exposure-sheet", "multibg.png", np.asarray(sheet), relation="diagnostic copy; input unchanged")
        return run.finish([rec], {"tile_scale": round(scale, 4), "zoom_box_inclusive": [zx0, zy0, zx1, zy1], "zoom_factor": zf},
                          [], {"backgrounds": [t[0] for t in tiles], "zoom": [zx0, zy0, zx1, zy1]},
                          ["tiles are downscaled; only the zoom box is shown at native pixels"])


def cmd_tiles(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    req = {"input_sha256": rec["sha256"], "tile": args.tile, "overlap": args.overlap, "overview": args.overview, "exif": args.exif}
    run = Run(args.out, "tiles", req)
    if (prev := run.existing()):
        return prev
    h, w = img.shape[:2]
    if args.overlap >= args.tile:
        raise AtomError("invalid-input", "overlap must be smaller than tile")
    step = args.tile - args.overlap
    entries = []
    with run:
        ys = list(range(0, max(1, h - args.overlap), step)) or [0]
        xs = list(range(0, max(1, w - args.overlap), step)) or [0]
        for ty, y in enumerate(ys):
            for tx, x in enumerate(xs):
                y1, x1 = min(h, y + args.tile), min(w, x + args.tile)
                name = f"tile-r{ty:03d}-c{tx:03d}.png"
                run.add("tile", name, img[y:y1, x:x1].copy(), relation="native-resolution crop")
                entries.append({"file": name, "box_inclusive": [x, y, x1 - 1, y1 - 1]})
        scale = min(1.0, args.overview / max(h, w))
        ov = resize_rgba(img, max(1, round(w * scale)), max(1, round(h * scale))) if scale < 1 else img
        run.add("overview", "overview.png", ov, relation=f"downscaled x{scale:.4f} for global context only")
        covered = np.zeros((h, w), bool)
        for e in entries:
            x0, y0, x1, y1 = e["box_inclusive"]
            covered[y0:y1 + 1, x0:x1 + 1] = True
        return run.finish([rec], {"tiles": entries, "overview_scale": round(scale, 5),
                                  "coverage_ratio": float(covered.mean()),
                                  "map": "tile pixel (u,v) -> original (box_x0+u, box_y0+v)"}, [],
                          {"region": "every pixel covered by at least one native tile" if covered.all() else "partial"},
                          ["overview is context only; reading small text requires the native tiles"])


def cmd_layout(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    req = {"input_sha256": rec["sha256"], "tolerance": args.tolerance, "min_band": args.min_band, "exif": args.exif}
    run = Run(args.out, "layout", req)
    if (prev := run.existing()):
        return prev
    meas, finds = layout_scan(img, args.tolerance, args.min_band)
    with run:
        return run.finish([rec], meas, finds, {"region": "full image", "method": "background-distance scan",
                                               "not_covered": ["missing glyphs", "broken image icons", "semantic omissions"]},
                          ["edge contact suggests clipping but legitimate full-bleed designs also touch edges"])


def cmd_annotate(args, limits):
    img, rec = load_image(args.input, "input", limits, args.exif)
    boxes = [parse_box(b) for b in (args.box or [])]
    arrows = [[int(v) for v in a.split(",")[:4]] + [",".join(a.split(",")[4:])] for a in (args.arrow or [])]
    req = {"input_sha256": rec["sha256"], "boxes": args.box or [], "arrows": args.arrow or [], "exif": args.exif}
    run = Run(args.out, "annotate", req)
    if (prev := run.existing()):
        return prev
    h, w = img.shape[:2]
    for (b, _) in boxes:
        if not (0 <= b[0] <= b[2] < w and 0 <= b[1] <= b[3] < h):
            raise AtomError("invalid-input", f"box outside image: {b}")
    out = draw_marks(img, [b for b, _ in boxes], [l for _, l in boxes])
    im = Image.fromarray(out, "RGBA")
    d = ImageDraw.Draw(im)
    for x0, y0, x1, y1, lab in arrows:
        d.line([x0, y0, x1, y1], fill=(255, 140, 0, 255), width=3)
        d.ellipse([x1 - 5, y1 - 5, x1 + 5, y1 + 5], fill=(255, 140, 0, 255))
        if lab:
            d.text((x0, y0), lab, fill=(255, 140, 0, 255))
    with run:
        run.add("annotated-copy", "annotated.png", np.asarray(im), relation="copy; input unchanged")
        return run.finish([rec], {"boxes_inclusive": [b for b, _ in boxes], "arrows": [a[:4] for a in arrows]}, [],
                          {"region": "marks only"})


def cmd_verify(args, limits):
    data = verify_dir(external_dir(args.out))
    drift = []
    for inp in data.get("inputs", []):
        p = Path(inp["path"])
        if p.is_file() and sha256_file(p) != inp["sha256"]:
            drift.append(inp["role"])
    return {"schema": SCHEMA, "command": "verify", "state": "succeeded", "integrity": "verified",
            "verified_command": data["command"], "artifacts": len(data["artifacts"]), "input_drift": drift}


def build_parser():
    p = argparse.ArgumentParser(prog="image_ops.py", description=__doc__)
    p.add_argument("--max-pixels", type=int, default=DEFAULT_MAX_PIXELS)
    p.add_argument("--max-bytes", type=int, default=DEFAULT_MAX_BYTES)
    sub = p.add_subparsers(dest="command", required=True)

    def add(name, fn, *, single=True, out=True):
        s = sub.add_parser(name)
        if single:
            s.add_argument("input")
        if out:
            s.add_argument("--out", required=True)
        s.add_argument("--exif", choices=["apply", "ignore"], default="apply")
        s.set_defaults(fn=fn)
        return s

    s = add("key-white", cmd_key_white)
    s.add_argument("--level", type=float, default=248); s.add_argument("--soft", type=float, default=28)
    s.add_argument("--connectivity", choices=["border", "global"], default="border")
    s.add_argument("--keep-largest", action="store_true"); s.add_argument("--profile", choices=list(PROFILES), default="cutout")
    s = add("key-chroma", cmd_key_chroma)
    s.add_argument("--color", choices=["green", "blue"], default="green"); s.add_argument("--tolerance", type=float, default=60)
    s.add_argument("--soft-lo", type=float, default=0.2); s.add_argument("--soft-hi", type=float, default=0.7)
    s.add_argument("--despill", type=float, default=0.8); s.add_argument("--profile", choices=list(PROFILES), default="cutout")
    s = add("shadow-extract", cmd_shadow_extract)
    s.add_argument("--reference"); s.add_argument("--subject"); s.add_argument("--floor", type=float, default=0.02)
    s.add_argument("--scale", type=float, default=0.0); s.add_argument("--scale-min", type=float, default=0.3)
    s = add("over", cmd_over, single=False)
    s.add_argument("layers", nargs="+", help="first is top"); s.add_argument("--fit", choices=["none", "stretch", "contain"], default="none")
    s = add("resize", cmd_resize)
    s.add_argument("--width", type=int); s.add_argument("--height", type=int)
    s = add("refine", cmd_refine); s.add_argument("--coarse", required=True); s.add_argument("--band", type=int, default=8)
    s.add_argument("--profile", choices=list(PROFILES), default="translucent")
    s = sub.add_parser("probe"); s.add_argument("input"); s.add_argument("--out")
    s.add_argument("--exif", choices=["apply", "ignore"], default="apply"); s.set_defaults(fn=cmd_probe)
    s = add("measure", cmd_measure)
    s.add_argument("--profile", choices=list(PROFILES), default="cutout"); s.add_argument("--min-stray", type=int)
    s.add_argument("--expect-parts", type=int); s.add_argument("--layout", action="store_true")
    s = add("diff", cmd_diff, single=False)
    s.add_argument("a"); s.add_argument("b"); s.add_argument("--backgrounds", default="white,black,checker")
    s.add_argument("--tolerance", type=int, default=0)
    s = add("multibg", cmd_multibg); s.add_argument("--zoom")
    s = add("tiles", cmd_tiles)
    s.add_argument("--tile", type=int, default=1600); s.add_argument("--overlap", type=int, default=160)
    s.add_argument("--overview", type=int, default=1568)
    s = add("layout", cmd_layout); s.add_argument("--tolerance", type=int, default=24); s.add_argument("--min-band", type=int)
    s = add("annotate", cmd_annotate); s.add_argument("--box", action="append"); s.add_argument("--arrow", action="append")
    s = sub.add_parser("verify"); s.add_argument("--out", required=True); s.set_defaults(fn=cmd_verify)
    return p


def main(argv=None) -> int:
    args = build_parser().parse_args(argv)
    limits = {"max_pixels": args.max_pixels, "max_bytes": args.max_bytes}
    try:
        result = args.fn(args, limits)
    except AtomError as exc:
        sys.stderr.write(json.dumps({"state": "failed", "reason": exc.reason, "message": str(exc)}, ensure_ascii=False) + "\n")
        return 2 if exc.reason in ("invalid-input", "unsupported", "too-large", "conflict") else 3
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
