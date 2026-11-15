#!/usr/bin/env python3
"""Copy the canonical atom scripts into each skill's scripts/ directory, or check they match.

A skill directory is the release unit: copied anywhere on its own, it must run without a
Babata checkout. The canonical source stays in 05_scripts/atoms (tests, CI and the Rust PNG
renderer use it); this script vendors exact copies into 02_skills/<skill>/scripts/.

    python 05_scripts/sync-skill-bundles.py          # write copies
    python 05_scripts/sync-skill-bundles.py --check  # fail if any copy differs or is missing
"""
from __future__ import annotations

import argparse
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
ATOMS = REPO / "05_scripts" / "atoms"
BUNDLES = {
    "babata-render": ["render.mjs", "common.mjs", "package.json", "package-lock.json",
                      "image_ops.py", "vision_backend.py", "rembg_matte.py"],
    "babata-vision": ["image_ops.py", "vision_backend.py", "rembg_matte.py"],
    "babata-research": ["research.mjs", "common.mjs"],
}


def normalized(path: Path) -> bytes:
    return path.read_bytes().replace(b"\r\n", b"\n")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    problems = []
    for skill, files in BUNDLES.items():
        target = REPO / "02_skills" / skill / "scripts"
        expected = set(files)
        present = {p.name for p in target.iterdir() if p.is_file()} if target.is_dir() else set()
        if args.check:
            for extra in sorted(present - expected):
                problems.append(f"{skill}/scripts/{extra}: not part of the bundle")
        for name in files:
            source, copy = ATOMS / name, target / name
            if args.check:
                if not copy.is_file():
                    problems.append(f"{skill}/scripts/{name}: missing")
                elif normalized(copy) != normalized(source):
                    problems.append(f"{skill}/scripts/{name}: differs from 05_scripts/atoms/{name}")
            else:
                target.mkdir(parents=True, exist_ok=True)
                copy.write_bytes(source.read_bytes())
        if not args.check:
            for extra in present - expected:
                (target / extra).unlink()
    if problems:
        print("Skill bundles out of sync; run python 05_scripts/sync-skill-bundles.py", file=sys.stderr)
        for line in problems:
            print("  " + line, file=sys.stderr)
        return 1
    print(("Skill bundles match" if args.check else "Skill bundles written") + f": {', '.join(BUNDLES)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
