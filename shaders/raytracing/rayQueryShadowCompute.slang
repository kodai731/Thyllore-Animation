import "include/hit_shading_record.slang";
import "include/torus_intersect.slang";

struct SceneData {
    float4 lightPosition;
    float4 lightColor;
    float4x4 view;
    float4x4 proj;
};

struct PushConstants {
    float normalOffset;
};

[[vk::binding(0, 0)]] [format("rgba32f")] RWTexture2D<float4> positionImage;
[[vk::binding(1, 0)]] [format("rgba32f")] RWTexture2D<float4> normalImage;
[[vk::binding(2, 0)]] [format("r32f")] RWTexture2D<float> shadowMaskImage;
[[vk::binding(3, 0)]] RaytracingAccelerationStructure topLevelAS;
[[vk::binding(4, 0)]] ConstantBuffer<SceneData> sceneData;
[[vk::binding(5, 0)]] StructuredBuffer<HitShadingRecord> hitTable;
[[vk::push_constant]] ConstantBuffer<PushConstants> pushConstants;

static const float SHADOW_LIT = 1.0;
static const float SHADOW_OCCLUDED = 0.0;

// Procedural candidates are water tori: intersect analytically and commit the first root in range.
void commitTorusCandidate(inout RayQuery<RAY_FLAG_NONE> rayQuery, float lightDistance) {
    HitShadingRecord rec = hitTable[rayQuery.CandidateInstanceID()];
    if (rec.params.x <= 0.5) {
        return;
    }
    float3 objectDir = rayQuery.CandidateObjectRayDirection();
    float3 o = rayQuery.CandidateObjectRayOrigin() / rec.params.y;
    float3 d = normalize(objectDir);
    float roots[4];
    bool fb;
    int n = intersectTorus(o, d, rec.params.z / rec.params.y, roots, fb);
    if (n > 0) {
        float t = roots[0] * rec.params.y / length(objectDir);
        if (t >= rayQuery.RayTMin() && t <= lightDistance) {
            rayQuery.CommitProceduralPrimitiveHit(t);
        }
    }
}

[shader("compute")]
[numthreads(16, 16, 1)]
void main(uint3 globalInvocationId : SV_DispatchThreadID) {
    int2 pixelCoord = int2(globalInvocationId.xy);
    uint width, height;
    positionImage.GetDimensions(width, height);
    if (pixelCoord.x >= int(width) || pixelCoord.y >= int(height)) {
        return;
    }

    float3 worldPosition = positionImage[pixelCoord].xyz;
    float3 worldNormal = normalImage[pixelCoord].xyz;
    if (length(worldNormal) < 0.01) {
        shadowMaskImage[pixelCoord] = SHADOW_LIT;
        return;
    }
    worldNormal = normalize(worldNormal);

    float3 lightDir = sceneData.lightPosition.xyz - worldPosition;
    float lightDistance = length(lightDir);
    lightDir = normalize(lightDir);

    RayDesc ray;
    ray.Origin = worldPosition + worldNormal * pushConstants.normalOffset;
    ray.TMin = 0.01;
    ray.Direction = lightDir;
    ray.TMax = lightDistance;
    RayQuery<RAY_FLAG_NONE> rayQuery;
    rayQuery.TraceRayInline(
        topLevelAS,
        RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH | RAY_FLAG_FORCE_OPAQUE | RAY_FLAG_SKIP_CLOSEST_HIT_SHADER,
        0xFF,
        ray
    );

    while (rayQuery.Proceed()) {
        if (rayQuery.CandidateType() == CANDIDATE_PROCEDURAL_PRIMITIVE) {
            commitTorusCandidate(rayQuery, lightDistance);
        }
    }

    float shadowMask = SHADOW_LIT;
    if (rayQuery.CommittedStatus() != COMMITTED_NOTHING) {
        shadowMask = SHADOW_OCCLUDED;
    }
    shadowMaskImage[pixelCoord] = shadowMask;
}
