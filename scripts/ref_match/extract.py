"""Reference link reading and yt-dlp download shared by the flame/wind reference extractors."""

import subprocess
import sys


def read_link(out_dir):
    for line in (out_dir / "link.txt").read_text().splitlines():
        if line.startswith("http"):
            return line.strip()
    raise SystemExit("link.txt has no http line")


def download(url, video_path, format_id):
    video_path.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(
        [sys.executable, "-m", "yt_dlp", "-q", "-f", format_id, "-o", str(video_path), "--force-overwrites", url],
        check=True,
    )
