"""Tests for blender_addon.common.spring_gizmo — pure geometry, no bpy."""

from __future__ import annotations

import math

from blender_addon.common.spring_gizmo import capsule_segments, wire_sphere_segments


def test_wire_sphere_segments_count():
    """segments=8 sphere has 3 circles * 8 = 24 segments."""
    segs = wire_sphere_segments((0.0, 0.0, 0.0), 1.0, 8)
    assert len(segs) == 24


def test_wire_sphere_circle_radius():
    """Every segment's endpoints lie on the sphere surface."""
    center = (1.0, 2.0, 3.0)
    radius = 0.5
    segs = wire_sphere_segments(center, radius, 8)
    for start, end in segs:
        for pt in (start, end):
            dist = math.sqrt(sum((pt[i] - center[i]) ** 2 for i in range(3)))
            assert abs(dist - radius) < 1e-6


def test_capsule_segments_degenerate_equals_sphere():
    """head == tail capsule returns exactly one wire sphere's segments."""
    pt = (0.0, 0.0, 0.0)
    segs = capsule_segments(pt, pt, 1.0, 8)
    sphere_segs = wire_sphere_segments(pt, 1.0, 8)
    assert len(segs) == len(sphere_segs)


def test_capsule_segments_count():
    """Non-degenerate capsule: 2 spheres (3 circles each * 8 segments) + 4 axis lines."""
    segs = capsule_segments((0.0, 0.0, 0.0), (2.0, 0.0, 0.0), 0.5, 8)
    assert len(segs) == 3 * 8 * 2 + 4


def test_four_joints_sphere_count():
    """4 joints each with a wire sphere (segments=8): 4 * 3 circles * 8 = 96 segments."""
    joints = [(i, 0.0, 0.0) for i in range(4)]
    total = sum(len(wire_sphere_segments(j, 0.1, 8)) for j in joints)
    assert total == 96
