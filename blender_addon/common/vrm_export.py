"""The VRM meta the wheel takes, built from the field table the wheel declares (bpy-independent)."""
from __future__ import annotations

import datetime
import getpass

try:
    import thyllore_ml_core as tml  # type: ignore

    _TML_AVAILABLE = True
except ImportError:
    tml = None  # type: ignore
    _TML_AVAILABLE = False

_META_TABLE: dict = tml.vrm_meta_fields() if _TML_AVAILABLE else {"default_version": "", "fields": []}

VRM_META_FIELDS: list[dict] = _META_TABLE["fields"]
VRM_META_DEFAULT_VERSION: str = _META_TABLE["default_version"]


def meta_fields_of_kind(kind: str) -> list[dict]:
    """The meta fields entered as ``kind`` ("text", "choice" or "flag"), in display order."""
    return [field for field in VRM_META_FIELDS if field["kind"] == kind]


def default_author() -> str:
    """The OS login name, the author a VRM gets when none is entered; empty when the OS has none."""
    try:
        return getpass.getuser()
    except (OSError, KeyError):
        return ""


def default_copyright(authors: list) -> str:
    """The copyright line a VRM gets when none is entered: the current year and the authors."""
    if not authors:
        return ""
    return f"(c) {datetime.date.today().year} {', '.join(authors)}"


def vrm_meta_to_dict(meta, fallback_name: str = "", fallback_author: str = "", fields: list[dict] = None) -> dict:
    """The ``VrmMeta`` the wheel deserialises, from a property group (or any object with the same attributes).

    Keys are the ``VRMC_vrm.meta`` keys of the field table. ``authors`` is a comma separated string; an
    empty name or author list takes the fallback so the two required fields never block an export, and
    an empty copyright takes ``default_copyright``. Empty texts are left out so the wheel's defaults apply."""
    fields = VRM_META_FIELDS if fields is None else fields
    authors = [author.strip() for author in meta.authors.split(",") if author.strip()]
    if not authors and fallback_author.strip():
        authors = [fallback_author.strip()]
    result = {
        "name": meta.name.strip() or fallback_name.strip(),
        "authors": authors,
    }
    for field in fields:
        value = getattr(meta, field["name"])
        if field["kind"] == "text":
            if value.strip():
                result[field["key"]] = value.strip()
        elif field["kind"] == "flag":
            result[field["key"]] = bool(value)
        else:
            result[field["key"]] = value
    copyright = result.get("copyrightInformation") or default_copyright(authors)
    if copyright:
        result["copyrightInformation"] = copyright
    return result
