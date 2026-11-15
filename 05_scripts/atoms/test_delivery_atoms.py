"""Real temporary Git repositories and local HTTP services; no live remotes or servers."""
from __future__ import annotations

import hashlib
import http.server
import json
import os
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
import unittest
from unittest import mock
import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SKILLS = ROOT / "02_skills"
TOKEN = "ghp_" + "A9b8C7d6E5f4G3h2I1j0K9l8M7n6O5p4"
COOKIE = "session=" + "synthetic-sensitive-value-2390"


def execute(script: Path, *args: str, cwd: Path) -> tuple[int, dict, str]:
    result = subprocess.run([sys.executable, str(script), *args], cwd=cwd,
                            capture_output=True, text=True, encoding="utf-8", timeout=45)
    return result.returncode, json.loads(result.stdout), result.stdout + result.stderr


def load_script(path: Path):
    spec = importlib.util.spec_from_file_location("delivery_atom_under_test", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class PackageCases(unittest.TestCase):
    def test_relative_references_stay_within_each_release_unit(self):
        for name in ("babata-git", "babata-ops"):
            with tempfile.TemporaryDirectory(prefix="babata-skill-package-") as temp:
                target = Path(temp) / name
                shutil.copytree(SKILLS / name, target)
                for document in target.rglob("*.md"):
                    for link in re.findall(r"\]\(([^)]+)\)", document.read_text(encoding="utf-8")):
                        if link.startswith("https://"):
                            continue
                        resolved = (document.parent / link.split("#", 1)[0]).resolve()
                        self.assertTrue(resolved.is_relative_to(target.resolve()), "outside package reference")
                        self.assertTrue(resolved.is_file(), "missing packaged reference")

    def test_tls_validation_failure_is_not_success(self):
        module = load_script(SKILLS / "babata-ops" / "scripts" / "probe.py")
        with mock.patch.object(module.http.client.HTTPSConnection, "request", side_effect=module.ssl.SSLError()):
            report, code = module.probe("https://example.invalid/health")
        self.assertEqual(code, 4)
        self.assertEqual(report["error"], "tls-failed")

    def test_concurrent_head_change_requires_review(self):
        module = load_script(SKILLS / "babata-git" / "scripts" / "preflight.py")
        count = 0

        def read_only_git(repo, *args, **kwargs):
            nonlocal count
            if args == ("rev-parse", "--verify", "HEAD"):
                count += 1
                return ("a" if count == 1 else "b").encode() * 40
            if args[:2] == ("symbolic-ref", "--quiet"):
                return b"codex/test"
            return b""

        import argparse
        options = argparse.Namespace(repo=".", scope="staged", base=None, branch_pattern=None,
            conventional_commits=False, max_objects=10, max_commits=10,
            max_blob_bytes=100, scan_bytes=100, total_bytes=100)
        with mock.patch.object(module, "git", side_effect=read_only_git):
            report = module.inspect(options)
        self.assertEqual(report["verdict"], "review-required")
        self.assertIn("head-changed-during-scan", [x["code"] for x in report["findings"]])


class GitCases(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="babata-git-tests-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.skill = self.root / "standalone" / "babata-git"
        shutil.copytree(SKILLS / "babata-git", self.skill)
        self.script = self.skill / "scripts" / "preflight.py"
        self.repo = self.root / "repo with space"
        self.repo.mkdir()
        self.g("init", "-b", "main")
        self.g("config", "user.name", "Synthetic User")
        self.g("config", "user.email", "synthetic@example.invalid")
        self.g("config", "commit.gpgsign", "false")
        self.g("config", "core.hooksPath", str(self.root / "no-hooks"))
        self.g("commit", "--allow-empty", "-m", "initial")
        self.base = self.g("rev-parse", "HEAD").strip()
        self.g("switch", "-c", "codex/topic")

    def g(self, *args: str, repo: Path | None = None) -> str:
        result = subprocess.run(["git", "-C", str(repo or self.repo), *args],
                                capture_output=True, text=True, encoding="utf-8", timeout=20)
        self.assertEqual(result.returncode, 0, "Git fixture failed")
        return result.stdout

    def run_probe(self, *args: str, repo: Path | None = None):
        return execute(self.script, "--repo", str(repo or self.repo), *args, cwd=self.root)

    def stage(self, name: str, content: str | bytes):
        path = self.repo / name
        path.write_bytes(content if isinstance(content, bytes) else content.encode())
        self.g("add", "--", name)

    def test_clean_independent_package_and_default_branch_policy(self):
        code, report, _ = self.run_probe("--base", self.base)
        self.assertEqual(code, 0)
        self.assertEqual(report["verdict"], "no-findings-in-scope")
        self.assertEqual(report["base_oid"], self.base)

    def test_missing_base_does_not_silently_fall_back(self):
        code, report, _ = self.run_probe()
        self.assertEqual(code, 2)
        self.assertEqual(report["error"], "explicit-base-required")

    def test_invalid_base_and_option_are_safe(self):
        for ref in ("missing-" + TOKEN, "--help"):
            with self.subTest(ref=ref[:7]):
                code, _, raw = self.run_probe("--base=" + ref)
                self.assertEqual(code, 2)
                self.assertNotIn(TOKEN, raw)

    def test_staged_unicode_and_secret_redacted(self):
        self.stage("中文 file.txt", TOKEN)
        code, report, raw = self.run_probe("--scope", "staged")
        self.assertEqual(code, 1)
        self.assertIn("github-token", [x["code"] for x in report["findings"]])
        self.assertNotIn(TOKEN, raw)
        self.assertNotIn("中文", raw)

    def test_historical_secret_deleted_at_head_is_still_detected(self):
        self.stage("removed.txt", TOKEN)
        self.g("commit", "-m", "temporary")
        self.g("rm", "removed.txt")
        self.g("commit", "-m", "remove")
        code, report, raw = self.run_probe("--scope", "history", "--base", self.base)
        self.assertEqual(code, 1)
        self.assertEqual(report["commits_observed"], 2)
        self.assertIn("github-token", [x["code"] for x in report["findings"]])
        self.assertNotIn(TOKEN, raw)

    def test_inspects_index_not_newer_worktree_content(self):
        self.stage("selected.txt", TOKEN)
        (self.repo / "selected.txt").write_text("now benign", encoding="utf-8")
        code, report, _ = self.run_probe("--scope", "staged")
        self.assertEqual(code, 1)
        self.assertIn("github-token", [x["code"] for x in report["findings"]])

    def test_large_staged_blob_uses_index_size(self):
        self.stage("large.txt", b"x" * 40)
        (self.repo / "large.txt").write_bytes(b"x")
        code, report, _ = self.run_probe("--scope", "staged", "--max-blob-bytes", "30")
        self.assertEqual(code, 1)
        self.assertIn("large-blob", [x["code"] for x in report["findings"]])

    def test_budget_and_commit_limits_are_not_pass(self):
        self.stage("data.txt", b"x" * 40)
        code, report, _ = self.run_probe("--scope", "staged", "--scan-bytes", "10")
        self.assertEqual(code, 1)
        self.assertIn("scan-budget-incomplete", [x["code"] for x in report["findings"]])
        self.g("commit", "-m", "one")
        self.g("commit", "--allow-empty", "-m", "two")
        code, report, _ = self.run_probe("--base", self.base, "--max-commits", "1")
        self.assertEqual(code, 1)
        self.assertIn("commit-limit-incomplete", [x["code"] for x in report["findings"]])

    def test_policy_is_opt_in(self):
        self.g("commit", "--allow-empty", "-m", "user custom subject")
        code, _, _ = self.run_probe("--base", self.base)
        self.assertEqual(code, 0)
        code, report, _ = self.run_probe("--base", self.base, "--conventional-commits")
        self.assertEqual(code, 1)
        self.assertIn("commit-subject-policy", [x["code"] for x in report["findings"]])
        code, _, _ = self.run_probe("--scope", "staged", "--branch-pattern", "codex/.+")
        self.assertEqual(code, 0)
        code, _, _ = self.run_probe("--scope", "staged", "--branch-pattern", "feat/.+")
        self.assertEqual(code, 1)

    def test_detached_and_linked_worktree(self):
        linked = self.root / "linked"
        self.g("worktree", "add", "-b", "codex/linked", str(linked), self.base)
        code, report, _ = self.run_probe("--base", self.base, repo=linked)
        self.assertEqual(code, 0)
        self.assertEqual(report["worktree_count"], 2)
        self.g("switch", "--detach", self.base, repo=linked)
        code, report, _ = self.run_probe("--base", self.base, repo=linked)
        self.assertEqual(code, 0)
        self.assertTrue(report["detached"])

    def test_rerun_has_no_index_refs_or_file_effect(self):
        self.stage("selected.txt", "benign")
        (self.repo / "untracked.txt").write_text("user work", encoding="utf-8")
        index = self.repo / ".git" / "index"
        before = hashlib.sha256(index.read_bytes()).hexdigest()
        refs = self.g("show-ref")
        files = {p.name: p.read_bytes() for p in self.repo.iterdir() if p.is_file()}
        for _ in range(2):
            code, report, _ = self.run_probe("--base", self.base)
            self.assertEqual(code, 0)
            self.assertTrue(report["dirty"])
        self.assertEqual(hashlib.sha256(index.read_bytes()).hexdigest(), before)
        self.assertEqual(self.g("show-ref"), refs)
        self.assertEqual({p.name: p.read_bytes() for p in self.repo.iterdir() if p.is_file()}, files)

    def test_invalid_input_and_nonrepo(self):
        code, _, _ = self.run_probe("--scope", "staged", "--max-objects", "0")
        self.assertEqual(code, 2)
        code, _, _ = self.run_probe("--scope", "staged", repo=self.root)
        self.assertEqual(code, 2)

    def test_missing_git_is_explicit(self):
        environment = os.environ.copy()
        environment["PATH"] = str(self.root / "missing-tools")
        result = subprocess.run([sys.executable, str(self.script), "--repo", str(self.repo),
                                 "--scope", "staged"], cwd=self.root, env=environment,
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(json.loads(result.stdout)["error"], "git-unavailable")

    def test_object_limit_and_invalid_pattern(self):
        self.stage("one.txt", "one")
        self.stage("two.txt", "two")
        code, report, _ = self.run_probe("--scope", "staged", "--max-objects", "1")
        self.assertEqual(code, 1)
        self.assertIn("object-limit-incomplete", [x["code"] for x in report["findings"]])
        code, _, _ = self.run_probe("--scope", "staged", "--branch-pattern", "[")
        self.assertEqual(code, 2)

    def test_git_cli_error_does_not_echo_value(self):
        code, report, raw = self.run_probe("--scope", "staged", "--max-objects", TOKEN)
        self.assertEqual(code, 2)
        self.assertEqual(report["error"], "invalid-arguments")
        self.assertNotIn(TOKEN, raw)

    def test_shallow_clone_never_claims_complete_history(self):
        shallow = self.root / "shallow"
        self.g("clone", "--depth", "1", self.repo.as_uri(), str(shallow))
        code, report, _ = self.run_probe("--base", self.base, repo=shallow)
        self.assertEqual(code, 1)
        self.assertIn("shallow-history-incomplete", [x["code"] for x in report["findings"]])


class ProbeHandler(http.server.BaseHTTPRequestHandler):
    visits = []

    def log_message(self, *args):
        pass

    def do_HEAD(self):
        self.do_GET()

    def do_GET(self):
        type(self).visits.append(self.path)
        if self.path.startswith("/slow"):
            time.sleep(0.3)
        status = 500 if self.path.startswith("/error") else 302 if self.path.startswith("/redirect") else 200
        self.send_response(status)
        self.send_header("Content-Type", "application/json; private=" + TOKEN)
        self.send_header("Set-Cookie", COOKIE)
        self.send_header("Set-Cookie", "another=" + TOKEN)
        self.send_header("X-Secret-Value", TOKEN)
        self.send_header("Location", "/followed?token=" + TOKEN)
        self.end_headers()
        if self.command != "HEAD":
            try:
                self.wfile.write(TOKEN.encode())
            except (ConnectionError, OSError):
                pass


class HttpCases(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temp = tempfile.TemporaryDirectory(prefix="babata-ops-tests-")
        cls.root = Path(cls.temp.name)
        cls.skill = cls.root / "standalone" / "babata-ops"
        shutil.copytree(SKILLS / "babata-ops", cls.skill)
        cls.script = cls.skill / "scripts" / "probe.py"
        cls.server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), ProbeHandler)
        cls.thread = threading.Thread(target=cls.server.serve_forever, daemon=True)
        cls.thread.start()
        cls.url = "http://127.0.0.1:" + str(cls.server.server_port)

    @classmethod
    def tearDownClass(cls):
        cls.server.shutdown()
        cls.server.server_close()
        cls.thread.join(timeout=3)
        cls.temp.cleanup()

    def run_probe(self, path="/health", *args):
        return execute(self.script, "--url", self.url + path, *args, cwd=self.root)

    def test_healthy_redaction_and_legal_duplicate_cookies(self):
        code, report, raw = self.run_probe("/health?secret=" + TOKEN)
        self.assertEqual(code, 0)
        self.assertTrue(report["http_healthy"])
        self.assertEqual(report["duplicate_headers"]["set-cookie"], 2)
        self.assertEqual(len([x for x in report["headers"] if x[0].lower() == "set-cookie"]), 2)
        self.assertEqual(report["fault_origin"], "unknown")
        self.assertFalse(report["body_read"])
        self.assertNotIn(TOKEN, raw)
        self.assertNotIn(COOKIE, raw)
        self.assertNotIn("/health", raw)

    def test_http_500_is_responded_but_not_healthy(self):
        code, report, _ = self.run_probe("/error")
        self.assertEqual(code, 1)
        self.assertEqual(report["transport"], "responded")
        self.assertEqual(report["status"], 500)
        self.assertFalse(report["http_healthy"])
        self.assertEqual(report["fault_origin"], "unknown")

    def test_redirect_is_not_followed(self):
        before = len(ProbeHandler.visits)
        code, report, _ = self.run_probe("/redirect")
        self.assertEqual(code, 1)
        self.assertEqual(report["status"], 302)
        self.assertEqual(ProbeHandler.visits[before:], ["/redirect"])

    def test_head(self):
        code, report, _ = self.run_probe("/health", "--method", "HEAD")
        self.assertEqual(code, 0)
        self.assertEqual(report["method"], "HEAD")

    def test_timeout(self):
        code, report, _ = self.run_probe("/slow", "--timeout", "0.05")
        self.assertEqual(code, 5)
        self.assertEqual(report["transport"], "timeout")

    def test_connection_failure(self):
        with socket.socket() as guard:
            guard.bind(("127.0.0.1", 0))  # bound but not listening; prevents port reuse
            url = "http://127.0.0.1:" + str(guard.getsockname()[1]) + "/" + TOKEN
            code, report, raw = execute(self.script, "--url", url, "--timeout", "5", cwd=self.root)
        self.assertEqual(code, 4)
        self.assertEqual(report["transport"], "failed")
        self.assertNotIn(TOKEN, raw)

    def test_invalid_cli_arguments_do_not_echo_secrets(self):
        for option in ("--timeout", "--method"):
            code, report, raw = self.run_probe("/health", option, TOKEN)
            self.assertEqual(code, 2)
            self.assertEqual(report["error"], "invalid-arguments")
            self.assertNotIn(TOKEN, raw)

    def test_invalid_urls_and_timeout_do_not_leak(self):
        for url in ("file:///" + TOKEN, "http://user:" + TOKEN + "@example.invalid/",
                    "http://example.invalid:bad/" + TOKEN, "http://[invalid/" + TOKEN):
            with self.subTest(url=url[:10]):
                code, _, raw = execute(self.script, "--url", url, cwd=self.root)
                self.assertEqual(code, 2)
                self.assertNotIn(TOKEN, raw)
        for timeout in ("0", "-1", "nan", "inf"):
            code, _, _ = self.run_probe("/health", "--timeout", timeout)
            self.assertEqual(code, 2)


if __name__ == "__main__":
    unittest.main()
