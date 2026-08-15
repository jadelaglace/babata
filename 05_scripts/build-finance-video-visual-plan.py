#!/usr/bin/env python3
"""Convert reviewed Qwen Vision batches into a stable C1B video plan."""
import argparse
import json
import re
from pathlib import Path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--probe-root", required=True)
    ap.add_argument("--output", required=True)
    args = ap.parse_args()
    root = Path(args.probe_root)
    ordered_files = sorted((root / "flat").glob("*.jpg"), key=lambda p: p.name)
    videos = {}
    for batch_index, batch_path in enumerate(sorted(root.glob("unique-batch-*.json"))):
        raw = json.loads(batch_path.read_text(encoding="utf-8"))
        text = str(raw.get("text", "")).strip()
        text = re.sub(r"^```json\s*", "", text)
        text = re.sub(r"\s*```$", "", text)
        rows = json.loads(text)
        batch = ordered_files[batch_index * 7 : batch_index * 7 + len(rows)]
        if len(batch) != len(rows):
            raise ValueError(f"Vision result batch alignment failed: {batch_path}")
        for path, row in zip(batch, rows):
            if not row.get("visual_knowledge"):
                continue
            match = re.match(r"M-(\d+)-frame-(\d+)\.jpg$", path.name)
            if not match:
                raise ValueError(f"Unexpected frame name: {path.name}")
            module_id = match.group(1)
            percentage = int(match.group(2)) / 100.0
            videos.setdefault(module_id, []).append(
                {
                    "percentage": percentage,
                    "kind": row.get("kind", "lecture_visual"),
                    "reason": row.get("reason", "视觉审查确认文字不可完整替代"),
                    "review_file": path.name,
                }
            )
    for records in videos.values():
        records.sort(key=lambda r: r["percentage"])
    payload = {
        "schema": "babata.c1b.video-visual-plan/v1",
        "review": {"adapter": "qianwen_vision", "model": "qwen3.6-plus", "sample_percentages": [0.15, 0.5, 0.85]},
        "videos": videos,
    }
    target = Path(args.output)
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps({"videos": len(videos), "frames": sum(len(v) for v in videos.values())}, ensure_ascii=False))


if __name__ == "__main__":
    main()
