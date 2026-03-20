```
src/
├── ecs/                 # Entity-Component-System core
│   ├── component/       # Component definitions (data)
│   ├── bundle/          # Common component combinations
│   ├── resource/        # Global dynamic state
│   ├── systems/         # System functions (behavior)
│   ├── events/          # Event definitions
│   ├── world.rs         # World container
│   └── query.rs         # Entity query functions
├── animation/           # Animation system
│   └── editable/        # Editable animation (components/ + systems/)
├── app/                 # Application initialization and main loop
├── loader/              # Asset loading (glTF, FBX)
├── renderer/            # Rendering pipeline
├── platform/            # Platform layer (windowing, UI)
├── vulkanr/             # Vulkan resource management
├── ml/                  # Machine learning integration
└── main.rs              # Entry point
```