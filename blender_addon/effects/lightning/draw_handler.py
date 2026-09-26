import math
import time
import traceback

from ._common import coordinates
from .lightning_shader import (
    build_lightning_shader,
    build_tonemap_composite_shader,
    matrix_column_major,
    pack_frame_ubo,
)
from .viewport_depth import ViewportDepthCapture

VIEWPORT_NEAR = 0.1
ENGINE_EXPOSURE = 1.0
DISPLAY_ENCODE_SRGB = 1.0


def flip_projection_y(proj) -> list:
    return [proj[0], [-v for v in proj[1]], proj[2], proj[3]]


_depth_handle = None
_draw_handle = None
_cached_shader = None
_renderers: dict[str, "LightningViewportRenderer"] = {}
_viewport_depth = ViewportDepthCapture()
_scene_depth = None
_composite_shader = None
_draw_failure_reported = False
_draw_diagnostic_reported = False


def _load_shader():
    global _cached_shader
    if _cached_shader is not None:
        return _cached_shader
    from pathlib import Path

    root = Path(__file__).resolve().parent
    glsl_path = str(root / "shaders" / "lightning_resolve.glsl")
    bindings_path = str(root / "shaders" / "lightning_resolve.bindings.json")
    started = time.perf_counter()
    shader = build_lightning_shader(glsl_path, bindings_path)
    print(f"[Thyllore Lightning] shader built in {time.perf_counter() - started:.2f}s", flush=True)
    _cached_shader = shader
    return shader


def blender_window_to_engine_projection(window_matrix, near):
    f_y = window_matrix[1][1]
    f_x = window_matrix[0][0]
    fovy = 2.0 * math.atan(1.0 / f_y)
    aspect = f_y / f_x
    return coordinates.engine_projection(fovy, aspect, near)


def blender_view_to_engine_view(view_matrix):
    inv = coordinates.mat4_inverse(view_matrix)
    if inv is None:
        identity = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
        return identity, (0.0, 0.0, 0.0)
    camera_world_blender = inv
    camera_world_engine = coordinates.blender_camera_to_engine_matrix(camera_world_blender)
    view = coordinates.engine_view_matrix(camera_world_engine)
    camera_pos = (camera_world_engine[0][3], camera_world_engine[1][3], camera_world_engine[2][3])
    return view, camera_pos


def update_uniform_buffer(buffer, data):
    import gpu

    if buffer is None:
        return gpu.types.GPUUniformBuf(data)
    buffer.update(data)
    return buffer


class LightningViewportRenderer:

    def __init__(self):
        self.shader = None
        self.batch = None
        self.frame_ubo = None
        self.lightning_ubo = None
        self.segments_ubo = None
        self.resolved = None
        self.fb_resolved = None
        self._w = 0
        self._h = 0

    def ensure_shader(self):
        from gpu_extras.batch import batch_for_shader

        if self.shader is not None:
            return
        self.shader = _load_shader()
        self.batch = batch_for_shader(self.shader, "TRIS", {"pos": [(-1.0, -1.0), (3.0, -1.0), (-1.0, 3.0)]})

    def ensure_size(self, w, h):
        if self._w == w and self._h == h:
            return
        self._w = w
        self._h = h
        import gpu

        self.resolved = gpu.types.GPUTexture((w, h), format="RGBA32F")
        self.fb_resolved = gpu.types.GPUFrameBuffer(color_slots=(self.resolved,))

    def clear_color(self):
        with self.fb_resolved.bind():
            self.fb_resolved.clear(color=(0.0, 0.0, 0.0, 0.0))

    def render(self, view, proj, camera_pos, params, time, position, rotation, w, h, depth_tex=None, flip_y=True, debug_view: int = 0, waypoints=()):
        import gpu
        import thyllore_effect_core as fx

        self.ensure_size(w, h)
        self.ensure_shader()
        if flip_y:
            proj = flip_projection_y(proj)

        frame_bytes = pack_frame_ubo(view, proj, camera_pos + (1.0,), camera_pos + (1.0,), (1.0, 1.0, 1.0, 1.0))
        view_cm = matrix_column_major(view)
        proj_cm = matrix_column_major(proj)
        ubo_bytes, segments_bytes, segment_count = fx.pack_lightning_ubo(params, time, position, rotation, view_cm, proj_cm, waypoints=list(waypoints))

        self.frame_ubo = update_uniform_buffer(self.frame_ubo, frame_bytes)
        self.lightning_ubo = update_uniform_buffer(self.lightning_ubo, ubo_bytes)
        self.segments_ubo = update_uniform_buffer(self.segments_ubo, segments_bytes)

        if depth_tex is None:
            return self.resolved

        self.clear_color()

        with self.fb_resolved.bind():
            gpu.state.blend_set("ADDITIVE")
            try:
                self.shader.bind()
                self.shader.uniform_block("frame", self.frame_ubo)
                self.shader.uniform_block("lightning", self.lightning_ubo)
                self.shader.uniform_block("segments", self.segments_ubo)
                self.shader.uniform_int("debugView", [debug_view])
                self.shader.uniform_int("shadingMode", [0])
                self.shader.uniform_int("stepCount", [0])
                self.shader.uniform_sampler("sceneDepthSampler", depth_tex)
                self.batch.draw(self.shader)
            finally:
                gpu.state.blend_set("NONE")

        return self.resolved

    def release(self):
        for attr in ("frame_ubo", "lightning_ubo", "segments_ubo", "resolved", "fb_resolved"):
            setattr(self, attr, None)


def capture_scene_depth():
    global _scene_depth
    import bpy

    region = bpy.context.region
    window_matrix = list(bpy.context.region_data.window_matrix)
    _scene_depth = _viewport_depth.capture(region.width, region.height, window_matrix, VIEWPORT_NEAR)


def draw_viewport():
    global _draw_failure_reported
    try:
        draw_lightning()
    except Exception:
        if not _draw_failure_reported:
            _draw_failure_reported = True
            print("[Thyllore Lightning] viewport draw failed:\n" + traceback.format_exc(), flush=True)


def scene_time_seconds(scene):
    return (scene.frame_current - scene.frame_start) / scene.render.fps


def find_lightning_objects(scene):
    return [obj for obj in scene.objects if hasattr(obj, "thyllore_lightning") and obj.thyllore_lightning.is_lightning]


def draw_lightning():
    import bpy

    from .properties import lightning_render_params, waypoint_local_points

    context = bpy.context
    region = context.region
    region_data = context.region_data
    view_matrix = list(region_data.view_matrix)
    window_matrix = list(region_data.window_matrix)
    proj = blender_window_to_engine_projection(window_matrix, VIEWPORT_NEAR)
    view, camera_pos = blender_view_to_engine_view(view_matrix)
    w = region.width
    h = region.height

    scene = context.scene
    scene_time = scene_time_seconds(scene)
    lightning_objects = find_lightning_objects(scene)

    rendered_colors = []
    render_started = time.perf_counter()
    for obj in lightning_objects:
        renderer = _renderers.setdefault(obj.name, LightningViewportRenderer())
        params = lightning_render_params(obj)
        position = coordinates.blender_to_engine_point(obj.matrix_world.translation)
        rotation = coordinates.blender_to_engine_quaternion(obj.matrix_world.to_quaternion())
        waypoints = [coordinates.blender_to_engine_point(p) for p in waypoint_local_points(obj)]
        rendered_colors.append(renderer.render(
            view, proj, camera_pos, params, scene_time, position, rotation, w, h, depth_tex=_scene_depth, waypoints=waypoints
        ))

    report_first_draw(w, h, camera_pos, lightning_objects, time.perf_counter() - render_started)

    for color in rendered_colors:
        composite_tonemapped(color, w, h)


def composite_tonemapped(color_tex, w, h):
    global _composite_shader
    import gpu
    from gpu_extras.batch import batch_for_shader

    if _composite_shader is None:
        _composite_shader = build_tonemap_composite_shader()
    batch = batch_for_shader(
        _composite_shader, "TRI_FAN",
        {"pos": [(0, 0), (w, 0), (w, h), (0, h)], "texCoord": [(0, 0), (1, 0), (1, 1), (0, 1)]},
    )
    previous_blend = gpu.state.blend_get()
    previous_depth_test = gpu.state.depth_test_get()
    try:
        gpu.state.blend_set("ADDITIVE")
        gpu.state.depth_test_set("NONE")
        _composite_shader.bind()
        _composite_shader.uniform_float("ModelViewProjectionMatrix", gpu.matrix.get_projection_matrix() @ gpu.matrix.get_model_view_matrix())
        _composite_shader.uniform_float("tonemapParams", (ENGINE_EXPOSURE, DISPLAY_ENCODE_SRGB))
        _composite_shader.uniform_sampler("image", color_tex)
        batch.draw(_composite_shader)
    finally:
        gpu.state.depth_test_set(previous_depth_test)
        gpu.state.blend_set(previous_blend)


def report_first_draw(w, h, camera_pos, lightning_objects, render_seconds):
    global _draw_diagnostic_reported
    if _draw_diagnostic_reported:
        return
    _draw_diagnostic_reported = True
    positions = [tuple(round(v, 3) for v in coordinates.blender_to_engine_point(o.matrix_world.translation)) for o in lightning_objects]
    print(
        f"[Thyllore Lightning] first draw: region={w}x{h} lights={len(lightning_objects)} "
        f"camera_engine={tuple(round(v, 3) for v in camera_pos)} lightning_engine={positions} render={render_seconds:.2f}s",
        flush=True,
    )


def register_draw_handler():
    global _depth_handle, _draw_handle
    import bpy

    _depth_handle = bpy.types.SpaceView3D.draw_handler_add(capture_scene_depth, (), "WINDOW", "POST_VIEW")
    _draw_handle = bpy.types.SpaceView3D.draw_handler_add(draw_viewport, (), "WINDOW", "POST_PIXEL")


def unregister_draw_handler():
    global _depth_handle, _draw_handle, _scene_depth, _composite_shader, _cached_shader
    import bpy

    for handle in (_draw_handle, _depth_handle):
        if handle is not None:
            bpy.types.SpaceView3D.draw_handler_remove(handle, "WINDOW")
    _depth_handle = _draw_handle = None
    for renderer in _renderers.values():
        renderer.release()
    _renderers.clear()
    _cached_shader = None
    _viewport_depth.release()
    _scene_depth = None
    _composite_shader = None
