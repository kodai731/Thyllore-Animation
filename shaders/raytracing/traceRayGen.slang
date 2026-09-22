import "include/trace_payload.slang";
import "include/trace_push.slang";

[[vk::binding(0, 0)]] RaytracingAccelerationStructure tlas;
[[vk::binding(1, 0)]] [format("rgba16f")] RWTexture2D<float4> outImage;

[shader("raygeneration")]
void main() {
    uint3 launchId = DispatchRaysIndex();
    uint3 launchSize = DispatchRaysDimensions();
    float2 uv = (float2(launchId.xy) + 0.5) / float2(launchSize.xy);
    float2 ndc = uv * 2.0 - 1.0;
    float4 world = mul(trace.invViewProj, float4(ndc, 1.0, 1.0));
    float3 dir = normalize(world.xyz / world.w - trace.cameraPos.xyz);

    RayDesc ray;
    ray.Origin = trace.cameraPos.xyz;
    ray.TMin = 1e-3;
    ray.Direction = dir;
    ray.TMax = 1e30;

    TracePayload payload;
    payload.depth = 0;
    TraceRay(tlas, RAY_FLAG_FORCE_OPAQUE, 0xff, 0, 0, 0, ray, payload);
    outImage[int2(launchId.xy)] = payload.color;
}
