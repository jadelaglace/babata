#!/usr/bin/env python3
"""Build a resumable full-course C1B visual candidate round for Cherno.

Long lessons are reviewed through complete chronological chunks.  Each chunk is
represented by an annotated contact sheet and the matching timestamped ASR span;
accepted evidence is then cut from the original read-only MP4.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tempfile
import threading
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from PIL import Image, ImageDraw, ImageFont


TIMESTAMP_RE = re.compile(
    r"^\[(?P<start>\d{2}:\d{2}:\d{2}\.\d{3}) --> "
    r"(?P<end>\d{2}:\d{2}:\d{2}\.\d{3})\]\s*(?P<text>.*)$"
)


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def sha256_json(value: Any) -> str:
    payload = json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    return hashlib.sha256(payload.encode("utf-8")).hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(temporary, path)


def seconds_from_timestamp(value: str) -> float:
    hours, minutes, rest = value.split(":")
    seconds, milliseconds = rest.split(".")
    return int(hours) * 3600 + int(minutes) * 60 + int(seconds) + int(milliseconds) / 1000


def display_timestamp(seconds: float) -> str:
    milliseconds = max(0, int(round(seconds * 1000)))
    hours, milliseconds = divmod(milliseconds, 3_600_000)
    minutes, milliseconds = divmod(milliseconds, 60_000)
    secs, milliseconds = divmod(milliseconds, 1000)
    return f"{hours:02d}:{minutes:02d}:{secs:02d}.{milliseconds:03d}"


def run(command: list[str], *, timeout: int | None = None) -> subprocess.CompletedProcess[str]:
    completed = subprocess.run(
        command,
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        timeout=timeout,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"Command failed ({completed.returncode}): {' '.join(command)}\n"
            f"stdout: {completed.stdout[-4000:]}\nstderr: {completed.stderr[-4000:]}"
        )
    return completed


def find_single(rows: list[dict[str, Any]], key: str, value: str, label: str) -> dict[str, Any]:
    found = [row for row in rows if str(row.get(key)) == value]
    if len(found) != 1:
        raise RuntimeError(f"Expected one {label} for {value}, found {len(found)}")
    return found[0]


def parse_transcript(path: Path) -> list[dict[str, Any]]:
    rows: list[dict[str, Any]] = []
    for line in path.read_text(encoding="utf-8-sig").splitlines():
        match = TIMESTAMP_RE.match(line)
        if not match:
            continue
        rows.append(
            {
                "start": seconds_from_timestamp(match.group("start")),
                "end": seconds_from_timestamp(match.group("end")),
                "line": line,
            }
        )
    return rows


def transcript_slice(rows: list[dict[str, Any]], start: float, end: float) -> str:
    selected = [row["line"] for row in rows if row["end"] >= start and row["start"] <= end]
    if not selected:
        return "(No ASR sentence overlapped this chunk.)"
    text = "\n".join(selected)
    return text if len(text) <= 70_000 else text[:70_000] + "\n[truncated at 70,000 characters]"


def load_font(size: int) -> ImageFont.ImageFont:
    candidates = [
        Path(r"C:\Windows\Fonts\segoeui.ttf"),
        Path(r"C:\Windows\Fonts\arial.ttf"),
    ]
    for candidate in candidates:
        if candidate.is_file():
            return ImageFont.truetype(str(candidate), size=size)
    return ImageFont.load_default()


def build_contact_sheet(
    *,
    ffmpeg: str,
    video: Path,
    output: Path,
    start: float,
    duration: float,
    frame_interval: float,
    max_frame_width: int,
    binding: dict[str, Any],
) -> None:
    sidecar = output.with_suffix(".binding.json")
    if output.is_file() and sidecar.is_file() and read_json(sidecar) == binding:
        return

    output.parent.mkdir(parents=True, exist_ok=True)
    temporary_root = Path(tempfile.mkdtemp(prefix="cherno-contact-", dir=str(output.parent)))
    try:
        frame_pattern = temporary_root / "frame-%04d.jpg"
        filter_graph = (
            f"select='isnan(prev_selected_t)+gte(t-prev_selected_t\\,{frame_interval})',"
            f"scale='min({max_frame_width},iw)':-2"
        )
        run(
            [
                ffmpeg,
                "-hide_banner",
                "-loglevel",
                "error",
                "-ss",
                f"{start:.3f}",
                "-i",
                str(video),
                "-t",
                f"{duration:.3f}",
                "-vf",
                filter_graph,
                "-fps_mode",
                "vfr",
                "-q:v",
                "3",
                "-pix_fmt",
                "yuvj420p",
                "-strict",
                "unofficial",
                str(frame_pattern),
            ],
            timeout=max(600, int(duration * 2)),
        )
        frames = sorted(temporary_root.glob("frame-*.jpg"))
        if not frames:
            raise RuntimeError(f"No contact-sheet frames were extracted from {video}")

        font = load_font(22)
        labelled: list[Image.Image] = []
        for index, frame_path in enumerate(frames):
            image = Image.open(frame_path).convert("RGB")
            label_height = 36
            canvas = Image.new("RGB", (image.width, image.height + label_height), "#111827")
            canvas.paste(image, (0, label_height))
            timestamp = min(start + index * frame_interval, start + duration)
            draw = ImageDraw.Draw(canvas)
            draw.text((10, 6), display_timestamp(timestamp), fill="white", font=font)
            labelled.append(canvas)

        columns = 4
        rows = math.ceil(len(labelled) / columns)
        cell_width = max(image.width for image in labelled)
        cell_height = max(image.height for image in labelled)
        sheet = Image.new("RGB", (columns * cell_width, rows * cell_height), "#0f172a")
        for index, image in enumerate(labelled):
            x = (index % columns) * cell_width
            y = (index // columns) * cell_height
            sheet.paste(image, (x, y))
        sheet.save(output, format="JPEG", quality=88, optimize=True)
        write_json(sidecar, binding)
    finally:
        shutil.rmtree(temporary_root, ignore_errors=True)


def vision_schema() -> dict[str, Any]:
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["text_sufficient", "decision_basis", "segments", "limitations"],
        "properties": {
            "text_sufficient": {"type": "boolean"},
            "decision_basis": {"type": "string"},
            "segments": {
                "type": "array",
                "maxItems": 6,
                "items": {
                    "type": "object",
                    "additionalProperties": False,
                    "required": [
                        "type",
                        "start_seconds",
                        "end_seconds",
                        "frame_timestamp_seconds",
                        "visual_role",
                        "summary",
                        "why_text_insufficient",
                        "importance",
                    ],
                    "properties": {
                        "type": {"type": "string", "enum": ["key_frame", "video_excerpt"]},
                        "start_seconds": {"type": "number", "minimum": 0},
                        "end_seconds": {"type": "number", "minimum": 0},
                        "frame_timestamp_seconds": {"type": "number", "minimum": 0},
                        "visual_role": {
                            "type": "string",
                            "enum": ["code", "ui_operation", "rendering_output", "diagram", "animation", "other"],
                        },
                        "summary": {"type": "string"},
                        "why_text_insufficient": {"type": "string"},
                        "importance": {"type": "integer", "minimum": 1, "maximum": 5},
                    },
                },
            },
            "limitations": {"type": "array", "items": {"type": "string"}},
        },
    }


def response_json(response: dict[str, Any]) -> dict[str, Any]:
    value = response.get("json")
    if isinstance(value, list):
        if len(value) != 1 or not isinstance(value[0], dict):
            raise RuntimeError("Qwen vision returned an ambiguous structured response")
        return value[0]
    if not isinstance(value, dict):
        raise RuntimeError("Qwen vision returned no structured JSON object")
    return value


def token_count(usage: dict[str, Any], names: tuple[str, ...]) -> int:
    for name in names:
        value = usage.get(name)
        if isinstance(value, (int, float)):
            return int(value)
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--source-manifest",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-stage2-20260821-v1\results\source-manifest.json",
    )
    parser.add_argument(
        "--c1-manifest",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-full-20260822-v1\manifest.json",
    )
    parser.add_argument(
        "--stage",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c1b-vision-full-20260822-v1",
    )
    parser.add_argument(
        "--vision-script",
        default=r"C:\Users\Aiano\.agents\skills\qianwen-vision\scripts\analyze.py",
    )
    parser.add_argument("--model", default="qwen3.6-plus")
    parser.add_argument("--expected-items", type=int, default=269)
    parser.add_argument("--chunk-seconds", type=float, default=900.0)
    parser.add_argument("--frame-interval-seconds", type=float, default=45.0)
    parser.add_argument("--max-frame-width", type=int, default=720)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--ffmpeg", default="ffmpeg")
    parser.add_argument("--input-price", type=float, default=2.0)
    parser.add_argument("--output-price", type=float, default=12.0)
    parser.add_argument(
        "--pricing-source",
        default="https://platform.qianwenai.com/docs/developer-guides/getting-started/pricing",
    )
    args = parser.parse_args()

    if args.expected_items < 1 or args.chunk_seconds <= 0 or args.frame_interval_seconds <= 0:
        raise RuntimeError("Expected items and time parameters must be positive")
    if not 1 <= args.workers <= 12:
        raise RuntimeError("Workers must be between 1 and 12")
    if shutil.which(args.ffmpeg) is None:
        raise RuntimeError("ffmpeg is unavailable")
    if not os.environ.get("DASHSCOPE_API_KEY"):
        raise RuntimeError("DASHSCOPE_API_KEY is unavailable")

    source_manifest_path = Path(args.source_manifest).resolve()
    c1_manifest_path = Path(args.c1_manifest).resolve()
    stage = Path(args.stage).resolve()
    vision_script = Path(args.vision_script).resolve()
    for required in (source_manifest_path, c1_manifest_path, vision_script):
        if not required.is_file():
            raise RuntimeError(f"Missing required input: {required}")

    source = read_json(source_manifest_path)
    c1 = read_json(c1_manifest_path)
    c1_items = list(c1.get("items", []))
    if len(c1_items) != args.expected_items or any(item.get("status") != "registered" for item in c1_items):
        raise RuntimeError(f"C1B round requires exactly {args.expected_items} registered C1 items")
    source_by_video = {str(item["video_id"]): item for item in source.get("items", [])}
    if len(source_by_video) != args.expected_items:
        raise RuntimeError("Source manifest denominator does not match the C1 round")

    requests_dir = stage / "requests"
    provider_dir = stage / "provider-temp"
    sheets_dir = stage / "preprocessed" / "contact-sheets"
    chunk_results_dir = stage / "results" / "chunks"
    results_dir = stage / "results"
    media_dir = results_dir / "media"
    for directory in (requests_dir, provider_dir, sheets_dir, chunk_results_dir, media_dir):
        directory.mkdir(parents=True, exist_ok=True)

    script_hash = sha256_file(Path(__file__).resolve())
    vision_hash = sha256_file(vision_script)
    schema = vision_schema()
    tasks: list[dict[str, Any]] = []
    transcript_cache: dict[str, list[dict[str, Any]]] = {}
    c1_by_video = {str(item["video_id"]): item for item in c1_items}
    source_manifest_sha256 = sha256_file(source_manifest_path)
    c1_manifest_sha256 = sha256_file(c1_manifest_path)
    source_hash_cache_path = stage / "source-hash-verification.json"
    source_hash_cache: dict[str, Any] = {}
    if source_hash_cache_path.is_file():
        candidate = read_json(source_hash_cache_path)
        if (
            candidate.get("schema") == "babata.cherno-source-hash-verification/v1"
            and candidate.get("source_manifest_sha256") == source_manifest_sha256
            and candidate.get("c1_manifest_sha256") == c1_manifest_sha256
        ):
            source_hash_cache = dict(candidate.get("items") or {})

    def persist_source_hash_cache() -> None:
        write_json(
            source_hash_cache_path,
            {
                "schema": "babata.cherno-source-hash-verification/v1",
                "source_manifest_sha256": source_manifest_sha256,
                "c1_manifest_sha256": c1_manifest_sha256,
                "items": source_hash_cache,
                "updated_at": utc_now(),
            },
        )

    for c1_item in c1_items:
        video_id = str(c1_item["video_id"])
        source_item = source_by_video.get(video_id)
        if source_item is None:
            raise RuntimeError(f"Missing frozen source row for {video_id}")
        video_path = Path(source_item["local_media"]["local_path"])
        if not video_path.is_file():
            raise RuntimeError(f"Source video is missing: {video_path}")
        if str(source_item["local_media"]["sha256"]).lower() != str(c1_item["c0"]["input_sha256"]).lower():
            raise RuntimeError(f"Source/C0 manifest hash mismatch for {video_id}")
        stat = video_path.stat()
        expected_sha256 = str(c1_item["c0"]["input_sha256"]).lower()
        cached = source_hash_cache.get(video_id) or {}
        cache_matches = (
            cached.get("path") == str(video_path.resolve())
            and int(cached.get("size_bytes", -1)) == stat.st_size
            and int(cached.get("mtime_ns", -1)) == stat.st_mtime_ns
            and cached.get("sha256") == expected_sha256
        )
        if not cache_matches:
            if sha256_file(video_path) != expected_sha256:
                raise RuntimeError(f"Read-only source video hash drift for {video_id}")
            source_hash_cache[video_id] = {
                "path": str(video_path.resolve()),
                "size_bytes": stat.st_size,
                "mtime_ns": stat.st_mtime_ns,
                "sha256": expected_sha256,
            }
            persist_source_hash_cache()

        transcript_derivatives = [row for row in c1_item["derivatives"] if row["kind"] == "transcript"]
        transcript_registrations = [row for row in c1_item["registrations"] if row["kind"] == "transcript"]
        if len(transcript_derivatives) != 1 or len(transcript_registrations) != 1:
            raise RuntimeError(f"Complete transcript identity is ambiguous for {video_id}")
        transcript_path = c1_manifest_path.parent / transcript_derivatives[0]["path"]
        if sha256_file(transcript_path) != transcript_derivatives[0]["sha256"]:
            raise RuntimeError(f"Transcript hash drift for {video_id}")
        transcript_rows = parse_transcript(transcript_path)
        transcript_cache[video_id] = transcript_rows

        duration = float(source_item["local_media"]["duration_seconds_local"])
        chunks = max(1, math.ceil(duration / args.chunk_seconds))
        for chunk_index in range(chunks):
            start = chunk_index * args.chunk_seconds
            chunk_duration = min(args.chunk_seconds, max(0.001, duration - start))
            end = start + chunk_duration
            task_id = f"{video_id}-{chunk_index + 1:03d}"
            sheet_path = sheets_dir / video_id / f"{task_id}.jpg"
            request_path = requests_dir / video_id / f"{task_id}.request.json"
            response_path = provider_dir / video_id / f"{task_id}.response.json"
            response_hash_path = response_path.with_suffix(".request.sha256")
            result_path = chunk_results_dir / video_id / f"{task_id}.decision.json"
            binding = {
                "schema": "babata.cherno-vision-contact-sheet-binding/v1",
                "source_sha256": c1_item["c0"]["input_sha256"],
                "start_seconds": round(start, 3),
                "duration_seconds": round(chunk_duration, 3),
                "frame_interval_seconds": args.frame_interval_seconds,
                "max_frame_width": args.max_frame_width,
                "frame_sampler": "first-frame-then-minimum-interval",
                "frame_encoder": "mjpeg/yuvj420p/strict-unofficial",
            }
            tasks.append(
                {
                    "task_id": task_id,
                    "video_id": video_id,
                    "chunk_index": chunk_index,
                    "chunk_count": chunks,
                    "start": start,
                    "end": end,
                    "duration": chunk_duration,
                    "video_path": video_path,
                    "sheet_path": sheet_path,
                    "request_path": request_path,
                    "response_path": response_path,
                    "response_hash_path": response_hash_path,
                    "result_path": result_path,
                    "binding": binding,
                    "source_item": source_item,
                    "c1_item": c1_item,
                    "transcript_sha256": transcript_derivatives[0]["sha256"],
                }
            )

    progress_lock = threading.Lock()
    progress = {
        "schema": "babata.cherno-c1b-vision-chunk-progress/v1",
        "status": "in_progress",
        "expected_items": args.expected_items,
        "expected_chunks": len(tasks),
        "completed_chunks": 0,
        "failed_chunks": 0,
        "updated_at": utc_now(),
    }

    def persist_progress() -> None:
        progress["updated_at"] = utc_now()
        write_json(stage / "progress.json", progress)

    persist_progress()

    def process_task(task: dict[str, Any]) -> dict[str, Any]:
        build_contact_sheet(
            ffmpeg=args.ffmpeg,
            video=task["video_path"],
            output=task["sheet_path"],
            start=task["start"],
            duration=task["duration"],
            frame_interval=args.frame_interval_seconds,
            max_frame_width=args.max_frame_width,
            binding=task["binding"],
        )
        video_id = task["video_id"]
        source_item = task["source_item"]
        contact_sheet_sha256 = sha256_file(task["sheet_path"])
        prompt = f"""You are performing a formal C1B visual-essence review for one chronological chunk of a programming course lesson.
The contact sheet covers the entire chunk at a fixed interval; every tile is labelled with an absolute timestamp in the original video.
Use the matching timestamped ASR span as context. Retain only visual evidence that materially changes understanding and is not reliably preserved by ASR: source-code state, IDE/editor operation, build/debug UI, diagrams, rendered graphics, or time-continuous animation.
Ignore talking heads, branding, decorative footage, repeated unchanged screens, and information already explicit in text.
Use key_frame for a stable state. Use video_excerpt only when motion or an interaction sequence is essential; keep it 3-20 seconds.
All returned timestamps MUST be absolute seconds in the complete source video and MUST stay within [{task['start']:.3f}, {task['end']:.3f}].
Return at most six candidates and rate importance from 1 to 5.

Course: {source_item['course_title']}
Lesson: {source_item['original_title']}
Video ID: {video_id}
Chunk: {task['chunk_index'] + 1}/{task['chunk_count']}
Complete video duration: {source_item['local_media']['duration_seconds_local']} seconds
Chunk range: {display_timestamp(task['start'])} - {display_timestamp(task['end'])}

Timestamped ASR span:
{transcript_slice(transcript_cache[video_id], task['start'], task['end'])}
"""
        request = {
            "prompt": prompt,
            "image": str(task["sheet_path"]),
            "image_sha256": contact_sheet_sha256,
            "model": args.model,
            "json_mode": True,
            "schema": schema,
            "enable_thinking": False,
            "max_tokens": 2400,
            "temperature": 0,
            "timeout_s": 1800,
            "max_retries": 2,
        }
        write_json(task["request_path"], request)
        request_hash = sha256_file(task["request_path"])
        reusable = (
            task["response_path"].is_file()
            and task["response_hash_path"].is_file()
            and task["response_hash_path"].read_text(encoding="ascii").strip() == request_hash
        )
        signals = ""
        if not reusable:
            task["response_path"].parent.mkdir(parents=True, exist_ok=True)
            completed = run(
                [
                    sys.executable,
                    str(vision_script),
                    "--file",
                    str(task["request_path"]),
                    "--upload-files",
                    "--output",
                    str(task["response_path"]),
                ],
                timeout=2100,
            )
            signals = completed.stdout + "\n" + completed.stderr
            task["response_hash_path"].write_text(request_hash, encoding="ascii")
        response = read_json(task["response_path"])
        decision = response_json(response)
        segments = decision.get("segments")
        if not isinstance(segments, list):
            raise RuntimeError(f"Qwen decision has no segments array for {task['task_id']}")
        normalizations: list[dict[str, Any]] = []
        if bool(decision.get("text_sufficient")) and segments:
            decision["text_sufficient"] = False
            note = "Provider returned text_sufficient=true together with retained segments; normalized to false because retained visual evidence takes precedence."
            limitations = decision.get("limitations")
            if not isinstance(limitations, list):
                limitations = []
                decision["limitations"] = limitations
            limitations.append(note)
            normalizations.append(
                {
                    "field": "text_sufficient",
                    "provider_value": True,
                    "normalized_value": False,
                    "reason": note,
                }
            )
        result = {
            "schema": "babata.cherno-c1b-vision-chunk-decision/v1",
            "task_id": task["task_id"],
            "video_id": video_id,
            "chunk_index": task["chunk_index"],
            "chunk_count": task["chunk_count"],
            "start_seconds": task["start"],
            "end_seconds": task["end"],
            "contact_sheet": str(task["sheet_path"]),
            "contact_sheet_sha256": contact_sheet_sha256,
            "request_sha256": request_hash,
            "model": response.get("model", args.model),
            "usage": response.get("usage") or {},
            "decision": decision,
            "normalizations": normalizations,
            "update_signal_observed": "[UPDATE_AVAILABLE]" in signals,
        }
        write_json(task["result_path"], result)
        with progress_lock:
            progress["completed_chunks"] += 1
            persist_progress()
        return result

    chunk_results: list[dict[str, Any]] = []
    failures: list[str] = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        future_map = {executor.submit(process_task, task): task for task in tasks}
        for future in concurrent.futures.as_completed(future_map):
            task = future_map[future]
            try:
                chunk_results.append(future.result())
            except Exception as error:  # noqa: BLE001 - preserve every failed chunk
                failures.append(f"{task['task_id']}: {error}")
                with progress_lock:
                    progress["failed_chunks"] += 1
                    progress["last_error"] = failures[-1]
                    persist_progress()
    if failures:
        raise RuntimeError("C1B chunk round failed:\n" + "\n".join(failures[:20]))

    chunks_by_video: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for result in chunk_results:
        chunks_by_video[result["video_id"]].append(result)
    manifest_items: list[dict[str, Any]] = []
    total_prompt_tokens = 0
    total_completion_tokens = 0
    update_signal_observed = False

    for video_id, c1_item in c1_by_video.items():
        source_item = source_by_video[video_id]
        duration = float(source_item["local_media"]["duration_seconds_local"])
        video_path = Path(source_item["local_media"]["local_path"])
        results = sorted(chunks_by_video[video_id], key=lambda row: row["chunk_index"])
        if not results:
            raise RuntimeError(f"No chunk decisions were produced for {video_id}")
        candidates: list[dict[str, Any]] = []
        rejected: list[dict[str, Any]] = []
        limitations: list[str] = []
        bases: list[str] = []
        request_hashes: list[str] = []
        for chunk in results:
            usage = chunk.get("usage") or {}
            total_prompt_tokens += token_count(usage, ("prompt_tokens", "input_tokens"))
            total_completion_tokens += token_count(usage, ("completion_tokens", "output_tokens"))
            update_signal_observed = update_signal_observed or bool(chunk.get("update_signal_observed"))
            request_hashes.append(chunk["request_sha256"])
            decision = chunk["decision"]
            bases.append(str(decision.get("decision_basis", "")).strip())
            limitations.extend(str(value) for value in decision.get("limitations", []) if str(value).strip())
            for segment in decision.get("segments", []):
                candidate = dict(segment)
                candidate["chunk_index"] = chunk["chunk_index"]
                candidate["contact_sheet_sha256"] = chunk["contact_sheet_sha256"]
                start = float(candidate.get("start_seconds", -1))
                end = float(candidate.get("end_seconds", -1))
                frame = float(candidate.get("frame_timestamp_seconds", -1))
                if (
                    start < float(chunk["start_seconds"]) - 1
                    or end > float(chunk["end_seconds"]) + 1
                    or start < 0
                    or end <= start
                    or frame < start
                    or frame > end
                    or end > duration + 1
                ):
                    rejected.append({"segment": candidate, "reason": "invalid_or_out_of_chunk_locator"})
                    continue
                if candidate.get("type") == "video_excerpt" and not 3 <= end - start <= 20:
                    rejected.append({"segment": candidate, "reason": "video_excerpt_duration_outside_3_20_seconds"})
                    continue
                candidates.append(candidate)

        candidates.sort(
            key=lambda row: (-int(row.get("importance", 1)), float(row["frame_timestamp_seconds"]), str(row.get("type")))
        )
        selected: list[dict[str, Any]] = []
        excerpt_seconds = 0.0
        for candidate in candidates:
            if len(selected) >= 8:
                rejected.append({"segment": candidate, "reason": "lesson_level_max_eight"})
                continue
            frame = float(candidate["frame_timestamp_seconds"])
            if any(abs(frame - float(existing["frame_timestamp_seconds"])) < 12 for existing in selected):
                rejected.append({"segment": candidate, "reason": "near_duplicate_locator"})
                continue
            if candidate["type"] == "video_excerpt":
                proposed = float(candidate["end_seconds"]) - float(candidate["start_seconds"])
                if excerpt_seconds + proposed > 90:
                    rejected.append({"segment": candidate, "reason": "lesson_level_video_excerpt_budget"})
                    continue
                excerpt_seconds += proposed
            selected.append(candidate)
        selected.sort(key=lambda row: float(row["frame_timestamp_seconds"]))

        retained: list[dict[str, Any]] = []
        for index, segment in enumerate(selected, start=1):
            role = str(segment["visual_role"])
            if segment["type"] == "key_frame":
                output = media_dir / f"{video_id}-{index:02d}-{role}.jpg"
                run(
                    [
                        args.ffmpeg,
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-ss",
                        f"{float(segment['frame_timestamp_seconds']):.3f}",
                        "-i",
                        str(video_path),
                        "-frames:v",
                        "1",
                        "-q:v",
                        "2",
                        "-y",
                        str(output),
                    ],
                    timeout=600,
                )
                kind = "key_frame"
            else:
                output = media_dir / f"{video_id}-{index:02d}-{role}.mp4"
                clip_duration = float(segment["end_seconds"]) - float(segment["start_seconds"])
                run(
                    [
                        args.ffmpeg,
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-ss",
                        f"{float(segment['start_seconds']):.3f}",
                        "-i",
                        str(video_path),
                        "-t",
                        f"{clip_duration:.3f}",
                        "-map",
                        "0:v:0",
                        "-map",
                        "0:a?",
                        "-c:v",
                        "libx264",
                        "-preset",
                        "medium",
                        "-crf",
                        "20",
                        "-c:a",
                        "aac",
                        "-b:a",
                        "128k",
                        "-movflags",
                        "+faststart",
                        "-y",
                        str(output),
                    ],
                    timeout=1200,
                )
                kind = "video_excerpt"
            retained.append(
                {
                    "kind": kind,
                    "path": output.relative_to(stage).as_posix(),
                    "sha256": sha256_file(output),
                    "source_locator": {
                        "start_seconds": round(float(segment["start_seconds"]), 3),
                        "end_seconds": round(float(segment["end_seconds"]), 3),
                        "frame_timestamp_seconds": round(float(segment["frame_timestamp_seconds"]), 3),
                    },
                    "original_model_source_locator": {
                        "chunk_index": int(segment["chunk_index"]),
                        "contact_sheet_sha256": segment["contact_sheet_sha256"],
                    },
                    "role": role,
                    "summary": str(segment["summary"]),
                    "why_text_insufficient": str(
                        segment.get(
                            "why_text_insufficient",
                            "Provider omitted the reason; retained visual evidence because text/ASR was insufficient for this segment.",
                        )
                    ),
                    "loss_notes": [
                        "Selected from complete chronological chunk contact sheets; sub-interval motion can be missed.",
                        "Retained bytes were cut from the authoritative read-only original MP4.",
                    ],
                }
            )

        aggregate_hash = hashlib.sha256("\n".join(sorted(request_hashes)).encode("ascii")).hexdigest()
        decision = {
            "schema": "babata.cherno-c1b-essence-decision/v2",
            "video_id": video_id,
            "text_sufficient": len(retained) == 0,
            "decision_basis": " ".join(value for value in bases if value)[:12_000],
            "normalized_retained_derivatives": retained,
            "rejected_segments": rejected,
            "limitations": sorted(set(limitations + [
                "The full timeline was sampled at a fixed interval into chunk contact sheets; very brief visual changes can be missed."
            ])),
            "chunk_decisions": [
                {
                    "task_id": row["task_id"],
                    "start_seconds": row["start_seconds"],
                    "end_seconds": row["end_seconds"],
                    "request_sha256": row["request_sha256"],
                    "contact_sheet_sha256": row["contact_sheet_sha256"],
                }
                for row in results
            ],
        }
        decision_path = results_dir / f"{video_id}.essence.json"
        write_json(decision_path, decision)
        transcript_derivative = find_single(c1_item["derivatives"], "kind", "transcript", "transcript derivative")
        transcript_registration = find_single(c1_item["registrations"], "kind", "transcript", "transcript registration")
        manifest_items.append(
            {
                "video_id": video_id,
                "course_slug": source_item["course_slug"],
                "original_title": source_item["original_title"],
                "c0": c1_item["c0"],
                "complete_c1": transcript_registration,
                "processing": {
                    "provider": "qianwen_skill",
                    "service": "dashscope",
                    "adapter": "qianwen-vision",
                    "model": args.model,
                    "tool_version": f"run-cherno-c1b-vision-round.py@sha256:{script_hash};analyze.py@sha256:{vision_hash}",
                    "credential_source": "environment",
                    "provider_input_sha256": aggregate_hash,
                    "video_input_sha256": c1_item["c0"]["input_sha256"],
                    "transcript_sha256": transcript_derivative["sha256"],
                    "fps": round(1 / args.frame_interval_seconds, 8),
                    "usage": {
                        "chunk_count": len(results),
                        "prompt_tokens": sum(token_count(row.get("usage") or {}, ("prompt_tokens", "input_tokens")) for row in results),
                        "completion_tokens": sum(token_count(row.get("usage") or {}, ("completion_tokens", "output_tokens")) for row in results),
                    },
                    "estimated_cost_cny": round(
                        sum(token_count(row.get("usage") or {}, ("prompt_tokens", "input_tokens")) for row in results)
                        / 1_000_000
                        * args.input_price
                        + sum(token_count(row.get("usage") or {}, ("completion_tokens", "output_tokens")) for row in results)
                        / 1_000_000
                        * args.output_price,
                        6,
                    ),
                },
                "essence_decision": {
                    "path": decision_path.relative_to(stage).as_posix(),
                    "sha256": sha256_file(decision_path),
                    "text_sufficient": len(retained) == 0,
                    "decision_basis": decision["decision_basis"],
                    "limitations": decision["limitations"],
                },
                "retained_derivatives": retained,
                "registrations": [],
                "status": "staged_only",
            }
        )

    created_at = utc_now()
    estimated_cost = round(
        total_prompt_tokens / 1_000_000 * args.input_price
        + total_completion_tokens / 1_000_000 * args.output_price,
        6,
    )
    manifest = {
        "schema": "babata.cherno-c1b-vision-chunked/v1",
        "created_at": created_at,
        "status": "staged_only",
        "scope": "full_269_chunked_timeline",
        "source_manifest": str(source_manifest_path),
        "source_manifest_sha256": source_manifest_sha256,
        "c1_manifest": str(c1_manifest_path),
        "c1_manifest_sha256": c1_manifest_sha256,
        "route": {
            "chunk_seconds": args.chunk_seconds,
            "frame_interval_seconds": args.frame_interval_seconds,
            "max_frame_width": args.max_frame_width,
            "expected_chunks": len(tasks),
            "provider_receives_original_mp4": False,
            "retained_media_cut_from_original_mp4": True,
        },
        "pricing": {
            "schema": "babata.provider-pricing/v1",
            "retrieved_at": created_at,
            "source_url": args.pricing_source,
            "model": args.model,
            "currency": "CNY",
            "input_price_per_million_tokens": args.input_price,
            "output_price_per_million_tokens": args.output_price,
            "prompt_tokens": total_prompt_tokens,
            "completion_tokens": total_completion_tokens,
            "estimated_cost_cny": estimated_cost,
            "free_tier_applied": "unknown_until_billing_evidence",
        },
        "update_available_signal_observed": update_signal_observed,
        "items": manifest_items,
    }
    manifest_path = stage / "manifest.json"
    write_json(manifest_path, manifest)
    report = "\n".join(
        [
            "# Cherno C1B full visual round",
            "",
            f"- Items: {len(manifest_items)}/{args.expected_items} staged.",
            f"- Timeline chunks: {len(tasks)}/{len(tasks)} complete.",
            f"- Retained visual derivatives: {sum(len(row['retained_derivatives']) for row in manifest_items)}.",
            f"- Qwen prompt/completion tokens: {total_prompt_tokens} / {total_completion_tokens}.",
            f"- Estimated model cost before free-tier reconciliation: {estimated_cost} CNY.",
            "- Provider input: annotated chronological contact sheets plus matching ASR spans; original MP4 was not uploaded.",
            "- Retained key frames/video excerpts were cut from hash-bound read-only C0 originals.",
            "- Status: staging only; formal C1B registration is still required.",
            "",
        ]
    )
    (stage / "REPORT.md").write_text(report, encoding="utf-8")
    progress.update({"status": "complete", "completed_items": len(manifest_items), "updated_at": utc_now()})
    persist_progress()
    print(
        json.dumps(
            {
                "items": len(manifest_items),
                "chunks": len(tasks),
                "retained_derivatives": sum(len(row["retained_derivatives"]) for row in manifest_items),
                "prompt_tokens": total_prompt_tokens,
                "completion_tokens": total_completion_tokens,
                "estimated_cost_cny": estimated_cost,
                "manifest": str(manifest_path),
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: {exc}", file=sys.stderr)
        raise
