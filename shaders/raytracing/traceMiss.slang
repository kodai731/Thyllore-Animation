import "include/trace_payload.slang";

[shader("miss")]
void main(inout TracePayload payload) {
    payload.color = float4(0.0);
    payload.hit = float4(0.0, 0.0, 0.0, TRACE_HIT_NONE);
}
