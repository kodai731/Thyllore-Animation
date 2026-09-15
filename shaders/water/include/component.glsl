#ifndef WATER_COMPONENT_GLSL
#define WATER_COMPONENT_GLSL

#include "water/include/ubo.glsl"

#ifndef WATER_UBO_SET
#define WATER_UBO_SET 1
#define WATER_UBO_BINDING 0
#endif

// Ray tracing hit shaders reach the instance block through the hit record's device address
// instead of a descriptor, so the shared pipeline layout stays effect independent.
#ifdef WATER_UBO_BY_REFERENCE
layout(buffer_reference, std140) buffer WaterUBORef { WaterUBO water; };
WaterUBO water;
#else
layout(set = WATER_UBO_SET, binding = WATER_UBO_BINDING, std140) uniform WaterBlock { WaterUBO water; };
#endif

#endif
