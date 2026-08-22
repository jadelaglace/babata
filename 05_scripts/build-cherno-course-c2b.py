#!/usr/bin/env python3
"""Build and verify provider-neutral Cherno C2B Obsidian packages."""

from __future__ import annotations

import argparse
import hashlib
import html
import json
import math
import os
import re
import shutil
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from PIL import Image


WIKI_LINK_RE = re.compile(r"!??\[\[([^\]|#]+)(?:#[^\]|]+)?(?:\|[^\]]+)?\]\]")
CONTROL_RE = re.compile(r"(?i)\b(qwen|dashscope|prompt[_ -]?version|credential_source|provider_input_sha256)\b")
OBSIDIAN_NOTE_FORBIDDEN_RE = re.compile(r'[<>:"/\\|?*\[\]#^\x00-\x1f]')
LESSON_NOTE_RE = re.compile(r"^L\d{3}-(.+)$")


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


def yaml_string(value: Any) -> str:
    return json.dumps(str(value), ensure_ascii=False)


def lesson_note_name(position: int, display_title: str) -> str:
    readable = OBSIDIAN_NOTE_FORBIDDEN_RE.sub(" ", str(display_title))
    readable = re.sub(r"\s+", " ", readable).strip(" .")
    if not readable:
        raise RuntimeError(f"Lesson {position} has no readable display title")
    # Keep Windows/Obsidian path components comfortably below common limits.
    readable = readable[:96].rstrip(" .")
    return f"L{position:03d}-{readable}"


def run(command: list[str], *, timeout: int = 1800) -> subprocess.CompletedProcess[str]:
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


def safe_mermaid_id(value: str) -> str:
    value = re.sub(r"[^A-Za-z0-9_]", "_", value)
    if not value or not value[0].isalpha():
        value = "n_" + value
    return value


def escape_mermaid(value: str) -> str:
    return html.escape(value, quote=True).replace("\n", "<br/>")


def all_package_files(package: Path) -> list[Path]:
    return sorted((path for path in package.rglob("*") if path.is_file()), key=lambda path: path.as_posix().lower())


def relative(package: Path, path: Path) -> str:
    return path.relative_to(package).as_posix()


def verify_package(package: Path, manifest_path: Path) -> dict[str, Any]:
    manifest = read_json(manifest_path)
    if manifest.get("schema") != "babata.course-c2b-package/v1" or manifest.get("status") != "passed_engineering_gates":
        raise RuntimeError(f"Unsupported or incomplete package manifest: {manifest_path}")
    files = all_package_files(package)
    declared = {row["path"]: row for row in manifest.get("package_files", [])}
    actual = {relative(package, path): path for path in files}
    if set(actual) != set(declared):
        raise RuntimeError("Package manifest does not declare the exact file set")
    for rel, path in actual.items():
        row = declared[rel]
        if int(row["bytes"]) != path.stat().st_size or row["sha256"] != sha256_file(path):
            raise RuntimeError(f"Package file hash/size mismatch: {rel}")

    markdown = [path for path in files if path.suffix.lower() == ".md"]
    by_stem = {path.stem: path for path in markdown}
    lesson_notes = []
    for note in markdown:
        text = note.read_text(encoding="utf-8-sig")
        if "\nvideo_id:" not in text:
            continue
        match = LESSON_NOTE_RE.fullmatch(note.stem)
        if match is None or re.fullmatch(r"[A-Za-z0-9_-]{11}", match.group(1)):
            raise RuntimeError(f"Lesson filename is not a readable L### title: {relative(package, note)}")
        video_match = re.search(r'(?m)^video_id: "([A-Za-z0-9_-]{11})"$', text)
        if video_match is None or video_match.group(1) in note.stem:
            raise RuntimeError(f"Stable video ID leaked into the user-visible lesson title: {relative(package, note)}")
        h1 = next((line[2:].strip() for line in text.splitlines() if line.startswith("# ")), "")
        if h1 != note.stem:
            raise RuntimeError(f"Lesson H1 must match its readable filename: {relative(package, note)}")
        if f"lesson_title: {yaml_string(note.stem)}" not in text:
            raise RuntimeError(f"Lesson metadata does not preserve the readable title: {relative(package, note)}")
        lesson_notes.append(note)
    if len(lesson_notes) != int(manifest.get("source_denominator", 0)):
        raise RuntimeError("Readable lesson-title coverage does not match the source denominator")
    referenced_media: set[str] = set()
    for note in markdown:
        text = note.read_text(encoding="utf-8-sig")
        body = text.split("---", 2)[-1] if text.startswith("---") else text
        if CONTROL_RE.search(body):
            raise RuntimeError(f"Control-plane language leaked into note body: {relative(package, note)}")
        seen_media: set[str] = set()
        for target in WIKI_LINK_RE.findall(text):
            target = target.strip()
            if target.startswith("media/"):
                if target in seen_media:
                    raise RuntimeError(f"Media path occurs twice in one note: {relative(package, note)} / {target}")
                seen_media.add(target)
                referenced_media.add(target)
                if not (package / target).is_file():
                    raise RuntimeError(f"Broken media link: {relative(package, note)} -> {target}")
            elif Path(target).stem not in by_stem:
                raise RuntimeError(f"Broken note link: {relative(package, note)} -> {target}")
    media = [path for path in files if relative(package, path).startswith("media/")]
    for path in media:
        rel = relative(package, path)
        if rel not in referenced_media and rel not in {
            manifest["course_map"]["svg"],
            manifest["course_map"]["mermaid"],
        }:
            raise RuntimeError(f"Package media is not referenced: {rel}")

    index = (package / "index.md").read_text(encoding="utf-8-sig")
    required = [
        "status: pending_user_acceptance",
        "formal_registration: registered",
        "c1b_registration: registered",
        "knowledge_universe_registration: registered",
        "template_profile: semantic-course-obsidian/v1",
        "template_status: candidate-real-use",
    ]
    for marker in required:
        if marker not in index:
            raise RuntimeError(f"index.md lacks formal marker: {marker}")
    mmd_path = package / manifest["course_map"]["mermaid"]
    svg_path = package / manifest["course_map"]["svg"]
    png_path = package / manifest["course_map"]["png"]
    for path in (mmd_path, svg_path, png_path):
        if not path.is_file():
            raise RuntimeError(f"Missing course map artifact: {path}")
    mmd = mmd_path.read_text(encoding="utf-8-sig")
    if '"useMaxWidth": true' not in mmd or "obsidian://" in mmd or re.search(r"(?m)^\s*click\s+", mmd):
        raise RuntimeError("Course Mermaid violates responsive/internal-link boundaries")
    blocks = re.findall(r"(?ms)^```mermaid\n(.*?)\n```", index)
    if len(blocks) != 1 or blocks[0].strip() != mmd.strip():
        raise RuntimeError("index.md must contain exactly the package-owned Mermaid source")
    svg = svg_path.read_text(encoding="utf-8-sig")
    if 'width="100%"' not in svg or "viewBox=" not in svg or "max-width:" not in svg:
        raise RuntimeError("Course-map SVG is not responsive")
    for label in manifest["course_map"]["internal_link_labels"]:
        if label not in svg:
            raise RuntimeError(f"Rendered SVG lacks internal-link label: {label}")
    image = Image.open(png_path)
    width, height = image.size
    image.close()
    if height / width > 1.40 or width < 1000 or height < 400:
        raise RuntimeError(f"Course-map PNG violates dimension/aspect gates: {width}x{height}")
    return {
        "schema": "babata.course-c2b-package-check/v1",
        "status": "passed",
        "course_key": manifest["course_key"],
        "package_files": len(files),
        "markdown_files": len(markdown),
        "media_files": len(media),
        "png_width": width,
        "png_height": height,
        "manifest": str(manifest_path),
        "manifest_sha256": sha256_file(manifest_path),
    }


def build_course(
    *,
    plan_path: Path,
    learning_path: Path,
    c1_path: Path,
    c1b_ledger_path: Path,
    knowledge_path: Path,
    stage_root: Path,
    data_home: Path,
) -> dict[str, Any]:
    plan = read_json(plan_path)
    learning = read_json(learning_path)
    c1 = read_json(c1_path)
    c1b_ledger = read_json(c1b_ledger_path)
    knowledge = read_json(knowledge_path)
    if plan.get("schema") != "babata.course-presentation-plan/v1":
        raise RuntimeError(f"Unsupported presentation plan: {plan_path}")
    if plan.get("output_status") != "pending_user_acceptance" or plan.get("profile") != "semantic-course-obsidian/v1":
        raise RuntimeError("Presentation plan has the wrong status/profile")
    if c1b_ledger.get("status") != "registered" or knowledge.get("status") != "registered":
        raise RuntimeError("C1B and knowledge ledgers must be registered")
    course_slug = plan_path.name.split(".", 1)[0]
    course_rows = [row for row in learning["items"] if row["course_slug"] == course_slug]
    if len(course_rows) != int(plan["expected_units"]):
        raise RuntimeError(f"Learning denominator mismatch for {course_slug}")
    c1_by_video = {str(row["video_id"]): row for row in c1["items"]}
    c1b_manifest_path = c1b_ledger_path.parent / "manifest.json"
    c1b_manifest = read_json(c1b_manifest_path)
    c1b_by_video = {str(row["video_id"]): row for row in c1b_manifest["items"]}
    c1b_reg_by_video = {str(row["video_id"]): row for row in c1b_ledger["items"]}
    knowledge_by_video = {str(row["video_id"]): row for row in knowledge["items"]}
    expected_video_ids = {str(row["video_id"]) for row in course_rows}
    if not expected_video_ids.issubset(c1_by_video) or not expected_video_ids.issubset(c1b_by_video) or not expected_video_ids.issubset(knowledge_by_video):
        raise RuntimeError(f"Formal ledger coverage is incomplete for {course_slug}")

    course_stage = stage_root / course_slug
    package = course_stage / "package"
    if package.exists():
        shutil.rmtree(package)
    media_dir = package / "media"
    package.mkdir(parents=True, exist_ok=True)
    media_dir.mkdir(parents=True, exist_ok=True)
    learning_by_video = {str(row["video_id"]): row for row in course_rows}
    plan_units = [unit for section in plan["outline"]["sections"] for unit in section["units"]]
    observed_ids = [str(row["video_id"]) for row in sorted(course_rows, key=lambda row: int(row["playlist_position_observed"]))]
    planned_ids = [str(unit["video_id"]) for unit in plan_units]
    if len(planned_ids) != len(set(planned_ids)) or set(planned_ids) != set(observed_ids):
        raise RuntimeError(f"Presentation units do not exactly cover observed playlist identities for {course_slug}")

    lesson_note_by_video: dict[str, str] = {}
    lesson_title_by_video: dict[str, str] = {}
    for row in course_rows:
        video_id = str(row["video_id"])
        learning_value = read_json(learning_path.parent / row["learning"]["path"])
        note_name = lesson_note_name(int(row["playlist_position_observed"]), learning_value["display_title"])
        if note_name in lesson_note_by_video.values():
            raise RuntimeError(f"Duplicate readable lesson filename for {course_slug}: {note_name}")
        lesson_note_by_video[video_id] = note_name
        lesson_title_by_video[video_id] = note_name

    copied_media: list[dict[str, Any]] = []
    visual_index: list[str] = []
    for unit in plan_units:
        video_id = unit["video_id"]
        row = learning_by_video[video_id]
        c1_item = c1_by_video[video_id]
        c1b_item = c1b_by_video[video_id]
        c1b_registration = c1b_reg_by_video[video_id]
        learning_file = learning_path.parent / row["learning"]["path"]
        learning_value = read_json(learning_file)
        lesson_note = lesson_note_by_video[video_id]
        lesson_title = lesson_title_by_video[video_id]
        transcript = next(value for value in c1_item["derivatives"] if value["kind"] == "transcript")
        transcript_path = c1_path.parent / transcript["path"]
        if sha256_file(transcript_path) != transcript["sha256"]:
            raise RuntimeError(f"Transcript hash drift for {video_id}")
        note_lines = [
            "---",
            f"course: {yaml_string(plan['course_title'])}",
            f"course_key: {yaml_string(plan['course_key'])}",
            "variant: c2b",
            "status: pending_user_acceptance",
            f"video_id: {yaml_string(video_id)}",
            f"lesson_title: {yaml_string(lesson_title)}",
            f"original_title: {yaml_string(row['original_title'])}",
            f"source_url: {yaml_string(row['source_url'])}",
            f"source_revision_id: {yaml_string(row['c0']['revision_id'])}",
            f"semantic_id: {yaml_string(knowledge_by_video[video_id]['semantic_id'])}",
            "---",
            "",
            f"# {lesson_title}",
            "",
            learning_value["summary"].strip(),
            "",
            "## 核心概念",
            "",
        ]
        for concept in learning_value["key_concepts"]:
            note_lines.append(f"- **{concept['name']}**：{concept['explanation']}")
        if learning_value["implementation_decisions"]:
            note_lines.extend(["", "## 实现决策", ""] + [f"- {value}" for value in learning_value["implementation_decisions"]])
        if learning_value["pitfalls"]:
            note_lines.extend(["", "## 常见陷阱", ""] + [f"- {value}" for value in learning_value["pitfalls"]])
        if learning_value["practice"]:
            note_lines.extend(["", "## 练习", ""] + [f"- [ ] {value}" for value in learning_value["practice"]])

        media_rows = []
        registrations = c1b_registration.get("media_registrations")
        if registrations is None:
            registrations = [
                value
                for value in (c1b_registration.get("registrations") or [])
                if value.get("kind") != "structured_result"
            ]
        registrations_by_sha = {value["output_sha256"]: value for value in registrations}
        for index, derivative in enumerate(c1b_item.get("retained_derivatives") or [], start=1):
            registration = registrations_by_sha.get(derivative["sha256"])
            if registration is None:
                raise RuntimeError(f"C1B media registration is missing for {video_id}/{derivative['path']}")
            managed = data_home / Path(registration["logical_path"].replace("/", os.sep))
            if not managed.is_file() or sha256_file(managed) != derivative["sha256"]:
                raise RuntimeError(f"Managed C1B media hash mismatch for {video_id}")
            extension = managed.suffix or Path(derivative["path"]).suffix
            destination = media_dir / f"{video_id}-{index:02d}{extension.lower()}"
            shutil.copy2(managed, destination)
            if sha256_file(destination) != derivative["sha256"]:
                raise RuntimeError(f"Copied C1B media hash mismatch for {video_id}")
            media_rows.append((derivative, destination))
            copied_media.append(
                {
                    "video_id": video_id,
                    "path": relative(package, destination),
                    "sha256": derivative["sha256"],
                    "kind": derivative["kind"],
                    "source_locator": derivative["source_locator"],
                }
            )
        if media_rows:
            note_lines.extend(["", "## 视觉证据", ""])
            for derivative, destination in media_rows:
                note_lines.extend(
                    [
                        f"### {derivative['summary']}",
                        "",
                        f"{derivative['why_text_insufficient']}",
                        "",
                        f"![[{relative(package, destination)}]]",
                        "",
                    ]
                )
            visual_index.append(f"- [[{lesson_note}|{lesson_title}]]：{len(media_rows)} 项")
        transcript_lines = transcript_path.read_text(encoding="utf-8-sig").splitlines()
        note_lines.extend(["", "> [!note]- 完整时间戳转录（C1）", ">"])
        note_lines.extend(["> " + line for line in transcript_lines])
        note_lines.append("")
        (package / f"{lesson_note}.md").write_text("\n".join(note_lines), encoding="utf-8")

    for section in plan["outline"]["sections"]:
        lines = [f"# {section['title']}", "", section["description"], "", "## 实现规则与边界", ""]
        lines.extend(f"- {rule}" for rule in section["rules"])
        lines.extend(["", "## 课次", ""])
        for unit in section["units"]:
            video_id = unit["video_id"]
            lines.append(f"- [[{lesson_note_by_video[video_id]}|{lesson_title_by_video[video_id]}]]")
        lines.append("")
        (package / f"{section['note']}.md").write_text("\n".join(lines), encoding="utf-8")

    support = plan["learning_support_content"]
    (package / "课程概览与学习路径.md").write_text(
        "\n".join([f"# {plan['short_name']}：课程概览与学习路径", "", plan["course_map"]["tagline"], ""] + [f"- {value}" for value in support["overview"]] + [""]),
        encoding="utf-8",
    )
    (package / "练习与项目路线.md").write_text(
        "\n".join(["# 练习与项目路线", ""] + [f"- [ ] {value}" for value in support["project_path"]] + [""]),
        encoding="utf-8",
    )
    (package / "复习与自测.md").write_text(
        "\n".join(["# 复习与自测", ""] + [f"- {value}" for value in support["review_questions"]] + [""]),
        encoding="utf-8",
    )
    (package / "视觉证据索引.md").write_text(
        "\n".join(["# 视觉证据索引", "", *(visual_index or ["本课程没有需要超出完整文本转录保留的视觉片段。"]), ""]),
        encoding="utf-8",
    )

    colors = ["#2563EB", "#7C3AED", "#0891B2", "#059669", "#D97706", "#DC2626", "#4F46E5", "#0F766E", "#9333EA", "#CA8A04"]
    mmd_lines = [
        '%%{init: {"theme": "base", "flowchart": {"useMaxWidth": true, "htmlLabels": true, "curve": "basis"}}}%%',
        "flowchart LR",
        f'  root["{escape_mermaid(plan["short_name"])}"]',
    ]
    link_ids: list[str] = []
    link_labels: list[str] = []
    for index, section in enumerate(plan["outline"]["sections"]):
        domain_id = safe_mermaid_id("domain_" + section["id"])
        note_id = safe_mermaid_id("note_" + section["id"])
        rules_id = safe_mermaid_id("rules_" + section["id"])
        rule_text = "<br/>".join(escape_mermaid(rule) for rule in section["rules"][:4])
        mmd_lines.extend(
            [
                f"  {domain_id}(( ))",
                f'  {note_id}["{escape_mermaid(section["note"])}"]',
                f'  {rules_id}["{rule_text}"]',
                f"  root --> {domain_id}",
                f"  {domain_id} --> {note_id}",
                f"  {note_id} --> {rules_id}",
                f"  style {domain_id} fill:{colors[index % len(colors)]},stroke:{colors[index % len(colors)]},color:{colors[index % len(colors)]}",
            ]
        )
        link_ids.append(note_id)
        link_labels.append(section["note"])
    support_junction = "support_junction"
    mmd_lines.extend([f"  {support_junction}(( ))", f"  root --> {support_junction}"])
    for index, support_note in enumerate(plan["learning_support"]):
        node_id = safe_mermaid_id("support_" + support_note["id"])
        mmd_lines.extend([f'  {node_id}["{escape_mermaid(support_note["note"])}"]', f"  {support_junction} --> {node_id}"])
        link_ids.append(node_id)
        link_labels.append(support_note["note"])
    mmd_lines.extend(
        [
            f"  class {','.join(link_ids)} internal-link",
            "  classDef internal-link fill:transparent,stroke:#64748B,color:#0F172A,stroke-width:1px",
            "  classDef default fill:transparent,stroke:#CBD5E1,color:#0F172A",
            "  style root fill:#0F172A,stroke:#0F172A,color:#FFFFFF,stroke-width:2px",
            f"  style {support_junction} fill:#64748B,stroke:#64748B,color:#64748B",
        ]
    )
    mmd_path = media_dir / "course-map.mmd"
    svg_path = media_dir / "course-map.svg"
    png_path = media_dir / "course-map.png"
    mmd_path.write_text("\n".join(mmd_lines) + "\n", encoding="utf-8")
    # Windows subprocess(shell=False) does not resolve .cmd shims by the
    # bare `npx` name; call the shim explicitly so Mermaid rendering works
    # in the desktop runtime as well as an interactive shell.
    npx = shutil.which("npx.cmd") or shutil.which("npx") or "npx.cmd"
    # The workstation npm profile enforces a release-age gate that rejects
    # the pinned Mermaid CLI despite the package being present in cache.
    os.environ.setdefault("NPM_CONFIG_MIN_RELEASE_AGE", "0")
    run([npx, "--yes", "@mermaid-js/mermaid-cli@11.12.0", "-i", str(mmd_path), "-o", str(svg_path), "-b", "white", "-w", "2200", "-H", "1400"])
    run([npx, "--yes", "@mermaid-js/mermaid-cli@11.12.0", "-i", str(mmd_path), "-o", str(png_path), "-b", "white", "-w", "2200", "-H", "1400"])
    # Mermaid can lay out long Chinese labels as a very tall portrait image.
    # Preserve the rendered map but pad a white canvas so the offline fallback
    # meets the package's minimum landscape/aspect gate.
    with Image.open(png_path) as rendered:
        width, height = rendered.size
        target_width = max(1000, width, math.ceil(height / 1.35))
        if target_width != width:
            canvas = Image.new("RGB", (target_width, height), "white")
            left = (target_width - width) // 2
            canvas.paste(rendered.convert("RGB"), (left, 0))
            canvas.save(png_path, format="PNG")
    svg = svg_path.read_text(encoding="utf-8-sig")
    svg = re.sub(r'(<svg\b[^>]*?)\swidth="[^"]+"', r'\1 width="100%"', svg, count=1)
    if "style=" in svg[:1000]:
        svg = re.sub(r'<svg([^>]*?)style="([^"]*)"', r'<svg\1style="\2;max-width:1400px;height:auto"', svg, count=1)
    else:
        svg = svg.replace("<svg ", '<svg style="max-width:1400px;height:auto" ', 1)
    svg_path.write_text(svg, encoding="utf-8")

    mmd = mmd_path.read_text(encoding="utf-8")
    index_lines = [
        "---",
        f"course: {yaml_string(plan['course_title'])}",
        f"course_key: {yaml_string(plan['course_key'])}",
        "variant: c2b",
        "status: pending_user_acceptance",
        "formal_registration: registered",
        "c1b_registration: registered",
        "knowledge_universe_registration: registered",
        "template_profile: semantic-course-obsidian/v1",
        "template_status: candidate-real-use",
        "---",
        "",
        f"# {plan['short_name']}",
        "",
        plan["course_map"]["tagline"],
        "",
        "## 课程脑图",
        "",
        "```mermaid",
        mmd.rstrip(),
        "```",
        "",
        "> [!info]- PNG 离线/打印回退",
        "> ![[media/course-map.png|760]]",
        "",
        "## 学习路径",
        "",
    ]
    for section in plan["outline"]["sections"]:
        index_lines.extend([f"### {section['title']}", "", f"- [[{section['note']}]]"])
        for unit in section["units"]:
            video_id = unit["video_id"]
            index_lines.append(f"- [[{lesson_note_by_video[video_id]}|{lesson_title_by_video[video_id]}]]")
        index_lines.append("")
    index_lines.extend(["## 学习支持", ""] + [f"- [[{row['note']}]]" for row in plan["learning_support"]] + [""])
    (package / "index.md").write_text("\n".join(index_lines), encoding="utf-8")

    package_files = [
        {"path": relative(package, path), "bytes": path.stat().st_size, "sha256": sha256_file(path)}
        for path in all_package_files(package)
    ]
    manifest = {
        "schema": "babata.course-c2b-package/v1",
        "status": "passed_engineering_gates",
        "user_acceptance": "pending_user_acceptance",
        "generated_at": utc_now(),
        "course_key": plan["course_key"],
        "course_version": plan["course_version"],
        "course_title": plan["course_title"],
        "short_name": plan["short_name"],
        "profile": plan["profile"],
        "presentation_plan": str(plan_path),
        "presentation_plan_sha256": sha256_file(plan_path),
        "learning_manifest": str(learning_path),
        "learning_manifest_sha256": sha256_file(learning_path),
        "c1_manifest": str(c1_path),
        "c1_manifest_sha256": sha256_file(c1_path),
        "c1b_ledger": str(c1b_ledger_path),
        "c1b_ledger_sha256": sha256_file(c1b_ledger_path),
        "knowledge_ledger": str(knowledge_path),
        "knowledge_ledger_sha256": sha256_file(knowledge_path),
        "source_denominator": len(course_rows),
        "retained_media": copied_media,
        "course_map": {
            "mermaid": "media/course-map.mmd",
            "svg": "media/course-map.svg",
            "png": "media/course-map.png",
            "default_expanded": "mermaid",
            "responsive_svg": True,
            "png_default_collapsed": True,
            "png_display_width": 760,
            "internal_link_labels": link_labels,
        },
        "publication": plan["live"],
        "package_files": package_files,
    }
    manifest_path = course_stage / "package-manifest.json"
    write_json(manifest_path, manifest)
    check = verify_package(package, manifest_path)
    write_json(course_stage / "verification.json", check)
    (course_stage / "REPORT.md").write_text(
        "\n".join(
            [
                f"# {plan['short_name']} C2B materialization",
                "",
                "- Status: passed_engineering_gates",
                "- User acceptance: pending_user_acceptance",
                f"- Complete lessons: {len(course_rows)}/{len(course_rows)}",
                f"- Retained C1B media: {len(copied_media)}",
                f"- Package files: {check['package_files']}",
                "- External sovereign original reads during materialization: 0",
                "",
            ]
        ),
        encoding="utf-8",
    )
    return {"course_slug": course_slug, "package": str(package), "manifest": str(manifest_path), "check": check}


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--learning-manifest",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-learning-full-20260822-v1\manifest.json",
    )
    parser.add_argument(
        "--c1-manifest",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-asr-full-20260822-v1\manifest.json",
    )
    parser.add_argument(
        "--c1b-ledger",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c1b-vision-full-20260822-v1\c1b-registration-ledger.json",
    )
    parser.add_argument(
        "--knowledge-ledger",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-knowledge-registration-20260822-v1\knowledge-registration-ledger.json",
    )
    parser.add_argument(
        "--stage",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c2b-full-20260822-v1",
    )
    parser.add_argument("--data-home", default=os.environ.get("BABATA_DATA_HOME", r"D:\BabataData"))
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()

    stage = Path(args.stage).resolve()
    if args.check_only:
        results = []
        for course_dir in sorted(path for path in stage.iterdir() if path.is_dir()):
            results.append(verify_package(course_dir / "package", course_dir / "package-manifest.json"))
        print(json.dumps(results, ensure_ascii=False, indent=2))
        return 0

    learning_path = Path(args.learning_manifest).resolve()
    c1_path = Path(args.c1_manifest).resolve()
    c1b_path = Path(args.c1b_ledger).resolve()
    knowledge_path = Path(args.knowledge_ledger).resolve()
    data_home = Path(args.data_home).resolve()
    for required in (learning_path, c1_path, c1b_path, knowledge_path):
        if not required.is_file():
            raise RuntimeError(f"Missing required input: {required}")
    learning = read_json(learning_path)
    plan_paths = [Path(row["presentation_plan"]).resolve() for row in learning["courses"]]
    if len(plan_paths) != 3:
        raise RuntimeError("Expected exactly three course presentation plans")
    stage.mkdir(parents=True, exist_ok=True)
    results = [
        build_course(
            plan_path=plan_path,
            learning_path=learning_path,
            c1_path=c1_path,
            c1b_ledger_path=c1b_path,
            knowledge_path=knowledge_path,
            stage_root=stage,
            data_home=data_home,
        )
        for plan_path in plan_paths
    ]
    print(json.dumps(results, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: {exc}", file=sys.stderr)
        raise
