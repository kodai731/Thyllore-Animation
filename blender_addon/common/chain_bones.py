"""Pure Python helpers for chain bone generation output.

No bpy imports: these functions work on the data returned by the wheel's
generate_chains call (engine coordinates, Y-up metres) and are consumed by the
addon's bpy operator, which converts to Blender space with ``coordinates``.
"""
from __future__ import annotations

from .coordinates import engine_metres_to_blender

VertexGroupWeights = list[tuple[int, list[tuple[int, float]]]]


class VertexGroupNames:
    """The wheel's ``vertex_group_names()``: the covered group and the prefix of the chain membership groups."""

    def __init__(self, names: dict):
        self.covered: str = names["covered"]
        self._membership_prefix: str = names["membership_prefix"]

    def membership(self, chain_name: str) -> str:
        """The vertex group that holds the vertices the user assigned to the chain."""
        return self._membership_prefix + chain_name

    def chain_of(self, group_name: str) -> str | None:
        """The chain a membership group belongs to, or ``None`` for any other vertex group."""
        if not group_name.startswith(self._membership_prefix):
            return None
        return group_name[len(self._membership_prefix) :]


def group_vertices(vertex_groups: VertexGroupWeights, group_index: int) -> list[int]:
    """The vertices weighted into a group: ``vertex_groups`` is ``[(vertex_index, [(group, weight), ...]), ...]``."""
    return [
        vertex
        for vertex, groups in vertex_groups
        if any(group == group_index and weight > 0.0 for group, weight in groups)
    ]


def given_chains_from_groups(
    vertex_groups: VertexGroupWeights, groups: list[tuple[str, int]], names: VertexGroupNames
) -> list[dict]:
    """The ``{name, vertices}`` of every membership group with a weighted vertex; ``groups`` is ``[(name, index), ...]``."""
    given = []
    for group_name, group_index in groups:
        chain = names.chain_of(group_name)
        if chain is None:
            continue
        vertices = group_vertices(vertex_groups, group_index)
        if vertices:
            given.append({"name": chain, "vertices": vertices})
    return given


def vertex_tex_coords(
    loop_vertices: list[int],
    loop_uvs: list[tuple[float, float]],
    vertex_count: int,
) -> list[tuple[float, float]]:
    """Per-vertex UV from mesh loops; each vertex gets its first loop occurrence's UV."""
    first_uvs: list[tuple[float, float] | None] = [None] * vertex_count
    for v_idx, uv in zip(loop_vertices, loop_uvs):
        if first_uvs[v_idx] is None:
            first_uvs[v_idx] = uv
    return [uv if uv is not None else (0.0, 0.0) for uv in first_uvs]


Point = tuple[float, float, float]


def joints_from_bones(bones: list[tuple[Point, Point]]) -> list[Point]:
    """The joint heads followed by the chain tail, from the ``(head, tail)`` of each chain bone root to tip."""
    return [head for head, _ in bones] + [bones[-1][1]]


def output_of_new_chains(output: dict, read_back: set[str]) -> dict:
    """The same output without the chains whose bones were read back from the armature."""
    return {**output, "chains": [chain for chain in output["chains"] if chain["name"] not in read_back]}


def mismatched_vertices(
    chains: list[dict], weights: VertexGroupWeights, group_names: dict[int, str], new_chains: set[str]
) -> set[int]:
    """The vertices whose bone weights disagree with their chain membership: a member of a chain that has no
    weight on that chain's joints, that has weight on another chain's joints, or whose chain was placed now;
    and a vertex of no chain that is still weighted to a chain joint. Root and body bone weights do not count."""
    chain_of_joint = {joint["name"]: chain["name"] for chain in chains for joint in chain["joints"]}
    chain_of_vertex = {int(vertex): chain["name"] for chain in chains for vertex in chain["vertices"]}
    mismatched = set()
    for vertex, groups in weights:
        weighted = {chain_of_joint[group_names[g]] for g, w in groups if w > 0.0 and group_names.get(g) in chain_of_joint}
        own = chain_of_vertex.get(vertex)
        if own is None:
            if weighted:
                mismatched.add(vertex)
        elif own in new_chains or weighted != {own}:
            mismatched.add(vertex)
    return mismatched


def output_to_blender(output: dict, scale_length: float = 1.0) -> dict:
    """The same output with every joint head and chain tail in Blender space and Blender units."""

    def to_blender(point):
        return engine_metres_to_blender(point, scale_length)

    chains = [
        {
            **chain,
            "joints": [{**joint, "head": to_blender(joint["head"])} for joint in chain["joints"]],
            "tail": to_blender(chain["tail"]),
        }
        for chain in output["chains"]
    ]
    return {**output, "chains": chains}


def settings_from_properties(ui_params: list[dict], owner) -> dict[str, float]:
    """Read every setting of the table from the attributes of ``owner`` (an operator or property group)."""
    return {param["name"]: float(getattr(owner, param["name"])) for param in ui_params}


def bone_specs(
    output: dict, root_bone: str
) -> list[tuple[str, str | None, tuple[float, float, float], tuple[float, float, float]]]:
    """Build (name, parent, head, tail) specs for every joint in every chain, then its end bone.

    Within a chain each joint's tail is the next joint's head; the last joint's
    tail is the chain's ``tail``, where the chain's ``end_bone`` starts and
    continues the last segment.  The first joint's parent is ``root_bone``,
    subsequent joints' parent is the preceding joint's name.
    """
    specs: list[tuple[str, str | None, tuple[float, float, float], tuple[float, float, float]]] = []
    for chain in output["chains"]:
        joints = chain["joints"]
        prev_name: str | None = root_bone
        for i, joint in enumerate(joints):
            name = joint["name"]
            head = tuple(joint["head"])
            if i + 1 < len(joints):
                tail = tuple(joints[i + 1]["head"])
            else:
                tail = tuple(chain["tail"])
            specs.append((name, prev_name, head, tail))
            prev_name = name
        if joints:
            specs.append((chain["end_bone"], prev_name, *_end_bone_segment(joints[-1]["head"], chain["tail"])))
    return specs


def _end_bone_segment(last_head, tail) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    """The end bone spans the chain tail onward by the last segment, so Blender gives it a length."""
    head = tuple(tail)
    return head, tuple(t + (t - h) for h, t in zip(last_head, tail))


def _target_bone_name(target, root_bone: str, bone_names: list[str]) -> str:
    """``"root"`` is the root bone, ``{"bone": index}`` a skin bone of the armature, anything else a joint name."""
    if isinstance(target, dict):
        return bone_names[int(target["bone"])]
    if target == "root":
        return root_bone
    return target


def group_weights(
    output: dict, root_bone: str, bone_names: list[str] = ()
) -> dict[str, list[tuple[int, float]]]:
    """Group vertex influences by bone name.

    Returns ``{bone_name: [(vertex_index, weight), ...]}``.  ``bone_names`` lists
    the armature bones so a ``{"bone": index}`` target of a covered garment vertex
    resolves to its name.
    """
    groups: dict[str, list[tuple[int, float]]] = {}
    for entry in output["weights"]:
        vertex = int(entry["vertex"])
        for target, weight in entry["influences"]:
            bone_name = _target_bone_name(target, root_bone, list(bone_names))
            groups.setdefault(bone_name, []).append((vertex, weight))
    return groups
