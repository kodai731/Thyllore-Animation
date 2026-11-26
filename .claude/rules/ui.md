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

## Widget kind follows the declaration

`ui(kind = ..)` picks the widget: `Scalar` (float slider, the default), `Int` (whole-number slider,
`u32` fields only), `Seed` (typed-in whole number without `min` / `max`, `u32` fields only),
`Color` / `Absorption` (`[f32; 3]` pickers) and `Offset` (three-component drag).
A hand-written `UiRow::IntSlider` or `UiRow::Slider` for a value that is a field of the effect struct
is a defect; declare `ui(..)` on the field. Hand-written rows remain only for knobs that are not
fields (a macro that writes several parameters, a remapped view of one parameter).

## Conditional parameters

A parameter that has no meaning in some situation declares `ui(hidden_when = CONDITION)`, where
`CONDITION` is a `static UiCondition` declared next to the struct in `thyllore-effect-core`.
Whoever shows the parameters reports the conditions that hold and `rows_from_params` drops the
parameters that name one of them. Conditions are compared by identity, never by name.

- A condition that depends on the effect's own fields is evaluated by one function in
  `thyllore-effect-core` (`emitter_ui_conditions`), so every consumer shares the rule.
- A condition that depends on another component is reported by the panel that reads that
  component (`LightningPanel::active_conditions` reports `END_FOLLOWS_TARGET` while a
  `LightningTarget` is attached).

```rust
// Good: the declaration says which situation hides the parameter
#[runtime(ui(min = 0.2, max = 5.0, hidden_when = EMITTER_IS_NOT_RING))]
pub ring_major_radius: f32,

// Bad: UI code that knows a parameter by name
let hidden: &[&str] = if has_target { &["shape_end_offset"] } else { &[] };
```

## Names

- The public name of a parameter is its underscore-joined path (`branch.depth` → `branch_depth`); it is
  the `cli_name`, the clip file key, the MCP / batch flag and the Blender property identifier.
- The label is the title-cased name (`Branch Depth`) unless `ui(label = "..")` says otherwise; there
  is no separate PascalCase or scene name.
- Renaming a field keeps saved clips loadable only with `renamed_from`; a rename without it is a
  breaking change for every clip that animates the channel.

## Form windows are `UiRow` declarations; imgui is called only from the renderer

A window or section that shows labelled values, choices, text fields and buttons (the Inspector
sections, Avatar Setup, the scene panel, every effect section) is declared as a list of
`UiRow<View>` (`src/platform/ui/ui_rows.rs`) and drawn by `draw_rows`, the single renderer. Only
the renderer decides the label column, widget width, label elision, tooltip placement and widget ids;
a declaration carries the label, the range and which `View` field or `UiCommand` it binds to.

- A form window registers with `ui_window!(name, panel, order, form = builder)`; the builder returns
  a `FormWindow` of `FormSection`s (`src/platform/ui/form_window.rs`) and never receives
  `&imgui::Ui`. Effect sections register
  `build_effect_section::<P>` with `effect_section_hook!` (see "Effect sections are `EffectPanel` impls").
- A `RowsSection` reads a `View` snapshot in the builder, lets the rows edit a copy during drawing,
  and sends commands from `commit(world, shown, view)` only for the fields that changed. Reading
  resources, scanning directories and validating paths happen in the builder, not in a row.
- Generated parameter tables are attached with `rows_from_params` inside a `UiRow::Field` that
  projects the view onto the effect struct; the builder passes the conditions that hold this frame
  (see "Conditional parameters"), never a list of parameter names.
  A section that can key its scalars declares `keys: Some(&<EFFECT>_DOMAIN)`; the renderer
  reserves the key button's width before the widget and sends `InsertScalarKey` itself. Never
  append a widget after a row from a callback.
- `UiRow::Canvas` is for read-only drawing that no row covers (status text, timing lists). It
  receives `&View`, not `&mut View`, and must not call a value widget with a visible label.
- Hand-written `build(ui, world, ..)` windows (`ui_window!` without `form =`) are only for
  self-drawn surfaces: the curve editor, the timeline, hierarchy drag-and-drop, file browsers, the
  toolbar. A labelled slider, combo or input inside one of these is a defect; move it to a form.
- Declarations are tested without imgui: `visible_labels` / `find_duplicate_labels` on the row
  list, and the `commit` function on a `World` holding a `UiCommandQueue`.

Do not add a `FormSection` that wraps an imgui closure; that reintroduces hand-written builds behind
the form API. Extend `UiRow` or the renderer instead.

## Effect sections are `EffectPanel` impls

Every effect section is `build_effect_section::<P>` over an `EffectPanel` impl
(`src/platform/ui/effect/sections.rs`). The shared builder owns the skeleton and its commit:
Add, Instance, Preset, the declared parameters, Curves and the `<Name> Debug` group; selecting an
instance, applying a preset (which clears the scalar keys and replaces that frame's effect update),
`EffectEdit::UpdateEffect` and `EffectEdit::UpdateRenderSettings`. An effect file never repeats
these rows or commands.

An impl states the component, the render settings, the generated tables and the debug rows, and
overrides a defaulted extension point only for what is specific to the effect:

| Extension point | Use |
|---|---|
| `type Extra` + `read_extra` | view state outside the component and the render settings (a target name, a UI state resource) |
| `extra_head_rows` | rows shown below Preset whether or not an instance is selected |
| `active_conditions` | the `UiCondition`s that hold for this view |
| `param_rows` | rows around the declared parameters (`declared_param_rows`) that edit the component |
| `edited_effect` | which component value an edit writes, or `None` when another command of the same frame replaces it |
| `commit_extra` | commands for the fields of `Extra` that changed |

When an effect needs a variation no extension point covers, add a defaulted one to the trait; do
not copy the skeleton into an effect file, and do not write a `RowsSection` by hand for an effect.
