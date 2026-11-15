#!/usr/bin/env python3
# /// script
# requires-python = ">=3.11,<3.14"
# dependencies = ["rembg[cpu]>=2.0.67,<3"]
# ///
"""Local rembg worker; uv owns the environment, rembg owns its weight location."""
import argparse
import importlib.metadata
import json
import time

from PIL import Image
from rembg import new_session, remove


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input")
    parser.add_argument("output")
    parser.add_argument("--model", default="isnet-general-use")
    args = parser.parse_args()
    started = time.monotonic()
    session = new_session(args.model, providers=["CPUExecutionProvider"])
    with Image.open(args.input) as image:
        result = remove(image.convert("RGBA"), session=session)
    result.save(args.output, "PNG")
    print(json.dumps({"elapsed_ms": int((time.monotonic() - started) * 1000),
                      "rembg": importlib.metadata.version("rembg"),
                      "onnxruntime": importlib.metadata.version("onnxruntime"),
                      "model": args.model, "provider": "CPUExecutionProvider"}))


if __name__ == "__main__":
    main()
