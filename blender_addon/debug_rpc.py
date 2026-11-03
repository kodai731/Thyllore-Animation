"""Debug-only JSON-RPC socket: run Python inside the live Blender session from the command line.

A listener thread accepts connections and queues request lines; a ``bpy.app.timers`` pump runs
them on the main thread (bpy is not thread-safe) and hands the responses back to the connection
thread. Registered only in a debug build (``_debuglog.is_debug_enabled``); the socket has no
authentication, so it binds the loopback address unless ``THYLLORE_DEBUG_RPC_HOST`` says otherwise.
"""
from __future__ import annotations

import os
import queue
import socket
import socketserver
import threading
from typing import Optional

import bpy

from . import _debuglog
from .common import json_rpc
from .common.logged_report import report

PORT_ENV_VAR = "THYLLORE_DEBUG_RPC_PORT"
HOST_ENV_VAR = "THYLLORE_DEBUG_RPC_HOST"
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 8799
PUMP_INTERVAL_SECONDS = 0.05


def configured_address() -> tuple[str, int]:
    return os.environ.get(HOST_ENV_VAR, DEFAULT_HOST), int(os.environ.get(PORT_ENV_VAR, DEFAULT_PORT))


class _Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True

    def __init__(self, address: tuple[str, int]) -> None:
        super().__init__(address, _Connection)
        self.pending: queue.Queue[tuple[str, queue.Queue[str]]] = queue.Queue()
        self.namespace: dict = {"__name__": "__thyllore_debug_rpc__", "bpy": bpy}


class _Connection(socketserver.StreamRequestHandler):
    def handle(self) -> None:
        for raw_line in self.rfile:
            line = raw_line.decode("utf-8", errors="replace").strip()
            if not line:
                continue
            reply: queue.Queue[str] = queue.Queue(maxsize=1)
            self.server.pending.put((line, reply))
            self.wfile.write(reply.get().encode("utf-8"))
            self.wfile.flush()


_server: Optional[_Server] = None
_thread: Optional[threading.Thread] = None


def _pump() -> Optional[float]:
    if _server is None:
        return None
    while True:
        try:
            line, reply = _server.pending.get_nowait()
        except queue.Empty:
            return PUMP_INTERVAL_SECONDS
        reply.put(json_rpc.handle_line(line, _server.namespace))


def listening_address() -> Optional[str]:
    if _server is None:
        return None
    host, port = _server.server_address[:2]
    return f"{host}:{port}"


def start() -> str:
    global _server, _thread
    if _server is not None:
        return listening_address() or ""
    _server = _Server(configured_address())
    _thread = threading.Thread(target=_server.serve_forever, name="thyllore-debug-rpc", daemon=True)
    _thread.start()
    bpy.app.timers.register(_pump, persistent=True)
    address = listening_address() or ""
    _debuglog.log_report("INFO", "debug_rpc", f"listening on {address}")
    return address


def stop() -> None:
    global _server, _thread
    if _server is None:
        return
    if bpy.app.timers.is_registered(_pump):
        bpy.app.timers.unregister(_pump)
    _server.shutdown()
    _server.server_close()
    _server = None
    _thread = None
    _debuglog.log_report("INFO", "debug_rpc", "stopped")


class THYLLORE_OT_StartDebugRpc(bpy.types.Operator):
    bl_idname = "thyllore.start_debug_rpc"
    bl_label = "Start RPC Server"
    bl_description = "Accept eval / exec requests from scripts/blender/rpc.py on the configured port"

    @classmethod
    def poll(cls, context):
        return _server is None

    def execute(self, context):
        try:
            address = start()
        except OSError as error:
            report(self, "ERROR", f"RPC server failed to start: {error}")
            return {"CANCELLED"}
        report(self, "INFO", f"RPC server listening on {address}")
        return {"FINISHED"}


class THYLLORE_OT_StopDebugRpc(bpy.types.Operator):
    bl_idname = "thyllore.stop_debug_rpc"
    bl_label = "Stop RPC Server"

    @classmethod
    def poll(cls, context):
        return _server is not None

    def execute(self, context):
        stop()
        report(self, "INFO", "RPC server stopped")
        return {"FINISHED"}


_CLASSES = (THYLLORE_OT_StartDebugRpc, THYLLORE_OT_StopDebugRpc)


def register() -> None:
    for cls in _CLASSES:
        bpy.utils.register_class(cls)
    if os.environ.get(PORT_ENV_VAR):
        try:
            print(f"[Thyllore] debug RPC listening on {start()}")
        except OSError as error:
            print(f"[Thyllore] debug RPC failed to start: {error}")


def unregister() -> None:
    stop()
    for cls in reversed(_CLASSES):
        bpy.utils.unregister_class(cls)
