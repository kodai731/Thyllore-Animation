"""Pure helpers for the VRM export: the meta dict the wheel takes and the T-pose check (bpy-independent)."""
from __future__ import annotations

import datetime
import getpass
import math

VRM_META_ENUMS = {
    "avatar_permission": (
        ("onlyAuthor", "Only Author", ""),
        ("onlySeparatelyLicensedPerson", "Separately Licensed Person", ""),
        ("everyone", "Everyone", ""),
    ),
    "commercial_usage": (
        ("personalNonProfit", "Personal Non-Profit", ""),
        ("personalProfit", "Personal Profit", ""),
        ("corporation", "Corporation", ""),
    ),
    "credit_notation": (
        ("required", "Required", ""),
        ("unnecessary", "Unnecessary", ""),
    ),
    "modification": (
        ("prohibited", "Prohibited", ""),
        ("allowModification", "Allow Modification", ""),
        ("allowModificationRedistribution", "Allow Modification And Redistribution", ""),
    ),
}

VRM_META_FLAGS = (
    "allow_excessively_violent_usage",
    "allow_excessively_sexual_usage",
    "allow_political_or_religious_usage",
    "allow_antisocial_or_hate_usage",
    "allow_redistribution",
)

VRM_META_TEXTS = ("version", "copyright_information", "contact_information", "other_license_url")

VRM_META_LABELS = {
    "version": "Version",
    "copyright_information": "Copyright",
    "contact_information": "Contact",
    "other_license_url": "Other License",
    "avatar_permission": "Permission",
    "commercial_usage": "Commercial",
    "credit_notation": "Credit",
    "modification": "Modification",
    "allow_excessively_violent_usage": "Violent Usage",
    "allow_excessively_sexual_usage": "Sexual Usage",
    "allow_political_or_religious_usage": "Political Or Religious Usage",
    "allow_antisocial_or_hate_usage": "Antisocial Or Hate Usage",
    "allow_redistribution": "Redistribution",
}

VRM_META_DEFAULT_VERSION = "1.0"


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


def _camel_case(field: str) -> str:
    head, *rest = field.split("_")
    return head + "".join(word.title() for word in rest)


def vrm_meta_to_dict(meta, fallback_name: str = "", fallback_author: str = "") -> dict:
    """The ``VrmMeta`` the wheel deserialises, from a property group (or any object with the same attributes).

    Keys are the camelCase names of ``VRMC_vrm.meta``, which is what the wheel accepts.
    ``authors`` is a comma separated string; an empty name or author list takes the fallback so the
    two required fields never block an export, and an empty copyright takes ``default_copyright``.
    Empty optional texts are left out so the wheel's defaults apply."""
    authors = [author.strip() for author in meta.authors.split(",") if author.strip()]
    if not authors and fallback_author.strip():
        authors = [fallback_author.strip()]
    result = {
        "name": meta.name.strip() or fallback_name.strip(),
        "authors": authors,
    }
    for field in VRM_META_TEXTS:
        value = getattr(meta, field).strip()
        if value:
            result[_camel_case(field)] = value
    copyright = result.get("copyrightInformation") or default_copyright(authors)
    if copyright:
        result["copyrightInformation"] = copyright
    for field in VRM_META_ENUMS:
        result[_camel_case(field)] = getattr(meta, field)
    for field in VRM_META_FLAGS:
        result[_camel_case(field)] = bool(getattr(meta, field))
    return result


T_POSE_LIMBS = (
    ("LeftUpperArm", "LeftHand", "horizontal"),
    ("RightUpperArm", "RightHand", "horizontal"),
    ("LeftUpperLeg", "LeftFoot", "down"),
    ("RightUpperLeg", "RightFoot", "down"),
)

T_POSE_TOLERANCE_DEG = 15.0


def t_pose_warnings(humanoid: dict, heads: dict, tolerance_deg: float = T_POSE_TOLERANCE_DEG) -> list:
    """Limbs whose rest direction is not the VRM T-pose: arms level, legs straight down (engine axes, Y up).

    ``humanoid`` maps Unity role names to bone names, ``heads`` bone names to rest head positions."""
    warnings = []
    for root_role, tip_role, expected in T_POSE_LIMBS:
        root = heads.get(humanoid.get(root_role))
        tip = heads.get(humanoid.get(tip_role))
        if root is None or tip is None:
            continue
        direction = (tip[0] - root[0], tip[1] - root[1], tip[2] - root[2])
        length = math.sqrt(sum(c * c for c in direction))
        if length < 1e-6:
            continue
        unit_y = direction[1] / length
        if expected == "horizontal":
            deviation_deg = math.degrees(math.asin(max(-1.0, min(1.0, unit_y))))
            if abs(deviation_deg) > tolerance_deg:
                warnings.append(f"{root_role}: arm is {abs(deviation_deg):.0f}° from level (VRM expects a T-pose)")
        else:
            deviation_deg = math.degrees(math.acos(max(-1.0, min(1.0, -unit_y))))
            if deviation_deg > tolerance_deg:
                warnings.append(f"{root_role}: leg is {deviation_deg:.0f}° from straight down (VRM expects a T-pose)")
    return warnings
