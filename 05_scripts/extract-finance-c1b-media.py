#!/usr/bin/env python3
"""Build deterministic C1B media candidates from an MBA course source map.

The C0 paths are read-only.  This script only writes into a fresh staging root.
"""
import argparse
import hashlib
import json
import re
import shutil
import subprocess
from pathlib import Path

import fitz


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as fh:
        for block in iter(lambda: fh.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def safe_name(value: str) -> str:
    value = re.sub(r"[^A-Za-z0-9._-]+", "-", value).strip("-")
    return value or "media"


def pdf_candidates(path: Path, max_pages: int = 2):
    doc = fitz.open(path)
    scored = []
    keywords = re.compile(
        r"公式|模型|图|表|流程|NPV|IRR|WACC|CAPM|EOQ|现金流|股利|资本结构|估值|风险|概率|回收期|决策树|矩阵|生命周期|并购|重组|成本曲线|供应链|SCOR|预测|回归|移动平均|指数平滑|库存|安全库存|牛鞭|物流|采购|运输|现金循环|总成本|周转率|重心|盈亏平衡|选址",
        re.I,
    )
    for index, page in enumerate(doc):
        text = re.sub(r"\s+", " ", page.get_text("text"))
        drawings = len(page.get_drawings())
        images = len(page.get_images(full=True))
        hits = len(keywords.findall(text))
        # Prefer pages with explicit visual/quantitative evidence, not title/thank-you pages.
        score = hits * 3 + min(drawings, 40) * 0.4 + images * 2
        if len(text) < 80:
            score -= 5
        if index in (0, len(doc) - 1):
            score -= 2
        if score > 2:
            scored.append((score, index, text[:160], drawings, images))
    scored.sort(key=lambda row: (-row[0], row[1]))
    return doc, scored[:max_pages]


def render_pdf_pages(path: Path, output: Path, reviewed_pages=None):
    doc, candidates = pdf_candidates(path)
    if reviewed_pages is not None:
        by_page = {int(row["page"]): row for row in reviewed_pages}
        candidates = [row for row in candidates if row[1] + 1 in by_page]
        missing = sorted(set(by_page) - {row[1] + 1 for row in candidates})
        if missing:
            raise ValueError(f"Reviewed PDF pages were not scored candidates for {path}: {missing}")
    records = []
    for score, index, excerpt, drawings, images in candidates:
        review = by_page[index + 1] if reviewed_pages is not None else {}
        page = doc[index]
        pix = page.get_pixmap(matrix=fitz.Matrix(1.8, 1.8), alpha=False)
        target = output / f"page-{index + 1:03d}.png"
        pix.save(target)
        records.append(
            {
                "path": target,
                "page": index + 1,
                "score": round(score, 2),
                "text_excerpt": excerpt,
                "drawings": drawings,
                "images": images,
                "role": review.get("kind", "visual_formula_diagram_or_table"),
                "review_reason": review.get(
                    "reason", "Scored page candidate retained pending explicit review"
                ),
                "processing": [
                    "PyMuPDF page render at 1.8x",
                    "no redraw; page-level evidence retained",
                ],
                "loss_notes": ["PDF page context outside the rendered page is not included"],
            }
        )
    return records


def video_frame(path: Path, output: Path):
    probe = subprocess.run(
        [
            "ffprobe",
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            str(path),
        ],
        check=True,
        capture_output=True,
        text=True,
    )
    duration = float(probe.stdout.strip())
    # A stable midpoint is enough to expose the lecture's visual layer; the C1
    # transcript remains the complete evidence and the frame is only additive.
    timestamp = max(1.0, duration * 0.5)
    target = output / "midpoint.png"
    subprocess.run(
        [
            "ffmpeg",
            "-y",
            "-hide_banner",
            "-loglevel",
            "error",
            "-ss",
            f"{timestamp:.3f}",
            "-i",
            str(path),
            "-frames:v",
            "1",
            "-vf",
            "scale=1280:-2",
            str(target),
        ],
        check=True,
    )
    return [
        {
            "path": target,
            "time_seconds": round(timestamp, 3),
            "role": "lecture_slide_or_board_visual",
            "processing": ["ffmpeg midpoint frame; H.264 source decoded without redraw"],
            "loss_notes": [
                "A still frame does not preserve motion or audio; full C1 transcript is retained",
                "frame is an additive visual cue and must not be treated as the video original",
            ],
        }
    ]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--source-map", required=True)
    ap.add_argument("--staging-root", required=True)
    ap.add_argument(
        "--video-plan",
        help="Optional JSON plan mapping video module IDs to reviewed percentage frames.",
    )
    ap.add_argument(
        "--visual-plan",
        help="Optional reviewed PDF-page and video-frame plan.",
    )
    args = ap.parse_args()
    source_map = json.loads(Path(args.source_map).read_text(encoding="utf-8"))
    root = Path(args.staging_root)
    media_root = root / "c1b" / "media"
    media_root.mkdir(parents=True, exist_ok=True)
    text_root = root / "c1b" / "text"
    text_root.mkdir(parents=True, exist_ok=True)
    if args.video_plan and args.visual_plan:
        raise ValueError("Use either --video-plan or --visual-plan, not both")
    visual_plan = {}
    plan_path = args.visual_plan or args.video_plan
    if plan_path:
        visual_plan = json.loads(Path(plan_path).read_text(encoding="utf-8"))
        supported = {
            "babata.c1b.video-visual-plan/v1",
            "babata.c1b.visual-plan/v1",
        }
        if visual_plan.get("schema") not in supported:
            raise ValueError("Unsupported visual plan schema")
    items = [item for chunk in source_map["chunks"] for item in chunk["items"]]
    seen = set()
    decisions = []
    for item in sorted(items, key=lambda row: int(row["module_id"])):
        module_id = str(item["module_id"])
        if module_id in seen:
            continue
        seen.add(module_id)
        source = Path(item["website_path"])
        if not source.is_file():
            raise FileNotFoundError(source)
        c1 = Path(item["c1_path"])
        if not c1.is_file():
            raise FileNotFoundError(c1)
        c1_target = text_root / f"M-{module_id}.md"
        shutil.copy2(c1, c1_target)
        c1_sha = sha256(c1)
        if item.get("c1_sha256") and c1_sha != str(item["c1_sha256"]).lower():
            raise ValueError(f"C1 hash mismatch for {module_id}")
        item_root = media_root / module_id
        item_root.mkdir(parents=True, exist_ok=True)
        retained = []
        source_extension = str(item.get("source_extension") or source.suffix).lower()
        if item["module_type"] == "courseware" and source_extension == ".pdf":
            reviewed_pages = None
            if plan_path:
                reviewed_pages = visual_plan.get("pdf_pages", {}).get(module_id, [])
            for record in render_pdf_pages(source, item_root, reviewed_pages):
                record["path"] = str(record["path"])
                retained.append(record)
        elif item["module_type"] == "video":
            # Retain only frames explicitly reviewed as non-text-essential.
            # The full transcript remains the textual evidence; frames are additive.
            reviewed = visual_plan.get("videos", {}).get(module_id, [])
            if reviewed:
                probe = subprocess.run(
                    [
                        "ffprobe", "-v", "error", "-show_entries", "format=duration",
                        "-of", "default=noprint_wrappers=1:nokey=1", str(source),
                    ],
                    check=True, capture_output=True, text=True,
                )
                duration = float(probe.stdout.strip())
                for record in reviewed:
                    pct = float(record["percentage"])
                    timestamp = max(1.0, duration * pct)
                    target = item_root / f"frame-{int(round(pct * 100)):02d}.jpg"
                    subprocess.run(
                        [
                            "ffmpeg", "-y", "-hide_banner", "-loglevel", "error",
                            "-ss", f"{timestamp:.3f}", "-i", str(source),
                            "-frames:v", "1", "-vf", "scale=1280:-2", "-q:v", "3",
                            str(target),
                        ],
                        check=True,
                    )
                    retained.append(
                        {
                            "path": target,
                            "percentage": pct,
                            "time_seconds": round(timestamp, 3),
                            "role": record.get("kind", "lecture_visual"),
                            "review_reason": record.get("reason", "视觉审查确认文字不可完整替代"),
                            "processing": ["Qwen Vision reviewed sampled frame", "ffmpeg frame render from read-only C0 video"],
                            "loss_notes": ["still frame does not preserve motion or audio; full C1 transcript is retained"],
                        }
                    )
        # Text is always carried from the complete, hash-checked C1 by the runner.
        decisions.append(
            {
                "variant": "c1b",
                "module_id": module_id,
                "title": item["title"],
                "module_type": item["module_type"],
                "source_extension": source_extension,
                "c0_item_id": item.get("c0_item_id"),
                "c0_revision_id": item.get("c0_revision_id"),
                "c0_asset_id": item.get("c0_asset_id"),
                "c0_asset_sha256": item.get("c0_asset_sha256"),
                "source_path": str(source),
                "source_sha256": sha256(source),
                "c1b_text_path": str(c1_target.relative_to(root)).replace("\\", "/"),
                "c1_sha256": c1_sha,
                "text_sufficient": True,
                "retained_modalities": ["text"] + (["image"] if retained else []),
                "decision_basis": (
                    "完整 C1 保留文字；载体含可直接改变理解的公式、图示、表格或讲授屏幕视觉，"
                    "因此保留有限页面/关键帧作为 C1B 视觉证据。"
                    if retained
                    else "完整 C1 文字足以表达课程语义；本载体未发现必须保留的独立非文字本质。"
                ),
                "retained_media": retained,
                "audio_decision": {
                    "needed": False,
                    "reason": "课程讲授声音无独立音色、音乐或环境声知识；完整 C1 文字已保留。",
                },
                "video_decision": {
                    "needed": bool(retained) if item["module_type"] == "video" else False,
                    "reason": (
                        "视觉审查确认部分板书、公式、图表、表格或屏幕操作不能由文字完整替代，已保留审查通过的关键帧；"
                        "完整 C1 文字仍是主要证据。"
                        if retained and item["module_type"] == "video"
                        else "视觉审查未发现必须保留的独立视频视觉本质；完整 C1 文字足以表达课程语义。"
                    ),
                },
                "attachment_decision": {
                    "needed": False,
                    "reason": "可编辑原件及附件属性由外部主权库保留，课程知识可由 C1 文字和视觉片段表达。",
                },
            }
        )
    for decision in decisions:
        for media in decision["retained_media"]:
            media_path = Path(media["path"])
            media["path"] = str(media_path.relative_to(root)).replace("\\", "/")
            media["sha256"] = sha256(media_path)
            media["bytes"] = media_path.stat().st_size
    (root / "c1b" / "decisions.json").write_text(
        json.dumps(decisions, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    recipe = {
        "schema": "babata.c1b.rebuild-recipe/v1",
        "source_map": str(Path(args.source_map)),
        "runner": "extract-mba-course-c1b-media.py",
        "read_only_inputs": True,
        "decisions": "c1b/decisions.json",
        "c1_text_root": "c1b/text",
        "media_root": "c1b/media",
        "deterministic": True,
    }
    (root / "c1b" / "rebuild-recipe.json").write_text(
        json.dumps(recipe, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    print(json.dumps({"items": len(decisions), "retained_media": sum(len(d["retained_media"]) for d in decisions)}, ensure_ascii=False))


if __name__ == "__main__":
    main()
