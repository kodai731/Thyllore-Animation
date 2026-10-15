"""Spring-bone parity: run engine and Unity spring simulation on the same model, compare tails.

    python3 tools/spring_parity.py --dood [--max-rms <float>] [--work-dir <path>]
"""

from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from engine_harness import dood_wrap, engine_env, engine_path, repo_root

DEFAULT_WORK_DIR = "target/spring_parity"
UNITY_IMAGE_TAG = "thyllore-unity-gui"
UNIVRM_MANIFEST_ENTRIES = {
    "com.vrmc.gltf": "https://github.com/vrm-c/UniVRM.git?path=/Packages/UniGLTF#v0.131.2",
    "com.vrmc.vrm": "https://github.com/vrm-c/UniVRM.git?path=/Packages/VRM10#v0.131.2",
}


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--dood", action="store_true", help="run engine inside docker (dood)")
    p.add_argument("--max-rms", type=float, default=None, help="max RMS threshold for spring_tail_compare")
    p.add_argument("--work-dir", default=DEFAULT_WORK_DIR, help="working directory (default: target/spring_parity)")
    return p.parse_args()


def build_rig(work_dir: Path) -> Path:
    """Build synthetic_avatar.fbx via Blender and copy to work_dir."""
    tmp = work_dir / "tmp"
    tmp.mkdir(parents=True, exist_ok=True)
    (work_dir / "synthetic_avatar.fbx").unlink(missing_ok=True)

    cmd = [
        "bash", "blender/docker/run_background.sh",
        "-b", "--python", "unity/verify/rig.py", "--", str(tmp), "--sway",
    ]
    env = dict(os.environ, TMPDIR=str(tmp))
    subprocess.run(cmd, cwd=repo_root(), env=env, check=True)

    fbx = work_dir / "synthetic_avatar.fbx"
    shutil.copy2(tmp / "synthetic_avatar.fbx", fbx)
    return fbx


def run_engine(work_dir: Path, fbx_path: Path, dood: bool) -> None:
    """Run engine to add spring chains, export avatar sidecar, and dump spring tails."""
    scene = work_dir / "synthetic_avatar.scene.ron"
    scene.write_text(
        f'(\n    version: 8,\n    model: (\n        path: "{fbx_path}",\n    ),\n)\n'
    )

    engine = engine_path()
    avatar_json = work_dir / "synthetic_avatar.avatar.json"
    avatar_json.unlink(missing_ok=True)

    cmd = [
        str(engine),
        "--batch-scene", str(scene),
        "--batch-screenshot", str(work_dir / "engine.png"),
        "--batch-frames", "20",
        "--batch-debug-action-at", "10:add_spring_chains=Left_braid",
        "--batch-debug-action-at", "12:export_unity_avatar",
        "--batch-debug-action-at", f"14:dump_spring_tails={work_dir / 'engine_tails.csv'},30",
    ]

    if dood:
        cmd = dood_wrap(cmd)

    subprocess.run(cmd, cwd=repo_root(), env=engine_env(), check=True)

    assert avatar_json.exists(), f"engine did not write {avatar_json}"
    assert (work_dir / "engine_tails.csv").exists(), "engine did not write engine_tails.csv"


def _unity_docker_args(work_dir: Path, env: dict[str, str]) -> list[str]:
    """Common docker run args for Unity container."""
    home = Path.home()
    license_dir = os.environ.get("UNITY_LICENSE_DIR", str(home / ".config/unity3d/Unity/licenses"))
    container_home = "/work/" + "home"
    return [
        "docker", "run", "--rm",
        "--user", f"{os.getuid()}:{os.getgid()}",
        "-e", f"HOME={container_home}",
        "-v", f"{work_dir}/unity:/work",
        "-v", f"{license_dir}:{container_home}/.config/unity3d/Unity/licenses:ro",
        "-v", "/etc/machine-id:/etc/machine-id:ro",
        "-w", "/work",
    ]


def ensure_unity_dirs(work_dir: Path) -> None:
    """Create directories that docker bind-mounts need before the first container run."""
    (work_dir / "unity" / "project").mkdir(parents=True, exist_ok=True)
    (work_dir / "unity" / "home" / ".config" / "unity3d" / "Unity").mkdir(parents=True, exist_ok=True)
    (work_dir / "unity" / "home" / ".local" / "share").mkdir(parents=True, exist_ok=True)


def install_vrchat_sdk(work_dir: Path) -> None:
    """Install com.vrchat.avatars inside the Unity container when Packages/com.vrchat.avatars is missing."""
    project = work_dir / "unity" / "project"
    if (project / "Packages" / "com.vrchat.avatars").exists():
        return
    cmd = _unity_docker_args(work_dir, {}) + [
        "--entrypoint", "bash", UNITY_IMAGE_TAG, "-c",
        "cd /work/project && vrc-get install com.vrchat.avatars --yes",
    ]
    subprocess.run(cmd, check=True)


def add_univrm_packages(manifest_path: Path) -> None:
    """Add UniVRM entries to manifest.json if missing."""
    data = json.loads(manifest_path.read_text())
    for name, url in UNIVRM_MANIFEST_ENTRIES.items():
        if name not in data.get("dependencies", {}):
            data.setdefault("dependencies", {})[name] = url
    manifest_path.write_text(json.dumps(data, indent=2) + "\n")


def prepare_unity_project(work_dir: Path, fbx_path: Path) -> None:
    """Create Unity project, install packages, copy assets."""
    project = work_dir / "unity" / "project"
    if not (project / "ProjectSettings" / "ProjectVersion.txt").exists():
        docker_dir = repo_root() / "unity" / "docker"
        subprocess.run(
            ["docker", "build", "-t", UNITY_IMAGE_TAG, str(docker_dir)],
            cwd=repo_root(), check=True,
        )
        cmd = _unity_docker_args(work_dir, {}) + [UNITY_IMAGE_TAG, "unity-editor",
                                                   "-batchmode", "-nographics", "-quit",
                                                   "-createProject", "/work/project",
                                                   "-logFile", "/work/create.log"]
        subprocess.run(cmd, check=True)

    install_vrchat_sdk(work_dir)
    add_univrm_packages(project / "Packages" / "manifest.json")

    sync_sh = repo_root() / "unity" / "sync_package.sh"
    subprocess.run(["bash", str(sync_sh), str(work_dir / "unity")], cwd=repo_root(), check=True)

    editor_dir = project / "Assets" / "Editor"
    if editor_dir.exists():
        shutil.rmtree(editor_dir)
    editor_dir.mkdir(parents=True, exist_ok=True)

    for src_name in ("SpringParityCheck.cs",):
        src = repo_root() / "unity" / "verify" / src_name
        dst = editor_dir / src_name
        shutil.copy2(src, dst)

    asmdef = repo_root() / "unity" / "verify" / "Thyllore.Verify.Spring.asmdef"
    shutil.copy2(asmdef, editor_dir / "Thyllore.Verify.Spring.asmdef")

    check_dir = project / "Assets" / "Check"
    if check_dir.exists():
        shutil.rmtree(check_dir)
    check_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(fbx_path, check_dir / fbx_path.name)

    sidecar_src = fbx_path.parent / (fbx_path.stem + ".avatar.json")
    sidecar_dst = work_dir / "unity" / "sidecar.json"
    shutil.copy2(sidecar_src, sidecar_dst)


def run_unity_check(work_dir: Path) -> None:
    """Run Unity SpringParityCheck and verify it succeeds."""
    log_path = work_dir / "unity" / "parity.log"

    cmd = _unity_docker_args(work_dir, {}) + [
        UNITY_IMAGE_TAG, "unity-editor",
        "-batchmode", "-nographics",
        "-projectPath", "/work/project",
        "-executeMethod", "Thyllore.Avatar.SpringParityCheck.Run",
        "-logFile", "/work/parity.log",
    ]

    env = dict(os.environ,
               THYLLORE_FBX_ASSET="Assets/Check/synthetic_avatar.fbx",
               THYLLORE_SIDECAR="/work/sidecar.json",
               THYLLORE_TAILS_OUT="/work/unity_tails.csv",
               THYLLORE_SAMPLE_RATE="30",
               THYLLORE_FRAME_COUNT="31")

    result = subprocess.run(cmd, env=env, capture_output=True, text=True)
    if result.returncode != 0:
        log = log_path.read_text() if log_path.exists() else "(missing)"
        for line in log.splitlines():
            if "SPRINGPARITY" in line or "error CS" in line:
                print(line)
        raise SystemExit(f"Unity SpringParityCheck failed (exit {result.returncode})")

    assert (work_dir / "unity" / "unity_tails.csv").exists(), "Unity did not write unity_tails.csv"


def compare(work_dir: Path, max_rms: float | None) -> int:
    """Run spring_tail_compare on engine vs Unity tails."""
    bin_path = repo_root() / "target" / "debug" / "spring_tail_compare"
    if not bin_path.exists():
        subprocess.run(["cargo", "build", "--bin", "spring_tail_compare"], cwd=repo_root(), check=True)

    cmd = [str(bin_path), str(work_dir / "engine_tails.csv"), str(work_dir / "unity" / "unity_tails.csv")]
    if max_rms is not None:
        cmd += ["--max-rms", str(max_rms)]

    result = subprocess.run(cmd, cwd=repo_root())
    return result.returncode


def main() -> None:
    args = parse_args()
    work_dir = Path(args.work_dir).resolve()
    work_dir.mkdir(parents=True, exist_ok=True)

    fbx_path = build_rig(work_dir)
    run_engine(work_dir, fbx_path, args.dood)
    ensure_unity_dirs(work_dir)
    prepare_unity_project(work_dir, fbx_path)
    run_unity_check(work_dir)
    sys.exit(compare(work_dir, args.max_rms))


if __name__ == "__main__":
    main()
