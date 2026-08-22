#!/usr/bin/env python3
"""Register Cherno learning candidates, knowledge entries, map nodes, lens, and courses."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import sqlite3
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


COURSE_CONFIG = {
    "cpp": {
        "course_key": "cherno-cpp",
        "branch_name": "C++程序设计",
        "source": "The Cherno YouTube",
        "foundations": [
            ("mapnode_p6_consciousness", "primary", 95, 95, "C++ 课程核心是语言语义、抽象、资源所有权与程序设计判断。"),
            ("mapnode_p6_matter", "secondary", 70, 85, "内存、对象布局和机器资源构成 C++ 实现的物质约束。"),
        ],
    },
    "game-engine": {
        "course_key": "cherno-game-engine",
        "branch_name": "游戏引擎开发",
        "source": "The Cherno YouTube",
        "foundations": [
            ("mapnode_p6_consciousness", "primary", 95, 95, "引擎架构需要持续做模块、接口、调试和演进决策。"),
            ("mapnode_p6_matter", "primary", 90, 92, "渲染、内存、物理和性能直接受计算资源约束。"),
            ("mapnode_p6_space", "secondary", 80, 88, "场景、坐标、变换和空间组织是引擎的基础结构。"),
            ("mapnode_p6_time", "secondary", 75, 85, "帧循环、动画、物理步进和事件顺序具有明确时间结构。"),
        ],
    },
    "opengl": {
        "course_key": "cherno-opengl",
        "branch_name": "OpenGL图形编程",
        "source": "The Cherno YouTube",
        "foundations": [
            ("mapnode_p6_space", "primary", 95, 95, "OpenGL 课程直接处理坐标、变换、几何和屏幕空间。"),
            ("mapnode_p6_matter", "primary", 90, 92, "缓冲、纹理、GPU 状态和像素输出受计算资源约束。"),
            ("mapnode_p6_consciousness", "secondary", 85, 90, "图形管线与 API 状态需要明确的程序设计和调试判断。"),
        ],
    },
}


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


def unique_tags(values: list[str]) -> list[str]:
    """Preserve tag order while avoiding Babata's normalized-tag collisions."""
    seen: set[str] = set()
    result: list[str] = []
    for value in values:
        tag = str(value).strip()
        key = " ".join(tag.casefold().replace("-", " ").split())
        if not tag or key in seen:
            continue
        seen.add(key)
        result.append(tag)
    return result


def invoke_json(executable: Path, arguments: list[str], env: dict[str, str]) -> Any:
    completed = subprocess.run(
        [str(executable), "--json", *arguments],
        text=True,
        encoding="utf-8",
        errors="replace",
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env=env,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            f"babata failed ({completed.returncode}): {' '.join(arguments)}\n"
            f"stdout: {completed.stdout[-4000:]}\nstderr: {completed.stderr[-4000:]}"
        )
    text = completed.stdout.strip()
    if not text:
        raise RuntimeError(f"babata returned no JSON: {' '.join(arguments)}")
    return json.loads(text)


def database_rows(connection: sqlite3.Connection, sql: str, parameters: tuple[Any, ...] = ()) -> list[sqlite3.Row]:
    return list(connection.execute(sql, parameters))


def get_or_create_node(
    *,
    babata: Path,
    env: dict[str, str],
    raw: sqlite3.Connection,
    level: str,
    name: str,
    parents: list[str],
    rationale: str,
) -> str:
    rows = database_rows(
        raw,
        "SELECT map_node_id FROM knowledge_map_nodes WHERE node_level=? AND name=? AND lifecycle_state='active'",
        (level, name),
    )
    if len(rows) > 1:
        raise RuntimeError(f"Multiple active map nodes match {level}/{name}")
    if len(rows) == 1:
        node_id = str(rows[0]["map_node_id"])
    else:
        arguments = ["knowledge", "create-map-node", "--level", level, "--name", name]
        for parent in parents:
            arguments.extend(["--parent", parent])
        arguments.extend(["--rationale", rationale])
        result = invoke_json(babata, arguments, env)
        node_id = str(result["map_node_id"])
    parent_rows = database_rows(
        raw,
        "SELECT parent_node_id FROM knowledge_map_edges WHERE child_node_id=? ORDER BY parent_node_id",
        (node_id,),
    )
    actual = sorted(str(row["parent_node_id"]) for row in parent_rows)
    if actual != sorted(parents):
        raise RuntimeError(f"Map node parent mismatch for {name}: {actual} != {sorted(parents)}")
    return node_id


def find_run(
    *,
    babata: Path,
    env: dict[str, str],
    revision_id: str,
    output_sha256: str,
    model: str,
) -> dict[str, Any] | None:
    runs = invoke_json(babata, ["process", "list-runs", "--revision", revision_id], env)
    matches: list[dict[str, Any]] = []
    for run in runs:
        if (
            run.get("pipeline_id") == "agent_import"
            and run.get("target_kind") == "structured_result"
            and run.get("provider") == "qianwen_skill"
            and run.get("tool_or_model") == model
            and run.get("state") == "succeeded"
            and run.get("invalidated_at") is None
        ):
            detail = invoke_json(babata, ["process", "show-run", "--run", str(run["id"])], env)
            derivatives = detail.get("derivatives") or []
            if len(derivatives) == 1 and derivatives[0].get("output_sha256") == output_sha256:
                matches.append(detail)
    if len(matches) > 1:
        raise RuntimeError(f"Multiple active semantic package runs match {revision_id}/{output_sha256}")
    return matches[0] if matches else None


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--learning-manifest",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-learning-full-20260822-v1\manifest.json",
    )
    parser.add_argument(
        "--c1b-ledger",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c1b-vision-full-20260822-v1\c1b-registration-ledger.json",
    )
    parser.add_argument(
        "--stage",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-knowledge-registration-20260822-v1",
    )
    parser.add_argument(
        "--babata",
        default=r"C:\Users\Aiano\Babata\01_app\target\debug\babata.exe",
    )
    parser.add_argument("--data-home", default=os.environ.get("BABATA_DATA_HOME", r"D:\BabataData"))
    parser.add_argument("--expected-items", type=int, default=269)
    args = parser.parse_args()

    learning_path = Path(args.learning_manifest).resolve()
    c1b_path = Path(args.c1b_ledger).resolve()
    stage = Path(args.stage).resolve()
    babata = Path(args.babata).resolve()
    data_home = Path(args.data_home).resolve()
    for required in (learning_path, c1b_path, babata):
        if not required.is_file():
            raise RuntimeError(f"Missing required input: {required}")
    raw_db = data_home / "01_raw" / "index" / "raw.sqlite"
    derived_db = data_home / "02_derived" / "index" / "derived.sqlite"
    for database in (raw_db, derived_db):
        if not database.is_file():
            raise RuntimeError(f"Missing Babata database: {database}")
    stage.mkdir(parents=True, exist_ok=True)
    semantic_root = stage / "semantic-packages"
    semantic_root.mkdir(parents=True, exist_ok=True)

    learning = read_json(learning_path)
    c1b = read_json(c1b_path)
    learning_items = list(learning.get("items", []))
    c1b_items = list(c1b.get("items", []))
    if learning.get("status") != "candidate_ready" or len(learning_items) != args.expected_items:
        raise RuntimeError("Learning manifest is not a complete candidate round")
    if c1b.get("status") != "registered" or len(c1b_items) != args.expected_items:
        raise RuntimeError("C1B registration ledger is not complete")
    c1b_by_video = {str(item["video_id"]): item for item in c1b_items}
    if len(c1b_by_video) != args.expected_items:
        raise RuntimeError("C1B ledger contains duplicate video identities")

    env = dict(os.environ)
    env["BABATA_DATA_HOME"] = str(data_home)
    raw = sqlite3.connect(raw_db)
    raw.row_factory = sqlite3.Row
    derived = sqlite3.connect(derived_db)
    derived.row_factory = sqlite3.Row
    try:
        discipline_id = get_or_create_node(
            babata=babata,
            env=env,
            raw=raw,
            level="discipline",
            name="计算机科学与软件工程",
            parents=["mapnode_p6_consciousness", "mapnode_p6_matter"],
            rationale="Cherno 编程课程的正式学科节点，连接程序语义与计算资源实现。",
        )
        branch_ids: dict[str, str] = {}
        for course_slug, config in COURSE_CONFIG.items():
            branch_ids[course_slug] = get_or_create_node(
                babata=babata,
                env=env,
                raw=raw,
                level="branch",
                name=config["branch_name"],
                parents=[discipline_id],
                rationale=f"The Cherno {config['branch_name']} 课程的正式覆盖分支。",
            )

        state_path = stage / "registration-state.json"
        if state_path.is_file():
            state = read_json(state_path)
        else:
            state = {
                "schema": "babata.cherno-course-knowledge-registration-state/v1",
                "status": "in_progress",
                "created_at": utc_now(),
                "learning_manifest": str(learning_path),
                "learning_manifest_sha256": sha256_file(learning_path),
                "c1b_ledger": str(c1b_path),
                "c1b_ledger_sha256": sha256_file(c1b_path),
                "discipline_id": discipline_id,
                "branch_ids": branch_ids,
                "lens": None,
                "items": [],
                "courses": [],
            }
            write_json(state_path, state)
        if state["learning_manifest_sha256"] != sha256_file(learning_path) or state["c1b_ledger_sha256"] != sha256_file(c1b_path):
            raise RuntimeError("Registration state is bound to different inputs")

        state_by_video = {str(item["video_id"]): item for item in state.get("items", [])}
        for item in learning_items:
            video_id = str(item["video_id"])
            if video_id not in c1b_by_video:
                raise RuntimeError(f"C1B ledger is missing {video_id}")
            if video_id in state_by_video and state_by_video[video_id].get("status") == "registered":
                continue
            learning_file = learning_path.parent / item["learning"]["path"]
            note_file = learning_path.parent / item["learning"]["note_path"]
            if sha256_file(learning_file) != item["learning"]["sha256"] or sha256_file(note_file) != item["learning"]["note_sha256"]:
                raise RuntimeError(f"Learning candidate hash drift for {video_id}")
            learning_value = read_json(learning_file)
            c1b_item = c1b_by_video[video_id]
            # The batched registrar stores all C1B registrations in a single
            # `registrations` list, while the original knowledge registrar
            # expected the older `decision_registration` shortcut.  Accept
            # either shape, but require exactly one structured-result
            # decision so we never silently bind to a media derivative.
            decision = c1b_item.get("decision_registration")
            if decision is None:
                candidates = [
                    row
                    for row in c1b_item.get("registrations", [])
                    if row.get("kind") == "structured_result"
                ]
                if len(candidates) != 1:
                    raise RuntimeError(
                        f"C1B ledger must expose exactly one structured_result decision for {video_id}"
                    )
                decision = candidates[0]
            evidence = [
                {
                    "derivative_id": item["transcript"]["derivative_id"],
                    "output_sha256": item["transcript"]["sha256"],
                },
                {
                    "derivative_id": decision["derivative_id"],
                    "output_sha256": decision["output_sha256"],
                },
            ]
            map_refs = {
                "cpp": ["foundation:consciousness", "foundation:matter"],
                "game-engine": ["foundation:consciousness", "foundation:matter", "foundation:space", "foundation:time"],
                "opengl": ["foundation:space", "foundation:matter", "foundation:consciousness"],
            }[item["course_slug"]]
            details = note_file.read_text(encoding="utf-8-sig")
            package = {
                "schema_version": "p6-semantic-candidate/v1",
                "source_item_id": item["c0"]["item_id"],
                "source_revision_id": item["c0"]["revision_id"],
                "evidence_derivatives": evidence,
                "provider": "qianwen_skill",
                "model": item["processing"]["model"],
                "model_version": item["processing"]["model"],
                "prompt_version": "cherno-course-learning/v1",
                "generated_at": utc_now(),
                "map_nodes": [],
                "entries": [
                    {
                        "local_ref": f"entry:cherno-{video_id}",
                        "title": learning_value["display_title"],
                        "payload": {"kind": "knowledge", "statement": learning_value["summary"], "details": details},
                        "map_node_refs": map_refs,
                        "tags": unique_tags(["Cherno", item["course_slug"], video_id, *learning_value["tags"]]),
                        "dense_expressions": [
                            {
                                "kind": "outline",
                                "content": "\n".join(f"- {value['name']}" for value in learning_value["key_concepts"]),
                            }
                        ],
                        "relevance": {
                            "interest": 65,
                            "strategy": 65,
                            "consensus": 70,
                            "rationale": "正式课程语义登记使用中性基线；课程内容与视觉验收仍由用户独立完成。",
                        },
                    }
                ],
                "relations": [],
                "limitations": [
                    "语义条目绑定完整 ASR transcript 与正式 C1B essence decision；视频原件不进入语义正文。",
                    f"learning_manifest_sha256:{sha256_file(learning_path)}",
                    f"c1b_ledger_sha256:{sha256_file(c1b_path)}",
                ],
            }
            package_path = semantic_root / item["course_slug"] / f"{video_id}.semantic-candidate.json"
            write_json(package_path, package)
            package_hash = sha256_file(package_path)
            existing = find_run(
                babata=babata,
                env=env,
                revision_id=item["c0"]["revision_id"],
                output_sha256=package_hash,
                model=item["processing"]["model"],
            )
            if existing is None:
                params = {
                    "service": "dashscope",
                    "adapter": "qianwen_skill",
                    "credential_source": "environment",
                    "provider_input_sha256": item["processing"]["provider_input_sha256"],
                    "learning_candidate_sha256": item["learning"]["sha256"],
                    "transcript_sha256": item["transcript"]["sha256"],
                    "c1b_decision_sha256": decision["output_sha256"],
                    "preprocessing": ["complete registered transcript", "formal C1B decision read-back"],
                }
                receipt = invoke_json(
                    babata,
                    [
                        "process",
                        "register",
                        "--pipeline",
                        "agent_import",
                        "--revision",
                        item["c0"]["revision_id"],
                        "--item",
                        item["c0"]["item_id"],
                        "--kind",
                        "structured_result",
                        "--provider",
                        "qianwen_skill",
                        "--model",
                        item["processing"]["model"],
                        "--tool-version",
                        item["processing"]["tool_version"],
                        "--input-sha256",
                        item["c0"]["input_sha256"],
                        "--input-asset-id",
                        item["c0"]["asset_id"],
                        "--json-file",
                        str(package_path),
                        "--output-file",
                        str(package_path),
                        "--params-json",
                        json.dumps(params, ensure_ascii=False, separators=(",", ":")),
                        "--usage-json",
                        json.dumps(item["processing"]["usage"], ensure_ascii=False, separators=(",", ":")),
                        "--language",
                        "zh",
                        "--loss-notes",
                        "学习笔记由完整 ASR 与正式 C1B 决策约束；短暂视觉变化仍受联系表采样限制。",
                    ],
                    env,
                )
                existing = invoke_json(babata, ["process", "show-run", "--run", receipt["run_id"]], env)
            derivatives = existing.get("derivatives") or []
            if len(derivatives) != 1 or derivatives[0].get("output_sha256") != package_hash:
                raise RuntimeError(f"Semantic package read-back mismatch for {video_id}")
            logical_path = str(derivatives[0].get("logical_path", ""))
            if not logical_path.startswith("02_derived/files/sha256/"):
                raise RuntimeError(f"Semantic package did not enter managed C1 storage for {video_id}")
            managed_path = data_home / Path(logical_path.replace("/", os.sep))
            if not managed_path.is_file() or sha256_file(managed_path) != package_hash:
                raise RuntimeError(f"Managed semantic package hash mismatch for {video_id}")
            derivative_id = str(derivatives[0]["id"])
            suggestion_rows = database_rows(
                raw,
                "SELECT suggestion_id FROM model_suggestions WHERE source_derivative_id=? AND source_output_sha256=?",
                (derivative_id, package_hash),
            )
            if len(suggestion_rows) > 1:
                raise RuntimeError(f"Duplicate semantic suggestion fingerprint for {video_id}")
            if suggestion_rows:
                suggestion_id = str(suggestion_rows[0]["suggestion_id"])
                semantic_rows = database_rows(raw, "SELECT semantic_id FROM semantic_entries WHERE suggestion_id=?", (suggestion_id,))
                if len(semantic_rows) != 1:
                    raise RuntimeError(f"Semantic suggestion has no unique entry for {video_id}")
                semantic_id = str(semantic_rows[0]["semantic_id"])
            else:
                ingested = invoke_json(babata, ["knowledge", "ingest", "--derivative", derivative_id], env)
                suggestion_id = str(ingested["suggestion_id"])
                semantic_ids = ingested.get("semantic_ids") or []
                if len(semantic_ids) != 1:
                    raise RuntimeError(f"Knowledge ingest returned no unique semantic id for {video_id}")
                semantic_id = str(semantic_ids[0])
            reviews = database_rows(raw, "SELECT decision FROM suggestion_reviews WHERE suggestion_id=?", (suggestion_id,))
            if any(str(review["decision"]) in {"rejected", "modified"} for review in reviews):
                raise RuntimeError(f"Semantic suggestion is rejected/modified for {video_id}")
            if not any(str(review["decision"]) == "accepted" for review in reviews):
                invoke_json(babata, ["knowledge", "review-suggestion", "--suggestion", suggestion_id, "--decision", "accept"], env)
            row = {
                "video_id": video_id,
                "course_slug": item["course_slug"],
                "module_id": video_id,
                "source_item_id": item["c0"]["item_id"],
                "source_revision_id": item["c0"]["revision_id"],
                "source_asset_id": item["c0"]["asset_id"],
                "source_asset_sha256": item["c0"]["input_sha256"],
                "semantic_package": str(package_path),
                "semantic_package_sha256": package_hash,
                "process_run_id": existing["run"]["id"],
                "process_derivative_id": derivative_id,
                "suggestion_id": suggestion_id,
                "semantic_id": semantic_id,
                "status": "registered",
            }
            state_by_video[video_id] = row
            state["items"] = sorted(state_by_video.values(), key=lambda value: (value["course_slug"], value["video_id"]))
            write_json(state_path, state)

        course_plans = {row["course_slug"]: read_json(Path(row["presentation_plan"])) for row in learning["courses"]}
        if set(course_plans) != set(COURSE_CONFIG):
            raise RuntimeError("Learning manifest does not contain exactly three course plans")
        if state.get("lens") is None:
            lens_definition = {
                "title": "The Cherno programming course lens",
                "purpose": "Versioned non-owning navigation lens for the C++, Game Engine, and OpenGL course registrations.",
                "selection": {},
                "manual_include": [],
                "manual_exclude": [],
                "course_refs": [f"course:{COURSE_CONFIG[slug]['course_key']}@1" for slug in COURSE_CONFIG],
                "map_node_refs": [branch_ids[slug] for slug in COURSE_CONFIG],
                "organisation_rules": ["map_then_title", "title"],
                "include_unreviewed": False,
            }
            lens_path = stage / "cherno-course-lens-v1.json"
            write_json(lens_path, lens_definition)
            lens = invoke_json(babata, ["sublibraries", "create", "--definition", str(lens_path)], env)
            state["lens"] = {
                "id": lens["id"],
                "version": 1,
                "definition": str(lens_path),
                "definition_sha256": sha256_file(lens_path),
                "authority": lens.get("authority"),
            }
            write_json(state_path, state)
        lens_id = state["lens"]["id"]

        semantic_by_video = {row["video_id"]: row for row in state["items"]}
        courses: list[dict[str, Any]] = []
        for course_slug, plan in course_plans.items():
            config = COURSE_CONFIG[course_slug]
            chapter_by_video: dict[str, str] = {}
            for section in plan["outline"]["sections"]:
                for unit in section["units"]:
                    if unit["video_id"] in chapter_by_video:
                        raise RuntimeError(f"Duplicate presentation assignment: {unit['video_id']}")
                    chapter_by_video[unit["video_id"]] = section["id"]
            course_rows = [row for row in learning_items if row["course_slug"] == course_slug]
            if len(chapter_by_video) != len(course_rows):
                raise RuntimeError(f"Presentation plan denominator mismatch for {course_slug}")
            modules = []
            for item in sorted(course_rows, key=lambda value: int(value["playlist_position_observed"])):
                semantic = semantic_by_video[item["video_id"]]
                assignments = [
                    {
                        "map_node_id": branch_ids[course_slug],
                        "role": "primary",
                        "strength": 100,
                        "confidence": 100,
                        "rationale": f"该语义条目直接属于 {config['branch_name']} 课程覆盖分支。",
                        "method_version": "cherno-course-map/v1",
                    },
                    {
                        "map_node_id": discipline_id,
                        "role": "secondary",
                        "strength": 95,
                        "confidence": 100,
                        "rationale": "该课次属于计算机科学与软件工程学科。",
                        "method_version": "cherno-course-map/v1",
                    },
                ]
                for node_id, role, strength, confidence, rationale in config["foundations"]:
                    assignments.append(
                        {
                            "map_node_id": node_id,
                            "role": role,
                            "strength": strength,
                            "confidence": confidence,
                            "rationale": rationale,
                            "method_version": "cherno-course-map/v1",
                        }
                    )
                modules.append(
                    {
                        "module_id": item["video_id"],
                        "semantic_id": semantic["semantic_id"],
                        "chapter_id": chapter_by_video[item["video_id"]],
                        "assignments": assignments,
                    }
                )
            relations = []
            if course_slug == "game-engine":
                relations = [
                    {
                        "from_map_node_id": branch_ids["game-engine"],
                        "kind": "draws_from",
                        "to_map_node_id": branch_ids["cpp"],
                        "rationale": "游戏引擎实现持续使用 C++ 的语言、内存和抽象能力。",
                    },
                    {
                        "from_map_node_id": branch_ids["game-engine"],
                        "kind": "applies_to",
                        "to_map_node_id": branch_ids["opengl"],
                        "rationale": "引擎渲染子系统把 OpenGL 图形管线用于真实编辑器和场景。",
                    },
                ]
            elif course_slug == "opengl":
                relations = [
                    {
                        "from_map_node_id": branch_ids["opengl"],
                        "kind": "prerequisite_of",
                        "to_map_node_id": branch_ids["game-engine"],
                        "rationale": "OpenGL 管线知识是理解引擎渲染实现的一项前置能力。",
                    }
                ]
            definition = {
                "schema_version": "babata.course-registration/v1",
                "course_key": config["course_key"],
                "version": 1,
                "title": plan["course_title"],
                "source": config["source"],
                "term": "evergreen",
                "acceptance_state": "pending_user_acceptance",
                "closure_state": "open",
                "branches": [
                    {
                        "branch_map_node_id": branch_ids[course_slug],
                        "rationale": f"课程完整覆盖 {config['branch_name']} 分支。",
                    }
                ],
                "modules": modules,
                "map_relations": relations,
                "lens": {"sublibrary_id": lens_id, "definition_version": 1},
                "author_kind": "machine",
                "author": "codex-cherno-course-refresh",
                "created_at": state["created_at"],
            }
            definition_path = stage / f"{course_slug}.course-registration-v1.json"
            write_json(definition_path, definition)
            existing_courses = database_rows(
                raw, "SELECT course_id,definition_sha256 FROM courses WHERE course_key=? AND course_version=1", (config["course_key"],)
            )
            if existing_courses:
                detail = invoke_json(babata, ["knowledge", "show-course", "--course", config["course_key"], "--version", "1"], env)
            else:
                detail = invoke_json(babata, ["knowledge", "register-course", "--definition", str(definition_path)], env)
                detail = invoke_json(babata, ["knowledge", "show-course", "--course", config["course_key"], "--version", "1"], env)
            # Babata hashes its Rust pretty-serialization of the parsed
            # definition; the Python file bytes are structurally identical
            # but can have a different byte-level hash.  Compare the parsed
            # definition and retain Babata's read-back hash as authoritative.
            if detail.get("definition") != definition or len(detail["definition"]["modules"]) != len(modules):
                raise RuntimeError(f"Course read-back mismatch for {config['course_key']}")
            courses.append(
                {
                    "course_slug": course_slug,
                    "course_key": config["course_key"],
                    "course_id": detail["course_id"],
                    "definition": str(definition_path),
                    "definition_sha256": detail["definition_sha256"],
                    "modules": len(modules),
                    "assignments": sum(len(module["assignments"]) for module in modules),
                    "acceptance_state": detail["definition"]["acceptance_state"],
                    "closure_state": detail["definition"]["closure_state"],
                }
            )
        state["courses"] = courses
        state["status"] = "registered"
        state["completed_at"] = utc_now()
        write_json(state_path, state)

        ledger = {
            "schema": "babata.cherno-course-knowledge-registration/v1",
            "status": "registered",
            "course_acceptance": "pending_user_acceptance",
            "created_at": state["completed_at"],
            "learning_manifest": str(learning_path),
            "learning_manifest_sha256": sha256_file(learning_path),
            "c1b_ledger": str(c1b_path),
            "c1b_ledger_sha256": sha256_file(c1b_path),
            "discipline": {"map_node_id": discipline_id, "name": "计算机科学与软件工程"},
            "branches": [
                {"course_slug": slug, "map_node_id": branch_ids[slug], "name": COURSE_CONFIG[slug]["branch_name"]}
                for slug in COURSE_CONFIG
            ],
            "lens": state["lens"],
            "coverage": {
                "items": len(state["items"]),
                "semantic_packages_registered": len(state["items"]),
                "semantic_entries_reviewed": len(state["items"]),
                "courses_registered": len(courses),
            },
            "items": state["items"],
            "courses": courses,
            "direct_sql_writes": 0,
        }
        ledger_path = stage / "knowledge-registration-ledger.json"
        write_json(ledger_path, ledger)
        print(
            json.dumps(
                {
                    "items": len(state["items"]),
                    "courses": len(courses),
                    "discipline_id": discipline_id,
                    "lens_id": state["lens"]["id"],
                    "ledger": str(ledger_path),
                },
                ensure_ascii=False,
                indent=2,
            )
        )
    finally:
        raw.close()
        derived.close()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: {exc}", file=sys.stderr)
        raise
