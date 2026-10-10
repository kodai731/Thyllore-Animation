---
name: author-clip
description: Turn a natural-language motion description into a humanoid clip through the pose table and the batch `compose=` path (the same path helm will use), verify it with screenshots and template_check, log it, and save it as a .anim.ron. Use when the user asks to create, author or generate a gesture / emote / pose clip from a sentence ("右手を振るクリップを作って", "お辞儀のアニメーションを作成").
user-invocable: true
allowed-tools: Read, Write, Edit, Bash, mcp__thyllore__anim_edit, mcp__thyllore__anim_state, mcp__thyllore__screenshot, mcp__thyllore__template_check
---

# Author Clip

Follows `.claude/rules/clip-authoring.md`. Read it first; it holds the standard space, the table
format, the hold rule and the writing rules. This skill fixes the order of operations and the
report. The principle: the sentence becomes slots, the table becomes keys, Claude never keys
angles by hand.

## Usage

```
/author-clip <sentence> [name=<clip_name>] [share]
```

## Steps

### 1. Paths, engine, scene

- Read `.claude/local/paths.md` for `PROJECT_ROOT` and `SharedDataPath`.
- Decide which engine runs ("Which engine runs" in the rule). In a worktree use Bash with the
  worktree binary; treat its printed JSON like the MCP result.
- Write `<scratchpad>/author_clip/<name>/scene.ron` pointing at
  `<PROJECT_ROOT>/assets/models/test_humanoid/test_humanoid.fbx` (`version: 8`).

### 2. Slots

Write `<scratchpad>/author_clip/<name>/slots.md`:

- the sentence,
- `motion`, `side`, `count`, `amount`, `speed`, and the table label the sentence matched,
- for a sentence with several clauses, one spec per clause (sequential clauses become separate
  clips for now; say so in the report).

Read `crates/thyllore-avatar-core/data/pose_table.toml` to match. Amount words: 少し → 0.6,
大きく → 1.3; speed words: 速く → 1.5, ゆっくり → 0.7; otherwise 1.0.

### 3. Grow the table when needed

If no motion matches, first write in `slots.md` which body parts the real action moves (head /
arms / torso / legs) and pick `region` from that list, then:

- reuse existing poses where possible; add a `[[pose]]` only for a new posture, right side, every
  defining axis listed, angles from the rule's angle table, reasoning in `slots.md`;
- add a `[[motion]]` with timed poses, `duration`, a `cycle` when the sentence can repeat, and
  labels (Japanese and English, including this sentence's wording);
- check that every listed body part has a pose in the motion (a torso twist needs a stance and a
  hip drive on the legs); the parser rejects a `region` that contradicts the keyed limbs, but it
  cannot see a part you forgot to list;
- run `cargo test -p thyllore-avatar-core --lib seed`; fix the table until it passes;
- rebuild the engine (`cargo build --bin thyllore-animation`) so `compose=` sees the entry.

If the motion cannot be expressed with roles and Euler keys at all (IK, contact, physics), stop and
report; do not approximate with hand keys.

### 4. Compose and save

Set `THYLLORE_SHARED_DATA_DIR` (from `.claude/local/paths.md`, `SharedDataPath`) in the engine's
environment so a motion with `settle` gets its Copilot follow-through; check the engine log for
`compose settle:` lines and report when it was skipped.

```
new_clip=<name>; compose=<spec>; save=<scratchpad>/author_clip/<name>/<name>.anim.ron
```

One engine call. On `ok: false` or a `compose` warning in the engine log, fix the slots or the
table and rerun from `new_clip=` (idempotent).

### 5. Verify structure

Read the saved RON: one track per touched role, keys per curve = pose count + 2, duration as the
table (times count and speed). Compare against what `compose_motion` would produce; a mismatch is
an engine bug, report it.

### 6. Verify intent by eye

Per pose time: `template=<saved>`, `play=true`, `frames=round(t*60)`, camera `2.5,1.2,2.5,0,0.9,0`,
screenshot. State in one line what the figure does. If the picture is ambiguous, dump world
positions (`--batch-anim-debug-dump <json>@<t1>,<t2>`). Copy PNGs to
`/tmp/thyllore_screenshots/<name>/`.

A wrong pose means a wrong table entry: fix the table (not the clip), note it in `slots.md` under
`## Revisions`, return to step 3.

### 7. Gate

`template_check(dood=true)` must return `ok: true`.

### 8. Log

Append one JSON line to `<SharedDataPath>/log/Rendering/clip_authoring/<YYYYMMDD>.jsonl`:
`date, text, spec, labels_matched, table_entries_added, outcome (adopted|rejected|needs_table),
clip_path, screenshots, notes`.

### 9. Share (only with `share`)

Copy the RON to `<PROJECT_ROOT>/assets/templates/<name>.anim.ron`. Do not commit.

### 10. Report

1. Sentence, slots, matched label.
2. Table entries added (names only) or "none".
3. One line per pose: time, screenshot path, what the figure does.
4. `template_check` result.
5. Saved paths and the log line.
6. What the table cannot express yet.
