#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11,<3.15"
# dependencies = ["pillow>=11,<14", "numpy>=2,<3"]
# ///
"""Babata vision/render backend adapters (narrow, replaceable).

ask    : image(s) + question -> answer, via `bl` (Qwen-VL through Bailian CLI) or `host`
         (records the host agent's own answer file; the host must actually have read the image).
matte  : local background segmentation through uv and the bundled rembg_matte.py.
probe  : installed / configured / probed state of each backend, without spending calls.

External tools are discovered on PATH; models are explicit --model arguments.
  U2NET_HOME           rembg's own override; otherwise rembg uses ~/.u2net
  COMFYUI_URL          reported by probe only; no endpoint scanning

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

ADAPTER_VERSION = "1.2.0"
DEFAULT_VISION_MODEL = "qwen3-vl-plus"
DEFAULT_REMBG_MODEL = "isnet-general-use"


def bl_executable():
    return shutil.which("bl")


def vision_model(explicit):
    return explicit or DEFAULT_VISION_MODEL


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


def run_bounded(cmd, timeout):
    """Run a command; on timeout kill the WHOLE process tree (Windows .cmd shims spawn children)."""
    flags = subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8",
                            errors="replace", creationflags=flags, start_new_session=os.name != "nt")
    try:
        out, err = proc.communicate(timeout=timeout)
        return proc.returncode, out, err, False
    except subprocess.TimeoutExpired:
        if os.name == "nt":
            subprocess.run(["taskkill", "/T", "/F", "/PID", str(proc.pid)], capture_output=True)
        else:
            import signal
            os.killpg(proc.pid, signal.SIGKILL)
        try:
            out, err = proc.communicate(timeout=10)
        except subprocess.TimeoutExpired:
            out, err = "", ""
        return None, out, err, True


def call_bl(image: str, prompt: str, model: str, timeout: int):
    exe = bl_executable()
    if not exe:
        raise io.AtomError("unavailable", "bl not found on PATH")
    cmd = [exe, "vision", "describe", "--image", image, "--prompt", prompt, "--model", model,
           "--output", "json", "--timeout", str(timeout), "--quiet"]
    started = time.time()
    code, stdout, stderr, timed_out = run_bounded(cmd, timeout + 15)
    elapsed = int((time.time() - started) * 1000)
    if timed_out:
        return {"transport": "timeout", "elapsed_ms": elapsed,
                "note": "local wait stopped and process tree killed; remote state unknown, do not blindly resubmit"}
    try:
        data = json.loads(stdout)
    except json.JSONDecodeError:
        return {"transport": "bad-output", "exit": code, "elapsed_ms": elapsed,
                "stderr_tail": stderr[-400:], "stdout_head": stdout[:400]}
    choice = (data.get("choices") or [{}])[0]
    msg = choice.get("message") or {}
    return {"transport": "ok" if code == 0 else f"exit-{code}", "elapsed_ms": elapsed,
            "content": msg.get("content"), "finish_reason": choice.get("finish_reason"), "usage": data.get("usage"),
            "model": data.get("model"), "request_id": data.get("id"),
            "error": data.get("error") if code else None}


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
    req = {"backend": args.backend, "model": vision_model(args.model) if args.backend == "bl" else args.model, "images": [r["sha256"] for r in recs], "prompt": args.prompt,
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
            model = vision_model(args.model)
            ident = backend_identity("bl", model)
            resp = call_bl(str(sent), args.prompt, model, args.timeout)
            if resp.get("model"):
                ident["model"] = resp["model"]
            if resp["transport"] != "ok":
                return run.finish(recs, {"backend_response": resp}, [], {"sent": sent_note},
                                  [f"transport {resp['transport']}"], state="failed",
                                  extra={"reason": resp["transport"], "producer_backend": ident})
            state, reason, parsed = validate_answer(resp["content"], resp["finish_reason"], args.expect)
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
        exe = bl_executable()
        code, out, _, timed_out = run_bounded([exe, "--version"], 20) if exe else (None, "", "", False)
        ident["bl_version"] = out.strip() if code == 0 and not timed_out else "unknown"
    return ident


def cmd_matte(args, limits):
    img, rec = io.load_image(args.image, "input", limits)
    args.model = args.model or DEFAULT_REMBG_MODEL
    req = {"backend": "rembg", "model": args.model, "input_sha256": rec["sha256"]}
    run = io.Run(args.out, "matte", req)
    if (prev := run.existing()):
        return prev
    uv = shutil.which("uv")
    helper = Path(__file__).resolve().with_name("rembg_matte.py")
    if not uv or not helper.is_file():
        raise io.AtomError("unavailable", "rembg needs uv on PATH and bundled rembg_matte.py; do not substitute keying")
    with run:
        src = run.add("normalized-input", "input.png", img, relation="RGBA copy of input after explicit probe")
        tmp = run.dir / "cutout.tmp.png"
        code, stdout, stderr, timed_out = run_bounded(
            [uv, "run", "--no-project", "--script", str(helper), str(src), str(tmp), "--model", args.model], args.timeout)
        if timed_out or code != 0 or not tmp.is_file():
            tmp.unlink(missing_ok=True)
            return run.finish([rec], {"stderr_tail": stderr[-600:]}, [], {}, [], state="failed",
                              extra={"reason": "timeout" if timed_out else "backend-error",
                                     "producer_backend": {"backend": "rembg", "model": args.model}})
        try:
            info = json.loads(stdout.strip().splitlines()[-1])
            if not isinstance(info, dict) or info.get("model") != args.model:
                raise ValueError("invalid rembg identity")
        except (ValueError, IndexError) as exc:
            raise io.AtomError("integrity", "rembg returned invalid metadata") from exc
        out, _ = io.load_image(str(tmp), "backend-output", limits)
        tmp.unlink()
        if out.shape[:2] != img.shape[:2]:
            raise io.AtomError("integrity", "backend returned a different size")
        run.add("cutout", "cutout.png", out, relation="rembg foreground segmentation")
        meas, finds = io.measure_alpha(out, args.profile)
        meas["backend"] = info
        return run.finish([rec], meas, finds, {"region": "full image"},
                          ["foreground segmentation is not true alpha matting; check on multiple backgrounds"],
                          extra={"producer_backend": {"adapter_version": ADAPTER_VERSION, "backend": "rembg", **info}})


def cmd_probe(args, limits):
    out = {}
    bl = bl_executable()
    out["bl"] = {"installed": bool(bl), "executable": bl, "default_model": vision_model(None),
                 "credentials": "resolved by bl itself; not inspected here",
                 "probed": "not-run (spends a call; use ask on a known image)"}
    uv = shutil.which("uv")
    weights = Path(os.environ.get("U2NET_HOME") or Path.home() / ".u2net").expanduser()
    out["rembg"] = {"launcher": uv, "launcher_available": bool(uv), "model": DEFAULT_REMBG_MODEL,
                    "weights_dir": str(weights), "default_weights_present": (weights / (DEFAULT_REMBG_MODEL + ".onnx")).is_file(),
                    "dependencies": "managed by uv from rembg_matte.py inline metadata",
                    "probed": "not-run; probe does not install dependencies or load weights"}
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
    s.add_argument("--image", required=True); s.add_argument("--model")
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
