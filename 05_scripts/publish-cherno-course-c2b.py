#!/usr/bin/env python3
"""Atomically publish verified Cherno C2B packages to their unique Obsidian lives."""

from __future__ import annotations

import argparse
import hashlib
import importlib.util
import json
import os
import shutil
import sys
import urllib.parse
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any


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


def tree_fingerprint(root: Path) -> list[dict[str, Any]]:
    return [
        {
            "path": path.relative_to(root).as_posix(),
            "bytes": path.stat().st_size,
            "sha256": sha256_file(path),
        }
        for path in sorted((value for value in root.rglob("*") if value.is_file()), key=lambda value: value.as_posix().lower())
    ]


def load_checker(script_path: Path):
    spec = importlib.util.spec_from_file_location("cherno_c2b_builder", script_path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"Cannot load checker: {script_path}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module.verify_package


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--stage",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-c2b-full-20260822-v1",
    )
    parser.add_argument(
        "--archive-root",
        default=r"D:\BabataData\04_runtime\staging\model-workspaces\cherno-course-obsidian-live-archive",
    )
    parser.add_argument(
        "--checker",
        default=r"C:\Users\Aiano\Babata\05_scripts\build-cherno-course-c2b.py",
    )
    args = parser.parse_args()

    stage = Path(args.stage).resolve()
    archive_root = Path(args.archive_root).resolve()
    checker_path = Path(args.checker).resolve()
    if not stage.is_dir() or not checker_path.is_file():
        raise RuntimeError("C2B stage/checker is missing")
    verify_package = load_checker(checker_path)
    course_dirs = sorted(path for path in stage.iterdir() if path.is_dir() and (path / "package-manifest.json").is_file())
    if len(course_dirs) != 3:
        raise RuntimeError(f"Expected exactly three verified course packages, found {len(course_dirs)}")

    prepared: list[dict[str, Any]] = []
    for course_dir in course_dirs:
        package = course_dir / "package"
        manifest_path = course_dir / "package-manifest.json"
        check = verify_package(package, manifest_path)
        manifest = read_json(manifest_path)
        live = Path(manifest["publication"]["path"]).resolve()
        vault_name = str(manifest["publication"]["vault"])
        live_file = str(manifest["publication"]["file"])
        if live_file != f"Babata/Cherno/{manifest['short_name']}/index.md":
            raise RuntimeError(f"Unexpected live file contract: {live_file}")
        cursor = live
        vault_root = None
        while cursor.parent != cursor:
            if cursor.name == vault_name:
                vault_root = cursor
                break
            cursor = cursor.parent
        if vault_root is None:
            raise RuntimeError(f"Live target is not below named vault: {live}")
        expected_index = vault_root / Path(live_file.replace("/", os.sep))
        if expected_index.resolve() != (live / "index.md").resolve():
            raise RuntimeError(f"Vault-relative live file does not match live path: {live_file}")
        prepared.append(
            {
                "course_dir": course_dir,
                "package": package,
                "manifest_path": manifest_path,
                "manifest": manifest,
                "check": check,
                "live": live,
                "vault_name": vault_name,
                "live_file": live_file,
            }
        )

    receipts: list[dict[str, Any]] = []
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    for row in prepared:
        package: Path = row["package"]
        live: Path = row["live"]
        manifest = row["manifest"]
        parent = live.parent.resolve()
        parent.mkdir(parents=True, exist_ok=True)
        token = uuid.uuid4().hex
        temporary = parent / f".{live.name}.publish-{token}"
        old_sibling = parent / f".{live.name}.old-{token}"
        for candidate in (temporary, old_sibling):
            resolved = candidate.resolve()
            if resolved.parent != parent or not resolved.name.startswith(f".{live.name}."):
                raise RuntimeError(f"Unsafe publication working path: {resolved}")
            if resolved.exists():
                raise RuntimeError(f"Publication working path already exists: {resolved}")

        archive = None
        if live.exists():
            archive = archive_root / manifest["course_key"] / timestamp
            if archive.exists():
                raise RuntimeError(f"Archive target already exists: {archive}")
            archive.parent.mkdir(parents=True, exist_ok=True)
            shutil.copytree(live, archive)
            if tree_fingerprint(archive) != tree_fingerprint(live):
                raise RuntimeError(f"Predecessor archive read-back failed: {live}")

        shutil.copytree(package, temporary)
        if tree_fingerprint(temporary) != tree_fingerprint(package):
            shutil.rmtree(temporary)
            raise RuntimeError(f"Publication copy verification failed: {manifest['course_key']}")

        replaced = False
        try:
            if live.exists():
                os.replace(live, old_sibling)
                replaced = True
            os.replace(temporary, live)
            if tree_fingerprint(live) != tree_fingerprint(package):
                raise RuntimeError(f"Live read-back failed: {live}")
        except Exception:
            if live.exists() and live != old_sibling:
                failed = parent / f".{live.name}.failed-{token}"
                os.replace(live, failed)
            if replaced and old_sibling.exists():
                os.replace(old_sibling, live)
            raise
        if old_sibling.exists():
            if archive is None or tree_fingerprint(old_sibling) != tree_fingerprint(archive):
                raise RuntimeError(f"Old live differs from its archive: {old_sibling}")
            shutil.rmtree(old_sibling)

        uri = "obsidian://open?vault=" + urllib.parse.quote(row["vault_name"], safe="") + "&file=" + urllib.parse.quote(
            row["live_file"], safe=""
        )
        receipt = {
            "schema": "babata.course-c2b-publication/v1",
            "status": "published_pending_user_acceptance",
            "published_at": utc_now(),
            "course_key": manifest["course_key"],
            "short_name": manifest["short_name"],
            "profile": manifest["profile"],
            "package_manifest": str(row["manifest_path"]),
            "package_manifest_sha256": sha256_file(row["manifest_path"]),
            "live_path": str(live),
            "live_file": row["live_file"],
            "vault": row["vault_name"],
            "obsidian_uri": uri,
            "live_files": tree_fingerprint(live),
            "predecessor_archive": str(archive) if archive else None,
            "generic_outputs_obsidian_enabled": False,
        }
        receipt_path = row["course_dir"] / "publication-receipt.json"
        write_json(receipt_path, receipt)
        receipts.append({**receipt, "receipt": str(receipt_path)})

    aggregate = {
        "schema": "babata.cherno-course-publication-round/v1",
        "status": "published_pending_user_acceptance",
        "published_at": utc_now(),
        "courses": receipts,
    }
    aggregate_path = stage / "publication-round.json"
    write_json(aggregate_path, aggregate)
    print(json.dumps({"courses": receipts, "round": str(aggregate_path)}, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: {exc}", file=sys.stderr)
        raise
