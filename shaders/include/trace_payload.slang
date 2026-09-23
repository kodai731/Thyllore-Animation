module trace_payload;

public static const int TRACE_HIT_NONE = 0;
public static const int TRACE_HIT_MESH = 1;
public static const int TRACE_HIT_EFFECT = 2;

// depth is set by the caller before TraceRay; a hit shader that traces its own secondary
// rays does so only at depth 0 so the pipeline recursion stays at two levels.
public struct TracePayload {
    public float4 color;  // rgb radiance, a = 1 hit / 0 miss
    public float4 hit;    // xyz world hit point, w = TRACE_HIT_*
    public int depth;
};
