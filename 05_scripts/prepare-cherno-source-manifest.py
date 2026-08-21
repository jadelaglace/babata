#!/usr/bin/env python3
"""Build a sanitized Cherno source manifest without changing cached media."""

from __future__ import annotations

import argparse
import concurrent.futures
import datetime as dt
import difflib
import hashlib
import json
import re
import subprocess
import unicodedata
from collections import Counter
from pathlib import Path


COURSES = (
    {
        "slug": "cpp",
        "title": "C++",
        "playlist_id": "PLlrATfBNZ98dudnM48yfGUldqGD0S4FFb",
        "relative_dir": "C++",
        "expected": 115,
        "position_alignment_observed": False,
    },
    {
        "slug": "opengl",
        "title": "OpenGL",
        "playlist_id": "PLlrATfBNZ98foTJPJ_Ev03o2oq3-GGOS2",
        "relative_dir": "OpenGL",
        "expected": 31,
        "position_alignment_observed": True,
    },
    {
        "slug": "game-engine",
        "title": "Game Engine",
        "playlist_id": "PLlrATfBNZ98dC-V-N3m0Go4deliWHPFwT",
        "relative_dir": "Game Engine",
        "expected": 123,
        "position_alignment_observed": True,
    },
)

VIDEO_ID_RE = re.compile(r"^[A-Za-z0-9_-]{11}$")
LEGACY_NAME_RE = re.compile(r"^(?P<index>[0-9]{4})\.\s*(?P<title>.+)$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--cache-root", required=True, type=Path)
    parser.add_argument("--workspace", required=True, type=Path)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--ffprobe", default="ffprobe")
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--reuse-inventory", action="store_true")
    return parser.parse_args()


def read_flat_playlist(path: Path, course: dict) -> list[dict]:
    rows: list[dict] = []
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if line.startswith('"{') and line.endswith('}"'):
            line = line[1:-1]
        if not line:
            continue
        try:
            raw = json.loads(line)
        except json.JSONDecodeError as exc:
            raise ValueError(f"invalid yt-dlp JSON at {path}:{line_number}: {exc}") from exc
        video_id = str(raw.get("id", ""))
        if not VIDEO_ID_RE.fullmatch(video_id):
            raise ValueError(f"invalid YouTube video id at {path}:{line_number}: {video_id!r}")
        row = {
            "video_id": video_id,
            "original_title": str(raw.get("title", "")).strip(),
            "source_url": f"https://www.youtube.com/watch?v={video_id}",
            "playlist_id": str(raw.get("playlist_id", "")),
            "playlist_title": str(raw.get("playlist_title") or raw.get("playlist") or course["title"]),
            "playlist_position_observed": int(raw.get("playlist_index")),
            "playlist_count_observed": int(raw.get("playlist_count") or raw.get("n_entries")),
            "duration_seconds_source": float(raw["duration"]) if raw.get("duration") is not None else None,
            "channel_id": raw.get("channel_id") or raw.get("playlist_channel_id"),
            "channel_title": raw.get("channel") or raw.get("uploader") or raw.get("playlist_channel"),
        }
        if row["playlist_id"] != course["playlist_id"]:
            raise ValueError(f"playlist id mismatch in {path}: {row['playlist_id']}")
        rows.append(row)
    rows.sort(key=lambda row: row["playlist_position_observed"])
    if len(rows) != course["expected"]:
        raise ValueError(f"{course['slug']} expected {course['expected']} source rows, found {len(rows)}")
    if [row["playlist_position_observed"] for row in rows] != list(range(1, len(rows) + 1)):
        raise ValueError(f"{course['slug']} playlist positions are not contiguous")
    if len({row["video_id"] for row in rows}) != len(rows):
        raise ValueError(f"{course['slug']} contains duplicate video ids")
    return rows


def read_sanitized_playlist(path: Path, course: dict) -> list[dict]:
    rows = [
        json.loads(line)
        for line in path.read_text(encoding="utf-8").splitlines()
        if line.strip()
    ]
    rows = [row for row in rows if row.get("course_slug") == course["slug"]]
    rows.sort(key=lambda row: row["playlist_position_observed"])
    if len(rows) != course["expected"]:
        raise ValueError(f"{course['slug']} expected {course['expected']} sanitized source rows, found {len(rows)}")
    if any(row["playlist_id"] != course["playlist_id"] for row in rows):
        raise ValueError(f"{course['slug']} sanitized source rows contain a playlist mismatch")
    if [row["playlist_position_observed"] for row in rows] != list(range(1, len(rows) + 1)):
        raise ValueError(f"{course['slug']} sanitized playlist positions are not contiguous")
    if len({row["video_id"] for row in rows}) != len(rows):
        raise ValueError(f"{course['slug']} sanitized source rows contain duplicate video ids")
    return rows


def normalize_title(value: str) -> str:
    value = unicodedata.normalize("NFKC", value).lower()
    replacements = (
        ("c++", " cplusplus "),
        ("c#", " csharp "),
        ("std::", " std "),
        ("--", " "),
        ("&", " and "),
        ("|", " or "),
        ("+", " plus "),
    )
    for old, new in replacements:
        value = value.replace(old, new)
    value = re.sub(r"\b(?:the\s+)?game\s+engine\s+series\b", " ", value)
    value = re.sub(r"\b0+([1-9][0-9]*d)\b", r"\1", value)
    value = re.sub(r"[^a-z0-9]+", " ", value)
    return " ".join(value.split())


def title_similarity(left: str, right: str) -> float:
    return difflib.SequenceMatcher(None, normalize_title(left), normalize_title(right)).ratio()


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        while chunk := stream.read(8 * 1024 * 1024):
            digest.update(chunk)
    return digest.hexdigest()


def rational_to_float(value: str | None) -> float | None:
    if not value or value == "0/0":
        return None
    numerator, separator, denominator = value.partition("/")
    try:
        return float(numerator) / float(denominator) if separator else float(value)
    except (ValueError, ZeroDivisionError):
        return None


def probe_file(path: Path, ffprobe: str, course_slug: str) -> dict:
    completed = subprocess.run(
        [
            ffprobe,
            "-v",
            "error",
            "-show_entries",
            "format=duration,size:stream=index,codec_type,codec_name,width,height,pix_fmt,avg_frame_rate,sample_rate,channels",
            "-of",
            "json",
            str(path),
        ],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    payload = json.loads(completed.stdout)
    streams = []
    for stream in payload.get("streams", []):
        item = {
            "index": int(stream["index"]),
            "codec_type": stream.get("codec_type"),
            "codec_name": stream.get("codec_name"),
        }
        for field in ("width", "height", "pix_fmt", "sample_rate", "channels"):
            if stream.get(field) is not None:
                item[field] = int(stream[field]) if field in {"width", "height", "sample_rate", "channels"} else stream[field]
        if stream.get("avg_frame_rate") is not None:
            item["avg_frame_rate"] = stream["avg_frame_rate"]
            item["avg_frame_rate_decimal"] = rational_to_float(stream["avg_frame_rate"])
        streams.append(item)
    stem = path.stem
    legacy = LEGACY_NAME_RE.fullmatch(stem)
    return {
        "course_slug": course_slug,
        "local_path": str(path.resolve()),
        "local_filename": path.name,
        "size_bytes": int(payload.get("format", {}).get("size") or path.stat().st_size),
        "sha256": sha256_file(path),
        "duration_seconds_local": float(payload.get("format", {}).get("duration")),
        "streams": streams,
        "embedded_subtitle_streams": sum(1 for stream in streams if stream["codec_type"] == "subtitle"),
        "filename_video_id": stem if VIDEO_ID_RE.fullmatch(stem) else None,
        "legacy_position": int(legacy.group("index")) if legacy else None,
        "legacy_title": legacy.group("title") if legacy else None,
    }


def probe_normalized_audio(path: Path, ffprobe: str, expected_duration: float) -> dict:
    completed = subprocess.run(
        [
            ffprobe,
            "-v",
            "error",
            "-show_entries",
            "format=duration,size:stream=codec_type,codec_name,sample_rate,channels",
            "-of",
            "json",
            str(path),
        ],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    payload = json.loads(completed.stdout)
    audio_streams = [stream for stream in payload.get("streams", []) if stream.get("codec_type") == "audio"]
    if len(audio_streams) != 1:
        raise ValueError(f"normalized audio must contain exactly one audio stream: {path}")
    stream = audio_streams[0]
    duration = float(payload["format"]["duration"])
    if stream.get("codec_name") != "flac" or int(stream.get("sample_rate", 0)) != 16000 or int(stream.get("channels", 0)) != 1:
        raise ValueError(f"normalized audio is not FLAC mono 16 kHz: {path}")
    if abs(duration - expected_duration) > 0.25:
        raise ValueError(f"normalized audio duration drift exceeds 0.25 seconds: {path}")
    digest = sha256_file(path)
    return {
        "path": str(path.resolve()),
        "sha256": digest,
        "provider_input_sha256": digest,
        "size_bytes": int(payload["format"].get("size") or path.stat().st_size),
        "duration_seconds": duration,
        "codec": stream["codec_name"],
        "sample_rate_hz": int(stream["sample_rate"]),
        "channels": int(stream["channels"]),
    }


def pair_score(local: dict, source: dict) -> tuple[float, float, float]:
    similarity = title_similarity(local["legacy_title"] or "", source["original_title"])
    source_duration = source["duration_seconds_source"]
    duration_delta = abs(local["duration_seconds_local"] - source_duration) if source_duration is not None else 9999.0
    duration_score = max(0.0, 1.0 - duration_delta / 30.0)
    position_delta = abs((local["legacy_position"] or 0) - source["playlist_position_observed"])
    position_score = max(0.0, 1.0 - position_delta / 25.0)
    score = 0.72 * similarity + 0.24 * duration_score + 0.04 * position_score
    return score, similarity, duration_delta


def map_course(course: dict, locals_: list[dict], sources: list[dict]) -> tuple[list[dict], list[dict]]:
    by_id = {row["video_id"]: row for row in sources}
    mapped_local: set[str] = set()
    mapped_source: set[str] = set()
    mappings: list[dict] = []

    def add_mapping(local: dict, source: dict, method: str, similarity: float, delta: float, score: float, margin: float) -> None:
        if method == "filename_video_id":
            confidence = "exact"
        elif normalize_title(local["legacy_title"] or "") == normalize_title(source["original_title"]) and delta <= 5.0:
            confidence = "high"
        elif delta <= 3.0 and similarity >= 0.68 and margin >= 0.04:
            confidence = "high"
        elif delta <= 3.0 and similarity >= 0.82:
            confidence = "high"
        else:
            confidence = "review_required"
        mappings.append(
            {
                "video_id": source["video_id"],
                "local_path": local["local_path"],
                "local_filename": local["local_filename"],
                "mapping_method": method,
                "mapping_confidence": confidence,
                "title_similarity": round(similarity, 6),
                "duration_delta_seconds": round(delta, 6),
                "score": round(score, 6),
                "score_margin": round(margin, 6),
            }
        )
        mapped_local.add(local["local_path"])
        mapped_source.add(source["video_id"])

    for local in locals_:
        video_id = local["filename_video_id"]
        if video_id and video_id in by_id:
            source = by_id[video_id]
            delta = abs(local["duration_seconds_local"] - (source["duration_seconds_source"] or 0.0))
            add_mapping(local, source, "filename_video_id", 1.0, delta, 1.0, 1.0)

    remaining_locals = [row for row in locals_ if row["local_path"] not in mapped_local]
    remaining_sources = [row for row in sources if row["video_id"] not in mapped_source]

    normalized_sources: dict[str, list[dict]] = {}
    for source in remaining_sources:
        normalized_sources.setdefault(normalize_title(source["original_title"]), []).append(source)
    for local in list(remaining_locals):
        candidates = normalized_sources.get(normalize_title(local["legacy_title"] or ""), [])
        candidates = [source for source in candidates if source["video_id"] not in mapped_source]
        if len(candidates) != 1:
            continue
        source = candidates[0]
        score, similarity, delta = pair_score(local, source)
        if delta <= 10.0:
            add_mapping(local, source, "normalized_title_duration", similarity, delta, score, 1.0)

    remaining_locals = [row for row in locals_ if row["local_path"] not in mapped_local]
    remaining_sources = [row for row in sources if row["video_id"] not in mapped_source]
    if course["position_alignment_observed"]:
        source_by_position = {row["playlist_position_observed"]: row for row in remaining_sources}
        for local in list(remaining_locals):
            source = source_by_position.get(local["legacy_position"])
            if not source or source["video_id"] in mapped_source:
                continue
            score, similarity, delta = pair_score(local, source)
            if delta <= 3.0 and similarity >= 0.5:
                add_mapping(local, source, "legacy_position_title_duration", similarity, delta, score, 1.0)

    remaining_locals = [row for row in locals_ if row["local_path"] not in mapped_local]
    remaining_sources = [row for row in sources if row["video_id"] not in mapped_source]
    ranked: dict[str, list[tuple[float, float, float, dict]]] = {}
    edges: list[tuple[float, str, str, dict, dict, float, float]] = []
    for local in remaining_locals:
        candidates = []
        for source in remaining_sources:
            score, similarity, delta = pair_score(local, source)
            candidates.append((score, similarity, delta, source))
            edges.append((score, local["local_path"], source["video_id"], local, source, similarity, delta))
        ranked[local["local_path"]] = sorted(candidates, key=lambda row: row[0], reverse=True)

    for score, _, _, local, source, similarity, delta in sorted(edges, key=lambda row: row[0], reverse=True):
        if local["local_path"] in mapped_local or source["video_id"] in mapped_source:
            continue
        second_score = ranked[local["local_path"]][1][0] if len(ranked[local["local_path"]]) > 1 else 0.0
        margin = score - second_score
        if delta <= 15.0 and similarity >= 0.55:
            add_mapping(local, source, "global_title_duration", similarity, delta, score, margin)

    return mappings, [mapping for mapping in mappings if mapping["mapping_confidence"] == "review_required"]


def write_ndjson(path: Path, rows: list[dict]) -> None:
    content = "".join(json.dumps(row, ensure_ascii=False, sort_keys=True) + "\n" for row in rows)
    path.write_text(content, encoding="utf-8")


def main() -> None:
    args = parse_args()
    workspace = args.workspace.resolve()
    source_dir = workspace / "source"
    inventory_dir = workspace / "inventory"
    results_dir = workspace / "results"
    inventory_dir.mkdir(parents=True, exist_ok=True)
    results_dir.mkdir(parents=True, exist_ok=True)

    all_sources: list[dict] = []
    source_by_course: dict[str, list[dict]] = {}
    sanitized_source_path = source_dir / "source-metadata.ndjson"
    raw_source_available = all((source_dir / f"{course['slug']}.raw.ndjson").exists() for course in COURSES)
    for course in COURSES:
        if raw_source_available:
            rows = read_flat_playlist(source_dir / f"{course['slug']}.raw.ndjson", course)
            for row in rows:
                row["course_slug"] = course["slug"]
                row["course_title"] = course["title"]
        elif sanitized_source_path.exists():
            rows = read_sanitized_playlist(sanitized_source_path, course)
        else:
            raise ValueError("neither complete raw playlist metadata nor sanitized source metadata is available")
        source_by_course[course["slug"]] = rows
        all_sources.extend(rows)
    write_ndjson(sanitized_source_path, all_sources)

    media_jobs: list[tuple[Path, str]] = []
    for course in COURSES:
        course_root = args.cache_root / course["relative_dir"]
        files = sorted(course_root.glob("*.mp4"), key=lambda path: path.name.casefold())
        if len(files) != course["expected"]:
            raise ValueError(f"{course['slug']} expected {course['expected']} MP4 files, found {len(files)}")
        media_jobs.extend((path, course["slug"]) for path in files)

    inventory_path = inventory_dir / "local-inventory.ndjson"
    if args.reuse_inventory and inventory_path.exists():
        inventory = [json.loads(line) for line in inventory_path.read_text(encoding="utf-8").splitlines() if line.strip()]
        expected_paths = {str(path.resolve()) for path, _ in media_jobs}
        if len(inventory) != len(media_jobs) or {row["local_path"] for row in inventory} != expected_paths:
            raise ValueError("existing inventory does not match the current cache file set")
        print(json.dumps({"reused_inventory": len(inventory)}))
    else:
        inventory = []
        with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
            futures = {
                executor.submit(probe_file, path, args.ffprobe, slug): path for path, slug in media_jobs
            }
            for completed, future in enumerate(concurrent.futures.as_completed(futures), 1):
                inventory.append(future.result())
                if completed % 25 == 0 or completed == len(futures):
                    print(json.dumps({"probed_and_hashed": completed, "total": len(futures)}))
        inventory.sort(key=lambda row: (row["course_slug"], row["local_filename"].casefold()))
        write_ndjson(inventory_path, inventory)

    mappings: list[dict] = []
    review_required: list[dict] = []
    for course in COURSES:
        local_rows = [row for row in inventory if row["course_slug"] == course["slug"]]
        course_mappings, course_review = map_course(course, local_rows, source_by_course[course["slug"]])
        mappings.extend(course_mappings)
        review_required.extend(course_review)
    mappings.sort(key=lambda row: row["local_path"].casefold())
    write_ndjson(inventory_dir / "legacy-map.ndjson", mappings)

    mapping_by_video_id = {row["video_id"]: row for row in mappings}
    local_by_path = {row["local_path"]: row for row in inventory}
    manifest_items = []
    for source in all_sources:
        mapping = mapping_by_video_id.get(source["video_id"])
        local = local_by_path.get(mapping["local_path"]) if mapping else None
        manifest_items.append(
            {
                **source,
                "local_mapping": mapping,
                "local_media": (
                    {
                        "local_path": local["local_path"],
                        "local_filename": local["local_filename"],
                        "size_bytes": local["size_bytes"],
                        "sha256": local["sha256"],
                        "duration_seconds_local": local["duration_seconds_local"],
                        "streams": local["streams"],
                        "embedded_subtitle_streams": local["embedded_subtitle_streams"],
                    }
                    if local
                    else None
                ),
                "readiness": "prepared" if local else "missing",
                "formal_c0_registration": "not_registered",
            }
        )
    manifest_items.sort(key=lambda row: (row["course_slug"], row["playlist_position_observed"]))

    counts = Counter(item["course_slug"] for item in manifest_items)
    mapped_count = sum(item["local_mapping"] is not None for item in manifest_items)
    subtitle_count = sum(item["local_media"]["embedded_subtitle_streams"] > 0 for item in manifest_items if item["local_media"])
    confidence_counts = Counter(mapping["mapping_confidence"] for mapping in mappings)
    manifest = {
        "schema": "babata.external-course-source-manifest/v1",
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "scope": "Cherno C++, OpenGL, and Game Engine YouTube playlists mapped to the protected E:\\Cherno cache",
        "identity_policy": {
            "stable_external_id": "youtube_video_id",
            "physical_filename_for_new_acquisitions": "<youtube_video_id>.mp4",
            "playlist_position": "observed mutable metadata only",
            "original_title": "verbatim YouTube playlist metadata",
        },
        "source_evidence": {
            "tool": "yt-dlp",
            "tool_version": "2026.07.04",
            "mode": "flat-playlist whitelist",
            "signed_urls_retained": False,
        },
        "local_probe": {"tool": "ffprobe", "hash": "sha256", "source_mutation": False},
        "sovereignty_depth": "C0-A2",
        "readiness": "prepared",
        "formal_registration": {
            "state": "not_registered",
            "reason": "manifest preparation does not itself register C0; use the enabled source.youtube Collector route",
        },
        "summary": {
            "source_items": len(manifest_items),
            "local_mp4": len(inventory),
            "mapped": mapped_count,
            "unmapped_source": len(manifest_items) - mapped_count,
            "mapping_confidence": dict(sorted(confidence_counts.items())),
            "local_files_with_embedded_subtitles": subtitle_count,
            "course_counts": dict(sorted(counts.items())),
        },
        "items": manifest_items,
    }
    (results_dir / "source-manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    unresolved_sources = [item for item in manifest_items if item["local_mapping"] is None]
    mapped_paths = {row["local_path"] for row in mappings}
    unresolved_locals = [row for row in inventory if row["local_path"] not in mapped_paths]
    (results_dir / "mapping-review.json").write_text(
        json.dumps(
            {
                "schema": "babata.external-course-mapping-review/v1",
                "review_required": review_required,
                "unresolved_sources": unresolved_sources,
                "unresolved_locals": unresolved_locals,
            },
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )
    representative_samples = []
    ffmpeg_version = subprocess.run(
        [args.ffmpeg, "-version"],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout.splitlines()[0]
    for course in COURSES:
        candidates = [
            item
            for item in manifest_items
            if item["course_slug"] == course["slug"]
            and item["local_media"]
            and item["local_media"]["duration_seconds_local"] >= 300.0
            and item["local_media"]["embedded_subtitle_streams"] == 0
            and item["local_mapping"]["mapping_confidence"] in {"exact", "high"}
        ]
        if not candidates:
            raise ValueError(f"{course['slug']} has no eligible representative sample")
        selected = min(candidates, key=lambda item: item["local_media"]["duration_seconds_local"])
        normalized_audio_path = workspace / "preprocessed" / course["slug"] / f"{selected['video_id']}.flac"
        normalized_audio = None
        if normalized_audio_path.exists():
            normalized_audio = probe_normalized_audio(
                normalized_audio_path,
                args.ffprobe,
                selected["local_media"]["duration_seconds_local"],
            )
            normalized_audio.update(
                {
                    "tool": "ffmpeg",
                    "tool_version": ffmpeg_version,
                    "preprocessing": [
                        "selected first audio stream only",
                        "full authorized duration",
                        "FLAC lossless audio",
                        "mono",
                        "16000 Hz",
                    ],
                }
            )
        representative_samples.append(
            {
                "course_slug": course["slug"],
                "course_title": course["title"],
                "video_id": selected["video_id"],
                "original_title": selected["original_title"],
                "playlist_position_observed": selected["playlist_position_observed"],
                "source_url": selected["source_url"],
                "source_mp4_path": selected["local_media"]["local_path"],
                "source_mp4_sha256": selected["local_media"]["sha256"],
                "duration_seconds": selected["local_media"]["duration_seconds_local"],
                "embedded_subtitle_streams": selected["local_media"]["embedded_subtitle_streams"],
                "normalized_audio_path": str(normalized_audio_path.resolve()),
                "normalized_audio": normalized_audio,
                "selection_basis": "shortest complete episode at least 300 seconds with no embedded subtitle stream and exact/high source mapping",
            }
        )
    (results_dir / "representative-samples.json").write_text(
        json.dumps(
            {"schema": "babata.cherno-representative-samples/v1", "items": representative_samples},
            ensure_ascii=False,
            indent=2,
        ),
        encoding="utf-8",
    )
    report = "\n".join(
        (
            "# Cherno course source-manifest report",
            "",
            f"- Source rows: {len(manifest_items)} ({dict(sorted(counts.items()))}).",
            f"- Local MP4 files: {len(inventory)}; mapped: {mapped_count}; unmapped: {len(manifest_items) - mapped_count}.",
            f"- Mapping confidence: {dict(sorted(confidence_counts.items()))}; review required: {len(review_required)}.",
            f"- Local files with embedded subtitle streams: {subtitle_count}; subtitle streams were observed only and are excluded from new C1 input.",
            "- Identity: YouTube video ID is stable; playlist position remains observed mutable metadata.",
            "- Media policy: source MP4 files were read only; no video was renamed, overwritten, deleted, or transcoded.",
            "- Registration status: this output is C0-A2 prepared only; formal C0 requires selection through the enabled source.youtube Collector route.",
            "- Sensitive-data policy: only whitelisted source metadata is retained; raw yt-dlp rows must be deleted after this report passes review.",
            "- Representative samples: one shortest eligible complete episode per course is frozen in results/representative-samples.json.",
            "",
        )
    )
    (workspace / "REPORT.md").write_text(report, encoding="utf-8")

    if len(inventory) != len(all_sources) or mapped_count != len(all_sources) or review_required:
        raise SystemExit(
            f"manifest requires review: sources={len(all_sources)} local={len(inventory)} mapped={mapped_count} review={len(review_required)}"
        )
    print(json.dumps(manifest["summary"], ensure_ascii=False, sort_keys=True))


if __name__ == "__main__":
    main()
