"""Developer debug switches for the Thyllore Animation addon.

``is_debug_build()`` is the Blender-side switch for the debug tools (debug RPC,
viewport recording); file logging additionally requires the Rust logger, which
the shipped wheel (``thyllore_ml_core`` or ``thyllore_avatar_api``) exports as
``debug_log`` / ``debug_log_init`` only when built with the ``debug-log``
feature. Production wheels omit those functions, so file logging is impossible
regardless of any env var or config.

This module only does the Blender-side glue: decide whether to log, resolve the
log file path, and format messages before handing them to Rust.

A debug build is one of:
1. ``THYLLORE_DEBUG`` environment variable is truthy — dev override.
2. A ``_debug_config.py`` baked into the addon by ``build_blender_addon.sh
   --debug`` sets ``DEBUG_BUILD = True``.

Log file path (first match wins): ``THYLLORE_BLENDER_LOG`` env, baked
``LOG_PATH``, or ``<repo_root>/log/log_blender.log`` discovered from the source
tree.
"""
from __future__ import annotations

import importlib
import os
from pathlib import Path
from typing import Callable, Optional, Tuple

_WHEEL_MODULES = ("thyllore_avatar_api", "thyllore_ml_core")
_TRUTHY = {"1", "true", "yes", "on"}


def _resolve_rust_logger() -> Tuple[Optional[Callable], Optional[Callable]]:
    for module_name in _WHEEL_MODULES:
        try:
            wheel = importlib.import_module(module_name)
        except ImportError:
            continue
        log_init = getattr(wheel, "debug_log_init", None)
        log = getattr(wheel, "debug_log", None)
        if log_init is not None and log is not None:
            return log_init, log
    return None, None


_rust_log_init, _rust_log = _resolve_rust_logger()


def _baked_config() -> Tuple[bool, str]:
    try:
        from . import _debug_config as config
    except ImportError:
        return False, ""
    return bool(getattr(config, "DEBUG_BUILD", False)), str(getattr(config, "LOG_PATH", ""))


def is_debug_build() -> bool:
    """True in a ``--debug`` addon build or under ``THYLLORE_DEBUG``; enables the debug tools."""
    if os.environ.get("THYLLORE_DEBUG", "").strip().lower() in _TRUTHY:
        return True
    baked_on, _ = _baked_config()
    return baked_on


def _discover_repo_log_path() -> Optional[Path]:
    for parent in Path(__file__).resolve().parents:
        if (parent / "Cargo.toml").is_file() and (parent / "log").is_dir():
            return parent / "log" / "log_blender.log"
    return None


def _resolve_log_path() -> Optional[Path]:
    env = os.environ.get("THYLLORE_BLENDER_LOG", "").strip()
    if env:
        return Path(env)

    _, baked = _baked_config()
    if baked:
        return Path(baked)

    return _discover_repo_log_path()


class _RustLogger:
    """Formats messages and forwards them to the Rust file logger."""

    def info(self, message: str, *args) -> None:
        _rust_log(message % args if args else message)

    def exception(self, message: str) -> None:
        import traceback

        _rust_log(f"{message}\n{traceback.format_exc()}")


_logger: Optional[_RustLogger] = None
_initialized = False


def get_logger() -> Optional[_RustLogger]:
    """Return a logger that writes via Rust, or ``None`` when debug is off."""
    global _logger, _initialized
    if _initialized:
        return _logger

    _initialized = True
    if not is_debug_build() or _rust_log is None or _rust_log_init is None:
        return None

    log_path = _resolve_log_path()
    if log_path is None:
        print("[Thyllore] debug mode on but no writable log path resolved")
        return None

    try:
        log_path.parent.mkdir(parents=True, exist_ok=True)
        _rust_log_init(str(log_path))
    except OSError as e:
        print(f"[Thyllore] failed to open debug log {log_path}: {e}")
        return None

    _logger = _RustLogger()
    _logger.info("debug log opened at %s", str(log_path))
    print(f"[Thyllore] debug log -> {log_path}")
    return _logger


def log_exception(message: str) -> None:
    """Record the current exception with traceback in the debug log.

    Call from ``except`` blocks that swallow errors, so failures stay
    diagnosable in debug builds. No-op in production wheels.
    """
    logger = get_logger()
    if logger is not None:
        logger.exception(f"ERROR {message}")


def log_report(level: str, source: str, message: str) -> None:
    """Record a message shown to the user (``Operator.report``) in the debug log."""
    logger = get_logger()
    if logger is not None:
        logger.info(f"{level} {source}: {message}")
