"""Released skills must work when their directory is copied out of the repository on its own."""
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

ATOMS = Path(__file__).resolve().parent
REPO = ATOMS.parent.parent
SKILLS = ["babata-render", "babata-vision", "babata-research"]


def detached(tmp_path, skill):
    target = tmp_path / "released" / skill
    shutil.copytree(REPO / "02_skills" / skill, target)
    return target


def clean_env(**extra):
    env = {k: v for k, v in os.environ.items() if not k.startswith("BABATA_") and k != "PYTHONPATH"}
    env.update(extra)
    return env


def run(cmd, cwd, env=None):
    p = subprocess.run([str(c) for c in cmd], cwd=cwd, capture_output=True, text=True, encoding="utf-8",
                       errors="replace", env=env or clean_env())
    assert p.returncode == 0, f"{cmd}\n{p.stderr[-800:]}"
    return json.loads(p.stdout) if p.stdout.strip().startswith("{") else p.stdout


@pytest.mark.parametrize("skill", SKILLS)
def test_skill_text_does_not_point_back_into_the_repository(skill):
    for path in (REPO / "02_skills" / skill).rglob("*.md"):
        text = path.read_text(encoding="utf-8")
        assert "05_scripts" not in text and "BABATA_REPO_HOME" not in text, path


@pytest.mark.parametrize("skill", ["babata-render", "babata-vision"])
def test_detached_image_skill_runs_its_own_scripts(tmp_path, skill):
    root = detached(tmp_path, skill)
    img = tmp_path / "in.png"
    arr = np.zeros((40, 60, 4), np.uint8); arr[10:30, 15:45] = (200, 40, 40, 255)
    Image.fromarray(arr, "RGBA").save(img)
    out = run([sys.executable, root / "scripts" / "image_ops.py", "measure", img, "--out", tmp_path / "m"], cwd=tmp_path)
    assert out["state"] == "succeeded" and out["producer"]["tool"].endswith("image_ops.py")
    probe = run([sys.executable, root / "scripts" / "vision_backend.py", "probe"], cwd=tmp_path,
                env=clean_env(BABATA_BL=str(tmp_path / "no-bl.exe")))
    assert probe["backends"]["bl"]["installed"] is False and probe["backends"]["rembg"]["configured"] is False


def test_detached_render_skill_renders_png(tmp_path):
    node = shutil.which("node")
    modules = ATOMS / "node_modules"
    if not node or not modules.is_dir():
        pytest.skip("node or renderer dependencies unavailable")
    root = detached(tmp_path, "babata-render")
    md = tmp_path / "doc.md"; md.write_text("# 独立发布\n\n切出仓库也能渲染。\n", encoding="utf-8")
    run([node, root / "scripts" / "render.mjs", "render", "--input", md, "--out", tmp_path / "png", "--mode", "long"],
        cwd=tmp_path, env=clean_env(BABATA_NODE_MODULES=str(modules)))
    assert (tmp_path / "png" / "output.png").is_file()


def test_detached_research_skill_records_a_run(tmp_path):
    node = shutil.which("node")
    if not node:
        pytest.skip("node unavailable")
    root = detached(tmp_path, "babata-research")
    req = tmp_path / "request.json"
    req.write_text(json.dumps({"question": "独立发布能否运行？", "scope": "test", "producer": "test", "depth": "research"}), encoding="utf-8")
    run([node, root / "scripts" / "research.mjs", "init", "--request", req, "--run", tmp_path / "run"], cwd=tmp_path)
    run([node, root / "scripts" / "research.mjs", "verify", "--run", tmp_path / "run"], cwd=tmp_path)


def test_bundles_match_canonical_sources():
    p = subprocess.run([sys.executable, REPO / "05_scripts" / "sync-skill-bundles.py", "--check"], capture_output=True, text=True)
    assert p.returncode == 0, p.stderr
