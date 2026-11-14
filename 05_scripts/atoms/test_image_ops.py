"""Contract tests for image_ops.py / vision_backend.py (pytest, offline, no model calls)."""
import json
import os
import subprocess
import sys
from pathlib import Path

import numpy as np
import pytest
from PIL import Image

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import image_ops as io  # noqa: E402
import vision_backend as vb  # noqa: E402


def save(path: Path, arr, mode="RGBA"):
    Image.fromarray(np.asarray(arr, np.uint8), mode).save(path)
    return str(path)


def run(argv, expect=0):
    code = io.main(argv)
    assert code == expect, f"exit {code} for {argv}"


def manifest(out):
    return json.loads((Path(out) / "manifest.json").read_text(encoding="utf-8"))


def solid(h, w, rgba):
    return np.broadcast_to(np.array(rgba, np.uint8), (h, w, 4)).copy()


def test_over_hand_computed_value_and_order(tmp_path):
    top = save(tmp_path / "red.png", solid(4, 4, (255, 0, 0, 128)))
    bottom = save(tmp_path / "blue.png", solid(4, 4, (0, 0, 255, 255)))
    out = tmp_path / "run"
    run(["over", top, bottom, "--out", str(out)])
    px = np.asarray(Image.open(out / "composite.png"))[0, 0]
    assert np.all(np.abs(px.astype(int) - [128, 0, 127, 255]) <= 1)


def test_over_refuses_size_mismatch_unless_fit(tmp_path):
    a = save(tmp_path / "a.png", solid(600, 800, (255, 0, 0, 255)))
    b = save(tmp_path / "b.png", solid(300, 400, (0, 0, 255, 255)))
    run(["over", a, b, "--out", str(tmp_path / "r1")], expect=2)
    assert not (tmp_path / "r1" / "manifest.json").exists()
    run(["over", a, b, "--fit", "stretch", "--out", str(tmp_path / "r2")])
    assert manifest(tmp_path / "r2")["measurements"]["transforms"][0]["fit"] == "stretch"


def test_input_hash_unchanged_and_replay_and_conflict(tmp_path):
    src = tmp_path / "in.png"
    arr = solid(20, 20, (255, 255, 255, 255)); arr[5:15, 5:15] = (10, 10, 10, 255)
    save(src, arr)
    before = io.sha256_file(src)
    out = tmp_path / "run"
    run(["key-white", str(src), "--out", str(out)])
    assert io.sha256_file(src) == before
    first = manifest(out)
    assert first["state"] == "succeeded"
    code = io.main(["key-white", str(src), "--out", str(out)])
    assert code == 0  # replay verifies and reuses
    run(["key-white", str(src), "--level", "240", "--out", str(out)], expect=2)  # different request, same run


def test_tampered_artifact_and_missing_manifest_not_ready(tmp_path):
    src = save(tmp_path / "in.png", solid(8, 8, (1, 2, 3, 255)))
    out = tmp_path / "run"
    run(["resize", src, "--width", "4", "--out", str(out)])
    with open(out / "resized.png", "ab") as fh:
        fh.write(b"x")
    run(["verify", "--out", str(out)], expect=3)
    (out / "manifest.json").unlink()
    run(["verify", "--out", str(out)], expect=3)
    run(["resize", src, "--width", "4", "--out", str(out)], expect=2)  # incomplete dir, not silently reused


def test_manifest_request_tamper_detected(tmp_path):
    src = save(tmp_path / "in.png", solid(8, 8, (1, 2, 3, 255)))
    out = tmp_path / "run"
    run(["resize", src, "--width", "4", "--out", str(out)])
    m = manifest(out); m["request"]["width"] = 5
    (out / "manifest.json").write_text(json.dumps(m), encoding="utf-8")
    run(["verify", "--out", str(out)], expect=3)


def test_alpha_only_and_hidden_rgb_are_separated(tmp_path):
    a = solid(10, 10, (255, 255, 255, 255))
    b = a.copy(); b[2:4, 2:4, 3] = 0          # alpha-only change: same RGB
    c = solid(10, 10, (0, 0, 0, 0)); d = c.copy(); d[5:7, 5:7, :3] = 200  # hidden RGB under alpha 0
    pa, pb, pc, pd = (save(tmp_path / f"{n}.png", x) for n, x in zip("abcd", (a, b, c, d)))
    run(["diff", pa, pb, "--out", str(tmp_path / "r1")])
    m = manifest(tmp_path / "r1")["measurements"]
    assert m["raw_alpha"]["bbox_inclusive"] == [2, 2, 3, 3]
    assert m["visible"]["white"]["changed_pixels"] == 0      # invisible on white
    assert m["visible"]["black"]["changed_pixels"] == 4      # visible on black
    run(["diff", pc, pd, "--out", str(tmp_path / "r2")])
    m = manifest(tmp_path / "r2")["measurements"]
    assert m["hidden_rgb_under_full_transparency"]["changed_pixels"] == 4
    assert all(v["changed_pixels"] == 0 for v in m["visible"].values())


def test_multi_part_profile_does_not_flag_legal_parts(tmp_path):
    arr = solid(60, 120, (0, 0, 0, 0))
    for x in (5, 45, 85):
        arr[20:40, x:x + 25] = (30, 60, 200, 255)
    p = save(tmp_path / "icons.png", arr)
    run(["measure", p, "--profile", "multi-part", "--expect-parts", "3", "--out", str(tmp_path / "r1")])
    m = manifest(tmp_path / "r1")
    assert m["measurements"]["alpha"]["major_components"] == 3
    assert not [f for f in m["findings"] if f["kind"] in ("multiple-components", "part-count-mismatch")]
    run(["measure", p, "--profile", "cutout", "--out", str(tmp_path / "r2")])
    assert any(f["kind"] == "multiple-components" for f in manifest(tmp_path / "r2")["findings"])


def test_symlink_corrupt_oversize_and_animated_inputs_refused(tmp_path):
    good = save(tmp_path / "good.png", solid(10, 10, (1, 1, 1, 255)))
    bad = tmp_path / "bad.png"; bad.write_bytes(b"\x89PNG\r\n\x1a\nnot really")
    run(["probe", str(bad)], expect=2)
    run(["probe", str(tmp_path / "missing.png")], expect=2)
    run(["--max-pixels", "50", "probe", good], expect=2)
    link = tmp_path / "link.png"
    try:
        os.symlink(good, link)
    except OSError:
        pytest.skip("symlink creation not permitted")
    run(["probe", str(link)], expect=2)


@pytest.mark.skipif(os.name != "nt", reason="junction is Windows-specific")
def test_input_through_junction_refused(tmp_path):
    real = tmp_path / "real"; real.mkdir()
    save(real / "x.png", solid(4, 4, (1, 1, 1, 255)))
    junction = tmp_path / "jn"
    import _winapi
    _winapi.CreateJunction(str(real), str(junction))
    run(["probe", str(junction / "x.png")], expect=2)


def test_relative_and_in_git_outputs_refused(tmp_path):
    src = save(tmp_path / "in.png", solid(4, 4, (1, 1, 1, 255)))
    run(["resize", src, "--width", "2", "--out", "relative-dir"], expect=2)
    run(["resize", src, "--width", "2", "--out", str(HERE / "should-not-exist")], expect=2)
    assert not (HERE / "should-not-exist").exists()


def test_exif_orientation_and_palette_transparency(tmp_path):
    im = Image.new("RGB", (40, 20), "white"); im.putpixel((0, 0), (255, 0, 0))
    exif = im.getexif(); exif[0x0112] = 6
    p = tmp_path / "rot.jpg"; im.save(p, exif=exif.tobytes(), quality=100)
    rgba, rec = io.load_image(str(p), "in", {"max_pixels": 10**8, "max_bytes": 10**9})
    assert (rec["width"], rec["height"]) == (20, 40) and rec["exif_orientation"] == 6
    pal = Image.new("P", (4, 4), 0); pal.putpalette([0, 0, 0, 255, 0, 0] + [0] * 762); pal.info["transparency"] = 0
    pal.putpixel((1, 1), 1); pp = tmp_path / "pal.png"; pal.save(pp, transparency=0)
    rgba, rec = io.load_image(str(pp), "in", {"max_pixels": 10**8, "max_bytes": 10**9})
    assert rec["has_alpha"] and rgba[0, 0, 3] == 0 and tuple(rgba[1, 1]) == (255, 0, 0, 255)


def test_key_white_keeps_enclosed_white_subject(tmp_path):
    arr = solid(40, 40, (255, 255, 255, 255))
    arr[8:32, 8:32] = (20, 20, 20, 255)
    arr[14:26, 14:26] = (252, 252, 252, 255)   # white label inside dark subject
    p = save(tmp_path / "in.png", arr)
    run(["key-white", p, "--out", str(tmp_path / "r")])
    out = np.asarray(Image.open(tmp_path / "r" / "cutout.png"))
    assert out[20, 20, 3] == 255 and out[0, 0, 3] == 0
    assert any(f["kind"] == "enclosed-near-white-kept" for f in manifest(tmp_path / "r")["findings"])


def test_annotate_writes_copy_and_rejects_out_of_bounds(tmp_path):
    p = save(tmp_path / "in.png", solid(50, 50, (9, 9, 9, 255)))
    before = io.sha256_file(Path(p))
    run(["annotate", p, "--box", "5,5,20,20,问题", "--out", str(tmp_path / "r")])
    assert io.sha256_file(Path(p)) == before
    assert manifest(tmp_path / "r")["measurements"]["boxes_inclusive"] == [[5, 5, 20, 20]]
    run(["annotate", p, "--box", "5,5,60,20", "--out", str(tmp_path / "r2")], expect=2)


def test_tiles_cover_every_pixel_and_map_back(tmp_path):
    arr = solid(3000, 500, (255, 255, 255, 255)); arr[2900, 450] = (0, 0, 0, 255)
    p = save(tmp_path / "long.png", arr)
    run(["tiles", p, "--tile", "1000", "--overlap", "100", "--out", str(tmp_path / "r")])
    m = manifest(tmp_path / "r")
    assert m["measurements"]["coverage_ratio"] == 1.0
    hit = [t for t in m["measurements"]["tiles"] if t["box_inclusive"][1] <= 2900 <= t["box_inclusive"][3]]
    t = hit[-1]; tile = np.asarray(Image.open(tmp_path / "r" / t["file"]))
    assert tuple(tile[2900 - t["box_inclusive"][1], 450 - t["box_inclusive"][0]][:3]) == (0, 0, 0)


@pytest.mark.parametrize("content,finish,expect,state,reason", [
    ("", "stop", "text", "failed", "empty-content"),
    ("partial answ", "length", "text", "failed", "truncated"),
    ("{not json", "stop", "json", "failed", "bad-json"),
    ("```json\n{\"a\": 1}\n```", "stop", "json", "succeeded", ""),
    ("fine", "content_filter", "text", "failed", "finish-reason-content_filter"),
])
def test_answer_validation(content, finish, expect, state, reason):
    s, r, _ = vb.validate_answer(content, finish, expect)
    assert s == state and r.startswith(reason)


def test_matte_unavailable_is_reported_not_substituted(tmp_path, monkeypatch):
    p = save(tmp_path / "in.png", solid(8, 8, (1, 1, 1, 255)))
    monkeypatch.delenv("BABATA_REMBG_PYTHON", raising=False)
    assert vb.main(["matte", "--image", p, "--out", str(tmp_path / "r")]) == 4
    assert not (tmp_path / "r" / "manifest.json").exists()


def test_bl_adapter_handles_fake_backend_outputs(tmp_path, monkeypatch):
    """Replaceability: a stub backend proves the contract only, never real recognition."""
    p = save(tmp_path / "in.png", solid(8, 8, (1, 1, 1, 255)))
    for i, payload in enumerate([{"content": "", "finish_reason": "stop"}, {"content": "ok", "finish_reason": "length"},
                                 {"content": "答案", "finish_reason": "stop"}]):
        monkeypatch.setattr(vb, "call_bl", lambda *a, payload=payload: {"transport": "ok", "elapsed_ms": 1, "usage": None,
                                                                        "model": "stub", "request_id": "x", **payload})
        monkeypatch.setattr(vb, "backend_identity", lambda k, m: {"backend": k, "model": m})
        code = vb.main(["ask", "--backend", "bl", "--image", p, "--prompt", "q", "--out", str(tmp_path / f"r{i}")])
        assert code == (0 if i == 2 else 3)
