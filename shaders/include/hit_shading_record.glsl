#ifndef HIT_SHADING_RECORD_GLSL
#define HIT_SHADING_RECORD_GLSL

// One record per TLAS instance, indexed by gl_InstanceCustomIndexEXT. Mesh instances fill the
// vertex / index addresses; procedural instances fill params (x = kind, yz = torus radii) and
// effectData, the device address of the effect's own instance block.
struct HitShadingRecord {
    uint64_t vertexAddress;
    uint64_t indexAddress;
    uint64_t effectData;
    uint64_t reserved;
    mat4 model;
    mat4 normalMatrix;
    vec4 baseColor;
    vec4 params;
};

#endif
