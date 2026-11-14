"""Tests for vrm_export: the meta dict built from the wheel's field table."""
from __future__ import annotations

from types import SimpleNamespace

from blender_addon.common.vrm_export import default_copyright, vrm_meta_to_dict

FIELDS = [
    {"name": "version", "key": "version", "kind": "text"},
    {"name": "copyright_information", "key": "copyrightInformation", "kind": "text"},
    {"name": "contact_information", "key": "contactInformation", "kind": "text"},
    {"name": "other_license_url", "key": "otherLicenseUrl", "kind": "text"},
    {"name": "avatar_permission", "key": "avatarPermission", "kind": "choice"},
    {"name": "commercial_usage", "key": "commercialUsage", "kind": "choice"},
    {"name": "credit_notation", "key": "creditNotation", "kind": "choice"},
    {"name": "modification", "key": "modification", "kind": "choice"},
    {"name": "allow_excessively_violent_usage", "key": "allowExcessivelyViolentUsage", "kind": "flag"},
    {"name": "allow_excessively_sexual_usage", "key": "allowExcessivelySexualUsage", "kind": "flag"},
    {"name": "allow_political_or_religious_usage", "key": "allowPoliticalOrReligiousUsage", "kind": "flag"},
    {"name": "allow_antisocial_or_hate_usage", "key": "allowAntisocialOrHateUsage", "kind": "flag"},
    {"name": "allow_redistribution", "key": "allowRedistribution", "kind": "flag"},
]


def _meta(**overrides):
    fields = dict(
        name=" Avatar ",
        authors="alice, bob,",
        version="",
        copyright_information="",
        contact_information="",
        other_license_url="",
        avatar_permission="onlyAuthor",
        commercial_usage="personalNonProfit",
        credit_notation="required",
        modification="prohibited",
        allow_excessively_violent_usage=False,
        allow_excessively_sexual_usage=False,
        allow_political_or_religious_usage=False,
        allow_antisocial_or_hate_usage=False,
        allow_redistribution=False,
    )
    fields.update(overrides)
    return SimpleNamespace(**fields)


def test_meta_dict_splits_authors_and_drops_empty_texts():
    meta = vrm_meta_to_dict(_meta(), fields=FIELDS)

    assert meta["name"] == "Avatar"
    assert meta["authors"] == ["alice", "bob"]
    assert "version" not in meta
    assert "contactInformation" not in meta
    assert meta["avatarPermission"] == "onlyAuthor"
    assert meta["allowRedistribution"] is False


def test_meta_dict_takes_fallbacks_only_when_empty():
    meta = vrm_meta_to_dict(_meta(name=" ", authors=" , "), fallback_name="Rig", fallback_author="me", fields=FIELDS)
    assert meta["name"] == "Rig"
    assert meta["authors"] == ["me"]

    meta = vrm_meta_to_dict(_meta(), fallback_name="Rig", fallback_author="me", fields=FIELDS)
    assert meta["name"] == "Avatar"
    assert meta["authors"] == ["alice", "bob"]


def test_meta_dict_fills_copyright_from_authors():
    meta = vrm_meta_to_dict(_meta(), fields=FIELDS)
    assert meta["copyrightInformation"] == default_copyright(["alice", "bob"])

    meta = vrm_meta_to_dict(_meta(copyright_information="mine"), fields=FIELDS)
    assert meta["copyrightInformation"] == "mine"


def test_meta_dict_keeps_filled_texts():
    meta = vrm_meta_to_dict(_meta(version="1.2", contact_information="x@example.com"), fields=FIELDS)

    assert meta["version"] == "1.2"
    assert meta["contactInformation"] == "x@example.com"


def test_meta_dict_uses_the_vrmc_vrm_camel_case_keys():
    meta = vrm_meta_to_dict(_meta(modification="allowModification", allow_redistribution=True), fields=FIELDS)

    assert meta["modification"] == "allowModification"
    assert meta["allowRedistribution"] is True
    assert all("_" not in key for key in meta)
