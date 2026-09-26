from __future__ import annotations

import os

import bpy


class THYLLORE_OT_lightning_dump_debug(bpy.types.Operator):
    """Write the same record as the engine's dump_lightning_debug: parameters, UBO, camera,
    plus the resolved lightning texture as PNG and .npy"""

    bl_idname = "thyllore.lightning_dump_debug"
    bl_label = "Dump Lightning Debug"

    out_dir: bpy.props.StringProperty(subtype="DIR_PATH", default="/screenshots/")

    @classmethod
    def poll(cls, context):
        return context.area is not None and context.area.type == "VIEW_3D"

    def execute(self, context):
        from .dump import write_lightning_debug_dump

        out_dir = self.out_dir if os.path.isdir(self.out_dir) else os.path.expanduser("~")
        try:
            json_path = write_lightning_debug_dump(context, out_dir)
        except RuntimeError as error:
            self.report({"ERROR"}, str(error))
            return {"CANCELLED"}

        print(f"[Thyllore Lightning] debug dumped: {json_path}", flush=True)
        self.report({"INFO"}, f"wrote {json_path}")
        return {"FINISHED"}
