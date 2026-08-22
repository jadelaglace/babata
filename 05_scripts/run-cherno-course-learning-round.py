#!/usr/bin/env python3
"""Generate resumable Qwen learning notes and course presentation plans for Cherno."""

from __future__ import annotations

import argparse
import concurrent.futures
import hashlib
import json
import os
import re
import subprocess
import sys
import threading
from collections import defaultdict
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


COURSES = {
    "cpp": {"course_key": "cherno-cpp", "short_name": "C++"},
    "game-engine": {"course_key": "cherno-game-engine", "short_name": "Game Engine"},
    "opengl": {"course_key": "cherno-opengl", "short_name": "OpenGL"},
}
OBSIDIAN_NOTE_FORBIDDEN_RE = re.compile(r'[<>:"/\\|?*\[\]#^\x00-\x1f]')


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(8 * 1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def read_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8-sig"))


def write_json(path: Path, value: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(value, ensure_ascii=False, indent=2), encoding="utf-8")
    os.replace(temporary, path)


def lesson_note_name(position: int, display_title: str) -> str:
    readable = OBSIDIAN_NOTE_FORBIDDEN_RE.sub(" ", str(display_title))
    readable = re.sub(r"\s+", " ", readable).strip(" .")
    if not readable:
        raise RuntimeError(f"Lesson {position} has no readable display title")
    return f"L{position:03d}-{readable[:96].rstrip(' .')}"


def run(command: list[str], *, timeout: int = 2100) -> subprocess.CompletedProcess[str]:
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


def response_content(response: dict[str, Any]) -> str:
    choices = response.get("choices") or []
    if not choices:
        raise RuntimeError("Qwen text response contains no choices")
    content = (choices[0].get("message") or {}).get("content")
    if not isinstance(content, str) or not content.strip():
        raise RuntimeError("Qwen text response contains no content")
    return content


def token_count(usage: dict[str, Any], names: tuple[str, ...]) -> int:
    for name in names:
        value = usage.get(name)
        if isinstance(value, (int, float)):
            return int(value)
    return 0


def lesson_schema() -> dict[str, Any]:
    return {
        "type": "object",
        "additionalProperties": False,
        "required": [
            "display_title",
            "summary",
            "key_concepts",
            "implementation_decisions",
            "pitfalls",
            "practice",
            "tags",
        ],
        "properties": {
            "display_title": {"type": "string"},
            "summary": {"type": "string"},
            "key_concepts": {
                "type": "array",
                "minItems": 2,
                "maxItems": 10,
                "items": {
                    "type": "object",
                    "additionalProperties": False,
                    "required": ["name", "explanation"],
                    "properties": {"name": {"type": "string"}, "explanation": {"type": "string"}},
                },
            },
            "implementation_decisions": {"type": "array", "maxItems": 10, "items": {"type": "string"}},
            "pitfalls": {"type": "array", "maxItems": 10, "items": {"type": "string"}},
            "practice": {"type": "array", "maxItems": 8, "items": {"type": "string"}},
            "tags": {"type": "array", "minItems": 2, "maxItems": 12, "items": {"type": "string"}},
        },
    }


def course_schema(video_ids: list[str]) -> dict[str, Any]:
    return {
        "type": "object",
        "additionalProperties": False,
        "required": ["course_summary", "classification_axis", "domains", "learning_support"],
        "properties": {
            "course_summary": {"type": "string"},
            "classification_axis": {"type": "string"},
            "domains": {
                "type": "array",
                "minItems": 4,
                "maxItems": 10,
                "items": {
                    "type": "object",
                    "additionalProperties": False,
                    "required": ["id", "title", "description", "rules", "video_ids"],
                    "properties": {
                        "id": {"type": "string", "pattern": "^[a-z0-9-]+$"},
                        "title": {"type": "string"},
                        "description": {"type": "string"},
                        "rules": {"type": "array", "minItems": 2, "maxItems": 6, "items": {"type": "string"}},
                        "video_ids": {
                            "type": "array",
                            "minItems": 1,
                            "items": {"type": "string", "enum": video_ids},
                        },
                    },
                },
            },
            "learning_support": {
                "type": "object",
                "additionalProperties": False,
                "required": ["overview", "project_path", "review_questions"],
                "properties": {
                    "overview": {"type": "array", "minItems": 3, "maxItems": 12, "items": {"type": "string"}},
                    "project_path": {"type": "array", "minItems": 3, "maxItems": 12, "items": {"type": "string"}},
                    "review_questions": {"type": "array", "minItems": 6, "maxItems": 30, "items": {"type": "string"}},
                },
            },
        },
    }


def json_schema_response(name: str, schema: dict[str, Any]) -> dict[str, Any]:
    return {"type": "json_schema", "json_schema": {"name": name, "strict": True, "schema": schema}}


def validate_lesson(value: dict[str, Any], video_id: str) -> None:
    for field in ("display_title", "summary"):
        if not str(value.get(field, "")).strip():
            raise RuntimeError(f"Lesson result lacks {field}: {video_id}")
    for field in ("key_concepts", "implementation_decisions", "pitfalls", "practice", "tags"):
        if not isinstance(value.get(field), list):
            raise RuntimeError(f"Lesson result lacks {field}: {video_id}")


def markdown_note(item: dict[str, Any], learning: dict[str, Any]) -> str:
    lines = [
        f"# {learning['display_title']}",
        "",
        learning["summary"].strip(),
        "",
        "## 核心概念",
        "",
    ]
    for concept in learning["key_concepts"]:
        lines.append(f"- **{concept['name']}**：{concept['explanation']}")
    if learning["implementation_decisions"]:
        lines.extend(["", "## 实现决策", ""] + [f"- {value}" for value in learning["implementation_decisions"]])
    if learning["pitfalls"]:
        lines.extend(["", "## 常见陷阱", ""] + [f"- {value}" for value in learning["pitfalls"]])
    if learning["practice"]:
        lines.extend(["", "## 练习", ""] + [f"- [ ] {value}" for value in learning["practice"]])
    lines.extend(
        [
            "",
            "## 来源身份",
            "",
            f"- Video ID: `{item['video_id']}`",
            f"- Original title: {item['original_title']}",
            f"- Source: {item['source_url']}",
            f"- Playlist position observed: {item['playlist_position_observed']}",
            "",
        ]
    )
    return "\n".join(lines)


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
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-learning-full-20260822-v1",
    )
    parser.add_argument(
        "--text-script",
        default=r"C:\Users\Aiano\.agents\skills\qianwen-text\scripts\text.py",
    )
    parser.add_argument("--model", default="qwen3.6-plus")
    parser.add_argument("--expected-items", type=int, default=269)
    parser.add_argument("--workers", type=int, default=4)
    parser.add_argument("--input-price", type=float, default=2.0)
    parser.add_argument("--output-price", type=float, default=12.0)
    parser.add_argument(
        "--pricing-source",
        default="https://platform.qianwenai.com/docs/developer-guides/getting-started/pricing",
    )
    parser.add_argument(
        "--vault-root",
        default=r"C:\Users\Aiano\Documents\Obsidian Vault",
    )
    args = parser.parse_args()

    if not 1 <= args.workers <= 12:
        raise RuntimeError("Workers must be between 1 and 12")
    if not os.environ.get("DASHSCOPE_API_KEY"):
        raise RuntimeError("DASHSCOPE_API_KEY is unavailable")
    source_path = Path(args.source_manifest).resolve()
    c1_path = Path(args.c1_manifest).resolve()
    stage = Path(args.stage).resolve()
    text_script = Path(args.text_script).resolve()
    for required in (source_path, c1_path, text_script):
        if not required.is_file():
            raise RuntimeError(f"Missing required input: {required}")

    source = read_json(source_path)
    c1 = read_json(c1_path)
    items = list(c1.get("items", []))
    if len(items) != args.expected_items or any(item.get("status") != "registered" for item in items):
        raise RuntimeError(f"Learning round requires exactly {args.expected_items} registered C1 items")
    source_by_video = {str(item["video_id"]): item for item in source.get("items", [])}
    if len(source_by_video) != args.expected_items:
        raise RuntimeError("Source manifest denominator mismatch")

    request_root = stage / "requests" / "lessons"
    response_root = stage / "provider-temp" / "lessons"
    result_root = stage / "results" / "learning"
    note_root = stage / "results" / "notes"
    plan_root = stage / "presentation-plans"
    for directory in (request_root, response_root, result_root, note_root, plan_root):
        directory.mkdir(parents=True, exist_ok=True)

    script_hash = sha256_file(Path(__file__).resolve())
    text_hash = sha256_file(text_script)
    progress_lock = threading.Lock()
    progress = {
        "schema": "babata.cherno-course-learning-progress/v1",
        "status": "in_progress",
        "expected_items": args.expected_items,
        "completed_items": 0,
        "failed_items": 0,
        "updated_at": utc_now(),
    }

    def persist_progress() -> None:
        progress["updated_at"] = utc_now()
        write_json(stage / "progress.json", progress)

    persist_progress()

    def process_item(c1_item: dict[str, Any]) -> dict[str, Any]:
        video_id = str(c1_item["video_id"])
        source_item = source_by_video[video_id]
        derivatives = [row for row in c1_item["derivatives"] if row["kind"] == "transcript"]
        registrations = [row for row in c1_item["registrations"] if row["kind"] == "transcript"]
        if len(derivatives) != 1 or len(registrations) != 1:
            raise RuntimeError(f"Transcript identity is ambiguous for {video_id}")
        transcript_path = c1_path.parent / derivatives[0]["path"]
        if sha256_file(transcript_path) != derivatives[0]["sha256"]:
            raise RuntimeError(f"Transcript hash drift for {video_id}")
        transcript = transcript_path.read_text(encoding="utf-8-sig")
        contract = json.dumps(lesson_schema(), ensure_ascii=False)
        prompt = f"""请把下面这节英文编程课程整理成面向中文学习者的高密度学习笔记。
要求：保留所有 C++、OpenGL、API、类名、函数名和代码标识符的原始拼写；不臆造代码；明确区分课程中实际讲到的内容与建议练习；摘要先讲本节解决什么问题，再讲方法与边界；避免提及模型、提示词、存储、发布或工作流。
只输出一个 JSON 对象，顶层字段必须且只能是 display_title、summary、key_concepts、implementation_decisions、pitfalls、practice、tags，并严格满足下面的 JSON Schema：
{contract}

Course: {source_item['course_title']}
Original title: {source_item['original_title']}
Video ID: {video_id}
Observed playlist position: {source_item['playlist_position_observed']}

Complete timestamped transcript:
{transcript}
"""
        request = {
            "model": args.model,
            "messages": [
                {"role": "system", "content": "你是严谨的编程课程编辑。只输出一个符合指定字段合同的简体中文 JSON 对象，不要增加任何未声明字段。"},
                {"role": "user", "content": prompt},
            ],
            "response_format": {"type": "json_object"},
            "enable_thinking": False,
            "temperature": 0,
            "max_tokens": 3500,
        }
        request_path = request_root / f"{video_id}.request.json"
        output_dir = response_root / video_id
        response_path = output_dir / "response.json"
        sidecar = output_dir / "request.sha256"
        write_json(request_path, request)
        request_hash = sha256_file(request_path)
        signals = ""
        if not (response_path.is_file() and sidecar.is_file() and sidecar.read_text(encoding="ascii").strip() == request_hash):
            if output_dir.exists():
                for child in output_dir.iterdir():
                    if child.is_file():
                        child.unlink()
            output_dir.mkdir(parents=True, exist_ok=True)
            completed = run([sys.executable, str(text_script), "--file", str(request_path), "--output", str(output_dir)])
            signals = completed.stdout + "\n" + completed.stderr
            sidecar.write_text(request_hash, encoding="ascii")
        response = read_json(response_path)
        learning = json.loads(response_content(response))
        if not isinstance(learning, dict):
            raise RuntimeError(f"Structured lesson result is not an object: {video_id}")
        validate_lesson(learning, video_id)
        result_path = result_root / f"{video_id}.learning.json"
        note_path = note_root / f"{video_id}.md"
        write_json(result_path, learning)
        note_path.write_text(markdown_note(source_item, learning), encoding="utf-8")
        usage = response.get("usage") or {}
        row = {
            "video_id": video_id,
            "course_slug": source_item["course_slug"],
            "course_title": source_item["course_title"],
            "playlist_position_observed": source_item["playlist_position_observed"],
            "original_title": source_item["original_title"],
            "source_url": source_item["source_url"],
            "c0": c1_item["c0"],
            "complete_c1": registrations[0],
            "transcript": {
                "path": derivatives[0]["path"],
                "sha256": derivatives[0]["sha256"],
                "derivative_id": registrations[0]["derivative_id"],
            },
            "processing": {
                "provider": "qianwen_skill",
                "model": response.get("model", args.model),
                "tool_version": f"run-cherno-course-learning-round.py@sha256:{script_hash};text.py@sha256:{text_hash}",
                "provider_input_sha256": request_hash,
                "credential_source": "environment",
                "usage": usage,
                "estimated_cost_cny": round(
                    token_count(usage, ("prompt_tokens", "input_tokens")) / 1_000_000 * args.input_price
                    + token_count(usage, ("completion_tokens", "output_tokens")) / 1_000_000 * args.output_price,
                    6,
                ),
            },
            "learning": {
                "path": result_path.relative_to(stage).as_posix(),
                "sha256": sha256_file(result_path),
                "note_path": note_path.relative_to(stage).as_posix(),
                "note_sha256": sha256_file(note_path),
            },
            "update_available_signal_observed": "[UPDATE_AVAILABLE]" in signals,
            "status": "candidate_ready",
        }
        with progress_lock:
            progress["completed_items"] += 1
            persist_progress()
        return row

    rows: list[dict[str, Any]] = []
    failures: list[str] = []
    pilot_item = items[0]
    try:
        rows.append(process_item(pilot_item))
    except Exception as error:  # noqa: BLE001
        with progress_lock:
            progress["failed_items"] += 1
            progress["last_error"] = f"{pilot_item['video_id']}: {error}"
            persist_progress()
        raise RuntimeError(f"Learning pilot failed for {pilot_item['video_id']}: {error}") from error

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as executor:
        future_map = {executor.submit(process_item, item): item for item in items[1:]}
        for future in concurrent.futures.as_completed(future_map):
            item = future_map[future]
            try:
                rows.append(future.result())
            except Exception as error:  # noqa: BLE001
                failures.append(f"{item['video_id']}: {error}")
                with progress_lock:
                    progress["failed_items"] += 1
                    progress["last_error"] = failures[-1]
                    persist_progress()
    if failures:
        raise RuntimeError("Learning round failed:\n" + "\n".join(failures[:20]))

    rows.sort(key=lambda row: (row["course_slug"], int(row["playlist_position_observed"]), row["video_id"]))
    rows_by_course: dict[str, list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        rows_by_course[row["course_slug"]].append(row)

    course_results: list[dict[str, Any]] = []
    for course_slug, course_rows in rows_by_course.items():
        if course_slug not in COURSES:
            raise RuntimeError(f"Unknown course slug: {course_slug}")
        video_ids = [row["video_id"] for row in course_rows]
        compact = []
        for row in course_rows:
            learning = read_json(stage / row["learning"]["path"])
            compact.append(
                {
                    "video_id": row["video_id"],
                    "position": row["playlist_position_observed"],
                    "original_title": row["original_title"],
                    "display_title": learning["display_title"],
                    "summary": learning["summary"],
                    "tags": learning["tags"],
                }
            )
        course_contract = json.dumps(course_schema(video_ids), ensure_ascii=False)
        course_prompt = f"""请把这门编程课程的全部课次组织成 4-10 个互斥且穷尽的学习域。
每个 video_id 必须出现且只出现一次；域顺序应形成合理学习路径；规则要概括实现决策、依赖关系和失败边界，而不是简单重复标题。
同时给出课程总览、可执行项目路径和复习问题。不要提及模型、提示词、发布或存储。
只输出一个 JSON 对象，顶层字段必须且只能是 course_summary、classification_axis、domains、learning_support，并严格满足下面的 JSON Schema：
{course_contract}

Course: {course_rows[0]['course_title']}
Lessons:
{json.dumps(compact, ensure_ascii=False)}
"""
        request = {
            "model": args.model,
            "messages": [
                {"role": "system", "content": "你是资深编程课程架构师。只输出一个符合指定字段合同的简体中文 JSON 对象，不要增加任何未声明字段。"},
                {"role": "user", "content": course_prompt},
            ],
            "response_format": {"type": "json_object"},
            "enable_thinking": False,
            "temperature": 0,
            "max_tokens": 12000,
        }
        request_path = stage / "requests" / "courses" / f"{course_slug}.request.json"
        output_dir = stage / "provider-temp" / "courses" / course_slug
        response_path = output_dir / "response.json"
        sidecar = output_dir / "request.sha256"
        write_json(request_path, request)
        request_hash = sha256_file(request_path)
        signals = ""
        if not (response_path.is_file() and sidecar.is_file() and sidecar.read_text(encoding="ascii").strip() == request_hash):
            output_dir.mkdir(parents=True, exist_ok=True)
            completed = run([sys.executable, str(text_script), "--file", str(request_path), "--output", str(output_dir)])
            signals = completed.stdout + "\n" + completed.stderr
            sidecar.write_text(request_hash, encoding="ascii")
        response = read_json(response_path)
        course_learning = json.loads(response_content(response))
        domains = course_learning.get("domains") or []
        valid_video_ids = set(video_ids)
        seen_video_ids: set[str] = set()
        original_assignment_count = 0
        for domain in domains:
            cleaned: list[str] = []
            for assigned_video_id in domain.get("video_ids", []):
                original_assignment_count += 1
                if assigned_video_id in valid_video_ids and assigned_video_id not in seen_video_ids:
                    cleaned.append(assigned_video_id)
                    seen_video_ids.add(assigned_video_id)
            domain["video_ids"] = cleaned

        missing_video_ids = [video_id for video_id in video_ids if video_id not in seen_video_ids]
        repair_usage: dict[str, Any] = {}
        repair_request_hash = ""
        repair_signals = ""
        if missing_video_ids:
            domain_ids = [str(domain["id"]) for domain in domains]
            compact_by_video = {str(row["video_id"]): row for row in compact}
            repair_contract = {
                "type": "object",
                "additionalProperties": False,
                "required": ["assignments"],
                "properties": {
                    "assignments": {
                        "type": "array",
                        "minItems": len(missing_video_ids),
                        "maxItems": len(missing_video_ids),
                        "items": {
                            "type": "object",
                            "additionalProperties": False,
                            "required": ["video_id", "domain_id"],
                            "properties": {
                                "video_id": {"type": "string", "enum": missing_video_ids},
                                "domain_id": {"type": "string", "enum": domain_ids},
                            },
                        },
                    }
                },
            }
            repair_prompt = f"""下面的课程域已经确定，但原始分区有重复项；重复项已按首次出现去重。请只把尚未分配的课次逐一分配到最合适的现有 domain_id，不得创建新域，不得遗漏或重复 video_id。
只输出一个 JSON 对象，顶层只能有 assignments，并严格满足下面的 JSON Schema：
{json.dumps(repair_contract, ensure_ascii=False)}

Existing domains:
{json.dumps([{'id': domain['id'], 'title': domain['title'], 'description': domain['description'], 'rules': domain['rules']} for domain in domains], ensure_ascii=False)}

Missing lessons:
{json.dumps([compact_by_video[video_id] for video_id in missing_video_ids], ensure_ascii=False)}
"""
            repair_request = {
                "model": args.model,
                "messages": [
                    {"role": "system", "content": "你是编程课程分类修复器。只输出精确覆盖给定 video_id 的 JSON 映射。"},
                    {"role": "user", "content": repair_prompt},
                ],
                "response_format": {"type": "json_object"},
                "enable_thinking": False,
                "temperature": 0,
                "max_tokens": 3000,
            }
            repair_request_path = stage / "requests" / "courses" / f"{course_slug}.partition-repair.request.json"
            repair_output_dir = stage / "provider-temp" / "courses" / f"{course_slug}-partition-repair"
            repair_response_path = repair_output_dir / "response.json"
            repair_sidecar = repair_output_dir / "request.sha256"
            write_json(repair_request_path, repair_request)
            repair_request_hash = sha256_file(repair_request_path)
            if not (
                repair_response_path.is_file()
                and repair_sidecar.is_file()
                and repair_sidecar.read_text(encoding="ascii").strip() == repair_request_hash
            ):
                repair_output_dir.mkdir(parents=True, exist_ok=True)
                completed = run(
                    [sys.executable, str(text_script), "--file", str(repair_request_path), "--output", str(repair_output_dir)]
                )
                repair_signals = completed.stdout + "\n" + completed.stderr
                repair_sidecar.write_text(repair_request_hash, encoding="ascii")
            repair_response = read_json(repair_response_path)
            repair = json.loads(response_content(repair_response))
            assignments = repair.get("assignments") or []
            assignment_video_ids = [str(row.get("video_id", "")) for row in assignments]
            if sorted(assignment_video_ids) != sorted(missing_video_ids) or len(assignment_video_ids) != len(set(assignment_video_ids)):
                raise RuntimeError(f"Course partition repair does not exactly cover missing lessons for {course_slug}")
            domain_by_id = {str(domain["id"]): domain for domain in domains}
            for assignment in assignments:
                domain_id = str(assignment.get("domain_id", ""))
                if domain_id not in domain_by_id:
                    raise RuntimeError(f"Course partition repair returned an unknown domain for {course_slug}")
                domain_by_id[domain_id]["video_ids"].append(str(assignment["video_id"]))
            repair_usage = repair_response.get("usage") or {}

        assigned = [video_id for domain in domains for video_id in domain.get("video_ids", [])]
        if sorted(assigned) != sorted(video_ids) or len(assigned) != len(set(assigned)):
            raise RuntimeError(f"Course domain partition is not exact after repair for {course_slug}")
        course_learning["domains"] = domains
        partition_repair = {
            "deduplicated_assignments": original_assignment_count - len(seen_video_ids),
            "missing_assignments_repaired": len(missing_video_ids),
            "repair_request_sha256": repair_request_hash or None,
        }

        display_by_video = {
            row["video_id"]: read_json(stage / row["learning"]["path"])["display_title"] for row in course_rows
        }
        note_by_video = {
            row["video_id"]: lesson_note_name(
                int(row["playlist_position_observed"]), display_by_video[row["video_id"]]
            )
            for row in course_rows
        }
        sections = []
        for index, domain in enumerate(domains, start=1):
            section_note = f"S{index:02d}-{domain['id']}"
            sections.append(
                {
                    "id": domain["id"],
                    "title": domain["title"],
                    "description": domain["description"],
                    "rules": domain["rules"],
                    "note": section_note,
                    "units": [
                        {
                            "unit_id": video_id,
                            "video_id": video_id,
                            "note": note_by_video[video_id],
                            "title": display_by_video[video_id],
                        }
                        for video_id in domain["video_ids"]
                    ],
                }
            )
        course_meta = COURSES[course_slug]
        short_name = course_meta["short_name"]
        live_root = Path(args.vault_root).resolve() / "Babata" / "Cherno" / short_name
        plan = {
            "schema": "babata.course-presentation-plan/v1",
            "course_key": course_meta["course_key"],
            "course_identity": {
                "channel_id": source_by_video[video_ids[0]]["channel_id"],
                "playlist_id": source_by_video[video_ids[0]]["playlist_id"],
                "playlist_title": source_by_video[video_ids[0]]["playlist_title"],
            },
            "course_version": 1,
            "course_title": course_rows[0]["course_title"],
            "collection": "Cherno",
            "short_name": short_name,
            "expected_units": len(course_rows),
            "profile": "semantic-course-obsidian/v1",
            "output_status": "pending_user_acceptance",
            "source": {
                "source_manifest": str(source_path),
                "source_manifest_sha256": sha256_file(source_path),
                "c1_manifest": str(c1_path),
                "c1_manifest_sha256": sha256_file(c1_path),
                "learning_manifest": str(stage / "manifest.json"),
            },
            "outline": {"mode": "sectioned", "sections": sections},
            "learning_support": [
                {"id": "overview", "title": "课程概览与学习路径", "note": "课程概览与学习路径"},
                {"id": "project", "title": "练习与项目路线", "note": "练习与项目路线"},
                {"id": "review", "title": "复习与自测", "note": "复习与自测"},
                {"id": "visual", "title": "视觉证据索引", "note": "视觉证据索引"},
            ],
            "course_map": {
                "classification_axis": course_learning["classification_axis"],
                "root_id": course_slug.replace("-", "_"),
                "root_label": short_name,
                "tagline": course_learning["course_summary"],
                "domains": [
                    {
                        "id": section["id"],
                        "title": section["title"],
                        "description": section["description"],
                        "rules": section["rules"],
                        "note": section["note"],
                        "unit_ids": [unit["unit_id"] for unit in section["units"]],
                    }
                    for section in sections
                ],
            },
            "learning_support_content": course_learning["learning_support"],
            "live": {
                "path": str(live_root),
                "vault": Path(args.vault_root).resolve().name,
                "file": f"Babata/Cherno/{short_name}/index.md",
            },
        }
        plan_path = plan_root / f"{course_slug}.presentation-plan.json"
        write_json(plan_path, plan)
        course_usage = response.get("usage") or {}
        combined_usage = {
            "prompt_tokens": token_count(course_usage, ("prompt_tokens", "input_tokens"))
            + token_count(repair_usage, ("prompt_tokens", "input_tokens")),
            "completion_tokens": token_count(course_usage, ("completion_tokens", "output_tokens"))
            + token_count(repair_usage, ("completion_tokens", "output_tokens")),
        }
        provider_input_sha256 = hashlib.sha256(
            "\n".join(value for value in (request_hash, repair_request_hash) if value).encode("ascii")
        ).hexdigest()
        course_results.append(
            {
                "course_slug": course_slug,
                "course_key": course_meta["course_key"],
                "short_name": short_name,
                "items": len(course_rows),
                "learning": course_learning,
                "processing": {
                    "provider": "qianwen_skill",
                    "model": response.get("model", args.model),
                    "provider_input_sha256": provider_input_sha256,
                    "usage": combined_usage,
                    "estimated_cost_cny": round(
                        combined_usage["prompt_tokens"] / 1_000_000 * args.input_price
                        + combined_usage["completion_tokens"] / 1_000_000 * args.output_price,
                        6,
                    ),
                },
                "partition_repair": partition_repair,
                "presentation_plan": str(plan_path),
                "presentation_plan_sha256": sha256_file(plan_path),
                "update_available_signal_observed": "[UPDATE_AVAILABLE]" in (signals + repair_signals),
            }
        )

    prompt_tokens = sum(
        token_count(row["processing"]["usage"], ("prompt_tokens", "input_tokens")) for row in rows
    ) + sum(token_count(row["processing"]["usage"], ("prompt_tokens", "input_tokens")) for row in course_results)
    completion_tokens = sum(
        token_count(row["processing"]["usage"], ("completion_tokens", "output_tokens")) for row in rows
    ) + sum(token_count(row["processing"]["usage"], ("completion_tokens", "output_tokens")) for row in course_results)
    estimated_cost = round(
        prompt_tokens / 1_000_000 * args.input_price + completion_tokens / 1_000_000 * args.output_price, 6
    )
    created_at = utc_now()
    manifest = {
        "schema": "babata.cherno-course-learning/v1",
        "created_at": created_at,
        "status": "candidate_ready",
        "source_manifest": str(source_path),
        "source_manifest_sha256": sha256_file(source_path),
        "c1_manifest": str(c1_path),
        "c1_manifest_sha256": sha256_file(c1_path),
        "pricing": {
            "schema": "babata.provider-pricing/v1",
            "retrieved_at": created_at,
            "source_url": args.pricing_source,
            "model": args.model,
            "currency": "CNY",
            "input_price_per_million_tokens": args.input_price,
            "output_price_per_million_tokens": args.output_price,
            "prompt_tokens": prompt_tokens,
            "completion_tokens": completion_tokens,
            "estimated_cost_cny": estimated_cost,
            "free_tier_applied": "unknown_until_billing_evidence",
        },
        "items": rows,
        "courses": course_results,
    }
    manifest_path = stage / "manifest.json"
    write_json(manifest_path, manifest)

    report = "\n".join(
        [
            "# Cherno course learning round",
            "",
            f"- Lesson learning candidates: {len(rows)}/{args.expected_items}.",
            f"- Course presentation plans: {len(course_results)}/3.",
            f"- Qwen prompt/completion tokens: {prompt_tokens} / {completion_tokens}.",
            f"- Estimated model cost before free-tier reconciliation: {estimated_cost} CNY.",
            "- Status: candidate learning notes and presentation plans; formal semantic/course registration is still required.",
            "",
        ]
    )
    (stage / "REPORT.md").write_text(report, encoding="utf-8")
    progress.update({"status": "complete", "updated_at": utc_now(), "completed_items": len(rows)})
    persist_progress()
    print(
        json.dumps(
            {
                "items": len(rows),
                "courses": len(course_results),
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
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
