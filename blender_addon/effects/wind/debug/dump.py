from __future__ import annotations

import json
import os
import struct
import time

from .. import draw_handler
from .._common import coordinates
from ..properties import wind_render_params
from ..wind_shader import matrix_column_major

UBO_BLOCK_NAMES = ("shape", "wall", "optics", "albedo", "lighting", "streak", "streak2", "eddy", "eddy2", "puff_params")
UBO_MATRIX_FLOATS = 16
UBO_MAX_PUFFS = 96


def columns(matrix_rows):
    return [[matrix_rows[r][c] for r in range(4)] for c in range(4)]


def unpack_wind_ubo(ubo_bytes: bytes, inv_view_proj_rows) -> dict:
    values = struct.unpack(f"{len(ubo_bytes) // 4}f", ubo_bytes)
    offset = 2 * UBO_MATRIX_FLOATS
    record = {
        "model": [list(values[c * 4:c * 4 + 4]) for c in range(4)],
        "inverse_model": [list(values[16 + c * 4:16 + c * 4 + 4]) for c in range(4)],
    }
    for name in UBO_BLOCK_NAMES:
        record[name] = list(values[offset:offset + 4])
        offset += 4
    puff_count = int(record["puff_params"][0])
    record["puffs"] = [list(values[offset + i * 4:offset + i * 4 + 4]) for i in range(puff_count)]
    offset += 4 * UBO_MAX_PUFFS
    record["inv_view_proj"] = [list(values[offset + c * 4:offset + c * 4 + 4]) for c in range(4)]
    return record


def write_npy(rows, path):
    height, width = len(rows), len(rows[0])
    flat = [v for row in rows for px in row for v in px]
    header = "{'descr': '<f4', 'fortran_order': False, 'shape': (%d, %d, 4), }" % (height, width)
    header += " " * (64 - ((10 + len(header) + 1) % 64)) + "\n"
    with open(path, "wb") as f:
        f.write(b"\x93NUMPY" + bytes([1, 0]) + struct.pack("<H", len(header)) + header.encode("latin1"))
        f.write(struct.pack(f"<{len(flat)}f", *flat))


def write_png(rows, path):
    import bpy

    height, width = len(rows), len(rows[0])
    image = bpy.data.images.new("thyllore_wind_dump", width, height, alpha=True, float_buffer=True)
    image.pixels.foreach_set([v for row in rows for px in row for v in px])
    image.filepath_raw = path
    image.file_format = "PNG"
    image.save()
    bpy.data.images.remove(image)


def write_wind_debug_dump(context, out_dir: str) -> str:
    import thyllore_effect_core as fx

    scene = context.scene
    region = context.region
    region_data = context.region_data
    unix_time = int(time.time())

    view_matrix = list(region_data.view_matrix)
    window_matrix = list(region_data.window_matrix)
    proj = draw_handler.blender_window_to_engine_projection(window_matrix, draw_handler.VIEWPORT_NEAR)
    view, camera_pos = draw_handler.blender_view_to_engine_view(view_matrix)
    light_pos = draw_handler.find_light_position(scene)
    scene_time = draw_handler.scene_time_seconds(scene)

    instances = []
    for index, obj in enumerate(draw_handler.find_wind_objects(scene)):
        params = wind_render_params(obj.thyllore_wind)
        position = coordinates.blender_to_engine_point(obj.matrix_world.translation)
        rotation = coordinates.blender_to_engine_quaternion(obj.matrix_world.to_quaternion())
        ubo_bytes = fx.pack_wind_ubo(params, scene_time, position, rotation, matrix_column_major(view), matrix_column_major(proj))
        effect = dict(params)
        effect.update({"time": scene_time, "time_scale": 1.0, "time_offset": 0.0, "position": list(position), "rotation": list(rotation)})
        instances.append({
            "instance_index": index,
            "name": obj.name,
            "applied_preset": obj.thyllore_wind.preset,
            "effect": effect,
            "ubo": unpack_wind_ubo(ubo_bytes, None),
        })
    if not instances:
        raise RuntimeError("no wind object in the scene")

    renderer = next(iter(draw_handler._renderers.values()), None)
    texture_paths = {}
    if renderer is not None and renderer.resolved is not None:
        rows = renderer.resolved.read().to_list()
        texture_paths["png"] = os.path.join(out_dir, f"wind_debug_{unix_time}.png")
        texture_paths["npy"] = os.path.join(out_dir, f"wind_debug_{unix_time}.npy")
        write_png(rows, texture_paths["png"])
        write_npy(rows, texture_paths["npy"])

    record = {
        "unix_time": unix_time,
        "host": "blender",
        "screenshot_path": texture_paths.get("png"),
        "texture_npy_path": texture_paths.get("npy"),
        "scene": {"frame_current": scene.frame_current, "frame_start": scene.frame_start, "fps": scene.render.fps},
        "camera": {"position": list(camera_pos), "near_plane": draw_handler.VIEWPORT_NEAR},
        "projection": {
            "view": columns(view),
            "proj": columns(proj),
            "screen_size": [float(region.width), float(region.height)],
            "aspect": region.width / max(region.height, 1),
        },
        "light": {"position": list(light_pos)},
        "wind_render_settings": {"resolve_divisor": fx.wind_resolve_divisor(), "exposure": draw_handler.ENGINE_EXPOSURE},
        "wind_instances": instances,
    }
    json_path = os.path.join(out_dir, f"wind_debug_{unix_time}.json")
    with open(json_path, "w") as f:
        json.dump(record, f, indent=2)
    return json_path
