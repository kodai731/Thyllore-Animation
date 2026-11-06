"""Newline-delimited JSON-RPC 2.0 for the debug socket: one request per line, one response per line.

No bpy imports: the methods run against a namespace the server provides, so the protocol is
testable outside Blender.
"""
from __future__ import annotations

import contextlib
import io
import json
import traceback
from typing import Any

PARSE_ERROR = -32700
INVALID_REQUEST = -32600
METHOD_NOT_FOUND = -32601
INVALID_PARAMS = -32602
EXECUTION_ERROR = -32000

METHODS = ("ping", "eval", "exec")


class RpcError(Exception):
    def __init__(self, code: int, message: str, data: Any = None) -> None:
        super().__init__(message)
        self.code = code
        self.data = data


def request_line(method: str, params: dict | None, request_id: int) -> str:
    request = {"jsonrpc": "2.0", "method": method, "id": request_id}
    if params:
        request["params"] = params
    return json.dumps(request) + "\n"


def _response(request_id: Any, result: Any) -> dict:
    return {"jsonrpc": "2.0", "id": request_id, "result": result}


def _error(request_id: Any, code: int, message: str, data: Any = None) -> dict:
    error = {"code": code, "message": message}
    if data is not None:
        error["data"] = data
    return {"jsonrpc": "2.0", "id": request_id, "error": error}


def _run_python(source: str, namespace: dict, mode: str) -> dict:
    stdout = io.StringIO()
    try:
        with contextlib.redirect_stdout(stdout):
            if mode == "eval":
                value = eval(source, namespace)
                return {"value": repr(value), "stdout": stdout.getvalue()}
            exec(source, namespace)
            return {"stdout": stdout.getvalue()}
    except Exception as error:
        raise RpcError(
            EXECUTION_ERROR,
            f"{type(error).__name__}: {error}",
            {"traceback": traceback.format_exc(), "stdout": stdout.getvalue()},
        ) from error


def _string_param(params: Any, name: str) -> str:
    if not isinstance(params, dict) or not isinstance(params.get(name), str):
        raise RpcError(INVALID_PARAMS, f"params.{name} must be a string")
    return params[name]


def _dispatch(method: str, params: Any, namespace: dict) -> Any:
    if method == "ping":
        return "pong"
    if method == "eval":
        return _run_python(_string_param(params, "expression"), namespace, "eval")
    if method == "exec":
        return _run_python(_string_param(params, "code"), namespace, "exec")
    raise RpcError(METHOD_NOT_FOUND, f"unknown method {method!r}; expected one of {METHODS}")


def handle_line(line: str, namespace: dict) -> str:
    """The response line for one request line; every failure becomes a JSON-RPC error."""
    try:
        request = json.loads(line)
    except ValueError as error:
        return json.dumps(_error(None, PARSE_ERROR, f"parse error: {error}")) + "\n"

    request_id = request.get("id") if isinstance(request, dict) else None
    if not isinstance(request, dict) or request.get("jsonrpc") != "2.0" or not isinstance(request.get("method"), str):
        return json.dumps(_error(request_id, INVALID_REQUEST, "expected {jsonrpc: '2.0', method: str, id, params?}")) + "\n"

    try:
        result = _dispatch(request["method"], request.get("params"), namespace)
    except RpcError as error:
        return json.dumps(_error(request_id, error.code, str(error), error.data)) + "\n"
    return json.dumps(_response(request_id, result)) + "\n"
