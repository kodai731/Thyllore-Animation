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
├── app/                 # Application initialization and main loop
├── renderer/            # Rendering pipeline
├── platform/            # Platform layer (windowing, UI)
├── vulkanr/             # Vulkan resource management
├── ml/                  # Machine learning integration
└── main.rs              # Entry point
```