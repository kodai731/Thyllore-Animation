// Reverse-Z depth constants and view-ray reconstruction from a screen uv.
module ray;

public static const float DEPTH_FAR = 0.0;
public static const float DEPTH_NEAR = 1.0;

public float3 reconstructRayDirection(float2 uv, float4x4 invViewProj, float3 cameraPos) {
    float2 ndc = uv * 2.0 - 1.0;
    float4 world = mul(invViewProj, float4(ndc, DEPTH_NEAR, 1.0));
    return normalize(world.xyz / world.w - cameraPos);
}
