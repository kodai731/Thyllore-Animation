// Reverse-Z depth constants and view-ray reconstruction from a screen uv.
module ray;

public static const float DEPTH_FAR = 0.0;
public static const float DEPTH_NEAR = 1.0;

public float3 reconstructRayDirection(float2 uv, float4x4 invViewProj, float3 cameraPos) {
    float2 ndc = uv * 2.0 - 1.0;
    float4 world = mul(invViewProj, float4(ndc, DEPTH_NEAR, 1.0));
    return normalize(world.xyz / world.w - cameraPos);
}

public float worldToClipDepth(float3 worldPos, float4x4 view, float4x4 proj) {
    float4 clipPos = mul(mul(proj, view), float4(worldPos, 1.0));
    return clipPos.z / clipPos.w;
}

// Scene depth projected onto the view ray cuts the interval where an opaque surface occludes.
public bool clampToSceneDepth(
    Sampler2D sceneDepthSampler, float2 screenTexCoord, float4x4 invViewProj,
    float3 cameraPos, float3 rayDir, inout float tFar, float tNear) {
    float sceneDepth = sceneDepthSampler.Sample(screenTexCoord).r;
    if (sceneDepth == DEPTH_FAR) {
        return true;
    }
    float4 surfaceClip = mul(invViewProj, float4(screenTexCoord * 2.0 - 1.0, sceneDepth, 1.0));
    float3 surfaceWorld = surfaceClip.xyz / surfaceClip.w;
    float tDepth = dot(surfaceWorld - cameraPos, rayDir);
    if (tNear >= tDepth) {
        return false;
    }
    tFar = min(tFar, tDepth);
    return true;
}
