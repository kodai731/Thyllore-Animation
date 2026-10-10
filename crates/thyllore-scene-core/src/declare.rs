/// Generates a component's scene serde impls, tag table, snapshot, scalar/UI registries and
/// overwrite fn from one declaration table (RON rejects serde(flatten); invoke in the component's crate).
#[macro_export]
macro_rules! declare_scene_format {
    (
        component: $component:ty,
        record: $record:ident,
        tag: $tag_ty:ty,
        items {
            key: $key:literal,
            tags: $tags_name:ident,
            snapshot: $snapshot_name:ident,
            scalars: $scalars_name:ident,
            ui: $ui_name:ident,
            overwrite: $overwrite_name:ident $(,)?
        },
        persisted {
            $( $name:ident : $ty:tt = $tag:ident {
                get: $get:expr,
                set: $set:expr
                $(, default: $default:expr)?
                $(, scalars { $( $alias:ident : {
                    get: $alias_get:expr,
                    set: $alias_set:expr $(,)?
                } ),+ $(,)? })?
                $(, scalars: $channels:ident)?
                $(, ui {
                    $( primary $ui_primary_comma:tt )?
                    $( kind: $ui_kind:ident, )?
                    $( label: $ui_label:expr, )?
                    min: $ui_min:expr,
                    max: $ui_max:expr
                    $(, format: $ui_format:expr)?
                    $(, tooltip: $ui_tooltip:expr)?
                    $(, group: $ui_group:expr)? $(,)?
                })?
                $(,)?
            } ),+ $(,)?
        },
        runtime {
            $( $runtime_name:ident : $runtime_ty:tt {
                get: $runtime_get:expr,
                set: $runtime_set:expr
                $(, ui {
                    $( primary $rt_ui_primary_comma:tt )?
                    $( label: $rt_ui_label:expr, )?
                    min: $rt_ui_min:expr,
                    max: $rt_ui_max:expr
                    $(, format: $rt_ui_format:expr)?
                    $(, tooltip: $rt_ui_tooltip:expr)?
                    $(, group: $rt_ui_group:expr)? $(,)?
                })?
                $(,)?
            } ),* $(,)?
        } $(,)?
    ) => {
        /// Persisted parameters (scene serde field names) mapped to their tag.
        pub const $tags_name: &[(&str, $tag_ty)] = &[
            $( (stringify!($name), <$tag_ty>::$tag) ),+
        ];

        $crate::declare_scene_format! {
            component: $component,
            record: $record,
            items {
                key: $key,
                snapshot: $snapshot_name,
                scalars: $scalars_name,
                ui: $ui_name,
                overwrite: $overwrite_name,
            },
            persisted {
                $( $name : $ty {
                    get: $get,
                    set: $set
                    $(, default: $default)?
                    $(, scalars { $( $alias : {
                        get: $alias_get,
                        set: $alias_set,
                    } ),+ })?
                    $(, scalars: $channels)?
                    $(, ui {
                        $( primary $ui_primary_comma )?
                        $( kind: $ui_kind, )?
                        $( label: $ui_label, )?
                        min: $ui_min,
                        max: $ui_max
                        $(, format: $ui_format)?
                        $(, tooltip: $ui_tooltip)?
                        $(, group: $ui_group)?
                    })?
                } ),+
            },
            runtime {
                $( $runtime_name : $runtime_ty {
                    get: $runtime_get,
                    set: $runtime_set
                    $(, ui {
                        $( primary $rt_ui_primary_comma )?
                        $( label: $rt_ui_label, )?
                        min: $rt_ui_min,
                        max: $rt_ui_max
                        $(, format: $rt_ui_format)?
                        $(, tooltip: $rt_ui_tooltip)?
                        $(, group: $rt_ui_group)?
                    })?
                } ),*
            },
        }
    };
    (
        component: $component:ty,
        record: $record:ident,
        items {
            key: $key:literal,
            snapshot: $snapshot_name:ident,
            scalars: $scalars_name:ident,
            ui: $ui_name:ident,
            overwrite: $overwrite_name:ident $(,)?
        },
        persisted {
            $( $name:ident : $ty:tt {
                get: $get:expr,
                set: $set:expr
                $(, default: $default:expr)?
                $(, scalars { $( $alias:ident : {
                    get: $alias_get:expr,
                    set: $alias_set:expr $(,)?
                } ),+ $(,)? })?
                $(, scalars: $channels:ident)?
                $(, ui {
                    $( primary $ui_primary_comma:tt )?
                    $( kind: $ui_kind:ident, )?
                    $( label: $ui_label:expr, )?
                    min: $ui_min:expr,
                    max: $ui_max:expr
                    $(, format: $ui_format:expr)?
                    $(, tooltip: $ui_tooltip:expr)?
                    $(, group: $ui_group:expr)? $(,)?
                })?
                $(,)?
            } ),+ $(,)?
        },
        runtime {
            $( $runtime_name:ident : $runtime_ty:tt {
                get: $runtime_get:expr,
                set: $runtime_set:expr
                $(, ui {
                    $( primary $rt_ui_primary_comma:tt )?
                    $( label: $rt_ui_label:expr, )?
                    min: $rt_ui_min:expr,
                    max: $rt_ui_max:expr
                    $(, format: $rt_ui_format:expr)?
                    $(, tooltip: $rt_ui_tooltip:expr)?
                    $(, group: $rt_ui_group:expr)? $(,)?
                })?
                $(,)?
            } ),* $(,)?
        } $(,)?
    ) => {
        #[derive(::serde::Serialize, ::serde::Deserialize)]
        #[serde(default)]
        struct $record {
            $( $name: $ty, )+
        }

        impl Default for $record {
            fn default() -> Self {
                let component = <$component as Default>::default();
                Self {
                    $( $name: $crate::declare_scene_format!(
                        @default component, $component, $ty, $get $(, $default)?
                    ), )+
                }
            }
        }

        impl $record {
            fn capture(component: &$component) -> Self {
                Self {
                    $( $name: {
                        let get: fn(&$component) -> $ty = $get;
                        get(component)
                    }, )+
                }
            }

            fn apply(self, component: &mut $component) {
                $( {
                    let set: fn(&mut $component, $ty) = $set;
                    set(component, self.$name);
                } )+
            }
        }

        impl ::serde::Serialize for $component {
            fn serialize<S: ::serde::Serializer>(
                &self,
                serializer: S,
            ) -> Result<S::Ok, S::Error> {
                $record::capture(self).serialize(serializer)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $component {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Self, D::Error> {
                let record = $record::deserialize(deserializer)?;
                let mut component = <$component as Default>::default();
                record.apply(&mut component);
                Ok(component)
            }
        }

        impl $crate::SceneComponent for $component {
            const TYPE_KEY: &'static str = $key;
            const PERSISTED_FIELDS: &'static [&'static str] = &[ $( stringify!($name) ),+ ];

            fn overwrite_persisted_fields(&mut self, loaded: &Self) {
                $record::capture(loaded).apply(self);
            }
        }

        /// Bit-exact snapshot of every persisted parameter; diffing two yields what a writer touched.
        pub fn $snapshot_name(component: &$component) -> Vec<(&'static str, Vec<f32>)> {
            vec![ $( (stringify!($name), {
                let get: fn(&$component) -> $ty = $get;
                $crate::SnapshotValues::snapshot_values(&get(component))
            }) ),+ ]
        }

        /// Scalar accessors: persisted f32/u32/bool, vector-component aliases, runtime-only keys.
        pub const $scalars_name: &[$crate::ScalarParam<$component>] =
            $crate::declare_scene_format!(@scalars $component, [
                $(
                    ($name, $ty, $get, $set)
                    $( $( ($alias, f32, $alias_get, $alias_set) )+ )?
                    $( ($name, $channels, $get, $set) )?
                )+
                $( ($runtime_name, $runtime_ty, $runtime_get, $runtime_set) )*
            ], []);

        /// Display metadata of the parameters that declared a `ui` node, in declaration order.
        pub const $ui_name: &[$crate::UiParam] = &[
            $( $(
                $crate::UiParam {
                    name: stringify!($name),
                    path: stringify!($name),
                    group: $crate::declare_scene_format!(@ui_or_default "" $(, $ui_group)?),
                    label: $crate::declare_scene_format!(@ui_label $(, $ui_label)?),
                    kind: $crate::declare_scene_format!(@ui_kind $(, $ui_kind)?),
                    min: $ui_min,
                    max: $ui_max,
                    format: $crate::declare_scene_format!(@ui_or_default "%.3f" $(, $ui_format)?),
                    tooltip: $crate::declare_scene_format!(@ui_or_default "" $(, $ui_tooltip)?),
                    persisted: true,
                    primary: $crate::declare_scene_format!(@ui_primary $(, primary $ui_primary_comma)?),
                },
            )? )+
            $( $(
                $crate::UiParam {
                    name: stringify!($runtime_name),
                    path: stringify!($runtime_name),
                    group: $crate::declare_scene_format!(@ui_or_default "" $(, $rt_ui_group)?),
                    label: $crate::declare_scene_format!(@ui_label $(, $rt_ui_label)?),
                    kind: $crate::UiKind::Scalar,
                    min: $rt_ui_min,
                    max: $rt_ui_max,
                    format: $crate::declare_scene_format!(@ui_or_default "%.3f" $(, $rt_ui_format)?),
                    tooltip: $crate::declare_scene_format!(@ui_or_default "" $(, $rt_ui_tooltip)?),
                    persisted: false,
                    primary: $crate::declare_scene_format!(@ui_primary $(, primary $rt_ui_primary_comma)?),
                },
            )? )*
        ];

        /// Writes every persisted parameter of `loaded` onto `target`, keeping runtime state.
        pub fn $overwrite_name(target: &mut $component, loaded: &$component) {
            $record::capture(loaded).apply(target);
        }
    };
    (@ui_kind) => {
        $crate::UiKind::Scalar
    };
    (@ui_kind, $kind:ident) => {
        $crate::UiKind::$kind
    };
    (@ui_primary) => {
        false
    };
    (@ui_primary, primary,) => {
        true
    };
    (@ui_label) => {
        None
    };
    (@ui_label, $label:expr) => {
        Some($label)
    };
    (@ui_or_default $default:expr) => {
        $default
    };
    (@ui_or_default $default:expr, $value:expr) => {
        $value
    };
    (@default $component_value:ident, $component:ty, $ty:tt, $get:expr) => {{
        let get: fn(&$component) -> $ty = $get;
        get(&$component_value)
    }};
    (@default $component_value:ident, $component:ty, $ty:tt, $get:expr, $default:expr) => {
        $default
    };
    (@scalars $component:ty, [], [ $($acc:tt)* ]) => {
        &[ $($acc)* ]
    };
    (@scalars $component:ty,
        [ ($name:ident, f32, $get:expr, $set:expr) $($rest:tt)* ],
        [ $($acc:tt)* ]
    ) => {
        $crate::declare_scene_format!(@scalars $component, [ $($rest)* ], [ $($acc)*
            $crate::ScalarParam {
                name: stringify!($name),
                get: {
                    fn get_scalar(component: &$component) -> f32 {
                        let get: fn(&$component) -> f32 = $get;
                        get(component)
                    }
                    get_scalar
                },
                set: {
                    fn set_scalar(component: &mut $component, value: f32) {
                        let set: fn(&mut $component, f32) = $set;
                        set(component, value);
                    }
                    set_scalar
                },
                debug_range: None,
                renamed_from: &[],
                curve: false,
            },
        ])
    };
    (@scalars $component:ty,
        [ ($name:ident, u32, $get:expr, $set:expr) $($rest:tt)* ],
        [ $($acc:tt)* ]
    ) => {
        $crate::declare_scene_format!(@scalars $component, [ $($rest)* ], [ $($acc)*
            $crate::ScalarParam {
                name: stringify!($name),
                get: {
                    fn get_scalar(component: &$component) -> f32 {
                        let get: fn(&$component) -> u32 = $get;
                        get(component) as f32
                    }
                    get_scalar
                },
                set: {
                    fn set_scalar(component: &mut $component, value: f32) {
                        let set: fn(&mut $component, u32) = $set;
                        set(component, value.round() as u32);
                    }
                    set_scalar
                },
                debug_range: None,
                renamed_from: &[],
                curve: false,
            },
        ])
    };
    (@scalars $component:ty,
        [ ($name:ident, bool, $get:expr, $set:expr) $($rest:tt)* ],
        [ $($acc:tt)* ]
    ) => {
        $crate::declare_scene_format!(@scalars $component, [ $($rest)* ], [ $($acc)*
            $crate::ScalarParam {
                name: stringify!($name),
                get: {
                    fn get_scalar(component: &$component) -> f32 {
                        let get: fn(&$component) -> bool = $get;
                        u8::from(get(component)) as f32
                    }
                    get_scalar
                },
                set: {
                    fn set_scalar(component: &mut $component, value: f32) {
                        let set: fn(&mut $component, bool) = $set;
                        set(component, value != 0.0);
                    }
                    set_scalar
                },
                debug_range: None,
                renamed_from: &[],
                curve: false,
            },
        ])
    };
    (@scalars $component:ty,
        [ ($name:ident, rgb, $get:expr, $set:expr) $($rest:tt)* ],
        [ $($acc:tt)* ]
    ) => {
        $crate::declare_scene_format!(@scalars $component, [ $($rest)* ], [ $($acc)*
            $crate::declare_scene_format!(@rgb_channel $component, $name, $get, $set, 0, "_r"),
            $crate::declare_scene_format!(@rgb_channel $component, $name, $get, $set, 1, "_g"),
            $crate::declare_scene_format!(@rgb_channel $component, $name, $get, $set, 2, "_b"),
        ])
    };
    (@scalars $component:ty,
        [ ($name:ident, $other:tt, $get:expr, $set:expr) $($rest:tt)* ],
        [ $($acc:tt)* ]
    ) => {
        $crate::declare_scene_format!(@scalars $component, [ $($rest)* ], [ $($acc)* ])
    };
    (@rgb_channel $component:ty, $name:ident, $get:expr, $set:expr,
        $channel:literal, $suffix:literal
    ) => {{
        struct Field;
        impl $crate::RgbField<$component> for Field {
            const GET: fn(&$component) -> [f32; 3] = $get;
            const SET: fn(&mut $component, [f32; 3]) = $set;
        }
        $crate::ScalarParam {
            name: concat!(stringify!($name), $suffix),
            get: $crate::get_rgb_channel::<$component, Field, $channel>,
            set: $crate::set_rgb_channel::<$component, Field, $channel>,
            debug_range: None,
            renamed_from: &[],
            curve: false,
        }
    }};
}
