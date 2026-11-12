"""Tests for blender_addon.common.skin_input on stand-ins for the bpy objects."""
from __future__ import annotations

from types import SimpleNamespace

from blender_addon.common.skin_input import skin_input, skin_material_indices


class _Bones:
    def __init__(self, names):
        self._names = list(names)

    def find(self, name):
        return self._names.index(name) if name in self._names else -1


class _Identity:
    def __matmul__(self, co):
        return co


def _mesh(roles, slots, triangles, vertex_groups, vertices):
    data = SimpleNamespace(
        loop_triangles=[SimpleNamespace(vertices=tri, material_index=mat) for tri, mat in triangles],
        vertices=[
            SimpleNamespace(index=i, co=(float(i), 0.0, 0.0), groups=[SimpleNamespace(group=g, weight=w) for g, w in groups])
            for i, groups in enumerate(vertices)
        ],
        calc_loop_triangles=lambda: None,
    )
    return SimpleNamespace(
        thyllore_submesh=SimpleNamespace(roles=[SimpleNamespace(material=m, role=r) for m, r in roles]),
        material_slots=[SimpleNamespace(material=SimpleNamespace(name=n)) for n in slots],
        vertex_groups=[SimpleNamespace(name=n) for n in vertex_groups],
        matrix_world=_Identity(),
        data=data,
    )


def test_skin_material_indices_follow_the_roles():
    mesh = _mesh([("Body", "skin"), ("Skirt", "garment")], ["Skirt", "Body"], [], [], [])
    assert skin_material_indices(mesh) == {1}


def test_only_skin_material_triangles_form_the_surface():
    mesh = _mesh(
        roles=[("Body", "skin"), ("Skirt", "garment")],
        slots=["Body", "Skirt"],
        triangles=[([0, 1, 2], 0), ([2, 3, 4], 1)],
        vertex_groups=["Hips", "chain_0"],
        vertices=[[(0, 1.0)], [(0, 1.0)], [(0, 0.5), (1, 0.5)], [(1, 1.0)], [(1, 1.0)]],
    )
    arm = SimpleNamespace(data=SimpleNamespace(bones=_Bones(["Hips", "chain_0"])))

    skin = skin_input(mesh, arm, 1.0)

    assert skin["triangles"] == [[0, 1, 2]]
    assert len(skin["positions"]) == 5
    assert skin["influences"][0] == [(0, 1.0)]
    assert skin["influences"][2] == [(0, 0.5), (1, 0.5)]
    assert skin["influences"][3] == []
    assert skin["influences"][4] == []


def test_collider_inputs_map_bone_names_and_keep_spheres_tailless():
    from blender_addon.common.skin_input import collider_inputs

    colliders = [
        {"bone": "Hips", "offset": [0.0, 1.0, 0.0], "tail": [0.0, 1.2, 0.0], "radius": 0.1},
        {"bone": "Head", "offset": [0.0, 1.6, 0.0], "tail": None, "radius": 0.09},
        {"bone": "missing", "offset": [0.0, 0.0, 0.0], "tail": None, "radius": 0.1},
    ]

    assert collider_inputs(colliders, {"Hips": 0, "Head": 5}) == [
        {"bone": 0, "head": [0.0, 1.0, 0.0], "tail": [0.0, 1.2, 0.0], "radius": 0.1},
        {"bone": 5, "head": [0.0, 1.6, 0.0], "tail": None, "radius": 0.09},
    ]
