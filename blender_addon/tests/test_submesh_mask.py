"""Tests for submesh_mask module: material_triangles, to_loop_triangle_indices, mask_mesh_input."""
from __future__ import annotations

from blender_addon.common.submesh_mask import (
    mask_mesh_input,
    material_loop_indices,
    material_triangles,
    to_loop_triangle_indices,
)


def test_material_triangles_single_material():
    """All triangles belong to one material."""
    vertices = [0, 1, 2, 3, 4, 5]
    materials = [0, 0]

    triangles, indices = material_triangles(vertices, materials, 0)

    assert triangles == [[0, 1, 2], [3, 4, 5]]
    assert indices == [0, 1]


def test_material_triangles_two_materials():
    """Triangles split between two materials."""
    vertices = [0, 1, 2, 3, 4, 5, 6, 7, 8]
    materials = [0, 1, 0]

    triangles_0, indices_0 = material_triangles(vertices, materials, 0)
    triangles_1, indices_1 = material_triangles(vertices, materials, 1)

    assert triangles_0 == [[0, 1, 2], [6, 7, 8]]
    assert indices_0 == [0, 2]
    assert triangles_1 == [[3, 4, 5]]
    assert indices_1 == [1]


def test_material_triangles_empty():
    """No triangles for the given material."""
    vertices = [0, 1, 2]
    materials = [0]

    triangles, indices = material_triangles(vertices, materials, 1)

    assert triangles == []
    assert indices == []


def test_to_loop_triangle_indices():
    """Map local indices back to loop_triangles indices."""
    hidden_local = [0, 2]
    loop_indices = [3, 5, 7, 9]

    result = to_loop_triangle_indices(hidden_local, loop_indices)

    assert result == [3, 7]


def test_to_loop_triangle_indices_empty():
    """Empty hidden list returns empty."""
    result = to_loop_triangle_indices([], [0, 1, 2])
    assert result == []


def test_mask_mesh_input_basic():
    """4 vertices, 2 triangles (material 0 and 1), material 1 selected.

    - positions / normals have 4 entries (no duplication).
    - triangles has only material 1's 1 triangle.
    - loop index is [1].
    - Blender (0,0,1) becomes engine (0,1,0) * scale_length=2 = (0,2,0).
    - dict has exactly 3 keys."""
    vertex_positions = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 1.0)]
    vertex_normals = [(0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0)]
    triangle_vertices = [0, 1, 2, 1, 3, 2]
    triangle_materials = [0, 1]

    result, loop_indices = mask_mesh_input(
        vertex_positions, vertex_normals, triangle_vertices, triangle_materials, material_index=1, scale_length=2.0
    )

    assert len(result["positions"]) == 4
    assert result["positions"][2] == (0.0, 2.0, 0.0)

    assert len(result["normals"]) == 4
    assert result["normals"][0] == (0.0, 1.0, 0.0)

    assert result["triangles"] == [[1, 3, 2]]

    assert loop_indices == [1]

    assert set(result.keys()) == {"positions", "normals", "triangles"}


def test_mask_mesh_input_two_materials():
    """Same mesh, material 0 selected — should get the first triangle only."""
    vertex_positions = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 1.0), (1.0, 0.0, 1.0)]
    vertex_normals = [(0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0)]
    triangle_vertices = [0, 1, 2, 1, 3, 2]
    triangle_materials = [0, 1]

    result, loop_indices = mask_mesh_input(
        vertex_positions, vertex_normals, triangle_vertices, triangle_materials, material_index=0, scale_length=1.0
    )

    assert len(result["positions"]) == 4
    assert result["positions"][0] == (0.0, 0.0, 0.0)

    assert result["triangles"] == [[0, 1, 2]]

    assert loop_indices == [0]


def test_mask_mesh_input_empty():
    """Material index with no triangles returns empty triangles and indices."""
    vertex_positions = [(0.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.0, 1.0)]
    vertex_normals = [(0.0, 0.0, 1.0), (0.0, 0.0, 1.0), (0.0, 0.0, 1.0)]
    triangle_vertices = [0, 1, 2]
    triangle_materials = [0]

    result, loop_indices = mask_mesh_input(
        vertex_positions, vertex_normals, triangle_vertices, triangle_materials, material_index=5, scale_length=1.0
    )

    assert len(result["positions"]) == 3
    assert result["triangles"] == []
    assert loop_indices == []


def test_mask_mesh_input_feeds_wheel():
    """mask_mesh_input output dict can be fed to wheel's find_hidden_triangles.

    A flat body plate at z=0 with normals pointing +z, and a garment plate
    directly above it at z=0.1 covering the same area. The body triangles should
    be detected as hidden."""
    import sys
    import zipfile
    from pathlib import Path

    wheels_dir = Path(__file__).resolve().parents[1] / "wheels"
    wheels = sorted(wheels_dir.glob("thyllore_ml_core-*.whl"))
    if not wheels:
        pytest.skip("thyllore_ml_core wheel not built (run scripts/collect_wheels.sh)")

    import tempfile

    extract_dir = tempfile.mkdtemp()
    with zipfile.ZipFile(wheels[-1]) as archive:
        archive.extractall(extract_dir)

    sys.path.insert(0, extract_dir)
    try:
        import thyllore_ml_core as tml

        body_positions = [
            (-0.5, 0.0, -0.5),
            (0.5, 0.0, -0.5),
            (0.5, 0.0, 0.5),
            (-0.5, 0.0, 0.5),
        ]
        body_normals = [(0.0, 1.0, 0.0)] * 4
        body_triangles = [0, 1, 2, 0, 2, 3]

        garment_positions = [
            (-1.0, 0.05, -1.0),
            (1.0, 0.05, -1.0),
            (1.0, 0.05, 1.0),
            (-1.0, 0.05, 1.0),
        ]
        garment_normals = [(0.0, 1.0, 0.0)] * 4
        garment_triangles = [0, 1, 2, 0, 2, 3]

        body_input, _ = mask_mesh_input(body_positions, body_normals, body_triangles, [0, 0], 0, 1.0)
        garment_input, _ = mask_mesh_input(garment_positions, garment_normals, garment_triangles, [0, 0], 0, 1.0)

        result = tml.find_hidden_triangles(body_input, [garment_input], tml.hidden_mask_defaults())

        assert len(result) == 2
    finally:
        sys.path.remove(extract_dir)


def test_material_loop_indices_groups_triangles_by_material_name():
    """Each material lists its loop_triangles indices in order; empty slots are dropped."""
    by_name = material_loop_indices([0, 1, 0, 2], ["Skin", "Cloth", None])

    assert by_name == {"Skin": [0, 2], "Cloth": [1]}
