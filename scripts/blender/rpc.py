#!/usr/bin/env python3
"""Talk to the debug RPC socket of a running Blender (blender_addon/debug_rpc.py).

    rpc.py ping
    rpc.py eval "len(bpy.data.objects)"
    rpc.py exec "bpy.ops.thyllore.generate_chain_bones(mesh='Skirt', material='Skirt')"
    rpc.py exec -f log/regen_check/check.py

The session keeps one namespace, so a variable set by one exec is visible to the next call.
"""
from __future__ import annotations

import argparse
import importlib.util
import json
import os
import socket
import sys
from pathlib import Path


def _load_json_rpc():
    """The addon's protocol module loaded by path, so the addon package (and its wheel setup) stays out."""
    path = Path(__file__).resolve().parents[2] / "blender_addon" / "common" / "json_rpc.py"
    spec = importlib.util.spec_from_file_location("json_rpc", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


json_rpc = _load_json_rpc()

DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = int(os.environ.get("THYLLORE_DEBUG_RPC_PORT", "8799"))


def call(host: str, port: int, method: str, params: dict | None) -> dict:
    with socket.create_connection((host, port), timeout=600) as connection:
        connection.sendall(json_rpc.request_line(method, params, 1).encode("utf-8"))
        with connection.makefile("r", encoding="utf-8") as reader:
            return json.loads(reader.readline())


def parse_args(argv: list[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--host", default=DEFAULT_HOST)
    parser.add_argument("--port", type=int, default=DEFAULT_PORT)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("ping")
    commands.add_parser("eval").add_argument("expression")
    exec_parser = commands.add_parser("exec")
    source = exec_parser.add_mutually_exclusive_group(required=True)
    source.add_argument("code", nargs="?")
    source.add_argument("-f", "--file", type=Path)
    return parser.parse_args(argv)


def main(argv: list[str]) -> int:
    args = parse_args(argv)
    try:
        response = _send(args)
    except OSError as error:
        sys.stderr.write(f"cannot reach the debug RPC socket at {args.host}:{args.port} ({error}); is a debug Blender running?\n")
        return 2
    return _print(response)


def _send(args: argparse.Namespace) -> dict:
    if args.command == "ping":
        response = call(args.host, args.port, "ping", None)
    elif args.command == "eval":
        response = call(args.host, args.port, "eval", {"expression": args.expression})
    else:
        code = args.file.read_text(encoding="utf-8") if args.file else args.code
        response = call(args.host, args.port, "exec", {"code": code})
    return response


def _print(response: dict) -> int:
    if "error" in response:
        error = response["error"]
        data = error.get("data") or {}
        sys.stdout.write(data.get("stdout", ""))
        sys.stderr.write(data.get("traceback") or f"{error['message']}\n")
        return 1
    result = response["result"]
    if isinstance(result, dict):
        sys.stdout.write(result.get("stdout", ""))
        if "value" in result:
            print(result["value"])
    else:
        print(result)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
