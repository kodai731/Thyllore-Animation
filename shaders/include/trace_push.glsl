#ifndef TRACE_PUSH_GLSL
#define TRACE_PUSH_GLSL

// Push constant layout of the effect trace pass: [0, 80) camera for the ray generation stage,
// [80, 112) light for the closest hit stages. Each stage declares only its own range.
#ifdef TRACE_STAGE_RAYGEN
layout(push_constant) uniform TraceCamera {
    mat4 invViewProj;
    vec4 cameraPos;
} trace;
#else
layout(push_constant) uniform TraceLight {
    layout(offset = 80) vec4 lightPos;
    layout(offset = 96) vec4 lightColor;
} trace;
#endif

#endif
