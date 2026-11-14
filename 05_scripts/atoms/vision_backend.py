#!/usr/bin/env python3
"""Babata vision/render backend adapters (narrow, replaceable).

ask    : image(s) + question -> answer, via `bl` (Qwen-VL through Bailian CLI) or `host`
         (records the host agent's own answer file; the host must actually have read the image).
matte  : complex background removal via a local rembg interpreter (BABATA_REMBG_PYTHON).
probe  : installed / configured / probed state of each backend, without spending calls.

Adapters never write Babata databases. Results go to a new run directory (same run
semantics as image_ops.py). Empty, truncated or malformed answers are failures.
Credentials are resolved by the backend tool itself and never passed on the command line.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

sys.path.insert(0, str(Path(__file__).resolve().parent))
import image_ops as io  # noqa: E402

ADAPTER_VERSION = "1.0.0"


def validate_answer(content, finish_reason, expect: str):
    """Return (state, reason, parsed). Never treats empty/truncated/bad JSON as success."""
    if finish_reason == "length":
        return "failed", "truncated", None
    if content is None or not str(content).strip():
        return "failed", "empty-content", None
    if finish_reason not in (None, "stop", "end_turn", "host"):
        return "failed", f"finish-reason-{finish_reason}", None
    if expect == "json":
        text = str(content).strip()
        fence = re.search(r"```(?:json)?\s*(.*?)```", text, re.S)
        if fence:
            text = fence.group(1).strip()
        try:
            return "succeeded", "", json.loads(text)
        except json.JSONDecodeError as exc:
            return "failed", f"bad-json: {exc.msg}", None
    return "succeeded", "", str(content)


def compose_labeled(paths: list[str], records: list[dict], limits) -> np.ndarray:
    imgs = [io.load_image(p, f"image{i}", limits)[0] for i, p in enumerate(paths)]
    h = max(i.shape[0] for i in imgs) + 28
    w = sum(i.shape[1] for i in imgs) + 16 * (len(imgs) + 1)
    sheet = Image.new("RGBA", (w, h), (255, 255, 255, 255))
    d = ImageDraw.Draw(sheet)
    x = 16
    for n, img in enumerate(imgs):
        d.text((x, 6), f"{chr(65 + n)}", fill=(200, 0, 0, 255))
        sheet.alpha_composite(Image.fromarray(img, "RGBA"), (x, 24))
        x += img.shape[1] + 16
    return np.asarray(sheet)


def call_bl(image: str, prompt: str, model: str, timeout: int):
    exe = shutil.which("bl")
    if not exe:
        raise io.AtomError("unavailable", "bl not found on PATH")
    cmd = [exe, "vision", "describe", "--image", image, "--prompt", prompt, "--model", model,
           "--output", "json", "--timeout", str(timeout), "--quiet"]
    started = time.time()
    try:
        proc = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout + 30)
    except subprocess.TimeoutExpired:
        return {"transport": "timeout", "elapsed_ms": int((time.time() - started) * 1000),
                "note": "local wait stopped; remote state unknown, do not blindly resubmit"}
    elapsed = int((time.time() - started) * 1000)
    try:
        data = json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"transport": "bad-output", "exit": proc.returncode, "elapsed_ms": elapsed,
                "stderr_tail": proc.stderr[-400:], "stdout_head": proc.stdout[:400]}
    choice = (data.get("choices") or [{}])[0]
    msg = choice.get("message") or {}
    return {"transport": "ok" if proc.returncode == 0 else f"exit-{proc.returncode}", "elapsed_ms": elapsed,
            "content": msg.get("content"), "finish_reason": choice.get("finish_reason"), "usage": data.get("usage"),
            "model": data.get("model"), "request_id": data.get("id"),
            "error": data.get("error") if proc.returncode else None}


def cmd_ask(args, limits):
    images = [str(Path(p)) for p in args.image]
    recs = []
    for i, p in enumerate(images):
        _, rec = io.load_image(p, f"image{i}", limits)
        recs.append(rec)
    if args.backend == "host":
        if not args.answer_file or not args.model:
            raise io.AtomError("invalid-input", "host backend needs --answer-file and --model (the host's real model id)")
        answer_path = Path(args.answer_file)
        answer_text = answer_path.read_text(encoding="utf-8")
        answer_sha = io.sha256_file(answer_path)
    req = {"backend": args.backend, "model": args.model, "images": [r["sha256"] for r in recs], "prompt": args.prompt,
           "expect": args.expect, "max_side": args.max_side}
    if args.backend == "host":
        req["answer_sha256"] = answer_sha
    run = io.Run(args.out, "ask", req)
    if (prev := run.existing()):
        return prev
    with run:
        sent_note = []
        if args.backend == "bl":
            if len(images) > 1:
                sheet = compose_labeled(images, recs, limits)
                sent = run.add("sent-image", "sent.png", sheet, relation="images composed side by side, labelled A,B,...")
                sent_note.append("multiple images composed into one labelled sheet")
            else:
                img = io.load_image(images[0], "image0", limits)[0]
                h, w = img.shape[:2]
                if max(h, w) > args.max_side:
                    s = args.max_side / max(h, w)
                    img = io.resize_rgba(img, max(1, round(w * s)), max(1, round(h * s)))
                    sent_note.append(f"downscaled x{s:.4f} before sending")
                sent = run.add("sent-image", "sent.png", img, relation="exact pixels sent to backend")
            resp = call_bl(str(sent), args.prompt, args.model or "qwen3-vl-plus", args.timeout)
            if resp["transport"] != "ok":
                return run.finish(recs, {"backend_response": resp}, [], {"sent": sent_note},
                                  [f"transport {resp['transport']}"], state="failed",
                                  extra={"reason": resp["transport"], "producer_backend": backend_identity("bl", resp.get("model"))})
            state, reason, parsed = validate_answer(resp["content"], resp["finish_reason"], args.expect)
            ident = backend_identity("bl", resp.get("model"))
            measurements = {"elapsed_ms": resp["elapsed_ms"], "usage": resp["usage"], "finish_reason": resp["finish_reason"],
                            "request_id": resp["request_id"], "cost": "unknown (usage returned; price not settled here)"}
            raw = resp["content"]
        else:
            state, reason, parsed = validate_answer(answer_text, "host", args.expect)
            ident = backend_identity("host", args.model)
            measurements = {"usage": "unknown (host session)", "cost": "unknown"}
            raw = answer_text
            sent_note.append("host read the original files directly")
        run.add("raw-answer", "answer.txt", data=(raw or "").encode("utf-8"), relation="verbatim backend answer")
        extra = {"answer": parsed if state == "succeeded" else None, "reason": reason, "producer_backend": ident}
        return run.finish(recs, measurements, [], {"sent": sent_note, "question": args.prompt},
                          ["model answer is a candidate reading, not fact certification"], state=state, extra=extra)


def backend_identity(kind: str, model):
    ident = {"adapter": "05_scripts/atoms/vision_backend.py", "adapter_version": ADAPTER_VERSION, "backend": kind, "model": model}
    if kind == "bl":
        try:
            ident["bl_version"] = subprocess.run([shutil.which("bl"), "--version"], capture_output=True, text=True, timeout=30).stdout.strip()
        except Exception:
            ident["bl_version"] = "unknown"
    return ident


REMBG_SNIPPET = r"""
import sys, json, time
from rembg import remove, new_session
from PIL import Image
src, dst, model = sys.argv[1], sys.argv[2], sys.argv[3]
t = time.time()
session = new_session(model)
out = remove(Image.open(src).convert("RGBA"), session=session)
out.save(dst, "PNG")
import rembg, onnxruntime
print(json.dumps({"elapsed_ms": int((time.time()-t)*1000), "rembg": getattr(rembg, "__version__", "?"),
                  "onnxruntime": onnxruntime.__version__, "model": model}))
"""


def cmd_matte(args, limits):
    img, rec = io.load_image(args.image, "input", limits)
    py = os.environ.get("BABATA_REMBG_PYTHON")
    if not py or not Path(py).is_file():
        raise io.AtomError("unavailable", "complex matting backend not configured (BABATA_REMBG_PYTHON); report blocked, do not substitute keying")
    req = {"backend": "rembg", "model": args.model, "input_sha256": rec["sha256"]}
    run = io.Run(args.out, "matte", req)
    if (prev := run.existing()):
        return prev
    with run:
        src = run.add("normalized-input", "input.png", img, relation="RGBA copy of input after explicit probe")
        tmp = run.dir / "cutout.tmp.png"
        proc = subprocess.run([py, "-c", REMBG_SNIPPET, str(src), str(tmp), args.model], capture_output=True, text=True,
                              encoding="utf-8", errors="replace", timeout=args.timeout)
        if proc.returncode != 0 or not tmp.is_file():
            tmp.unlink(missing_ok=True)
            return run.finish([rec], {"stderr_tail": proc.stderr[-600:]}, [], {}, [], state="failed",
                              extra={"reason": "backend-error", "producer_backend": {"backend": "rembg", "model": args.model}})
        info = json.loads(proc.stdout.strip().splitlines()[-1])
        out = np.asarray(Image.open(tmp).convert("RGBA")).copy()
        tmp.unlink()
        if out.shape[:2] != img.shape[:2]:
            raise io.AtomError("integrity", "backend returned a different size")
        run.add("cutout", "cutout.png", out, relation="rembg alpha matte")
        meas, finds = io.measure_alpha(out, args.profile)
        meas["backend"] = info
        return run.finish([rec], meas, finds, {"region": "full image"},
                          ["learned matting: quality varies by material; check on multiple backgrounds"],
                          extra={"producer_backend": {"adapter_version": ADAPTER_VERSION, "backend": "rembg", **info}})


def cmd_probe(args, limits):
    out = {}
    bl = shutil.which("bl")
    out["bl"] = {"installed": bool(bl), "configured": bool(os.environ.get("DASHSCOPE_API_KEY")) if bl else False,
                 "probed": "not-run (spends a call; use ask on a known image)", "real_tested": "see evidence receipts"}
    py = os.environ.get("BABATA_REMBG_PYTHON")
    out["rembg"] = {"configured": bool(py), "installed": False}
    if py and Path(py).is_file():
        proc = subprocess.run([py, "-c", "import rembg, onnxruntime; print('ok')"], capture_output=True, text=True, timeout=120)
        out["rembg"]["installed"] = proc.returncode == 0
    out["comfyui"] = {"configured": bool(os.environ.get("COMFYUI_URL")), "note": "no endpoint scanning; set COMFYUI_URL explicitly"}
    out["host"] = {"note": "available only when the running host model can actually read images"}
    return {"schema": "babata.vision-backend-probe/v1", "backends": out}


def main(argv=None):
    p = argparse.ArgumentParser(prog="vision_backend.py", description=__doc__)
    p.add_argument("--max-pixels", type=int, default=io.DEFAULT_MAX_PIXELS)
    sub = p.add_subparsers(dest="command", required=True)
    s = sub.add_parser("ask")
    s.add_argument("--image", action="append", required=True); s.add_argument("--prompt", required=True)
    s.add_argument("--backend", choices=["bl", "host"], required=True); s.add_argument("--model")
    s.add_argument("--expect", choices=["text", "json"], default="text"); s.add_argument("--answer-file")
    s.add_argument("--max-side", type=int, default=2048); s.add_argument("--timeout", type=int, default=120)
    s.add_argument("--out", required=True); s.set_defaults(fn=cmd_ask)
    s = sub.add_parser("matte")
    s.add_argument("--image", required=True); s.add_argument("--model", default="isnet-general-use")
    s.add_argument("--profile", choices=list(io.PROFILES), default="cutout"); s.add_argument("--timeout", type=int, default=600)
    s.add_argument("--out", required=True); s.set_defaults(fn=cmd_matte)
    s = sub.add_parser("probe"); s.set_defaults(fn=cmd_probe)
    args = p.parse_args(argv)
    limits = {"max_pixels": args.max_pixels, "max_bytes": io.DEFAULT_MAX_BYTES}
    try:
        result = args.fn(args, limits)
    except io.AtomError as exc:
        sys.stderr.write(json.dumps({"state": "failed", "reason": exc.reason, "message": str(exc)}, ensure_ascii=False) + "\n")
        return 4 if exc.reason == "unavailable" else 2
    sys.stdout.reconfigure(encoding="utf-8")
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0 if result.get("state", "succeeded") == "succeeded" else 3


if __name__ == "__main__":
    sys.exit(main())
