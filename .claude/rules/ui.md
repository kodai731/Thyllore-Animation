# Parameter UI

**IMPORTANT:** Rules for how an effect's parameters appear in the editor windows, the curve editor,
the batch CLI and the Blender addons. The declaration in `crates/thyllore-effect-core` is the only
source; no file in `src/` lists, orders or numbers parameters.

## Declaration order is display order

Parameters are shown in the order their fields are declared, with a `#[nested]` struct expanded in
place. This holds for every consumer of the generated tables:

- `UiParam` tables (`<EFFECT>_UI_PARAMS`): the shared widget (`src/platform/ui/param_widgets.rs`)
  draws `primary` parameters first and then one section per `group`, both in first-appearance order,
  with the ungrouped remainder last. Inside a section the order is the field order.
- `ScalarParam` tables (`<EFFECT>_SCALAR_PARAMS`) and the derived `ScalarChannel` list: the curve
  editor's channel selector and the batch CLI list channels in the same field order.
- The Blender addon property panels read the same tables and keep the same order.

To change where a parameter appears, move the field (or the nested struct) in the Rust declaration.
Never add an index, a priority number or an ordering table.

## No numeric codes on parameters

An animatable field is marked `#[persist(curve)]` and nothing else. The `PropertyType::Custom` code
the engine uses at runtime is derived from the domain's registry position and the channel's declaration
position (`src/ecs/component/animation/scalar_channel.rs`); it never appears in a declaration, a file or a test
fixture.

Per-field numbers (`code = 775`, `code = [768, 769, 770]`) were rejected: every field needs a unique
number inside the effect's range, inserting a field in the middle of a struct forces a renumbering
decision, array fields need one number per component, and keeping the numbers consistent across four
effects and a golden table did not scale. Persistence does not need them either: clip files key scalar
curves by `cli_name` (`AnimationClipFile.scalar_curves`), and a rename is absorbed by
`#[persist(renamed_from = ["old_name"])]`, the same way Unity's `FormerlySerializedAs` and Unreal's
property redirects work.

```rust
// Good: the position in the struct is the whole specification
#[persist(curve, debug_range = (0.0, 1.0), ui(min = 0.0, max = 1.0))]
pub probability: f32,

// Bad: a number that must be unique, in range, and never reused
#[persist(code = 780, ui(min = 0.0, max = 1.0))]
pub probability: f32,
```

## Names

- The public name of a parameter is its underscore-joined path (`branch.depth` → `branch_depth`); it is
  the `cli_name`, the clip file key, the MCP / batch flag and the Blender property identifier.
- The label is the title-cased name (`Branch Depth`) unless `ui(label = "..")` says otherwise; there
  is no separate PascalCase or scene name.
- Renaming a field keeps saved clips loadable only with `renamed_from`; a rename without it is a
  breaking change for every clip that animates the channel.
