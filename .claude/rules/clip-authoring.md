---
paths:
  - "assets/templates/**"
  - "crates/thyllore-avatar-core/data/pose_table.toml"
  - "crates/thyllore-avatar-core/src/motion/seed/**"
  - "tools/template_smoke.py"
  - "tools/mcp/thyllore_mcp.py"
  - "src/ecs/systems/batch_run_systems/anim_edits.rs"
---

# Clip Authoring

**IMPORTANT:** How a sentence such as "wave the right hand twice" becomes a humanoid clip, in a way
that helm (the in-engine text router, an embedding classifier that cannot generate numbers) can
reproduce. The motion vocabulary lives in `crates/thyllore-avatar-core/data/pose_table.toml`; the
composer `motion::seed::systems::compose_motion` turns slots into keys. Claude translates the
sentence into slots and grows the table. Claude never decides angles inside a clip. Design:
`${RustRenderingDocPath}/Design/20261008_clip_authoring_for_helm/index.md`.

## What a clip is

- A plain bone clip: track name, duration, sparse keys. No kind, no target skeleton name.
- On a humanoid model the mapped bones are addressed by their standard name (`HumanoidRole`, e.g.
  `RightUpperArm`) in the standard space below. Unmapped bones keep their own name and local space.
- A clip authored on one humanoid model plays unchanged on every other humanoid model.

## Standard space (single source of truth)

| Item | Definition |
|---|---|
| +X / +Y / +Z | character right / up / forward |
| Handedness | left-handed (Unity). Positive rotation is clockwise when looking from the axis tip toward the origin |
| Euler order | Unity `Quaternion.Euler(x, y, z)`: rotate Z, then X, then Y |
| Zero | T-pose (arms horizontal, palms down). Every role is 0 in T-pose |
| Parent-relative | each role rotates relative to its parent, in the axes it has in T-pose |
| Mirror | left = right with `Left`/`Right` swapped and the Y and Z degrees negated; X unchanged |
| Translation | `Hips` only, metres, normalised to hips height 1 |

The engine-side conversion is `retarget_to_bones` in `crates/thyllore-avatar-core/src/motion/`.
`RightUpperArm` Z +90 raises the arm; pinned by `test_tpose_right_upper_arm_z_plus_90`.

## The pipeline

```
sentence ──Claude (today) / helm (later)──▶ MotionSpec { motion, side, count, amount, speed }
            ──compose_motion(pose_table)──▶ role keys ──batch compose=──▶ clip
```

- `pose_table.toml` holds named poses (right side, role + axis + degrees) and motions (timed pose
  list, duration, optional cycle, labels). Labels are the phrases helm will match.
- `compose=<motion>[,side=left|right][,count=<n>][,amount=<f>][,speed=<f>]` keys the current clip:
  rest at 0, every touched curve keyed at every pose (unset curves hold their previous value), rest
  at the end. Count repeats the cycle; amount scales degrees; speed divides times.
- A named pose must list every axis that defines it (hold rule): `hikite` sets `UpperArm.y = 0` as
  well as `z = -80`, otherwise an arm that was forward stays forward.
- `key=` remains for fine adjustment after `compose=`; it is not how a clip is authored.
- `settle = { blend, frames, min_change }` on a motion lets the Curve Copilot add follow-through
  after an arrival into a hold: the composer lists the curves that changed by at least
  `min_change` and then hold, the engine forecasts each from its arrival key and keeps `blend` of
  the deviation for up to `frames` frames (16 max). It never touches curves that did not just
  move, so chamber arms and planted legs stay. Needs the copilot model (`THYLLORE_SHARED_DATA_DIR`);
  without it the keys stay as composed. The Copilot cannot shape the approach to a key or the
  return to rest; those are the table's job.

## Angle table (for writing new table entries)

Right side; left mirrors Y and Z. These are the base values a new pose is built from.

| Motion | Key |
|---|---|
| Raise right arm sideways | `RightUpperArm` z +90 (−90 lowers it to the body) |
| Raise right arm forward | `RightUpperArm` y −90 |
| Bend right elbow forward | `RightLowerArm` y −90 |
| Nod | `Head` x positive. Look right: `Head` y positive |
| Bow | `Spine` x +20, `Chest` x +20 |
| Twist torso to the right | `Spine` y positive |
| Raise right leg forward | `RightUpperLeg` x −90. Bend right knee: `RightLowerLeg` x +90 |
| Lower hips | `Hips` ty negative (offset, relative to hip height) |

Avoid one role near x ±90 together with large y and z (gimbal lock).

## Writing rules for the table

- A pose keys only the roles that define it, with every axis that matters for them.
- A motion starts and ends at rest (the composer adds both); write only the intermediate poses.
- Repetition is a `cycle` on the motion, never hand-copied poses and never the Copilot.
- Copilot `copilot_extend` is for a non-periodic settling tail only; it continues the curve at its
  final velocity, so key a hold first if the curve must come to rest. Horizon at most 64 frames.
- Labels: 2 or more phrases per motion, Japanese and English, including the sentence that caused
  the entry.
- `region` is a contract, not a tag: `upper_body` may not key legs, `lower_body` may not key arms,
  `full_body` must key legs and arms or torso. The table fails to parse otherwise.
- Before writing a motion, list the body parts the real-world action moves (head, arms, torso,
  legs). A technique with a torso twist (punch, throw, swing) moves the legs: declare
  `full_body` and compose a stance (`stance`) and a hip drive (`hips_twist`) with the arms. An
  arms-only `upper_body` punch parses, but it is the wrong entry.
- Keep a motion under 10 seconds and 16 poses.

## Procedure (Claude, MCP or Bash)

The model is always `assets/models/test_humanoid/test_humanoid.fbx`. Models under
`assets/models/purchased/` are never loaded by the AI.

1. Scene file in the scratchpad (`version: 8`, `model.path` = the test humanoid).
2. Write the slots before touching the engine: motion name, side, count, amount, speed, and which
   label of the table the sentence matched. If no motion matches, add poses / a motion to
   `pose_table.toml` first, with the angle reasoning in the slot file, and run
   `cargo test -p thyllore-avatar-core --lib seed` (the table is validated at parse time).
3. One engine call: `new_clip=<name>; compose=<spec>; save=<scratch>/<name>.anim.ron`.
4. Structure: the saved RON has one track per touched role and the key counts `compose_motion`
   produces (one key per pose time + 2 rest keys).
5. Intent by eye: per pose time, `template=<saved>` with `play=true`, `frames = time × 60`, a
   `camera` close to the model (`2.5,1.2,2.5,0,0.9,0` works), and a screenshot. When the picture is
   ambiguous, use `--batch-anim-debug-dump <json>@<t1>,<t2>` and read world positions.
6. Gate: `template_check` returns `ok: true`.
7. Log one JSON line to `${SharedDataPath}/log/Rendering/clip_authoring/<date>.jsonl`
   (`text`, `spec`, `labels_matched`, `table_entries_added`, `outcome`, `clip_path`, `screenshots`).
8. Share: copy to `assets/templates/<name>.anim.ron` when asked. Report sentence, slots, table
   diff, screenshots, gate result, paths, and what the table could not express.

Which engine runs: the `thyllore` MCP server is started from the repository Claude was launched in
and prefers `target/release`. In a worktree, call the worktree binary from Bash with the same
flags (`--batch-scene`, repeated `--batch-anim-edit`, `--batch-screenshot`, `--batch-frames`,
`--batch-play`, `--batch-camera`, `--batch-anim-dump`); `tools/engine_harness.py` gives
`engine_path()`, `engine_env()`, `dood_wrap()`. `--batch-frames` needs `--batch-screenshot`.

## Files

| Path | Role |
|---|---|
| `crates/thyllore-avatar-core/data/pose_table.toml` | the vocabulary (poses, motions, labels) |
| `crates/thyllore-avatar-core/src/motion/seed/` | `PoseTable`, `MotionSpec`, `compose_motion` |
| `src/ecs/systems/batch_run_systems/anim_edits.rs` | `compose=`, `key=`, `new_clip=`, `save=`, `template=` |
| `tools/mcp/thyllore_mcp.py` | `anim_edit`, `anim_state`, `screenshot`, `template_check` |
| `tools/template_smoke.py` | the gate behind `template_check` |
| `assets/templates/` | shared clips, regenerable from the table |
