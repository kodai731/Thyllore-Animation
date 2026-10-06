"""Ask the running engine for its animation state and print the JSON.

The engine listens on log/live_dump/engine.sock (a Unix socket in the
repository it was launched from). Each connection carries one JSON request
line and gets one JSON reply, applied on the engine's main thread through the
UI command queue, so nothing is polled.

Usage:
    python3 tools/live_dump.py                       # clips, timeline, curve editor
    python3 tools/live_dump.py --tracks              # also every bone track's keyframes
    python3 tools/live_dump.py --pose 0 1.0          # sampled poses at the given times
    python3 tools/live_dump.py --tracks --clip "New Clip"   # only that clip
    python3 tools/live_dump.py --out dump.json       # write instead of printing
"""

from __future__ import annotations

import argparse
import json
import socket
import sys
from pathlib import Path

SOCKET_PATH = Path("log/live_dump/engine.sock")


def build_request(args: argparse.Namespace) -> dict:
    if args.pose is not None:
        return {"kind": "pose", "times": args.pose}
    return {"kind": "anim", "include_tracks": args.tracks}


def send_request(request: dict, timeout_seconds: float) -> dict:
    if not SOCKET_PATH.exists():
        raise SystemExit(f"{SOCKET_PATH} not found: is the engine running from the repository root?")

    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as connection:
        connection.settimeout(timeout_seconds)
        try:
            connection.connect(str(SOCKET_PATH))
        except (ConnectionRefusedError, FileNotFoundError) as error:
            raise SystemExit(f"cannot connect to {SOCKET_PATH}: {error}; the engine has probably exited")
        connection.sendall((json.dumps(request) + "\n").encode())
        connection.shutdown(socket.SHUT_WR)

        chunks: list[bytes] = []
        while True:
            chunk = connection.recv(1 << 16)
            if not chunk:
                break
            chunks.append(chunk)

    return json.loads(b"".join(chunks).decode())


def select_clip(data: dict, clip_name: str) -> dict:
    clips = [clip for clip in data.get("clips", []) if clip.get("name") == clip_name]
    if not clips:
        names = [clip.get("name") for clip in data.get("clips", [])]
        raise SystemExit(f"clip '{clip_name}' not found; clips: {names}")
    return {"clips": clips, "timeline": data.get("timeline"), "curve_editor": data.get("curve_editor")}


def main() -> None:
    parser = argparse.ArgumentParser(description="Read the running engine's animation state.")
    parser.add_argument("--tracks", action="store_true", help="include every bone track's curves")
    parser.add_argument("--pose", nargs="+", type=float, metavar="TIME", help="sample poses at the given times")
    parser.add_argument("--clip", help="keep only the clip with this name (anim dump only)")
    parser.add_argument("--out", help="write the JSON to this file instead of stdout")
    parser.add_argument("--timeout", type=float, default=15.0, help="seconds to wait for the engine")
    args = parser.parse_args()

    response = send_request(build_request(args), args.timeout)
    if not response.get("ok"):
        raise SystemExit(f"engine error: {response.get('error')}")

    data = response["data"]
    if args.clip and args.pose is None:
        data = select_clip(data, args.clip)

    text = json.dumps(data, indent=2)
    if args.out:
        Path(args.out).write_text(text)
        print(f"wrote {args.out}", file=sys.stderr)
    else:
        print(text)


if __name__ == "__main__":
    main()
