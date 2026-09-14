#ifndef WATER_COMPONENT_GLSL
#define WATER_COMPONENT_GLSL

#ifndef WATER_UBO_SET
#define WATER_UBO_SET 1
#define WATER_UBO_BINDING 0
#endif

#define WATER_UBO_FIELDS \
    mat4 model; \
    mat4 inverseModel; \
    vec4 radii; \
    vec4 absorption; \
    vec4 flow; \
    vec4 composite; \
    vec4 tint; \
    vec4 lighting; \
    vec4 scattering; \
    vec4 temporal; \
    vec4 waveModes[16]; \
    mat4 invViewProj; \
    vec4 lbModes[20];

// Ray tracing hit shaders reach the instance block through the hit record's device address
// instead of a descriptor, so the shared pipeline layout stays effect independent.
#ifdef WATER_UBO_BY_REFERENCE
layout(buffer_reference, std140) buffer WaterUBORef { WATER_UBO_FIELDS };
WaterUBORef water;
#else
layout(set = WATER_UBO_SET, binding = WATER_UBO_BINDING) uniform WaterUBO { WATER_UBO_FIELDS } water;
#endif

#endif
