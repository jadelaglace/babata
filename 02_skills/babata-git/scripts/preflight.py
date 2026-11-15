# /// script
# requires-python = ">=3.11"
# dependencies = []
# ///
"""Bounded, read-only Git index/history preflight. JSON never includes matched bytes."""
from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

VERSION = "1.0.0"
SECRET_PATTERNS = (
    ("private-key", re.compile(rb"-----BEGIN (?:[A-Z0-9]+ )?PRIVATE KEY-----")),
    ("github-token", re.compile(rb"\b(?:gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{20,})")),
    ("aws-key", re.compile(rb"\bAKIA[A-Z0-9]{16}\b")),
    ("url-credentials", re.compile(rb"https?://[^\s/:@]+:[^\s/@]+@")),
    ("assigned-secret", re.compile(
        rb"(?i)\b(?:password|passwd|api[_-]?key|access[_-]?token|secret)\b"
        rb"[\"']?\s*[:=]\s*[\"']?[^\s\"';,}{]{12,}")),
)
CONVENTIONAL = re.compile(r"^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert)(\([^)\r\n]+\))?!?: .+")


class ProbeError(Exception):
    """Safe error code; never expose subprocess stderr (it may contain secrets)."""


class SafeParser(argparse.ArgumentParser):
    def error(self, message: str) -> None:
        print(json.dumps({"schema": "babata.git-preflight/v1", "error": "invalid-arguments"}))
        raise SystemExit(2)


def git(repo: Path, *args: str, stdin: bytes | None = None, optional: bool = False) -> bytes:
    try:
        result = subprocess.run(
            ["git", "--no-pager", "--no-optional-locks", "-c", "core.fsmonitor=false",
             "-C", str(repo), *args], input=stdin, capture_output=True, timeout=30, check=False,
        )
    except FileNotFoundError as exc:
        raise ProbeError("git-unavailable") from exc
    except (subprocess.TimeoutExpired, OSError) as exc:
        raise ProbeError("git-execution-failed") from exc
    if result.returncode and not optional:
        raise ProbeError("git-command-failed")
    return result.stdout if result.returncode == 0 else b""


def decode(value: bytes) -> str:
    return value.decode("utf-8", "replace").strip()


def resolve(repo: Path, ref: str) -> str:
    if not ref or ref.startswith("-"):
        raise ProbeError("invalid-ref")
    value = git(repo, "rev-parse", "--verify", "--end-of-options", ref + "^{commit}", optional=True)
    if not value:
        raise ProbeError("unresolved-ref")
    return decode(value)


def inspect(args: argparse.Namespace) -> dict:
    repo = Path(args.repo).resolve()
    git(repo, "rev-parse", "--show-toplevel")
    head = decode(git(repo, "rev-parse", "--verify", "HEAD", optional=True)) or None
    branch = decode(git(repo, "symbolic-ref", "--quiet", "--short", "HEAD", optional=True)) or None
    findings: list[dict] = []

    def finding(code: str, where: str = "", severity: str = "error") -> None:
        findings.append({"code": code, "where": where, "severity": severity})

    status = git(repo, "status", "--porcelain=v1", "-z", "--untracked-files=normal")
    # Refuse a mixed observation if another writer changes HEAD/index during scanning.
    index_before = git(repo, "ls-files", "--stage", "-z")
    # Do not return names or diffs: filenames can themselves contain confidential text.
    worktrees = git(repo, "worktree", "list", "--porcelain")
    if args.branch_pattern and (branch is None or not re.fullmatch(args.branch_pattern, branch)):
        finding("branch-policy")

    objects: dict[str, set[str]] = {}
    if args.scope in ("staged", "both"):
        if git(repo, "ls-files", "--unmerged", "-z"):
            finding("unmerged-index")
        names = git(repo, "diff", "--cached", "--no-ext-diff", "--no-textconv", "--name-only",
                    "--diff-filter=ACMRT", "-z").split(b"\0")
        selected = {name for name in names if name}
        for record in git(repo, "ls-files", "--stage", "-z").split(b"\0"):
            if not record:
                continue
            header, name = record.split(b"\t", 1)
            mode, oid, stage = header.split()
            if name in selected and stage == b"0" and mode != b"160000":
                objects.setdefault(decode(oid), set()).add("index")

    base_oid = None
    commit_count = 0
    if args.scope in ("history", "both"):
        if head is None:
            raise ProbeError("history-needs-head")
        ref = args.base or decode(git(repo, "symbolic-ref", "--quiet", "refs/remotes/origin/HEAD", optional=True))
        if not ref:
            raise ProbeError("explicit-base-required")
        base_oid = resolve(repo, ref)
        if git(repo, "rev-parse", "--is-shallow-repository").strip() == b"true":
            finding("shallow-history-incomplete")
        comparison = base_oid + ".." + head
        commits = git(repo, "rev-list", "--max-count=" + str(args.max_commits + 1), comparison).splitlines()
        commit_count = len(commits)
        if commit_count > args.max_commits:
            finding("commit-limit-incomplete")
        else:
            if args.conventional_commits:
                for oid in commits:
                    subject = decode(git(repo, "show", "-s", "--format=%s", decode(oid)))
                    if not CONVENTIONAL.fullmatch(subject):
                        finding("commit-subject-policy", decode(oid))
            for oid in git(repo, "rev-list", "--objects", "--no-object-names", comparison).splitlines():
                objects.setdefault(decode(oid), set()).add("history")

    if len(objects) > args.max_objects:
        finding("object-limit-incomplete")
    scanned = 0
    scanned_bytes = 0
    selected_oids = list(objects)[:args.max_objects]
    if selected_oids:
        rows = git(repo, "cat-file", "--batch-check=%(objectname) %(objecttype) %(objectsize)",
                   stdin=("\n".join(selected_oids) + "\n").encode()).splitlines()
        for row in rows:
            parts = row.split()
            if len(parts) != 3:
                raise ProbeError("object-read-failed")
            oid, kind, raw_size = parts
            if kind != b"blob":
                continue
            size = int(raw_size)
            if size > args.max_blob_bytes:
                finding("large-blob", decode(oid))
            if size > args.scan_bytes or scanned_bytes + size > args.total_bytes:
                finding("scan-budget-incomplete", decode(oid))
                continue
            content = git(repo, "cat-file", "blob", decode(oid))
            if len(content) != size:
                raise ProbeError("object-size-changed")
            scanned += 1
            scanned_bytes += size
            for code, pattern in SECRET_PATTERNS:
                if pattern.search(content):
                    finding(code, decode(oid))
    if (decode(git(repo, "rev-parse", "--verify", "HEAD", optional=True)) or None) != head:
        finding("head-changed-during-scan")
    if git(repo, "ls-files", "--stage", "-z") != index_before:
        finding("index-changed-during-scan")
    return {
        "schema": "babata.git-preflight/v1", "tool_version": VERSION,
        "scope": args.scope, "head_oid": head, "base_oid": base_oid,
        "detached": branch is None, "dirty": bool(status),
        "worktree_count": sum(line.startswith(b"worktree ") for line in worktrees.splitlines()),
        "commits_observed": commit_count, "objects_selected": len(objects),
        "blobs_scanned": scanned, "bytes_scanned": scanned_bytes,
        "findings": findings, "verdict": "review-required" if findings else "no-findings-in-scope",
        "limitations": ["heuristic-not-security-certification", "no-network-fetch",
                        "no-unstaged-untracked", "no-other-refs-or-base-history",
                        "no-lfs-submodule-or-archive-contents", "no-commit-message-secret-scan"],
    }


def main(argv: list[str] | None = None) -> int:
    parser = SafeParser(description=__doc__)
    parser.add_argument("--repo", default=".")
    parser.add_argument("--scope", choices=("staged", "history", "both"), default="both")
    parser.add_argument("--base")
    parser.add_argument("--max-blob-bytes", type=int, default=5 * 1024 * 1024)
    parser.add_argument("--scan-bytes", type=int, default=10 * 1024 * 1024)
    parser.add_argument("--total-bytes", type=int, default=64 * 1024 * 1024)
    parser.add_argument("--max-objects", type=int, default=20000)
    parser.add_argument("--max-commits", type=int, default=1000)
    parser.add_argument("--branch-pattern")
    parser.add_argument("--conventional-commits", action="store_true")
    args = parser.parse_args(argv)
    try:
        if min(args.max_blob_bytes, args.scan_bytes, args.total_bytes, args.max_objects, args.max_commits) < 1:
            raise ProbeError("positive-limits-required")
        if args.branch_pattern:
            re.compile(args.branch_pattern)
        report = inspect(args)
    except (ProbeError, OSError, ValueError, re.error) as exc:
        code = str(exc) if isinstance(exc, ProbeError) else "invalid-input-or-read-failed"
        print(json.dumps({"schema": "babata.git-preflight/v1", "error": code}))
        return 2
    print(json.dumps(report, ensure_ascii=True, indent=2))
    return 1 if report["findings"] else 0


if __name__ == "__main__":
    raise SystemExit(main())
