#!/usr/bin/env python3
"""Build a stable YouTube-ID filename mirror without mutating the C0 cache."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
from datetime import datetime, timezone
from pathlib import Path


VIDEO_ID = re.compile(r"^[A-Za-z0-9_-]{11}$")
DATA_ROOT = Path(r"D:\BabataData\04_runtime\staging\model-workspaces")
SOURCE_ROOT = Path(r"E:\Cherno")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", type=Path, required=True)
    parser.add_argument("--output-root", type=Path, required=True)
    parser.add_argument("--verify-only", action="store_true")
    return parser.parse_args()


def ensure_within(path: Path, root: Path, label: str) -> Path:
    resolved = path.resolve()
    root_resolved = root.resolve()
    try:
        resolved.relative_to(root_resolved)
    except ValueError as exc:
        raise RuntimeError(f"{label} escapes its allowed root: {resolved}") from exc
    return resolved


def main() -> int:
    args = parse_args()
    manifest_path = args.manifest.resolve()
    output_root = ensure_within(args.output_root, DATA_ROOT, "output root")
    if not manifest_path.is_file():
        raise FileNotFoundError(manifest_path)

    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    items = manifest.get("items")
    if not isinstance(items, list) or len(items) != 269:
        raise RuntimeError(f"expected 269 manifest items, got {len(items) if isinstance(items, list) else 'invalid'}")

    if not args.verify_only:
        (output_root / "media").mkdir(parents=True, exist_ok=True)

    records: list[dict[str, object]] = []
    total_bytes = 0
    for index, item in enumerate(items, start=1):
        video_id = item.get("video_id")
        if not isinstance(video_id, str) or not VIDEO_ID.fullmatch(video_id):
            raise RuntimeError(f"invalid YouTube video ID at item {index}: {video_id!r}")

        source_path = Path(item["local_mapping"]["local_path"])
        ensure_within(source_path, SOURCE_ROOT, "source path")
        if not source_path.is_file():
            raise FileNotFoundError(source_path)

        expected_hash = item["local_media"]["sha256"]
        expected_size = item["local_media"]["size_bytes"]
        if sha256(source_path) != expected_hash:
            raise RuntimeError(f"source hash changed before copy: {source_path}")
        if source_path.stat().st_size != expected_size:
            raise RuntimeError(f"source size changed before copy: {source_path}")

        target_path = output_root / "media" / f"{video_id}.mp4"
        ensure_within(target_path, output_root, "target path")
        if target_path.exists():
            if target_path.stat().st_size != expected_size or sha256(target_path) != expected_hash:
                raise RuntimeError(f"existing target does not match source; refusing overwrite: {target_path}")
        elif args.verify_only:
            raise RuntimeError(f"missing target in verify-only mode: {target_path}")
        else:
            shutil.copyfile(source_path, target_path)

        target_hash = sha256(target_path)
        target_size = target_path.stat().st_size
        if target_hash != expected_hash or target_size != expected_size:
            raise RuntimeError(f"target read-back mismatch: {target_path}")
        total_bytes += target_size
        records.append(
            {
                "video_id": video_id,
                "course_slug": item["course_slug"],
                "original_title": item["original_title"],
                "source_url": item["source_url"],
                "playlist_id": item["playlist_id"],
                "playlist_title": item["playlist_title"],
                "playlist_position_observed": item["playlist_position_observed"],
                "source_path": str(source_path),
                "target_path": str(target_path),
                "mapping_confidence": item["local_mapping"]["mapping_confidence"],
                "sha256": target_hash,
                "size_bytes": target_size,
                "duration_seconds": item["local_media"]["duration_seconds_local"],
            }
        )
        print(f"[{index:03d}/269] {video_id}.mp4", flush=True)

    if args.verify_only:
        return 0

    output_manifest = {
        "schema": "babata.cherno-id-cache/v1",
        "status": "verified",
        "created_at": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
        "source_manifest": str(manifest_path),
        "source_manifest_sha256": sha256(manifest_path),
        "source_root": str(SOURCE_ROOT),
        "output_root": str(output_root),
        "filename_policy": "<youtube_video_id>.mp4",
        "original_title_policy": "preserved in metadata records; never encoded into the filename",
        "source_mutation": False,
        "items": records,
        "summary": {"items": len(records), "total_bytes": total_bytes, "failed": 0},
    }
    (output_root / "id-cache-manifest.json").write_text(
        json.dumps(output_manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    (output_root / "REPORT.md").write_text(
        "# Cherno stable-ID cache\n\n"
        f"- Status: `verified`\n- Items: `{len(records)}/269`\n"
        f"- Total bytes: `{total_bytes}`\n"
        "- Source C0: `E:\\Cherno` (read-only; no rename, overwrite, or deletion)\n"
        "- Filename policy: `<youtube_video_id>.mp4`\n"
        "- Original title, URL, playlist identity, order, source path and hashes: `id-cache-manifest.json`\n",
        encoding="utf-8",
    )
    print(f"verified {len(records)}/269 items, {total_bytes} bytes", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
