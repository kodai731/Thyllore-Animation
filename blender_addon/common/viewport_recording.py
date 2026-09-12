"""Record the 3D viewport exactly as it is drawn, draw handlers included, into a video.

Viewport Render Animation renders offscreen without the Python draw handlers, so the
effects never appear in it. This steps the scene frame from a modal timer, screenshots the
viewport area once it has redrawn and encodes the frames with the sequencer of a
throwaway scene."""

from pathlib import Path

FRAME_FILE_PREFIX = "frame_"
FRAME_FILE_DIGITS = 4
FRAMES_DIRECTORY_SUFFIX = "_frames"
RECORDING_SCENE_NAME = "thyllore_viewport_recording"
DEFAULT_VIDEO_PATH = "//viewport_recording.mp4"
TIMER_INTERVAL_SECONDS = 0.05
REDRAW_TICKS_BEFORE_SHOT = 2


def frame_numbers(frame_start: int, frame_end: int, frame_step: int) -> list[int]:
    if frame_end < frame_start or frame_step < 1:
        return []
    return list(range(frame_start, frame_end + 1, frame_step))


def frame_file_name(index: int) -> str:
    return f"{FRAME_FILE_PREFIX}{index:0{FRAME_FILE_DIGITS}d}.png"


def frames_directory(video_path: str) -> Path:
    video = Path(video_path)
    return video.with_name(video.stem + FRAMES_DIRECTORY_SUFFIX)


def frame_paths(video_path: str, frame_count: int) -> list[Path]:
    directory = frames_directory(video_path)
    return [directory / frame_file_name(index) for index in range(1, frame_count + 1)]


def clear_frames_directory(directory: Path) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    for stale in directory.glob(f"{FRAME_FILE_PREFIX}*.png"):
        stale.unlink()


def find_window_region(area):
    return next((region for region in area.regions if region.type == "WINDOW"), None)


class HiddenSidebars:
    """Hides the sidebar and toolbar of one 3D view while recording and restores them after."""

    def __init__(self, space):
        self.space = space
        self.shown = (space.show_region_ui, space.show_region_toolbar)

    def hide(self):
        self.space.show_region_ui = False
        self.space.show_region_toolbar = False

    def restore(self):
        self.space.show_region_ui, self.space.show_region_toolbar = self.shown


class FrameCapture:
    """Screenshots one 3D view per frame, advanced by the operator's modal timer so the
    viewport redraws between setting the frame and reading the pixels."""

    def __init__(self, window, area, scene, frames: list[int], paths: list[Path]):
        self.window = window
        self.area = area
        self.region = find_window_region(area)
        self.scene = scene
        self.frames = frames
        self.paths = paths
        self.original_frame = scene.frame_current
        self.sidebars = HiddenSidebars(area.spaces.active)
        self.index = 0
        self.ticks_since_frame_set = 0

    def start(self):
        self.sidebars.hide()
        self.set_frame()

    def finished(self) -> bool:
        return self.index >= len(self.frames)

    def set_frame(self):
        self.scene.frame_set(self.frames[self.index])
        self.area.tag_redraw()
        self.ticks_since_frame_set = 0

    def tick(self):
        import bpy

        if self.finished():
            return
        self.ticks_since_frame_set += 1
        if self.ticks_since_frame_set < REDRAW_TICKS_BEFORE_SHOT:
            return

        with bpy.context.temp_override(window=self.window, area=self.area, region=self.region):
            bpy.ops.screen.screenshot_area(filepath=str(self.paths[self.index]))
        self.index += 1
        if not self.finished():
            self.set_frame()

    def stop(self):
        self.sidebars.restore()
        self.scene.frame_set(self.original_frame)


def image_size(path: Path) -> tuple[int, int]:
    import bpy

    image = bpy.data.images.load(str(path))
    try:
        return tuple(image.size)
    finally:
        bpy.data.images.remove(image)


def strip_collection(sequence_editor):
    strips = getattr(sequence_editor, "strips", None)
    return strips if strips is not None else sequence_editor.sequences


def encode_frames_to_video(paths: list[Path], video_path: str, fps: int, fps_base: float) -> None:
    import bpy

    stale = bpy.data.scenes.get(RECORDING_SCENE_NAME)
    if stale is not None:
        bpy.data.scenes.remove(stale)

    scene = bpy.data.scenes.new(RECORDING_SCENE_NAME)
    try:
        width, height = image_size(paths[0])
        scene.render.resolution_x = width - width % 2
        scene.render.resolution_y = height - height % 2
        scene.render.resolution_percentage = 100
        scene.render.fps = fps
        scene.render.fps_base = fps_base
        scene.frame_start = 1
        scene.frame_end = len(paths)

        scene.sequence_editor_create()
        strip = strip_collection(scene.sequence_editor).new_image(
            name="frames", filepath=str(paths[0]), channel=1, frame_start=1
        )
        for path in paths[1:]:
            strip.elements.append(path.name)
        strip.crop.max_x = width % 2
        strip.crop.max_y = height % 2

        if hasattr(scene.render.image_settings, "media_type"):
            scene.render.image_settings.media_type = "VIDEO"
        scene.render.image_settings.file_format = "FFMPEG"
        scene.render.ffmpeg.format = "MPEG4"
        scene.render.ffmpeg.codec = "H264"
        scene.render.ffmpeg.constant_rate_factor = "HIGH"
        scene.render.use_file_extension = True
        scene.render.filepath = video_path
        bpy.ops.render.render(animation=True, scene=scene.name)
    finally:
        bpy.data.scenes.remove(scene)


def build_operator():
    import bpy

    class THYLLORE_OT_viewport_record(bpy.types.Operator):
        """Step through the frame range, screenshot the 3D viewport and encode an MP4"""

        bl_idname = "thyllore.viewport_record"
        bl_label = "Record Viewport"

        filepath: bpy.props.StringProperty(subtype="FILE_PATH", default=DEFAULT_VIDEO_PATH)
        filter_glob: bpy.props.StringProperty(default="*.mp4", options={"HIDDEN"})

        @classmethod
        def poll(cls, context):
            return context.area is not None and context.area.type == "VIEW_3D"

        def invoke(self, context, event):
            self.view_window = context.window
            self.view_area = context.area
            context.window_manager.fileselect_add(self)
            return {"RUNNING_MODAL"}

        def execute(self, context):
            scene = context.scene
            frames = frame_numbers(scene.frame_start, scene.frame_end, scene.frame_step)
            if not frames:
                return self.fail("Frame range is empty")

            self.video_path = bpy.path.abspath(self.filepath)
            paths = frame_paths(self.video_path, len(frames))
            try:
                clear_frames_directory(paths[0].parent)
            except OSError as error:
                return self.fail(f"Cannot write frames next to {self.video_path} ({error}); choose a writable output directory")

            window = getattr(self, "view_window", context.window)
            area = getattr(self, "view_area", context.area)
            self.capture = FrameCapture(window, area, scene, frames, paths)
            self.capture.start()
            self.timer = context.window_manager.event_timer_add(TIMER_INTERVAL_SECONDS, window=window)
            context.window_manager.modal_handler_add(self)
            return {"RUNNING_MODAL"}

        def modal(self, context, event):
            if event.type == "ESC":
                self.teardown(context)
                return self.fail("Recording cancelled")
            if event.type != "TIMER":
                return {"PASS_THROUGH"}

            self.capture.tick()
            if not self.capture.finished():
                return {"RUNNING_MODAL"}

            self.teardown(context)
            return self.encode(context.scene)

        def encode(self, scene):
            paths = self.capture.paths
            width, height = image_size(paths[0])
            if min(width, height) < 2:
                return self.fail(f"Viewport capture is empty ({width}x{height}); record from the 3D view")

            encode_frames_to_video(paths, self.video_path, scene.render.fps, scene.render.fps_base)
            message = f"Recorded {len(paths)} frames to {self.video_path}"
            print(f"[Thyllore] {message}", flush=True)
            self.report({"INFO"}, message)
            return {"FINISHED"}

        def teardown(self, context):
            context.window_manager.event_timer_remove(self.timer)
            self.capture.stop()

        def fail(self, message):
            print(f"[Thyllore] {message}", flush=True)
            self.report({"ERROR"}, message)
            return {"CANCELLED"}

    return THYLLORE_OT_viewport_record


def build_panel():
    import bpy

    class VIEW3D_PT_thyllore_viewport_recording(bpy.types.Panel):

        bl_space_type = "VIEW_3D"
        bl_region_type = "UI"
        bl_category = "Thyllore"
        bl_label = "Viewport Recording"
        bl_options = {"DEFAULT_CLOSED"}

        def draw(self, context):
            scene = context.scene
            layout = self.layout
            row = layout.row(align=True)
            row.prop(scene, "frame_start", text="Start")
            row.prop(scene, "frame_end", text="End")
            layout.prop(scene, "frame_step", text="Step")
            layout.operator("thyllore.viewport_record", icon="RENDER_ANIMATION")

    return VIEW3D_PT_thyllore_viewport_recording


_registered_classes: list = []


def register() -> None:
    """Registers once per Blender session: every Thyllore effect addon ships this module."""
    import bpy

    for build in (build_operator, build_panel):
        cls = build()
        if getattr(bpy.types, cls.__name__, None) is not None:
            continue
        bpy.utils.register_class(cls)
        _registered_classes.append(cls)


def unregister() -> None:
    import bpy

    for cls in reversed(_registered_classes):
        bpy.utils.unregister_class(cls)
    _registered_classes.clear()
