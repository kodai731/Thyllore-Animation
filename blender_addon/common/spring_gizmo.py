"""Pure-geometry spring-bone gizmo line segment generators.

Every function returns a list of (start, end) pairs where each point is an
(f32, f32, f32) tuple — the wireframe lines to draw. No bpy dependency."""

from __future__ import annotations

import math


def wire_sphere_segments(
    center: tuple[float, float, float],
    radius: float,
    segments: int,
) -> list[tuple[tuple[float, float, float], tuple[float, float, float]]]:
    """Wireframe sphere as 3 circles (XY, YZ, ZX planes), each with `segments` line segments."""
    seg = max(segments, 4)
    lines: list[tuple[tuple[float, float, float], tuple[float, float, float]]] = []

    for plane in _PLANES:
        for i in range(seg):
            a0 = i / seg * math.tau
            a1 = (i + 1) / seg * math.tau
            lines.append((
                _circle_point(center, radius, a0, plane),
                _circle_point(center, radius, a1, plane),
            ))

    return lines


def capsule_segments(
    head: tuple[float, float, float],
    tail: tuple[float, float, float],
    radius: float,
    segments: int,
) -> list[tuple[tuple[float, float, float], tuple[float, float, float]]]:
    """Wireframe capsule: wire spheres at both ends + 4 lines parallel to the axis.

    When head == tail (distance < 1e-6), returns a single wire sphere."""
    dx = tail[0] - head[0]
    dy = tail[1] - head[1]
    dz = tail[2] - head[2]
    length = math.sqrt(dx * dx + dy * dy + dz * dz)

    if length < 1e-6:
        return wire_sphere_segments(head, radius, segments)

    lines: list[tuple[tuple[float, float, float], tuple[float, float, float]]] = []
    lines.extend(wire_sphere_segments(head, radius, segments))
    lines.extend(wire_sphere_segments(tail, radius, segments))

    dir_x, dir_y, dir_z = dx / length, dy / length, dz / length
    right, up = _perpendicular_axes(dir_x, dir_y, dir_z)

    offsets = [
        (right[0] * radius, right[1] * radius, right[2] * radius),
        (-right[0] * radius, -right[1] * radius, -right[2] * radius),
        (up[0] * radius, up[1] * radius, up[2] * radius),
        (-up[0] * radius, -up[1] * radius, -up[2] * radius),
    ]

    for ox, oy, oz in offsets:
        lines.append((
            (head[0] + ox, head[1] + oy, head[2] + oz),
            (tail[0] + ox, tail[1] + oy, tail[2] + oz),
        ))

    return lines


def _circle_point(
    center: tuple[float, float, float],
    radius: float,
    angle: float,
    plane: tuple[int, int],
) -> tuple[float, float, float]:
    """One point on a circle in the given plane."""
    c = list(center)
    cos_a, sin_a = math.cos(angle), math.sin(angle)
    i, j = plane
    c[i] += cos_a * radius
    c[j] += sin_a * radius
    return tuple(c)


def _perpendicular_axes(dx: float, dy: float, dz: float) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    """Two unit vectors perpendicular to the direction (dx, dy, dz)."""
    up_hint = (0.0, 1.0, 0.0) if abs(dy) < 0.99 else (1.0, 0.0, 0.0)
    right = _normalize(_cross((dx, dy, dz), up_hint))
    up = _normalize(_cross((dx, dy, dz), right))
    return right, up


def _cross(a: tuple[float, float, float], b: tuple[float, float, float]) -> tuple[float, float, float]:
    return (
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    )


def _normalize(v: tuple[float, float, float]) -> tuple[float, float, float]:
    length = math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
    if length < 1e-10:
        return (0.0, 0.0, 0.0)
    inv = 1.0 / length
    return (v[0] * inv, v[1] * inv, v[2] * inv)


_PLANES: tuple[tuple[int, int], ...] = ((0, 1), (1, 2), (2, 0))
