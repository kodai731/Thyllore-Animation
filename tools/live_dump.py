"""Ask the engine that is already running for its animation state and print the JSON.

This never launches an engine: start it first (./run.sh engine), then run this
at any moment; a --timings DURATION recording begins when the request arrives.

The engine listens on log/live_dump/engine.sock (a Unix socket in the
repository it was launched from). Each connection carries one JSON request
line and gets one JSON reply, applied on the engine's main thread through the
UI command queue, so nothing is polled.

Usage:
    python3 tools/live_dump.py                       # clips, timeline, curve editor
    python3 tools/live_dump.py --tracks              # also every bone track's keyframes
    python3 tools/live_dump.py --pose 0 1.0          # sampled poses at the given times
    python3 tools/live_dump.py --tracks --clip "New Clip"   # only that clip
    python3 tools/live_dump.py --set-time 1.25       # scrub the timeline like the UI does
    python3 tools/live_dump.py --timings             # CPU ms of the last frame by phase and hook
    python3 tools/live_dump.py --timings 1s          # record every frame for 1 s, print mean/p50/max
    python3 tools/live_dump.py --timings 1s --out t.json   # keep every sample instead
    python3 tools/live_dump.py --out dump.json       # write instead of printing
"""

from __future__ import annotations

import argparse
import json
import socket
import sys
from pathlib import Path

SOCKET_PATH = Path("log/live_dump/engine.sock")


def parse_duration_seconds(text: str) -> float:
    unit_scale = {"ms": 0.001, "s": 1.0}
    for unit, scale in unit_scale.items():
        if text.endswith(unit):
            return float(text[: -len(unit)]) * scale
    return float(text)


def build_request(args: argparse.Namespace) -> dict:
    if args.set_time is not None:
        return {"kind": "set_time", "time": args.set_time}
    if args.timings is not None:
        if args.timings == "":
            return {"kind": "timings"}
        return {"kind": "timings", "seconds": parse_duration_seconds(args.timings)}
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


def flatten_sample(sample: dict) -> dict[str, float]:
    flat = {"dt_ms": sample.get("dt_ms")}
    for group in ("cpu", "update_phases", "render_prep"):
        for key, value in (sample.get(group) or {}).items():
            flat[f"{group}.{key}"] = value
    return {key: value for key, value in flat.items() if isinstance(value, (int, float))}


def summarize_samples(samples: list[dict]) -> dict:
    rows = [flatten_sample(sample) for sample in samples]
    keys = sorted({key for row in rows for key in row})
    summary = {}
    for key in keys:
        values = [row[key] for row in rows if key in row]
        values.sort()
        summary[key] = {
            "mean": sum(values) / len(values),
            "p50": values[len(values) // 2],
            "max": values[-1],
        }
    return {"frames": len(rows), "ms": summary}


def print_summary(summary: dict) -> None:
    print(f"frames: {summary['frames']}")
    print(f"{'metric':32} {'mean':>8} {'p50':>8} {'max':>8}")
    for key, stats in summary["ms"].items():
        if stats["max"] < 0.05:
            continue
        print(f"{key:32} {stats['mean']:8.2f} {stats['p50']:8.2f} {stats['max']:8.2f}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Read the running engine's animation state.")
    parser.add_argument("--tracks", action="store_true", help="include every bone track's curves")
    parser.add_argument("--pose", nargs="+", type=float, metavar="TIME", help="sample poses at the given times")
    parser.add_argument("--set-time", type=float, metavar="TIME", help="move the timeline to TIME (stops playback)")
    parser.add_argument(
        "--timings", nargs="?", const="", metavar="DURATION",
        help="CPU cost of the last frame by phase and hook; with DURATION (1s, 500ms) record every frame that long and print a summary",
    )
    parser.add_argument("--clip", help="keep only the clip with this name (anim dump only)")
    parser.add_argument("--out", help="write the JSON to this file instead of stdout")
    parser.add_argument("--timeout", type=float, default=15.0, help="seconds to wait for the engine")
    args = parser.parse_args()

    request = build_request(args)
    timeout = args.timeout + request.get("seconds", 0.0)
    response = send_request(request, timeout)
    if not response.get("ok"):
        raise SystemExit(f"engine error: {response.get('error')}")

    data = response["data"]
    if args.clip and args.pose is None:
        data = select_clip(data, args.clip)
    if "samples" in data and not args.out:
        print_summary(summarize_samples(data["samples"]))
        return

    text = json.dumps(data, indent=2)
    if args.out:
        Path(args.out).write_text(text)
        print(f"wrote {args.out}", file=sys.stderr)
    else:
        print(text)


if __name__ == "__main__":
    main()
