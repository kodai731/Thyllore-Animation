#ifndef TRACE_PAYLOAD_GLSL
#define TRACE_PAYLOAD_GLSL

#define TRACE_HIT_NONE 0
#define TRACE_HIT_MESH 1
#define TRACE_HIT_EFFECT 2

// depth is set by the caller before traceRayEXT; a hit shader that traces its own secondary
// rays does so only at depth 0 so the pipeline recursion stays at two levels.
struct TracePayload {
    vec4 color;  // rgb radiance, a = 1 hit / 0 miss
    vec4 hit;    // xyz world hit point, w = TRACE_HIT_*
    int depth;
};

#endif
