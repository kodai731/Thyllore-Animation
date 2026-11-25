"""Tests for blender_addon.common.json_rpc."""
from __future__ import annotations

import json

from blender_addon.common import json_rpc


def _call(method, params=None, namespace=None):
    line = json_rpc.request_line(method, params, 7)
    return json.loads(json_rpc.handle_line(line, namespace if namespace is not None else {}))


def test_ping():
    assert _call("ping") == {"jsonrpc": "2.0", "id": 7, "result": "pong"}


def test_eval_returns_repr_and_stdout():
    response = _call("eval", {"expression": "print('hi') or 1 + 2"})
    assert response["result"] == {"value": "3", "stdout": "hi\n"}


def test_exec_keeps_the_namespace_between_requests():
    namespace = {}
    _call("exec", {"code": "answer = 6 * 7"}, namespace)
    assert _call("eval", {"expression": "answer"}, namespace)["result"]["value"] == "42"


def test_execution_error_carries_the_traceback():
    response = _call("exec", {"code": "raise ValueError('boom')"})
    assert response["error"]["code"] == json_rpc.EXECUTION_ERROR
    assert response["error"]["message"] == "ValueError: boom"
    assert "ValueError: boom" in response["error"]["data"]["traceback"]


def test_invalid_params():
    assert _call("eval", {"code": "1"})["error"]["code"] == json_rpc.INVALID_PARAMS


def test_unknown_method():
    assert _call("render")["error"]["code"] == json_rpc.METHOD_NOT_FOUND


def test_parse_error_and_invalid_request():
    assert json.loads(json_rpc.handle_line("{not json", {}))["error"]["code"] == json_rpc.PARSE_ERROR
    assert json.loads(json_rpc.handle_line('{"id": 1}', {}))["error"]["code"] == json_rpc.INVALID_REQUEST
